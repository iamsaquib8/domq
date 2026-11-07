# Crawl results to a table

**Goal.** Turn pages you have fetched into rows: every link with its text and
whether it leaves the site, the resources a page loads, and its data tables as
CSV, ready for a spreadsheet, a database or a link checker.

**Input.** [shop.html](../sample/shop.html) and [post.html](../sample/post.html).
domq reads HTML you give it; it does not fetch pages or follow links, so your
crawler supplies each page's HTML and its URL.

## Links

`links` resolves every `href` against the page's URL (and its `<base href>`,
if it has one) and gives each link's visible text, falling back to
`aria-label`, an image's `alt` or `title` for icon links.

<div class="domq-tabs">

**CLI**

```sh
domq --links -b https://shop.example/gadgets < shop.html
```

**Python**

```python
import domq

doc = domq.parse(open("shop.html", encoding="utf-8").read())
for link in doc.links(base_url="https://shop.example/gadgets"):
    print(link.url, link.text, sep="\t")
```

**JavaScript**

```js
import { readFileSync } from "node:fs";
import { links } from "domq-wasm/extract";

const html = readFileSync("shop.html", "utf8");
for (const link of links(html, "https://shop.example/gadgets")) {
  console.log(`${link.url}\t${link.text}`);
}
```

**Rust**

```rust
# extern crate domq;
# let html = r##"<!doctype html>
# <html lang="en">
# <head>
#   <meta charset="utf-8">
#   <title>Gadgets | Example Shop</title>
#   <meta name="description" content="Widgets, gizmos and gadgets, shipped in a week.">
#   <link rel="canonical" href="https://shop.example/gadgets">
#   <script>window.dataLayer = []; /* tracking */</script>
# </head>
# <body>
#   <nav>
#     <a href="/">Home</a>
#     <a href="/gadgets">Gadgets</a>
#     <a href="/cart"><img src="/img/cart.svg"></a>
#   </nav>
#   <main>
#     <h1>Gadgets</h1>
#     <ul class="products">
#       <li class="product" data-sku="W-1">
#         <a href="/gadgets/widget"><img src="/img/widget.jpg" alt="Widget"></a>
#         <h2>Widget</h2>
#         <span class="price" data-value="9.5">$9.50</span>
#         <span class="tag">new</span> <span class="tag">blue</span>
#       <li class="product out-of-stock" data-sku="G-2">
#         <a href="/gadgets/gizmo"><img src="/img/gizmo.jpg" alt="Gizmo"></a>
#         <h2>Gizmo</h2>
#         <span class="price" data-value="12">$12</span>
#       <li class="product" data-sku="D-3">
#         <a href="/gadgets/doohickey"><img src="/img/doohickey.jpg"></a>
#         <h2>Doohickey</h2>
#         <span class="price" data-value="24.99">$24.99</span>
#         <span class="tag">sale</span>
#     </ul>
#     <h2>Specifications</h2>
#     <table id="specs">
#       <tr><th>Product<th>Weight (g)<th>Warranty
#       <tr><td>Widget<td>120<td rowspan="2">2 years
#       <tr><td>Gizmo<td>85
#       <tr><td>Doohickey<td>300<td>1 year
#     </table>
#   </main>
#   <footer>
#     <a href="/terms">Terms</a> <a href="https://twitter.com/shop_example">Twitter</a>
#   </footer>
# </body>
# </html>"##;
let dom = domq::parse(html);
for link in dom.links(Some("https://shop.example/gadgets")) {
    println!("{}\t{}", link.url, link.text);
}
```

</div>

All four print the same eight lines (a tab between the URL and the text):

```text
https://shop.example/	Home
https://shop.example/gadgets	Gadgets
https://shop.example/cart	
https://shop.example/gadgets/widget	Widget
https://shop.example/gadgets/gizmo	Gizmo
https://shop.example/gadgets/doohickey	
https://shop.example/terms	Terms
https://twitter.com/shop_example	Twitter
```

The cart and Doohickey links have no text: their images have no `alt`.
`--links` takes a selector to keep only the links inside it
(`domq --links -b URL main`), and `--json` or `--jsonl` prints
`{"url", "text"}` objects.

## Resources

`resources` lists what the page loads: images (with every `srcset`
candidate), scripts, stylesheets and other `src` / `href` targets, resolved
the same way. There is no CLI flag for it.

<div class="domq-tabs">

**Python**

