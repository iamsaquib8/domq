"""Type stubs for ``domq`` (the ``domq`` package on PyPI).

Parse messy real-world HTML, query it with CSS selectors, and get readable
text, Markdown, HTML or JSON back. Powered by the domq Rust crate.
"""

from typing import Any, Callable, Iterator, Literal, Optional, TypeVar, Union, final, overload

from _typeshed import DataclassInstance

__all__ = [
    "Attr",
    "Chunk",
    "Decoded",
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
    "decode",
    "extract",
    "extract_all",
    "parse",
    "resolve_srcset",
    "resolve_url",
]

__version__: str

_T = TypeVar("_T")
_D = TypeVar("_D", bound=DataclassInstance)

def parse(source: str | bytes, encoding: Optional[str] = None) -> Document:
    """Parse an HTML document (or fragment) into a :class:`Document`.

    Accepts a ``str`` or raw ``bytes``. When ``bytes`` are passed, character
    encoding is detected automatically via the WHATWG encoding-sniffing
    algorithm (or overridden with ``encoding``).

    Never raises on malformed HTML: anything the parser had to recover from is
    listed in :attr:`Document.errors`. Large inputs are parsed with the GIL
    released.
    """

def decode(bytes: bytes, encoding: Optional[str] = None) -> Decoded:
    """Decode raw HTML bytes into a string, using WHATWG encoding sniffing."""

@final
class Decoded:
    """The result of :func:`domq.decode`."""

    @property
    def text(self) -> str:
        """The decoded text."""
    @property
    def encoding(self) -> str:
        """The encoding detected or chosen."""
    @property
    def warning(self) -> Optional[str]:
        """Any warning produced during decoding."""
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

def resolve_url(base: str, relative: str) -> str:
    """Resolve ``relative`` against ``base``, following RFC 3986 §5.2."""

def resolve_srcset(base: str, srcset: str) -> str:
    """Resolve URLs in a ``srcset`` attribute string against ``base``."""

@final
class Selector:
    """A precompiled CSS selector for fast repeated queries."""

    def __new__(cls, query: str) -> Selector: ...
    @property
    def query(self) -> str:
        """The CSS selector query string."""
    def matches(self, element: Element) -> bool:
        """Whether ``element`` matches this selector."""
    def closest(self, element: Element) -> Optional[Element]:
        """The nearest element matching this selector, starting with ``element``
        itself and walking up its ancestors."""
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

@final
class Metadata:
    """Page metadata extracted from ``<title>``, ``<meta>``, ``<link>``, and JSON-LD blocks."""

    @property
    def title(self) -> Optional[str]:
        """``<title>``, else ``og:title``, else ``twitter:title``."""
    @property
    def description(self) -> Optional[str]:
        """``<meta name="description">``, else ``og:description``, else ``twitter:description``."""
    @property
    def canonical(self) -> Optional[str]:
        """``<link rel="canonical" href="...">``, else ``og:url``."""
    @property
    def lang(self) -> Optional[str]:
        """The ``lang`` attribute of ``<html>``, else ``<meta http-equiv="content-language">``."""
    @property
    def language(self) -> Optional[str]:
        """Alias for :attr:`lang`."""
    @property
    def site_name(self) -> Optional[str]:
        """``og:site_name``, else ``<meta name="application-name">``."""
    @property
    def image(self) -> Optional[str]:
        """``og:image``, else ``twitter:image``, else ``<link rel="image_src">``."""
    @property
    def author(self) -> Optional[str]:
        """``<meta name="author">``, else ``article:author``, else ``<a rel="author">``."""
    @property
    def published(self) -> Optional[str]:
        """Publication date as written (e.g. ISO 8601)."""
    @property
    def keywords(self) -> list[str]:
        """``<meta name="keywords">`` split on commas, else ``article:tag`` values."""
    @property
    def og(self) -> list[tuple[str, str]]:
        """OpenGraph ``<meta property=... content=...>`` pairs in document order."""
    @property
    def twitter(self) -> list[tuple[str, str]]:
        """Twitter-card ``<meta name="twitter:..." content=...>`` pairs in document order."""
    @property
    def json_ld(self) -> list[str]:
        """Raw JSON-LD strings from ``<script type="application/ld+json">`` blocks."""
    @property
    def feeds(self) -> list[str]:
        """Feed URLs from ``<link rel="alternate">`` (RSS, Atom, JSON Feed)."""
    def to_dict(self) -> dict[str, Any]:
        """Convert metadata fields into a dictionary."""
    def __repr__(self) -> str: ...

