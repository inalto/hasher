//! Test d'integrazione del motore: esplorazione degli input, sessioni in background, annullamento.

use crossbeam_channel::{Receiver, RecvTimeoutError};
use hasher::backend::algo::Algo;
use hasher::backend::engine::{collect_files, hash_file, start, EngineConfig, EngineEvent, EngineHandle, FileId};
use hasher::backend::hasher::hash_bytes;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

fn config(algos: &[Algo], recurse: bool, include_hidden: bool) -> EngineConfig {
    EngineConfig {
        algos: algos.to_vec(),
        parallel_files: 2,
        recurse,
        follow_symlinks: false,
        include_hidden,
    }
}

fn write(path: &Path, data: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, data).unwrap();
}

/// Percorsi relativi a `root`, con `/` come separatore.
fn rel(root: &Path, paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|p| {
            p.strip_prefix(root)
                .unwrap()
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect()
}

/// Albero di prova:
/// ```text
/// a.txt  b.bin  .hidden.txt  empty.dat
/// sub/c.txt  sub/.h2  sub/deeper/d.txt
/// .hiddendir/e.txt
/// ```
fn sample_tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("a.txt"), b"abc");
    write(&r.join("b.bin"), &vec![7u8; 3 * 1024 * 1024 + 17]);
    write(&r.join(".hidden.txt"), b"hidden");
    write(&r.join("empty.dat"), b"");
    write(&r.join("sub/c.txt"), b"ccc");
    write(&r.join("sub/.h2"), b"h2");
    write(&r.join("sub/deeper/d.txt"), b"dddd");
    write(&r.join(".hiddendir/e.txt"), b"eeeee");
    dir
}

/// Raccoglie gli eventi fino ad `AllDone` compreso e verifica che dopo non ne arrivino altri.
fn drain(rx: &Receiver<EngineEvent>) -> Vec<EngineEvent> {
    let mut events = Vec::new();
    loop {
        match rx.recv_timeout(Duration::from_secs(120)) {
            Ok(ev) => {
                let done = matches!(ev, EngineEvent::AllDone { .. });
                events.push(ev);
                if done {
                    break;
                }
            }
            Err(e) => panic!("engine did not finish: {e:?}; events so far: {events:#?}"),
        }
    }
    match rx.recv_timeout(Duration::from_secs(10)) {
        Err(RecvTimeoutError::Disconnected) => {}
        other => panic!("unexpected after AllDone: {other:?}"),
    }
    events
}

