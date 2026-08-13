//! Decode logic: load each image and run the rxing multi-barcode reader.
//!
//! v0.1 strategy: always run the multi-barcode reader (it pays for itself on
//! batch / multi-code inputs and gracefully returns a single result on
//! single-code images). The `--only` flag is honored as a hint to rxing so
//! it doesn't waste time scanning for every format we enable.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use rxing::helpers::{detect_in_file_filtered_with_hints, detect_multiple_in_file_with_hints};
use rxing::{BarcodeFormat, DecodeHints};

#[derive(Debug, Clone)]
pub struct DecodeOptions {
    pub try_harder: bool,
    pub only: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Decoded {
    pub format: BarcodeFormat,
    pub text: String,
    pub points: Vec<(f32, f32)>,
}

/// Try to decode a single image. Returns Ok(empty Vec) if no barcode was
/// found (not an error), or Err on read / decode failure.
pub fn decode_path(path: &Path, opts: &DecodeOptions, quiet: bool) -> Result<Vec<Decoded>> {
    if !path.exists() {
        return Err(anyhow!("file not found"));
    }
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow!("path is not valid UTF-8"))?;

    let hints = build_hints(opts)?;

    // Filtered reader is the right default when --only is given or when
    // we expect small modules. The standard multi-reader handles plain
    // images well. We pick based on whether the image might have very
    // small modules (--try-harder does that).
    let results: Vec<rxing::RXingResult> = if opts.try_harder {
        // With try-harder we use the FilteredImageReader which upscales
        // and tries rotated variants. It can only return a single result
        // per call, so we call it once per requested format (or just
        // once with no format filter).
        match hints.PossibleFormats.as_ref() {
            Some(formats) => {
                let mut all = Vec::new();
                for fmt in formats.iter() {
                    let mut h = hints.clone();
                    h.PossibleFormats = Some(HashSet::from([*fmt]));
                    match detect_in_file_filtered_with_hints(path_str, Some(*fmt), &mut h) {
                        Ok(r) => all.push(r),
                        Err(rxing::Exceptions::NotFoundException(_)) => {}
                        Err(e) => {
                            if !quiet {
                                eprintln!(
                                    "zxing: {}: filter-pass {} failed: {e}",
                                    path.display(),
                                    format_label(*fmt)
                                );
                            }
                            return Err(anyhow!(e.to_string()))
                                .with_context(|| format!("decode '{}'", path.display()));
                        }
                    }
                }
                all
            }
            None => match detect_in_file_filtered_with_hints(path_str, None, &mut hints.clone()) {
                Ok(r) => vec![r],
                Err(rxing::Exceptions::NotFoundException(_)) => Vec::new(),
                Err(e) => {
                    return Err(anyhow!(e.to_string()))
                        .with_context(|| format!("decode '{}'", path.display()));
                }
            },
        }
    } else {
        match detect_multiple_in_file_with_hints(path_str, &mut hints.clone()) {
            Ok(v) => v,
            Err(rxing::Exceptions::NotFoundException(_)) => Vec::new(),
            Err(e) => {
                return Err(anyhow!(e.to_string()))
                    .with_context(|| format!("decode '{}'", path.display()));
            }
        }
    };

    Ok(results
        .into_iter()
        .map(|r| {
            let points = r.getPoints().iter().map(|p| (p.x, p.y)).collect::<Vec<_>>();
            Decoded {
                format: *r.getBarcodeFormat(),
                text: r.getText().to_owned(),
                points,
            }
        })
        .collect())
}

fn build_hints(opts: &DecodeOptions) -> Result<DecodeHints> {
    let mut hints = DecodeHints {
        TryHarder: Some(opts.try_harder),
        ..DecodeHints::default()
    };

    if !opts.only.is_empty() {
        let mut set: HashSet<BarcodeFormat> = HashSet::new();
        for s in &opts.only {
            let fmt = parse_format(s).ok_or_else(|| {
                anyhow!(
                    "unknown --only value '{s}'; expected one of: \
                     qr, ean-13, ean-8, upc-a, upc-e, code-128, code-39, code-93, \
                     codabar, itf, pdf417, aztec, data-matrix, maxicode"
                )
            })?;
            set.insert(fmt);
        }
        hints.PossibleFormats = Some(set);
    }

    Ok(hints)
}

