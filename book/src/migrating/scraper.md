# From scraper

[`scraper`](https://crates.io/crates/scraper) builds on html5ever, a fully
spec-compliant parser. domq trades that compliance for zero dependencies, a
0.6 s build (scraper: 42 crates, 5.3 s), `no_std`, a 57 KB wasm module (78 KB with
extraction), and
content output (text, Markdown, chunks, metadata). If you need a browser's
exact tree, keep scraper; if you scrape content, domq is usually a drop-in.

## The same code

```rust,ignore
// scraper
use scraper::{Html, Selector};
let document = Html::parse_document(html);
let sel = Selector::parse("a.story").unwrap();
for a in document.select(&sel) {
    let href = a.value().attr("href");
    let text: String = a.text().collect();
}
```

```rust
# extern crate domq;
// domq
let html = r#"<a class="story" href="/1">One</a>"#;
let dom = domq::parse(html);
for a in dom.select("a.story") {            // or a parsed `domq::Selector`
    let href = a.attr("href");
    let text = a.text_content();            // or `to_text()` for readable text
    # assert_eq!((href, text.as_str()), (Some("/1"), "One"));
}
```

| scraper | domq |
|---|---|
| `Html::parse_document(s)` / `Html::parse_fragment(s)` | `domq::parse(s)` (one entry point for both) |
| `Selector::parse(s)` | `Selector::parse(s)`, or pass `&str` directly |
| `document.select(&sel)` | `dom.select(&sel)` / `dom.select("…")` |
| `element.select(&sel)` | `node.select(…)` |
| `element.value().attr("x")` | `node.attr("x")` |
| `element.value().name()` | `node.tag_name()` |
| `element.value().classes()` | `node.classes()` |
| `element.text().collect::<String>()` | `node.text_content()` |
| `element.html()` / `element.inner_html()` | `node.outer_html()` / `node.inner_html()` |
| `element.parent()`, `.children()`, `.next_sibling()` | same names on `NodeRef` |
| `ElementRef::wrap(node)` | not needed: every `NodeRef` has the element methods |
| `document.errors` | `dom.errors()`, with byte positions |

## Differences to know

- **No implied `<html>`, `<head>`, `<body>` or `<tbody>`.** html5ever adds
  them, domq keeps what the source has. A selector like `html > body > p` or
  `table > tbody > tr` needs those elements in the source; `body > p` and
  `table tr` work either way.
- **Recovery is browser-like for the common cases**, not the full algorithm:
  no adoption agency or foster parenting. The
  [conformance report](../measured.md) shows where trees differ.
- **Readable text.** `to_text()` is what a reader sees; `text_content()` is
  scraper's concatenated text.
- **More out of the box:** `to_markdown()`, `to_markdown_chunks()`,
  `main_content()`, `links()`, `metadata()`, `table_rows()`, an editing API,
  source spans, streaming events and `#[derive(FromHtml)]`.
