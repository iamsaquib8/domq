"""Parse messy real-world HTML, query it with CSS selectors, and get readable
text, Markdown, HTML or JSON back. Powered by the domq Rust crate.

>>> import domq
>>> doc = domq.parse("<ul><li class='x'>a</li><li>b</li></ul>")
>>> [li.text for li in doc.select("li")]
['a', 'b']
>>> doc.select_first("li.x").html
'<li class="x">a</li>'

Declarative extraction into dataclasses (:func:`extract`, :func:`extract_all`):

>>> from dataclasses import dataclass
>>> from typing import Annotated
>>> @dataclass
... class Item:
...     name: Annotated[str, domq.Select("li.x")]
>>> domq.extract(Item, "<ul><li class='x'>a</li><li>b</li></ul>")
Item(name='a')
"""

from ._domq import (
    Chunk,
    Document,
    Element,
    Link,
    Metadata,
    OutlineNode,
    PageOutline,
    Resource,
    Selector,
    __version__,
    parse,
    resolve_srcset,
    resolve_url,
)
from ._extract import Attr, ExtractError, Html, Markdown, OuterHtml, Select, extract, extract_all

__all__ = [
    "Attr",
    "Chunk",
    "Document",
    "Element",
    "ExtractError",
    "Html",
    "Link",
    "Markdown",
    "Metadata",
    "OuterHtml",
    "OutlineNode",
    "PageOutline",
    "Resource",
    "Select",
    "Selector",
    "__version__",
    "extract",
    "extract_all",
    "parse",
    "resolve_srcset",
    "resolve_url",
]
