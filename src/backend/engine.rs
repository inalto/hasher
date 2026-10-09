//! Motore di hashing: esplora gli input, legge ogni file una sola volta e aggiorna in parallelo
//! tutti gli hasher attivi; comunica con l'interfaccia tramite canale ed atomici.
//!
//! I tipi pubblici qui sotto sono il contratto usato da UI ed export: non cambiarne nomi e campi
//! senza aggiornare gli altri moduli.

use super::algo::{to_hex, Algo};
use super::fileinfo::{self, FileAttr, FileInfo};
use super::hasher::{self, StreamHasher};
use chrono::{DateTime, Local};
use crossbeam_channel::{Receiver, Sender};
use rayon::prelude::*;
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use walkdir::WalkDir;

/// Identificatore progressivo di un file all'interno di una sessione del motore.
pub type FileId = u64;

/// Risultato completo per un file.
#[derive(Debug, Clone)]
pub struct FileResult {
    pub id: FileId,
    pub info: FileInfo,
    /// Digest grezzi per algoritmo (ordinati per `Algo`).
    pub digests: BTreeMap<Algo, Vec<u8>>,
    pub elapsed: Duration,
}

impl FileResult {
    /// Digest esadecimale per l'algoritmo richiesto, se calcolato.
    pub fn hex(&self, algo: Algo, uppercase: bool) -> Option<String> {
        self.digests.get(&algo).map(|d| to_hex(d, uppercase))
    }

    /// Algoritmi il cui digest coincide con l'hash (normalizzato, minuscolo) fornito.
    pub fn matching_algos(&self, normalized_hex: &str) -> Vec<Algo> {
        self.digests
            .iter()
            .filter(|(_, d)| hex::encode(d) == normalized_hex)
            .map(|(a, _)| *a)
            .collect()
    }
}

/// Parametri di una sessione di calcolo.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub algos: Vec<Algo>,
    /// Numero di file elaborati contemporaneamente; 0 = automatico (min(core, 4)).
    pub parallel_files: usize,
    /// Esplora ricorsivamente le cartelle.
    pub recurse: bool,
    pub follow_symlinks: bool,
    /// Include file e cartelle nascosti (prefisso `.` oppure attributo Hidden su Windows).
    pub include_hidden: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            algos: Algo::DEFAULT.to_vec(),
            parallel_files: 0,
            recurse: true,
            follow_symlinks: false,
            include_hidden: false,
        }
    }
}

/// Eventi inviati all'interfaccia. L'interfaccia li consuma con `try_recv` a ogni frame.
#[derive(Debug)]
pub enum EngineEvent {
    /// Un file è stato trovato (anche durante l'esplorazione delle cartelle) e messo in coda.
    Discovered { id: FileId, path: PathBuf, size: u64 },
    /// Inizio del calcolo per il file.
    Started { id: FileId },
    /// Avanzamento (inviato al massimo ~20 volte al secondo per file).
    Progress { id: FileId, bytes_done: u64 },
    /// Calcolo completato.
    Finished(Box<FileResult>),
    /// Errore (file non leggibile, permessi, ecc.).
    Failed { id: FileId, path: PathBuf, error: String },
    /// Tutti i file sono stati elaborati (o la sessione è stata annullata).
    AllDone {
        files_ok: usize,
        files_failed: usize,
        cancelled: bool,
        elapsed: Duration,
    },
}

/// Maniglia condivisa con l'interfaccia per annullare e leggere l'avanzamento complessivo.
#[derive(Debug, Clone, Default)]
pub struct EngineHandle {
    cancel: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    bytes_done: Arc<AtomicU64>,
    bytes_total: Arc<AtomicU64>,
    files_total: Arc<AtomicU64>,
    files_done: Arc<AtomicU64>,
}

