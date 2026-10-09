//! Metadati di un file: nome, dimensione, date, attributi (specifici per sistema operativo).

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs::Metadata;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Attributi mostrati all'utente. `id()` è la chiave di traduzione (`attr.<id>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FileAttr {
    ReadOnly,
    Hidden,
    System,
    Archive,
    Compressed,
    Encrypted,
    Temporary,
    Sparse,
    Offline,
    NotIndexed,
    ReparsePoint,
    Symlink,
    Executable,
    SetUid,
    SetGid,
    Sticky,
}

impl FileAttr {
    pub fn id(self) -> &'static str {
        match self {
            FileAttr::ReadOnly => "read_only",
            FileAttr::Hidden => "hidden",
            FileAttr::System => "system",
            FileAttr::Archive => "archive",
            FileAttr::Compressed => "compressed",
            FileAttr::Encrypted => "encrypted",
            FileAttr::Temporary => "temporary",
            FileAttr::Sparse => "sparse",
            FileAttr::Offline => "offline",
            FileAttr::NotIndexed => "not_indexed",
            FileAttr::ReparsePoint => "reparse_point",
            FileAttr::Symlink => "symlink",
            FileAttr::Executable => "executable",
            FileAttr::SetUid => "setuid",
            FileAttr::SetGid => "setgid",
            FileAttr::Sticky => "sticky",
        }
    }

    /// Lettera breve in stile Windows (R, H, S, A, …) per la vista compatta.
    pub fn short(self) -> &'static str {
        match self {
            FileAttr::ReadOnly => "R",
            FileAttr::Hidden => "H",
            FileAttr::System => "S",
            FileAttr::Archive => "A",
            FileAttr::Compressed => "C",
            FileAttr::Encrypted => "E",
            FileAttr::Temporary => "T",
            FileAttr::Sparse => "P",
            FileAttr::Offline => "O",
            FileAttr::NotIndexed => "I",
            FileAttr::ReparsePoint => "L",
            FileAttr::Symlink => "L",
            FileAttr::Executable => "X",
            FileAttr::SetUid => "U",
            FileAttr::SetGid => "G",
            FileAttr::Sticky => "K",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub extension: Option<String>,
    pub size: u64,
    pub created: Option<DateTime<Local>>,
    pub modified: Option<DateTime<Local>>,
    pub accessed: Option<DateTime<Local>>,
    pub attributes: Vec<FileAttr>,
    /// Bit dei permessi POSIX (es. `0o644`), solo su Unix.
    pub unix_mode: Option<u32>,
    pub is_symlink: bool,
}

impl FileInfo {
    /// Dimensione leggibile (es. `12,3 MB`) in base 10 come i file manager, con separatori locali minimi.
    pub fn size_human(&self) -> String {
        humansize::format_size(self.size, humansize::DECIMAL)
    }

    /// Permessi in stile `ls -l` (`rwxr-xr-x`), se disponibili.
    pub fn unix_mode_string(&self) -> Option<String> {
        let m = self.unix_mode?;
        let mut s = String::with_capacity(9);
        for (shift, special) in [(6u32, 0o4000u32), (3, 0o2000), (0, 0o1000)] {
            let r = (m >> shift) & 0o4 != 0;
            let w = (m >> shift) & 0o2 != 0;
            let x = (m >> shift) & 0o1 != 0;
            let sp = m & special != 0;
            s.push(if r { 'r' } else { '-' });
            s.push(if w { 'w' } else { '-' });
            s.push(match (x, sp, shift) {
                (true, true, 0) => 't',
                (false, true, 0) => 'T',
                (true, true, _) => 's',
                (false, true, _) => 'S',
                (true, false, _) => 'x',
                (false, false, _) => '-',
            });
        }
        Some(s)
    }

    /// Attributi come stringa compatta in stile Windows (es. `RHA`), vuota se nessuno.
    pub fn attributes_short(&self) -> String {
        self.attributes.iter().map(|a| a.short()).collect()
    }
}

