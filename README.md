# zxing

> Local QR + 1D barcode decoder CLI. Single static binary, zero network,
> AI-agent friendly.

`zxing` is a pure-Rust wrapper around the [ZXing][zxing] barcode-scanning
algorithm via the [`rxing`][rxing] crate. It decodes QR codes and 1D
barcodes (EAN-13, Code 128, UPC-A, Code 39, etc.) from images — no
network calls, no Python, no OpenCV.

```
$ zxing dec qr.png
qr.png   QR_CODE   https://x-cmd.com
```

[zxing]: https://github.com/zxing/zxing
[rxing]: https://github.com/rxing-core/rxing

## Why

The existing `x qr dec` flow routes through `api.qrserver.com` (a
third-party service that sees your images). `zxing` replaces that with a
local binary so:

- the image **never leaves your machine**;
- latency drops from ~200ms to ~30ms;
- the dependency surface is one ~1.7 MB static binary.

## Install

```sh
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/ljh-sh/zxing/main/install.sh | sh

# via x-cmd
x eget ljh-sh/zxing
```

Pre-built binaries: linux-musl (x86_64 + aarch64), macOS
(x86_64 + aarch64), Windows (x86_64 + aarch64). Every release artifact
is signed with [cosign][cosign].

[cosign]: https://docs.sigstore.dev/

## Usage

```
zxing dec [OPTIONS] <IMAGE>...

OPTIONS:
    -f, --format <FMT>   Output format: txt (default) | json | tsv
        --try-harder     Try harder to detect (default: enabled).
                         Slower but picks up damaged / small / rotated codes.
        --fast           Disable try-harder for faster scanning on clean images.
        --only <FMT>     Only search for the given barcode format.
                         Formats: qr, ean-13, ean-8, upc-a, upc-e,
                                  code-128, code-39, code-93, codabar,
                                  itf, pdf417, aztec, data-matrix, maxicode
    -0, --null           Treat input as NUL-separated path list.
        --files-from <P> Read paths from a file ('-' for stdin); one per line.
        -q, --quiet       Suppress per-file stderr error logs (script-friendly).
    -h, --help           Show this help.
    -V, --version        Show version.
```

`zxing dec <img>` is the canonical form — `dec` is the only subcommand
(kept mandatory so future specialised subcommands can coexist
without breaking the dispatch).

### Examples

```sh
# Decode one QR code
zxing dec photo.png

# Batch decode with JSON output
zxing dec --format json img1.png img2.png

# Limit scan to QR only (faster)
zxing dec --only qr photo.png

# NUL-separated file list from find/xargs
find . -name '*.png' -print0 | xargs -0 zxing dec --null --files-from -

# File list from stdin (chardet-style)
ls *.png | zxing dec --files-from -

# Quiet mode for scripts (exit code only)
zxing dec --quiet bad.png || echo "no code found"
```

### Output formats

The `points` field is always emitted in JSON output (when available
from rxing) — there is no `--points` flag to toggle it on.

**txt** (default; tab-separated, one line per detection):

```
qr.png   QR_CODE   https://x-cmd.com
multi.png QR_CODE   hello world
multi.png EAN_13   4006381333931
```

**json** (one entry per file; `points` is always present):

```json
[{"file": "qr.png", "results": [{"format": "QR_CODE", "text": "https://x-cmd.com", "points": [[40.5, 40.5], [250.5, 40.5], [40.5, 250.5], [250.5, 250.5]]}]}]
```

When rxing doesn't expose corner points (some 1D formats), `points`
renders as `[]`.

**tsv** (three columns — points omitted, no clean column shape):

```
qr.png   QR_CODE   https://x-cmd.com
```

### Exit codes

| code | meaning |
|---|---|
| 0 | at least one image decoded at least one barcode |
| 1 | all images scanned, no barcodes found (not an error) |
| 2 | read failure or decode exception |
| 64 | usage error (bad flag, missing image, etc.) |

## Build from source

```sh
git clone https://github.com/ljh-sh/zxing
cd zxing
cargo build --release
./target/release/zxing --version
```

Cross-compile (via `cargo-zigbuild`):

```sh
cargo zigbuild --release --target x86_64-unknown-linux-musl
cargo zigbuild --release --target aarch64-unknown-linux-musl
cargo zigbuild --release --target x86_64-apple-darwin
cargo zigbuild --release --target aarch64-apple-darwin
cargo zigbuild --release --target x86_64-pc-windows-gnu
cargo zigbuild --release --target aarch64-pc-windows-gnu
```

## Architecture

```
src/
├── main.rs       — CLI binary (thin shim that calls zxing::run())
├── lib.rs        — pub mod cli / decode / format; public API
├── cli.rs        — Hand-rolled arg parser + dispatcher
│                   (no clap; ~250 lines, fully explicit)
├── decode.rs     — Wraps rxing helpers (filtered + multi-detect readers,
│                   DecodeHints, try-harder, format filters)
└── format.rs     — txt / json / tsv emitters, all hand-written
                    (no serde / json crate dependency)
```

The CLI deliberately avoids `clap` / `serde` / `serde_json` to keep the
binary small and the dependency surface auditable.

## License

Apache-2.0. See [LICENSE](LICENSE).

Includes code from:
- [`rxing`](https://github.com/rxing-core/rxing) (Apache-2.0)
- [`image`](https://github.com/image-rs/image) (MIT OR Apache-2.0)
- Original [ZXing](https://github.com/zxing/zxing) Java library (Apache-2.0)

## Related projects

- [`ljh-sh/wxqr`](../wxqr) — sibling project that **uses a CNN
  detector** (WeChatCV WeChatQRCode, OpenCV contrib) for degraded
  / blurry / reflective images where zxing falls short. wxqr is the
  recommended fallback when this CLI returns exit 1. See
  [mneme/wxqr-design/README.md](https://github.com/ljh-sh/mneme/blob/main/wxqr-design/README.md)
  for why the two projects are kept separate rather than merged.
- `x-bash/qr` — the x-cmd module that will use both binaries
  (`zxing` fast-path + `wxqr` fallback) as the local decode backend
  (replacing `api.qrserver.com`). Tracked in
  [x-cmd/x-cmd#467](https://github.com/x-cmd/x-cmd/issues/467).

### zxing vs wxqr at a glance

| | `ljh-sh/zxing` (this) | `ljh-sh/wxqr` |
|---|---|---|
| Algorithm | ZXing (rxing) — pure Rust | WeChatCV WeChatQRCode — CNN (OpenCV) |
| Subcommand | `dec` (mandatory) | `decode` is implicit (`wxqr <img>` works directly) |
| Format coverage | QR + 1D (EAN/UPC/Code 128/…) | QR only |
| Models | none — algorithm only | ~1 MB WeChatCV Caffe models bundled |
| Native deps | none | OpenCV + bundled .dylib/.so |
| Binary size | ~1.7 MB (linux-musl) | ~10 MB + ~50 MB bundled libopencv |
| Cold-start cost | none | ~500 ms (model load) |
| Typical decode | ~30 ms / image | ~30-300 ms / image (CNN) |
| Best at | clean / standard barcodes | blurry, reflective, wrinkled, tiny, low-contrast |

Both backends emit **byte-compatible JSON**, so a dispatcher can fan
out without per-result special-casing:

```sh
x qr dec <img>   # zxing first, wxqr fallback — see x-cmd/x-cmd#467
```