impl EngineHandle {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
    pub fn bytes_done(&self) -> u64 {
        self.bytes_done.load(Ordering::Relaxed)
    }
    pub fn bytes_total(&self) -> u64 {
        self.bytes_total.load(Ordering::Relaxed)
    }
    pub fn files_total(&self) -> u64 {
        self.files_total.load(Ordering::Relaxed)
    }
    pub fn files_done(&self) -> u64 {
        self.files_done.load(Ordering::Relaxed)
    }
    /// Frazione completata 0..=1 (0 se il totale non è ancora noto).
    pub fn fraction(&self) -> f32 {
        let t = self.bytes_total();
        if t == 0 {
            0.0
        } else {
            (self.bytes_done() as f64 / t as f64).min(1.0) as f32
        }
    }

    // --- accessori per l'implementazione del motore ---
    pub(crate) fn cancel_flag(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }
    pub(crate) fn set_running(&self, v: bool) {
        self.running.store(v, Ordering::Relaxed);
    }
    pub(crate) fn add_bytes_done(&self, n: u64) {
        self.bytes_done.fetch_add(n, Ordering::Relaxed);
    }
    pub(crate) fn add_bytes_total(&self, n: u64) {
        self.bytes_total.fetch_add(n, Ordering::Relaxed);
    }
    pub(crate) fn add_files_total(&self, n: u64) {
        self.files_total.fetch_add(n, Ordering::Relaxed);
    }
    pub(crate) fn add_files_done(&self, n: u64) {
        self.files_done.fetch_add(n, Ordering::Relaxed);
    }
}

// Accessori privati del motore (stesso modulo: accesso diretto ai campi).
impl EngineHandle {
    fn sub_bytes_total(&self, n: u64) {
        self.bytes_total.fetch_sub(n, Ordering::Relaxed);
    }
}

/// Avvia una sessione in background. Restituisce subito; gli eventi arrivano sul canale.
///
/// `inputs` può contenere file e cartelle (le cartelle vengono esplorate secondo `config`).
///
/// Funzionamento: un thread coordinatore esplora gli input e, per ogni file trovato, aggiorna i
/// totali, invia `Discovered` e accoda il lavoro; nel frattempo un gruppo di `parallel_files` thread
/// preleva i file dalla coda (in ordine di `id`) e li calcola, quindi esplorazione e calcolo si
/// sovrappongono. Alla fine (o all'annullamento) arriva `AllDone` e subito dopo `is_running()`
/// diventa `false`. Se il ricevitore viene chiuso, la sessione si annulla da sola.
///
/// Un file annullato mentre è in calcolo non produce né `Finished` né `Failed` (l'interfaccia lo
/// riconosce da `AllDone { cancelled: true, .. }`); i file ancora in coda non ricevono `Started`.
pub fn start(inputs: Vec<PathBuf>, config: EngineConfig) -> (EngineHandle, Receiver<EngineEvent>) {
    let handle = EngineHandle::default();
    let (tx, rx) = crossbeam_channel::unbounded();
    handle.set_running(true);

    let started = Instant::now();
    let session = Session {
        cancel: handle.cancel_flag(),
        handle: handle.clone(),
        tx: tx.clone(),
        started,
    };
    let spawned = thread::Builder::new()
        .name("hasher-engine".into())
        .spawn(move || run_session(session, inputs, config));
    if let Err(e) = spawned {
        log::error!("cannot start the hashing thread: {e}");
        let _ = tx.send(EngineEvent::AllDone {
            files_ok: 0,
            files_failed: 0,
            cancelled: false,
            elapsed: started.elapsed(),
        });
        handle.set_running(false);
    }
    (handle, rx)
}

/// Calcolo sincrono di un singolo file (usato dal motore, dai test e da eventuali verifiche puntuali).
/// `on_progress` riceve i byte letti finora. Restituisce `ErrorKind::Interrupted` se annullato.
pub fn hash_file(
    path: &Path,
    algos: &[Algo],
    cancel: Option<&AtomicBool>,
    on_progress: &mut dyn FnMut(u64),
) -> io::Result<BTreeMap<Algo, Vec<u8>>> {
    let algos = unique_algos(algos);
    let mut buf = Vec::new();
    hash_path(path, &algos, cancel, on_progress, &mut buf)
}

