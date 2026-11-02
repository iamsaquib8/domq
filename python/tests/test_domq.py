"""Tests for the `domq` Python bindings (`pip install domq`)."""

import gc
import json
import random
import re
import threading

import pytest

import domq

ARTICLE = (
    "<article><h2>News</h2><p>It <em>works</em>!</p>"
    "<script>track()</script></article>"
)

PAGE = """<!doctype html>
<html>
<head><title>Shop</title><style>p { color: red }</style></head>
<body>
  <nav><a href="/">Home</a> <a href="/about">About</a></nav>
  <main id="content">
    <div class="card featured" data-id="1">
      <h2 class="title">Widget</h2>
      <p class="price">$9.99</p>
      <a class="buy" href="/buy/1">Buy</a>
    </div>
    <div class="card" data-id="2">
      <h2 class="title">Gadget</h2>
      <p class="price">$19.99</p>
      <a class="buy" href="/buy/2">Buy</a>
    </div>
    <ul><li>one<li>two<li>three</ul>
  </main>
</body>
</html>"""


@pytest.fixture
def page():
    return domq.parse(PAGE)


# --- module ------------------------------------------------------------------


def test_version():
    assert re.fullmatch(r"\d+\.\d+\.\d+.*", domq.__version__)


def test_classes_are_not_constructible():
    with pytest.raises(TypeError):
        domq.Element()
    with pytest.raises(TypeError):
        domq.Document()
    with pytest.raises(TypeError):
        domq.Metadata()
    with pytest.raises(TypeError):
        domq.Link()
    with pytest.raises(TypeError):
        domq.Resource()
    with pytest.raises(TypeError):
        domq.Chunk()
    with pytest.raises(TypeError):
        domq.OutlineNode()
    with pytest.raises(TypeError):
        domq.PageOutline()


def test_parse_requires_str():
    with pytest.raises(TypeError):
        domq.parse(b"<p>bytes</p>")
    with pytest.raises(TypeError):
        domq.parse(None)


# --- Document ----------------------------------------------------------------


def test_document_serializers():
    doc = domq.parse(ARTICLE)
    assert doc.to_text() == "News\n\nIt works!"
    assert doc.to_markdown() == "## News\n\nIt *works*!"
    assert doc.to_html() == ARTICLE
    assert str(doc) == ARTICLE


def test_document_to_markdown_options():
    doc = domq.parse(
        '<p><a href="/guide">Guide</a> <img src="data:image/png;base64,abc" alt="Logo"></p>'
    )
    assert doc.to_markdown(links="reference") == (
        "[Guide][1] ![Logo](data:image/png;base64,abc)\n\n[1]: /guide"
    )
    assert doc.to_markdown(links="footer") == (
        "[Guide][1] ![Logo](data:image/png;base64,abc)\n\n### Links\n\n1. [Guide](/guide)"
    )
    assert doc.to_markdown(images="alt") == "[Guide](/guide) Logo"
    assert doc.to_markdown(images="drop") == "[Guide](/guide)"
    assert doc.to_markdown(drop_data_uris=True) == "[Guide](/guide) Logo"
    assert doc.to_markdown(base_url="https://example.com/blog/") == (
        "[Guide](https://example.com/guide) ![Logo](data:image/png;base64,abc)"
    )

    import pytest

    with pytest.raises(ValueError, match="unknown links style"):
        doc.to_markdown(links="invalid")
    with pytest.raises(ValueError, match="unknown images style"):
        doc.to_markdown(images="invalid")


def test_document_to_json():
    data = json.loads(domq.parse("<p class=a>hi</p>").to_json())
    assert data["type"] == "document"
    (p,) = data["children"]
    assert p["tag"] == "p"
    assert p["attrs"] == {"class": "a"}
    assert p["children"] == [{"type": "text", "value": "hi"}]


def test_root_and_roots(page):
    # The doctype is not an element, so `root` is <html>.
    assert page.root.tag == "html"
    assert [e.tag for e in page.roots] == ["html"]

    frag = domq.parse("text <b>one</b><!-- c --><i>two</i>")
    assert frag.root.tag == "b"
    assert [e.tag for e in frag.roots] == ["b", "i"]

    empty = domq.parse("")
    assert empty.root is None
    assert empty.roots == []
    assert empty.to_text() == ""