@final
class Link:
    """A hyperlink (``<a href>`` or ``<area href>``) with its resolved URL and readable text."""

    @property
    def url(self) -> str:
        """The resolved URL."""
    @property
    def text(self) -> str:
        """The readable text of the link, with whitespace collapsed."""
    @property
    def href(self) -> str:
        """The raw ``href`` attribute as written in the markup."""
    @property
    def element(self) -> Element:
        """The link element (``<a>`` or ``<area>``)."""
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

@final
class Resource:
    """An embedded resource URL (``<img src>``, ``<link href>``, ``<script src>``, etc.)."""

    @property
    def url(self) -> str:
        """The resolved resource URL."""
    @property
    def attr(self) -> str:
        """The attribute name (``"href"``, ``"src"``, ``"srcset"``, ``"poster"``, or ``"data"``)."""
    @property
    def value(self) -> str:
        """The raw attribute value as written before resolution."""
    @property
    def tag(self) -> str:
        """The tag name of the element."""
    @property
    def element(self) -> Element:
        """The element carrying the resource URL."""
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

@final
class Chunk:
    """A piece of a document's Markdown split at headings under a character budget."""

    @property
    def markdown(self) -> str:
        """The Markdown text of this chunk."""
    @property
    def heading_path(self) -> list[str]:
        """Plain-text titles of the headings this chunk sits under, outermost first."""
    @property
    def context(self) -> str:
        """The heading path joined with ``" > "`` for RAG context."""
    @property
    def byte_size(self) -> int:
        """Byte size of the chunk Markdown."""
    def __len__(self) -> int: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

@final
class OutlineNode:
    """An element in the semantic page outline."""

    @property
    def ref_id(self) -> int:
        """1-based index reference assigned in outline order."""
    @property
    def role(self) -> str:
        """Semantic role (``"banner"``, ``"navigation"``, ``"heading"``, ``"link"``, ``"button"``, etc.)."""
    @property
    def name(self) -> Optional[str]:
        """Accessible name, if available."""
    @property
    def level(self) -> Optional[int]:
        """Heading level (1..=6) for ``role == "heading"``."""
    @property
    def selector(self) -> str:
        """Minimal, unique CSS selector resolving back to this element."""
    @property
    def href(self) -> Optional[str]:
        """Target URL for links (``href``), if present."""
    @property
    def tag(self) -> str:
        """HTML tag name."""
    @property
    def depth(self) -> int:
        """Hierarchy depth in the outline (0 for top-level landmarks)."""
    @property
    def source_range(self) -> Optional[tuple[int, int]]:
        """Byte range in the original HTML source, if known."""
    def to_json(self) -> str:
        """Render this outline node as a JSON string."""
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

@final
class PageOutline:
    """Semantic page outline of landmarks, headings, interactive controls, and forms."""

    @property
    def nodes(self) -> list[OutlineNode]:
        """All outline nodes in document order."""
    def to_text(self) -> str:
        """Render as a compact indented text outline for model prompts."""
    def to_json(self) -> str:
        """Render as a JSON array string."""
    def __len__(self) -> int: ...
    def __getitem__(self, key: int, /) -> OutlineNode: ...
    def __iter__(self) -> Iterator[OutlineNode]: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

