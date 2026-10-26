# Roadmap

This is a direction, not a promise: order and scope will move with what people
actually report and ask for. It is organised around **the people who reach for
a tiny HTML parser** and the job each of them is trying to finish, not around
parser internals. Issues and PRs that fit the principles below are very welcome.

**Where we are:** 0.5.0 shipped on 2026-10-25 on crates.io, npm, PyPI,
Homebrew and the release page. It added `domq mcp` (an MCP server for agents),
browser-style recovery for the common mis-nesting cases, and published numbers:
**61.8%** of the 1,959 html5lib tree-construction tests match a browser's tree
structure (0 panics), and reader mode scores **0.898 F1 on the AEB article
benchmark** and **0.747 / 0.803 on WCXB** (dev / test), against human labels
([docs/reader-quality.md](docs/reader-quality.md)). The reports regenerate in
CI on every PR.
The [guide](https://iamsaquib8.github.io/domq/docs/) covers every surface. See
[what works where](#what-works-where), [what shipped](#shipped) and the
[changelog](./CHANGELOG.md).

## Who it's for

1. **Rust developers pulling data out of HTML** `rust` — scrapers, crawlers,
   feed readers, link checkers, test assertions on rendered pages. They want
   the answer in one line, fast compiles, and no 40-crate tree for a two-line
   job.
2. **Shell users** `shell` — `curl … | domq 'a.title' -a href`. `htmlq` (last
   release 2022) and `pup` (2016) are unmaintained; `domq` is a maintained
   replacement for both, flag-compatible with `htmlq`.
3. **LLM, RAG and agent pipelines** `ai` — turn a page into clean Markdown or
   chunks for a prompt or an index, at the edge, on a server or as an agent's
   MCP tool, without a headless browser.
4. **JavaScript on the edge** `js` — Workers, Deno, Bun, the browser: no DOM
   needed, and a ~43 KB gzipped wasm module.
5. **Python scraping and data code** `py` — where most scraping and RAG code
   lives; a fast, never-panicking parser with selectors and Markdown.
6. **People who ship HTML** `ship` — docs sites, static-site generators, email
   templates: `domq lint` in CI catches missing alt text, empty links,
   duplicate ids and broken markup with `file:line:col`.

Every item below should make at least one of these people's day better, and is
tagged with who it's for. If it doesn't help anyone, it probably doesn't
belong.

## What works where

What each surface can do today, and the release that's planned to fill each
gap (— means no plan yet).

| | Rust | CLI | MCP | JavaScript | Python |
|---|---|---|---|---|---|
| CSS selectors | ✅ | ✅ | ✅ | ✅ | ✅ |
| readable text, Markdown | ✅ | ✅ `-t` `-m` | ✅ | ✅ | ✅ |
| Markdown / text options (link style, images, frontmatter, LLM preset) | 0.7 | 0.7 | 0.7 | 0.7 | 0.7 |
| reader mode (main content) | ✅ | ✅ `--main` | ✅ | 0.6 | 0.6 |
| article metadata and a confidence score | 0.7 | 0.7 | 0.7 | 0.7 | 0.7 |
| RAG chunks | ✅ | 0.7 `--chunks` | ✅ | 0.6 | 0.6 |
| metadata (OpenGraph, JSON-LD, …) | ✅ | ✅ `--meta` | ✅ | 0.6 | 0.6 |
| links and resources | ✅ | ✅ `--links` | ✅ | 0.6 | 0.6 |
| tables, CSV | ✅ | ✅ `--csv` | ✅ | 0.6 | 0.6 |
| declarative extraction | ✅ derive | ✅ `-e` | ✅ | 0.6 | ✅ dataclasses |
| bytes in, encoding detection | 0.6 | ✅ | — | 0.6 | 0.6 |
| page outline for agents (roles, names, refs) | 0.8 | 0.8 | 0.8 | 0.8 | 0.8 |
| editing | ✅ | `-r` only | — | 0.11 | 0.11 |
| streaming events | ✅ | 0.11 `--stream` | — | 0.11 | 0.11 |
| accessibility and markup lint | 0.10 | ✅ | ✅ | 0.10 | 0.10 |
| parse errors with positions | ✅ | via lint | via lint | ✅ | ✅ |

## Principles

Every addition is weighed against what defines the project:

1. **Tiny** — small code, especially in WASM.
2. **Zero-dependency by default** — new dependencies must be optional, behind a
   feature or in a separate crate.
3. **Forgiving** — never panic, never hang; recover from real-world mess.
4. **Honest** — no over-claiming. Numbers in the README are reproducible, and
   what we don't do is written down.

If a feature can't keep all four, it goes behind a feature flag, into a
companion crate, or onto the non-goals list.

## Budgets

Numbers we hold ourselves to (✅ = enforced in CI):

| budget | limit | today |
|---|---|---|
| runtime dependencies, default build | **0** ✅ | 0 |
| gzipped wasm, all bindings | **< 45 KiB** ✅ | ~42.9 KB (0.5.0: +1.5 KB browser-style recovery, +0.4 KB reader tuning) |
| panics or hangs on any input (HTML, selectors, edits) | **none** ✅ proptest + fuzz | none known |
| stack use on deeply nested input | **bounded** ✅ 100k-deep tests | bounded |
| super-linear time on adversarial input | **none** — every fix gets a timing test | none known |
| clean debug build of the library | **< 1 s** (M-series) | 0.55 s |
| allocations per parse | **O(1)**, not O(nodes) | ~20 |
| MSRV | **Rust 1.88** ✅ (raised only in minor releases, to versions ≥ ~6 months old) | 1.88 |
| HTML → Markdown time | to be published (0.9), against mdream and htmd on the same pages | not measured |
| CLI binary size, wheel size | to be set (0.9) | ~0.9 MB CLI (Apple Silicon) |

The wasm budget leaves little room for bringing reader mode, chunks and the
data helpers to JavaScript (0.6). Rather than raise it, those may ship as a
second, optional wasm module, so the core stays small for people who only
select and convert.

## Non-goals

Deliberately out of scope. Reach for the right tool instead:

- **Full WHATWG tree construction** (adoption agency, automatic
  `<html>`/`<head>`/`<body>`, foster parenting). For browser-identical trees,
  use [`html5ever`](https://crates.io/crates/html5ever). We *measure* how far
  from the spec we are (see 0.5) rather than pretend.
- **Security sanitization.** A forgiving, non-spec parser disagrees with
  browsers on edge cases, which is exactly what mutation-XSS exploits. Use
  [`ammonia`](https://crates.io/crates/ammonia).
- **Rendering.** No layout, no CSS cascade, no `:hover`, no JavaScript. If a
  page builds its content with script, you need a browser.
- **Networking.** Neither the library, the CLI nor the MCP server fetches
  URLs; pipe from `curl`, or pair `domq mcp` with a fetch tool. The MCP server
  reads only the HTML it is given and, once `--allow-read` lands (0.5.x),
  files under a directory you name.
- **Speed at any cost.** We stay among the fastest owned-tree parsers, but we
  won't trade safety, correctness or size for a benchmark. If raw speed over a
  borrowed tree is your only metric, use [`tl`](https://crates.io/crates/tl).

---
- **Machine-learned extraction in the core** (small models such as
  ReaderLM-v2 or MinerU-HTML, or a trained page classifier). domq exposes a
  confidence score (0.7) as the hook for an LLM or model fallback instead.
- **Other document formats** (PDF, Office, RSS/Atom, Djot output). Use
  markitdown or docling; feeds and Djot belong in companion crates, if
  anywhere.

## The landscape (October 2026)

A full comparison with sources is in [docs/landscape.md](docs/landscape.md).
In short:

- **Where domq leads:** one zero-dependency, never-panicking core that does
  selectors, reader mode, Markdown, chunks, metadata, links, tables, lint and
  MCP, on every surface, in ~43 KB of gzipped wasm; the only forgiving parser
  with a published conformance report; an MCP server with no network or file
  access; the maintained successor to htmlq and pup.
- **Where it's behind:** reader-mode quality is measured only as agreement
  with Readability (competitors publish on the human-labelled AEB and WCXB
  corpora); no Markdown options; no confidence score; metadata agreement
  under 50% on titles, bylines and dates; Python and JS parity.
- **Who to watch:** mdream (zero-dependency Rust Markdown engine behind
  Cloudflare's `/markdown`, with an LLM preset), turbohtml and justhtml
  (Python), xberg html-to-markdown (16 bindings and Agent Skills for coding
  agents), rs-trafilatura (best published Rust extraction quality), defuddle,
  and Cloudflare's Markdown for Agents.

The releases below are shaped by this: honest quality numbers first, then
the bindings, then Markdown built for models, then what agents need to act
on a page without a browser.

## How releases are ordered

**By how many people an item unblocks, then by size.** Releases are themed
around one group of users each, so each release finishes a job for someone
instead of being half of several. Small items that don't need a release of
their own live in the [anytime lane](#anytime--small-and-welcome); they land
in whichever release is next when someone picks them up, and most are good
first issues. Every item is tagged with its size and with who it's for:

- **S**: small, a contained change in one area.
- **M**: medium, touches several areas or a binding.
- **L**: large, new infrastructure, a new crate, or a research-style project.

## Ongoing — Launch feedback and honest numbers

The first months after the announcement decide whether this becomes a tool
people rely on.

- [ ] **S** Triage every report within 48 hours; small HTML repros become corpus
      fixtures and regression tests.
- [ ] **S** Patch releases for any panic, hang, wrong match or wrong output, in
      lock-step across crates.io, npm, PyPI and Homebrew.
- [ ] **S** Turn recurring questions into README cookbook entries and doc
      examples.
- [ ] **S** Keep 5–10 `good first issue`s labelled from the
      [anytime lane](#anytime--small-and-welcome) (7 open:
      [#22](https://github.com/iamsaquib8/domq/issues/22)–[#28](https://github.com/iamsaquib8/domq/issues/28)).
- [x] **M** `ai` **Honest quality numbers before quality claims:**
      `main_content()` on the Zyte/ScrapingHub article-extraction benchmark
      (AEB, 181 pages, MIT) and WCXB (2,008 pages in 7 page types,
      CC-BY-4.0), scored like their own `evaluate.py` (verified to match) and
      checked in CI ([docs/reader-quality.md](docs/reader-quality.md)):
      **AEB 0.898**, **WCXB 0.738 dev / 0.803 test** at first (0.747 dev
      after the `<noscript>` fix below), ~1–3 ms/page, next to
      the published scores (rs-trafilatura 0.970 / 0.859, Readability.js
      0.947 / 0.675, dom_smoothie 0.865 / 0.762). The Readability table is
      now labelled *agreement*.
- [ ] **M** `ai` Close the gaps WCXB shows, tuning on the dev split only.
      Done: `<noscript>` that holds a page's content (Discourse forums) is
      shown, forums 0.365 → 0.498. Next: listings (0.466), products,
      collections and service pages, where one container holds only part of
      the content; and AEB precision (0.828; recall is already 0.981).
- [x] **S** `rust` Re-measure the "ahead of `tl`" claim against astral-html
      on its real-page corpus. It doesn't hold: on real pages domq is 5–6×
      faster than scraper but 25–48% slower than `tl` and slower than
      astral-html everywhere (`benchmarks/benches/real_pages.rs`). The README
      now says so.
- [x] **S** `ai` An output-stability policy for `to_text`, `to_markdown` and
      chunks: output changes only in minor releases, under an "Output changes"
      heading in the changelog, so re-embedding an index is planned rather
      than a surprise ([README](./README.md#output-stability); 0.5.0's
      changes are listed that way in hindsight).

## 0.5.x — "Agents that don't pay for markup"

`domq mcp` saves an agent from reading raw markup, but only if the agent
doesn't have to *write* that markup first. Today every tool takes the page as
an `html` argument, so the agent re-types the whole page as output tokens:
tens of thousands for an ordinary article, which is slower and costlier than
reading the HTML. This release makes the MCP server cheap to use on real pages.

- [x] **L** `ai` `domq mcp`: an MCP server over stdio exposing select, text,
      Markdown, chunks, links, metadata, tables, extract and lint on HTML the
      agent provides (still no fetching). Speaks MCP 2026-07-28 and the
      handshake-based revisions back to 2024-11-05. *Shipped in 0.5.0.*
- [ ] **M** `ai` `domq mcp --allow-read DIR`: every tool accepts a `path`
      instead of `html`, restricted to files under DIR (no `..` or symlink
      escapes), off by default. Documented recipe: a fetch tool that saves the
      page to disk, then domq on the file.
- [ ] **S** `ai` Bounded responses on every tool: `max_chars` and an
      `offset` on `markdown` and `text` (returning `truncated` and
      `next_offset`, as the official fetch server's `max_length` /
      `start_index` do), a cursor for `chunks`, `select`, `links` and
      `tables`, and an `estimated_tokens` field, so one big page can't flood
      the context window.
- [ ] **S** `ai` `domq mcp` in the official MCP Registry (a `server.json`
      published from the release workflow) and the MCP client directories.
- [ ] **M** `ai` `shell` Install with tools people already have: the native
      `domq` binary in a PyPI package and in npm packages with per-platform
      binaries (the ruff / biome model; names to be confirmed), so
      `uvx … mcp` and `npx -y … mcp` work in MCP client configs and shell users
      get a one-line install on every OS.
- [ ] **S** `ai` Input budgets: `ParseOptions { max_input_bytes, max_nodes,
      max_depth }` with a `truncated` flag, and defaults in `domq mcp`, so one
      hostile page can't exhaust a server (Readability's `maxElemsToParse`,
      lol_html's `MemorySettings`).
- [ ] **S** `ai` Agent Skills and one-click installs: a `SKILL.md` that
      teaches Claude Code, Codex, Cursor and Gemini CLI to use the `domq` CLI
      directly (no MCP needed), and install links for MCP clients that support
      them. xberg's html-to-markdown already ships skills.

## 0.6 — "Where people are: Python and JavaScript"

Reader mode, chunks and the data helpers are what personas 3–5 come for, and
today they exist only in Rust, the CLI and MCP. In Python, turbohtml and
justhtml now pitch the same "replace readability + markdownify" story; in JS,
defuddle and mdream. Bring everything to the bindings before anything else.

- [ ] **M** `py` Python: `main_content()`, `metadata()`, `links()` /
      `resources()`, `to_markdown_chunks()`, `table_rows()` and CSV,
      `resolve_url`, and reusable `Selector` objects.
- [ ] **S** `py` `js` `rust` Bytes in, everywhere: `domq::decode(&[u8])` in
      the library (the CLI's sniffing: BOM, `<meta charset>`, UTF-8,
      Windows-1252, reporting which rule decided), `domq.parse(bytes)` in
      Python (`requests`' `r.content`, Scrapy's `response.body`) and
      `Uint8Array` in JS (cheerio's `loadBuffer`).
- [ ] **M** `js` JavaScript: `mainContent()`, `metadata()`, `links()`,
      `toMarkdownChunks()` and `tables()`, inside the size budget, as a
      second, optional wasm module if the core can't hold them.
- [ ] **S** `js` `extract(html, spec)`: cheerio's `extract()` descriptor maps
      (a selector string, `[selector]` for all matches, `{selector, value}`,
      nested maps), so cheerio code ports one-to-one.
- [ ] **S** `py` `shell` `ai` Scrapy-style pseudo-elements: `a::attr(href)`
      and `h2::text` in `select` (Python, CLI, MCP), labelled non-standard
      like `:contains`, so parsel selectors work unchanged.
- [ ] **M** `py` `ai` Integrations: a LangChain document transformer and a
      LlamaIndex reader (in a companion package, or as documented recipes),
      and tables into pandas.
- [ ] **S** `py` More wheels: musllinux x86_64 / aarch64 (Alpine images),
      Windows arm64, free-threaded CPython 3.14t; and conda-forge.
- [ ] **S** `js` Publish to JSR for Deno; Cloudflare Workers and Deno
      examples, and when to use domq rather than Workers' `HTMLRewriter`.
- [ ] **M** `py` `js` Benchmarks where people choose: selectolax's
      754-domain harness for Python (selectolax, lxml, BeautifulSoup,
      html5-parser, justhtml, turbohtml) and htmlparser-benchmark for JS
      (htmlparser2, node-html-parser, parse5, cheerio), plus gzipped bundle
      size and cold start in Workers, Deno and Bun.
- [ ] **M** Playground "copy as Rust / JS / Python / CLI" for the current
      selector, so it doubles as a code generator.
- [ ] **M** More package managers: Scoop / winget (Windows), Nix, AUR, and a
      tiny Docker image.

## 0.7 — "Markdown built for models"

LLM and RAG pipelines are the fastest-growing group, and every converter they
compare us with lets them shape the output. One options type, exposed the same
way in every surface (library, CLI flags, MCP arguments, JS, Python).

- [ ] **M** `ai` `MarkdownOptions` / `to_markdown_with()`: link style (inline,
      reference, or a links footer like Jina's `X-With-Links-Summary`),
      images (keep, alt text only, drop; drop `data:` URIs), heading, bullet,
      emphasis and fence style, tags to skip or keep as HTML, absolute URLs
      from a base, wrap width, maximum blank lines. Folds in the old "links
      as footnotes" item. (turndown, htmd, markdownify, html2text.)
- [ ] **S** `ai` An LLM preset and a token estimate: `--llm` drops empty and
      redundant links, self-link headings, fragment-only links, tracking
      parameters (utm_*, fbclid, gclid), SVG and data URIs, and keeps image
      alt text; `--tokens` and an `estimated_tokens` field, labelled a
      heuristic. (mdream's minimal preset, Cloudflare's `x-markdown-tokens`.)
- [ ] **S** `ai` `shell` YAML frontmatter: `--frontmatter`,
      `to_markdown_document()`, MCP `frontmatter: true` (title, canonical,
      author, date, description, language, site, image, word count).
- [ ] **M** `ai` A standardization pass before conversion: code-block
      languages from Prism, Shiki, highlight.js and `data-lang` (line-number
      gutters stripped), footnotes to `[^n]`, MathJax/KaTeX to `$…$`, lazy
      images (`data-src`, `<noscript>`) to `src`, heading permalinks dropped,
      a first H1 that repeats the title de-duplicated, callouts to
      `> [!NOTE]`. (defuddle, readabilityrs.) Scored with WebMainBench's
      code, formula and table subset.
- [ ] **M** `ai` `rust` `Dom::article()`: the main node with a 0–1
      confidence score and reasons, `is_probably_readable()`, and the
      article's title (site suffix stripped), byline (also from "By …" text),
      date, excerpt, language, word count and reading time. CLI
      `--main --json`; `--main` exits non-zero below a threshold if asked.
      Absorbs the metadata-vs-Readability item; dates measured on htmldate's
      1,000-page set.
- [ ] **M** `ai` Extraction knobs on reader mode: a content hint or root,
      include and exclude selectors, a precision / recall bias, keep or drop
      comments, tables and images, and a debug log of what was removed and
      why. (defuddle, trafilatura, legible.)
- [ ] **S** `ai` Token-aware chunking with your own tokenizer
      (`to_markdown_chunks_by(max, |s| tokens(s))`), overlap, stable chunk ids
      for incremental re-indexing, table headers repeated in every chunk of a
      split table, small sibling chunks merged; CLI `--chunks --jsonl`.
      (docling's HybridChunker, mdream's splitter.)
- [ ] **M** `ai` Query-focused relevance: rank chunks or prune a page for a
      question (`--query "…"`, BM25 plus density filters, no stemming by
      default), so an agent reads only what answers it. (Crawl4AI's
      `fit_markdown`.)
- [ ] **M** `ai` `rust` Output mapped back to the source: `to_text_with_map()`
      and per-chunk source ranges, for citations and highlighting. domq
      already records spans; inscriptis and turbohtml offer annotated text.
- [ ] **M** `shell` `ai` `to_text` options: wrap width, tables as aligned
      grids (colspan-aware), form fields as `[ value ]`, a `noscript` toggle.
- [ ] **M** `rust` Custom Markdown handlers for chosen tags (closures), and
      keep / skip lists in the bindings.
- [ ] **M** `ai` Comments and replies extracted separately from the article.

## 0.8 — "Agents that act on the page"

Browser-free agents still need to *find* things: the login form, the price,
the "next page" link. Give them the page's structure the way an accessibility
tree does, and selectors they can reuse.

- [ ] **L** `ai` `ship` A page outline: `Dom::outline()`, `domq --outline`
      and an MCP `outline` tool listing landmarks, headings, links, buttons
      and form controls with their implicit ARIA roles and accessible names,
      each with a short ref that resolves to a unique selector and a source
      span. URLs are returned by ref, so a model never writes one it could
      hallucinate (Stagehand's approach). Static semantics only: no CSS, no
      script, said plainly.
- [ ] **S** `ai` `rust` `shell` `NodeRef::unique_selector()` (preferring `id`,
      `data-testid` and stable classes over `:nth-child`), `find_text("Add to
      cart")`, and `domq --selector-for TEXT`; MCP results carry selectors.
- [ ] **M** `ai` `shell` `py` `js` Schema files: `domq -E spec.json` and the
      same schema in MCP `extract`, JS and Python: a base selector, typed
      fields (text, attribute, HTML, Markdown, number, boolean, date), nested
      and list fields, defaults, sibling and parent navigation (`+ tr`),
      trimming and number parsing. An agent writes the schema once and reruns
      it with no LLM. (Crawl4AI's JsonCss, orf/hq, xidel.)
- [ ] **M** `ai` `rust` Complete structured data: Microdata and RDFa next to
      JSON-LD, with typed views for Article, Product/Offer, Recipe,
      BreadcrumbList, FAQPage and Event. (xberg, turbohtml, Firecrawl's
      `product` format.)
- [ ] **S** `ai` Forms: `Dom::forms()` with action, method, fields, labels,
      types and current values, for agents that fill them in elsewhere.

## 0.9 — "Trust, compared"

The remaining measurement work, against the tools people would otherwise pick.

- [ ] **L** `rust` Reopen formatting elements (a bounded "adoption agency
      lite"): `<p><b>x<p>y` keeps `y` bold, `<b><i>x</b>y</i>` keeps `y`
      italic. The weakest area in the conformance report (26.7%).
- [ ] **M** `ai` Reader mode and Markdown next to trafilatura, Readability.js,
      defuddle, markdownify, turndown, htmd and mdream on the same corpora
      (AEB, WCXB, trafilatura's set, WebMainBench), published either way.
- [ ] **M** `rust` Close the real-page parsing gap: profile domq on the five
      real pages (25–48% behind `tl`, more behind astral-html) and fix what
      generated inputs hid (attribute-heavy markup, scripts, long URLs).
- [ ] **M** `rust` `shell` Speed where people choose: HTML → Markdown against
      mdream, htmd and html-to-markdown-rs on mdream's pages; parsing against
      astral-html, tl, lol_html, html5gum and scraper on real pages; reader
      mode in ms/page against dom_smoothie and rs-trafilatura; the CLI
      (hyperfine) against htmlq, pup, xq and the Markdown CLIs; tokens saved,
      counted with a fixed offline tokenizer.
- [ ] **S** `rust` A robustness matrix: panics and timeouts on adversarial
      inputs (100k-deep nesting, attribute floods, huge unclosed documents)
      across the Rust crates above.
- [ ] **S** Tokenizer-suite and serializer conformance numbers next to the
      tree-construction report (html5gum and astral-html publish theirs).
- [ ] **M** Apply to OSS-Fuzz once eligible, and run ClusterFuzzLite on every
      PR and nightly in the meantime; crashers become regression tests.
- [ ] **S** More budgets: CLI binary size, wheel size, Markdown time, and a
      regression threshold on the weekly benchmark instead of a report only.

## 0.10 — "Lint for teams"

Everything a team needs to put `domq lint` in CI and keep it there, in one
release.

- [ ] **S** `ship` `--allow` / `--deny` per rule, a summary line, and
      `--format sarif` for GitHub code scanning.
- [ ] **S** `ship` Inline suppressions (`<!-- domq-disable-next-line img-alt -->`)
      and a config file, so a team can adopt the linter on an existing site.
- [ ] **M** `ship` More rules people expect: `html-lang`, `missing-title`,
      `form-label`, `iframe-title`, nested interactive elements (an `<a>` in a
      `<button>`), in-page anchors that point at no `id`, `target=_blank`
      without `rel=noopener`, images without dimensions, positive `tabindex`,
      invalid ARIA roles and attributes, buttons without `type` in forms,
      deprecated elements, and tables without headers (markuplint, HTMLHint,
      html-validate, axe-core's static subset).
- [ ] **M** `ship` `js` `py` The linter in the library (a `lint` feature), so
      the JS and Python packages and editors can use it.
- [ ] **S** `ship` `ai` `domq llms-txt public/`: `llms.txt` and
      `llms-full.txt` from a built static site, locally, with no crawling.
- [ ] **M** `ship` `rust` Local link checking: relative `href` / `src` that
      point at files that don't exist, for static sites (no network).
- [ ] **M** `ship` A reusable GitHub Action for `domq lint` that annotates pull
      requests.
- [ ] **S** `ship` A pre-commit hook for `domq lint`.

## 0.11 — "Huge documents and full trees"

For inputs too big to hold as a tree, for comparing documents in tests, and
for the rest of the binding parity.

- [ ] **L** `rust` `shell` Streaming selectors: run a `Selector` over
      `domq::events` for multi-gigabyte dumps without building a tree; CLI
      `--stream`.
- [ ] **L** `js` `rust` A streaming rewriter: `domq::rewrite(src, rules)` over
      the event stream (set or remove attributes, remove, insert, replace
      text) with a JS API for Workers, Deno and Bun, behind a feature and
      inside the wasm budget. (lol_html, `HTMLRewriter`, parse5's rewriting
      stream.)
- [ ] **L** `rust` `shell` `domq diff a.html b.html`: a structural diff
      (node-level, whitespace-insensitive) for snapshot tests of rendered pages.
- [ ] **M** `py` Python: editing API and `events()`.
- [ ] **M** `js` JavaScript: editing on `Document` and `events()` in the wasm
      package, inside the size budget.
- [ ] **M** `rust` Optional `<tbody>`/`<colgroup>` insertion: a builder option
      for browser-identical table structure.
- [ ] **M** `rust` Context-aware fragments: `append_html` and
      `parse_fragment_in("tr", …)` parse in the parent's context (a `<td>`
      inside a `<tr>`).

## Anytime — small and welcome

Small, high-value polish that doesn't need a release of its own. Most items are
good first issues; each lands in whichever release is next.

- [ ] **S** `shell` Several inputs in one run (`-f` repeatable, globs), with
      grep-style `FILE:` prefixes and a `file` field in `--jsonl`; batch output
      with `-o DIR --keep-dirs --parallel N` and `--deduplicate`. ([#26](https://github.com/iamsaquib8/domq/issues/26))
- [x] **S** `shell` `--exit-status`: exit 1 when nothing matches, for
      `if domq --exit-status …; then`. The default stays htmlq-compatible. ([#22](https://github.com/iamsaquib8/domq/issues/22))
- [ ] **S** `shell` Shell completions (bash, zsh, fish, PowerShell) and a man
      page, generated from `--help` and shipped in the release archives and
      the Homebrew formula. ([#23](https://github.com/iamsaquib8/domq/issues/23))
- [x] **S** `shell` A warning on stderr when `<meta charset>` names an encoding
      the CLI can't decode (Shift_JIS, GBK, …) and the page isn't valid
      UTF-8, instead of silent mojibake. (Already in the CLI's
      `encoding.rs`.)
- [ ] **S** `rust` `Selection` helpers, chainable and allocation-light:
      `dom.select("a").attrs("href")`, `.texts()`, `.first()`, `.len()`.
- [ ] **S** `rust` Reusable parser: `Dom::parse_into(&mut self, html)` reuses
      the arenas, so a crawler parsing millions of pages stops allocating per
      page.
- [ ] **S** `rust` Cookbook: fetching with `reqwest` / `ureq`, and parsing
      pages in parallel with rayon. ([#28](https://github.com/iamsaquib8/domq/issues/28))
- [ ] **S** A counting-allocator test, so the allocations-per-parse budget is
      enforced in CI. ([#27](https://github.com/iamsaquib8/domq/issues/27))
- [ ] **S** Release automation: one command bumps every version in lock-step
      (crate, CLI, wasm, Python) and drafts the changelog; the release workflow
      updates the Homebrew tap itself (token scoped to the tap repo).
- [ ] **S** Supply chain: signed build provenance (GitHub artifact attestations)
      and an SBOM attached to every release.
- [ ] **M** Playground: reader-mode, chunks, metadata and lint tabs, within the
      wasm size budget.
- [ ] **S** `shell` A pup migration guide in the docs (pup's `text{}`,
      `attr{}`, `json{}` and `:parent-of()` map to `-t`, `-a`, `-j` and
      `:has()`). ([#25](https://github.com/iamsaquib8/domq/issues/25))
- [ ] **S** `shell` Shell niceties: `--shell` for `eval`-able `NAME=…` output
      (xidel), coloured HTML with a pager (pup, xq), and `-i` to format a file
      in place (xq; the start of `domq fmt`).
- [ ] **S** `rust` `shell` A URL pass: `absolutize_urls(base)` over `href`,
      `src`, `srcset`, `poster`, `action`, `data-src`, CSS `url()` and meta
      refresh; `rewrite_urls(|u| …)`; `strip_tracking_params()`; CLI
      `--absolute` for HTML output.
- [ ] **S** `rust` `py` Text matchers: `:has-text()`, `:only-text`,
      case-insensitive `:contains("x" i)`, and `NodeRef::own_text()` (direct
      text only), as dom_query and selectolax have. ([#24](https://github.com/iamsaquib8/domq/issues/24))
- [ ] **S** `rust` Editing conveniences: `replace_with_html`, `remove_attrs` /
      `remove_all_attrs`, `strip_tags` (unwrap by selector).

## 1.0 — "Stable"

1.0 ships when all of these hold, not on a date:

- [ ] The public API is reviewed against the Rust API Guidelines, with
      `#[non_exhaustive]` where growth is expected and no known breaking
      changes pending.
- [x] A written MSRV policy (see [Budgets](#budgets)).
- [ ] Conformance and reader-mode numbers are published and tracked, including
      reader mode on held-out, human-labelled corpora (AEB, WCXB).
- [ ] Months of continuous fuzzing with no open crashers.
- [ ] Used by real projects (a "used by" list in the README).
- [ ] SemVer covers everything people script against: the crate API, CLI
      flags, output formats and exit codes, the MCP tools and their schemas,
      and the JS and Python APIs.
- [ ] The output-stability policy (see Ongoing) has held for at least two
      minor releases.
- [ ] A deprecation policy: anything removed is deprecated, with a warning,
      for at least one minor release first.
- [ ] A written support policy for Python, Node and Deno versions.
- [ ] A security response target in [SECURITY.md](./SECURITY.md).

---

## Distribution

Being easy to *get* matters as much as features.

**Done:** crates.io (`domq`, `domq-cli`, `domq-derive`), `cargo binstall`,
release binaries (Linux x86_64/arm64, macOS, Windows) with checksums, WASI
build, Homebrew tap, npm (trusted publishing, provenance), PyPI (trusted
publishing, abi3 wheels), the playground, and the guide.

**Planned:** the MCP Registry, `uvx` / `npx`-launchable binaries and Agent
Skills (0.5.x); musllinux, Windows arm64 and free-threaded wheels,
conda-forge, JSR, Scoop / winget / Nix / AUR / Docker (0.6); release
automation and the Homebrew tap auto-update (anytime); homebrew-core once the
project meets its notability bar.

## Community

- [ ] `good first issue`s kept stocked; first response on issues within 48
      hours.
- [ ] A "used by" section and credit for every contributor in release notes.
- [ ] GitHub Discussions for questions and show-and-tell, so issues stay
      actionable.

## Exploring

Ideas we like but haven't committed to. Prototypes and use-cases welcome.

- **More encodings in the CLI** (Shift_JIS, GBK, EUC-KR, ISO-8859-x) through
  an optional `encoding_rs` feature, used in the release binaries.
- **WASI component (WIT)** so any component runtime can call the parser.
- **More bindings** (C ABI, Ruby, PHP, Go via wasm) if there's demand.
- **Language-server-lite** — lint diagnostics in editors, using source spans;
  a natural follow-up to lint for teams (0.7).
- **`domq fmt`** — an HTML formatter built on the pretty-printer and spans.
- **XPath subset** — parsel, lxml, xq and turbohtml users ask for it; still
  only with strong demand, since CSS covers most real queries.
- **Incremental re-parse** for editors.
- **Page types** (article, listing, forum, product, docs) with per-type
  rules from JSON-LD `@type`, `og:type`, repeated card structure and
  docs-framework fingerprints. WCXB shows extractors tie on articles and fall
  to 0.41–0.84 F1 on other page types. Rules in the core; any trained model in
  a companion crate.
- **Site extractors** (GitHub, Hacker News, Reddit, Wikipedia, Discourse,
  Stack Overflow, …) behind an `Extractor` trait in a companion crate
  (`domq-sites`), as defuddle and article-extractor do; too much churn for
  the core.
- **Simplified semantic HTML** (`to_clean_html()`: semantic tags only, no
  classes, styles or handlers), as mdream and Firecrawl offer. Only as
  formatting simplification, never as a security boundary.
- **Near-duplicate chunk detection** (simhash) for RAG indexes.

## Considered and declined

- **Borrowed-tree mode** — measured in 0.3: string copying is 10–15% of parse
  time on dense markup and nothing on prose, and it would need a lifetime on
  `Dom` or a second API. Numbers in [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md).
- **Fetching inside `domq mcp`** — agents already have fetch tools, and
  fetching brings TLS, redirects, robots.txt and server-side request forgery
  into a project that has none of them. A fetch tool that saves to disk plus
  `--allow-read` gives the same savings. We'll revisit if that pairing proves
  impractical.
- **Crawling and rendering** — every competing CLI fetches; staying
  network-free is what makes domq the safe tool to hand an agent.
- **Fetching URLs**, **sanitization**, **full WHATWG**, **ML extraction**,
  **other document formats** — see non-goals.

---

## Shipped

### 0.5.0 — 2026-10-25 · "Trust, measured"

Details in the [changelog](./CHANGELOG.md#050--2026-10-25).

- [x] `domq mcp`: an MCP server over stdio with nine read-only tools
      (markdown, text, select, extract, chunks, links, metadata, tables, lint)
      on HTML the agent provides; MCP 2026-07-28 and the handshake-based
      revisions back to 2024-11-05; zero dependencies.
- [x] A conformance report: the 1,959 html5lib tree-construction tests (now
      in web-platform-tests), 0 panics, tracked in CI
      ([docs/conformance.md](docs/conformance.md)).
- [x] Spec-aligned recovery where it's cheap: scoped implicit closes
      (`<li><a>One<li>`, `<td><b>x<td>`, `<p><b>x<div>`), character references
      without `;`, browser comment recovery, the other raw-text elements and
      `<plaintext>`, stray `</p>`, `</br>`, `<image>`. Same structure as a
      browser 55.5% → 61.8%; character references 100%.
- [x] A reader-mode benchmark against Readability's 130 pages
      ([docs/reader-mode.md](docs/reader-mode.md)) and tuning with it: 91.9% →
      95.2% F1 agreement (whole page: 82.1%); hidden elements left out of
      `to_text` / `to_markdown`.
- [x] [The guide](https://iamsaquib8.github.io/domq/docs/): install, quick
      start, cookbook, CLI / selector / MCP references, and migration guides
      from htmlq, scraper, BeautifulSoup and cheerio, with tested examples.

### 0.4.0 — 2026-10-22 · "Declarative scraping"

Details in the [changelog](./CHANGELOG.md#040--2026-10-22).

- [x] **One name everywhere:** the crate `domq`, the CLI `domq-cli` (binary
      `domq`), the npm package `domq-wasm`, the PyPI package and import
      `domq`, and `brew install iamsaquib8/tap/domq`.
- [x] `#[derive(FromHtml)]` in the optional companion crate `domq-derive`
      (the core stays zero-dependency): declarative extraction into structs
      with `select`, `attr`, `html`, `markdown`, `default`, `with`, `Option<T>`,
      `Vec<T>`, nested and generic structs, compile-time diagnostics, and
      errors that name the field path and selector that failed.
- [x] Python `domq.extract` / `extract_all`: the same extraction into
      dataclasses with `Annotated[T, Select(...)]`, with the same error paths
      and messages as Rust.
- [x] MSRV 1.88, Python 3.11+; `to_text()` / `to_markdown()` called *on* a
      hidden element return its content.

### 0.3.0 — 2026-10-21 · "One-line scraping"

Everything below shipped in 0.3.0 (what was planned as phases 0.3–0.7, plus
distribution and two rounds of review fixes). Details in the
[changelog](./CHANGELOG.md).

<details>
<summary>Everything in 0.3.0</summary>

**Selectors** (the `query` feature, now on by default)

- [x] `+` / `~` sibling combinators.
- [x] Attribute operators `^=` `$=` `*=` `~=` `|=` with the `i` / `s` flags;
      HTML's case-insensitive enumerated attributes (`type`, `method`, …).
- [x] Structural pseudo-classes: `:first-child`, `:last-child`, `:only-child`,
      `:nth-child(An+B [of S])`, `:nth-last-child`, `:first-of-type`,
      `:last-of-type`, `:only-of-type`, `:nth-of-type`, `:nth-last-of-type`,
      `:empty`, `:root`, `:scope`.
- [x] Logical pseudo-classes: `:not()`, `:is()`, `:where()`, `:has()` with
      relative selectors.
- [x] `:checked`, `:disabled`, `:enabled`, CSS escapes (`.md\:flex`), and the
      non-standard `:contains("text")`, clearly labelled.
- [x] `Selector::parse` / `FromStr` with positioned errors; `select` accepts
      `&str` or a compiled `&Selector`.
- [x] Scoped queries: `NodeRef::select`, `select_first`, `matches`, `closest`;
      `Dom::select_first`.
- [x] Linear-time matching: per-query memo for `:nth-*`, sibling and ancestor
      searches; bounded memo memory; scoped queries pay only for what they walk.

**Getting content out**

- [x] `to_text()`: readable text (blocks become lines, whitespace collapsed,
      `<head>`/`<script>`/`<style>`/`<title>` dropped).
- [x] `to_markdown()`: zero-dependency CommonMark + GFM (headings, emphasis,
      links, images, nested lists, `<li value>`/`<ol reversed>`, code blocks
      with languages, quotes, tables).
- [x] `inner_html()`, `outer_html()`, `next_element_sibling()`,
      `prev_element_sibling()`.
- [x] Windows-1252 remapping for numeric references `&#128;`–`&#159;`.

**Edit and point**

- [x] Editing: `remove`, `set_attr`, `remove_attr`, `set_text`, `rename`,
      `create_element` / `create_text`, `append_child`, `insert_before`,
      `wrap` / `unwrap`, `append_html` / `prepend_html` /
      `insert_html_before` / `insert_html_after`. No cycles; document order is
      kept after every edit; `is_detached()`.
- [x] Source spans: `NodeRef::source_range()` and `domq::line_col()`.
- [x] Top-level nodes linked as siblings (`li + li` works on fragments).
- [x] More WHATWG optional-tag rules: the full `<p>`-closing list, headings,
      unclosed `<head>`, ruby, `<hr>` in `<select>`, captions / `colgroup` / `col`.
- [x] `entities-full` feature: all 2,125 WHATWG named references (default: the
      178 most common).

**Fast and lean**

- [x] One attribute arena per document; 52-byte nodes; no per-tag allocations
      (~20 allocations per parse); unused arena capacity released after parsing.
- [x] Word-at-a-time byte scanning; optional `simd` feature (`memchr`).
- [x] Parsing 25–37% faster on markup and 69% on prose, ahead of `tl` on large
      documents.
- [x] `domq::events()`: streaming parse events with the same recovery as
      the tree builder.
- [x] Benchmarks against `tl`, `scraper` and `html5ever`, a memory report, and a
      weekly benchmark workflow.
- [x] Differential fuzzing against html5ever on the supported subset.
- [x] Borrowed-tree mode evaluated and declined (see above).

**Structure and data**

- [x] `Dom::main_content()`: deterministic reader-mode heuristic.
- [x] `to_markdown_chunks()`: heading-aware RAG chunks with heading paths.
- [x] `Dom::links()` / `resources()` / `base_url()`, honouring `<base href>`
      (but not inside `<template>`).
- [x] `Dom::metadata()`: title, description, canonical, lang, OpenGraph,
      Twitter cards, JSON-LD and feeds.
- [x] `NodeRef::table_rows()` with colspan/rowspan expanded; `domq::to_csv`.
- [x] `domq::resolve_url` / `resolve_srcset`: RFC 3986, HTML `srcset` rules,
      and browsers' backslash handling.

**CLI: `domq`** (htmlq-compatible: `-a -t -p -w -r -b -B -f -o`)

- [x] Output: `--markdown`, `--json`, `--jsonl`, `--csv`, `--count`, `--first`,
      pretty HTML; conflicting formats are a usage error.
- [x] `-e NAME=SELECTOR[@attr][[]]` structured extraction, scoped per match.
- [x] `--main` reader mode, `--links`, `--meta`, `--remove-nodes`, `--base`.
- [x] Encoding detection (BOM, `<meta charset>`, UTF-8, Windows-1252) and
      `--encoding`.
- [x] `domq lint`: alt text, empty links and buttons, duplicate ids, heading
      order and parse errors, as `file:line:col` or JSON, with CI exit codes.

**JavaScript, Python and the browser**

- [x] The npm package (`dom-tree-wasm` in 0.3.0, `domq-wasm` from 0.4.0): `parse`, `select`, `selectFirst`, `toText`,
      `toMarkdown`, `toHtml`, `errors`, and a parse-once `Document`; full
      TypeScript types; works in Node with no setup; ~41 KB gzipped.
- [x] Playground: <https://iamsaquib8.github.io/domq/>.
- [x] `domq` on PyPI (`import domq`): abi3 wheels for 5 platforms,
      typed stubs, the GIL released on large inputs.

**Distribution**

- [x] crates.io: `domq` and `domq-cli`.
- [x] Release binaries for Linux x86_64/arm64, macOS Intel/Apple Silicon and
      Windows, with `SHA256SUMS`; `cargo binstall domq-cli`.
- [x] WASI build (`domq.wasm`, runs under wasmtime).
- [x] Homebrew: `brew install iamsaquib8/tap/domq`.
- [x] npm and PyPI publishing from CI with trusted publishing (no stored
      tokens; npm with provenance).

**Quality and hardening**

- [x] `Dom::errors()` no longer flags valid HTML (a bare `&`, optional end
      tags); positions point at the problem.
- [x] Two independent reviews. All findings fixed: quadratic paths (tag-name
      hash collisions, scoped queries, `append_html`, chunk paths, nested tables,
      CLI `--links`/`-e`), markup injection through edited content (raw-text
      escaping, name validation), and URL/`<base>` edge cases.
- [x] Never-panic proptests for selectors, output, editing and source spans;
      fuzz targets for `parse`, `tokenize`, `select`, `markdown` and
      `differential`; 100k-deep nesting tests.
- [x] CI: tests (stable/beta), MSRV 1.85, `no_std`, clippy, fuzz smoke, wasm
      size gate, zero-dependency gate, Python, WASI, Pages, release.
- [x] Project: README redesign, `CODE_OF_CONDUCT.md`, `SECURITY.md`,
      `docs/ARCHITECTURE.md`, issue/PR templates, dependabot, launch kit.

</details>

### 0.2 — 2026-06

Never-panicking, error-tolerant parser; arena tree with O(1) traversal; a CSS
subset; zero-dependency JSON; `to_html()`; `no_std + alloc`; WASM bindings;
property tests, snapshot corpus, fuzzing and CI.

## Have an idea?

Open an issue describing the use case, ideally naming which of the six groups
above it helps. The bar is "does it keep the project tiny, zero-dependency,
forgiving and honest?" If yes, let's talk.