fn wait_not_running(handle: &EngineHandle) {
    let t0 = Instant::now();
    while handle.is_running() {
        assert!(t0.elapsed() < Duration::from_secs(10), "engine still running");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn all_done(events: &[EngineEvent]) -> (usize, usize, bool) {
    match events.last() {
        Some(EngineEvent::AllDone {
            files_ok,
            files_failed,
            cancelled,
            ..
        }) => (*files_ok, *files_failed, *cancelled),
        other => panic!("last event is not AllDone: {other:?}"),
    }
}

// --- collect_files ----------------------------------------------------------------------------

#[test]
fn collect_files_recurse_and_hidden() {
    let dir = sample_tree();
    let root = dir.path().to_path_buf();
    let inputs = [root.clone()];

    let files = collect_files(&inputs, &config(&[], true, false));
    assert_eq!(
        rel(&root, &files),
        ["a.txt", "b.bin", "empty.dat", "sub/c.txt", "sub/deeper/d.txt"]
    );

    let files = collect_files(&inputs, &config(&[], false, false));
    assert_eq!(rel(&root, &files), ["a.txt", "b.bin", "empty.dat"]);

    let files = collect_files(&inputs, &config(&[], true, true));
    assert_eq!(
        rel(&root, &files),
        [
            ".hidden.txt",
            ".hiddendir/e.txt",
            "a.txt",
            "b.bin",
            "empty.dat",
            "sub/.h2",
            "sub/c.txt",
            "sub/deeper/d.txt"
        ]
    );

    let files = collect_files(&inputs, &config(&[], false, true));
    assert_eq!(rel(&root, &files), [".hidden.txt", "a.txt", "b.bin", "empty.dat"]);
}

#[test]
fn collect_files_explicit_inputs_dedup_and_missing() {
    let dir = sample_tree();
    let root = dir.path().to_path_buf();
    let inputs = [
        root.join(".hidden.txt"), // esplicito: mantenuto anche se nascosto
        root.join("missing.txt"), // inesistente: saltato
        root.join("sub"),
        root.join("sub/c.txt"), // già trovato esplorando `sub`
        root.join("sub"),       // cartella ripetuta
    ];
    let files = collect_files(&inputs, &config(&[], true, false));
    assert_eq!(rel(&root, &files), [".hidden.txt", "sub/c.txt", "sub/deeper/d.txt"]);
}

#[test]
fn collect_files_hidden_root_directory_is_walked() {
    let dir = sample_tree();
    let root = dir.path().to_path_buf();
    let files = collect_files(&[root.join(".hiddendir")], &config(&[], true, false));
    assert_eq!(rel(&root, &files), [".hiddendir/e.txt"]);
}

#[cfg(unix)]
#[test]
fn collect_files_symlinks() {
    let dir = sample_tree();
    let root = dir.path().to_path_buf();
    std::os::unix::fs::symlink(root.join("a.txt"), root.join("link_a")).unwrap();
    std::os::unix::fs::symlink(root.join("sub/deeper"), root.join("link_dir")).unwrap();

    let mut cfg = config(&[], true, false);
    let files = collect_files(std::slice::from_ref(&root), &cfg);
    assert_eq!(
        rel(&root, &files),
        ["a.txt", "b.bin", "empty.dat", "sub/c.txt", "sub/deeper/d.txt"]
    );

    cfg.follow_symlinks = true;
    let files = collect_files(std::slice::from_ref(&root), &cfg);
    assert_eq!(
        rel(&root, &files),
        [
            "a.txt",
            "b.bin",
            "empty.dat",
            "link_a",
            "link_dir/d.txt",
            "sub/c.txt",
            "sub/deeper/d.txt"
        ]
    );

    // Un link esplicito a un file è sempre accettato.
    cfg.follow_symlinks = false;
    let files = collect_files(&[root.join("link_a")], &cfg);
    assert_eq!(rel(&root, &files), ["link_a"]);
}

// --- hash_file --------------------------------------------------------------------------------

#[test]
fn hash_file_basics() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.bin");
    let data: Vec<u8> = (0..2_500_000u32).map(|i| (i % 253) as u8).collect();
    write(&path, &data);

    let algos = [Algo::Sha256, Algo::Crc32, Algo::Md5, Algo::Sha256];
    let mut progress = Vec::new();
    let out = hash_file(&path, &algos, None, &mut |b| progress.push(b)).unwrap();
    assert_eq!(
        out.keys().copied().collect::<Vec<_>>(),
        [Algo::Crc32, Algo::Md5, Algo::Sha256]
    );
    for (algo, digest) in &out {
        assert_eq!(digest, &hash_bytes(*algo, &data), "{algo}");
    }
    assert!(progress.windows(2).all(|w| w[0] < w[1]), "{progress:?}");
    assert_eq!(progress.last().copied(), Some(data.len() as u64));

    // Nessun algoritmo: mappa vuota ma successo.
    assert!(hash_file(&path, &[], None, &mut |_| {}).unwrap().is_empty());

    // Annullato in partenza.
    let cancel = AtomicBool::new(true);
    let err = hash_file(&path, &[Algo::Md5], Some(&cancel), &mut |_| {}).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::Interrupted);

    // File inesistente.
    let err = hash_file(&dir.path().join("nope"), &[Algo::Md5], None, &mut |_| {}).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);

    // File vuoto.
    let empty = dir.path().join("empty");
    write(&empty, b"");
    let out = hash_file(&empty, &[Algo::Md5, Algo::Ed2k], None, &mut |_| {}).unwrap();
    assert_eq!(hex::encode(&out[&Algo::Md5]), "d41d8cd98f00b204e9800998ecf8427e");
    assert_eq!(hex::encode(&out[&Algo::Ed2k]), "31d6cfe0d16ae931b73c59d7e0c089c0");
}

// --- start ------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct PerFile {
    path: PathBuf,
    size: u64,
    order: Vec<&'static str>,
    progress: Vec<u64>,
    digests: Option<BTreeMap<Algo, Vec<u8>>>,
    info_size: Option<u64>,
    info_name: Option<String>,
    error: Option<String>,
}

