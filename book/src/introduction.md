# domq guide

**Messy HTML in. Clean tree, text, Markdown, or JSON out.**

domq is a forgiving HTML parser with CSS selectors: one small Rust crate with
zero dependencies, also available as a command-line tool, a WebAssembly module
for JavaScript, a Python package and an MCP server.

## What do you want to do?

Each task goes to the page that does it, with the code for every surface that
supports it (in italics).

- [Turn a page into Markdown for an LLM](cookbook/reader-mode.md)
  *(Rust, CLI, JavaScript, Python, MCP)*
- [Get the article text without the navigation and footer](cookbook/reader-mode.md#the-article-as-plain-text)
  *(Rust, CLI, JavaScript, Python, MCP)*
- [Pull fields from a list of products](cookbook/extracting.md)
  *(Rust, CLI, Python, MCP)*
- [Scrape links, tables or metadata](cookbook/links-metadata-tables.md)
  *(Rust, CLI, JavaScript, Python, MCP)*
- [Check HTML for accessibility problems in CI](cookbook/lint.md)
  *(CLI, MCP)*
- [Give an agent a page outline and selectors it can act on](mcp.md)
  *(MCP; the same outline from the CLI, Rust and Python)*
- [Replace `htmlq` or `pup` in a shell script](migrating/htmlq.md)
  *(CLI)*
- [Edit or clean HTML and write it back](cookbook/editing.md)
  *(Rust; the CLI can remove nodes)*
- [Process a huge dump without building a tree](cookbook/streaming.md)
  *(Rust)*
- [Run it in a Worker, Deno or the browser](install.md#javascript-and-webassembly)
  *(JavaScript)*

Not on the list? [Scrape elements and attributes](cookbook/scraping.md) is the
general case: select, read, repeat.

## Pick how you will use it

- [Quick start](quickstart.md): one task in Rust, the shell, JavaScript and
  Python, side by side.
- [Choose your surface](surfaces.md): when the library, the CLI,
  `domq-wasm`, the Python package or the MCP server fits, and what each can do.
- [Which API do I use?](which-api.md): text or Markdown or chunks, `find_by_*`
  or `select`, `parse` or `events` or `tokenize`, `parse` or `parse_bytes`.
- Coming from another tool? [htmlq](migrating/htmlq.md),
  [scraper](migrating/scraper.md), [BeautifulSoup](migrating/beautifulsoup.md)
  or [cheerio](migrating/cheerio.md).

Try it without installing anything in the
[playground](https://iamsaquib8.github.io/domq/).

## One engine, five ways in

The Rust crate has zero dependencies, and the same engine ships as:

- a **Rust library** (`no_std` + `alloc`, `#![forbid(unsafe_code)]`, never
  panics on any input);
- a **command-line tool**, `domq`, a drop-in replacement for `htmlq` with
  Markdown, JSON, CSV, extraction, reader mode and a linter on top;
- a **~57 KB (gzipped) WebAssembly module**, or ~78 KB with reader mode and
  the other extraction helpers (`domq-wasm/extract`), for browsers, Node, Deno, Bun and
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
- **Being small and boring to depend on.** One crate in your `cargo tree`, an
  about 1 s clean build, a static binary, a 57 KB wasm module (78 KB with extraction).

## What it is not

It is not a spec-compliant WHATWG parser: it does not insert `<html>`,
`<head>` and `<body>`, run the adoption agency algorithm or foster-parent
table content. How close its trees get to a browser's is
[measured](measured.md). It is not a sanitizer either: use
[`ammonia`](https://crates.io/crates/ammonia) or DOMPurify for untrusted HTML
that you render.