fn parse_format(s: &str) -> Option<BarcodeFormat> {
    match s.to_ascii_lowercase().as_str() {
        "qr" | "qrcode" | "qr-code" => Some(BarcodeFormat::QR_CODE),
        "micro-qr" | "mqr" | "micro_qr" => Some(BarcodeFormat::MICRO_QR_CODE),
        "rectangular-micro-qr" | "rmqr" => Some(BarcodeFormat::RECTANGULAR_MICRO_QR_CODE),
        "ean-13" | "ean13" | "ean_13" => Some(BarcodeFormat::EAN_13),
        "ean-8" | "ean8" | "ean_8" => Some(BarcodeFormat::EAN_8),
        "upc-a" | "upca" | "upc_a" => Some(BarcodeFormat::UPC_A),
        "upc-e" | "upce" | "upc_e" => Some(BarcodeFormat::UPC_E),
        "code-128" | "code128" | "code_128" => Some(BarcodeFormat::CODE_128),
        "code-39" | "code39" | "code_39" => Some(BarcodeFormat::CODE_39),
        "code-93" | "code93" | "code_93" => Some(BarcodeFormat::CODE_93),
        "codabar" => Some(BarcodeFormat::CODABAR),
        "itf" => Some(BarcodeFormat::ITF),
        "pdf417" | "pdf-417" | "pdf_417" => Some(BarcodeFormat::PDF_417),
        "aztec" => Some(BarcodeFormat::AZTEC),
        "data-matrix" | "datamatrix" | "data_matrix" => Some(BarcodeFormat::DATA_MATRIX),
        "maxicode" | "maxi-code" => Some(BarcodeFormat::MAXICODE),
        "rss-14" | "rss14" => Some(BarcodeFormat::RSS_14),
        "rss-expanded" | "rssexpanded" => Some(BarcodeFormat::RSS_EXPANDED),
        "telepen" => Some(BarcodeFormat::TELEPEN),
        _ => None,
    }
}

fn format_label(f: BarcodeFormat) -> &'static str {
    // Returns the BarcodeFormat enum variant name (UPPER_SNAKE_CASE) so
    // the output schema is byte-compatible with `ljh-sh/wxqr`'s JSON,
    // which uses OpenCV's variant names. This makes `x qr dec` able to
    // fan out to either backend without per-result special-casing.
    match f {
        BarcodeFormat::QR_CODE => "QR_CODE",
        BarcodeFormat::MICRO_QR_CODE => "MICRO_QR_CODE",
        BarcodeFormat::RECTANGULAR_MICRO_QR_CODE => "RECTANGULAR_MICRO_QR_CODE",
        BarcodeFormat::EAN_13 => "EAN_13",
        BarcodeFormat::EAN_8 => "EAN_8",
        BarcodeFormat::UPC_A => "UPC_A",
        BarcodeFormat::UPC_E => "UPC_E",
        BarcodeFormat::CODE_128 => "CODE_128",
        BarcodeFormat::CODE_39 => "CODE_39",
        BarcodeFormat::CODE_93 => "CODE_93",
        BarcodeFormat::CODABAR => "CODABAR",
        BarcodeFormat::ITF => "ITF",
        BarcodeFormat::PDF_417 => "PDF_417",
        BarcodeFormat::AZTEC => "AZTEC",
        BarcodeFormat::DATA_MATRIX => "DATA_MATRIX",
        BarcodeFormat::MAXICODE => "MAXICODE",
        BarcodeFormat::RSS_14 => "RSS_14",
        BarcodeFormat::RSS_EXPANDED => "RSS_EXPANDED",
        BarcodeFormat::TELEPEN => "TELEPEN",
        BarcodeFormat::UPC_EAN_EXTENSION => "UPC_EAN_EXTENSION",
        BarcodeFormat::DXFilmEdge => "DXFILM_EDGE",
        BarcodeFormat::UNSUPORTED_FORMAT => "UNSUPORTED_FORMAT",
    }
}

/// Format a `BarcodeFormat` for human / JSON output.
pub fn format_name(f: BarcodeFormat) -> &'static str {
    format_label(f)
}