fn per_file(events: &[EngineEvent]) -> BTreeMap<FileId, PerFile> {
    let mut map: BTreeMap<FileId, PerFile> = BTreeMap::new();
    for ev in events {
        match ev {
            EngineEvent::Discovered { id, path, size } => {
                let f = map.entry(*id).or_default();
                f.order.push("discovered");
                f.path = path.clone();
                f.size = *size;
            }
            EngineEvent::Started { id } => map.entry(*id).or_default().order.push("started"),
            EngineEvent::Progress { id, bytes_done } => {
                let f = map.entry(*id).or_default();
                f.order.push("progress");
                f.progress.push(*bytes_done);
            }
            EngineEvent::Finished(r) => {
                let f = map.entry(r.id).or_default();
                f.order.push("finished");
                f.digests = Some(r.digests.clone());
                f.info_size = Some(r.info.size);
                f.info_name = Some(r.info.name.clone());
            }
            EngineEvent::Failed { id, path, error } => {
                let f = map.entry(*id).or_default();
                f.order.push("failed");
                assert_eq!(&f.path, path);
                f.error = Some(error.clone());
            }
            EngineEvent::AllDone { .. } => {}
        }
    }
    map
}

#[test]
fn start_hashes_every_file_and_reports_totals() {
    let dir = sample_tree();
    let root = dir.path().to_path_buf();
    let algos = [Algo::Crc32, Algo::Md5, Algo::Sha256, Algo::Ed2k];
    let cfg = config(&algos, true, false);
    let expected_files = collect_files(std::slice::from_ref(&root), &cfg);

    let (handle, rx) = start(vec![root.clone()], cfg);
    let events = drain(&rx);
    wait_not_running(&handle);

    let files = per_file(&events);
    let ids: Vec<FileId> = files.keys().copied().collect();
    assert_eq!(
        ids,
        (1..=expected_files.len() as u64).collect::<Vec<_>>(),
        "sequential ids"
    );
    let discovered: Vec<PathBuf> = files.values().map(|f| f.path.clone()).collect();
    assert_eq!(discovered, expected_files, "discovery order");

    let mut total = 0;
    for (id, f) in &files {
        let data = fs::read(&f.path).unwrap();
        assert_eq!(f.size, data.len() as u64, "#{id} size");
        total += f.size;
        assert_eq!(f.order.first(), Some(&"discovered"), "#{id} {:?}", f.order);
        assert_eq!(f.order.get(1), Some(&"started"), "#{id} {:?}", f.order);
        assert_eq!(f.order.last(), Some(&"finished"), "#{id} {:?}", f.order);
        assert_eq!(f.order.iter().filter(|s| **s == "started").count(), 1);
        assert!(
            f.progress.windows(2).all(|w| w[0] <= w[1]),
            "#{id} progress {:?}",
            f.progress
        );
        assert_eq!(f.progress.last().copied(), Some(f.size), "#{id} final progress");
        let digests = f.digests.as_ref().unwrap();
        assert_eq!(digests.keys().copied().collect::<Vec<_>>(), algos);
        for (algo, d) in digests {
            assert_eq!(d, &hash_bytes(*algo, &data), "#{id} {algo}");
        }
        assert_eq!(f.info_size, Some(f.size));
        assert_eq!(f.info_name.as_deref(), f.path.file_name().and_then(|n| n.to_str()));
    }

    assert_eq!(all_done(&events), (expected_files.len(), 0, false));
    assert_eq!(handle.files_total(), expected_files.len() as u64);
    assert_eq!(handle.files_done(), expected_files.len() as u64);
    assert_eq!(handle.bytes_total(), total);
    assert_eq!(handle.bytes_done(), handle.bytes_total());
    assert_eq!(handle.fraction(), 1.0);
    assert!(!handle.is_cancelled());
}

#[test]
fn start_with_no_inputs_finishes_immediately() {
    let (handle, rx) = start(Vec::new(), EngineConfig::default());
    let events = drain(&rx);
    assert_eq!(events.len(), 1);
    assert_eq!(all_done(&events), (0, 0, false));
    wait_not_running(&handle);
    assert_eq!(handle.fraction(), 0.0);
}