```python
import domq

doc = domq.parse(open("shop.html", encoding="utf-8").read())
for r in doc.resources(base_url="https://shop.example/gadgets"):
    print(r.tag, r.attr, r.url)
```

**Rust**

```rust
# extern crate domq;
# let html = r##"<!doctype html>
# <html lang="en">
# <head>
#   <meta charset="utf-8">
#   <title>Gadgets | Example Shop</title>
#   <meta name="description" content="Widgets, gizmos and gadgets, shipped in a week.">
#   <link rel="canonical" href="https://shop.example/gadgets">
#   <script>window.dataLayer = []; /* tracking */</script>
# </head>
# <body>
#   <nav>
#     <a href="/">Home</a>
#     <a href="/gadgets">Gadgets</a>
#     <a href="/cart"><img src="/img/cart.svg"></a>
#   </nav>
#   <main>
#     <h1>Gadgets</h1>
#     <ul class="products">
#       <li class="product" data-sku="W-1">
#         <a href="/gadgets/widget"><img src="/img/widget.jpg" alt="Widget"></a>
#         <h2>Widget</h2>
#         <span class="price" data-value="9.5">$9.50</span>
#         <span class="tag">new</span> <span class="tag">blue</span>
#       <li class="product out-of-stock" data-sku="G-2">
#         <a href="/gadgets/gizmo"><img src="/img/gizmo.jpg" alt="Gizmo"></a>
#         <h2>Gizmo</h2>
#         <span class="price" data-value="12">$12</span>
#       <li class="product" data-sku="D-3">
#         <a href="/gadgets/doohickey"><img src="/img/doohickey.jpg"></a>
#         <h2>Doohickey</h2>
#         <span class="price" data-value="24.99">$24.99</span>
#         <span class="tag">sale</span>
#     </ul>
#     <h2>Specifications</h2>
#     <table id="specs">
#       <tr><th>Product<th>Weight (g)<th>Warranty
#       <tr><td>Widget<td>120<td rowspan="2">2 years
#       <tr><td>Gizmo<td>85
#       <tr><td>Doohickey<td>300<td>1 year
#     </table>
#   </main>
#   <footer>
#     <a href="/terms">Terms</a> <a href="https://twitter.com/shop_example">Twitter</a>
#   </footer>
# </body>
# </html>"##;
for r in domq::parse(html).resources(Some("https://shop.example/gadgets")) {
    println!("{} {} {}", r.node.tag_name().unwrap_or(""), r.attr, r.url);
}
```

</div>

```text
link href https://shop.example/gadgets
img src https://shop.example/img/cart.svg
img src https://shop.example/img/widget.jpg
img src https://shop.example/img/gizmo.jpg
img src https://shop.example/img/doohickey.jpg
```

The first line is the page's `<link rel="canonical">`. Filter on `tag` and
`attr` for what you want (`img`/`src` for images). Inline scripts and
styles have no URL and are not listed.

## Tables to CSV

A table's `rowspan` and `colspan` are expanded, so every row has every column:
in `shop.html` the Widget's "2 years" warranty spans two rows and appears in
both.

<div class="domq-tabs">

**CLI**

```sh
domq --csv -1 '#specs' < shop.html > specs.csv
```

**Python**

```python
import domq

doc = domq.parse(open("shop.html", encoding="utf-8").read())
print(doc.to_csv("#specs"), end="")
```

**JavaScript**

```js
import { readFileSync } from "node:fs";
import { toCsv } from "domq-wasm/extract";

process.stdout.write(toCsv(readFileSync("shop.html", "utf8"), "#specs"));
```

**Rust**

