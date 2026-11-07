# JavaScript API reference

Every function, method and type that `domq-wasm` exports. The package has two
entries, built from the same Rust crate:

| Import | What you get |
|---|---|
| `domq-wasm` | the core: parse, select, text, Markdown, HTML, JSON, decoding and URL helpers, and `Document` |
| `domq-wasm/extract` | everything in the core, plus reader mode, metadata, links, resources, chunks and tables |

Both entries export the same loading functions. Install, bundlers, Workers and
CDN use are in [Install](install.md#javascript-and-webassembly); this page
lists what you can call. In the tables below, **extract** marks a name that only
`domq-wasm/extract` exports. The page is checked against the package's
generated `.d.ts` files in CI, so a function added to the package and missing
here fails the build.

Every function takes the HTML as a string. A function that takes a `selector`
throws a `SelectorError` when it is invalid or unsupported; nothing else
throws, whatever the HTML.

```js
import { select, toMarkdown } from "domq-wasm";

const html = "<ul id=l><li><a href=/a>One<li><a href=/b>Two</ul>";
select(html, "#l a").map((m) => [m.attrs.href, m.text]); // [["/a", "One"], ["/b", "Two"]]
toMarkdown(html);                                        // "- [One](/a)\n- [Two](/b)"
```

## Loading the module

| Name | What it does |
|---|---|
| `init(input?)` | The default export. Loads the `.wasm`: call `await init()` once before anything else in a browser or with a bundler. In Node, Bun and Deno the entry loads it at import time and `init()` is a harmless no-op. |
| `initSync({ module })` | Loads an already compiled `WebAssembly.Module` or its bytes, synchronously. This is how a Cloudflare Worker starts. |
| `InitInput` | Type of what `init` accepts: a URL, `Request`, `Response`, bytes or a module. |
| `SyncInitInput` | Type of what `initSync` accepts: bytes or a compiled module. |
| `InitOutput` | Type of what both return: the raw WebAssembly exports. You do not need it. |

## Functions in both entries

| Function | Returns | See |
|---|---|---|
| `select(html, selector)` | `Match[]`: every matching element, in document order | [Scrape elements and attributes](cookbook/scraping.md) |
| `selectFirst(html, selector)` | `Match`, or `undefined` when nothing matches | [Quick start](quickstart.md) |
| `toText(html, selector?)` | readable text of the page, or of each match separated by a blank line | [Quick start](quickstart.md) |
| `toMarkdown(html, selector?)` | CommonMark and GFM Markdown of the page, or of each match separated by a blank line | [Turn a page into Markdown](cookbook/reader-mode.md) |
| `toHtml(html)` | the repaired HTML: every element closed and nested, attributes quoted, text escaped | [Edit and serialize](cookbook/editing.md) |
| `parse(input)` | `DomDocument`: the tree as plain objects (`input` is a string or a `Uint8Array`) | |
| `parseBytes(bytes)` | the same tree, from bytes, with the encoding detected | |
| `parseJson(html)` | the same tree as a JSON string, to send without re-encoding | |
| `errors(html)` | `ParseError[]`: what the parser repaired, and where (`position`, `message`) | |
| `decode(bytes, label?)` | `Decoded`: bytes to text with WHATWG encoding sniffing, or the encoding you name | |
| `resolveUrl(base, relative)` | the URL resolved as RFC 3986 section 5.2 says | [Scrape links, metadata and tables](cookbook/links-metadata-tables.md) |
| `resolveSrcset(base, srcset)` | a `srcset` value with every URL resolved | |

```js
import { decode, errors, resolveUrl, selectFirst, toHtml, toText } from "domq-wasm";

const html = "<ul id=l><li><a href=/a>One<li><a href=/b>Two</ul>";
selectFirst(html, "li")?.html;               // '<li><a href="/a">One</a></li>'
toText(html);                                // "One\nTwo"
toHtml(html);                                // '<ul id="l"><li><a href="/a">One</a></li><li><a href="/b">Two</a></li></ul>'
errors(html).length;                         // 3
resolveUrl("https://a.example/x/y", "../z"); // "https://a.example/z"
decode(new Uint8Array([0x3c, 0x70, 0x3e, 0x63, 0x61, 0x66, 0xe9, 0x3c, 0x2f, 0x70, 0x3e])).encoding; // "windows-1252"
```

## Functions in `domq-wasm/extract` only

| Function | Returns | See |
|---|---|---|
| `mainContent(html)` | `MainContent` (`text`, `markdown`, `html`) of the article, or `undefined` when no container stands out | [Turn a page into Markdown](cookbook/reader-mode.md) |
| `metadata(html, base_url?)` | `Metadata`: title, description, canonical URL, language, OpenGraph, Twitter Cards, JSON-LD, feeds | [Scrape links, metadata and tables](cookbook/links-metadata-tables.md) |
| `links(html, base_url?)` | `Link[]`: every `<a href>` and `<area href>` with its resolved URL and visible text | [Scrape links, metadata and tables](cookbook/links-metadata-tables.md) |
| `resources(html, base_url?)` | `Resource[]`: every embedded URL (images, scripts, styles, `srcset` candidates) | [Scrape links, metadata and tables](cookbook/links-metadata-tables.md) |
| `toMarkdownChunks(html, max_chars?)` | `Chunk[]`: Markdown split at headings, each with its heading path | [Turn a page into Markdown](cookbook/reader-mode.md#chunks-for-embeddings) |
| `tables(html, selector?)` | `Table[]`: each table as rows of cell strings, spans expanded | [Scrape links, metadata and tables](cookbook/links-metadata-tables.md) |
| `toCsv(html, selector?)` | RFC 4180 CSV of the matched tables | [Scrape links, metadata and tables](cookbook/links-metadata-tables.md) |

`base_url` and `selector` are optional (`null` or omitted). `max_chars` has a
default; pass it yourself so a library upgrade cannot change your chunks.

```js
import { links, mainContent, tables, toCsv, toMarkdownChunks } from "domq-wasm/extract";

links("<a href=/a>One</a>", "https://x.example/").map((l) => l.url); // ["https://x.example/a"]
tables("<table><tr><td>1<td>2</table>");                             // [[["1", "2"]]]
toCsv("<table><tr><td>1<td>2</table>");                              // "1,2\r\n"
toMarkdownChunks("<h1>A</h1><p>x</p><h2>B</h2><p>y</p>", 100).map((c) => c.context); // ["A", "A > B"]
```

## `Document`

Every function above parses its HTML again on each call. `Document` parses once,
then answers as many queries as you like. Both entries export it; the extract
entry's `Document` has the extraction methods too (marked **extract**).

```js
import { Document } from "domq-wasm";

const doc = new Document(html);
doc.select("a").length;   // 2
doc.toText();             // "One\nTwo"
doc.free();               // optional: releases the wasm memory now
```

| Member | What it does |
|---|---|
| `new Document(input)` | Parses a string or a `Uint8Array` (a full page or a fragment). Never throws on markup errors. |
| `Document.parse(input)` | The same as `new Document(input)`. |
| `Document.parseBytes(bytes)` | Parses bytes, detecting the encoding. |
| `select(selector)` | Every match, in document order. Throws `SelectorError` when invalid. |
| `selectFirst(selector)` | The first match, or `undefined`. |
| `toText(selector?)` | Readable text of the document, or of each match. |
| `toMarkdown(selector?)` | Markdown of the document, or of each match. |
| `toHtml()` | The repaired HTML. |
| `tree()` | The tree as plain objects (`DomDocument`). |
| `toJSON()` | The same as `tree()`, so `JSON.stringify(doc)` gives the JSON tree. |
| `errors()` | What the parser repaired (`ParseError[]`). |
| `free()` | Releases the wasm memory now instead of at garbage collection. |
| `[Symbol.dispose]()` | The same as `free()`, for `using doc = new Document(html)` where explicit resource management exists. |
| `mainContent()` | **extract.** The article, or `undefined`. |
| `metadata(base_url?)` | **extract.** Page metadata. |
| `links(base_url?)` | **extract.** Links with resolved URLs. |
| `resources(base_url?)` | **extract.** Embedded resource URLs. |
| `toMarkdownChunks(max_chars?)` | **extract.** Markdown chunks with heading paths. |
| `tables(selector?)` | **extract.** Tables as rows of cells. |
| `toCsv(selector?)` | **extract.** Tables as CSV. |

If you also use the browser's own `Document` in the same file, rename the
import: `import { Document as HtmlDocument } from "domq-wasm"`. A `Document`
made by one entry is not accepted by the other entry's functions.

## Types

| Type | Fields |
|---|---|
| `Match` | `tag` (lower-case), `attrs` (a `Record<string, string>`; boolean attributes are `""`), `text` (readable, as `toText` gives it), `html` (outer HTML, re-serialized), `nodeIndex` (position in a depth-first walk of `tree()`) |
| `DomDocument` | `type: "document"`, `errors` (how many repairs; `0` is clean input), `children: DomNode[]` |
| `DomNode` | one of `DomElement`, `{ type: "text", value }`, `{ type: "comment", value }`, `{ type: "doctype", value }` |
| `DomElement` | `type: "element"`, `tag`, `attrs`, `children: DomNode[]` |
| `ParseError` | `position` (index into the input string), `message` (for example `"element implicitly closed"`) |
| `SelectorError` | an `Error` with `name === "SelectorError"` and `position` (index into the selector) |
| `Decoded` | `text`, `encoding` (for example `"windows-1252"`), `warning` (`string \| null`) |
| `MainContent` | **extract.** `text`, `markdown`, `html` |
| `Metadata` | **extract.** `title`, `description`, `canonical`, `lang`, `language`, `siteName`, `image`, `author`, `published` (each `string \| null`), `keywords`, `jsonLd`, `feeds` (`string[]`), `og`, `twitter` (`[string, string][]`) |
| `Link` | **extract.** `url` (resolved), `text`, `href` (as written) |
| `Resource` | **extract.** `url` (resolved), `attr`, `value` (as written), `tag` |
| `Chunk` | **extract.** `markdown`, `text` (the same), `headingPath`, `context` (the path joined with `" > "`), `byteSize` |
| `Table` | **extract.** `string[][]`: rows of cells |

```js
import { select } from "domq-wasm";

try {
  select("<a>x</a>", "a:hover");
} catch (e) {
  e.name;     // "SelectorError"
  e.position; // 1
  e.message;  // "unsupported pseudo-class ':hover' at byte 1"
}
```

## Not in JavaScript

The page outline, `domq lint`, editing a document and declarative extraction
into structs are in the other surfaces: the [command line](cli.md), the [MCP
server](mcp.md), Rust and [Python](python.md).