/// Espande file e cartelle in un elenco di file regolari secondo la configurazione.
///
/// Un input che è un file viene mantenuto così com'è (anche se nascosto); le cartelle vengono
/// esplorate in ordine di percorso. I percorsi duplicati sono rimossi, quelli non accessibili
/// saltati con un avviso nel log.
pub fn collect_files(inputs: &[PathBuf], config: &EngineConfig) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_inputs(inputs, config, None, &mut |found| {
        if let Found::File { path, .. } = found {
            files.push(path);
        }
        true
    });
    files
}

// =============================================================================================
// Lettura e calcolo
// =============================================================================================

/// Dimensione del buffer di lettura (riutilizzato per tutti i blocchi e, nel motore, per tutti i
/// file elaborati dallo stesso worker).
const READ_BUF_SIZE: usize = 1 << 20;
/// Sotto questa dimensione l'aggiornamento parallelo degli hasher costa più di quanto rende.
const PAR_MIN_BYTES: usize = 64 * 1024;
/// Intervallo minimo fra due eventi `Progress` dello stesso file.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);
/// File in parallelo con `parallel_files == 0`: min(core, questo valore).
const AUTO_MAX_PARALLEL_FILES: usize = 4;
/// Limite di sicurezza per valori di `parallel_files` fuori scala.
const MAX_PARALLEL_FILES: usize = 64;

fn is_set(flag: Option<&AtomicBool>) -> bool {
    flag.is_some_and(|f| f.load(Ordering::Relaxed))
}

fn cancelled_error() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "cancelled")
}

/// Rimuove i duplicati mantenendo l'ordine.
fn unique_algos(algos: &[Algo]) -> Vec<Algo> {
    let mut out = Vec::with_capacity(algos.len());
    for &a in algos {
        if !out.contains(&a) {
            out.push(a);
        }
    }
    out
}

/// Come `hash_file`, con buffer fornito dal chiamante e algoritmi già senza duplicati.
fn hash_path(
    path: &Path,
    algos: &[Algo],
    cancel: Option<&AtomicBool>,
    on_progress: &mut dyn FnMut(u64),
    buf: &mut Vec<u8>,
) -> io::Result<BTreeMap<Algo, Vec<u8>>> {
    if is_set(cancel) {
        return Err(cancelled_error());
    }
    let mut file = File::open(path)?;
    if algos.is_empty() {
        // Niente da calcolare: basta aver verificato che il file sia apribile.
        return Ok(BTreeMap::new());
    }
    hash_reader(&mut file, algos, cancel, on_progress, buf)
}

/// Legge `reader` fino alla fine a blocchi di `READ_BUF_SIZE`, aggiornando tutti gli hasher.
pub(crate) fn hash_reader<R: Read + ?Sized>(
    reader: &mut R,
    algos: &[Algo],
    cancel: Option<&AtomicBool>,
    on_progress: &mut dyn FnMut(u64),
    buf: &mut Vec<u8>,
) -> io::Result<BTreeMap<Algo, Vec<u8>>> {
    if buf.len() < READ_BUF_SIZE {
        buf.resize(READ_BUF_SIZE, 0);
    }
    let buf = &mut buf[..READ_BUF_SIZE];
    let mut hashers: Vec<(Algo, Box<dyn StreamHasher>)> = algos.iter().map(|&a| (a, hasher::build(a))).collect();
    let mut total = 0u64;
    loop {
        if is_set(cancel) {
            return Err(cancelled_error());
        }
        let n = read_full(reader, buf)?;
        if n == 0 {
            break;
        }
        update_all(&mut hashers, &buf[..n]);
        total += n as u64;
        on_progress(total);
        if n < buf.len() {
            break; // `read_full` restituisce un blocco corto solo a fine file
        }
    }
    Ok(hashers.into_iter().map(|(a, h)| (a, h.finalize())).collect())
}

