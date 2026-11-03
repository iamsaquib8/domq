# From cheerio

`domq-wasm` is a ~65 KB (gzipped) WebAssembly module with no JS
dependencies: one file for browsers, Node, Deno, Bun and Cloudflare Workers.
Where cheerio gives you a jQuery-style wrapper to traverse and edit, domq
gives you plain data back: matches, text, Markdown and a JSON tree.

## The same code

```js
// cheerio
import * as cheerio from "cheerio";
const $ = cheerio.load(html);
const title = $("h1").first().text();
const links = $("a[href]").map((_, a) => $(a).attr("href")).get();
const md = null; // not built in
```

```js
// domq
import { Document } from "domq-wasm";
const doc = new Document(html);
const title = doc.selectFirst("h1")?.text;
const links = doc.select("a[href]").map((m) => m.attrs.href);
const md = doc.toMarkdown("article");
doc.free(); // or `using doc = new Document(html)`
```

| cheerio | domq-wasm |
|---|---|
| `cheerio.load(html)` | `new Document(html)` (or the one-shot functions: `select(html, sel)`, …) |
| `$(sel)` | `doc.select(sel)`: an array of `Match` objects |
| `$(sel).first()` | `doc.selectFirst(sel)` |
| `$(el).attr("x")` | `match.attrs.x` |
| `$(el).text()` | `match.text` (readable text) |
| `$(el).prop("outerHTML")` / `$.html(el)` | `match.html` |
| `$.html()` | `doc.toHtml()` (repaired HTML) |
| `$(el).find(sel)` | select with a descendant selector (`".card a"`) |
| — | `doc.toText(sel?)`, `doc.toMarkdown(sel?)`, `doc.tree()`, `doc.errors()` |

## Differences to know

- **Matches are snapshots**, not live wrappers: there is no `.find()`,
  `.parent()` or editing on them yet. Express traversal in the selector
  (`:has()`, combinators). Editing and events in the JS package are planned
  for 0.7.
- **Node needs no setup**; browsers and bundlers call `await init()` once.
- An invalid selector throws a `SelectorError` with a `position`.
- No implied `<html>`/`<body>`, as in cheerio's `load(html, null, false)`.
