#!/usr/bin/env sh
# Pack pkg/ exactly as `npm publish` would, install the tarball into a scratch
# project, and run tests/node.mjs against it by package name (both entries:
# `domq-wasm` and `domq-wasm/extract`).
#
#   node npm/build.mjs && sh npm/smoke-test.sh
set -eu
crate=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

(cd "$crate/pkg" && npm pack --silent --pack-destination "$work" >/dev/null)
# The license and both entries (module, Node shim, types, .wasm) must ship in
# the tarball.
tar -tzf "$work"/domq-wasm-*.tgz > "$work/files.txt"
for f in LICENSE package.json README.md \
  domq_wasm.js domq_wasm.d.ts domq_wasm_bg.wasm node.js \
  extract/domq_wasm.js extract/domq_wasm.d.ts extract/domq_wasm_bg.wasm extract/node.js; do
  grep -qx "package/$f" "$work/files.txt" || {
    echo "smoke-test: $f missing from the npm package" >&2
    exit 1
  }
done
cd "$work"
npm init -y >/dev/null
npm install --silent --no-audit --no-fund ./domq-wasm-*.tgz
cp "$crate/tests/node.mjs" .
node node.mjs
