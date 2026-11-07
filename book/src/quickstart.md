# Quick start

The same task in every surface: from a product page, get the title, each
product's name and price, and the page as Markdown. Pick a language below; the
choice is remembered on the other pages of this guide.

```html
<html><head><title>Gadgets</title></head><body>
  <nav><a href="/">Home</a></nav>
  <main>
    <h1>Gadgets</h1>
    <div class="product"><h2>Widget</h2><span class="price" data-value="9.5">$9.50</span></div>
    <div class="product"><h2>Gizmo</h2><span class="price" data-value="12">$12</span></div>
  </main>
</body></html>
```

<div class="domq-tabs">

**Rust**

```rust
# extern crate domq;
let html = r#"<title>Gadgets</title><main><h1>Gadgets</h1>
  <div class="product"><h2>Widget</h2><span class="price" data-value="9.5">$9.50</span></div>
  <div class="product"><h2>Gizmo</h2><span class="price" data-value="12">$12</span></div></main>"#;

let dom = domq::parse(html);

let title = dom.select_first("title").map(|t| t.to_text());
assert_eq!(title.as_deref(), Some("Gadgets"));

for product in dom.select(".product") {
    let name = product.select_first("h2").map(|h| h.to_text()).unwrap_or_default();
    let price = product.select_first(".price").and_then(|p| p.attr("data-value"));
    println!("{name}: {}", price.unwrap_or("?"));
}
// Widget: 9.5
// Gizmo: 12

println!("{}", dom.to_markdown());
```

`domq::parse` never fails: any input gives a tree, and what it repaired is
in `dom.errors()`. Selectors are parsed on every call when passed as `&str`;
to reuse one, parse it once with `domq::Selector::parse` and pass `&selector`.

**CLI**

```sh
domq -t title < page.html
# Gadgets
domq '.product' -e name=h2 -e price=.price@data-value < page.html
# [{"name":"Widget","price":"9.5"},{"name":"Gizmo","price":"12"}]
domq --main --markdown < page.html
# # Gadgets
#
# ## Widget
#
# $9.50
#
# ## Gizmo
#
# $12
```

**JavaScript**

```js
import { select, selectFirst, toMarkdown } from "domq-wasm";

selectFirst(html, "title")?.text;                                   // "Gadgets"
select(html, ".product .price").map((m) => m.attrs["data-value"]);  // ["9.5", "12"]
toMarkdown(html, "main");                                           // "# Gadgets\n\n## Widget\n\n$9.50\n\n## Gizmo\n\n$12"
```

Each function parses the HTML again. To run several queries, parse once with
`new Document(html)` and call the same methods on it.

**Python**

```python
import domq

doc = domq.parse(html)
doc.select_first("title").text                      # 'Gadgets'
[(p.select_first("h2").text, p.select_first(".price").attr("data-value"))
 for p in doc.select(".product")]                   # [('Widget', '9.5'), ('Gizmo', '12')]
doc.select_first("main").to_markdown()              # '# Gadgets\n\n## Widget\n\n$9.50\n\n## Gizmo\n\n$12'
```

</div>

## Next

- Not sure which surface fits? [Choose your surface](surfaces.md).
- Not sure which function to call? [Which API do I use?](which-api.md)
- Pull those fields straight into structs or dataclasses: [Pull fields from a
  list of items](cookbook/extracting.md).
- Feed pages to a model: [Turn a page into Markdown for an LLM](cookbook/reader-mode.md).
