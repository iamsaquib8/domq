# Playground

A static page that runs the `domq-wasm` module in the browser: paste HTML, try
a selector, and see matches, the tree, text, Markdown, JSON and HTML.
Deployed to <https://iamsaquib8.github.io/domq/> on pushes to `main` by
[`pages.yml`](../.github/workflows/pages.yml).

## Run it locally

The page imports the wasm build from `playground/pkg/`, which is not checked in.
It uses the full `extract` build (the crate's default features), the same
module as `domq-wasm/extract` on npm, so reader mode, metadata, chunks and the
rest of the extraction API are available to it. The npm package's core build
(`domq-wasm`) is smaller and leaves extraction out; see
[`wasm/README.md`](../wasm/README.md).

```sh
cd wasm
wasm-pack build --target web --release --no-pack --out-dir ../playground/pkg
cd ..
python3 -m http.server -d playground 8000   # then open http://localhost:8000
```

Files: `index.html` (markup), `app.js` (wiring), `presets.js` (example inputs),
`style.css`, `favicon.svg`.