/// Legge i metadati del file. Non segue i link simbolici per determinare `is_symlink`, ma
/// riporta dimensione e date del bersaglio quando raggiungibile.
///
/// Le date non fornite dal sistema operativo o dal file system (per esempio la data di creazione
/// su alcuni file system Linux) risultano `None` senza generare errori.
pub fn read(path: &Path) -> io::Result<FileInfo> {
    // Metadati del link stesso (non segue i link simbolici).
    let link_meta = std::fs::symlink_metadata(path)?;
    let is_symlink = link_meta.file_type().is_symlink();
    // Metadati del bersaglio; se il link è interrotto si ripiega su quelli del link.
    let target_meta = if is_symlink { std::fs::metadata(path).ok() } else { None };
    let meta = target_meta.as_ref().unwrap_or(&link_meta);

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .filter(|e| !e.is_empty());

    let (mut attributes, unix_mode) = platform_attributes(&name, extension.as_deref(), meta, &link_meta);
    if is_symlink {
        attributes.push(FileAttr::Symlink);
    }
    attributes.sort();
    attributes.dedup();

    Ok(FileInfo {
        path: path.to_path_buf(),
        name,
        extension,
        size: meta.len(),
        created: to_local(meta.created()),
        modified: to_local(meta.modified()),
        accessed: to_local(meta.accessed()),
        attributes,
        unix_mode,
        is_symlink,
    })
}

/// Converte una data di sistema in data locale; `None` se non disponibile.
fn to_local(time: io::Result<SystemTime>) -> Option<DateTime<Local>> {
    time.ok().map(DateTime::<Local>::from)
}

// --- Windows -------------------------------------------------------------------------------

#[cfg(any(windows, test))]
mod win_attr {
    pub const READONLY: u32 = 0x1;
    pub const HIDDEN: u32 = 0x2;
    pub const SYSTEM: u32 = 0x4;
    pub const ARCHIVE: u32 = 0x20;
    pub const TEMPORARY: u32 = 0x100;
    pub const SPARSE_FILE: u32 = 0x200;
    pub const REPARSE_POINT: u32 = 0x400;
    pub const COMPRESSED: u32 = 0x800;
    pub const OFFLINE: u32 = 0x1000;
    pub const NOT_CONTENT_INDEXED: u32 = 0x2000;
    pub const ENCRYPTED: u32 = 0x4000;
}

/// Traduce i bit `FILE_ATTRIBUTE_*` di Windows (più l'estensione per `Executable`) negli
/// attributi mostrati. Funzione pura, compilata anche sulle altre piattaforme per i test.
#[cfg(any(windows, test))]
fn windows_attributes(bits: u32, extension: Option<&str>) -> Vec<FileAttr> {
    const MAP: [(u32, FileAttr); 11] = [
        (win_attr::READONLY, FileAttr::ReadOnly),
        (win_attr::HIDDEN, FileAttr::Hidden),
        (win_attr::SYSTEM, FileAttr::System),
        (win_attr::ARCHIVE, FileAttr::Archive),
        (win_attr::TEMPORARY, FileAttr::Temporary),
        (win_attr::SPARSE_FILE, FileAttr::Sparse),
        (win_attr::REPARSE_POINT, FileAttr::ReparsePoint),
        (win_attr::COMPRESSED, FileAttr::Compressed),
        (win_attr::OFFLINE, FileAttr::Offline),
        (win_attr::NOT_CONTENT_INDEXED, FileAttr::NotIndexed),
        (win_attr::ENCRYPTED, FileAttr::Encrypted),
    ];
    let mut attrs: Vec<FileAttr> = MAP.iter().filter(|(bit, _)| bits & bit != 0).map(|&(_, a)| a).collect();
    if let Some(ext) = extension {
        if matches!(ext, "exe" | "com" | "bat" | "cmd" | "msi" | "ps1") {
            attrs.push(FileAttr::Executable);
        }
    }
    attrs
}

#[cfg(windows)]
fn platform_attributes(
    _name: &str,
    extension: Option<&str>,
    meta: &Metadata,
    link_meta: &Metadata,
) -> (Vec<FileAttr>, Option<u32>) {
    use std::os::windows::fs::MetadataExt;
    // Il bit REPARSE_POINT appartiene al link stesso, non al bersaglio.
    let bits = meta.file_attributes() | (link_meta.file_attributes() & win_attr::REPARSE_POINT);
    (windows_attributes(bits, extension), None)
}

// --- Unix ----------------------------------------------------------------------------------

/// Traduce i permessi POSIX (e il nome, per i file "nascosti" con il punto) negli attributi
/// mostrati. Funzione pura.
#[cfg(unix)]
fn unix_attributes(mode: u32, name: &str) -> Vec<FileAttr> {
    let mut attrs = Vec::new();
    if mode & 0o200 == 0 {
        attrs.push(FileAttr::ReadOnly);
    }
    if mode & 0o111 != 0 {
        attrs.push(FileAttr::Executable);
    }
    if mode & 0o4000 != 0 {
        attrs.push(FileAttr::SetUid);
    }
    if mode & 0o2000 != 0 {
        attrs.push(FileAttr::SetGid);
    }
    if mode & 0o1000 != 0 {
        attrs.push(FileAttr::Sticky);
    }
    if name.starts_with('.') {
        attrs.push(FileAttr::Hidden);
    }
    attrs
}

