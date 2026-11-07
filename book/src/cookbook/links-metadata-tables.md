# Links, metadata and tables

## Links

```rust
# extern crate domq;
let dom = domq::parse(r##"<head><base href="/docs/"></head>
  <a href="intro.html">Intro</a> <a href="#top"><img alt="Back to top"></a>"##);

for link in dom.links(Some("https://example.com/index.html")) {
    println!("{} {}", link.url, link.text);
}
// https://example.com/docs/intro.html Intro
// https://example.com/docs/#top Back to top
# let links = dom.links(Some("https://example.com/index.html"));
# assert_eq!(links[0].url, "https://example.com/docs/intro.html");
# assert_eq!((links[1].url.as_str(), links[1].text.as_str()), ("https://example.com/docs/#top", "Back to top"));
```

URLs are resolved against the base you pass and the page's `<base href>`.
`text` is the visible link text on one line, falling back to `aria-label`, an
image's `alt` or `title` for icon links. `href` keeps the attribute as
written. `Dom::resources` lists what a page loads (images, scripts, styles,
`srcset` candidates) the same way.

```sh
curl -sL https://www.rust-lang.org | domq -b https://www.rust-lang.org --links nav
```

## Metadata

```rust
# extern crate domq;
let dom = domq::parse(r#"<html lang="en"><head>
  <title>Arena trees</title>
  <meta name="description" content="How arenas work">
  <meta property="og:site_name" content="The Blog">
  <link rel="canonical" href="/arena-trees">
</head></html>"#);

let m = dom.metadata();
assert_eq!(m.title.as_deref(), Some("Arena trees"));
assert_eq!(m.description.as_deref(), Some("How arenas work"));
assert_eq!(m.site_name.as_deref(), Some("The Blog"));
assert_eq!(m.lang.as_deref(), Some("en"));
```

`Metadata` also has `canonical`, `image`, `author`, `published`, `keywords`,
the OpenGraph and Twitter tags as `(property, value)` pairs, JSON-LD
blocks (unparsed, minus any CDATA or comment wrapper) and feed URLs. `domq --meta` prints it as JSON.

## Tables

```rust
# extern crate domq;
let dom = domq::parse("<table>
  <tr><th>Region<th>Q1<th>Q2
  <tr><td rowspan=2>EU<td>10<td>12
  <tr><td colspan=2>n/a
</table>");

let table = dom.select_first("table").unwrap();
let rows = table.table_rows();
assert_eq!(rows[2], ["EU", "n/a", "n/a"]); // spans expanded
print!("{}", domq::to_csv(&rows));
```

```sh
domq --csv -1 '#results' < report.html > results.csv
```