@final
class Document:
    """A parsed HTML document. Create one with :func:`domq.parse`.

    Immutable and safe to share between threads.
    """

    def select(self, selector: Union[str, Selector]) -> list[Element]:
        """Every element matching a CSS selector, in document order.

        Raises :class:`ValueError` if the selector is invalid or unsupported.
        """
    def select_first(self, selector: Union[str, Selector]) -> Optional[Element]:
        """The first element matching a CSS selector, or ``None``.

        Raises :class:`ValueError` if the selector is invalid or unsupported.
        """
    def main_content(self) -> Optional[Element]:
        """The element holding the page's main content ("reader mode"), or ``None``."""
    def metadata(self, base_url: Optional[str] = None) -> Metadata:
        """Page metadata: title, description, canonical URL, language, OpenGraph,
        Twitter Cards, feeds, and JSON-LD blocks."""
    def links(self, base_url: Optional[str] = None) -> list[Link]:
        """Every hyperlink (``<a href>`` and ``<area href>``) in document order."""
    def resources(self, base_url: Optional[str] = None) -> list[Resource]:
        """Every embedded resource URL in document order."""
    def to_text(self) -> str:
        """Readable plain text: no ``<script>``/``<style>``/``<head>``,
        whitespace collapsed, a line per block, a blank line between
        paragraphs."""
    def to_markdown(
        self,
        *,
        links: Optional[Literal["inline", "reference", "footer"]] = None,
        images: Optional[Literal["keep", "alt", "drop"]] = None,
        drop_data_uris: bool = False,
        base_url: Optional[str] = None,
    ) -> str:
        """CommonMark + GFM Markdown (headings, lists, links, images, code
        blocks, tables, quotes) — handy for LLM prompts and RAG indexes.

        :param links: Link formatting: ``"inline"`` (default), ``"reference"``, or ``"footer"``.
        :param images: Image formatting: ``"keep"`` (default), ``"alt"``, or ``"drop"``.
        :param drop_data_uris: If True, data: URIs in links and images are dropped.
        :param base_url: Base URL to resolve relative URLs against.
        """
    def to_markdown_chunks(
        self,
        max_chars: Optional[int] = None,
        *,
        links: Optional[Literal["inline", "reference", "footer"]] = None,
        images: Optional[Literal["keep", "alt", "drop"]] = None,
        drop_data_uris: bool = False,
        base_url: Optional[str] = None,
    ) -> list[Chunk]:
        """CommonMark + GFM Markdown split at headings into chunks under ``max_chars``."""
    def tables(self, selector: str = "table") -> list[list[list[str]]]:
        """Extract rows of cell text for tables matching ``selector`` (default ``"table"``)."""
    def to_csv(self, selector: str = "table") -> str:
        """Format matched tables as RFC 4180 CSV, separated by ``\\r\\n``."""
    def outline(self) -> PageOutline:
        """Extract a semantic page outline of landmarks, headings, interactive controls, and forms."""
    def to_html(self) -> str:
        """The document re-serialized as HTML."""
    def to_json(self) -> str:
        """The whole tree as a JSON string (``json.loads`` it for nested
        dicts)."""
    @property
    def errors(self) -> list[tuple[int, str]]:
        """Recovery diagnostics as ``(byte_offset, message)`` tuples; empty for
        clean input. Offsets index the UTF-8 encoding of the source."""
    @property
    def root(self) -> Optional[Element]:
        """The first top-level element (``<html>`` for a typical page), or
        ``None``."""
    @property
    def roots(self) -> list[Element]:
        """All top-level elements, in document order."""
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