/// Riempie `buf` il più possibile; restituisce meno di `buf.len()` byte solo a fine file.
fn read_full<R: Read + ?Sized>(reader: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

/// Aggiorna tutti gli hasher con lo stesso blocco: in parallelo (un task rayon per hasher) se ce
/// n'è più di uno e il blocco è abbastanza grande, altrimenti in sequenza.
fn update_all(hashers: &mut [(Algo, Box<dyn StreamHasher>)], chunk: &[u8]) {
    if hashers.len() > 1 && chunk.len() >= PAR_MIN_BYTES {
        hashers
            .par_iter_mut()
            .with_max_len(1)
            .for_each(|(_, h)| h.update(chunk));
    } else {
        for (_, h) in hashers.iter_mut() {
            h.update(chunk);
        }
    }
}

// =============================================================================================
// Esplorazione degli input
// =============================================================================================

enum Found {
    /// File regolare da calcolare.
    File { path: PathBuf, size: u64 },
    /// Input esplicito non utilizzabile (inesistente, non accessibile, non è un file regolare).
    Unusable { path: PathBuf, error: io::Error },
}

/// Esplora gli input chiamando `on_found` per ogni file, in ordine deterministico e senza duplicati.
/// Restituisce `false` se l'esplorazione è stata interrotta (annullamento o `on_found` → `false`).
fn walk_inputs(
    inputs: &[PathBuf],
    config: &EngineConfig,
    cancel: Option<&AtomicBool>,
    on_found: &mut dyn FnMut(Found) -> bool,
) -> bool {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    for input in inputs {
        if is_set(cancel) {
            return false;
        }
        let found = match fs::metadata(input) {
            Ok(meta) if meta.is_dir() => {
                if !walk_dir(input, config, cancel, &mut seen, on_found) {
                    return false;
                }
                continue;
            }
            Ok(meta) if meta.is_file() => Found::File {
                path: input.clone(),
                size: meta.len(),
            },
            Ok(_) => {
                log::warn!("skipping {}: not a regular file", input.display());
                let error = io::Error::new(io::ErrorKind::InvalidInput, "not a regular file");
                Found::Unusable {
                    path: input.clone(),
                    error,
                }
            }
            Err(error) => {
                log::warn!("cannot access {}: {error}", input.display());
                Found::Unusable {
                    path: input.clone(),
                    error,
                }
            }
        };
        if seen.insert(input.clone()) && !on_found(found) {
            return false;
        }
    }
    true
}

fn walk_dir(
    root: &Path,
    config: &EngineConfig,
    cancel: Option<&AtomicBool>,
    seen: &mut HashSet<PathBuf>,
    on_found: &mut dyn FnMut(Found) -> bool,
) -> bool {
    let include_hidden = config.include_hidden;
    let walker = WalkDir::new(root)
        .follow_links(config.follow_symlinks)
        .min_depth(1)
        .max_depth(if config.recurse { usize::MAX } else { 1 })
        // Fratelli ordinati per nome = ordine per percorso (confronto per componenti).
        .sort_by_file_name()
        .into_iter()
        .filter_entry(move |e| include_hidden || e.depth() == 0 || !is_hidden(e));
    for entry in walker {
        if is_set(cancel) {
            return false;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                log::warn!("skipping unreadable entry: {err}");
                continue;
            }
        };
        let file_type = entry.file_type();
        if !file_type.is_file() {
            if file_type.is_symlink() {
                log::debug!(
                    "skipping symbolic link {} (not following links)",
                    entry.path().display()
                );
            }
            continue;
        }
        let size = match entry.metadata() {
            Ok(meta) => meta.len(),
            Err(err) => {
                log::warn!("skipping unreadable entry: {err}");
                continue;
            }
        };
        let path = entry.into_path();
        if seen.insert(path.clone()) && !on_found(Found::File { path, size }) {
            return false;
        }
    }
    true
}

/// Nascosto: nome che inizia con `.` oppure, su Windows, attributo Hidden.
fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    if entry.file_name().as_encoded_bytes().first() == Some(&b'.') {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        if let Ok(meta) = entry.metadata() {
            if meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0 {
                return true;
            }
        }
    }
    false
}