def test_errors():
    assert domq.parse("<p>clean</p>").errors == []
    errors = domq.parse("<div><p>x</span></div>").errors
    assert errors, "expected a recovery diagnostic"
    for pos, msg in errors:
        assert isinstance(pos, int) and pos >= 0
        assert isinstance(msg, str) and msg
    assert any("stray end tag" in msg for _, msg in errors)


def test_repr(page):
    r = repr(page)
    assert r.startswith("<Document roots=[<html>]")
    assert f"errors={len(page.errors)}>" in r
    assert repr(domq.parse("<p>a</p><p>b</p>")) == "<Document roots=[<p>, <p>] nodes=4 errors=0>"


# --- selectors ---------------------------------------------------------------


def test_select(page):
    titles = page.select(".card .title")
    assert [t.text for t in titles] == ["Widget", "Gadget"]
    assert len(page.select("div.card, ul")) == 3
    assert [li.text for li in page.select("ul > li:nth-child(odd)")] == ["one", "three"]
    assert page.select("table") == []


def test_select_first(page):
    assert page.select_first("a.buy").attr("href") == "/buy/1"
    assert page.select_first(".card:not(.featured) a").attr("href") == "/buy/2"
    assert page.select_first("table") is None


def test_has_and_contains(page):
    assert page.select_first("div:has(> h2:contains('Gadget'))").attr("data-id") == "2"


@pytest.mark.parametrize("bad", ["p)", "a:hover", "p::before", "ul >", "[x", ""])
def test_invalid_selector_raises_value_error(page, bad):
    with pytest.raises(ValueError, match="invalid selector"):
        page.select(bad)
    with pytest.raises(ValueError):
        page.select_first(bad)
    el = page.root
    for method in (el.select, el.select_first, el.closest, el.matches):
        with pytest.raises(ValueError):
            method(bad)


def test_invalid_selector_message_has_position(page):
    with pytest.raises(ValueError) as info:
        page.select("div > p)")
    assert "unexpected ')' at byte 7" in str(info.value)
    with pytest.raises(ValueError, match="unsupported pseudo-class ':hover'"):
        page.select("a:hover")


def test_selector_must_be_str(page):
    with pytest.raises(TypeError):
        page.select(42)


# --- Element -----------------------------------------------------------------


def test_element_basics(page):
    card = page.select_first("div.card")
    assert card.tag == "div"
    assert card.attrs == {"class": "card featured", "data-id": "1"}
    assert card.attr("data-id") == "1"
    assert card.attr("missing") is None
    assert card.attr("missing", "fallback") == "fallback"
    assert card.attr("missing", 0) == 0
    assert card.matches("div.featured")
    assert not card.matches("span")


def test_attrs_is_a_fresh_dict(page):
    card = page.select_first("div.card")
    attrs = card.attrs
    attrs["class"] = "changed"
    assert card.attr("class") == "card featured"


def test_boolean_and_duplicate_attributes():
    el = domq.parse('<input disabled type="text" type="number">').root
    assert el.attr("disabled") == ""
    assert el.attrs["disabled"] == ""
    # The first of a duplicated attribute wins, in both accessors.
    assert el.attr("type") == "text"
    assert el.attrs["type"] == "text"


def test_text_vs_text_content():
    el = domq.parse("<div><p>a  b</p><script>x()</script><p>c</p></div>").root
    assert el.text == "a b\n\nc"
    assert el.text_content == "a  bx()c"


def test_html_and_inner_html():
    el = domq.parse('<ul id="l"><li>a</li><li>b &amp; c</li></ul>').root
    assert el.html == '<ul id="l"><li>a</li><li>b &amp; c</li></ul>'
    assert el.inner_html == "<li>a</li><li>b &amp; c</li>"
    assert str(el) == el.html
    assert el.children[1].text == "b & c"


def test_element_select_is_scoped_to_descendants(page):
    second = page.select("div.card")[1]
    assert [a.attr("href") for a in second.select("a")] == ["/buy/2"]
    assert second.select_first("h2").text == "Gadget"
    ul = page.select_first("ul")
    assert len(ul.select(":scope > li")) == 3


def test_tree_navigation(page):
    price = page.select_first("p.price")
    card = price.parent
    assert card.tag == "div" and card.attr("data-id") == "1"
    assert [c.tag for c in card.children] == ["h2", "p", "a"]
    assert price.closest(".card") == card
    assert price.closest("p") == price  # self-inclusive
    assert price.closest("table") is None
    assert page.root.parent is None


