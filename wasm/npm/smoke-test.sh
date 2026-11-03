#!/usr/bin/env sh
# Pack pkg/ exactly as `npm publish` would, install the tarball into a scratch
# project, and run tests/node.mjs against it by package name.
#
#   wasm-pack build --target web --release && node npm/finish.mjs && sh npm/smoke-test.sh
set -eu
crate=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

(cd "$crate/pkg" && npm pack --silent --pack-destination "$work" >/dev/null)
# The license must ship in the tarball.
tar -tzf "$work"/domq-wasm-*.tgz | grep -qx 'package/LICENSE' || {
  echo "smoke-test: LICENSE missing from the npm package" >&2
  exit 1
}
cd "$work"
npm init -y >/dev/null
npm install --silent --no-audit --no-fund ./domq-wasm-*.tgz
cp "$crate/tests/node.mjs" .
node node.mjs
