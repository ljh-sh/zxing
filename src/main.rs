//! `zxing` — local QR + 1D barcode decoder CLI
//!
//! Hand-rolled CLI parser (no clap) to keep the dependency tree minimal
//! and the static binary small. The argument form is:
//!
//! ```text
//! zxing dec [OPTIONS] <IMAGE>...
//! zxing --help | --version
//! ```
//!
//! All other subcommands (`enc`, `info`) are deferred — see ROADMAP.md.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};

mod decode;
mod format;

use decode::{DecodeOptions, Decoded};
use format::{emit_json, emit_tsv, emit_txt};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

const HELP: &str = "\
zxing — local QR + 1D barcode decoder

USAGE:
    zxing dec [OPTIONS] <IMAGE>...
    zxing --help | --version | --info

Reads each IMAGE, decodes any QR / 1D barcode, and writes one record per
detection to stdout. Errors on a single image do not stop the batch —
subsequent images are still attempted. Exit code is 0 if at least one
image produced at least one result, 1 if all images were scanned but no
barcodes were found, 2 on read/decode failure, 64 on usage error.

OPTIONS:
    -f, --format <FMT>   Output format: txt (default) | json | tsv | yml
        --try-harder     Try harder to detect (default: enabled).
                         Slower but picks up damaged / small / rotated codes.
        --fast           Disable try-harder for faster scanning on clean images.
        --only <FMT>     Only search for the given barcode format.
                         May be repeated: --only qr --only ean-13.
                         Formats: qr, ean-13, ean-8, upc-a, upc-e,
                                  code-128, code-39, code-93, codabar,
                                  itf, pdf417, aztec, data-matrix, maxicode
        --points         Include the four corner points in JSON / TSV output.
    -0, --null           Treat input as NUL-separated path list.
        --files-from <P> Read paths from a file ('-' for stdin); one per line.
        -q, --quiet       Suppress per-file stderr error logs (script-friendly).
    -h, --help           Show this help.
    -V, --version        Show version.

EXAMPLES:
    zxing dec qr.png                              # one image
    zxing dec --format json img1.png img2.png     # batch
    zxing dec --fast --only qr photo.png          # narrow scope
    find . -name '*.png' -print0 | \\
        xargs -0 zxing dec --format json --files-from -
    cat urls.txt | zxing dec --files-from -       # chardet-style batch
";

#[derive(Debug)]
enum Subcmd {
    Dec(DecArgs),
    Help,
    Version,
    Info,
}

