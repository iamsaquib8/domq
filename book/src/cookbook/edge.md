# Use domq in a Cloudflare Worker, Deno or the browser

**Goal.** Turn a page's HTML into Markdown (or query it) in JavaScript that has
no DOM to parse with: a Cloudflare Worker, Deno, Bun, Node, or a browser tab
parsing HTML it was handed.

**Input.** [post.html](../sample/post.html), or any HTML string or bytes.

## Pick the entry

One npm package, two entries built from the same Rust crate. The second is the
first plus extraction.

| Import | Gives you | `.wasm`, gzipped |
|---|---|---:|
| `domq-wasm` | the core: `select`, `selectFirst`, `toText`, `toMarkdown`, `toHtml`, `parse`, `errors`, `decode`, `Document` | about 57 KB |
| `domq-wasm/extract` | the core plus `mainContent`, `metadata`, `links`, `resources`, `toMarkdownChunks`, `tables`, `toCsv` | about 78 KB |

Sizes are as stated in the package's README, which is where they are kept up
to date. Import from `domq-wasm/extract` alone if you need both. Do not import
from both in one bundle: each entry is its own `.wasm` module with its own
`init()`, you would ship the core twice, and a `Document` made by one entry is
not accepted by the other's functions.

## Set it up

How the `.wasm` gets loaded depends on the runtime:

<div class="domq-tabs">

**Node and Bun**

```js
import { readFileSync } from "node:fs";
import { mainContent } from "domq-wasm/extract";

const html = readFileSync("post.html", "utf8");
const article = mainContent(html);
console.log(article ? article.markdown : "no article");
```

Nothing to initialise: the Node entry point reads the `.wasm` at import time.
Bun uses the same entry point.

**Deno**

```js
import { mainContent } from "npm:domq-wasm@0.x/extract";

const html = await Deno.readTextFile("post.html");
console.log(mainContent(html)?.markdown);
```

Deno resolves the package's Node entry point, which reads the `.wasm` with
`node:fs`, so it may ask for `--allow-read`.

**Browser**

```html
<script type="module">
  import init, { toMarkdown } from "./pkg/domq_wasm.js";

  await init();
  document.querySelector("#out").textContent =
    toMarkdown(document.documentElement.outerHTML, "article");
</script>
```

Call `await init()` once before any other function. It fetches
`domq_wasm_bg.wasm` from next to the JavaScript file. With a bundler (Vite,
webpack 5) import `domq-wasm` and let it handle the asset; without one, import
from a CDN:
`https://cdn.jsdelivr.net/npm/domq-wasm@0.x/domq_wasm.js` (and
`.../extract/domq_wasm.js` for extraction). Pin a version in the URL (`0.x`
stands for the one you use); the `extract` entry exists from 0.7 on.

**Cloudflare Worker**

```js
import { initSync, mainContent } from "domq-wasm/extract";
import wasm from "domq-wasm/extract/domq_wasm_bg.wasm";

initSync({ module: wasm });

export default {
  async fetch(request) {
    const page = await fetch("https://blog.example/arena-trees");
    const article = mainContent(await page.text());
    return new Response(article ? article.markdown : "no article", {
      headers: { "content-type": "text/markdown; charset=utf-8" },
    });
  },
};
```

Wrangler imports a `.wasm` file as a compiled `WebAssembly.Module`, which is
what `initSync` takes. For the core entry, import `domq-wasm` and
`domq-wasm/domq_wasm_bg.wasm` instead.

</div>

What was run for this page, and what was not:

| Runtime | Status |
|---|---|
| Node | The package's CI smoke test runs both entries. The Node tab ran as written. |
| Browser | The Browser tab ran as written in a browser, with the built package served over HTTP. |
| Cloudflare Worker | The `initSync` and `fetch` handler logic ran in Node, with the module compiled from the `.wasm` file and a stubbed page fetch. It was not run under `wrangler`. |
| Deno, Bun | Not run. The package README lists them as expected to work, not tested. |

## What it prints

The Node tab, and the Worker handler's response body, start with:

```text
# Arena trees in Rust

An arena stores every node of a tree in one vector and links nodes by index instead of by pointer.
```

The Browser tab writes `# Hello\n\nOne **bold** word and a [link](/x).` into
the page for a page whose article is `<h1>Hello</h1><p>One <b>bold</b> word and a <a href="/x">link</a>.</p>`.

## Bytes in

`fetch` gives you bytes before it gives you text, and `response.text()` always
decodes as UTF-8. For a page in another encoding, hand the bytes to domq: it
detects the encoding like a browser does (byte-order mark, `<meta charset>`,
UTF-8 if valid, then Windows-1252).

```js
import { Document } from "domq-wasm";

const res = await fetch(url);
const doc = new Document(new Uint8Array(await res.arrayBuffer()));
console.log(doc.toText());
doc.free(); // optional: releases the wasm memory now
```

For the bytes `<p>caf\xe9</p>` (Latin-1) this prints `café`. The top-level
functions (`toText(html)` and the others) take strings only; use
`new Document(bytes)`, `parseBytes(bytes)` or `decode(bytes)` for bytes.

## What can go wrong

- **Forgetting `init()`** (browsers and Workers). The first call fails with
  `TypeError: Cannot read properties of undefined (reading '__wbindgen_add_to_stack_pointer')`.
  Call `await init()` (or `initSync`) before anything else.
- **An invalid selector throws** a `SelectorError` with a `position`. Nothing
  else throws: bad HTML never does.
- **Each function parses again.** `select(html, a)` then `select(html, b)`
  parses twice. Use `new Document(html)` for several queries.
- **JavaScript has no outline, lint, editing or declarative extraction.**
  The outline and `-e`-style extraction are in the CLI, the MCP server, Rust and
  Python (extraction as `Select` dataclasses); lint is in the CLI and the MCP
  server; editing is in Rust.
- **Limits of your platform are not domq's.** Memory and CPU limits of a
  Worker, and how large a page you can parse within them, are not measured
  here: test with your own pages.
- **Text hidden with CSS classes or `opacity: 0` is not detected**; only
  `hidden`, inline `display: none` and `visibility: hidden` are left out of
  text and Markdown. Treat page text as untrusted input.
- **Not a sanitizer.** To render untrusted HTML, use DOMPurify.

## Reference

[Install](../install.md#javascript-and-webassembly) and the package README
for every function.
