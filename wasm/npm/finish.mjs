// Turn the two wasm-pack `--target web` outputs into the published npm
// package `domq-wasm`: add a Node.js entry point to each build and write the
// "exports" map. `npm/build.mjs` runs this after building both:
//
//   pkg/            the core build (parser, selectors, text/Markdown/JSON/HTML):
//                   `import … from "domq-wasm"`
//   pkg/extract/    the full build (the core plus extraction: mainContent,
//                   metadata, links, resources, toMarkdownChunks, tables,
//                   toCsv): `import … from "domq-wasm/extract"`
//
// Node and Bun get the entry point that loads the .wasm synchronously; every
// other environment gets the plain ES module (call `await init()`).
//
//   node npm/finish.mjs [pkg-dir]
import { copyFileSync, existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = new URL(".", import.meta.url);
const pkgDir = resolve(process.argv[2] ?? fileURLToPath(new URL("../pkg", here)));
const extractDir = resolve(pkgDir, "extract");
const manifestPath = resolve(pkgDir, "package.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));

for (const dir of [pkgDir, extractDir]) {
  if (!existsSync(resolve(dir, "domq_wasm_bg.wasm"))) {
    throw new Error(`${dir} has no domq_wasm_bg.wasm: run npm/build.mjs (it builds both entries)`);
  }
  // Each entry's node.js loads the .wasm that sits next to it.
  copyFileSync(new URL("node.js", here), resolve(dir, "node.js"));
}
// wasm-pack leaves a `.gitignore` ("*") in its out-dir; keep it out of the
// package so the nested entry's files are never skipped.
rmSync(resolve(extractDir, ".gitignore"), { force: true });
// npm always packs a file named LICENSE; make sure it is there whatever
// wasm-pack copied.
copyFileSync(new URL("../LICENSE", here), resolve(pkgDir, "LICENSE"));

/** The exports of one entry: its types, its Node shim and its web module. */
const entry = (prefix) => ({
  types: `./${prefix}domq_wasm.d.ts`,
  node: `./${prefix}node.js`,
  default: `./${prefix}domq_wasm.js`,
});

const ordered = {};
// The npm package is `domq-wasm`, like the crate (npm reserves plain `domq`
// as too similar to an existing package).
manifest.name = "domq-wasm";
// npm normalises this on publish (and warns); write it in its canonical form.
if (typeof manifest.repository?.url === "string" && manifest.repository.url.startsWith("https://")) {
  manifest.repository.url = `git+${manifest.repository.url.replace(/\.git$/, "")}.git`;
}
for (const [key, value] of Object.entries(manifest)) {
  ordered[key] = value;
  if (key === "types") {
    ordered.exports = {
      ".": entry(""),
      "./extract": entry("extract/"),
      // The plain ES modules, for loading the .wasm by hand (`init`, `initSync`).
      "./domq_wasm.js": { types: "./domq_wasm.d.ts", default: "./domq_wasm.js" },
      "./extract/domq_wasm.js": { types: "./extract/domq_wasm.d.ts", default: "./extract/domq_wasm.js" },
      "./domq_wasm_bg.wasm": "./domq_wasm_bg.wasm",
      "./extract/domq_wasm_bg.wasm": "./extract/domq_wasm_bg.wasm",
      "./package.json": "./package.json",
    };
  }
}
const own = (name) => [
  `${name}domq_wasm_bg.wasm`,
  `${name}domq_wasm.js`,
  `${name}domq_wasm.d.ts`,
  `${name}node.js`,
];
ordered.files = [...new Set([...(manifest.files ?? []), ...own(""), ...own("extract/"), "LICENSE"])];
// node.js instantiates the module when imported: never tree-shake it away.
ordered.sideEffects = [...new Set([...(manifest.sideEffects ?? []), "./node.js", "./extract/node.js"])];

// The extraction API: what the core build must not declare in its types, and
// the extract build must. Functions and methods are found by `name(`, types by
// their declaration (prose in doc comments may use the same words).
const extraction = [
  ...["mainContent", "metadata", "links", "resources", "toMarkdownChunks", "tables", "toCsv"].map(
    (name) => new RegExp(`\\b${name}\\(`),
  ),
  ...["MainContent", "Metadata", "Link", "Resource", "Chunk", "Table"].map(
    (name) => new RegExp(`\\b(?:interface|type) ${name}\\b`),
  ),
];

for (const [dir, label] of [[pkgDir, "core"], [extractDir, "extract"]]) {
  const dtsPath = resolve(dir, "domq_wasm.d.ts");
  let dts = readFileSync(dtsPath, "utf8");
  dts = dts
    .replace("constructor(input: any);", "constructor(input: string | Uint8Array);")
    .replace("static parse(input: any): Document;", "static parse(input: string | Uint8Array): Document;")
    .replace("export function parse(input: any): DomDocument;", "export function parse(input: string | Uint8Array): DomDocument;");
  writeFileSync(dtsPath, dts);
  for (const pattern of extraction) {
    const declared = pattern.test(dts);
    if (label === "core" && declared) {
      throw new Error(`the core ${dtsPath} declares ${pattern}: the core build must not export extraction`);
    }
    if (label === "extract" && !declared) {
      throw new Error(`the extract ${dtsPath} lacks ${pattern}: build it with the default features`);
    }
  }
}

writeFileSync(manifestPath, JSON.stringify(ordered, null, 2) + "\n");
console.log(`finished ${manifestPath}`);
