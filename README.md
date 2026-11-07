<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/banner-dark.svg">
  <img src="docs/assets/banner-light.svg" width="100%" alt="domq — messy HTML in, clean tree, text, Markdown, or JSON out">
</picture>

A tiny, zero-dependency, never-panicking HTML parser with CSS selectors —
for Rust, your shell, JavaScript (a ~65 KB gzipped WebAssembly module) and Python.

[![crates.io](https://img.shields.io/crates/v/domq.svg?logo=rust)](https://crates.io/crates/domq)
[![npm](https://img.shields.io/npm/v/domq-wasm.svg?logo=npm)](https://www.npmjs.com/package/domq-wasm)
[![docs.rs](https://img.shields.io/docsrs/domq?logo=docs.rs)](https://docs.rs/domq)
[![CI](https://github.com/iamsaquib8/domq/actions/workflows/ci.yml/badge.svg)](https://github.com/iamsaquib8/domq/actions/workflows/ci.yml)
![dependencies](https://img.shields.io/badge/dependencies-0-success)
![no_std](https://img.shields.io/badge/no__std-yes-success)
![unsafe](https://img.shields.io/badge/unsafe-forbidden-success)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue)](#msrv)
[![license](https://img.shields.io/github/license/iamsaquib8/domq)](./LICENSE)

[**▶ Try it in your browser**](https://iamsaquib8.github.io/domq/) ·
[**Guide**](https://iamsaquib8.github.io/domq/docs/) ·
[Rust docs](https://docs.rs/domq) ·
[CLI](./cli) ·
[JavaScript](./wasm) ·
[Roadmap](./ROADMAP.md)

<img src="docs/assets/demo.svg" width="760" alt="Terminal demo: domq lists the navigation links of rust-lang.org, converts its first section to Markdown, counts images without alt text, and pretty-prints a fragment with unclosed list items.">

</div>

> **New name:** up to 0.3 this project was published as `dom-tree-rs` (crates.io,
> PyPI), `domtree-cli` and `dom-tree-wasm` (npm). From 0.4 it is **`domq`**
> everywhere; the old packages point here.

---

```rust
let dom = domq::parse(r#"<ul id=links><li><a href="/a">One</a><li><a href="/b">Two</a></ul>"#);

for a in dom.select("#links a[href]") {
    println!("{} -> {}", a.text_content(), a.attr("href").unwrap_or_default());
}
// One -> /a
// Two -> /b        (the missing </li>s were recovered)

let md = domq::parse("<h2>Notes</h2><p>Some <b>bold</b> text</p>").to_markdown();
assert_eq!(md, "## Notes\n\nSome **bold** text");
```

## Pick your entry point

<table>
<tr>
<th width="33%">🦀 Rust library</th>
<th width="33%">🖥️ Command line</th>
<th width="33%">🌐 JavaScript / WASM</th>
</tr>
<tr valign="top">
<td>

```toml
[dependencies]
domq = "0.6"
```

```rust
let dom = domq::parse(html);
dom.select_first("h1")
   .map(|h| h.to_text());
```

</td>
<td>

```sh
cargo install domq-cli
```

```sh
curl -sL rust-lang.org \
  | domq -t h1
```

</td>
<td>

```sh
npm install domq-wasm
```

```js
import init, { select }
  from "domq-wasm";
await init();
select(html, "a[href]");
```

</td>
</tr>
</table>

Also: 🤖 **AI agents** — `domq mcp` ([setup](#ai-agents-mcp)) · 🐍 **Python** —
`pip install domq` ([docs](./python)) · 🍺 **Homebrew** —
`brew install iamsaquib8/tap/domq` · ⚡ **WASI** — `domq.wasm` on every
[release](https://github.com/iamsaquib8/domq/releases).

## Why domq?

- **Zero dependencies.** One crate in your `cargo tree`. A clean build takes
  **0.6 s** — `scraper` pulls in 42 crates and takes 5.3 s.¹
- **Never panics.** Any byte sequence gives you a best-effort tree. Proptested and
  fuzzed: `parse`, `select`, `to_text`, `to_markdown`. Deep nesting can't blow the
  stack.
- **Real CSS selectors.** Combinators (` `, `>`, `+`, `~`), every attribute
  operator, `:nth-child()`, `:not()`, `:is()`, `:has()`, and the scraper's
  favourite, `:contains("text")`.
- **Content out, not just a tree.** Readable text (`to_text`), CommonMark/GFM
  Markdown (`to_markdown`) for LLM prompts and RAG pipelines, JSON, and HTML.
- **Runs everywhere.** `no_std + alloc`, `#![forbid(unsafe_code)]`, a ~65 KB
  gzipped wasm module for browsers, Cloudflare Workers, Deno, Bun and Node, and a
  single static binary for your shell.
- **Fast for an owned tree.** 5–6× faster than `scraper` on real pages, with
  ~20 allocations per parse. Speed specialists that do less work are faster:
  [`tl`][tl] by 1.3–1.5× and astral-html by 1.5–1.7× on real pages (see
  [Performance](#performance)).
- **Streams, too.** `domq::events` yields start/end/text events with the
  same recovery as the tree builder, for documents you don't want to keep.
- **Agent-ready.** `domq mcp` is an MCP server, so Claude, Cursor or any MCP
  client can turn the HTML it has into Markdown, fields or links.

<sub>¹ Clean debug `cargo build` of an empty crate depending only on the parser (default
features), Apple M4 Pro, 14 cores: domq 0.6 s · tl 0.4 s · html5ever 2.9 s ·
scraper 5.3 s · lol_html 6.9 s. Fewer cores widens the gap.</sub>

## Cookbook

<details open>
<summary><b>Scrape links, titles, and attributes</b></summary>

```rust
let dom = domq::parse(html);

let title = dom.select_first("title").map(|t| t.to_text());
let links: Vec<&str> = dom.select("a[href^='https://']").filter_map(|a| a.attr("href")).collect();
let price = dom.select_first(".product:has(.in-stock) .price").map(|p| p.to_text());
```
</details>

<details open>
<summary><b>Extract structs declaratively</b> (<code>derive</code> feature)</summary>

```rust
use domq::FromHtml;

#[derive(FromHtml)]
#[html(select = ".product")]
struct Product {
    #[html(select = "h2")] name: String,
    #[html(select = ".price", attr = "data-value")] price: f64,
    #[html(select = ".tag")] tags: Vec<String>,                // every match
    #[html(select = "a.more", attr = "href")] link: Option<String>, // None if missing
}

let products: Vec<Product> = domq::parse(html).extract_all()?;
```

Nested structs, `default`, `with = parse_fn`, and `html` / `outer_html` /
`markdown` sources are supported, and misused attributes are compile errors.
Runtime errors name the field and selector:
`Product.price (".price" @data-value): cannot parse "cheap" as f64: invalid float literal`.
</details>

<details>
<summary><b>Turn a web page into Markdown for an LLM</b></summary>

```rust
let dom = domq::parse(html);
let article = dom.select_first("article, main").map(|n| n.to_markdown());
```

Or from the shell, stripping the chrome first:

```sh
curl -sL https://blog.rust-lang.org | domq -r 'nav, footer, script' --markdown body
```
</details>

<details>
<summary><b>Keep just the article, chunked for RAG</b></summary>

```rust
let dom = domq::parse(html);
let article = dom.main_content();                   // reader mode: drops nav, ads, comments…
let chunks = match article {
    Some(node) => node.to_markdown_chunks(1500),
    None => dom.to_markdown_chunks(1500),
};
for chunk in &chunks {
    println!("[{}]\n{}", chunk.context(), chunk.markdown); // e.g. "[Guide > Install]"
}
```
</details>

<details>
<summary><b>Shape the Markdown: reference links, no images, absolute URLs</b></summary>

```rust
use domq::{ImageStyle, LinkStyle, MarkdownOptions};

let opts = MarkdownOptions::new()
    .link_style(LinkStyle::Reference)    // [text][1] … [1]: url (or Footer: a "### Links" list)
    .image_style(ImageStyle::AltOnly)    // keep the alt text, drop the image
    .drop_data_uris(true)                // no base64 blobs in the prompt
    .base_url("https://example.com/blog/");
let md = domq::parse(html).to_markdown_with(&opts);
```

Same options in the CLI (`--markdown-links reference --markdown-images alt`),
MCP (`links`, `images`, `drop_data_uris`) and Python (`to_markdown(links="reference")`).
</details>

<details>
<summary><b>A page outline an agent can act on</b></summary>

```rust
let outline = domq::parse(html).outline();
print!("{}", outline.to_text());
// [1] navigation
// [2]   link "Docs" -> nav > a
// [3] main
// [4]   heading 1 "Pricing"
// [5]   button "Start free trial" -> #trial
```

Landmarks, headings, links, buttons and form controls with their ARIA roles,
accessible names and a selector to act on. Also `domq --outline`, the MCP
`outline` tool and `doc.outline()` in Python.
</details>

<details>
<summary><b>Links, page metadata, and tables as data</b></summary>

```rust
let dom = domq::parse(html);

for link in dom.links(Some("https://example.com/blog/")) {
    println!("{} -> {}", link.text, link.url);       // resolved, honours <base href>
}

let meta = dom.metadata();                          // title, description, OpenGraph, JSON-LD…
println!("{:?} {:?}", meta.title, meta.canonical);

if let Some(table) = dom.select_first("table#prices") {
    let rows = table.table_rows();                  // colspan/rowspan expanded
    print!("{}", domq::to_csv(&rows));
}
```
</details>

<details>
<summary><b>Compile a selector once, reuse it, and get real errors</b></summary>

```rust
use domq::Selector;

let sel = Selector::parse("li:nth-child(odd) > a").expect("valid selector");
for page in pages {
    let dom = domq::parse(page);
    let count = dom.select(&sel).count();
}

let err = Selector::parse("a:hover").unwrap_err();
println!("{err}"); // unsupported pseudo-class ':hover' at byte 1
```
</details>

<details>
<summary><b>Walk the tree</b></summary>

```rust
let link = dom.select_first("a").unwrap();
let section = link.closest("section");          // nearest matching ancestor
let parent = link.parent();                     // O(1) — arena-backed
let next = link.next_element_sibling();
for node in link.ancestors() { /* … */ }
for node in dom.root().unwrap().descendants() { /* … */ }
```
</details>

<details>
<summary><b>Edit and re-serialize</b></summary>

```rust
let mut dom = domq::parse("<p>Hi <script>track()</script><a href=/x>x</a></p>");
let scripts: Vec<_> = dom.select("script").map(|n| n.id()).collect();
for id in scripts {
    dom.remove(id);
}
assert_eq!(dom.to_html(), r#"<p>Hi <a href="/x">x</a></p>"#);

// Insert parsed HTML, wrap, rename, set attributes and text:
let a = dom.select_first("a").unwrap().id();
dom.set_attr(a, "rel", "nofollow");
dom.insert_html_after(a, " <b>new</b>");
dom.wrap(a, "em");
let p = dom.root().unwrap().id();
dom.rename(p, "div");
assert_eq!(
    dom.to_html(),
    r#"<div>Hi <em><a href="/x" rel="nofollow">x</a></em> <b>new</b></div>"#
);
```
</details>

<details>
<summary><b>Point back into the source</b></summary>

```rust
let src = "<ul>\n  <li>one\n  <li>two</ul>";
let dom = domq::parse(src);
let li = dom.select("li").nth(1).unwrap();
let range = li.source_range().unwrap();              // byte range in `src`
assert_eq!(&src[range.clone()], "<li>two");
assert_eq!(domq::line_col(src, range.start), (3, 3)); // 1-based line, column
```
</details>

## Command line

`domq` is a maintained, zero-dependency, drop-in replacement for
[`htmlq`](https://github.com/mgdm/htmlq) — same flags, plus Markdown, JSON/JSON
Lines and CSV output, structured extraction, reader mode, encoding detection,
a linter, and modern selectors. [Full CLI docs →](./cli)

```sh
curl -s https://news.ycombinator.com | domq -t '.titleline > a'           # headlines
curl -sL https://www.rust-lang.org | domq -b https://www.rust-lang.org -a href 'nav a'
curl -sL https://blog.rust-lang.org | domq -r 'nav, footer' --markdown body # page → Markdown
domq -c 'img:not([alt])' < page.html                                       # a11y check
echo '<ul><li>a<li>b</ul>' | domq -p                                       # pretty-print

# Structured extraction: one JSON object per post
curl -sL https://blog.rust-lang.org | domq --jsonl -b https://blog.rust-lang.org/ \
  -e title=a -e url=a@href 'tr:has(a)'
# {"title":"Announcing Rust 1.99.1","url":"https://blog.rust-lang.org/2026/10/15/Rust-1.99.1/"}

# Reader mode → Markdown, tables → CSV, page metadata → JSON
curl -sL https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/ | domq --main --markdown
domq --csv 'table#prices' < report.html
domq --meta < page.html

# Lint HTML in CI (file:line:col, exit 1 on findings)
domq lint public/**/*.html
```

## AI agents (MCP)

`domq mcp` runs the CLI as a [Model Context Protocol](https://modelcontextprotocol.io)
server over stdio. An agent that has fetched a page hands its HTML to domq and
gets back what it needs, instead of reading raw markup:

| Tool | Returns |
|---|---|
| `markdown` | the page (or each match of a selector) as Markdown; `main` keeps just the article |
| `text` | readable text |
| `select` | elements matching a CSS selector: tag, attributes, text (and HTML) |
| `extract` | named fields as JSON, one object per item: `{"name": "h2", "url": "a@href"}` |
| `chunks` | Markdown chunks with their heading paths, for RAG |
| `outline` | landmarks, headings and controls with roles, names and selectors to act on |
| `links`, `metadata`, `tables` | links with resolved URLs; title, OpenGraph, JSON-LD…; tables as rows |
| `lint` | accessibility and markup problems, with line and column |

Every tool is read-only and works only on the HTML in its arguments: domq never
fetches URLs or reads files. It speaks MCP 2026-07-28 and the earlier
handshake-based revisions (2025-11-25 back to 2024-11-05).

```sh
claude mcp add domq -- domq mcp        # Claude Code
codex mcp add domq -- domq mcp         # Codex CLI
```

For Claude Desktop, Cursor, VS Code, Windsurf and other clients, add it to the
MCP config:

```json
{ "mcpServers": { "domq": { "command": "domq", "args": ["mcp"] } } }
```

## JavaScript & WebAssembly

The same parser — selectors, text, Markdown and all — in a ~65 KB (gzipped) wasm
module with zero JS dependencies and full TypeScript types. Works in the browser,
Cloudflare Workers, Deno, Bun, and Node (no setup needed in Node).
[Package docs →](./wasm) · [**Playground →**](https://iamsaquib8.github.io/domq/)

```js
import init, { parse, select, toMarkdown, mainContent, toMarkdownChunks, Document } from "domq-wasm";
await init(); // browsers and Workers; harmless in Node

const tree = parse("<ul><li>a<li>b</ul>");   // typed tree: { type: "document", children: [...] }
const links = select(html, "a[href]");       // [{ tag, attrs, text, html, nodeIndex }, ...]
const md = toMarkdown(html, "article");      // just the article, as Markdown
const article = mainContent(html)?.markdown; // reader mode: drops nav, ads, comments…
const chunks = toMarkdownChunks(html, 1500); // [{ markdown, headingPath, context }, ...]

const page = new Document(html);             // parse once, query many times
const headings = page.select("h2").map((h) => h.text);
page.free();
```

## Python

Same engine, Pythonic API, abi3 wheels for Linux, macOS and Windows (Python ≥ 3.11).
The GIL is released while parsing large documents. [Package docs →](./python)

```python
import domq

doc = domq.parse(html)
links = [a.attr("href") for a in doc.select("a[href]")]
price = doc.select_first(".product:has(.in-stock) .price").text
markdown = doc.select_first("article").to_markdown()   # ready for an LLM prompt
article = doc.main_content()                           # reader mode, or None
chunks = doc.to_markdown_chunks(1500)                  # [Chunk(markdown, heading_path, context)]
meta = doc.metadata()                                  # title, OpenGraph, JSON-LD…

# Declarative extraction into dataclasses (mirrors Rust's #[derive(FromHtml)])
from dataclasses import dataclass
from typing import Annotated, Optional
from domq import extract_all, Select, Attr

@dataclass
class Product:
    __select__ = ".product"
    name: Annotated[str, Select("h2")]
    price: Annotated[float, Attr(".price", "data-value")]
    link: Annotated[Optional[str], Attr("a.more", "href")]

products = extract_all(Product, html)
```

## How it compares

| | domq | [tl] | [scraper] | [html5ever] | [lol_html] |
|---|:---:|:---:|:---:|:---:|:---:|
| Transitive dependencies | **0** | 0 | 42 | 19 | 41 |
| CSS selectors | **✅** | limited | ✅ | ❌ | ✅ |
| Text / Markdown output | **✅** | ❌ | ❌ | ❌ | ❌ |
| JSON output | **✅** | ❌ | ❌ | ❌ | ❌ |
| WASM + npm package | **✅** | ❌ | ❌ | ❌ | ❌ |
| CLI | **✅** | ❌ | ❌ | ❌ | ❌ |
| `no_std` | **✅** | ❌ | ❌ | ❌ | ❌ |
| Edit + re-serialize | **✅** | ❌ | ❌ | ✅ | ✅ (streaming) |
| Streaming events with recovery | **✅** | ❌ | ❌ | ✅ (`TreeSink`) | ✅ |
| Never panics on input | ✅ | ✅ | ✅ | ✅ | ✅ |
| WHATWG spec-compliant tree | ❌ | ❌ | ✅ | ✅ | ❌ |

**When to pick something else:** if you need byte-exact browser parsing, use
[`html5ever`][html5ever]/[`scraper`][scraper]. If raw throughput is all that matters, use [`tl`][tl]. If
you're sanitizing untrusted HTML, use [`ammonia`](https://crates.io/crates/ammonia)
— never a forgiving, non-spec parser.

[tl]: https://crates.io/crates/tl
[scraper]: https://crates.io/crates/scraper
[html5ever]: https://crates.io/crates/html5ever
[lol_html]: https://crates.io/crates/lol_html

## Performance

Fast *for an owned tree*: full recovery, entity decoding, source spans and
all. **Real pages** (the five astral-html publishes numbers for, parsed into
each library's document; GitHub Actions, AMD EPYC 7763; µs, lower is better;
see [benchmarks/](./benchmarks)):

| page | domq | astral-html | [tl] | [scraper] |
|---|---:|---:|---:|---:|
| iniconfig (6 KB) | 6.5 | **5.2** | 7.3 | 62.0 |
| Bootstrap dashboard (22 KB) | 56.1 | **36.7** | 43.1 | 350 |
| Rust Book chapter (89 KB) | 224 | **141** | 171 | 1,469 |
| PEP 8 (122 KB) | 489 | **319** | 390 | 2,826 |
| PEP index (419 KB) | 1,707 | **995** | 1,155 | 9,545 |

On real pages domq is 5–6× faster than `scraper`, and `tl` and astral-html
are faster than domq (closing that gap is on the [roadmap](./ROADMAP.md)).
**Generated inputs** (Apple M4 Pro, `cargo bench`):

| input | domq | [tl] | [scraper] | [html5ever]¹ |
|---|---:|---:|---:|---:|
| small (0.5 KiB) | **400 MB/s** | 395 MB/s | 53 MB/s | 48 MB/s |
| medium (21 KiB) | **491 MB/s** | 462 MB/s | 60 MB/s | 53 MB/s |
| large (420 KiB) | **512 MB/s** | 358 MB/s | 61 MB/s | 54 MB/s |
| table-heavy (712 KiB) | **422 MB/s** | 326 MB/s | 58 MB/s | 51 MB/s |
| text-heavy (547 KiB) | 7.7 GB/s | **8.5 GB/s** | 1.9 GB/s | 1.6 GB/s |

On these generated documents domq is about 8× faster than
`scraper`/`html5ever`, ahead of `tl` on large markup, and ~10% behind it on
prose (`tl` borrows from the input and skips entity decoding and recovery);
the real pages above are the fairer comparison. A parse makes ~20 allocations
in total, and the optional `simd` feature cuts text-heavy parse time by
another ~24%. Run them yourself: `cd benchmarks && sh fetch-pages.sh && cargo bench`.

<sub>¹ into its reference `RcDom`.</sub>

## What it handles

Void elements, self-closing tags, comments (including browsers' recovery of
`<!-->` and `--!>`), doctypes, CDATA, raw text (`<script>`, `<style>`, `<xmp>`,
`<iframe>`, `<noembed>`, `<noframes>`, `<plaintext>`), escapable raw text
(`<title>`/`<textarea>`), boolean and unquoted attributes,
hyphenated/namespaced/custom tags, named + numeric entities (with browser-style
Windows-1252 remapping, and a missing `;` recovered where browsers do:
`&copy 2026`, `&#169`), and implicit closing as browsers scope it: a `<li>`,
`<dd>`/`<dt>`, table cell or row closes its predecessor even past unclosed
inline elements (`<li><a>One<li><a>Two`), a block closes an open `<p>` past them
(`<p><b>x<div>`), a stray `</p>` makes an empty paragraph, `</br>` is a `<br>`,
a `<a>`, `<button>`, `<nobr>` or `<select>` closes an open one instead of
nesting in it, a `</h3>` closes an open `<h2>`, and an end tag does not reach
past a table cell, caption or table (a stray `</div>` in a `<td>` is ignored).
Everything it recovers from is recorded in
[`Dom::errors`](https://docs.rs/domq/latest/domq/struct.Dom.html#method.errors).
How close the trees are to a browser's is measured in
[docs/conformance.md](./docs/conformance.md).

**Deliberately not done:** full WHATWG tree construction (adoption agency and
reopening formatting elements, automatic `<html>`/`<head>`/`<body>`, foster
parenting, namespaces). See the [non-goals](./ROADMAP.md#non-goals).

## Cargo features

| feature | default | what it adds |
|---|:---:|---|
| `json` | ✅ | `to_json()` — zero-dependency JSON |
| `query` | ✅ | CSS selectors: `select`, `select_first`, `Selector`, `closest`, `matches` |
| `markdown` | ✅ | `to_markdown()` and `to_markdown_chunks()` — zero-dependency HTML → Markdown |
| `derive` | | `#[derive(FromHtml)]` — declarative extraction into structs (adds the `domq-derive` proc-macro crate) |
| `entities-full` | | all 2,125 WHATWG named character references (default: the 178 most common) |
| `simd` | | `memchr`-backed scanning (~24% faster on text-heavy pages; adds `memchr`) |
| `serde` | | `serde::Serialize` for the tree (same JSON shape; adds `serde`) |
| `std` | | `std::error::Error` impls; the crate is `no_std + alloc` otherwise |

## MSRV

Rust **1.88** (edition 2024), enforced in CI. Raised only in minor releases.

## Output stability

If you embed or index domq's output, re-embedding should be planned, not a
surprise. The text it produces (`to_text`, `to_markdown`,
`to_markdown_chunks`, the choice `main_content()` makes, and the same output
from the CLI, the MCP tools, JavaScript and Python) changes **only in minor
releases** (0.5 → 0.6), and every such change is listed under an **"Output
changes"** heading in the [changelog](./CHANGELOG.md). Patch releases keep it
identical, except to fix a panic or a hang. The parsed tree follows the same
rule, so selectors keep matching what they matched.

## Quality

Property-tested ("never panics" + tree invariants), snapshot-tested against a
corpus of messy HTML, fuzzed with `cargo-fuzz`, and CI-checked for fmt, clippy,
`no_std`, MSRV, zero dependencies and a wasm size budget.

**Measured against browsers:** CI runs the 1,959 HTML tree-construction tests
(html5lib, in web-platform-tests) and publishes the results in
[docs/conformance.md](./docs/conformance.md): 0 panics, and 61.8% of trees
match a browser's structure in domq's model (no implied `<html>`/`<body>`),
with the gaps explained area by area.

**Reader mode on human-labelled benchmarks**
([docs/reader-quality.md](./docs/reader-quality.md)), scored with each
benchmark's own method:

| | AEB (181 articles) | WCXB dev / test (2,008 pages, 7 types) |
|---|---:|---:|
| **domq `main_content()`** | **0.898** | **0.747 / 0.803** |
| trafilatura (Python) | 0.958 | 0.791 / 0.833 |
| Readability.js | 0.947 | 0.675 / 0.726 |
| dom_smoothie (Rust) | 0.865 | 0.762 / 0.808 |

F1, published scores for the others. Strongest on articles and documentation
(0.90 on WCXB dev), weakest on listings, products and forums; about 1–3 ms per
page.
Agreement with Readability itself is tracked in
[docs/reader-mode.md](./docs/reader-mode.md).

## Contributing

Issues and PRs are welcome! Start with [CONTRIBUTING.md](./CONTRIBUTING.md), the
[architecture guide](./docs/ARCHITECTURE.md), and the [roadmap](./ROADMAP.md) —
issues labelled **good first issue** are a friendly way in. Please follow the
[code of conduct](./CODE_OF_CONDUCT.md), and report security problems privately
as described in [SECURITY.md](./SECURITY.md).

If domq saved you some time, a ⭐ helps other people find it.

## License

[MIT](./LICENSE)
