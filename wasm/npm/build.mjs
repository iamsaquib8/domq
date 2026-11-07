// Build the npm package `domq-wasm` from one crate, as two wasm modules:
//
//   pkg/            core build (`--no-default-features`):   import … from "domq-wasm"
//   pkg/extract/    full build (the default `extract` feature): import … from "domq-wasm/extract"
//
// then add the Node entry points and the "exports" map (finish.mjs). Needs
// wasm-pack on the PATH. Run it from anywhere:
//
//   node npm/build.mjs
import { execFileSync } from "node:child_process";
import { rmSync } from "node:fs";
import { fileURLToPath } from "node:url";

const crate = fileURLToPath(new URL("..", import.meta.url));
const run = (command, args) => {
  console.log(`$ ${command} ${args.join(" ")}`);
  execFileSync(command, args, { cwd: crate, stdio: "inherit", shell: process.platform === "win32" });
};

// A stale pkg/ from an older build must not leak into the package.
rmSync(`${crate}/pkg`, { recursive: true, force: true });

// 1. The core, with package.json, README and LICENSE.
run("wasm-pack", ["build", "--target", "web", "--release", "--out-dir", "pkg", "--", "--no-default-features"]);
// 2. Everything, one directory down. finish.mjs writes the one package.json.
run("wasm-pack", ["build", "--target", "web", "--release", "--no-pack", "--out-dir", "pkg/extract"]);
// 3. The Node entry points and the "exports" map.
run(process.execPath, ["npm/finish.mjs"]);