def test_element_markdown_and_json(page):
    assert page.select_first("a.buy").to_markdown() == "[Buy](/buy/1)"
    assert (
        page.select_first("a.buy").to_markdown(links="reference")
        == "[Buy][1]\n\n[1]: /buy/1"
    )
    data = json.loads(page.select_first("h2").to_json())
    assert data["tag"] == "h2"


def test_element_equality_and_hash(page):
    a1 = page.select_first("main")
    a2 = page.select("main")[0]
    assert a1 is not a2
    assert a1 == a2
    assert hash(a1) == hash(a2)
    assert len({a1, a2}) == 1
    assert a1 != page.root
    assert a1 != "main"
    # Same markup, different documents: different nodes.
    other = domq.parse(PAGE).select_first("main")
    assert other != a1


def test_element_repr():
    el = domq.parse('<a href="/x" class="btn" hidden>go</a>').root
    assert repr(el) == '<Element <a href="/x" class="btn" hidden>>'
    long = domq.parse("<p " + " ".join(f'data-a{i}="{i}"' for i in range(50)) + ">").root
    assert len(repr(long)) < 100
    assert repr(long).endswith("…>>")


def test_element_keeps_document_alive():
    def first_link():
        return domq.parse('<p>see <a href="/x">here</a></p>').select_first("a")

    link = first_link()
    gc.collect()
    assert link.attr("href") == "/x"
    assert link.parent.text == "see here"
    assert link.document.root.tag == "p"
    assert link.document.to_text() == "see here"


def test_elements_outlive_the_document_variable():
    doc = domq.parse("<p><b>x</b></p>")
    elements = doc.select("*")
    del doc
    gc.collect()
    assert [e.tag for e in elements] == ["p", "b"]
    assert elements[1].parent == elements[0]
    del elements
    gc.collect()


# --- robustness ----------------------------------------------------------------


def test_unicode():
    doc = domq.parse("<p title='naïve'>héllo 🌍 &amp; 日本語</p>")
    p = doc.root
    assert p.text == "héllo 🌍 & 日本語"
    assert p.attr("title") == "naïve"
    assert doc.select_first("p[title='naïve']") == p


def test_error_offsets_are_utf8_bytes():
    src = "<p>é</span>"
    ((pos, _),) = [e for e in domq.parse(src).errors if "stray" in e[1]]
    assert src.encode()[pos:].startswith(b"</span>")


def test_never_raises_on_garbage():
    rng = random.Random(1234)
    alphabet = "<>/=\"'!-&;#abcdpliu x\n\té\U0001f600[]()"
    for _ in range(300):
        html = "".join(rng.choice(alphabet) for _ in range(rng.randrange(0, 200)))
        doc = domq.parse(html)
        doc.to_text()
        doc.to_markdown()
        doc.to_html()
        json.loads(doc.to_json())
        for el in doc.select("*"):
            el.attrs
            el.text


def test_lone_surrogate_is_rejected():
    with pytest.raises(UnicodeEncodeError):
        domq.parse("<p>\ud800</p>")


def test_deep_nesting_does_not_overflow():
    html = "<div>" * 50_000 + "deep" + "</div>" * 50_000
    doc = domq.parse(html)
    assert doc.to_text() == "deep"
    assert len(doc.select("div")) == 50_000


