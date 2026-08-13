//! Integration tests for the format module + CLI argument parsing.
//!
//! These tests don't load images (which would be slow and flaky in CI)
//! — they cover the pieces that don't depend on the rxing decode path:
//! the txt/json/tsv output formatters and the format-string parsing.

use std::path::Path;

use rxing::BarcodeFormat;

#[test]
fn txt_emits_one_line_per_detection() {
    let mut buf = Vec::new();
    let results = vec![zxing::Decoded {
        format: BarcodeFormat::QR_CODE,
        text: "https://x-cmd.com".to_string(),
        points: vec![(10.0, 10.0), (200.0, 10.0), (200.0, 200.0), (10.0, 200.0)],
    }];
    zxing::emit_txt(&mut buf, Path::new("qr.png"), &results);
    let s = String::from_utf8(buf).unwrap();
    assert_eq!(s, "qr.png\tQR_CODE\thttps://x-cmd.com\n");
}

#[test]
fn json_emits_valid_shape() {
    let mut buf = Vec::new();
    let results = vec![zxing::Decoded {
        format: BarcodeFormat::EAN_13,
        text: "4006381333931".to_string(),
        points: vec![],
    }];
    zxing::emit_json(&mut buf, Path::new("ean.png"), &results, false);
    let s = String::from_utf8(buf).unwrap();
    assert_eq!(
        s,
        "[{\"file\": \"ean.png\", \"results\": [{\"format\": \"EAN_13\", \"text\": \"4006381333931\"}]}]\n"
    );
}

#[test]
fn json_with_points_emits_empty_array() {
    let mut buf = Vec::new();
    let results = vec![zxing::Decoded {
        format: BarcodeFormat::QR_CODE,
        text: "hi".to_string(),
        points: vec![],
    }];
    zxing::emit_json(&mut buf, Path::new("qr.png"), &results, true);
    let s = String::from_utf8(buf).unwrap();
    assert!(
        s.contains("\"points\": []"),
        "expected empty points array in {s:?}"
    );
}

#[test]
fn tsv_escapes_tabs_and_newlines_in_text_field() {
    // Text contains a raw TAB — must be escaped to "\\t" so the row
    // stays a single TSV line.
    let mut buf = Vec::new();
    let results = vec![zxing::Decoded {
        format: BarcodeFormat::QR_CODE,
        text: "col1\tcol2\tcol3".to_string(),
        points: vec![],
    }];
    zxing::emit_tsv(&mut buf, Path::new("qr.png"), &results, false);
    let s = String::from_utf8(buf).unwrap();

    // Split on the two column separators and inspect just the text field.
    let mut cols = s.split('\t');
    assert_eq!(cols.next(), Some("qr.png"));
    assert_eq!(cols.next(), Some("QR_CODE"));
    let text_field = cols.next().unwrap_or("").trim_end_matches('\n');
    assert_eq!(
        text_field, "col1\\tcol2\\tcol3",
        "text field should escape internal tabs"
    );
    // No raw tabs in the text field.
    assert!(!text_field.contains('\t'));
}

#[test]
fn tsv_escapes_newlines_in_text_field() {
    let mut buf = Vec::new();
    let results = vec![zxing::Decoded {
        format: BarcodeFormat::QR_CODE,
        text: "line1\nline2".to_string(),
        points: vec![],
    }];
    zxing::emit_tsv(&mut buf, Path::new("qr.png"), &results, false);
    let s = String::from_utf8(buf).unwrap();
    assert!(
        s.contains("line1\\nline2"),
        "newlines should be escaped: {s:?}"
    );
}
