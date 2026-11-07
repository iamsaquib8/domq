// Smoke test of the *built npm package* in Node.js, imported by its name the
// way users do (so the "exports" map and the Node entry points are exercised).
// The package has two entries: `domq-wasm` (the core) and `domq-wasm/extract`
// (the core plus extraction). `npm/smoke-test.sh` packs pkg/, installs the
// tarball and runs this file:
//
//   node npm/build.mjs && sh npm/smoke-test.sh
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname } from "node:path";
import { pathToFileURL } from "node:url";

// 1. Plain ESM import of the core: ready to use, no init call.
import init, {
  Document,
  decode,
  errors,
  parse,
  parseBytes,
  parseJson,
  resolveSrcset,
  resolveUrl,
  select,
  selectFirst,
  toHtml,
  toMarkdown,
  toText,
} from "domq-wasm";
import * as core from "domq-wasm";
// The extraction API lives in the second entry (a superset of the core).
import * as full from "domq-wasm/extract";
import {
  Document as FullDocument,
  links,
  mainContent,
  metadata,
  resources,
  tables,
  toCsv,
  toMarkdownChunks,
} from "domq-wasm/extract";

// 0. What each entry exports. The core must NOT have the extraction API;
//    the extract entry has everything the core has, and the extraction on top.
const extraction = ["links", "mainContent", "metadata", "resources", "tables", "toCsv", "toMarkdownChunks"];
for (const name of extraction) {
  assert.equal(name in core, false, `the core must not export ${name}`);
  assert.equal(typeof full[name], "function", `domq-wasm/extract must export ${name}`);
  assert.equal(name in Document.prototype, false, `the core Document must not have ${name}()`);
  assert.equal(typeof FullDocument.prototype[name], "function", `the extract Document must have ${name}()`);
}
for (const name of Object.keys(core)) {
  assert.ok(name in full, `domq-wasm/extract must also export ${name}`);
}
assert.equal(typeof core.mainContent, "undefined");

const html = `<ul id=list><li class=item>One<li>Two &amp; <b>more</b></ul>`;

assert.equal(parse(html).children[0].tag, "ul");
assert.equal(parse(html).type, "document");
assert.equal(typeof parseJson(html), "string");
assert.deepEqual(JSON.parse(parseJson(html)), parse(html));

assert.deepEqual(select(html, "li").map((m) => m.text), ["One", "Two & more"]);
assert.deepEqual(selectFirst(html, ".item"), {
  tag: "li",
  attrs: { class: "item" },
  text: "One",
  html: `<li class="item">One</li>`,
  nodeIndex: 1,
});
assert.equal(selectFirst(html, "table"), undefined);
assert.equal(toText(html), "One\nTwo & more");
assert.equal(toText(html, "li"), "One\n\nTwo & more");
assert.equal(toMarkdown(html), "- One\n- Two & **more**");
assert.equal(toMarkdown(html, "b"), "**more**");
assert.equal(
  toHtml(html),
  `<ul id="list"><li class="item">One</li><li>Two &amp; <b>more</b></li></ul>`,
);
assert.deepEqual(errors("<p>ok</p>"), []);
assert.deepEqual(errors("<b><i>x</b></i>")[0], {
  position: 7,
  message: "end tag closed mis-nested elements",
});

// 2. Data extraction parity: reader mode, metadata, links, chunks, tables, and URL helpers
const articleHtml = `<html><head><title>Blog</title></head><body>
  <nav><a href="/">Home</a></nav>
  <div class="post">
    <h1>Understanding domq</h1>
    <p>domq provides Rust bindings for WebAssembly, allowing you to write native extensions with ease.</p>
    <p>It provides memory safety, zero JS dependencies, and fast HTML extraction at the edge.</p>
  </div>
  <div id="comments"><p>First comment here!</p></div>
  <footer>Copyright 2026</footer>
</body></html>`;
const mc = mainContent(articleHtml);
assert.ok(mc);
assert.ok(mc.text.includes("Understanding domq"));
assert.ok(mc.markdown.includes("# Understanding domq"));
assert.ok(mc.html.includes("class=\"post\""));
assert.equal(mainContent("<nav><a href='/'>Only Nav</a></nav>"), undefined);

