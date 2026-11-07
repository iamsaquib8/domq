# From BeautifulSoup

`pip install domq` gives you a fast, typed alternative to BeautifulSoup for
reading HTML: CSS selectors instead of `find_all`, readable text and Markdown
built in, and dataclass extraction. Parsing and serialization run in Rust
with the GIL released, so a thread pool parses pages in parallel.

## The same code

```python
# BeautifulSoup
from bs4 import BeautifulSoup
soup = BeautifulSoup(html, "html.parser")
title = soup.title.string
links = [a["href"] for a in soup.find_all("a", href=True)]
price = soup.find("span", class_="price").get_text(strip=True)
```

```python
# domq
import domq
doc = domq.parse(html)
title = doc.select_first("title").text
links = [a.attr("href") for a in doc.select("a[href]")]
price = doc.select_first("span.price").text
```

| BeautifulSoup | domq |
|---|---|
| `BeautifulSoup(html, "html.parser")` | `domq.parse(html)` |
| `soup.select(css)` / `soup.select_one(css)` | `doc.select(css)` / `doc.select_first(css)` |
| `soup.find_all("a", class_="x")` | `doc.select("a.x")` |
| `soup.find("div", id="main")` | `doc.select_first("div#main")` |
| `tag["href"]`, `tag.get("href")` | `el.attrs["href"]`, `el.attr("href")` |
| `tag.name` | `el.tag` |
| `tag.get_text(" ", strip=True)` | `el.text` (readable, whitespace collapsed) |
| `tag.text` | `el.text_content` (raw) |
| `str(tag)` / `tag.decode_contents()` | `el.html` / `el.inner_html` |
| `tag.parent`, `tag.children` | `el.parent`, `el.children` |
| `tag.find_parent(css)` | `el.closest(css)` |
| — | `doc.to_markdown()`, `el.to_markdown()` |

## Extract into dataclasses

```python
from dataclasses import dataclass
from typing import Annotated
from domq import Select, extract_all

@dataclass
class Story:
    __select__ = ".story"
    title: Annotated[str, Select("a")]
    url: Annotated[str, Select("a", attr="href")]

stories = extract_all(Story, html)
```

## Differences to know

- domq's documents and elements are **read-only** in Python today (no
  `decompose()` or `tag.string = …`). The Rust crate has an editing API;
  bringing it to Python is planned for 0.10.
- An invalid selector raises `ValueError` with the position of the problem.
- Like `html.parser`, domq does not add `<html>`/`<body>` around fragments;
  unlike it, it repairs unclosed and mis-nested tags the way browsers do.