def test_large_input_and_threads():
    # Large inputs take the GIL-released path; run several at once.
    rows = "".join(f"<tr><td>{i}</td><td><a href='/r/{i}'>row {i}</a></td></tr>" for i in range(5000))
    html = f"<table>{rows}</table>"
    assert len(html) > 64 * 1024
    results = []

    def work():
        doc = domq.parse(html)
        results.append((len(doc.select("tr")), len(doc.to_markdown()) > 0))

    threads = [threading.Thread(target=work) for _ in range(4)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    assert results == [(5000, True)] * 4


# --- the README examples ---------------------------------------------------------


def test_readme_scraping_example():
    doc = domq.parse(PAGE)
    products = [
        {
            "name": card.select_first(".title").text,
            "price": card.select_first(".price").text,
            "url": card.select_first("a.buy").attr("href"),
        }
        for card in doc.select("div.card")
    ]
    assert products == [
        {"name": "Widget", "price": "$9.99", "url": "/buy/1"},
        {"name": "Gadget", "price": "$19.99", "url": "/buy/2"},
    ]


def test_readme_markdown_example():
    html = """
    <html><head><title>Post</title><script>analytics()</script></head>
    <body>
      <nav><a href="/">Home</a></nav>
      <article>
        <h1>Release notes</h1>
        <p>Version <strong>2.0</strong> is out. See the <a href="/docs">docs</a>.</p>
        <ul><li>Faster</li><li>Smaller</li></ul>
        <table><tr><th>OS</th><th>Status</th></tr><tr><td>Linux</td><td>ok</td></tr></table>
      </article>
    </body></html>
    """
    doc = domq.parse(html)
    md = doc.select_first("article").to_markdown()
    assert md == (
        "# Release notes\n\n"
        "Version **2.0** is out. See the [docs](/docs).\n\n"
        "- Faster\n- Smaller\n\n"
        "| OS | Status |\n| --- | --- |\n| Linux | ok |"
    )
    assert "analytics" not in doc.to_markdown()


# --- precompiled selectors ----------------------------------------------------


def test_precompiled_selector(page):
    sel = domq.Selector("div.card")
    assert sel.query == "div.card"
    assert str(sel) == "div.card"
    assert repr(sel) == 'Selector("div.card")'

    cards = page.select(sel)
    assert len(cards) == 2
    assert page.select_first(sel) == cards[0]

    first_card = cards[0]
    assert sel.matches(first_card)
    assert first_card.matches(sel)

    title = first_card.select_first("h2.title")
    assert sel.closest(title) == first_card
    assert title.closest(sel) == first_card

    # Invalid selector string in constructor raises ValueError with details
    with pytest.raises(ValueError, match="invalid selector"):
        domq.Selector("div >")

    # Wrong type for selector argument raises TypeError
    with pytest.raises(TypeError, match="selector must be str or domq.Selector"):
        page.select(12345)
    with pytest.raises(TypeError, match="selector must be str or domq.Selector"):
        first_card.matches(None)


# --- URL resolution -----------------------------------------------------------


def test_url_resolution():
    base = "https://example.com/blog/post.html?x=1"
    assert domq.resolve_url(base, "../img/a.png") == "https://example.com/img/a.png"
    assert domq.resolve_url(base, "/about") == "https://example.com/about"
    assert domq.resolve_url(base, "#top") == "https://example.com/blog/post.html?x=1#top"
    assert domq.resolve_url(base, "//cdn.example.com/x.js") == "https://cdn.example.com/x.js"
    assert domq.resolve_url(base, "mailto:hi@example.com") == "mailto:hi@example.com"
    assert domq.resolve_url("/docs/", "a.html") == "a.html"

    srcset = "a.png 1x, /b.png 2x"
    resolved = domq.resolve_srcset("https://example.com/p/", srcset)
    assert resolved == "https://example.com/p/a.png 1x, https://example.com/b.png 2x"


# --- reader mode (main_content) -----------------------------------------------


def test_main_content():
    html = """
    <html>
    <head><title>Blog</title></head>
    <body>
      <nav><a href="/">Home</a> <a href="/blog">Blog</a></nav>
      <div class="post">
        <h1>Understanding PyO3</h1>
        <p>PyO3 provides Rust bindings for Python, allowing you to write native extensions with ease.</p>
        <p>It provides memory safety, fearless concurrency without the GIL, and seamless integration.</p>
      </div>
      <div id="comments"><p>First comment here!</p></div>
      <footer>Copyright 2026</footer>
    </body>
    </html>
    """
    doc = domq.parse(html)
    main = doc.main_content()
    assert main is not None
    assert main.attr("class") == "post"
    assert "Understanding PyO3" in main.to_text()
    assert "First comment" not in main.to_text()

    empty = domq.parse("<nav><a href='/'>Only Nav</a></nav>")
    assert empty.main_content() is None


# --- metadata -----------------------------------------------------------------


def test_metadata():
    html = """
    <html lang="en">
    <head>
      <title>Article Title</title>
      <meta name="description" content="Article summary here.">
      <link rel="canonical" href="/canonical-post">
      <meta property="og:site_name" content="My Site">
      <meta property="og:image" content="/images/og.png">
      <meta name="author" content="Jane Doe">
      <meta property="article:published_time" content="2026-11-01T12:00:00Z">
      <meta name="keywords" content="rust, python, bindings">
      <meta property="og:title" content="OG Title">
      <meta name="twitter:card" content="summary_large_image">
      <script type="application/ld+json">{"@context": "https://schema.org", "@type": "NewsArticle"}</script>
      <link rel="alternate" type="application/rss+xml" href="/rss.xml">
    </head>
    <body><p>Content</p></body>
    </html>
    """
    doc = domq.parse(html)
    meta = doc.metadata(base_url="https://example.com/posts/")
    assert meta.title == "Article Title"
    assert meta.description == "Article summary here."
    assert meta.canonical == "https://example.com/canonical-post"
    assert meta.lang == "en"
    assert meta.language == "en"
    assert meta.site_name == "My Site"
    assert meta.image == "https://example.com/images/og.png"
    assert meta.author == "Jane Doe"
    assert meta.published == "2026-11-01T12:00:00Z"
    assert meta.keywords == ["rust", "python", "bindings"]
    assert any(k == "og:title" and v == "OG Title" for k, v in meta.og)
    assert any(k == "twitter:card" and v == "summary_large_image" for k, v in meta.twitter)
    assert len(meta.json_ld) == 1
    assert "NewsArticle" in meta.json_ld[0]
    assert meta.feeds == ["https://example.com/rss.xml"]

    d = meta.to_dict()
    assert d["title"] == "Article Title"
    assert d["canonical"] == "https://example.com/canonical-post"
    assert d["lang"] == "en"
    assert d["language"] == "en"
    assert "Article Title" in repr(meta)


# --- links & resources --------------------------------------------------------


def test_links_and_resources():
    html = """
    <div id="wrapper">
      <nav>
        <a href="/home">Home</a>
        <a href="../about">About Us</a>
      </nav>
      <main>
        <p>Check the <a href="docs.html"><img src="/logo.png" alt="Docs Logo"></a>.</p>
        <link rel="stylesheet" href="/assets/style.css">
        <script src="/app.js"></script>
      </main>
    </div>
    """
    doc = domq.parse(html)
    base = "https://example.com/sub/"
    links = doc.links(base)
    assert len(links) == 3
    assert links[0].url == "https://example.com/home"
    assert links[0].text == "Home"
    assert links[0].href == "/home"
    assert links[0].element.tag == "a"
    assert links[1].url == "https://example.com/about"
    assert links[1].text == "About Us"
    assert links[2].url == "https://example.com/sub/docs.html"
    assert links[2].text == "Docs Logo"
    assert str(links[0]) == "https://example.com/home"
    assert "<Link" in repr(links[0])

    resources = doc.resources(base)
    assert len(resources) == 3
    urls = [r.url for r in resources]
    assert "https://example.com/assets/style.css" in urls
    assert "https://example.com/logo.png" in urls
    assert "https://example.com/app.js" in urls
    img_res = next(r for r in resources if r.attr == "src" and "logo" in r.url)
    assert img_res.tag == "img"
    assert img_res.value == "/logo.png"
    assert img_res.element.tag == "img"
    assert str(img_res) == "https://example.com/logo.png"
    assert "<Resource" in repr(img_res)

    # Scoped to element
    nav = doc.select_first("nav")
    nav_links = nav.links(base)
    assert len(nav_links) == 2
    assert [l.text for l in nav_links] == ["Home", "About Us"]
    assert nav.resources(base) == []

    main = doc.select_first("main")
    assert len(main.links(base)) == 1
    assert len(main.resources(base)) == 3


# --- tables & CSV -------------------------------------------------------------


def test_tables_and_csv():
    html = """
    <div>
      <table id="t1">
        <thead>
          <tr><th>Item</th><th>Qty</th><th>Price</th></tr>
        </thead>
        <tbody>
          <tr><td rowspan="2">Apples</td><td>10</td><td>$1.50</td></tr>
          <tr><td>5</td><td>$1.40</td></tr>
          <tr><td colspan="2">Total</td><td>$2.90</td></tr>
        </tbody>
      </table>
      <table id="t2">
        <tr><th>User</th><th>Status</th></tr>
        <tr><td>Alice</td><td>Active</td></tr>
      </table>
    </div>
    """
    doc = domq.parse(html)
    t1 = doc.select_first("#t1")
    rows = t1.table_rows()
    assert rows == [
        ["Item", "Qty", "Price"],
        ["Apples", "10", "$1.50"],
        ["Apples", "5", "$1.40"],
        ["Total", "Total", "$2.90"],
    ]
    csv = t1.to_csv()
    assert csv == (
        "Item,Qty,Price\r\n"
        "Apples,10,$1.50\r\n"
        "Apples,5,$1.40\r\n"
        "Total,Total,$2.90\r\n"
    )

    # Non-table elements return empty
    assert doc.select_first("div").table_rows() == []
    assert doc.select_first("div").to_csv() == ""

    # Document-level tables and CSV
    all_tables = doc.tables()
    assert len(all_tables) == 2
    assert all_tables[0] == rows
    assert all_tables[1] == [["User", "Status"], ["Alice", "Active"]]

    doc_csv = doc.to_csv()
    assert "Apples" in doc_csv
    assert "Alice,Active" in doc_csv


# --- RAG markdown chunking ----------------------------------------------------


def test_markdown_chunks():
    html = """
    <h1>Chapter 1: Getting Started</h1>
    <p>Introduction to domq parser and fast HTML manipulation.</p>
    <h2>Installation</h2>
    <p>Run pip install domq to get the wheel.</p>
    <h2>Quickstart</h2>
    <p>Import domq and parse your HTML documents.</p>
    <h1>Chapter 2: Advanced Usage</h1>
    <p>Details about reader mode and metadata extraction.</p>
    """
    doc = domq.parse(html)
    chunks = doc.to_markdown_chunks(max_chars=120)
    assert len(chunks) >= 3
    assert chunks[0].heading_path == ["Chapter 1: Getting Started"]
    assert chunks[0].context == "Chapter 1: Getting Started"
    assert chunks[0].byte_size == len(chunks[0].markdown.encode("utf-8"))
    assert len(chunks[0]) == len(chunks[0].markdown)
    assert str(chunks[0]) == chunks[0].markdown
    assert "<Chunk" in repr(chunks[0])

    # Element level chunking
    h1 = doc.select_first("h1")
    h1_chunks = h1.to_markdown_chunks(max_chars=100)
    assert len(h1_chunks) == 1
    assert "Chapter 1" in h1_chunks[0].markdown

    # Supports max_bytes alias
    bytes_chunks = doc.to_markdown_chunks(max_bytes=120)
    assert len(bytes_chunks) == len(chunks)


# --- page outline -------------------------------------------------------------


def test_page_outline():
    html = """
    <!doctype html>
    <html>
    <head><title>Test Outline</title></head>
    <body>
      <header role="banner">
        <h1>Site Header</h1>
      </header>
      <nav role="navigation">
        <a href="/home">Home</a>
        <button id="btn-menu">Menu</button>
      </nav>
      <main id="main-content">
        <h2>Article Heading</h2>
        <form id="contact-form">
          <input type="text" name="username" aria-label="Username">
          <button type="submit">Send</button>
        </form>
      </main>
    </body>
    </html>
    """
    doc = domq.parse(html)
    outline = doc.outline()
    assert len(outline) > 0
    assert len(outline.nodes) == len(outline)

    # Sequence protocol & indexing
    first = outline[0]
    assert first.ref_id == 1
    assert first.role in ("banner", "heading")
    assert outline[-1].ref_id == len(outline)
    with pytest.raises(IndexError):
        _ = outline[9999]

    # Iteration
    nodes = list(outline)
    assert len(nodes) == len(outline)

    # Outline node properties
    button_node = next(n for n in nodes if n.role == "button" and n.name == "Menu")
    assert button_node.tag == "button"
    assert button_node.selector == "#btn-menu"
    assert button_node.depth >= 1
    assert "button" in button_node.to_json()
    assert "Menu" in repr(button_node)
    assert str(button_node) == button_node.to_json()

    # PageOutline formatting
    text = outline.to_text()
    assert "Site Header" in text
    assert "Menu" in text
    assert str(outline) == text
    assert "<PageOutline" in repr(outline)

    json_str = outline.to_json()
    parsed_json = json.loads(json_str)
    assert isinstance(parsed_json, list)
    assert len(parsed_json) == len(outline)

    # Element outline
    main_el = doc.select_first("main")
    main_outline = main_el.outline()
    assert len(main_outline) < len(outline)
    assert any(n.role == "heading" and n.name == "Article Heading" for n in main_outline)

