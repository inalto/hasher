//! Esportazione dei risultati in vari formati. `export()` è il punto d'ingresso unico.
//!
//! IMPLEMENTAZIONE DEI SOTTOMODULI A CARICO DELL'AGENTE "export" (fase 1C).

use super::algo::Algo;
use super::engine::FileResult;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod checksum_fmt;
pub mod csv_fmt;
pub mod docx_fmt;
pub mod json_fmt;
pub mod txt_fmt;
pub mod xlsx_fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Txt,
    Csv,
    Json,
    Xlsx,
    Docx,
    /// File di checksum in formato GNU (`hex  nome`) per un singolo algoritmo.
    Checksum(Algo),
}

impl ExportFormat {
    pub const BASIC: [ExportFormat; 5] = [
        ExportFormat::Txt,
        ExportFormat::Csv,
        ExportFormat::Json,
        ExportFormat::Xlsx,
        ExportFormat::Docx,
    ];

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Txt => "txt",
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Xlsx => "xlsx",
            ExportFormat::Docx => "docx",
            ExportFormat::Checksum(a) => a.checksum_extension(),
        }
    }

    /// Chiave di traduzione del nome del formato (`export.format.<id>`).
    pub fn id(self) -> &'static str {
        match self {
            ExportFormat::Txt => "txt",
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Xlsx => "xlsx",
            ExportFormat::Docx => "docx",
            ExportFormat::Checksum(_) => "checksum",
        }
    }
}

/// Etichette già tradotte dall'interfaccia (l'export non dipende da rust-i18n).
#[derive(Debug, Clone, Default)]
pub struct ExportLabels {
    pub title: String,
    pub generated_by: String,
    pub generated_at: String,
    pub file: String,
    pub path: String,
    pub size: String,
    pub size_bytes: String,
    pub created: String,
    pub modified: String,
    pub attributes: String,
    pub permissions: String,
    pub algorithm: String,
    pub hash: String,
    pub files_count: String,
    pub elapsed: String,
    pub not_available: String,
}

#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Algoritmi da esportare, nell'ordine delle colonne.
    pub algos: Vec<Algo>,
    pub uppercase_hex: bool,
    pub labels: ExportLabels,
    pub app_name: String,
    pub app_version: String,
    pub company: String,
    pub generated_at: DateTime<Local>,
    /// Formato data `chrono` (es. `%d/%m/%Y %H:%M:%S`).
    pub date_format: String,
}

impl ExportOptions {
    /// Formatta una data con `date_format`; se il formato non è valido ricade su RFC 3339
    /// invece di andare in panic (chrono segnala l'errore solo in fase di scrittura).
    pub fn fmt_date(&self, d: Option<DateTime<Local>>) -> String {
        use std::fmt::Write as _;
        let Some(d) = d else {
            return self.labels.not_available.clone();
        };
        let mut s = String::new();
        if write!(s, "{}", d.format(&self.date_format)).is_ok() {
            s
        } else {
            d.to_rfc3339()
        }
    }
}

/// Scrive `results` nel file `path` nel formato richiesto.
pub fn export(path: &Path, format: ExportFormat, results: &[FileResult], opts: &ExportOptions) -> anyhow::Result<()> {
    match format {
        ExportFormat::Txt => txt_fmt::write(path, results, opts),
        ExportFormat::Csv => csv_fmt::write(path, results, opts),
        ExportFormat::Json => json_fmt::write(path, results, opts),
        ExportFormat::Xlsx => xlsx_fmt::write(path, results, opts),
        ExportFormat::Docx => docx_fmt::write(path, results, opts),
        ExportFormat::Checksum(algo) => checksum_fmt::write(path, algo, results, opts),
    }
}

// ======================================================================================
// Helper condivisi dai formati (sezione privata, fase 1C). Non fanno parte del contratto
// pubblico: sono `pub(crate)` e usati solo dai sottomoduli `*_fmt`.
// ======================================================================================

/// Riga di dati già formattata per un file, condivisa dai formati tabellari (CSV, XLSX, TXT, DOCX).
pub(crate) struct FileRow {
    pub name: String,
    pub path: String,
    /// Dimensione esatta in byte.
    pub size: u64,
    pub size_human: String,
    /// Date già formattate con `opts.fmt_date` (etichetta `not_available` se assenti).
    pub created: String,
    pub modified: String,
    /// Codici brevi degli attributi (es. `RH`); vuoto se nessuno.
    pub attributes: String,
    /// Permessi in stile `ls -l`; vuoto se non disponibili.
    pub permissions: String,
    /// Un hash per ciascun algoritmo di `opts.algos` (stesso ordine); `not_available` se mancante.
    pub hashes: Vec<String>,
}

impl FileRow {
    pub(crate) fn new(r: &FileResult, opts: &ExportOptions) -> Self {
        let info = &r.info;
        FileRow {
            name: info.name.clone(),
            path: info.path.to_string_lossy().into_owned(),
            size: info.size,
            size_human: info.size_human(),
            created: opts.fmt_date(info.created),
            modified: opts.fmt_date(info.modified),
            attributes: info.attributes_short(),
            permissions: info.unix_mode_string().unwrap_or_default(),
            hashes: opts.algos.iter().map(|a| hex_or_na(r, *a, opts)).collect(),
        }
    }
}

/// Converte tutti i risultati in righe formattate.
pub(crate) fn file_rows(results: &[FileResult], opts: &ExportOptions) -> Vec<FileRow> {
    results.iter().map(|r| FileRow::new(r, opts)).collect()
}

/// Hash esadecimale dell'algoritmo (rispettando `uppercase_hex`) oppure l'etichetta `not_available`.
pub(crate) fn hex_or_na(r: &FileResult, algo: Algo, opts: &ExportOptions) -> String {
    r.hex(algo, opts.uppercase_hex)
        .unwrap_or_else(|| opts.labels.not_available.clone())
}

/// Firma del generatore: `Hasher 0.1.0 — Martini Multimedia s.a.s.`.
pub(crate) fn app_signature(opts: &ExportOptions) -> String {
    format!("{} {} \u{2014} {}", opts.app_name, opts.app_version, opts.company)
}

/// Data di generazione formattata con `opts.date_format`.
pub(crate) fn generated_at_text(opts: &ExportOptions) -> String {
    opts.fmt_date(Some(opts.generated_at))
}

/// Coppie etichetta/valore dell'intestazione: generato da, generato il, numero di file.
pub(crate) fn meta_rows(files_count: usize, opts: &ExportOptions) -> Vec<(String, String)> {
    vec![
        (opts.labels.generated_by.clone(), app_signature(opts)),
        (opts.labels.generated_at.clone(), generated_at_text(opts)),
        (opts.labels.files_count.clone(), files_count.to_string()),
    ]
}

/// Crea il file di destinazione con buffer; l'errore riporta il percorso.
pub(crate) fn create_output(path: &Path) -> anyhow::Result<std::io::BufWriter<std::fs::File>> {
    use anyhow::Context;
    let file = std::fs::File::create(path).with_context(|| format!("cannot create {}", path.display()))?;
    Ok(std::io::BufWriter::new(file))
}

/// Svuota il buffer riportando eventuali errori di scrittura (il `Drop` di `BufWriter` li ignora).
pub(crate) fn flush_output(w: &mut std::io::BufWriter<std::fs::File>, path: &Path) -> anyhow::Result<()> {
    use anyhow::Context;
    use std::io::Write;
    w.flush().with_context(|| format!("cannot write {}", path.display()))
}