// =============================================================================================
// Sessione in background
// =============================================================================================

/// Stato condiviso fra coordinatore e worker di una sessione.
struct Session {
    handle: EngineHandle,
    cancel: Arc<AtomicBool>,
    tx: Sender<EngineEvent>,
    started: Instant,
}

impl Session {
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Invia un evento. Se il ricevitore è stato chiuso nessuno ascolta più: la sessione si annulla.
    fn send(&self, event: EngineEvent) -> bool {
        if self.tx.send(event).is_ok() {
            true
        } else {
            self.cancel.store(true, Ordering::Relaxed);
            false
        }
    }

    /// Allinea `bytes_total` ai byte effettivamente calcolati per un file concluso (dimensione
    /// cambiata durante il calcolo, oppure file fallito), così che a fine sessione
    /// `bytes_done == bytes_total`.
    fn reconcile_total(&self, expected: u64, actual: u64) {
        if actual > expected {
            self.handle.add_bytes_total(actual - expected);
        } else if expected > actual {
            self.handle.sub_bytes_total(expected - actual);
        }
    }
}

/// Riporta `running = false` all'uscita del coordinatore, anche in caso di panic.
struct RunningGuard(EngineHandle);

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.0.set_running(false);
    }
}

struct Job {
    id: FileId,
    path: PathBuf,
    size: u64,
}

enum Outcome {
    Ok,
    Failed,
    Cancelled,
}

#[derive(Default, Clone, Copy)]
struct Counts {
    ok: usize,
    failed: usize,
}

fn worker_count(requested: usize) -> usize {
    if requested > 0 {
        return requested.min(MAX_PARALLEL_FILES);
    }
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, AUTO_MAX_PARALLEL_FILES)
}

/// Corpo del thread coordinatore.
fn run_session(session: Session, inputs: Vec<PathBuf>, config: EngineConfig) {
    let _running = RunningGuard(session.handle.clone());
    let algos = unique_algos(&config.algos);
    let workers = worker_count(config.parallel_files);
    let (job_tx, job_rx) = crossbeam_channel::unbounded::<Job>();

    let (counts, discovery_complete) = thread::scope(|scope| {
        let mut joins = Vec::with_capacity(workers);
        for i in 0..workers {
            let (session, algos, rx) = (&session, algos.as_slice(), job_rx.clone());
            let spawned = thread::Builder::new()
                .name(format!("hasher-worker-{}", i + 1))
                .spawn_scoped(scope, move || worker_loop(session, algos, rx));
            match spawned {
                Ok(join) => joins.push(join),
                Err(e) => log::warn!("cannot start hashing worker: {e}"),
            }
        }
        // Se nessun worker è partito il coordinatore calcolerà da sé dopo l'esplorazione.
        let inline_rx = joins.is_empty().then(|| job_rx.clone());
        drop(job_rx);

        // Esplorazione incrementale: i worker iniziano dal primo file mentre si continua a esplorare.
        let mut counts = Counts::default();
        let mut next_id: FileId = 1;
        let discovery_complete = walk_inputs(&inputs, &config, Some(&*session.cancel), &mut |found| {
            let id = next_id;
            next_id += 1;
            session.handle.add_files_total(1);
            match found {
                Found::File { path, size } => {
                    session.handle.add_bytes_total(size);
                    session.send(EngineEvent::Discovered {
                        id,
                        path: path.clone(),
                        size,
                    }) && job_tx.send(Job { id, path, size }).is_ok()
                }
                Found::Unusable { path, error } => {
                    if !session.send(EngineEvent::Discovered {
                        id,
                        path: path.clone(),
                        size: 0,
                    }) {
                        return false;
                    }
                    session.handle.add_files_done(1);
                    counts.failed += 1;
                    session.send(EngineEvent::Failed {
                        id,
                        path,
                        error: error.to_string(),
                    })
                }
            }
        });
        drop(job_tx);

        if let Some(rx) = inline_rx {
            let c = worker_loop(&session, &algos, rx);
            counts.ok += c.ok;
            counts.failed += c.failed;
        }
        for join in joins {
            match join.join() {
                Ok(c) => {
                    counts.ok += c.ok;
                    counts.failed += c.failed;
                }
                Err(_) => log::error!("a hashing worker panicked"),
            }
        }
        (counts, discovery_complete)
    });

    let all_processed = (counts.ok + counts.failed) as u64 >= session.handle.files_total();
    let cancelled = session.cancelled() && !(discovery_complete && all_processed);
    session.send(EngineEvent::AllDone {
        files_ok: counts.ok,
        files_failed: counts.failed,
        cancelled,
        elapsed: session.started.elapsed(),
    });
}

