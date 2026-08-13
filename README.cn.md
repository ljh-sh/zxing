# zxing

> 本地 QR + 一维条码解码 CLI。单文件静态二进制，零网络，AI-agent 友好。

`zxing` 是 [ZXing][zxing] 条码扫描算法的纯 Rust 封装（通过 [`rxing`][rxing] crate）。
解码 QR 码和 1D 条码（EAN-13、Code 128、UPC-A、Code 39 等），无需网络调用、无需
Python、无需 OpenCV。

```
$ zxing dec qr.png
qr.png   QR   https://x-cmd.com
```

[zxing]: https://github.com/zxing/zxing
[rxing]: https://github.com/rxing-core/rxing

## 为什么

现有的 `x qr dec` 走 `api.qrserver.com`（第三方服务，会看到你的图片）。
`zxing` 用本地二进制替换它：

- 图片 **绝不离机**；
- 延迟从 ~200ms 降到 ~30ms；
- 依赖面就一个 ~1.7 MB 静态二进制。

## 安装

```sh
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/ljh-sh/zxing/main/install.sh | sh

# 通过 x-cmd
x eget ljh-sh/zxing
```

预编译 binary: linux-musl (x86_64 + aarch64), macOS (x86_64 + aarch64),
Windows (x86_64 + aarch64)。每个 release artifact 都用 [cosign][cosign] 签名。

[cosign]: https://docs.sigstore.dev/

## 用法

```
zxing dec [OPTIONS] <IMAGE>...

OPTIONS:
    -f, --format <FMT>   输出格式: txt (默认) | json | tsv
        --try-harder     更努力地检测（默认开启）。
        --fast           关闭 try-harder（清洁图加速）。
        --only <FMT>     只搜索指定条码格式。
                         格式: qr, ean-13, ean-8, upc-a, upc-e,
                               code-128, code-39, code-93, codabar,
                               itf, pdf417, aztec, data-matrix, maxicode
        --points         JSON/TSV 输出包含四角点坐标。
    -0, --null           输入路径用 NUL 分隔。
        --files-from <P> 从文件（或 stdin '-'）读取路径列表（一行一个）。
        -q, --quiet       抑制 per-file stderr 错误日志（脚本友好）。
    -h, --help           显示帮助。
    -V, --version        显示版本。
```

### 示例

```sh
# 解码一个 QR
zxing dec photo.png

# 批量 + JSON 输出
zxing dec --format json img1.png img2.png

# 限制只扫 QR（更快）
zxing dec --only qr photo.png

# NUL 分隔文件列表（来自 find/xargs）
find . -name '*.png' -print0 | xargs -0 zxing dec --null --files-from -

# 从 stdin 读取文件列表（chardet 风格）
ls *.png | zxing dec --files-from -

# 包含角点坐标（用于裁剪 / 调试）
zxing dec --format json --points qr.png

# 静默模式（脚本用，只看退出码）
zxing dec --quiet bad.png || echo "没找到码"
```

### 输出格式

**txt**（默认；tab 分隔，每个 detection 一行）:

```
qr.png   QR   https://x-cmd.com
multi.png QR  hello world
multi.png EAN-13  4006381333931
```

**json**（每个文件一项）:

```json
[{"file": "qr.png", "results": [{"format": "QR_CODE", "text": "https://x-cmd.com"}]}]
```

加 `--points`:

```json
[{"file": "qr.png", "results": [{"format": "QR_CODE", "text": "https://x-cmd.com", "points": [[40.5, 40.5], [250.5, 40.5], [40.5, 250.5], [250.5, 250.5]]}]}]
```

**tsv**（无 header，三列）:

```
qr.png   QR   https://x-cmd.com
```

### 退出码

| 码 | 含义 |
|---|---|
| 0 | 至少一个 image 解码出至少一个条码 |
| 1 | 所有 image 都扫过，没找到条码（不是错误） |
| 2 | 读取失败或解码异常 |
| 64 | usage 错误（错 flag、缺 image 等） |

## 源码构建

```sh
git clone https://github.com/ljh-sh/zxing
cd zxing
cargo build --release
./target/release/zxing --version
```

跨编译（`cargo-zigbuild`）:

```sh
cargo zigbuild --release --target x86_64-unknown-linux-musl
cargo zigbuild --release --target aarch64-unknown-linux-musl
cargo zigbuild --release --target x86_64-apple-darwin
cargo zigbuild --release --target aarch64-apple-darwin
cargo zigbuild --release --target x86_64-pc-windows-gnu
cargo zigbuild --release --target aarch64-pc-windows-gnu
```

## 架构

```
src/
├── main.rs       — CLI 入口，手写参数解析
├── decode.rs     — 包装 rxing helpers + DecodeHints
│                   (filtered reader + try-harder + format filter)
└── format.rs     — txt / json / tsv 序列化，全手写
                    (不依赖 serde / json crate)
```

CLI 故意避开 `clap` / `serde` / `serde_json`，保持 binary 小且依赖面可审计。
参数解析 ~150 行；JSON 写 ~40 行。

## 协议

Apache-2.0。见 [LICENSE](LICENSE)。

包含来自:
- [`rxing`](https://github.com/rxing-core/rxing) (Apache-2.0)
- [`image`](https://github.com/image-rs/image) (MIT OR Apache-2.0)
- 原始 [ZXing](https://github.com/zxing/zxing) Java 库 (Apache-2.0)

## 相关

- [`ljh-sh/wxqr`](../wxqr) — 兄弟项目，用 WeChatCV 的 CNN-based QR 检测器处理
  退化 / 模糊 / 反光图像（zxing 在这些场景会失效）。
- `x-bash/qr` — x-cmd 模块，会把这个 binary 当本地 decode 后端（取代
  `api.qrserver.com`）。