```rust
# extern crate domq;
# let html = r##"<!doctype html>
# <html lang="en">
# <head>
#   <meta charset="utf-8">
#   <title>Gadgets | Example Shop</title>
#   <meta name="description" content="Widgets, gizmos and gadgets, shipped in a week.">
#   <link rel="canonical" href="https://shop.example/gadgets">
#   <script>window.dataLayer = []; /* tracking */</script>
# </head>
# <body>
#   <nav>
#     <a href="/">Home</a>
#     <a href="/gadgets">Gadgets</a>
#     <a href="/cart"><img src="/img/cart.svg"></a>
#   </nav>
#   <main>
#     <h1>Gadgets</h1>
#     <ul class="products">
#       <li class="product" data-sku="W-1">
#         <a href="/gadgets/widget"><img src="/img/widget.jpg" alt="Widget"></a>
#         <h2>Widget</h2>
#         <span class="price" data-value="9.5">$9.50</span>
#         <span class="tag">new</span> <span class="tag">blue</span>
#       <li class="product out-of-stock" data-sku="G-2">
#         <a href="/gadgets/gizmo"><img src="/img/gizmo.jpg" alt="Gizmo"></a>
#         <h2>Gizmo</h2>
#         <span class="price" data-value="12">$12</span>
#       <li class="product" data-sku="D-3">
#         <a href="/gadgets/doohickey"><img src="/img/doohickey.jpg"></a>
#         <h2>Doohickey</h2>
#         <span class="price" data-value="24.99">$24.99</span>
#         <span class="tag">sale</span>
#     </ul>
#     <h2>Specifications</h2>
#     <table id="specs">
#       <tr><th>Product<th>Weight (g)<th>Warranty
#       <tr><td>Widget<td>120<td rowspan="2">2 years
#       <tr><td>Gizmo<td>85
#       <tr><td>Doohickey<td>300<td>1 year
#     </table>
#   </main>
#   <footer>
#     <a href="/terms">Terms</a> <a href="https://twitter.com/shop_example">Twitter</a>
#   </footer>
# </body>
# </html>"##;
let dom = domq::parse(html);
let table = dom.select_first("#specs").unwrap();
print!("{}", domq::to_csv(&table.table_rows()));
```

</div>

```text
Product,Weight (g),Warranty
Widget,120,2 years
Gizmo,85,2 years
Doohickey,300,1 year
```

The file is RFC 4180: cells with commas, quotes or line breaks are quoted, and
the line ending is `\r\n` (the CLI prints it too). A `<br>` inside a cell
becomes a line break in the cell. With no selector, `--csv` prints every table
on the page, separated by a blank line; `tables()` / `table_rows()` give the
rows as lists instead, for code that wants cells rather than a file.

## A whole crawl, as one CSV

Loop over the pages your crawler fetched, and write one row per link:

```python
import csv
import sys
from urllib.parse import urlsplit

import domq

pages = {
    "https://shop.example/gadgets": "shop.html",
    "https://blog.example/arena-trees": "post.html",
}

out = csv.writer(sys.stdout, lineterminator="\n")
out.writerow(["page", "url", "text", "internal"])
for page_url, path in pages.items():
    doc = domq.parse(open(path, encoding="utf-8").read())
    host = urlsplit(page_url).netloc
    for link in doc.links(base_url=page_url):
        out.writerow([page_url, link.url, link.text, urlsplit(link.url).netloc == host])
```

```text
page,url,text,internal
https://shop.example/gadgets,https://shop.example/,Home,True
https://shop.example/gadgets,https://shop.example/gadgets,Gadgets,True
https://shop.example/gadgets,https://shop.example/cart,,True
https://shop.example/gadgets,https://shop.example/gadgets/widget,Widget,True
https://shop.example/gadgets,https://shop.example/gadgets/gizmo,Gizmo,True
https://shop.example/gadgets,https://shop.example/gadgets/doohickey,,True
https://shop.example/gadgets,https://shop.example/terms,Terms,True
https://shop.example/gadgets,https://twitter.com/shop_example,Twitter,False
https://blog.example/arena-trees,https://blog.example/,The Example Blog,True
https://blog.example/arena-trees,https://blog.example/archive,Archive,True
https://blog.example/arena-trees,https://blog.example/about,About,True
https://blog.example/arena-trees,https://twitter.com/intent/tweet,Share,False
https://blog.example/arena-trees,https://blog.example/posts/indexes,indexes,True
```

To keep only the pages' own content links, scope the query first, for example
`doc.select_first("main")` and then `.links(base_url=...)` on that element.

## What can go wrong

- **Not every link is a web page.** `mailto:` and `tel:` links are listed as
  they are, `#fragment` links resolve to the page itself plus the fragment,
  and `javascript:` links are left out. Filter on the scheme before you
  queue URLs.
- **Resolve against the right URL.** Pass the URL the page was fetched
  from after redirects, not the one you asked for. `<base href>` is applied
  for you.
- **Same site is your rule.** The `internal` column above compares host names;
  decide yourself about `www.` prefixes and subdomains.
- **Links that JavaScript adds are not there.** domq sees the HTML as
  delivered.
- **Layout tables.** `--csv` and `tables()` take every `<table>`, including
  ones used for layout. Pass a selector.

## Reference

[Scrape links, metadata and tables](links-metadata-tables.md),
[Command line](../cli.md), [Selectors](../selectors.md).
