// Turn wasm-pack's `--target web` output into the published npm package:
// add the Node.js entry point and an "exports" map that picks it for Node/Bun
// and the plain ES module (call `await init()`) everywhere else.
//
//   wasm-pack build --target web --release && node npm/finish.mjs [pkg-dir]
import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = new URL(".", import.meta.url);
const pkgDir = resolve(process.argv[2] ?? fileURLToPath(new URL("../pkg", here)));
const manifestPath = resolve(pkgDir, "package.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));

copyFileSync(new URL("node.js", here), resolve(pkgDir, "node.js"));
// npm always packs a file named LICENSE; make sure it is there whatever
// wasm-pack copied.
copyFileSync(new URL("../LICENSE", here), resolve(pkgDir, "LICENSE"));

const types = "./domq_wasm.d.ts";
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
      ".": { types, node: "./node.js", default: "./domq_wasm.js" },
      "./domq_wasm.js": { types, default: "./domq_wasm.js" },
      "./domq_wasm_bg.wasm": "./domq_wasm_bg.wasm",
      "./package.json": "./package.json",
    };
  }
}
ordered.files = [...new Set([...(manifest.files ?? []), "node.js", "LICENSE"])];
// node.js instantiates the module when imported: never tree-shake it away.
ordered.sideEffects = [...new Set([...(manifest.sideEffects ?? []), "./node.js"])];

const dtsPath = resolve(pkgDir, "domq_wasm.d.ts");
let dts = readFileSync(dtsPath, "utf8");
dts = dts
  .replace("constructor(input: any);", "constructor(input: string | Uint8Array);")
  .replace("static parse(input: any): Document;", "static parse(input: string | Uint8Array): Document;")
  .replace("export function parse(input: any): DomDocument;", "export function parse(input: string | Uint8Array): DomDocument;");
writeFileSync(dtsPath, dts);

writeFileSync(manifestPath, JSON.stringify(ordered, null, 2) + "\n");
console.log(`finished ${manifestPath}`);