/// Ciclo di un worker: preleva i file dalla coda finché è aperta e la sessione non è annullata.
fn worker_loop(session: &Session, algos: &[Algo], jobs: Receiver<Job>) -> Counts {
    let mut counts = Counts::default();
    let mut buf = Vec::new(); // allocato al primo file, poi riutilizzato
    while !session.cancelled() {
        let Ok(job) = jobs.recv() else { break };
        if session.cancelled() {
            break;
        }
        match process_file(session, algos, job, &mut buf) {
            Outcome::Ok => counts.ok += 1,
            Outcome::Failed => counts.failed += 1,
            Outcome::Cancelled => break,
        }
    }
    counts
}

fn process_file(session: &Session, algos: &[Algo], job: Job, buf: &mut Vec<u8>) -> Outcome {
    let Job { id, path, size } = job;
    if !session.send(EngineEvent::Started { id }) {
        return Outcome::Cancelled;
    }
    let t0 = Instant::now();
    let mut hashed = 0u64;
    let result = {
        let mut last_emit = t0;
        let mut on_progress = |done: u64| {
            session.handle.add_bytes_done(done.saturating_sub(hashed));
            hashed = done;
            let now = Instant::now();
            if now.duration_since(last_emit) >= PROGRESS_INTERVAL {
                last_emit = now;
                session.send(EngineEvent::Progress { id, bytes_done: done });
            }
        };
        let run = || hash_path(&path, algos, Some(&*session.cancel), &mut on_progress, buf);
        panic::catch_unwind(AssertUnwindSafe(run))
            .unwrap_or_else(|_| Err(io::Error::other("internal error while hashing")))
    };
    let elapsed = t0.elapsed();

    match result {
        Ok(digests) => {
            session.reconcile_total(size, hashed);
            session.handle.add_files_done(1);
            session.send(EngineEvent::Progress { id, bytes_done: hashed });
            let info = file_info(&path, hashed);
            session.send(EngineEvent::Finished(Box::new(FileResult {
                id,
                info,
                digests,
                elapsed,
            })));
            Outcome::Ok
        }
        Err(e) if e.kind() == io::ErrorKind::Interrupted && session.cancelled() => Outcome::Cancelled,
        Err(e) => {
            session.reconcile_total(size, hashed);
            session.handle.add_files_done(1);
            session.send(EngineEvent::Failed {
                id,
                path,
                error: e.to_string(),
            });
            Outcome::Failed
        }
    }
}

// =============================================================================================
// Metadati
// =============================================================================================

/// Metadati per `FileResult`: `fileinfo::read`, con ripiego su metadati essenziali se fallisce
/// (o va in panic), così che un risultato calcolato non vada mai perso.
fn file_info(path: &Path, hashed: u64) -> FileInfo {
    match panic::catch_unwind(AssertUnwindSafe(|| fileinfo::read(path))) {
        Ok(Ok(info)) => return info,
        Ok(Err(e)) => log::debug!("fileinfo::read({}) failed: {e}", path.display()),
        Err(_) => log::debug!("fileinfo::read({}) panicked", path.display()),
    }
    basic_info(path).unwrap_or_else(|e| {
        log::warn!("cannot read metadata of {}: {e}", path.display());
        minimal_info(path, hashed)
    })
}

