# The landscape

Who else turns HTML into data, text and Markdown, what they do that domq
doesn't (yet), and the numbers worth measuring against. This feeds the
[roadmap](../ROADMAP.md); it is a snapshot as of **2026-10-25**, from project
READMEs, docs and release notes read that day. Numbers are the projects' own
unless a benchmark is named. Corrections welcome.

## Where domq stands

**Clearly differentiated**

- **One small core for every surface.** Selectors, reader mode, Markdown, RAG
  chunks, metadata, links, tables, lint and an MCP server in one
  zero-dependency, `no_std`, `forbid(unsafe)`, never-panicking crate, shipped
  to Rust, a CLI, a ~58 KB gzipped wasm module (~80 KB with extraction), Python and WASI. The closest
  bundles are narrower (mdream: Markdown-centric, no selectors or lint;
  defuddle: JS-only, needs a DOM implementation) or heavier (the Rust stack
  `dom_query` + `dom_smoothie` + `htmd` is three crates on html5ever;
  xberg's html-to-markdown wasm package is 19 MB unpacked).
- **Measured honestly.** No other forgiving parser (tl, htmlparser2,
  node-html-parser) publishes a tree-construction conformance report.
- **A safe MCP server.** No network, no file access. The alternatives fetch:
  markitdown-mcp's README warns it reads any file the user can and reaches the
  network; the official fetch server needs an SSRF guard.
- **A maintained htmlq-compatible CLI.** htmlq's last release was January 2022
  (7.6k★), pup's July 2016 (8.4k★), orf/hq's November 2023.

**Behind**

- Reader-mode quality, now measured on the human-labelled corpora
  ([docs/reader-quality.md](reader-quality.md)): AEB 0.900 (trafilatura
  0.958, Readability.js 0.947, dom_smoothie 0.865) and WCXB 0.792 / 0.830
  dev / test (rs-trafilatura 0.859 / 0.893, dom_smoothie 0.762 / 0.808).
  Precision and non-article page types (forums, listings) are the gaps.
- No options on `to_markdown` / `to_text`; no confidence score from
  `main_content()`; no comments extraction, page types or site extractors.
- Metadata agreement with Readability: title 48.8%, byline 36.8%, date 48.5%.
- Reader mode, metadata, links, chunks and tables are not in the Python and
  JS packages yet.
- Not a WHATWG tree builder, while html5ever, parse5, lexbor, justhtml and
  turbohtml all advertise spec trees (a deliberate trade-off, but a real one).

**Threats**

- **[mdream](https://github.com/harlan-zw/mdream)**: zero-dependency Rust core
  with NAPI, WASM and JS builds; powers Cloudflare Browser Run's `/markdown`.
  An LLM "minimal" preset, isolate-main, a splitter, llms.txt generation, a
  GitHub Action. The most direct rival for "LLM Markdown at the edge".
- **[turbohtml](https://pypi.org/project/turbohtml/)** (Python, C core) and
  **[justhtml](https://pypi.org/project/justhtml/)** (pure Python, WHATWG,
  adopted by Mozilla Support): the same "replace readability + markdownify"
  pitch, in Python.
- **[xberg html-to-markdown](https://github.com/xberg-io/html-to-markdown)**:
  16 language bindings, Microdata/RDFa metadata, and Agent Skills packaged for
  Claude Code, Codex and Cursor.
- **[rs-trafilatura](https://github.com/Murrough-Foley/rs-trafilatura)**: the
  best published Rust extraction quality (AEB 0.97), with a page-type
  classifier and a 0–1 quality score.
- **[defuddle](https://github.com/kepano/defuddle)**: Obsidian's extractor;
  standardizes code, footnotes and math; 27 site extractors.
- **[Cloudflare Markdown for Agents](https://blog.cloudflare.com/markdown-for-agents/)**:
  `Accept: text/markdown` at the edge, with an `x-markdown-tokens` header.
- MCP: markitdown-mcp, firecrawl-mcp, docling-mcp, Jina's MCP, and
  Readability / trafilatura / defuddle wrappers.

**Openings**

- `scraper`'s recent releases only bump dependencies: Rust users looking for
  more than a tree have room for something new.
- Firecrawl's native module (kuchikiki + lol_html) and spider's
  transformations each re-implement links, metadata, main content and
  images: domq's exact surface.

## What competitors have that domq doesn't

Grouped by the job. Each idea links to where it comes from; the roadmap says
when (or whether) domq does it.

### Markdown and text output

| Idea | From |
|---|---|
| Options: link style (inline, reference, footer summary), images (keep / alt-only / drop, drop `data:` URIs), heading / bullet / emphasis / fence style, tags to skip or keep as HTML, wrap width, blank-line limit | [turndown](https://github.com/mixmark-io/turndown), [htmd](https://github.com/letmutex/htmd), [markdownify](https://github.com/matthewwithanm/python-markdownify), [node-html-markdown](https://github.com/crosstype/node-html-markdown), [html2text](https://github.com/Alir3z4/html2text/blob/master/docs/usage.md), Jina Reader's `X-Md-*` / `X-Retain-*` headers |
| An LLM preset: drop empty and redundant links, self-link headings, fragment links, tracking parameters, SVG and data URIs; images as alt text | mdream `minimal` clean rules |
| Token estimate and budgets | Cloudflare `x-markdown-tokens`, Jina `X-Token-Budget` / `X-Max-Tokens` |
| YAML frontmatter (title, canonical, author, date, description, …) | defuddle `-f`, Jina `markdown+frontmatter`, trafilatura `--with-metadata` |
| Standardization before conversion: code-block languages (Prism, Shiki, highlight.js), footnotes to `[^n]`, MathJax/KaTeX to `$…$`, lazy images, heading permalinks, duplicate title H1, callouts | defuddle, [readabilityrs](https://github.com/theiskaa/readabilityrs), [legible](https://github.com/ryanfowler/legible) |
| Custom element handlers | turndown `addRule`, htmd `add_handler`, markdownify `convert_<tag>` |
| Text: wrap width, tables as aligned grids, form fields, `noscript` toggle | [html-to-text](https://github.com/html-to-text/node-html-to-text), [inscriptis](https://github.com/weblyzard/inscriptis), [rust-html2text](https://github.com/jugglerchris/rust-html2text), [resiliparse](https://resiliparse.chatnoir.eu/) |
| Output text mapped back to source spans (citations, highlighting) | inscriptis annotations, turbohtml `to_annotated_text` |

### Reader mode

| Idea | From |
|---|---|
| A confidence score and `is_probably_readable()` | [@mozilla/readability](https://github.com/mozilla/readability) `isProbablyReaderable`, [dom_smoothie](https://github.com/niklak/dom_smoothie), rs-trafilatura (pages under 0.80 go to an LLM fallback) |
| An article result: title without the site suffix, byline, date, excerpt, word count, reading time | Readability, dom_smoothie, legible, article-extractor |
| Knobs: content hint / root, include and exclude selectors, precision vs recall, keep comments / tables / images, a debug log of what was removed and why | defuddle, legible, [trafilatura](https://trafilatura.readthedocs.io/en/latest/usage-cli.html), Firecrawl |
| Comments separate from the article | trafilatura, rs-trafilatura, resiliparse, defuddle |
| Page types (article, listing, forum, product, docs) with per-type rules | rs-trafilatura, Diffbot Analyze |
| Query-focused pruning (BM25, density filters) | [Crawl4AI](https://docs.crawl4ai.com/core/markdown-generation/) `fit_markdown` |

### Extraction and agents

| Idea | From |
|---|---|
| Schema files: base selector, typed fields, nested lists, sibling / parent navigation, transforms; written once, reused with no LLM | Crawl4AI JsonCss and `generate_schema`, [cheerio `extract()`](https://cheerio.js.org/docs/basics/extract/), [orf/hq](https://github.com/orf/hq), xidel templates |
| `::text` / `::attr(name)` pseudo-elements, `.re()` | [parsel](https://parsel.readthedocs.io/en/latest/usage.html) (Scrapy) |
| `:has-text()`, `:only-text`, `:contains("x" i)`, own text | dom_query, selectolax |
| An accessibility outline with refs, so a model picks elements (and URLs) by ref instead of writing them | Stagehand `extract`, Playwright MCP snapshots |
| Unique selector generation, find by text | Crawl4AI schema validation, llm-scraper |
| Microdata and RDFa next to JSON-LD; typed Product / Article / Recipe views | xberg, turbohtml, Firecrawl `product` |
| Paginated tool output | the official MCP fetch server (`max_length`, `start_index`) |
| Input budgets (bytes, nodes, depth) | legible `parse_budget`, Readability `maxElemsToParse`, html-to-text limits, lol_html `MemorySettings` |
| Agent Skills and one-click installs for coding agents | xberg |

### Shell and files

| Idea | From |
|---|---|
| Exit non-zero when nothing matches | [xq](https://github.com/sibprogrammer/xq) `-s` |
| Batch: input dirs, `-o DIR --keep-dirs --parallel N`, deduplication | trafilatura, mdka, Go html2markdown |
| `llms.txt` / `llms-full.txt` from a built site (local, no crawling) | `@mdream/crawl` |
| `eval`-able shell variables; coloured output with a pager; format in place | xidel, xq, pup |
| Absolute / rewritten URLs in HTML output, tracking-parameter stripping | lxml `make_links_absolute` / `rewrite_links`, Go html-to-markdown `WithDomain` |
| Bytes in with encoding sniffing, in every binding | cheerio `loadBuffer`, BeautifulSoup UnicodeDammit, rs-trafilatura `extract_bytes` |

### People who ship HTML

Static-rule candidates for `domq lint`: `<html lang>`, `<title>`, unlabeled
form controls, `target=_blank` without `rel=noopener`, images without
dimensions, positive `tabindex`, invalid ARIA roles and attributes, buttons
without `type` in forms, deprecated elements, tables without headers (from
markuplint, HTMLHint, html-validate and axe-core's static subset).

## Numbers to measure against

### Extraction quality

| Benchmark | Size, licence | Published scores |
|---|---|---|
| [Zyte / ScrapingHub AEB](https://github.com/scrapinghub/article-extraction-benchmark) (archived 2026-06; run locally) | 181 articles, MIT | rs-trafilatura 0.970, trafilatura 0.958, Readability.js 0.947, go-readability (fork) 0.947, dom_smoothie 0.865, readabilityrs 0.832; whole-page text (html-text) 0.665 |
| [WCXB](https://github.com/Murrough-Foley/web-content-extraction-benchmark) ([leaderboard](https://webcontentextraction.org)) | 2,008 pages in 7 page types (1,497 dev / 511 test), CC-BY-4.0 | dev / test F1: rs-trafilatura 0.859 / 0.893, trafilatura 0.791 / 0.833, dom_smoothie 0.762 / 0.808, Readability 0.675 / 0.726 (785 ms/page) |
| [trafilatura evaluation](https://trafilatura.readthedocs.io/en/latest/evaluation.html) | 990 documents, Apache-2.0 | trafilatura 0.926, justext 0.862, readability-lxml 0.853, resiliparse 0.811, goose3 0.810 |
| [WebMainBench](https://github.com/opendatalab/WebMainBench) | 7,809 pages, 46 languages, Apache-2.0; a 545-page subset scores code, formula and table fidelity | Readability 0.654, trafilatura 0.640, resiliparse 0.629 (ROUGE-N F1) |
| [htmldate](https://github.com/adbar/htmldate) | 1,000 pages, Apache-2.0 | publication dates: htmldate 0.926–0.949, newspaper4k 0.810 |
| Mozilla Readability's test pages | 130 pages, Apache-2.0 | readabilityrs passes 119/130; domq's 95.2% F1 is *agreement* with Readability, not comparable with the rows above |

### Speed

- **HTML → Markdown** (Rust, mdream's pages of 166 KB / 420 KB / 1.8 MB):
  mdream 0.34 / 0.41 / 5.2 ms, html_to_markdown 1.7 / 2.7 / 28.5 ms, htmd
  2.1 / 3.5 / 34.4 ms; html2md-rs panicked on the first page. mdream's CLI
  starts in 1.4 ms.
- **Parsing** (Rust): on [astral-html](https://github.com/astral-sh/astral-html)'s
  five real pages, measured on one machine (`benchmarks/benches/real_pages.rs`),
  astral-html is fastest, `tl` second and domq third (22–35% behind `tl`),
  all far ahead of scraper. domq's old "ahead of tl" held only on generated
  inputs.
- **Python**: selectolax's 754-domain harness: selectolax 2.39 s, lxml 9.09 s,
  html5-parser 16.10 s, BeautifulSoup 61.02 s; turbohtml claims 2–5× lxml.
- **JS**: [htmlparser-benchmark](https://github.com/AndreasMadsen/htmlparser-benchmark),
  ms/file: htmlparser2 0.71, node-html-parser 0.97, parse5 2.44, cheerio 3.27.
- **Reader mode**: dom_smoothie 26 ms/page, rs-trafilatura 44 ms/page,
  Readability.js 785 ms/page (WCXB).
- **Tokens**: Cloudflare reports 16,180 → 3,150 tokens on an example page;
  mdream's minimal preset 21K → 6.1K on Wikipedia. Count with a fixed offline
  tokenizer and check content with WCXB's must-include snippets.

## Deliberately not adopted

Fetching, crawling and JavaScript rendering (every competing CLI fetches;
domq stays the safe choice for agents), sanitization (justhtml, turbohtml
and article-extractor ship sanitizers; use ammonia), machine-learned
extraction in the core (ReaderLM-v2, MinerU-HTML, rs-trafilatura's
classifier; domq exposes a confidence score as the hook for an LLM
fallback instead), Office and PDF input (markitdown, docling), RSS/Atom
parsing, and Djot output.
