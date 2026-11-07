# Scrape elements and attributes

## Select, read, repeat

```rust
# extern crate domq;
let dom = domq::parse(r#"
  <ul id="links">
    <li><a href="https://example.com/a" class="ext">Alpha</a></li>
    <li><a href="/b">Beta</a></li>
    <li class="in-stock"><span class="price">$9</span></li>
  </ul>"#);

// Every match, in document order.
let hrefs: Vec<&str> = dom.select("#links a[href]").filter_map(|a| a.attr("href")).collect();
assert_eq!(hrefs, ["https://example.com/a", "/b"]);

// The first match.
let first = dom.select_first("a.ext").unwrap();
assert_eq!(first.to_text(), "Alpha");

// `:has()` and jQuery's `:contains()` work too.
let price = dom.select_first("li:has(.price) .price").map(|p| p.to_text());
assert_eq!(price.as_deref(), Some("$9"));
assert_eq!(dom.select("a:contains('Beta')").count(), 1);
```

`to_text()` is what a reader sees: whitespace collapsed, block elements on
their own lines, scripts, styles and hidden elements left out.
`text_content()` is the DOM's raw `textContent`.

## Search inside a match

Every `NodeRef` has `select`, `select_first`, `closest` and `matches`:

```rust
# extern crate domq;
let dom = domq::parse(r#"<div class="card"><h2>Title</h2><a href="/x">More</a></div>"#);
let link = dom.select_first("a").unwrap();
let card = link.closest(".card").unwrap();
assert_eq!(card.select_first("h2").unwrap().to_text(), "Title");
assert!(link.matches(".card > a"));
```

## Reuse a selector

A `&str` selector is parsed on every call, and an invalid one simply matches
nothing. Parse it once with `Selector::parse` when you run it on many
documents, or when it comes from outside your code: that returns an error
with the position of the problem.

```rust
# extern crate domq;
use domq::Selector;

let sel = Selector::parse("article h2 > a[href]").unwrap();
for page in ["<article><h2><a href=/1>One</a></h2></article>", "<p>none</p>"] {
    let dom = domq::parse(page);
    println!("{}", dom.select(&sel).count());
}

let err = Selector::parse("a:hover").unwrap_err();
assert_eq!(err.to_string(), "unsupported pseudo-class ':hover' at byte 1");
```

## From the shell

```sh
curl -s https://news.ycombinator.com | domq -t '.titleline > a'     # text
curl -sL https://www.rust-lang.org | domq -a href 'nav a'           # attribute
domq -c 'img:not([alt])' < page.html                                # count
domq -1 -p 'article' < page.html                                    # first, pretty
```

## In JavaScript and Python

```js
import { select } from "domq-wasm";
select(html, "#links a[href]").map((a) => a.attrs.href);
```

```python
import domq
[a.attr("href") for a in domq.parse(html).select("#links a[href]")]
```
