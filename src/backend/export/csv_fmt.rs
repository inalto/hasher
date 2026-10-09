//! Esportazione in formato CSV: UTF-8 con BOM (così Excel riconosce la codifica), separatore
//! virgola, terminatore di riga CRLF (RFC 4180).

use super::{create_output, file_rows, flush_output, ExportOptions};
use crate::backend::engine::FileResult;
use anyhow::Context;
use std::io::Write;
use std::path::Path;

/// Neutralizza l'iniezione di formule nei fogli di calcolo: un testo che inizia con `= + - @`
/// (o tabulazione / ritorno a capo) verrebbe interpretato da Excel come formula; lo si
/// prefissa con un apice, secondo le raccomandazioni OWASP.
fn defuse(s: &str) -> String {
    match s.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{s}"),
        _ => s.to_string(),
    }
}

pub fn write(path: &Path, results: &[FileResult], opts: &ExportOptions) -> anyhow::Result<()> {
    let mut out = create_output(path)?;
    // BOM UTF-8.
    out.write_all(b"\xEF\xBB\xBF")
        .with_context(|| format!("cannot write {}", path.display()))?;

    let mut w = csv::WriterBuilder::new()
        .delimiter(b',')
        .terminator(csv::Terminator::CRLF)
        .from_writer(&mut out);

    let l = &opts.labels;
    let mut header: Vec<&str> = vec![
        &l.file,
        &l.path,
        &l.size_bytes,
        &l.size,
        &l.created,
        &l.modified,
        &l.attributes,
        &l.permissions,
    ];
    header.extend(opts.algos.iter().map(|a| a.name()));
    w.write_record(&header)
        .with_context(|| format!("cannot write {}", path.display()))?;

    for row in file_rows(results, opts) {
        let mut rec: Vec<String> = vec![
            defuse(&row.name),
            defuse(&row.path),
            row.size.to_string(),
            row.size_human,
            row.created,
            row.modified,
            row.attributes,
            row.permissions,
        ];
        rec.extend(row.hashes);
        w.write_record(&rec)
            .with_context(|| format!("cannot write {}", path.display()))?;
    }
    w.flush().with_context(|| format!("cannot write {}", path.display()))?;
    drop(w);
    flush_output(&mut out, path)
}