@final
class Element:
    """An HTML element inside a :class:`Document`. Keeps its document alive.

    Elements compare equal (and hash equally) when they are the same node of
    the same document.
    """

    @property
    def tag(self) -> str:
        """The lower-cased tag name, e.g. ``"div"``."""
    @property
    def attrs(self) -> dict[str, str]:
        """The attributes as a new ``dict`` (valueless attributes map to
        ``""``)."""
    @overload
    def attr(self, name: str) -> Optional[str]:
        """The value of attribute ``name``, or ``None`` if it is absent."""
    @overload
    def attr(self, name: str, default: _T) -> Union[str, _T]:
        """The value of attribute ``name``, or ``default`` if it is absent."""
    @property
    def text(self) -> str:
        """Readable text of this element (like :meth:`Document.to_text`)."""
    def to_text(self) -> str:
        """Readable plain text of this element (like :meth:`Document.to_text`)."""
    @property
    def text_content(self) -> str:
        """All descendant text concatenated, verbatim (DOM ``textContent``)."""
    @property
    def html(self) -> str:
        """This element as HTML, including its own tags (DOM ``outerHTML``)."""
    @property
    def inner_html(self) -> str:
        """The HTML of this element's children (DOM ``innerHTML``)."""
    def select(self, selector: Union[str, Selector]) -> list[Element]:
        """Descendants matching a CSS selector, in document order.

        As with the DOM's ``querySelectorAll``, the selector is matched against
        the whole document; use ``:scope`` to anchor it here
        (``":scope > li"``). Raises :class:`ValueError` for an invalid selector.
        """
    def select_first(self, selector: Union[str, Selector]) -> Optional[Element]:
        """The first descendant matching a CSS selector, or ``None``."""
    def closest(self, selector: Union[str, Selector]) -> Optional[Element]:
        """The nearest element matching ``selector``, starting with this
        element itself and walking up its ancestors."""
    def matches(self, selector: Union[str, Selector]) -> bool:
        """Whether this element matches a CSS selector."""
    def table_rows(self) -> list[list[str]]:
        """If this element is a ``<table>``, rows of cell text with colspan/rowspan expanded."""
    def to_csv(self) -> str:
        """If this element is a ``<table>``, its cell text formatted as RFC 4180 CSV."""
    def links(self, base_url: Optional[str] = None) -> list[Link]:
        """Hyperlinks inside this element's subtree."""
    def resources(self, base_url: Optional[str] = None) -> list[Resource]:
        """Embedded resource URLs inside this element's subtree."""
    @property
    def parent(self) -> Optional[Element]:
        """The parent element, or ``None`` for a top-level element."""
    @property
    def children(self) -> list[Element]:
        """The child elements (text and comment nodes are skipped)."""
    @property
    def document(self) -> Document:
        """The :class:`Document` this element belongs to."""
    def to_markdown(
        self,
        *,
        links: Optional[Literal["inline", "reference", "footer"]] = None,
        images: Optional[Literal["keep", "alt", "drop"]] = None,
        drop_data_uris: bool = False,
        base_url: Optional[str] = None,
    ) -> str:
        """This element as Markdown.

        :param links: Link formatting: ``"inline"`` (default), ``"reference"``, or ``"footer"``.
        :param images: Image formatting: ``"keep"`` (default), ``"alt"``, or ``"drop"``.
        :param drop_data_uris: If True, data: URIs in links and images are dropped.
        :param base_url: Base URL to resolve relative URLs against.
        """
    def to_markdown_chunks(
        self,
        max_chars: Optional[int] = None,
        *,
        links: Optional[Literal["inline", "reference", "footer"]] = None,
        images: Optional[Literal["keep", "alt", "drop"]] = None,
        drop_data_uris: bool = False,
        base_url: Optional[str] = None,
    ) -> list[Chunk]:
        """CommonMark + GFM Markdown chunks for this element's subtree."""
    def outline(self) -> PageOutline:
        """Extract a semantic page outline for this element's subtree."""
    def to_json(self) -> str:
        """This element's subtree as a JSON string."""
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

# --- declarative extraction ---------------------------------------------------

def extract(cls: type[_D], source: Union[str, Document, Element]) -> _D:
    """Fill dataclass ``cls`` from ``source``.

    ``source`` is an HTML string (parsed first), a :class:`Document` or an
    :class:`Element`; it is the scope every field's selector runs in. Each
    field is declared as ``Annotated[T, Select(...)]`` (or one of the
    :class:`Attr`/:class:`Html`/:class:`OuterHtml`/:class:`Markdown`
    shorthands). ``T`` is ``str``, ``int``, ``float``, ``bool``, a nested
    dataclass, any class built from a string (``Decimal``, an ``Enum``, ...),
    ``Optional[T]`` (``None`` when nothing matches) or ``list[T]`` (every
    match).

    Raises :class:`ExtractError` when a field cannot be extracted (no match,
    missing attribute, unparsable value, invalid selector) and
    :class:`TypeError` when ``cls`` is not a usable dataclass.
    """

