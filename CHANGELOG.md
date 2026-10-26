# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

**Output changes** (anything that changes the text `to_text`, `to_markdown`
or the chunks produce, what `main_content()` picks, or the parsed tree) ship
only in minor releases and are listed under their own heading, so you know
when to re-embed. See [Output stability](./README.md#output-stability).

## [Unreleased]

### Added
- **Reader mode on human-labelled benchmarks.** `conformance/` gains a
  `quality` binary that scores `main_content()` on the Zyte/ScrapingHub
  article extraction benchmark (AEB, 181 articles) and WCXB (2,008 pages in
  seven page types, dev and held-out test splits), each with its own scoring
  (verified against the benchmarks' `evaluate.py`), and writes
  [docs/reader-quality.md](docs/reader-quality.md) next to the published
  scores of other extractors; CI checks it. First numbers: AEB F1 0.898, WCXB
  0.738 (dev) / 0.803 (test). The Readability comparison
  ([docs/reader-mode.md](docs/reader-mode.md)) is now labelled as agreement.
- **Real-page parsing benchmark** next to astral-html, `tl` and `scraper`
  (`benchmarks/benches/real_pages.rs`), on the five pages astral-html
  publishes numbers for, with its two workloads. It corrects the README: on
  real pages domq is 5–6× faster than scraper but 25–48% slower than `tl`
  (the "ahead of `tl`" claim held only for generated inputs).
- **Output stability policy:** output changes only in minor releases, listed
  under "Output changes" ([README](README.md#output-stability)).
- CLI `--exit-status` to exit 1 when no results match; without it, no matches still exit 0.

### Output changes
- **`<noscript>` that holds the page is shown.** When a document has more
  text inside `<noscript>` than outside it (a page that renders with script
  and ships its content for clients without it, as Discourse forums do),
  `to_text`, `to_markdown`, link text and `main_content()` show that content,
  as a browser with scripting off does. An "enable JavaScript" notice next to
  real content stays hidden. WCXB forums 0.365 → 0.498, WCXB dev 0.738 →
  0.747.

## [0.5.0] — 2026-10-25

"Trust, measured": an MCP server for agents, and published numbers for how
close domq's trees are to a browser's and how well reader mode finds the
article, with the parser and the heuristic tuned against them.

**Upgrading from 0.4:** no API was removed. Trees, text and Markdown change
for some inputs, each time to match browsers (see "Changed"): mis-nested list
items, cells and paragraphs; character references without `;`; hidden
elements in `to_text`/`to_markdown`. Re-embed indexes built from 0.4's
Markdown if exact text matters. `ErrorKind` gains `MissingSemicolon`
(it is `#[non_exhaustive]`, so matches with a `_` arm keep compiling).

### Added
- **`domq mcp`: an MCP server for AI agents.** The CLI serves the Model
  Context Protocol over stdio with nine read-only tools: `markdown`, `text`,
  `select`, `extract`, `chunks`, `links`, `metadata`, `tables` and `lint`.
  They take the HTML as an argument (domq still never fetches URLs or reads
  files) and return JSON results as `structuredContent` with output schemas.
  Speaks MCP 2026-07-28 (per-request `_meta`, `server/discover`) and the
  handshake-based revisions back to 2024-11-05. Zero dependencies: the CLI
  carries its own small JSON parser. Set up with
  `claude mcp add domq -- domq mcp`; see `domq mcp --help`.

- **Conformance report.** `conformance/` runs the 1,959 HTML
  tree-construction tests (html5lib, now maintained in web-platform-tests)
  against domq and writes [docs/conformance.md](docs/conformance.md): pass
  rates per area for exact trees and for structure in domq's model, with the
  by-design gaps explained. CI regenerates it and fails when it changes, so
  every parser change shows its effect. First run: 0 panics, 4.1% exact, 55.5%
  same structure.

- **Reader-mode benchmark.** `conformance/` also scores `main_content()`
  and `metadata()` against Mozilla Readability's 130 labelled test pages and
  writes [docs/reader-mode.md](docs/reader-mode.md) (checked in CI like the
  conformance report). `--explain PAGE` shows what was picked and missed.

- **The guide**, a docs site next to the playground
  (<https://iamsaquib8.github.io/domq/docs/>, built with mdBook from `book/`):
  install, a quick start in Rust, the shell, JavaScript and Python, a
  cookbook (scraping, extraction, Markdown for LLMs and RAG, links, metadata
  and tables, editing, streaming, linting in CI), references for the CLI,
  selectors and the MCP server, and migration guides from htmlq, scraper,
  BeautifulSoup and cheerio. CI runs its Rust examples and checks the CLI
  page against `domq --help`.

### Output changes
Listed here in hindsight (the policy above starts with 0.6): re-embed
content indexed with 0.4 if exact text matters.
- **Tree:** mis-nested list items, cells and paragraphs close as in browsers
  (`<li><a>One<li><a>Two` gives two items), so text and Markdown from such
  pages change shape.
- **Text and Markdown:** character references without `;` decode
  (`&copy 2026` → `© 2026`); elements hidden with `hidden` or inline
  `display:none` / `visibility:hidden` are left out; the raw-text elements
  (`<xmp>`, `<iframe>`, `<noembed>`, `<noframes>`, `<plaintext>`) keep their
  content as text.
- **Reader mode:** `main_content()` picks differently on many pages (an
  `articleBody` first, div/span paragraphs counted, split articles kept
  whole).

### Changed
- **Better reader mode** (95.2% F1 against Readability, from 91.9%; the
  whole page scores 82.1%): a Schema.org `itemprop="articleBody"` is the
  first semantic hint, a block with no block inside it counts as a paragraph
  (`<div><span>…</span></div>`, as Readability counts it), and widening
  climbs over the rest of an article split into sibling blocks, tolerating a
  small ad slot between them.
- **`to_text` and `to_markdown` skip elements hidden** with the `hidden`
  attribute or an inline `display: none` / `visibility: hidden`, as a
  browser's `innerText` does (link text already did). Hidden text, a common
  place for prompt injections aimed at LLM agents, no longer reaches the
  output.
- **Trees closer to a browser's** (conformance: 55.5% → 61.8% same
  structure). Each change follows the HTML standard's recovery:
  - Implicit closes look past unclosed inline elements, within the
    standard's scopes: `<li><a>One<li><a>Two` gives two list items (it used
    to nest the second inside the first link), `<td><b>x<td>` two cells, and a
    block start tag closes an open `<p>` past inline elements
    (`<p><b>x<div>` puts the `<div>` after the paragraph).
  - Character references without their `;` decode where browsers decode
    them: numeric ones (`&#169 2026`) and the 106 legacy names (`&copy 2026`,
    `AT&ampT`), except in an attribute before `=` or an alphanumeric
    (`?a=1&copy=2` stays). They are reported as the new
    `ErrorKind::MissingSemicolon` (lint rule `missing-semicolon`) instead of
    `UnknownEntity`. The default table gains the 23 legacy names it lacked
    (`&AMP`, `&COPY`, …).
  - Comments: `<!-->` and `<!--->` are empty comments, `--!>` also closes
    one, a comment cut off at the end drops its trailing dashes, and `<?xml>`
    becomes the comment `?xml` (with its `?`). `</` at the end of the input
    is text, `</>` is dropped, and `</ x>` or `</#>` is a bogus comment.
  - `<xmp>`, `<iframe>`, `<noembed>` and `<noframes>` hold raw text like
    `<script>`, and everything after `<plaintext>` is its text.
  - A stray `</p>` inserts an empty paragraph, `</br>` is a `<br>`, and
    `<image>` is an `<img>` (outside SVG and MathML).

### Fixed
- Docs: the README's Cargo snippet asked for `domq = "0.3"`, which does not
  exist under the new name, and the npm README's CDN URL pointed at the wrong
  package.

## [0.4.0] — 2026-10-22

"Declarative scraping", and one name everywhere.

### Changed
- **Renamed to `domq`.** The crate is `domq` (was `dom-tree-rs`, imported as
  `domtree`), the CLI crate `domq-cli` with the binary `domq` (was
  `domtree-cli` / `domtree`), the npm package `domq-wasm` (was `dom-tree-wasm`; npm reserves plain `domq`), the
  PyPI package and Python import `domq` (was `dom-tree-rs` / `domtree`), and
  the Homebrew formula `iamsaquib8/tap/domq`. The repository moves to
  `github.com/iamsaquib8/domq`; the old packages get a final release pointing
  here.
- **MSRV raised to Rust 1.88** (from 1.85).
- **Python 3.11+** (3.9 and 3.10 are end of life); wheels are abi3 for
  CPython ≥ 3.11.
- `NodeRef::to_text()` / `to_markdown()` called *on* a hidden element
  (`<title>`, `<script>`, `<style>`, `<template>`, …) now return its content
  instead of `""`; hiding applies only to descendants. `to_text` gives a
  `<script>`/`<style>` source verbatim. Document-level output is unchanged.
- CI and release builds use the current Node LTS; the derive crate uses
  `syn` 3.

### Added
- **`#[derive(FromHtml)]`** (new `derive` feature, companion crate
  `domq-derive`): declarative extraction into structs with
  `#[html(select, attr, html, outer_html, markdown, default, with, skip)]`,
  `Option<T>`, `Vec<T>`, nested and generic structs, a struct-level
  `#[html(select = "…")]`, and compile-time diagnostics for misused
  attributes.
- `Dom::extract`, `Dom::extract_all`, `NodeRef::extract`, the
  `FromHtml` / `FromHtmlValue` traits, `Scope`, and `ExtractError`, whose
  messages name the field path, selector and source:
  `Product.reviews[2].stars (".stars" @data-value): cannot parse "x" as u8: …`.
  Zero-dependency and `no_std`; hand-written impls need no feature.
- **Python `domq.extract` / `extract_all`**: the same declarative extraction
  into dataclasses with `Annotated[T, Select(...)]` (`Attr`, `Html`,
  `OuterHtml`, `Markdown` shorthands), `Optional`, `list`, nested dataclasses,
  `default=`, `with_=`, a class-level `__select__`, and `ExtractError` with the
  same paths and messages as Rust.

## [0.3.0] — 2026-10-21

"One-line scraping": find things and get their content out, in Rust, in the
shell, in JavaScript and in Python. This release covers what the roadmap had
planned as phases 0.3 to 0.7, plus distribution (npm, PyPI, Homebrew, release
binaries) and the fixes from two independent reviews.

### Added
- **CSS selector engine v2** (`query` feature, now **on by default**):
  `+`/`~` combinators; every attribute operator (`^=` `$=` `*=` `~=` `|=`) with
  the `i`/`s` flags; `:first-child`, `:last-child`, `:only-child`,
  `:nth-child(An+B [of S])`, `:nth-last-child`, the `*-of-type` family,
  `:empty`, `:root`, `:scope`, `:not()`, `:is()`, `:where()`, `:has()` (with
  relative selectors), `:checked`/`:disabled`/`:enabled`, CSS escapes, and the
  non-standard `:contains("text")`. Matching never backtracks exponentially and
  never recurses on document depth.
- `Selector::parse` (and `FromStr`) — compile once, reuse, and get a
  `SelectorError` with a byte position instead of silently matching nothing.
  `Dom::select` accepts `&str`, `&String`, or `&Selector`.
- Scoped queries: `Dom::select_first`, `NodeRef::select`, `select_first`,
  `matches`, `closest`.
- `to_text()` on `Dom`/`NodeRef` — readable plain text (block elements become
  line breaks, whitespace collapsed, `<script>`/`<style>`/`<head>` dropped).
- `to_markdown()` (new `markdown` feature, on by default) — zero-dependency HTML
  → CommonMark + GFM: headings, emphasis, links, images, nested lists, code
  blocks with languages, blockquotes, tables.
- `NodeRef::inner_html`, `outer_html`, `next_element_sibling`,
  `prev_element_sibling`.
- Editing: `Dom::remove` (detach a subtree), `Dom::set_attr`, `Dom::remove_attr`.
- **`domtree` CLI** (`cargo install domtree-cli`) — htmlq-compatible flags
  (`-a`, `-t`, `-p`, `-w`, `-r`, `-b`, `-B`, `-f`, `-o`) plus `--markdown`,
  `--json`, `--count`, `--first`; zero dependencies. Prebuilt binaries are
  attached to GitHub releases.

- **`dom-tree-wasm` 0.3.0** (npm): `parse` now returns a typed object
  (`parseJson` keeps the string); new `select`, `selectFirst`, `toText`,
  `toMarkdown`, `toHtml`, `errors`, and a parse-once `Document` class. Invalid
  selectors throw a `SelectorError` with a position. Full TypeScript types,
  works in Node with no setup, ~41 KB gzipped. **Breaking:** `parseErrorCount`
  and `query` are removed, and `dom-node.d.ts` is replaced by the generated
  types.
- **Live playground** on GitHub Pages: paste HTML, try selectors, and see the
  tree, text, Markdown, JSON and repaired HTML, all running locally in wasm.
- `CODE_OF_CONDUCT.md`, `SECURITY.md`, `docs/ARCHITECTURE.md`, issue and PR
  templates.
- **Streaming events:** `domtree::events(html)` yields `Start` / `End`
  (including implied ends) / `Text` / `Comment` / `Doctype`, each with its byte
  range, applying exactly the same recovery as `parse` but without building a
  tree; `Events::errors()` matches `Dom::errors()`.
- **`simd` feature** (off by default): `memchr`-backed scanning, identical
  output, ~24% faster on text-heavy input. The default build stays
  zero-dependency.
- Benchmarks against `scraper` and `html5ever`, text- and table-heavy inputs,
  a memory report, and a weekly `Benchmarks` workflow; a `differential` fuzz
  target against html5ever.
- **Editing, part 2:** `Dom::set_text`, `rename`, `create_element`,
  `create_text`, `append_child`, `insert_before`, `wrap`, `unwrap`, and
  parsed-fragment insertion (`append_html`, `prepend_html`,
  `insert_html_before`, `insert_html_after`, returning the inserted ids). Moves
  never create cycles; `nodes()`, `select` and `find_by_*` stay in document
  order after any edit. `NodeRef::is_detached()`.
- **Source spans:** `NodeRef::source_range()` (byte range in the input) and
  `domtree::line_col(src, byte)`.
- **`entities-full` feature:** all 2,125 WHATWG named character references
  (off by default; the default table has the 178 most common).
- `Dom::main_content()` — a deterministic "reader mode" heuristic (semantic
  markup first, then Readability-style paragraph scoring) that picks the
  article and drops nav, sidebars, comments, ads and footers.
- `Dom::to_markdown_chunks(max_chars)` / `NodeRef::to_markdown_chunks` —
  heading-aware Markdown chunks for RAG (`Chunk { heading_path, markdown }`,
  `Chunk::context()`), never splitting inside code blocks or tables unless a
  single block exceeds the budget.
- `Dom::links(base)` (readable link text, resolved URLs, `<base href>`,
  `javascript:` skipped), `Dom::resources(base)` (every embedded URL incl.
  `srcset` candidates), `Dom::base_url(base)`.
- `Dom::metadata()` — title, description, canonical, lang, site name, image,
  author, published date, keywords, OpenGraph and Twitter fields, raw JSON-LD
  and feed links (`serde::Serialize` under `serde`).
- `NodeRef::table_rows()` (colspan/rowspan expanded, bounded on hostile input)
  and `domtree::to_csv` (RFC 4180).
- `domtree::resolve_url` / `resolve_srcset` — zero-dependency RFC 3986 and
  spec-compliant `srcset` resolution, moved into the library from the CLI.
- **Python package** `dom-tree-rs` on PyPI (import `domtree`): `parse`,
  `Document.select`/`select_first`/`to_text`/`to_markdown`/`to_html`/`to_json`,
  `Element` with attributes, text, HTML and traversal; typed stubs; abi3 wheels
  for Linux, macOS and Windows; the GIL is released while parsing large inputs.
- **Homebrew formula** for the `domtree` CLI (`packaging/homebrew/`), filled in
  with release checksums by the release workflow.
- **WASI build** of the CLI (`domtree.wasm`, attached to releases, smoke-tested
  under wasmtime), including quiet exit on a closed stdout pipe.
- **CLI structured extraction:** `domtree -e NAME=SELECTOR[@attr][[]]` builds a
  JSON object of named fields, or one object per match of a scope selector
  (JSON array, or `--jsonl`).
- **CLI output modes:** `--jsonl` (JSON Lines), `--csv` (tables as RFC 4180
  CSV), `--main` (reader-mode narrowing before selecting), `--links`
  (url/text, TSV or JSON), `--meta` (page metadata as JSON).
- **`domtree lint`:** images without `alt`, empty links and buttons, duplicate
  ids, skipped heading levels, and every recovered parse error, as
  `file:line:col: level: message [rule]` or `--format json`; exit 0/1/2 for CI.
  Omitted optional end tags are notes, missing required ones warnings.
- **CLI encoding detection:** BOM, `<meta charset>`, UTF-8, then Windows-1252;
  `--encoding` override.
- Prebuilt CLI binaries for Linux arm64.

### Changed
- Top-level nodes are now linked as siblings (`next_sibling`/`prev_sibling`),
  so sibling combinators and `:first-child` work on multi-root fragments.
- Numeric character references in `0x80–0x9F` are remapped per the WHATWG
  Windows-1252 table (`&#150;` → `–`), as browsers do; `&#0;`, surrogates and
  out-of-range values become U+FFFD.
- Default features are now `json`, `query`, `markdown` (still zero
  dependencies). Use `default-features = false` for the smallest build.
- **Faster parsing:** one attribute arena per document (nodes are now 52
  bytes), no per-tag allocations, and a word-at-a-time byte search. Parse time
  is 25–37% lower on markup and 69% lower on text-heavy pages; a parse makes
  ~20 allocations instead of thousands; peak memory is 10–14% lower, and unused
  arena capacity is released after parsing.
- More implicit-close rules (WHATWG optional tags): the full list of elements
  that close `<p>`; a heading closes an open heading; an unclosed `<head>` ends
  at the first body content (previously such pages lost their body in
  `to_text`); ruby `rb`/`rt`/`rtc`/`rp`; `<hr>` closes an open
  `option`/`optgroup`; captions, `colgroup` and `col` close open cells, rows
  and row groups.
- Nodes are 64 bytes (were 80); `NodeId` is non-zero, so `Option<NodeId>` is
  4 bytes.

- CLI: undeclared non-UTF-8 input now decodes as Windows-1252 (as browsers
  do) instead of lossy UTF-8, e.g. `caf\xe9` → `café`.

### Security / hardening
- Edited content can no longer inject markup on output: text set inside
  `<script>`, `<style>`, `<textarea>` or `<title>` cannot close the element
  early (a `</script` is written `<\/script`; parsed documents serialize
  byte-for-byte as before), and `set_attr`, `rename`, `create_element` and
  `wrap` refuse names that are empty or contain whitespace, quotes, `<`, `>`,
  `/`, `=` or NUL (tag names must start with an ASCII letter).
  `create_element` now returns `Option<NodeId>`.
- Attacker-chosen tag names could make `parse`/`events` quadratic (40,000
  colliding names: 6.4 s → 2 ms). An end tag now looks for its element among
  the 256 innermost and 8 outermost open elements; an element open only in
  between is left open and the end tag is reported as `StrayEndTag`.
- `resolve_url` reads `\` as `/` in http(s)/ftp/ws(s)/file URLs, as browsers
  do (`\\evil.com\x` resolves to evil.com, not a relative path), and
  `base_url` ignores a `<base>` inside `<template>`, `<svg>` or `<math>`.

### Fixed
- Linear-time fixes found in review: scoped selector queries
  (`tr.select(…)`, `matches`, `closest`) no longer pay for the whole document
  per call (160k rows: 18 s → 70 ms) and selector memo memory is bounded;
  `append_html` no longer slows with the parent's depth (3.4 s → 1.4 ms);
  chunk heading paths are capped at 256 characters per title; `table_rows` is
  linear on nested tables (a cell's text no longer includes tables nested in
  it — **behaviour change**); CLI `--links SELECTOR` and nested `-e` scopes
  are linear.
- `UnknownEntity` only reports WHATWG's 106 legacy names when their `;` is
  missing (so `x&order=1`, `?a=1&or` are clean), and its position points at
  the `&`, also inside attribute values.
- `colspan`/`rowspan` accept a leading `+` and skip only ASCII whitespace;
  `Link::text` leaves out content hidden with `hidden` or inline
  `display:none` / `visibility:hidden`.
- CLI: `lint` no longer reports correctly closed elements as unclosed
  (`</span foo>`, a final `</span` without `>`), and does report an outer
  element whose same-named child ends at the same spot; `-B` resolves
  `<base href>` against `--base` once; combining two output formats is a
  usage error; `--links -w` skips links without text.
- `Dom::errors()` no longer flags valid HTML: a bare `&` (`a & b`,
  `?a=1&b=2`) isn't an `UnknownEntity` (reference-shaped `&…` still is, with
  WHATWG's attribute-value exception), and an end tag closing elements whose
  end tag is optional (`<ul><li>a</ul>`) is `ImplicitlyClosed`, not
  `MismatchedEndTag`.
- Quadratic parsing of many unmatched end tags over a deep open-element stack,
  and quadratic entity decoding when the lookahead ended inside a multi-byte
  character.
- CLI `--base` keeps commas inside `srcset` URLs, drops empty candidates, and
  rewrites the first of a repeated attribute (the one that counts).

## [0.2.1] — 2026-06-24

### Added
- Zero-dependency HTML re-serialization (`Dom::to_html`, `NodeRef::to_html`) —
  round-trips a parsed tree back to HTML with correct escaping and void/raw-text
  handling. Always available (no feature required).
- `#![forbid(unsafe_code)]` — the crate contains no `unsafe`.

### Changed
- Moved to **edition 2024**; minimum supported Rust is now **1.85**
  (developed and tested on the latest stable).

## [0.2.0] — 2026-06-23

A full rewrite. The 0.1 line was a learning-project parser that panicked on
almost any real-world HTML; 0.2 is a robust, tiny, zero-dependency library.

### Added
- Error-tolerant parser that **never panics**; recovery diagnostics via
  `Dom::errors()`.
- Streaming, allocation-light `Tokenizer` (`domtree::tokenize`) implementing
  `Iterator<Item = Token>`.
- Arena-based tree with O(1) `parent`/`children`/`descendants`/`ancestors`/
  sibling traversal through a `Copy` `NodeRef` handle.
- `find_by_tag` / `find_by_id` / `find_by_class` helpers.
- Built-in, zero-dependency JSON serialization (`Dom::to_json`, `NodeRef::to_json`)
  behind the default `json` feature.
- Optional `serde::Serialize` (`serde` feature) producing the identical JSON shape.
- Optional CSS-subset selector engine (`Dom::select`, `query` feature):
  type/`*`/`#id`/`.class`/`[attr]`/`[attr=value]`, descendant and child
  combinators, selector lists.
- `no_std` + `alloc` support; new `std` feature for std-only conveniences.
- Coverage for void elements, self-closing tags, comments, doctype, CDATA,
  raw-text `<script>`/`<style>`, escapable raw text, boolean/unquoted attributes,
  hyphenated/namespaced/custom tags, HTML entities (named + numeric), and
  implicit closing for the common cases.
- Rebuilt WebAssembly bindings (`dom-tree-wasm`): ~19 KB gzipped, zero JS deps.
- `cargo-fuzz` harness, criterion benchmarks, and a full GitHub Actions CI.

### Changed
- **Breaking:** the public API is redesigned. The crate is now used as `domtree`;
  `parse` returns an owned `Dom`; `NodeRef::kind()` returns a borrowed `Copy`
  `NodeKind<'a>`. The old owned `Node`/`Props`/`NodeData` types are gone.
- `serde`/`serde_json` are no longer dependencies of the default build.

### Performance
- ~250 MiB/s on a synthetic article workload — within ~5–15 % of `tl` and
  roughly on par on large documents — while remaining zero-dependency.

## [0.1.1] — 2020
- Initial learning-project parser (HTML → debug tree / `serde_json` string).
