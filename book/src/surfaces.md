# Choose your surface

domq is one engine with five ways in. They parse the same way and select the
same way, so pick the one that is closest to where your HTML already is.

| If you are... | Use | Install |
|---|---|---|
| writing Rust | [the library](#rust) | `cargo add domq` |
| in a shell script or a pipeline | [the command line](#command-line) | `brew install iamsaquib8/tap/domq` |
| writing JavaScript for Node, Bun, Deno, a Worker or a browser | [`domq-wasm`](#javascript) | `npm install domq-wasm` |
| writing Python | [the Python package](#python) | `pip install domq` |
| building an agent that already has the HTML | [the MCP server](#mcp-server) | `claude mcp add domq -- domq mcp` |

Every example below does the same job: list the `href` of every link in a
fragment. [Install](install.md) has the details for each line above.

## Rust

**Use it when** you are pulling data out of HTML inside a Rust program: a
scraper, a crawler, a feed reader, a test that asserts on rendered pages. It
is the only surface with `#[derive(FromHtml)]`, the streaming `events` API,
the tokenizer and in-place editing with `NodeId`s. Dependency-free and
`no_std` (with `alloc`).

```rust
# extern crate domq;
# let html = "<a href=/a>One</a> <a href=/b>Two</a>";
let dom = domq::parse(html);
let hrefs: Vec<_> = dom.select("a[href]").filter_map(|a| a.attr("href")).collect();
assert_eq!(hrefs, ["/a", "/b"]);
```

Next: [Scrape elements and attributes](cookbook/scraping.md), then [Which API do I use?](which-api.md).

## Command line

**Use it when** the job is a pipeline: `curl ... | domq ...`, a cron job, a
CI step, a replacement for `htmlq` or `pup` (the flags are `htmlq`'s). It also
has Markdown, JSON Lines, CSV, field extraction, reader mode and `domq lint`.
It does not keep a tree between invocations, so work that needs several
passes over one page belongs in a library.

```sh
printf '<a href=/a>One</a> <a href=/b>Two</a>' | domq -a href a
# /a
# /b
```

Next: [Command line](cli.md) (every flag), [From htmlq](migrating/htmlq.md).

## JavaScript

**Use it when** you have no DOM: Node, Bun, Deno, Cloudflare Workers, or a
browser where you want to parse HTML that is not the current page. It is the
same engine compiled to WebAssembly: a core entry (`domq-wasm`) for parsing,
selecting and converting, and `domq-wasm/extract` for reader mode, metadata,
links, chunks and tables. There is no `outline`, no `lint` and no
`#[derive]`-style extraction in JavaScript.

```js
import { select } from "domq-wasm";

select(html, "a[href]").map((a) => a.attrs.href);
// ["/a", "/b"]
```

Next: [Install](install.md#javascript-and-webassembly), [From cheerio](migrating/cheerio.md).

## Python

**Use it when** the surrounding code is Python: scraping, data cleaning, RAG
ingestion. Wheels are prebuilt for Python 3.11+. It reads everything the library
reads (selectors, reader mode, chunks, tables, outline) and extracts into
dataclasses with `Select`; it does not edit documents or stream events.

```python
import domq

[a.attr("href") for a in domq.parse(html).select("a[href]")]
# ['/a', '/b']
```

Next: [Pull fields from a list of items](cookbook/extracting.md#python-dataclasses), [From BeautifulSoup](migrating/beautifulsoup.md).

## MCP server

**Use it when** an AI agent has fetched a page and should read it without
spending its context on raw markup. `domq mcp` is the CLI running as a
[Model Context Protocol](https://modelcontextprotocol.io) server: the agent
passes the HTML as an argument and gets Markdown, fields, links, an outline or
lint findings back. Every tool is read-only and never fetches a URL.

```sh
claude mcp add domq -- domq mcp
```

The agent then calls tools such as `select` with arguments like
`{"html": "<a href=/a>One</a> <a href=/b>Two</a>", "selector": "a[href]"}`, and
gets:

```json
{"count": 2, "truncated": false, "matches": [
  {"tag": "a", "attrs": {"href": "/a"}, "text": "One"},
  {"tag": "a", "attrs": {"href": "/b"}, "text": "Two"}]}
```

Next: [AI agents (MCP)](mcp.md).

## What each surface can do

Read from the Rust API, the CLI's `--help`, the MCP tool list, the JavaScript
`.d.ts` files and the Python `.pyi` stub. A dash means it is not there.

Every function of the two packages: [JavaScript API](javascript.md) and
[Python API](python.md).

| | Rust | CLI | JavaScript | Python | MCP |
|---|---|---|---|---|---|
| CSS selectors | `select` | `domq SELECTOR` | `select` | `select` | `select` |
| Readable text, Markdown | `to_text`, `to_markdown` | `-t`, `-m` | `toText`, `toMarkdown` | `to_text`, `to_markdown` | `text`, `markdown` |
| Reader mode (the article) | `main_content` | `--main` | `mainContent` (extract) | `main_content` | `main: true` |
| RAG chunks | `to_markdown_chunks` | - | `toMarkdownChunks` (extract) | `to_markdown_chunks` | `chunks` |
| Metadata, links | `metadata`, `links` | `--meta`, `--links` | `metadata`, `links` (extract) | `metadata`, `links` | `metadata`, `links` |
| Tables, CSV | `table_rows`, `to_csv` | `--csv` | `tables`, `toCsv` (extract) | `tables`, `to_csv` | `tables` |
| Declarative extraction | `#[derive(FromHtml)]` | `-e NAME=SPEC` | - | `Select` dataclasses | `extract` |
| Page outline for agents | `outline` | `--outline` | - | `outline` | `outline` |
| Lint | - | `domq lint` | - | - | `lint` |
| Edit and serialize | `remove`, `set_attr`, ... | `-r` (remove) | - | - | - |
| Streaming events | `events`, `tokenize` | - | - | - | - |
| Bytes in, encoding detection | `parse_bytes` | automatic, `--encoding` | `parseBytes`, `decode` | `parse(bytes)` | - |

## Not sure?

- Which function do I call for text, Markdown or chunks, and when do I use
  `events` instead of `parse`? [Which API do I use?](which-api.md)
- Starting from a job rather than a tool? Go back to
  [What do you want to do?](introduction.md#what-do-you-want-to-do).
