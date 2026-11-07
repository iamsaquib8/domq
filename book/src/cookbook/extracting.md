# Pull fields from a list of items

Describe the fields you want once, and get typed values back, with errors
that name the selector that failed.

## Rust: `#[derive(FromHtml)]`

Enable the `derive` feature (`domq = { version = "0.7", features = ["derive"] }`).

```rust,ignore
use domq::FromHtml;

#[derive(FromHtml)]
#[html(select = ".product")]
struct Product {
    #[html(select = "h2")]
    name: String,
    #[html(select = ".price", attr = "data-value")]
    price: f64,                        // parsed with FromStr
    #[html(select = ".tag")]
    tags: Vec<String>,                 // every match
    #[html(select = "a.more", attr = "href")]
    link: Option<String>,              // None when missing
}

let products: Vec<Product> = domq::parse(html).extract_all()?;
let first: Product = domq::parse(html).extract()?;
```

- `#[html(select = …)]` on the struct is the scope: `extract_all` returns one
  value per match, `extract` the first.
- A field reads the readable text of its first match by default; `attr = …`
  reads an attribute, `html`, `outer_html` and `markdown` read those forms.
- `Option<T>` is `None` when nothing matches, `Vec<T>` collects every match,
  and a field of another `FromHtml` type nests.
- A missing required field or a value that does not parse is an
  `ExtractError` with the field, the selector and the reason.

## Shell: `-e NAME=SPEC`

```sh
domq '.product' -e name=h2 -e price=.price@data-value -e 'tags=.tag[]' --jsonl < page.html
# {"name":"Widget","price":"9.5","tags":["new","blue"]}
# {"name":"Gizmo","price":"12","tags":[]}
```

`SPEC` is a selector (the text of the first match), `selector@attr` (an
attribute of it), or either with `[]` for every match. An empty selector is
the element itself (`-e id=@id`). Without a scope selector you get one object
for the page.

## Python: dataclasses

```python
from dataclasses import dataclass
from typing import Annotated, Optional
from domq import Select, extract_all

@dataclass
class Product:
    __select__ = ".product"
    name: Annotated[str, Select("h2")]
    price: Annotated[float, Select(".price", attr="data-value")]
    tags: Annotated[list[str], Select(".tag")]
    link: Annotated[Optional[str], Select("a.more", attr="href")]

products = extract_all(Product, html)
```

## AI agents

The MCP server's `extract` tool takes the same specs as `-e`:
`{"fields": {"name": "h2", "price": ".price@data-value"}, "selector": ".product"}`.
See [AI agents (MCP)](../mcp.md).