#[test]
fn start_reports_unreadable_paths_as_failed() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    write(&root.join("ok.txt"), b"fine");
    let missing = root.join("does-not-exist.bin");

    #[cfg(unix)]
    let locked = {
        use std::os::unix::fs::PermissionsExt;
        let p = root.join("locked.bin");
        write(&p, b"secret");
        fs::set_permissions(&p, fs::Permissions::from_mode(0o000)).unwrap();
        // Da root i permessi non contano: in quel caso il file è leggibile e non fallisce.
        fs::File::open(&p).is_err().then_some(p)
    };
    #[cfg(not(unix))]
    let locked: Option<PathBuf> = None;

    let mut inputs = vec![missing.clone(), root.join("ok.txt")];
    if let Some(p) = &locked {
        inputs.push(p.clone());
    }
    let (handle, rx) = start(inputs, config(&[Algo::Md5], true, false));
    let events = drain(&rx);
    wait_not_running(&handle);

    let files = per_file(&events);
    let by_path = |p: &Path| files.values().find(|f| f.path == p).unwrap();
    let m = by_path(&missing);
    assert_eq!(m.order, ["discovered", "failed"]);
    assert!(m.error.as_deref().is_some_and(|e| !e.is_empty()));
    assert_eq!(by_path(&root.join("ok.txt")).order.last(), Some(&"finished"));
    let expected_failed = 1 + usize::from(locked.is_some());
    if let Some(p) = &locked {
        let l = by_path(p);
        assert_eq!(l.order, ["discovered", "started", "failed"]);
    }
    assert_eq!(all_done(&events), (1, expected_failed, false));
    assert_eq!(handle.files_done(), handle.files_total());
    assert_eq!(handle.bytes_done(), handle.bytes_total());
    assert_eq!(handle.bytes_done(), 4);

    #[cfg(unix)]
    if let Some(p) = &locked {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(p, fs::Permissions::from_mode(0o600));
    }
}

#[test]
fn cancel_stops_promptly() {
    let dir = tempfile::tempdir().unwrap();
    let big = dir.path().join("big.bin");
    // MD2 è molto lento: il file grande non può finire prima dell'annullamento.
    write(&big, &vec![0x5au8; 48 * 1024 * 1024]);
    let mut inputs = vec![big.clone()];
    for i in 0..3 {
        let p = dir.path().join(format!("small{i}.txt"));
        write(&p, b"small");
        inputs.push(p);
    }
    let cfg = EngineConfig {
        parallel_files: 1,
        ..config(&[Algo::Md2, Algo::Sha512], true, false)
    };
    let (handle, rx) = start(inputs, cfg);

    let mut events = Vec::new();
    let mut cancelled_at = None;
    loop {
        let ev = rx.recv_timeout(Duration::from_secs(120)).expect("engine stalled");
        if cancelled_at.is_none() && matches!(ev, EngineEvent::Progress { .. }) {
            handle.cancel();
            cancelled_at = Some(Instant::now());
        }
        let done = matches!(ev, EngineEvent::AllDone { .. });
        events.push(ev);
        if done {
            break;
        }
    }
    let stop_latency = cancelled_at.expect("no progress before completion").elapsed();
    assert!(
        stop_latency < Duration::from_secs(10),
        "cancellation took {stop_latency:?}"
    );
    wait_not_running(&handle);

    let (ok, failed, cancelled) = all_done(&events);
    assert!(cancelled);
    assert_eq!((ok, failed), (0, 0));
    let files = per_file(&events);
    assert_eq!(files.len(), 4, "all four files discovered");
    assert_eq!(
        files[&1]
            .order
            .iter()
            .filter(|s| **s == "finished" || **s == "failed")
            .count(),
        0
    );
    for id in 2..=4 {
        assert_eq!(files[&id].order, ["discovered"], "#{id} must not start after cancel");
    }
    assert!(handle.bytes_done() < handle.bytes_total());
    assert!(handle.is_cancelled());
}

#[test]
fn dropping_the_receiver_stops_the_session() {
    let dir = sample_tree();
    let (handle, rx) = start(vec![dir.path().to_path_buf()], config(&[Algo::Md2], true, true));
    drop(rx);
    wait_not_running(&handle);
}

#[test]
fn many_small_files_with_parallel_workers() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..300 {
        write(
            &dir.path().join(format!("d{}/f{i:03}.txt", i % 7)),
            format!("file {i}").as_bytes(),
        );
    }
    let cfg = EngineConfig {
        parallel_files: 0,
        ..config(&Algo::DEFAULT, true, false)
    };
    let (handle, rx) = start(vec![dir.path().to_path_buf()], cfg);
    let events = drain(&rx);
    wait_not_running(&handle);
    assert_eq!(all_done(&events), (300, 0, false));
    let files = per_file(&events);
    for f in files.values() {
        let data = fs::read(&f.path).unwrap();
        assert_eq!(
            f.digests.as_ref().unwrap()[&Algo::Sha256],
            hash_bytes(Algo::Sha256, &data)
        );
    }
    assert_eq!(handle.bytes_done(), handle.bytes_total());
}
