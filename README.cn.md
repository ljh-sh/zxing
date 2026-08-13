# zxing

> 本地 QR + 一维条码解码 CLI。单文件静态二进制，零网络，AI-agent 友好。

`zxing` 是 [ZXing][zxing] 条码扫描算法的纯 Rust 封装（通过 [`rxing`][rxing] crate）。
解码 QR 码和 1D 条码（EAN-13、Code 128、UPC-A、Code 39 等），无需网络调用、无需
Python、无需 OpenCV。

```
$ zxing dec qr.png
qr.png   QR_CODE   https://x-cmd.com
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
    -0, --null           输入路径用 NUL 分隔。
        --files-from <P> 从文件（或 stdin '-'）读取路径列表（一行一个）。
        -q, --quiet       抑制 per-file stderr 错误日志（脚本友好）。
    -h, --help           显示帮助。
    -V, --version        显示版本。
```

`zxing dec <img>` 是规范形式 —— `dec` 是唯一的子命令（强制保留是为将来
可能的专用子命令留位）。

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

# 静默模式（脚本用，只看退出码）
zxing dec --quiet bad.png || echo "没找到码"
```

### 输出格式

JSON 输出里 `points` 字段总是出现（rxing 有就给，没有就是空数组）—— 没有
`--points` flag 切换。

**txt**（默认；tab 分隔，每个 detection 一行）:

```
qr.png   QR_CODE   https://x-cmd.com
multi.png QR_CODE   hello world
multi.png EAN_13   4006381333931
```

**json**（每个文件一项；`points` 总是存在）:

```json
[{"file": "qr.png", "results": [{"format": "QR_CODE", "text": "https://x-cmd.com", "points": [[40.5, 40.5], [250.5, 40.5], [40.5, 250.5], [250.5, 250.5]]}]}]
```

rxing 不暴露 corner points 的格式（部分 1D），`points` 渲染成 `[]`。

**tsv**（三列 —— 不带 points，没有干净的列形状）:

```
qr.png   QR_CODE   https://x-cmd.com
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
├── main.rs       — CLI 入口（薄壳，调 zxing::run()）
├── lib.rs        — pub mod cli / decode / format
├── cli.rs        — 手写参数解析 + 分发（无 clap，约 250 行）
├── decode.rs     — 包装 rxing helpers（filtered + multi-detect，
│                   DecodeHints、try-harder、format 过滤）
└── format.rs     — txt / json / tsv，全手写
                    (不依赖 serde / json crate)
```

CLI 故意避开 `clap` / `serde` / `serde_json`，保持 binary 小且依赖面可审计。

## 协议

Apache-2.0。见 [LICENSE](LICENSE)。

包含来自:
- [`rxing`](https://github.com/rxing-core/rxing) (Apache-2.0)
- [`image`](https://github.com/image-rs/image) (MIT OR Apache-2.0)
- 原始 [ZXing](https://github.com/zxing/zxing) Java 库 (Apache-2.0)

## 相关项目

- [`ljh-sh/wxqr`](../wxqr) — 兄弟项目，**用 CNN 检测器**
  （WeChatCV WeChatQRCode，OpenCV contrib）处理退化 / 模糊 / 反光
  图像（zxing 在这些场景会失效）。wxqr 是这个 CLI 返回 exit 1 时
  推荐的 fallback。详见
  [mneme/wxqr-design/README.md](https://github.com/ljh-sh/mneme/blob/main/wxqr-design/README.md)
  解释为什么两个项目分开而不是合并。
- `x-bash/qr` — x-cmd 模块，会用这两个 binary（zxing 快路径 +
  wxqr fallback）作为本地 decode 后端（取代 `api.qrserver.com`）。
  跟踪于 [x-cmd/x-cmd#467](https://github.com/x-cmd/x-cmd/issues/467)。

### zxing vs wxqr 对比

| | `ljh-sh/zxing` (本项目) | `ljh-sh/wxqr` |
|---|---|---|
| 算法 | ZXing (rxing) — 纯 Rust | WeChatCV WeChatQRCode — CNN (OpenCV) |
| 子命令 | `dec`（强制） | `decode` 默认（`wxqr <img>` 直接工作） |
| 格式覆盖 | QR + 1D (EAN/UPC/Code 128/…) | 仅 QR |
| 模型 | 无 —— 算法自身 | ~1 MB WeChatCV Caffe 模型内嵌 |
| 原生依赖 | 无 | OpenCV + bundled .dylib/.so |
| Binary 大小 | ~1.7 MB (linux-musl) | ~10 MB + ~50 MB bundled libopencv |
| 冷启动开销 | 无 | ~500 ms（模型加载） |
| 典型解码速度 | ~30 ms / 张 | ~30-300 ms / 张（CNN） |
| 最擅长场景 | 清洁 / 标准条码 | 模糊、反光、褶皱、极小、低对比度 |

两个后端输出 **字节兼容的 JSON**，调度层可以无差别扇出：

```sh
x qr dec <img>   # zxing 先试，wxqr 兜底 —— 见 x-cmd/x-cmd#467
```