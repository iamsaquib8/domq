# Python API reference

Every function, class and member that `import domq` exports (its `__all__`).
`pip install domq` gives wheels for Python 3.11+. Install and the other surfaces
are in [Install](install.md); this page lists what you can call. It is checked
against the package's `__init__.pyi` stub in CI, so a name added to the package
and missing here fails the build.

```python
import domq

doc = domq.parse("<ul id=l><li><a href=/a>One<li><a href=/b>Two</ul>")
[(a.attr("href"), a.text) for a in doc.select("#l a")]  # [('/a', 'One'), ('/b', 'Two')]
doc.to_markdown()                                       # '- [One](/a)\n- [Two](/b)'
```

A bad selector raises `ValueError`. Nothing else raises on bad HTML: `parse`
repairs it and `Document.errors` lists the repairs. Objects are immutable. The
package has no editing and no streaming API: those are in Rust.

## Module functions

| Function | What it does |
|---|---|
| `parse(source, encoding=None)` | Parses a `str` or `bytes` (a full page or a fragment) into a `Document`. For `bytes` the encoding is sniffed like a browser does, or set by `encoding`. Large inputs are parsed with the GIL released. |
| `decode(bytes, encoding=None)` | Decodes bytes to text with WHATWG encoding sniffing; returns `Decoded`. |
| `resolve_url(base, relative)` | Resolves `relative` against `base` (RFC 3986 section 5.2). |
| `resolve_srcset(base, srcset)` | Resolves every URL in a `srcset` value. |
| `extract(cls, source)` | Fills dataclass `cls` from a string, `Document` or `Element`. See [dataclass extraction](#dataclass-extraction). |
| `extract_all(cls, source)` | One `cls` per element matching `cls.__select__`. |
| `__version__` | The package version, a `str`. |

```python
import domq

domq.resolve_url("https://a.example/x/y", "../z")                  # 'https://a.example/z'
domq.resolve_srcset("https://a.example/", "a.png 1x, b.png 2x")    # 'https://a.example/a.png 1x, https://a.example/b.png 2x'
d = domq.decode(b"<p>caf\xe9</p>")
(d.text, d.encoding)                                               # ('<p>café</p>', 'windows-1252')
```

## `Document`

A parsed page, from `domq.parse`. Safe to share between threads.

| Member | What it gives |
|---|---|
| `select(selector)` | Every matching `Element`, in document order. `selector` is a `str` or a `Selector`. |
| `select_first(selector)` | The first match, or `None`. |
| `main_content()` | The `Element` holding the article ("reader mode"), or `None`. |
| `metadata(base_url=None)` | `Metadata`: title, description, canonical URL, language, OpenGraph, Twitter Cards, feeds, JSON-LD. |
| `links(base_url=None)` | Every `<a href>` and `<area href>` as a `Link`. |
| `resources(base_url=None)` | Every embedded URL as a `Resource`. |
| `to_text()` | Readable text: no scripts or styles, whitespace collapsed, a line per block. |
| `to_markdown(*, links=None, images=None, drop_data_uris=False, base_url=None)` | CommonMark and GFM Markdown. `links` is `"inline"`, `"reference"` or `"footer"`; `images` is `"keep"`, `"alt"` or `"drop"`. |
| `to_markdown_chunks(max_chars=None, *, links=None, images=None, drop_data_uris=False, base_url=None)` | Markdown split at headings into `Chunk`s of at most `max_chars` characters. Links are always inline. |
| `tables(selector="table")` | Matching tables as rows of cell text, spans expanded. |
| `to_csv(selector="table")` | Matching tables as RFC 4180 CSV. |
| `outline()` | A `PageOutline` of landmarks, headings, links and controls. |
| `to_html()` | The repaired HTML. |
| `to_json()` | The whole tree as a JSON string. |
| `errors` | Property: what the parser repaired, as `(byte_offset, message)` tuples; empty for clean input. |
| `root` | Property: the first top-level `Element` (usually `<html>`), or `None`. |
| `roots` | Property: all top-level elements. |

```python
import domq

doc = domq.parse("<title>T</title><main><h1>A</h1><p>x</p><h2>B</h2><p>y</p></main>")
doc.select_first("title").text                              # 'T'
[c.context for c in doc.to_markdown_chunks(100)]            # ['A', 'A > B']
doc.tables()                                                # []
```

## `Element`

An element inside a `Document`; it keeps its document alive.

| Member | What it gives |
|---|---|
| `tag` | Property: the lower-case tag name. |
| `attrs` | Property: the attributes as a new `dict` (valueless attributes map to `""`). |
| `attr(name)` | The value of attribute `name`, or `None`. |
| `attr(name, default)` | The value, or `default` when it is absent. |
| `text` | Property: readable text of this element. |
| `to_text()` | Readable text of this element (the same as `text`). |
| `text_content` | Property: all descendant text, verbatim (the DOM's `textContent`). |
| `html` | Property: this element as HTML, with its own tags (`outerHTML`). |
| `inner_html` | Property: the HTML of its children (`innerHTML`). |
| `select(selector)` | Matching descendants, in document order. |
| `select_first(selector)` | The first matching descendant, or `None`. |
| `closest(selector)` | The nearest element matching `selector`, starting with this one and walking up. |
| `matches(selector)` | Whether this element matches `selector`. |
| `table_rows()` | For a `<table>`: rows of cell text, `colspan` and `rowspan` expanded. |
| `to_csv()` | For a `<table>`: its cells as RFC 4180 CSV. |
| `links(base_url=None)` | Hyperlinks inside this element. |
| `resources(base_url=None)` | Embedded resource URLs inside this element. |
| `parent` | Property: the parent element, or `None` for a top-level one. |
| `children` | Property: the child elements (text and comments are skipped). |
| `document` | Property: the `Document` this element belongs to. |
| `to_markdown(*, links=None, images=None, drop_data_uris=False, base_url=None)` | This element as Markdown. |
| `to_markdown_chunks(max_chars=None, *, links=None, images=None, drop_data_uris=False, base_url=None)` | Markdown chunks for this element's subtree. |
| `outline()` | A `PageOutline` for this element's subtree. |
| `to_json()` | This element's subtree as a JSON string. |

Elements support `==` and hashing, so they work in sets and as dictionary
keys.

```python
import domq

doc = domq.parse('<div class=card><h2>Title</h2><a href="/x" class=more>More</a></div>')
link = doc.select_first("a")
link.attr("href"), link.attr("rel", "none"), link.closest(".card").tag   # ('/x', 'none', 'div')
link.matches(".card > a"), link.parent.select_first("h2").text           # (True, 'Title')
```

## `Selector`

A precompiled CSS selector, for running one selector on many documents. A
`str` selector is parsed on every call; `Selector` parses once.

| Member | What it gives |
|---|---|
| `Selector(query)` | Compiles `query`. Raises `ValueError` when it is invalid or unsupported. |
| `query` | Property: the selector text. |
| `matches(element)` | Whether `element` matches. |
| `closest(element)` | The nearest matching element, starting with `element` and walking up. |

```python
import domq

sel = domq.Selector("article h2 > a[href]")
[len(domq.parse(p).select(sel)) for p in ("<article><h2><a href=/1>One</a></h2></article>", "<p>none</p>")]  # [1, 0]
```

## Content: `Metadata`, `Link`, `Resource`, `Chunk`, `Decoded`

Plain read-only result objects.

### `Metadata`

From `Document.metadata(base_url=None)`; every field is `None` (or an empty
list) when the page does not say.

| Member | What it gives |
|---|---|
| `title` | `<title>`, else `og:title`, else `twitter:title`. |
| `description` | `<meta name="description">`, else `og:description`, else `twitter:description`. |
| `canonical` | `<link rel="canonical">`, else `og:url`. |
| `lang` | The `lang` of `<html>`, else the `content-language` meta. |
| `language` | An alias for `lang`. |
| `site_name` | `og:site_name`, else `application-name`. |
| `image` | `og:image`, else `twitter:image`, else `<link rel="image_src">`. |
| `author` | `<meta name="author">`, else `article:author`, else `<a rel="author">`. |
| `published` | The publication date as written (for example ISO 8601). |
| `keywords` | `<meta name="keywords">` split on commas, else `article:tag` values. |
| `og` | OpenGraph `(property, content)` pairs, in document order. |
| `twitter` | Twitter-card `(name, content)` pairs, in document order. |
| `json_ld` | The raw text of each `application/ld+json` block. |
| `feeds` | Feed URLs from `<link rel="alternate">`. |
| `to_dict()` | All of the above as a `dict`. |

### `Link`

From `links()`.

| Member | What it gives |
|---|---|
| `url` | The resolved URL. |
| `text` | The visible text, whitespace collapsed (falling back to `aria-label`, an image's `alt` or `title`). |
| `href` | The `href` as written. |
| `element` | The `<a>` or `<area>` `Element`. |

### `Resource`

From `resources()`.

| Member | What it gives |
|---|---|
| `url` | The resolved URL. |
| `attr` | The attribute it came from: `"href"`, `"src"`, `"srcset"`, `"poster"` or `"data"`. |
| `value` | The attribute value as written. |
| `tag` | The tag name of the element. |
| `element` | The `Element` carrying it. |

### `Chunk`

From `to_markdown_chunks()`. `len(chunk)` and `str(chunk)` work too.

| Member | What it gives |
|---|---|
| `markdown` | The chunk's Markdown. |
| `heading_path` | The titles of the headings it sits under, outermost first. |
| `context` | The heading path joined with `" > "`, to prepend when embedding. |
| `byte_size` | The size of `markdown` in bytes. |

### `Decoded`

From `decode()`.

| Member | What it gives |
|---|---|
| `text` | The decoded text. |
| `encoding` | The encoding detected or chosen. |
| `warning` | A warning from decoding, or `None`. |

```python
import domq

doc = domq.parse('<title>T</title><meta name=author content="Ada"><a href=/a>One</a>')
m = doc.metadata()
(m.title, m.author, m.to_dict()["keywords"])                  # ('T', 'Ada', [])
[(l.url, l.text, l.href) for l in doc.links("https://x.example/")]  # [('https://x.example/a', 'One', '/a')]
```

## Page outline: `PageOutline`, `OutlineNode`

For agents and for finding a stable handle on a part of a page: each node has
a CSS selector that finds the element again. See [Command line](cli.md) for
`--outline`.

### `PageOutline`

From `outline()`. Supports `len()`, indexing and iteration over its nodes.

| Member | What it gives |
|---|---|
| `nodes` | Property: every `OutlineNode`, in document order. |
| `to_text()` | A compact indented text outline, for prompts. |
| `to_json()` | A JSON array string. |

### `OutlineNode`

| Member | What it gives |
|---|---|
| `ref_id` | The 1-based reference in outline order. |
| `role` | The semantic role: `"navigation"`, `"main"`, `"heading"`, `"link"`, `"button"` and so on. |
| `name` | The accessible name, or `None`. |
| `level` | The heading level 1 to 6 for headings, else `None`. |
| `selector` | A minimal CSS selector that finds this element again. |
| `href` | The link target for links, else `None`. |
| `tag` | The tag name. |
| `depth` | The depth in the outline (0 for top-level landmarks). |
| `source_range` | The byte range in the source, `(start, end)`, or `None`. |
| `to_json()` | This node as a JSON string. |

```python
import domq

outline = domq.parse("<nav><a href=/>Home</a></nav><main><h1>Hi</h1></main>").outline()
len(outline), outline[1].role, outline[1].name, outline[1].selector  # (4, 'link', 'Home', 'nav > a')
print(outline.to_text())
# [1] navigation
# [2]   link "Home" -> nav > a
# [3] main
# [4]   heading 1 "Hi"
```

## Dataclass extraction

Declare the fields once with `Annotated[T, Select(...)]`, then call `extract`
(one object, scoped to the whole source) or `extract_all` (one per match of the
class's `__select__`). `T` is `str`, `int`, `float`, `bool`, a nested
dataclass, any class built from a string (`Decimal`, an `Enum`),
`Optional[T]` (`None` when nothing matches) or `list[T]` (every match).

### `Select` and its shortcuts `Attr`, `Html`, `OuterHtml`, `Markdown`

`Select(selector=None, *, attr=None, html=False, outer_html=False, markdown=False, default=..., with_=None)`
says where a field's value comes from. Without a selector the field reads its
scope. The value is the element's readable text unless `attr`, `html`,
`outer_html` or `markdown` says otherwise.

| Member | What it is |
|---|---|
| `selector` | The CSS selector, or `None` for the scope itself. |
| `attr` | The attribute to read instead of text, or `None`. |
| `html` | `True` to read the inner HTML. |
| `outer_html` | `True` to read the outer HTML. |
| `markdown` | `True` to read the Markdown. |
| `default` | The value used when nothing matches (`dataclasses.MISSING` if none was given). |
| `with_` | A function that converts the source string itself; its result is used as is. |
| `Attr(name)` or `Attr(selector, name)` | Shorthand for `Select(selector, attr=name)`. |
| `Html(selector=None)` | Shorthand for `Select(selector, html=True)`. |
| `OuterHtml(selector=None)` | Shorthand for `Select(selector, outer_html=True)`. |
| `Markdown(selector=None)` | Shorthand for `Select(selector, markdown=True)`. |

### `ExtractError`

A `ValueError` raised when a field cannot be extracted: no match, a missing
attribute, a value that does not parse, or an invalid selector.

| Member | What it gives |
|---|---|
| `path` | The field, for example `"Product.reviews[2].rating"`. |
| `selector` | The CSS selector involved, or `None`. |
| `source` | The source string that failed to convert. |
| `reason` | Why it failed. |

```python
from dataclasses import dataclass
from typing import Annotated, Optional

from domq import Attr, ExtractError, Select, extract_all

@dataclass
class Product:
    __select__ = ".product"
    name: Annotated[str, Select("h2")]
    price: Annotated[float, Attr(".price", "data-value")]
    tags: Annotated[list[str], Select(".tag")]
    link: Annotated[Optional[str], Attr("a.more", "href")]

html = """<div class=product><h2>Widget</h2><span class=price data-value=9.5>$9.50</span>
<span class=tag>new</span><span class=tag>blue</span></div>
<div class=product><h2>Gizmo</h2><span class=price data-value=12>$12</span></div>"""
extract_all(Product, html)
# [Product(name='Widget', price=9.5, tags=['new', 'blue'], link=None),
#  Product(name='Gizmo', price=12.0, tags=[], link=None)]

try:
    extract_all(Product, "<div class=product><h2>X</h2></div>")
except ExtractError as e:
    print(e.path, e.selector, e.reason)   # Product[0].price .price no element matches
```

For the shell and MCP equivalents of `Select`, see [Pull fields from a list of
items](cookbook/extracting.md).