const metaHtml = `<html lang="en"><head><title>Title</title><meta name="description" content="Desc"><link rel="canonical" href="sub/doc"><script type="application/ld+json">{"@type":"Blog"}</script></head></html>`;
const meta = metadata(metaHtml, "https://example.com/base/");
assert.equal(meta.title, "Title");
assert.equal(meta.description, "Desc");
assert.equal(meta.canonical, "https://example.com/base/sub/doc");
assert.equal(meta.lang, "en");
assert.equal(meta.language, "en");
assert.deepEqual(meta.jsonLd, ['{"@type":"Blog"}']);

const linkHtml = `<a href="page.html">Link text</a><img src="pic.jpg">`;
const linkList = links(linkHtml, "https://example.com/");
assert.deepEqual(linkList, [{ url: "https://example.com/page.html", text: "Link text", href: "page.html" }]);
const resList = resources(linkHtml, "https://example.com/");
assert.deepEqual(resList, [{ url: "https://example.com/pic.jpg", attr: "src", value: "pic.jpg", tag: "img" }]);

const chunkHtml = `<h1>Section A</h1><p>Text A</p><h2>Section B</h2><p>Text B</p>`;
const chunkList = toMarkdownChunks(chunkHtml, 30);
assert.ok(chunkList.length >= 2);
assert.ok(chunkList[0].markdown);
assert.ok(chunkList[0].headingPath);
assert.equal(typeof chunkList[0].byteSize, "number");

const tableHtml = `<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>`;
assert.deepEqual(tables(tableHtml), [[["A", "B"], ["1", "2"]]]);
assert.equal(toCsv(tableHtml), "A,B\r\n1,2\r\n");

assert.equal(resolveUrl("https://example.com/a/", "b"), "https://example.com/a/b");
assert.equal(resolveSrcset("https://example.com/", "x.png 1x"), "https://example.com/x.png 1x");

// 3. Invalid selectors throw a real Error with a position.
assert.throws(() => select(html, "li:hover"), (e) => {
  assert.ok(e instanceof Error);
  assert.equal(e.name, "SelectorError");
  assert.equal(e.message, "unsupported pseudo-class ':hover' at byte 2");
  assert.equal(e.position, 2);
  return true;
});
assert.throws(() => toText(html, "ul >"), { name: "SelectorError", position: 4 });

// 4. The Document class: parse once, query many times.
const doc = new Document(html);
assert.equal(doc.select("li").length, 2);
assert.equal(doc.selectFirst("b").text, "more");
assert.equal(doc.toText("b"), "more");
assert.equal(doc.toMarkdown(), toMarkdown(html));
assert.equal(doc.toHtml(), toHtml(html));
assert.deepEqual(doc.tree(), parse(html));
assert.equal(JSON.stringify(doc), parseJson(html)); // toJSON()
assert.deepEqual(doc.errors(), errors(html));

const richDoc = new FullDocument(articleHtml + metaHtml + linkHtml + tableHtml);
assert.ok(richDoc.mainContent());
assert.equal(richDoc.metadata("https://example.com/base/").title, "Blog");
assert.equal(richDoc.links("https://example.com/").length, 2);
assert.ok(richDoc.resources("https://example.com/").length >= 1);
assert.ok(richDoc.toMarkdownChunks(30).length >= 1);
assert.deepEqual(richDoc.tables(), [[["A", "B"], ["1", "2"]]]);
assert.equal(richDoc.toCsv(), "A,B\r\n1,2\r\n");
richDoc.free();

doc.free();
const doc2 = Document.parse(html);
assert.equal(doc2.select("#list > li").length, 2);
doc2.free();

// Bytes-in, Uint8Array and decoding
const utf8Bytes = Buffer.from("<p class='item'>hello bytes</p>");
assert.equal(parse(utf8Bytes).children[0].children[0].value, "hello bytes");
assert.equal(parseBytes(utf8Bytes).children[0].children[0].value, "hello bytes");