/// Metadati essenziali ricavati da `std::fs`.
fn basic_info(path: &Path) -> io::Result<FileInfo> {
    let meta = fs::metadata(path)?;
    let is_symlink = fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    let mut info = minimal_info(path, meta.len());
    info.created = meta.created().ok().map(DateTime::<Local>::from);
    info.modified = meta.modified().ok().map(DateTime::<Local>::from);
    info.accessed = meta.accessed().ok().map(DateTime::<Local>::from);
    info.is_symlink = is_symlink;
    if meta.permissions().readonly() {
        info.attributes.push(FileAttr::ReadOnly);
    }
    if is_symlink {
        info.attributes.push(FileAttr::Symlink);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        info.unix_mode = Some(meta.permissions().mode() & 0o7777);
    }
    Ok(info)
}

/// Solo nome e dimensione: ultimo ripiego quando nemmeno `std::fs::metadata` riesce.
fn minimal_info(path: &Path, size: u64) -> FileInfo {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    FileInfo {
        path: path.to_path_buf(),
        name,
        extension: path.extension().map(|e| e.to_string_lossy().into_owned()),
        size,
        created: None,
        modified: None,
        accessed: None,
        attributes: Vec::new(),
        unix_mode: None,
        is_symlink: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_full_handles_short_reads() {
        /// Lettore che restituisce al massimo 3 byte per chiamata.
        struct Trickle(Cursor<Vec<u8>>);
        impl Read for Trickle {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                let n = buf.len().min(3);
                self.0.read(&mut buf[..n])
            }
        }
        let mut r = Trickle(Cursor::new((0..20u8).collect()));
        let mut buf = [0u8; 8];
        assert_eq!(read_full(&mut r, &mut buf).unwrap(), 8);
        assert_eq!(read_full(&mut r, &mut buf).unwrap(), 8);
        assert_eq!(read_full(&mut r, &mut buf).unwrap(), 4);
        assert_eq!(read_full(&mut r, &mut buf).unwrap(), 0);
    }

    #[test]
    fn hash_reader_matches_hash_bytes_across_buffer_boundaries() {
        let data: Vec<u8> = (0..(READ_BUF_SIZE * 2 + 12_345)).map(|i| (i % 251) as u8).collect();
        let algos = [Algo::Crc32, Algo::Sha256, Algo::Blake3, Algo::Ed2k];
        let mut seen = Vec::new();
        let mut buf = Vec::new();
        let out = hash_reader(&mut Cursor::new(&data), &algos, None, &mut |b| seen.push(b), &mut buf).unwrap();
        for a in algos {
            assert_eq!(out[&a], hasher::hash_bytes(a, &data), "{a}");
        }
        assert_eq!(
            seen,
            vec![READ_BUF_SIZE as u64, 2 * READ_BUF_SIZE as u64, data.len() as u64]
        );
    }

    #[test]
    fn unique_algos_keeps_order() {
        let v = unique_algos(&[Algo::Sha1, Algo::Md5, Algo::Sha1, Algo::Crc32, Algo::Md5]);
        assert_eq!(v, vec![Algo::Sha1, Algo::Md5, Algo::Crc32]);
    }

    #[test]
    fn worker_count_rules() {
        assert_eq!(worker_count(3), 3);
        assert_eq!(worker_count(1000), MAX_PARALLEL_FILES);
        let auto = worker_count(0);
        assert!((1..=AUTO_MAX_PARALLEL_FILES).contains(&auto));
    }

    #[test]
    fn handle_reconcile_total() {
        let (tx, _rx) = crossbeam_channel::unbounded();
        let handle = EngineHandle::default();
        let session = Session {
            cancel: handle.cancel_flag(),
            handle: handle.clone(),
            tx,
            started: Instant::now(),
        };
        handle.add_bytes_total(100);
        session.reconcile_total(100, 60);
        assert_eq!(handle.bytes_total(), 60);
        session.reconcile_total(10, 25);
        assert_eq!(handle.bytes_total(), 75);
    }
}
