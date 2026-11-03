# Install

## Rust

```toml
[dependencies]
domq = "0.6"
```

Default features are `json`, `query` (CSS selectors) and `markdown`; all of
them are zero-dependency. Optional features:

| Feature | Adds |
|---|---|
| `derive` | `#[derive(FromHtml)]` for declarative extraction (pulls in `syn` at compile time only) |
| `serde` | `serde::Serialize` for the tree, metadata and chunks |
| `simd` | `memchr`-accelerated scanning (one `no_std` dependency, same output) |
| `entities-full` | all 2,125 named character references (default: the ~200 common ones) |
| `std` | `std::error::Error` impls |

The minimum supported Rust version is **1.88**.

**Output stability:** the text domq produces (`to_text`, `to_markdown`,
chunks, what `main_content()` picks) changes only in minor releases, listed
under "Output changes" in the
[changelog](https://github.com/iamsaquib8/domq/blob/main/CHANGELOG.md), so
re-embedding an index is something you plan.

## Command line

```sh
brew install iamsaquib8/tap/domq   # Homebrew (macOS, Linux)
cargo binstall domq-cli            # prebuilt binary
cargo install domq-cli             # from source
```

Prebuilt binaries for Linux (x86_64, arm64, statically linked), macOS (Intel,
Apple Silicon) and Windows, plus a WASI build (`wasmtime domq.wasm …`), are
attached to every [GitHub release](https://github.com/iamsaquib8/domq/releases)
with a `SHA256SUMS` file.

## JavaScript and WebAssembly

```sh
npm install domq-wasm
```

Node 22+, Bun and Deno use it directly (`import { select } from "domq-wasm"`).
Browsers and bundlers call `await init()` once first. Without a bundler:

```html
<script type="module">
  import init, { toText } from "https://cdn.jsdelivr.net/npm/domq-wasm@0.6/domq_wasm.js";
  await init();
  console.log(toText("<p>Hello <b>world</b>"));
</script>
```

## Python

```sh
pip install domq
```

Wheels for Python 3.11+ on Linux, macOS and Windows (abi3, so one wheel per
platform covers every version).

## MCP server

The CLI is the server: install it, then register `domq mcp` with your agent.

```sh
claude mcp add domq -- domq mcp
```

See [AI agents (MCP)](mcp.md) for other clients.