const latin1Bytes = Buffer.from("<meta charset=latin1><p>caf\xe9</p>", "latin1");
assert.equal(parse(latin1Bytes).children[1].children[0].value, "café");

const dec = decode(Buffer.from("<p>café</p>", "utf8"));
assert.equal(dec.text, "<p>café</p>");
assert.equal(dec.encoding, "utf-8");
assert.equal(dec.warning, null);

const decWin = decode(Buffer.from("<meta charset=latin1>\xe9", "latin1"));
assert.equal(decWin.encoding, "windows-1252");

const decStray = decode(Buffer.concat([Buffer.from("<p>café</p>", "utf8"), Buffer.from([0xa0])]));
assert.equal(decStray.encoding, "utf-8");
assert.equal(decStray.warning, "1 invalid UTF-8 sequence replaced with U+FFFD");

const decLatin1 = decode(Buffer.from("<p>caf\xe9</p>", "latin1"));
assert.equal(decLatin1.encoding, "windows-1252");
assert.match(decLatin1.warning, /windows-1252/);

const bytesDoc = new Document(utf8Bytes);
assert.equal(bytesDoc.toText(), "hello bytes");
bytesDoc.free();

const bytesDoc2 = Document.parse(utf8Bytes);
assert.equal(bytesDoc2.toText(), "hello bytes");
bytesDoc2.free();

const bytesDoc3 = Document.parseBytes(utf8Bytes);
assert.equal(bytesDoc3.toText(), "hello bytes");
bytesDoc3.free();

// 4. Browser-style code still works: init() is a no-op here.
await init();
assert.equal(toText("<p>after init</p>"), "after init");

// 5. CommonJS: require() works where Node can require ES modules (20.19+,
//    22.12+); import() works everywhere.
const require = createRequire(import.meta.url);
const viaImport = await import("domq-wasm");
assert.equal(viaImport.toText("<i>import()</i>"), "import()");
let viaRequire = "require() unsupported on this Node";
try {
  viaRequire = require("domq-wasm").toText("<i>require() works</i>");
  assert.equal(require("domq-wasm/extract").links("<a href=x>y</a>").length, 1);
} catch (e) {
  assert.equal(e.code, "ERR_REQUIRE_ESM");
}

// 6. The plain web builds, initialised by hand (what node.js does for you).
//    The query string gives a fresh, not yet initialised module instance.
for (const [entry, wasmEntry] of [
  ["domq-wasm/domq_wasm.js", "domq-wasm/domq_wasm_bg.wasm"],
  ["domq-wasm/extract/domq_wasm.js", "domq-wasm/extract/domq_wasm_bg.wasm"],
]) {
  const web = await import(`${pathToFileURL(require.resolve(entry))}?fresh`);
  assert.throws(() => web.toText("<p>x</p>")); // not initialised yet
  await web.default({ module_or_path: readFileSync(require.resolve(wasmEntry)) });
  assert.equal(web.toText("<p>web build</p>"), "web build");
}

// 7. The package ships exactly the two entries, and each .d.ts describes only
//    what its entry exports.
const root = dirname(require.resolve("domq-wasm/package.json"));
const coreTypes = readFileSync(`${root}/domq_wasm.d.ts`, "utf8");
const fullTypes = readFileSync(`${root}/extract/domq_wasm.d.ts`, "utf8");
for (const type of ["MainContent", "Metadata", "Link", "Resource", "Chunk", "Table"]) {
  assert.doesNotMatch(coreTypes, new RegExp(`(?:interface|type) ${type}\\b`), `core types declare ${type}`);
  assert.match(fullTypes, new RegExp(`(?:interface|type) ${type}\\b`), `extract types lack ${type}`);
}

console.log(`domq-wasm OK on Node ${process.version} (${viaRequire})`);
console.log(select(html, "li").map((m) => m.html).join("\n"));
