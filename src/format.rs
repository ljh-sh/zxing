//! Output formatters: txt, json, tsv.
//!
//! All formatters are byte-stable for a given input so callers can diff /
//! cache results. The schema is intentionally compatible with wxqr's
//! output so an outer dispatcher (`x qr dec`) can fan out to either
//! backend without per-result special-casing.

use std::io::Write;
use std::path::Path;

use crate::decode::{format_name, Decoded};

/// `txt` — one line per detection:
///   `<file>\t<FORMAT>\t<text>`
///
/// For multi-barcode images, one line per code is emitted (all sharing
/// the same `<file>`). Empty results produce no output.
pub fn emit_txt<W: Write>(w: &mut W, file: &Path, results: &[Decoded]) {
    let f = file.display().to_string();
    for r in results {
        if let Err(e) = writeln!(w, "{f}\t{}\t{}", format_name(r.format), r.text) {
            // BrokenPipe is normal (head -n etc); other errors we let
            // bubble by panicking — this is a CLI, not a library.
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                panic!("stdout write failed: {e}");
            }
        }
    }
}

/// `json` — a JSON array of objects, one per input file:
/// ```json
/// [
///   {
///     "file": "qr.png",
///     "results": [
///       {"format": "QR_CODE", "text": "https://x-cmd.com", "points": [[40.5, 40.5], [250.5, 40.5], [250.5, 250.5], [40.5, 250.5]]}
///     ]
///   }
/// ]
/// ```
///
/// The `points` field is always emitted. rxing exposes the four corner
/// points of each detection (or an empty Vec when not available);
/// we surface them unconditionally. Hiding them behind a flag would
/// just add ceremony for the common case.
pub fn emit_json<W: Write>(w: &mut W, file: &Path, results: &[Decoded]) {
    let f = file.display().to_string();
    // We hand-write JSON because we want stable formatting and zero
    // dependencies. Every text and file string is escaped properly.
    let _ = write!(w, "[{{");
    write_str_obj(w, "file", &f);
    let _ = write!(w, ", \"results\": [");
    for (i, r) in results.iter().enumerate() {
        if i > 0 {
            let _ = write!(w, ", ");
        }
        let _ = write!(w, "{{");
        write_str_obj(w, "format", format_name(r.format));
        let _ = write!(w, ", ");
        write_str_obj(w, "text", &r.text);
        let _ = write!(w, ", \"points\": [");
        for (j, (x, y)) in r.points.iter().enumerate() {
            if j > 0 {
                let _ = write!(w, ", ");
            }
            let _ = write!(w, "[{x:.1}, {y:.1}]");
        }
        let _ = write!(w, "]");
        let _ = write!(w, "}}");
    }
    let _ = writeln!(w, "]}}]");
}

/// `tsv` — three columns: `<file>\t<format>\t<text>`.
/// Points are not emitted in TSV (no clean column shape for an
/// arbitrary-length array); JSON is the format that carries them.
pub fn emit_tsv<W: Write>(w: &mut W, file: &Path, results: &[Decoded]) {
    let f = file.display().to_string();
    for r in results {
        if let Err(e) = writeln!(w, "{f}\t{}\t{}", format_name(r.format), tsv_escape(&r.text)) {
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                panic!("stdout write failed: {e}");
            }
        }
    }
}

fn tsv_escape(s: &str) -> String {
    // Replace tabs and newlines with their literal escape so each row
    // stays a single TSV line. Other control chars are left alone.
    s.replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn write_str_obj<W: Write>(w: &mut W, key: &str, val: &str) {
    let _ = write!(w, "\"{key}\": \"{}\"", json_escape(val));
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