def extract_all(cls: type[_D], source: Union[str, Document, Element]) -> list[_D]:
    """One ``cls`` per element matching ``cls.__select__`` (a CSS selector set
    on the class), in document order, each extracted with that element as its
    scope. Raises :class:`TypeError` if ``cls`` has no ``__select__``."""

class Select:
    """Where a dataclass field's value comes from; use it as
    ``Annotated[T, Select(...)]``.

    ``selector`` picks the element (the first match, or every match for
    ``list[T]``); without one the field reads its scope (the element or
    document being extracted). The value is the element's readable text unless
    ``attr=`` (an attribute), ``html=True`` (inner HTML), ``outer_html=True``
    or ``markdown=True`` says otherwise. ``default=`` is used when nothing
    matches or the attribute is absent; ``with_=`` converts the source string
    itself and its result is used as-is.
    """

    selector: Optional[str]
    attr: Optional[str]
    html: bool
    outer_html: bool
    markdown: bool
    default: Any
    """The ``default=`` value, or ``dataclasses.MISSING`` if none was given."""
    with_: Optional[Callable[[str], Any]]
    def __init__(
        self,
        selector: Optional[str] = None,
        *,
        attr: Optional[str] = None,
        html: bool = False,
        outer_html: bool = False,
        markdown: bool = False,
        default: Any = ...,
        with_: Optional[Callable[[str], Any]] = None,
    ) -> None: ...

class Attr(Select):
    """``Attr(name)``: attribute ``name`` of the scope element;
    ``Attr(selector, name)``: of the first element matching ``selector``.
    Shorthand for ``Select(selector, attr=name)``."""

    @overload
    def __init__(
        self,
        selector: Optional[str],
        name: str,
        /,
        *,
        default: Any = ...,
        with_: Optional[Callable[[str], Any]] = None,
    ) -> None: ...
    @overload
    def __init__(
        self,
        name: str,
        /,
        *,
        default: Any = ...,
        with_: Optional[Callable[[str], Any]] = None,
    ) -> None: ...

class Html(Select):
    """Inner HTML of the match. Shorthand for ``Select(selector, html=True)``."""

    def __init__(
        self,
        selector: Optional[str] = None,
        *,
        default: Any = ...,
        with_: Optional[Callable[[str], Any]] = None,
    ) -> None: ...

class OuterHtml(Select):
    """Outer HTML of the match. Shorthand for ``Select(selector, outer_html=True)``."""

    def __init__(
        self,
        selector: Optional[str] = None,
        *,
        default: Any = ...,
        with_: Optional[Callable[[str], Any]] = None,
    ) -> None: ...

class Markdown(Select):
    """The match as Markdown. Shorthand for ``Select(selector, markdown=True)``."""

    def __init__(
        self,
        selector: Optional[str] = None,
        *,
        default: Any = ...,
        with_: Optional[Callable[[str], Any]] = None,
    ) -> None: ...

class ExtractError(ValueError):
    """A field could not be extracted: no element matched, an attribute was
    missing, a value did not parse, or a selector is invalid.

    ``str(err)`` reads like
    ``Product.price (".price" @data-value): cannot parse "$5" as float: ...``.
    """

    path: str
    """The field, e.g. ``"Product.reviews[2].rating"``."""
    selector: Optional[str]
    """The CSS selector involved, or ``None`` when the field reads its scope."""
    source: str
    """What was read: ``"text"``, ``"@<attr>"``, ``"html"``, ``"outer_html"``,
    ``"markdown"`` or ``"element"`` (a nested dataclass / ``__select__``)."""
    reason: str
    """What went wrong, without the path and selector."""
    def __init__(self, path: str, selector: Optional[str], source: str, reason: str) -> None: ...