#[derive(Debug)]
struct DecArgs {
    format: FormatKind,
    try_harder: bool,
    only: Vec<String>,
    points: bool,
    null_sep: bool,
    files_from: Option<String>,
    quiet: bool,
    images: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormatKind {
    Txt,
    Json,
    Tsv,
}

impl FormatKind {
    fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "txt" | "text" | "yml" | "yaml" => Some(Self::Txt),
            "json" => Some(Self::Json),
            "tsv" => Some(Self::Tsv),
            _ => None,
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match parse_args(&args) {
        Ok(Subcmd::Help) => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        Ok(Subcmd::Version) => {
            println!("zxing {VERSION}");
            ExitCode::SUCCESS
        }
        Ok(Subcmd::Info) => {
            println!("zxing {VERSION}");
            println!("{PKG_DESCRIPTION}");
            ExitCode::SUCCESS
        }
        Ok(Subcmd::Dec(args)) => run_dec(args),
        Err(e) => {
            eprintln!("zxing: {e}");
            eprintln!();
            eprintln!("Try 'zxing --help' for usage.");
            ExitCode::from(64)
        }
    }
}

fn parse_args(args: &[String]) -> Result<Subcmd> {
    if args.len() < 2 {
        return Ok(Subcmd::Help);
    }
    match args[1].as_str() {
        "-h" | "--help" => Ok(Subcmd::Help),
        "-V" | "--version" => Ok(Subcmd::Version),
        "--info" => Ok(Subcmd::Info),
        "dec" | "decode" => parse_dec(&args[2..]),
        "enc" | "encode" => Err(anyhow::anyhow!(
            "'zxing enc' is not implemented in v0.1 (decode-only). Use v0.2."
        )),
        other => Err(anyhow::anyhow!("unknown subcommand '{other}'")),
    }
}

fn parse_dec(argv: &[String]) -> Result<Subcmd> {
    let mut args = DecArgs {
        format: FormatKind::Txt,
        try_harder: true,
        only: Vec::new(),
        points: false,
        null_sep: false,
        files_from: None,
        quiet: false,
        images: Vec::new(),
    };

    let mut i = 0;
    while i < argv.len() {
        let a = &argv[i];
        match a.as_str() {
            "-h" | "--help" => return Ok(Subcmd::Help),
            "-V" | "--version" => return Ok(Subcmd::Version),
            "-f" | "--format" => {
                let v = argv
                    .get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--format requires a value"))?;
                args.format = FormatKind::parse(v).ok_or_else(|| {
                    anyhow::anyhow!(
                        "unknown --format '{v}'; expected one of txt|json|tsv|yml"
                    )
                })?;
                i += 2;
            }
            "--try-harder" => {
                args.try_harder = true;
                i += 1;
            }
            "--fast" => {
                args.try_harder = false;
                i += 1;
            }
            "--only" => {
                let v = argv
                    .get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--only requires a value"))?;
                args.only.push(v.clone());
                i += 2;
            }
            "--points" => {
                args.points = true;
                i += 1;
            }
            "-0" | "--null" => {
                args.null_sep = true;
                i += 1;
            }
            "--files-from" => {
                let v = argv
                    .get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--files-from requires a value"))?;
                args.files_from = Some(v.clone());
                i += 2;
            }
            "-q" | "--quiet" => {
                args.quiet = true;
                i += 1;
            }
            "--" => {
                args.images.extend(argv[i + 1..].iter().map(PathBuf::from));
                i = argv.len();
            }
            s if s.starts_with('-') => {
                return Err(anyhow::anyhow!("unknown flag '{s}'"));
            }
            _ => {
                args.images.push(PathBuf::from(a));
                i += 1;
            }
        }
    }

    // Expand --files-from
    if let Some(src) = args.files_from.take() {
        let content = read_files_from(&src)
            .with_context(|| format!("reading --files-from '{src}'"))?;
        for line in content.split(if args.null_sep { '\0' } else { '\n' }) {
            if line.is_empty() {
                continue;
            }
            args.images.push(PathBuf::from(line));
        }
    }

    if args.images.is_empty() {
        return Err(anyhow::anyhow!(
            "no input images; provide at least one <IMAGE> or use --files-from"
        ));
    }

    Ok(Subcmd::Dec(args))
}

fn read_files_from(src: &str) -> Result<String> {
    if src == "-" {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s)?;
        return Ok(s);
    }
    Ok(std::fs::read_to_string(src)?)
}

fn run_dec(args: DecArgs) -> ExitCode {
    let opts = DecodeOptions {
        try_harder: args.try_harder,
        only: args.only.clone(),
    };
    let mut stdout = io::stdout().lock();
    let mut had_any = false;

    for path in &args.images {
        let decoded = decode::decode_path(path, &opts, args.quiet);
        match decoded {
            Ok(results) => {
                if results.is_empty() {
                    continue;
                }
                had_any = true;
                emit(&mut stdout, args.format, path, &results, args.points);
            }
            Err(e) => {
                if !args.quiet {
                    eprintln!("zxing: {}: {e}", path.display());
                }
                return ExitCode::from(2);
            }
        }
    }

    if had_any {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn emit<W: Write>(
    w: &mut W,
    fmt: FormatKind,
    path: &Path,
    results: &[Decoded],
    with_points: bool,
) {
    match fmt {
        FormatKind::Txt => emit_txt(w, path, results),
        FormatKind::Json => emit_json(w, path, results, with_points),
        FormatKind::Tsv => emit_tsv(w, path, results, with_points),
    }
    let _ = w.flush();
}