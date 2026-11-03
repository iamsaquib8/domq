# domq

**Messy HTML in. Clean tree, text, Markdown, or JSON out.**

domq is a forgiving HTML parser with CSS selectors. It is one small Rust
crate with zero dependencies, and the same engine ships as:

- a **Rust library** (`no_std` + `alloc`, `#![forbid(unsafe_code)]`, never
  panics on any input);
- a **command-line tool**, `domq`, a drop-in replacement for `htmlq` with
  Markdown, JSON, CSV, extraction, reader mode and a linter on top;
- a **~65 KB (gzipped) WebAssembly module** for browsers, Node, Deno, Bun and
  edge workers (`npm install domq-wasm`);
- a **Python package** (`pip install domq`);
- an **MCP server** (`domq mcp`), so AI agents can turn the HTML they have into
  Markdown, fields, links or lint findings.

```sh
curl -sL https://blog.rust-lang.org | domq --main --markdown
```

## What it is good at

- **Getting content out.** Readable text, CommonMark/GFM Markdown, RAG chunks
  with heading paths, JSON, CSV for tables, page metadata, links.
- **Real CSS selectors.** Combinators, every attribute operator,
  `:nth-child(An+B of S)`, `:not()`, `:is()`, `:has()`, and jQuery's
  `:contains("text")`. Unsupported selectors are errors, never silently
  empty.
- **Messy input.** Unclosed tags, mis-nesting, stray end tags, unquoted
  attributes: repaired the way browsers repair the common cases, with every
  repair recorded as a diagnostic.
- **Being small and boring to depend on.** One crate in your `cargo tree`, a
  0.6 s clean build, a static binary, a 65 KB wasm module.

## What it is not

It is not a spec-compliant WHATWG parser: it does not insert `<html>`,
`<head>` and `<body>`, run the adoption agency algorithm or foster-parent
table content. How close its trees get to a browser's is
[measured](measured.md). It is not a sanitizer either: use
[`ammonia`](https://crates.io/crates/ammonia) or DOMPurify for untrusted HTML
that you render.

## Where to go next

- [Install](install.md) it for your language or shell.
- The [quick start](quickstart.md) shows the same task in Rust, the shell,
  JavaScript and Python.
- The [cookbook](cookbook/scraping.md) has recipes for scraping, extraction,
  LLM pipelines, editing and CI.
- Coming from another tool? See the guides for [htmlq](migrating/htmlq.md),
  [scraper](migrating/scraper.md), [BeautifulSoup](migrating/beautifulsoup.md)
  and [cheerio](migrating/cheerio.md).

Try it without installing anything in the
[playground](https://iamsaquib8.github.io/domq/).
