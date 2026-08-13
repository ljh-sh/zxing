# Roadmap

## Done

### v0.1.0 (in progress, 2026-08-14)

- `zxing dec [OPTIONS] <IMAGE>...` subcommand
- Pure-Rust via `rxing = "0.9"` + custom features (decoders, qrcode, oned,
  multi_barcode_readers, image, image_formats, encoding_rs)
- Hand-rolled CLI parser (no clap) + hand-rolled JSON / TSV writers (no
  serde) — keeps dependency surface minimal
- Hand-rolled output formatters: `txt` / `json` / `tsv` (`yml` alias)
- `--try-harder` (default on) + `--fast` (off)
- `--only <format>` filter (qr / ean-13 / ean-8 / upc-a / upc-e /
  code-128 / code-39 / code-93 / codabar / itf / pdf417 / aztec /
  data-matrix / maxicode)
- `--points` (include corner coordinates in JSON / TSV)
- `--null` / `--files-from <PATH|->` for chardet-style batch input
- `--quiet` for script-friendly use
- Exit codes: 0 / 1 / 2 / 64
- Local build: 1.7 MB static binary on aarch64-apple-darwin
- Verified on qrencode-generated test PNGs (including UTF-8 Chinese text
  and tab characters)
- README.md (English) + README.cn.md (中文)
- Apache-2.0 license

## Next

### v0.2.0 — encode + 1D focus

- **`zxing enc`** — pure-Rust encoder via the `qrcode` crate (5 KB dep);
  mirrors `qrencode` for QR and `zint` for 1D barcodes
- `--format <qr|ean-13|...>` flag for encoder (currently fixed QR)
- `--output <PATH>` to write PNG (via `image` crate)
- `--terminal` for half-block Unicode rendering in the terminal
  (mirrors `x qr enc` behaviour)
- WASM build target (`--features wasm` → `wasm32-unknown-unknown`)
- Streaming stdin for `dec` (raw RGB / RGBA bytes) — useful for piping
  from `x ffmpeg ...` pipelines
- Bench suite (criterion): easy / medium / hard corpus with N=5 runs

### v0.3.0 — production hardening

- 6-target release CI (linux-musl ×2 + macOS ×2 + win ×2) with cosign
  signing + OpenSSF Scorecard
- `cargo-deny` license / advisory gates
- `dependabot` weekly updates
- Multi-page TIFF / animated GIF input support
- SVG output for `enc` (mirror `qrencode -t SVG`)
- HEIC input (currently skipped; needs libheif)

### Deferred

- Live camera capture (`x zxing cam`): out of scope for a CLI — needs a
  separate tool
- Server mode / HTTP endpoint: out of scope — wrap with `x qr serve` if
  needed
- AR / mixed-reality: not planned

## Compatibility with `x qr`

`zxing` is designed as a drop-in backend for `x qr dec`. The output
schema (txt / json / tsv) is intentionally byte-compatible with
`ljh-sh/wxqr` so that `x qr dec` can dispatch to either binary without
per-result special-casing:

```
zxing  →  --format json  →  [{"file": "...", "results": [{"format": "...", "text": "...", "points": [...]}]}]
wxqr   →  --format json  →  [{"file": "...", "results": [{"format": "...", "text": "...", "points": [...]}]}]
```

The `format` field uses the `BarcodeFormat` enum variant name (`QR_CODE`,
`EAN_13`, `CODE_128`, etc.) so the consumer can switch on it directly.