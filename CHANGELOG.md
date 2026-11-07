# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

**Output changes** (anything that changes the text `to_text`, `to_markdown`
or the chunks produce, what `main_content()` picks, or the parsed tree) ship
only in minor releases and are listed under their own heading, so you know
when to re-embed. See [Output stability](./README.md#output-stability).

## [Unreleased]

Pending changes are files in [`changes/`](./changes/README.md).

## [0.7.0] — 2026-11-07

"Correct trees, Markdown built for models." The parser and the Markdown
writer are right on more real pages before more surface is added: 65.3% of
the html5lib tree-construction tests now match a browser's tree structure
(61.8% in 0.6), reader mode scores 0.900 F1 on AEB and 0.792 / 0.830 on WCXB
(dev / test; 0.898 and 0.747 / 0.803 in 0.6), and metadata reads JSON-LD.
Bytes in with encoding detection reaches Rust, the CLI, Python and
JavaScript, and several quadratic cases are now linear.

**Upgrading from 0.6:**

- **JavaScript:** the extraction API (`mainContent`, `metadata`, `links`,
  `resources`, `toMarkdownChunks`, `tables`, `toCsv`) moved to the new
  `domq-wasm/extract` entry; the root `domq-wasm` is the core only, now 58 KB
  gzipped (the extract entry is 80 KB; 0.6 was one 65 KB module). Change
  `import { mainContent } from "domq-wasm"` to `"domq-wasm/extract"`.
- **Rust, Python, CLI, MCP:** no API was removed.
- **Output changes:** the parsed tree, text, Markdown and chunks change for
  many pages (see "Output changes"), and `main_content()` picks a wider node
  on pages where the winner held under a tenth of the text. Re-embed if exact
  text matters.

### Added

- **Bytes in, with automatic character-encoding detection.** `domq::parse_bytes(bytes)`, `parse_bytes_with(bytes, label)`, `decode(bytes)` and `decode_with(bytes, label)` follow the browser's sniffing order: a byte-order mark, then an explicit label, then `<meta charset>` or `http-equiv` in the first 1024 bytes, then UTF-8 if the bytes are valid, else windows-1252. Supported: UTF-8, UTF-16LE, UTF-16BE and windows-1252 (with the WHATWG aliases such as `latin1`). `Decoded` reports the encoding used and any warning; `Encoding` and `lookup_encoding` are public. The CLI reads bytes the same way, Python's `parse()` takes `str` or `bytes` (and `encoding=`) and has `decode()`, and JavaScript has `parseBytes()` and `decode()` ([#46](https://github.com/iamsaquib8/domq/issues/46)).
- `NodeRef::links` and `NodeRef::resources` return the hyperlinks and embedded resources inside one node's subtree, with the URLs and text `Dom::links` / `Dom::resources` give for it; `links_resolved` and `resources_resolved` take a base URL already worked out with `Dom::base_url`, so a loop over many nodes scans for `<base>` once ([#114](https://github.com/iamsaquib8/domq/issues/114)).
- JavaScript: `domq-wasm` has a second entry, `domq-wasm/extract`, the core plus the extraction API, with its own Node entry point, TypeScript declarations and `.wasm` (`domq-wasm/extract/domq_wasm_bg.wasm`). The `wasm` crate has a default-on cargo feature `extract`; `--no-default-features` builds the core module. `node npm/build.mjs` builds both for the package ([#111](https://github.com/iamsaquib8/domq/issues/111)).

### Changed

- **Breaking for JavaScript: the extraction API moved to `domq-wasm/extract`.** The `domq-wasm` root entry is now the core only (parse, `select`, `selectFirst`, `toText`, `toMarkdown`, `toHtml`, `errors`, `decode`, `resolveUrl`, `resolveSrcset` and `Document`), and `mainContent`, `metadata`, `links`, `resources`, `toMarkdownChunks`, `tables` and `toCsv` (the free functions and the `Document` methods) are no longer exported from it. If you used any of them in 0.6, change `import { mainContent } from "domq-wasm"` to `import { mainContent } from "domq-wasm/extract"`; `domq-wasm/extract` also exports everything in the core, so you can import all of it from there. Without the change the import fails (`mainContent` is not exported). The functions and their results are the same in both entries. Why: the single module grew from ~43 KB to ~80 KB gzipped as extraction landed ([#111](https://github.com/iamsaquib8/domq/issues/111)), so an app that only selects and converts paid for code it never ran. The npm package now holds two builds of one crate: the core is 58,468 bytes gzipped and `domq-wasm/extract` is 79,534 (0.6 shipped one module of ~65 KB). CI gates them at 60,000 and 80,000 bytes. The old 45 KiB goal is not met (the core is 57.1 KiB); see the ROADMAP's Budgets.

### Output changes

- Chunks: `to_markdown_chunks_with` (and the Python and MCP `chunks` options) write links inline whatever the link style ([#94](https://github.com/iamsaquib8/domq/issues/94)). With `Reference` or `Footer`, every chunk but the last kept `[text][n]` with no definition (they sat at the end of the document), so the URL never reached the embedding or the prompt, and the footer's `### Links` became a chunk of its own, nested under the last section. Heading paths no longer contain reference numbers (`Docs1 home` is now `Docs home`), and `heading_text` skips a `][n]` label in Markdown you chunk with `Chunk::from_markdown`. The other Markdown options (images, data URIs, base URL) still apply.
- CLI: `-1` keeps the first result that survives `-a` and `-w` filtering in every mode, and `-c` honours `-w`.
- Metadata: `Metadata::json_ld` blocks are ready for a JSON parser ([#99](https://github.com/iamsaquib8/domq/issues/99)). The `<![CDATA[ … ]]>` wrapper (also `//<![CDATA[ … //]]>` and `/*<![CDATA[*/ … /*]]>*/`) and a `<!-- … -->` wrapper round the block are stripped, and a block that is nothing but a wrapper is dropped. On Readability's 130 test pages, 33 of the 34 JSON-LD blocks parse with `serde_json` as returned (7 did before); `docs/reader-mode.md` reports it. `domq --meta`, the MCP `metadata` tool and the Python and JavaScript `metadata()` carry the same blocks.
- Markdown: block syntax formed across nodes is escaped ([#92](https://github.com/iamsaquib8/domq/issues/92)). `<p><span>1</span>. Introduction</p>` and React's `<p>1<!-- -->. Introduction</p>` give `1\. Introduction` (they were `1. Introduction`, which renders as an ordered list; also `2024\)`), a line of only `|`, `-` and `:` after a hard break is escaped so it cannot turn the line above into a table, and a rule first in a bullet item is written `- ***` instead of `- ---` (a thematic break that dropped the list).
- Markdown: `LinkStyle::Footer` writes `text [1]` inline instead of `[text][1]` ([#97](https://github.com/iamsaquib8/domq/issues/97)). The footer never had `[1]: url` definitions, so CommonMark rendered the old form as literal brackets. The `### Links` list now builds each entry from the link's content as it reads inline (escaped, with its emphasis and images), instead of `text_content()`: `<a href=/a>A *star*</a>` is `1. [A \*star\*](/a)` rather than `[A *star*](/a)`, which rendered as emphasis, hidden and script text is gone, and an image-only link keeps its image (`1. [![Logo](i.png)](/x)`) instead of showing its URL.
- Markdown: a `&` that starts a character reference in a link destination, image source or title is written `&amp;` ([#105](https://github.com/iamsaquib8/domq/issues/105)). CommonMark decodes references there, so `href="/x?a=1&amp;amp;b=2"` was written `[q](/x?a=1&amp;b=2)` and rendered as `/x?a=1&b=2`; it is now `[q](/x?a=1&amp;amp;b=2)`. A plain `&` (`/x?a=1&b=2`) is unchanged.
- Markdown: an ordered list nested in a list item that does not start at 1 (`<ol start="5">`, `<ol reversed>` with more than one item, an item with `value`) follows a blank line ([#93](https://github.com/iamsaquib8/domq/issues/93)): `- Item\n\n  5. five`. Right after the item's text it was part of the item's paragraph (`<li>Item 5. five 6. six</li>`), since only a list starting at 1 can interrupt a paragraph.
- Markdown: GFM tables honour `colspan` and `rowspan` ([#98](https://github.com/iamsaquib8/domq/issues/98)). A spanning cell is written once and the slots it covers are empty, so later cells stay under their columns (`| Name | Score | | Rank |` for a `colspan=2` header; it was `| Name | Score | Rank | |`, with `Rank` over the wrong column). Spans follow `table_rows()`: values parse as HTML integers, stop at the end of their `<thead>`/`<tbody>`/`<tfoot>`, `rowspan=0` covers the rest of the group, and a table's spans add a bounded number of cells (the same budget as `table_rows()`).
- Metadata: `Metadata::title`, `author`, `published` and `image` read the page's JSON-LD article ([#100](https://github.com/iamsaquib8/domq/issues/100)). A small, bounded, zero-dependency JSON reader finds the article (an `Article`, `NewsArticle`, `BlogPosting`, … object at the top of a block, in an array or in `@graph`) and takes its `headline`, `author` (a name, an object with a `name`, an `@id` into the graph, or a list joined with `, `) and `datePublished`, ahead of `<title>`, the author and date meta tags and `<time>`; its `image` is a last fallback after `og:image` and `twitter:image`. `title` therefore changes on most pages that have JSON-LD: it is the clean `headline`, without the site name `<title>` usually adds. On Readability's pages, agreement goes from 48.8% to 59.1% (title), 36.8% to 52.6% (byline) and 48.5% to 97.0% (published time).
- Parser: an end tag no longer closes an element beyond a scope boundary ([#67](https://github.com/iamsaquib8/domq/issues/67)). A `</div>`, `</span>` or `</a>` inside a `<td>`, `<th>` or `<caption>` whose element opened outside the table is ignored, instead of closing the cell, the row and the table, so later cells and rows stay in the table. Tables, `table_rows()`, CSV and Markdown tables change on pages with mis-nested markup in cells. Other end tags stop at `applet caption html table td th marquee object template`; table end tags (`</td>`, `</tr>`, `</table>`, …) stop at `table template html`; `</body>`, `</html>` and `</template>` look through everything, as before.
- Parser: any heading end tag closes the innermost open heading of any level, so `<h2>Title</h3>` closes the `<h2>` ([#68](https://github.com/iamsaquib8/domq/issues/68)). The paragraphs after such a heading no longer land inside it, which fixes `to_markdown()` heading lines and chunk heading paths on those pages.
- Parser: `<a>`, `<button>`, `<nobr>` and `<select>` no longer nest: a start tag closes an open element of the same name, looking past anything but a scope boundary ([#69](https://github.com/iamsaquib8/domq/issues/69)). `<a>One<a>Two` gives two sibling links, as in browsers, so link text and Markdown no longer swallow the following link. Formatting elements in between are closed with the link and not reopened (that belongs to the adoption agency, which domq does not implement).
- Parser: a tag name runs to the first whitespace, `/` or `>` instead of stopping at the first character that is not an ASCII letter, digit, `-`, `:`, `_` or `.` ([#84](https://github.com/iamsaquib8/domq/issues/84)). `<my-élément>` is one element (it used to be `<my- élément>` with an attribute), and `<div$x>` is an element named `div$x`. Selectors such as `my-élément` now match it.
- Parser: one newline (`\n` or `\r\n`) right after a `<pre>`, `<listing>` or `<textarea>` start tag is dropped, as in browsers ([#75](https://github.com/iamsaquib8/domq/issues/75)), so `<pre>\ncode</pre>` has the text `code`, not `\ncode`. `text_content()`, JSON, `#[derive(FromHtml)]` extraction, text and Markdown code blocks, and `<textarea>` values no longer start with that newline. `to_html()` writes a newline back after the start tag when the content starts with one, so `<pre>\n\nx</pre>` round-trips.
- Parser: a `<` that starts no markup (`a < b`, `<3`, `<-`) stays in its text run instead of splitting it into three nodes ([#76](https://github.com/iamsaquib8/domq/issues/76)). `<p>if a < b then</p>` has one text node, so `first_child().text()`, the JSON tree, `children()` walks and the `Event::Text` stream see the whole sentence. `tokenize` yields one `Text` token for the run, and no longer yields an empty `Text("")` for a dropped `</>`. (The text on either side of a `</>` is still two nodes.)
- Reader mode: `main_content()` widens a winner that holds under a tenth of the page's text, up to `<body>` ([#91](https://github.com/iamsaquib8/domq/issues/91)). Listing, collection, product, service and forum pages often scored one stray paragraph (a signup or currency notice, a popup, a filter group, a card) while their real content is links; the answer is now the whole results area. A new step 5 in the `main_content` docs. The chosen element changes on those pages, and so do `--main`, the MCP `main` option and the Markdown/text of it. On WCXB (scored with its own method) F1 goes from 0.748 to 0.792 on the dev split (the split the rule was tuned on) and from 0.801 to 0.830 on the held-out test split, with every page type better on dev; AEB goes from 0.898 to 0.900 (precision 0.828 to 0.827, recall 0.981 to 0.987). Agreement with Readability (docs/reader-mode.md) is unchanged.
- Tokenizer: a `<script>`'s content follows the HTML script data states ([#234](https://github.com/iamsaquib8/domq/issues/234)). After `<!--`, a `<script` starts a "double escaped" part in which `</script>` does not end the element, so `<script><!--<script>a</script>b</script><p>x</p>` is one script with the text `<!--<script>a</script>b`, followed by the paragraph; the `b</script>` that used to leak into `to_text()` and Markdown is gone. `-->` leaves the escaped part. Old-style inline scripts that write script tags (ad loaders, analytics snippets) are the usual cause. `to_html()` writes `<\/script` only for an end tag that would really end the element, and writes `<\script` for a `<script` that would leave an edited or unterminated script double escaped to its end, so the element still closes.
- Tokenizer: an `=` where an attribute name starts begins the name, as in browsers ([#235](https://github.com/iamsaquib8/domq/issues/235)): `<div =x>` has an attribute named `=x` (it was `x`), and a parse error, the new `ErrorKind::UnexpectedEqualsSign`, is recorded at the `=`.

### Fixed

- CLI: no panic when stderr is a closed pipe, for example when a warning is printed ([#145](https://github.com/iamsaquib8/domq/issues/145)).
- CLI: an unknown or unsupported `--encoding` label (supported: utf-8, utf-16le, utf-16be, windows-1252) is a usage error instead of being ignored ([#152](https://github.com/iamsaquib8/domq/issues/152)).
- CLI: `-1` is applied after attribute (`-a`) and whitespace (`-w`) filtering rather than before, and `-c` honours `-w` to skip blank matches ([#144](https://github.com/iamsaquib8/domq/issues/144)).
- Reject Markdown options combined with another output format, regardless of flag order.
- CLI and MCP: `-b` / `base_url` must be an absolute URL (`scheme://…`); a relative one is rejected with a clear message ([#151](https://github.com/iamsaquib8/domq/issues/151)).
- Parser: HTML conformance (same structure) goes from 1226 to 1229 of 1959 tests, and exact from 87 to 88, with no regressions (`html5test-com.dat:1`, `webkit01.dat:174`, `webkit02.dat:309`).
- Parser: HTML conformance (same structure) goes from 1210 to 1216 of 1959 tests, and exact from 83 to 87, with no regressions (`tests3.dat:63`, `:75`, `:89`, `:215`, `:227`, `tests7.dat:188`).
- Parser: HTML conformance (same structure) goes from 1216 to 1226 of 1959 tests, with no regressions: 3 tests from #67 (`template.dat:78`, `template.dat:1224`, `tests16.dat:2587`), 2 from #68 (`tests19.dat:277`, `tests19.dat:305`) and 5 from #69 (`tests20.dat:1`, `tests26.dat:381`, `tests3.dat:256`, `tests3.dat:271`, `tests6.dat:166`).
- Parser: HTML conformance (same structure) goes from 1229 to 1279 of 1959 tests, with no regressions: 50 script data cases in `domjs-unsafe.dat`, `scriptdata01.dat` and `tests16.dat`.
- CSV: a row whose only cell is empty is written as `""` instead of a blank line, so it reads back as a row ([#150](https://github.com/iamsaquib8/domq/issues/150)).
- MCP: a request whose `_meta` specifies a handshake-era legacy version is served with legacy semantics rather than modern, allowing `initialize` and omitting modern result fields, and `clientCapabilities` is validated to be an object ([#159](https://github.com/iamsaquib8/domq/issues/159)).
- After edits, `to_text()`, `to_markdown()`, `links()` and `main_content()` decide
  whether `<noscript>` holds the page's content from the document as it is now. The
  decision was made at parse time and went stale: a document edited into some shape
  (`insert_html_after`, `append_html`, `create_element("noscript")`, `rename`,
  `set_text`, `remove`, moves) read differently from `parse(edited.to_html())`. Each
  edit that created a `<noscript>` also walked the whole document; creating 1,000 of
  them on a 644 KB document took 1.2 s and now takes under a millisecond
  ([#70](https://github.com/iamsaquib8/domq/issues/70)). Parsed documents are unaffected.
- CLI `--csv` and MCP `tables` with a selector that matches a container (not a `<table>`) use the outermost tables inside it, without descending into nested tables ([#149](https://github.com/iamsaquib8/domq/issues/149)).
- Input that is mostly valid UTF-8 with a few invalid sequences is decoded as UTF-8, with U+FFFD for the bad bytes and a warning, instead of being decoded as windows-1252 and garbling every accented character ([#143](https://github.com/iamsaquib8/domq/issues/143)).

### Performance

- `#[derive(FromHtml)]`: the field selectors are compiled once per `extract` / `extract_all` call (nested types included) instead of once per field per item, so `extract_all` over 10,000 cards with four selector fields takes 8.7 ms instead of 16.0 ms ([#104](https://github.com/iamsaquib8/domq/issues/104)). A side effect: an invalid selector anywhere in a type is now reported before anything is extracted, even in an `Option` field that matches nothing, and its error path has no item index.
- Markdown: `LinkStyle::Reference` and `LinkStyle::Footer` look up repeated links in an index instead of scanning every earlier link, so a page with many distinct links serializes in linear time: 40,000 links took 1.7 s and now take about 40 ms. The output is unchanged ([#95](https://github.com/iamsaquib8/domq/issues/95)).
- `Dom::outline()` and `NodeRef::outline()` take linear time instead of quadratic: the document is read once for the elements that share an id, class, `name` or `data-testid`, for the labels, and for the position among siblings, and the walk carries the rest down, instead of every outline node scanning the document again. A page of 20,000 links took 2.5 s and now takes about 40 ms, and 20,000 labelled inputs went from 19 s to about 60 ms. Names read only the start of an element's text. The outline is unchanged ([#90](https://github.com/iamsaquib8/domq/issues/90)).
- Parsing is faster on real pages: the tokenizer classifies each tag name once, so the tree builder's rules (void, implicit close, scope, counting) are integer tests instead of string matches, and an `<image>` outside `<svg>` and `<math>` no longer scans the open elements for them (200,000 `<image>` under 300 `<div>`s: about 1 s to about 10 ms). The tree and the diagnostics are unchanged ([#73](https://github.com/iamsaquib8/domq/issues/73)).
- Python: `Element` methods that do real work (`text`, `to_text`, `text_content`, `html`, `inner_html`, `to_markdown`, `to_markdown_chunks`, `table_rows`, `to_csv`, `outline`, `to_json`) release the GIL when the element's source is 16 KiB or more, as the `Document` methods already did, so a thread pool serializing elements runs in parallel: four threads calling `to_markdown()` on a 6,000-section page take about 12 ms instead of about 31 ms ([#125](https://github.com/iamsaquib8/domq/issues/125)).
- Python: `Element.links()` and `Element.resources()` walk only the element's subtree (and release the GIL for a large one) instead of the whole document, so calling them once per card is linear in the page, not quadratic: 1,000 cards take about 2 ms instead of about 12 s on a loaded machine ([#114](https://github.com/iamsaquib8/domq/issues/114)).
- `:has(> x)` (and any `:has()` whose argument uses only `>`, `+` and `~`) looks only
  as many levels down as the argument has `>`s, not at the candidate's whole
  subtree: `div:has(> span)` on 20,000 nested divs took 1.8 s to 8 s and now takes
  under a millisecond ([#77](https://github.com/iamsaquib8/domq/issues/77)).
- `:first-of-type`, `:last-of-type`, `:nth-of-type()`, `:nth-last-of-type()` and
  `:only-of-type` are linear on sibling lists with many distinct tag names (they were
  quadratic: 20,000 distinct siblings took 1.4 to 4 s and now take about 5 ms)
  ([#72](https://github.com/iamsaquib8/domq/issues/72)).

## [0.6.0] — 2026-11-03

"Where people are": reader mode, metadata, links, chunks and tables in Python
and JavaScript, Markdown shaped for models, and a page outline for agents.

**Upgrading from 0.5:** no API was removed. The JavaScript module grows from
~43 KB to ~65 KB gzipped because it now carries the extraction helpers
([#111](https://github.com/iamsaquib8/domq/issues/111) tracks moving them into
a second, optional module). Text and Markdown change for pages whose content
sits in `<noscript>`, and chunk heading paths for some headings with links
(see "Output changes"); re-embed if exact text matters. The new
`MarkdownOptions`, `LinkStyle`, `ImageStyle`, `OutlineNode` and `PageOutline`
are `#[non_exhaustive]`, so they can grow without breaking you: build
`MarkdownOptions` with `MarkdownOptions::new()` and its setters, and give a
`match` on `LinkStyle` or `ImageStyle` a `_` arm.

### Added

- **Python: reader mode and the data helpers.** `Document.main_content()`,
  `metadata()` (a `Metadata` object with `to_dict()`), `links()` and
  `resources()` (with `base_url`), `to_markdown_chunks(max_chars)` (each
  `Chunk` with its `heading_path` and `context`), `tables()` and `to_csv()`,
  and `outline()`; the same on `Element` for its subtree; `domq.resolve_url`,
  `domq.resolve_srcset`; and reusable, compiled `domq.Selector` objects. All
  typed in the stubs (stubtest and `mypy --strict` clean).
- **JavaScript: reader mode and the data helpers.** `mainContent()`,
  `metadata()`, `links()`, `resources()`, `toMarkdownChunks()`, `tables()`,
  `toCsv()`, `resolveUrl()` and `resolveSrcset()`, as functions and as
  `Document` methods, with TypeScript types.
- **Markdown options.** `MarkdownOptions` with `to_markdown_with()` and
  `to_markdown_chunks_with()` on `Dom` and `NodeRef`: link style
  (`LinkStyle::Inline`; `Reference`, `[text][1]` with `[1]: url` definitions
  at the end; or `Footer`, a `### Links` list), images (`ImageStyle::Keep`,
  `AltOnly`, `Drop`), dropping `data:` URIs, and a base URL for relative
  links. In the CLI as `--markdown-links`, `--markdown-images` and
  `--markdown-drop-data-uris`; in the MCP `markdown` and `chunks` tools as
  `links`, `images` and `drop_data_uris`; in Python as keyword-only
  arguments of `to_markdown()` and `to_markdown_chunks()`.
  `Chunk::from_markdown` chunks Markdown you already have.
- **A page outline for agents.** `Dom::outline()` and `NodeRef::outline()`
  list landmarks, headings, links, buttons and form controls with their ARIA
  roles, accessible names, a ref, a selector and a source span. Also
  `domq --outline [SELECTOR]` (text, `-j`, `--jsonl`, `-c`), an MCP `outline`
  tool, and `outline()` in Python. Static semantics only: no CSS, no script.
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
- CLI `--exit-status` to exit 1 when no results match; without it, no matches
  still exit 0.

### Output changes

- **`<noscript>` that holds the page is shown.** When a document has more
  text inside `<noscript>` than outside it (a page that renders with script
  and ships its content for clients without it, as Discourse forums do),
  `to_text`, `to_markdown`, link text and `main_content()` show that content,
  as a browser with scripting off does. An "enable JavaScript" notice next to
  real content stays hidden. WCXB forums 0.365 → 0.498, WCXB dev 0.738 →
  0.747.
- **Chunk heading paths keep the whole heading** when it holds a link whose
  URL has balanced parentheses (`/wiki/Rust_(language)`) or whose title is
  single-quoted; both used to cut the heading short.

### Fixed

- **The guide is online.** The Pages deploy wrote the mdBook guide outside
  the uploaded site (mdBook 0.5 resolves `--dest-dir` against the current
  directory), so <https://iamsaquib8.github.io/domq/docs/> was a 404 since
  0.5.0. The deploy now fails when the guide or the playground is missing.
- **Image-input accessibility lint.** Flag unnamed `<input type="image">`
  controls and recognize their `alt` text as a name for enclosing links and
  buttons. A submission `value` alone does not name an image button.
- The npm package ships its `LICENSE` file.
- **`drop_data_uris` no longer panics on non-ASCII URLs.** `is_data_uri`
  checked `trimmed[..5]`, which panics when byte 5 falls inside a multi-byte
  character (for example `href="ééé"`). It now uses a byte-prefix check, so
  the CLI, MCP server and Python bindings stay up on such input.

### Performance

- Serializing or querying many subtrees no longer walks the whole document
  each time to decide whether `<noscript>` holds the page.

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
