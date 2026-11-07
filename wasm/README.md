# domq for JavaScript

**Messy HTML in. Clean tree, text, Markdown, or JSON out.** A forgiving HTML
parser with CSS selectors for JavaScript: browsers, Node, Deno, Bun and edge
workers. It is the Rust crate [`domq`](https://github.com/iamsaquib8/domq)
compiled to a **~57 KB (gzipped)** WebAssembly module (or **~78 KB** with
data extraction: reader mode, metadata, links, chunks, tables), with zero JS
dependencies.

**[Try it in the playground](https://iamsaquib8.github.io/domq/)**: paste
any HTML and query it live.

```js
import { select, toMarkdown } from "domq-wasm";

const html = `<ul id=links><li><a href="/a">One</a><li><a href="/b">Two</a></ul>`;

select(html, "#links a[href]").map((a) => a.attrs.href); // ["/a", "/b"]
toMarkdown(html); // "- [One](/a)\n- [Two](/b)"
```

- **Never throws on bad HTML.** Unclosed tags, mis-nesting and stray end tags
  are repaired, and `errors()` tells you where.
- **Real CSS selectors.** Combinators, attribute operators, `:nth-child()`,
  `:not()`, `:is()`, `:has()`, plus jQuery-style `:contains("text")`.
- **Content out.** Readable text (no scripts or styles), CommonMark/GFM Markdown
  for LLM prompts and RAG, a JSON-ready tree, or repaired HTML.
- **Small.** The core `.wasm` is ~57 KB gzipped plus ~5 KB of generated glue
  JS. Extraction (reader mode, metadata, links, chunks, tables) is a second,
  optional entry, `domq-wasm/extract`, for ~78 KB in all.

It is not a spec-compliant WHATWG parser (no implicit `<html>`/`<body>`, no
adoption agency) and not a sanitizer. Use DOMPurify for untrusted HTML that you
render.

## Install

```sh
npm install domq-wasm
```

## What to import from where

One npm package, two entries. Both are built from the same Rust crate; the
second is the first plus extraction.

| Import | What you get | `.wasm` gzipped |
|---|---|---:|
| `domq-wasm` | **Core:** `parse`, `parseJson`, `parseBytes`, `select`, `selectFirst`, `toText`, `toMarkdown`, `toHtml`, `errors`, `decode`, `resolveUrl`, `resolveSrcset`, and the parse-once `Document` | 56,647 B |
| `domq-wasm/extract` | **Everything in the core, plus extraction:** `mainContent`, `metadata`, `links`, `resources`, `toMarkdownChunks`, `tables`, `toCsv` (also as `Document` methods) | 77,769 B |

```js
import { select, toMarkdown } from "domq-wasm"; // the core
import { mainContent, metadata, links } from "domq-wasm/extract"; // extraction
```

`domq-wasm/extract` exports everything `domq-wasm` does, so import from it
alone if you want both. Don't import from both in one bundle: each entry is its
own `.wasm` module, so you would ship the core twice, each entry has its own
`init()` (browsers and Workers), and a `Document` made by one entry is not
accepted by the other's functions.

The two entries have separate TypeScript declarations. The core's
`domq_wasm.d.ts` declares none of the extraction API or its types
(`MainContent`, `Metadata`, `Link`, `Resource`, `Chunk`, `Table`), so a missing
import is a compile error rather than a runtime surprise.

## Use it

Each entry is an ES module. How it loads the `.wasm` depends on the runtime:

| Runtime | Setup | Status |
|---|---|---|
| Node.js 22+ (supported LTS lines) | nothing: `import { select } from "domq-wasm"` (or `"domq-wasm/extract"`) | tested in CI on the current LTS |
| Browsers, bundlers (Vite, webpack 5, …) | `await init()` once | tested (the playground uses this build) |
| Bun | nothing (uses the Node entry point) | expected to work, not tested |
| Deno | nothing with `npm:domq-wasm` (Node entry point) | expected to work, not tested |
| Cloudflare Workers | `initSync({ module })` with the imported `.wasm` | expected to work, not tested |

### Node.js

Import and call. The Node entry point loads the module synchronously at import
time, so there is no `init()` to await (calling it anyway is harmless).

```js
import { select, toText } from "domq-wasm";

const res = await fetch("https://news.ycombinator.com");
const html = await res.text();
for (const link of select(html, ".titleline > a")) {
  console.log(link.text, link.attrs.href);
}
```

From CommonJS, `require("domq-wasm")` works on Node versions that can
`require()` ES modules (20.19+ and 22.12+). On older versions, use
`const { select } = await import("domq-wasm")`.

### Browsers and bundlers

Call the default export once before anything else. It fetches
`domq_wasm_bg.wasm` from next to the JS file, which bundlers such as Vite
and webpack 5 handle as an asset.

```js
import init, { select, toMarkdown } from "domq-wasm";

await init();
const md = toMarkdown(document.documentElement.outerHTML, "article");
```

Without a bundler, import it from a CDN:

```html
<script type="module">
  import init, { toText } from "https://cdn.jsdelivr.net/npm/domq-wasm@0.7/domq_wasm.js";
  await init();
  console.log(toText("<p>Hello <b>world</b>"));
</script>
```

For extraction, load `domq-wasm@0.7/extract/domq_wasm.js` instead (it loads
`extract/domq_wasm_bg.wasm`).

### Deno and Bun

```js
import { select } from "npm:domq-wasm@0.7"; // Deno; in Bun: "domq-wasm"
```

Both resolve the package's Node entry point (also for `domq-wasm/extract`),
which reads the `.wasm` with `node:fs`. Deno may ask for read permission (`--allow-read`).

### Cloudflare Workers

Wrangler imports `.wasm` files as compiled `WebAssembly.Module`s; hand it to
`initSync`:

```js
import { initSync, toMarkdown } from "domq-wasm";
import wasm from "domq-wasm/domq_wasm_bg.wasm";

initSync({ module: wasm });

export default {
  async fetch(request) {
    const page = await fetch("https://blog.rust-lang.org");
    return new Response(toMarkdown(await page.text(), "main"));
  },
};
```

For extraction, import from `domq-wasm/extract` and
`domq-wasm/extract/domq_wasm_bg.wasm` instead.

## API

Every function takes the HTML as a string. Functions that take a `selector`
throw a `SelectorError` if it is invalid; nothing else throws.

### Core: `domq-wasm`

| Function | Returns |
|---|---|
| `parse(html)` | `DomDocument`: the tree as plain objects |
| `parseJson(html)` | `string`: the same tree as JSON text |
| `select(html, selector)` | `Match[]`: every matching element, in document order |
| `selectFirst(html, selector)` | `Match \| undefined`: the first match |
| `toText(html, selector?)` | `string`: readable text of the document, or of every match separated by a blank line |
| `toMarkdown(html, selector?)` | `string`: Markdown of the document, or of every match separated by a blank line |
| `toHtml(html)` | `string`: the repaired HTML (every element closed and properly nested) |
| `errors(html)` | `ParseError[]`: what the parser had to repair, and where |
| `decode(bytes, label?)` | `Decoded`: bytes to text with WHATWG encoding sniffing (`text`, `encoding`, `warning`) |
| `resolveUrl(base, relative)` | `string`: resolved URL (RFC 3986 §5.2) |
| `resolveSrcset(base, srcset)` | `string`: resolved URLs in `srcset` |
| `init(input?)`, `initSync({ module })` | load the `.wasm` (browsers and Workers only; see above) |

`parse(html)` and `new Document(html)` also accept a `Uint8Array`, and
`parseBytes(bytes)` always takes one: the encoding is sniffed as `decode()`
does.

### Extraction: `domq-wasm/extract` only

| Function | Returns |
|---|---|
| `mainContent(html)` | `MainContent \| undefined`: reader mode article body (`text`, `markdown`, `html`) |
| `metadata(html, baseUrl?)` | `Metadata`: title, description, canonical, language, OpenGraph, Twitter Cards, JSON-LD, feeds |
| `links(html, baseUrl?)` | `Link[]`: resolved links (`url`, `text`, `href`) |
| `resources(html, baseUrl?)` | `Resource[]`: embedded URLs (`url`, `attr`, `value`, `tag`) |
| `toMarkdownChunks(html, maxChars?)` | `Chunk[]`: Markdown chunks with heading path and byte sizes for RAG |
| `tables(html, selector?)` | `Table[]`: rectangularized 2D arrays of cell strings |
| `toCsv(html, selector?)` | `string`: RFC 4180 CSV representation of tables |

### Parse once, query many times

Each function above parses the HTML again. To run several queries on one
document, parse it once:

```js
import { Document } from "domq-wasm";

const doc = new Document(html); // or Document.parse(html)
const title = doc.selectFirst("h1")?.text;
const prices = doc.select("tr:has(td.in-stock) .price").map((m) => m.text);
const body = doc.toMarkdown("article");
doc.free(); // release its wasm memory now instead of waiting for the GC
```

`Document` has the same operations as methods. In the core: `select(selector)`,
`selectFirst(selector)`, `toText(selector?)`, `toMarkdown(selector?)`,
`toHtml()`, `tree()`, `errors()` and `free()`. In `domq-wasm/extract`, also
`mainContent()`, `metadata(baseUrl?)`, `links(baseUrl?)`, `resources(baseUrl?)`,
`toMarkdownChunks(maxChars?)`, `tables(selector?)` and `toCsv(selector?)`.
`toJSON()` returns the same as
`tree()`, so `JSON.stringify(doc)` gives the JSON tree. Where explicit resource
management is available, `using doc = new Document(html)` frees it
automatically.

If you also use the DOM's `Document` type in the same file, rename the import:
`import { Document as HtmlDocument } from "domq-wasm"`.

### Types

The package includes TypeScript declarations for both entries; none of the API
is typed `any`. The core's declarations cover the types below. The extraction
types (`MainContent`, `Metadata`, `Link`, `Resource`, `Chunk`, `Table`) are
declared by `domq-wasm/extract` only.

```ts
interface Match {
  tag: string;                   // lower-cased tag name
  attrs: Record<string, string>; // attributes; boolean attributes are ""
  text: string;                  // readable text, as toText() gives it
  html: string;                  // outer HTML, re-serialized
  nodeIndex: number;             // position in a depth-first walk of tree()
}

interface DomDocument {
  type: "document";
  errors: number;                // how many repairs (0 = clean input)
  children: DomNode[];
}

type DomNode =
  | { type: "element"; tag: string; attrs: Record<string, string>; children: DomNode[] }
  | { type: "text"; value: string }
  | { type: "comment"; value: string }
  | { type: "doctype"; value: string };

interface ParseError {
  position: number;              // index into the html string
  message: string;               // e.g. "element implicitly closed"
}

interface SelectorError extends Error {
  name: "SelectorError";
  position: number;              // index into the selector string
}
```

### Invalid selectors

```js
try {
  select(html, "a:hover");
} catch (e) {
  e.name;     // "SelectorError"
  e.message;  // "unsupported pseudo-class ':hover' at byte 1"
  e.position; // 1
}
```

Unsupported selectors are errors rather than silently matching nothing:
pseudo-elements, `:hover` and other interactive states, namespaces, and
`:has()` inside `:has()`. The full grammar is in the
[`Selector` docs](https://docs.rs/domq/latest/domq/struct.Selector.html).

## Size

Measured with `gzip -9` on the files in the package:

| Entry | `.wasm` raw | `.wasm` gzipped | glue JS gzipped | CI gate (`.wasm`, gzipped) |
|---|---:|---:|---:|---:|
| `domq-wasm` (core) | 128,838 B | **56,647 B** | 5,152 B | < 60,000 B |
| `domq-wasm/extract` (core + extraction) | 178,270 B | **77,769 B** | 5,886 B | < 80,000 B |

The core holds the parser, the selector engine, bytes-in decoding, and the
text, Markdown, JSON and HTML serializers. `domq-wasm/extract` adds data
extraction: reader mode, metadata (with JSON-LD), links, resources, chunks and
tables. Extraction costs ~21 KB gzipped, which is why it is a separate entry:
an app that only selects and converts no longer ships it. CI fails if either
module grows past its gate; the extract gate (80,000 B) is temporary, the core
one is not.

For comparison, 0.5 was ~43 KB gzipped, before configurable Markdown options,
extraction, bytes-in decoding and the script-data tokenizer states landed. The
core is therefore about 10 KiB over the old 45 KiB goal, not under it; the
[ROADMAP budgets](https://github.com/iamsaquib8/domq/blob/main/ROADMAP.md#budgets)
say where the bytes went.

## Upgrading from 0.6

The extraction API moved out of the root entry. In 0.6 `domq-wasm` exported
everything; in 0.7 it exports the core only, and `mainContent`, `metadata`,
`links`, `resources`, `toMarkdownChunks`, `tables` and `toCsv` (the free
functions and the `Document` methods) come from `domq-wasm/extract`:

```diff
-import { select, mainContent, metadata } from "domq-wasm";
+import { select } from "domq-wasm";
+import { mainContent, metadata } from "domq-wasm/extract";
```

If you use extraction, the easiest change is to import everything from
`domq-wasm/extract`, which exports the core too:

```diff
-import { select, mainContent, toMarkdown } from "domq-wasm";
+import { select, mainContent, toMarkdown } from "domq-wasm/extract";
```

The same goes for deep imports: `domq-wasm/domq_wasm_bg.wasm` is the core's
module, `domq-wasm/extract/domq_wasm_bg.wasm` the full one. Nothing else
changed: the functions and their results are the same in both entries.

## Upgrading from 0.1 or 0.2

0.3 is a new API. `parse()` now returns objects, so drop the `JSON.parse`, or
call `parseJson()` for the string. `parseErrorCount(html)` is now
`errors(html).length`. `query(html, selector)` is now `select(html, selector)`,
which returns `Match` objects instead of a JSON string of subtrees.

## Building from source

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

cd wasm
node npm/build.mjs                       # core -> pkg/, extract -> pkg/extract/, then the Node entry points + "exports"
wasm-pack test --node                    # Rust-side tests of the bindings (the extract build)
wasm-pack test --node -- --no-default-features   # the same, for the core build
sh npm/smoke-test.sh                     # pack pkg/ and run tests/node.mjs against both entries
```

`npm/build.mjs` runs `wasm-pack build --target web --release` twice: with
`--no-default-features` for the core, and with the default `extract` cargo
feature for the full build.

For the playground (it uses the full build):
`wasm-pack build --target web --release --no-pack --out-dir ../playground/pkg`,
then serve `playground/` over HTTP (for example `python3 -m http.server -d ../playground`).

## License

MIT