#[cfg(unix)]
fn platform_attributes(
    name: &str,
    _extension: Option<&str>,
    meta: &Metadata,
    _link_meta: &Metadata,
) -> (Vec<FileAttr>, Option<u32>) {
    use std::os::unix::fs::MetadataExt;
    let mode = meta.mode() & 0o7777;
    (unix_attributes(mode, name), Some(mode))
}

// --- Altre piattaforme ---------------------------------------------------------------------

#[cfg(not(any(unix, windows)))]
fn platform_attributes(
    name: &str,
    _extension: Option<&str>,
    _meta: &Metadata,
    _link_meta: &Metadata,
) -> (Vec<FileAttr>, Option<u32>) {
    let mut attrs = Vec::new();
    if name.starts_with('.') {
        attrs.push(FileAttr::Hidden);
    }
    (attrs, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_info(mode: Option<u32>, attributes: Vec<FileAttr>) -> FileInfo {
        FileInfo {
            path: PathBuf::from("x"),
            name: "x".into(),
            extension: None,
            size: 0,
            created: None,
            modified: None,
            accessed: None,
            attributes,
            unix_mode: mode,
            is_symlink: false,
        }
    }

    #[test]
    fn unix_mode_string_renders_ls_style() {
        assert_eq!(
            sample_info(Some(0o644), vec![]).unix_mode_string().as_deref(),
            Some("rw-r--r--")
        );
        assert_eq!(
            sample_info(Some(0o755), vec![]).unix_mode_string().as_deref(),
            Some("rwxr-xr-x")
        );
        assert_eq!(
            sample_info(Some(0o000), vec![]).unix_mode_string().as_deref(),
            Some("---------")
        );
        // setuid con x → 's', senza x → 'S'
        assert_eq!(
            sample_info(Some(0o4755), vec![]).unix_mode_string().as_deref(),
            Some("rwsr-xr-x")
        );
        assert_eq!(
            sample_info(Some(0o4644), vec![]).unix_mode_string().as_deref(),
            Some("rwSr--r--")
        );
        // setgid e sticky
        assert_eq!(
            sample_info(Some(0o2755), vec![]).unix_mode_string().as_deref(),
            Some("rwxr-sr-x")
        );
        assert_eq!(
            sample_info(Some(0o1777), vec![]).unix_mode_string().as_deref(),
            Some("rwxrwxrwt")
        );
        assert_eq!(
            sample_info(Some(0o1776), vec![]).unix_mode_string().as_deref(),
            Some("rwxrwxrwT")
        );
        assert_eq!(sample_info(None, vec![]).unix_mode_string(), None);
    }

    #[test]
    fn attributes_short_concatenates_letters() {
        assert_eq!(sample_info(None, vec![]).attributes_short(), "");
        let info = sample_info(None, vec![FileAttr::ReadOnly, FileAttr::Hidden, FileAttr::Archive]);
        assert_eq!(info.attributes_short(), "RHA");
    }

    #[test]
    fn attr_ids_and_letters_are_unique_ids() {
        let all = [
            FileAttr::ReadOnly,
            FileAttr::Hidden,
            FileAttr::System,
            FileAttr::Archive,
            FileAttr::Compressed,
            FileAttr::Encrypted,
            FileAttr::Temporary,
            FileAttr::Sparse,
            FileAttr::Offline,
            FileAttr::NotIndexed,
            FileAttr::ReparsePoint,
            FileAttr::Symlink,
            FileAttr::Executable,
            FileAttr::SetUid,
            FileAttr::SetGid,
            FileAttr::Sticky,
        ];
        let mut ids: Vec<_> = all.iter().map(|a| a.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all.len());
    }

    #[test]
    fn windows_bits_map_to_attributes() {
        let all_bits = 0x1 | 0x2 | 0x4 | 0x20 | 0x100 | 0x200 | 0x400 | 0x800 | 0x1000 | 0x2000 | 0x4000;
        let mut attrs = windows_attributes(all_bits, None);
        attrs.sort();
        let mut expected = vec![
            FileAttr::ReadOnly,
            FileAttr::Hidden,
            FileAttr::System,
            FileAttr::Archive,
            FileAttr::Temporary,
            FileAttr::Sparse,
            FileAttr::ReparsePoint,
            FileAttr::Compressed,
            FileAttr::Offline,
            FileAttr::NotIndexed,
            FileAttr::Encrypted,
        ];
        expected.sort();
        assert_eq!(attrs, expected);

        assert_eq!(windows_attributes(0x20, Some("txt")), vec![FileAttr::Archive]);
        assert_eq!(windows_attributes(0, None), vec![]);
        // Bit non mappati (es. DIRECTORY 0x10, NORMAL 0x80) sono ignorati.
        assert_eq!(windows_attributes(0x10 | 0x80, None), vec![]);
        for ext in ["exe", "com", "bat", "cmd", "msi", "ps1"] {
            assert_eq!(windows_attributes(0, Some(ext)), vec![FileAttr::Executable], "{ext}");
        }
        assert_eq!(windows_attributes(0, Some("dll")), vec![]);
    }

    #[test]
    fn read_missing_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = read(&dir.path().join("non-existent.bin")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[cfg(unix)]
    mod unix {
        use super::super::*;
        use std::os::unix::fs::{symlink, PermissionsExt};

        fn write_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
            let p = dir.join(name);
            std::fs::write(&p, content).unwrap();
            p
        }

        fn chmod(path: &Path, mode: u32) {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
        }

        #[test]
        fn regular_file_has_size_dates_and_name() {
            let dir = tempfile::tempdir().unwrap();
            let p = write_file(dir.path(), "Report.FINAL.TXT", b"hello world");
            chmod(&p, 0o644);
            let info = read(&p).unwrap();
            assert_eq!(info.path, p);
            assert_eq!(info.name, "Report.FINAL.TXT");
            assert_eq!(info.extension.as_deref(), Some("txt"));
            assert_eq!(info.size, 11);
            assert!(info.modified.is_some());
            assert!(info.accessed.is_some());
            // `created` dipende dal file system (statx): se presente deve essere plausibile.
            if let (Some(c), Some(m)) = (info.created, info.modified) {
                assert!((m - c).num_seconds().abs() < 3600);
            }
            let age = Local::now() - info.modified.unwrap();
            assert!(age.num_seconds().abs() < 3600);
            assert!(!info.is_symlink);
            assert_eq!(info.unix_mode, Some(0o644));
            assert_eq!(info.unix_mode_string().as_deref(), Some("rw-r--r--"));
            assert!(info.attributes.is_empty(), "{:?}", info.attributes);
            assert_eq!(info.attributes_short(), "");
            assert_eq!(info.size_human(), "11 B");
        }

        #[test]
        fn extension_absent_or_dotfile_is_none() {
            let dir = tempfile::tempdir().unwrap();
            let p = write_file(dir.path(), "Makefile", b"");
            assert_eq!(read(&p).unwrap().extension, None);
            let p = write_file(dir.path(), ".bashrc", b"");
            assert_eq!(read(&p).unwrap().extension, None);
            let p = write_file(dir.path(), "archive.tar.GZ", b"");
            assert_eq!(read(&p).unwrap().extension.as_deref(), Some("gz"));
        }

        #[test]
        fn dotfile_is_hidden() {
            let dir = tempfile::tempdir().unwrap();
            let hidden = write_file(dir.path(), ".secret", b"x");
            chmod(&hidden, 0o644);
            let visible = write_file(dir.path(), "visible", b"x");
            chmod(&visible, 0o644);
            assert_eq!(read(&hidden).unwrap().attributes, vec![FileAttr::Hidden]);
            assert_eq!(read(&hidden).unwrap().attributes_short(), "H");
            assert!(!read(&visible).unwrap().attributes.contains(&FileAttr::Hidden));
        }

        #[test]
        fn readonly_after_chmod_444() {
            let dir = tempfile::tempdir().unwrap();
            let p = write_file(dir.path(), "ro.txt", b"x");
            chmod(&p, 0o644);
            assert!(!read(&p).unwrap().attributes.contains(&FileAttr::ReadOnly));
            chmod(&p, 0o444);
            let info = read(&p).unwrap();
            assert_eq!(info.unix_mode, Some(0o444));
            assert_eq!(info.attributes, vec![FileAttr::ReadOnly]);
            assert_eq!(info.unix_mode_string().as_deref(), Some("r--r--r--"));
        }

        #[test]
        fn executable_after_chmod_755() {
            let dir = tempfile::tempdir().unwrap();
            let p = write_file(dir.path(), "run.sh", b"#!/bin/sh\n");
            chmod(&p, 0o644);
            assert!(!read(&p).unwrap().attributes.contains(&FileAttr::Executable));
            chmod(&p, 0o755);
            let info = read(&p).unwrap();
            assert_eq!(info.unix_mode, Some(0o755));
            assert_eq!(info.attributes, vec![FileAttr::Executable]);
            assert_eq!(info.attributes_short(), "X");
            assert_eq!(info.unix_mode_string().as_deref(), Some("rwxr-xr-x"));
        }

        #[test]
        fn special_bits_are_reported() {
            let dir = tempfile::tempdir().unwrap();
            let p = write_file(dir.path(), "suid", b"x");
            chmod(&p, 0o6755);
            let info = read(&p).unwrap();
            assert_eq!(info.unix_mode, Some(0o6755));
            assert_eq!(
                info.attributes,
                vec![FileAttr::Executable, FileAttr::SetUid, FileAttr::SetGid]
            );
            assert_eq!(info.unix_mode_string().as_deref(), Some("rwsr-sr-x"));
            assert_eq!(info.attributes_short(), "XUG");
        }

        #[test]
        fn unix_attributes_pure_function() {
            assert_eq!(unix_attributes(0o644, "a"), vec![]);
            assert_eq!(unix_attributes(0o444, ".a"), vec![FileAttr::ReadOnly, FileAttr::Hidden]);
            assert_eq!(
                unix_attributes(0o1777, "tmp"),
                vec![FileAttr::Executable, FileAttr::Sticky]
            );
            assert_eq!(
                unix_attributes(0o2755, "x"),
                vec![FileAttr::Executable, FileAttr::SetGid]
            );
            assert_eq!(
                unix_attributes(0o100, "x"),
                vec![FileAttr::ReadOnly, FileAttr::Executable]
            );
        }

        #[test]
        fn symlink_is_detected_and_reports_target_size() {
            let dir = tempfile::tempdir().unwrap();
            let target = write_file(dir.path(), "target.bin", &[0u8; 1234]);
            chmod(&target, 0o640);
            let link = dir.path().join("link.bin");
            symlink(&target, &link).unwrap();

            let info = read(&link).unwrap();
            assert!(info.is_symlink);
            assert!(info.attributes.contains(&FileAttr::Symlink));
            assert_eq!(info.name, "link.bin");
            assert_eq!(info.extension.as_deref(), Some("bin"));
            // Dimensione e permessi sono quelli del bersaglio.
            assert_eq!(info.size, 1234);
            assert_eq!(info.unix_mode, Some(0o640));
            assert!(info.modified.is_some());
            assert!(!read(&target).unwrap().is_symlink);
        }

        #[test]
        fn broken_symlink_does_not_error() {
            let dir = tempfile::tempdir().unwrap();
            let link = dir.path().join("dangling");
            symlink(dir.path().join("missing-target"), &link).unwrap();
            let info = read(&link).unwrap();
            assert!(info.is_symlink);
            assert!(info.attributes.contains(&FileAttr::Symlink));
            assert!(info.unix_mode.is_some());
            assert!(info.modified.is_some());
        }

        #[test]
        fn attributes_are_sorted_and_deduplicated() {
            let dir = tempfile::tempdir().unwrap();
            let target = write_file(dir.path(), ".hidden-target", b"x");
            chmod(&target, 0o555);
            let link = dir.path().join(".hidden-link");
            symlink(&target, &link).unwrap();
            let info = read(&link).unwrap();
            assert_eq!(
                info.attributes,
                vec![
                    FileAttr::ReadOnly,
                    FileAttr::Hidden,
                    FileAttr::Symlink,
                    FileAttr::Executable
                ]
            );
            let mut sorted = info.attributes.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(info.attributes, sorted);
            assert_eq!(info.attributes_short(), "RHLX");
        }

        #[test]
        fn non_utf8_name_is_lossy() {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;
            let dir = tempfile::tempdir().unwrap();
            let p = dir.path().join(OsStr::from_bytes(b"bad\xffname.TXT"));
            std::fs::write(&p, b"x").unwrap();
            let info = read(&p).unwrap();
            assert_eq!(info.name, "bad\u{fffd}name.TXT");
            assert_eq!(info.extension.as_deref(), Some("txt"));
        }
    }
}
