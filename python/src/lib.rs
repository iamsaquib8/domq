//! Python bindings for [domq](https://github.com/iamsaquib8/domq):
//! `import domq`.
//!
//! A thin layer over the core crate. Every `Element` holds a strong reference
//! to its `Document` plus a node id, so elements stay valid for as long as
//! Python keeps them around. Both classes are immutable (`frozen`), which makes
//! them safe to share between threads and lets the heavy work (parsing and
//! serializing large documents) run with the GIL released.

#![forbid(unsafe_code)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use domq::{
    Dom, Metadata as CoreMetadata, NodeId, NodeRef, OutlineNode as CoreOutlineNode,
    PageOutline as CorePageOutline, Selector as CoreSelector,
};
use pyo3::exceptions::{PyIndexError, PyRuntimeError, PyTypeError, PyValueError};
use pyo3::marker::Ungil;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedStr;
use pyo3::types::{PyDict, PyIterator, PyList, PyString};

/// Inputs at least this many bytes long are parsed with the GIL released.
const DETACH_BYTES: usize = 16 * 1024;
/// Documents with at least this many nodes are serialized with the GIL released.
const DETACH_NODES: usize = 1024;

/// Run `f`, releasing the GIL first when the work is large enough to be worth it.
fn maybe_detach<T: Ungil>(py: Python<'_>, large: bool, f: impl Ungil + FnOnce() -> T) -> T {
    if large { py.detach(f) } else { f() }
}

/// Parse a selector, raising `ValueError` (with the position) when it is invalid.
fn compile(selector: &str) -> PyResult<CoreSelector> {
    CoreSelector::parse(selector)
        .map_err(|e| PyValueError::new_err(format!("invalid selector '{selector}': {e}")))
}

enum SelectorInput<'a> {
    Compiled(CoreSelector),
    Precompiled(pyo3::PyRef<'a, Selector>),
}

impl SelectorInput<'_> {
    fn as_selector(&self) -> &CoreSelector {
        match self {
            SelectorInput::Compiled(s) => s,
            SelectorInput::Precompiled(p) => &p.inner,
        }
    }
}

fn to_selector<'a>(obj: &'a Bound<'_, PyAny>) -> PyResult<SelectorInput<'a>> {
    if let Ok(s) = obj.cast::<PyString>() {
        let text = s.to_str()?;
        let compiled = compile(text)?;
        Ok(SelectorInput::Compiled(compiled))
    } else if let Ok(sel) = obj.cast::<Selector>() {
        Ok(SelectorInput::Precompiled(sel.borrow()))
    } else {
        Err(PyTypeError::new_err(
            "selector must be str or domq.Selector",
        ))
    }
}

/// Parse an HTML document (or fragment) into a `Document`.
///
/// Never raises on malformed HTML: anything the parser had to recover from is
/// listed in `Document.errors`. Large inputs are parsed with the GIL released.
#[pyfunction]
fn parse(py: Python<'_>, html: PyBackedStr) -> Document {
    let dom = maybe_detach(py, html.len() >= DETACH_BYTES, || domq::parse(&html));
    Document { dom }
}

/// Resolve `relative` against `base`, following RFC 3986 §5.2.
#[pyfunction]
fn resolve_url(base: &str, relative: &str) -> String {
    domq::resolve_url(base, relative)
}

/// Resolve URLs in a `srcset` attribute string against `base`.
#[pyfunction]
fn resolve_srcset(base: &str, srcset: &str) -> String {
    domq::resolve_srcset(base, srcset)
}

/// A precompiled CSS selector for fast repeated queries.
#[pyclass(frozen, module = "domq")]
struct Selector {
    inner: CoreSelector,
    query: String,
}

#[pymethods]
impl Selector {
    #[new]
    fn new(query: &str) -> PyResult<Self> {
        let inner = compile(query)?;
        Ok(Selector {
            inner,
            query: query.to_string(),
        })
    }

    /// The CSS selector query string.
    #[getter]
    fn query(&self) -> &str {
        &self.query
    }

    /// Whether `element` matches this selector.
    fn matches(&self, element: &Bound<'_, Element>) -> PyResult<bool> {
        let node = element.get().node()?;
        Ok(self.inner.matches(node))
    }

    /// The nearest element matching this selector, starting with `element`
    /// itself and walking up its ancestors.
    fn closest(&self, py: Python<'_>, element: &Bound<'_, Element>) -> PyResult<Option<Element>> {
        let el = element.get();
        let node = el.node()?;
        Ok(node.closest(&self.inner).map(|n| el.wrap(py, n)))
    }

    fn __str__(&self) -> &str {
        &self.query
    }

    fn __repr__(&self) -> String {
        format!("Selector({:?})", self.query)
    }
}

/// Page metadata from `<title>`, `<meta>`, `<link>`, OpenGraph, Twitter Cards,
/// and JSON-LD blocks.
#[pyclass(frozen, module = "domq")]
struct Metadata {
    inner: CoreMetadata,
}

#[pymethods]
impl Metadata {
    /// `<title>` (not one inside `<svg>`), else `og:title`, else `twitter:title`.
    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }

    /// `<meta name="description">`, else `og:description`, else `twitter:description`.
    #[getter]
    fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    /// `<link rel="canonical" href="...">`, else `og:url`.
    #[getter]
    fn canonical(&self) -> Option<&str> {
        self.inner.canonical.as_deref()
    }

    /// The `lang` attribute of `<html>`, else `<meta http-equiv="content-language">`.
    #[getter]
    fn lang(&self) -> Option<&str> {
        self.inner.lang.as_deref()
    }

    /// Alias for `lang`.
    #[getter]
    fn language(&self) -> Option<&str> {
        self.inner.lang.as_deref()
    }

    /// `og:site_name`, else `<meta name="application-name">`.
    #[getter]
    fn site_name(&self) -> Option<&str> {
        self.inner.site_name.as_deref()
    }

    /// `og:image`, else `twitter:image`, else `<link rel="image_src">`.
    #[getter]
    fn image(&self) -> Option<&str> {
        self.inner.image.as_deref()
    }

    /// `<meta name="author">`, else `article:author`, else `<a rel="author">`.
    #[getter]
    fn author(&self) -> Option<&str> {
        self.inner.author.as_deref()
    }

    /// Publication date as written (e.g. ISO 8601).
    #[getter]
    fn published(&self) -> Option<&str> {
        self.inner.published.as_deref()
    }

    /// `<meta name="keywords">` split on commas, else `article:tag` values.
    #[getter]
    fn keywords(&self) -> Vec<String> {
        self.inner.keywords.clone()
    }

    /// OpenGraph `<meta property=... content=...>` pairs in document order.
    #[getter]
    fn og(&self) -> Vec<(String, String)> {
        self.inner.og.clone()
    }

    /// Twitter-card `<meta name="twitter:..." content=...>` pairs in document order.
    #[getter]
    fn twitter(&self) -> Vec<(String, String)> {
        self.inner.twitter.clone()
    }

    /// Raw JSON-LD strings from `<script type="application/ld+json">` blocks.
    #[getter]
    fn json_ld(&self) -> Vec<String> {
        self.inner.json_ld.clone()
    }

    /// Feed URLs from `<link rel="alternate">` (RSS, Atom, JSON Feed).
    #[getter]
    fn feeds(&self) -> Vec<String> {
        self.inner.feeds.clone()
    }

    /// Convert metadata fields into a dictionary.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("title", &self.inner.title)?;
        dict.set_item("description", &self.inner.description)?;
        dict.set_item("canonical", &self.inner.canonical)?;
        dict.set_item("lang", &self.inner.lang)?;
        dict.set_item("language", &self.inner.lang)?;
        dict.set_item("site_name", &self.inner.site_name)?;
        dict.set_item("image", &self.inner.image)?;
        dict.set_item("author", &self.inner.author)?;
        dict.set_item("published", &self.inner.published)?;
        dict.set_item("keywords", &self.inner.keywords)?;
        dict.set_item("og", &self.inner.og)?;
        dict.set_item("twitter", &self.inner.twitter)?;
        dict.set_item("json_ld", &self.inner.json_ld)?;
        dict.set_item("feeds", &self.inner.feeds)?;
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        format!(
            "<Metadata title={:?} canonical={:?} lang={:?}>",
            self.inner.title, self.inner.canonical, self.inner.lang
        )
    }
}

/// A hyperlink (`<a href>` or `<area href>`) with its resolved URL and readable text.
#[pyclass(frozen, module = "domq")]
struct Link {
    element: Py<Element>,
    url: String,
    text: String,
    href: String,
}

#[pymethods]
impl Link {
    /// The resolved URL.
    #[getter]
    fn url(&self) -> &str {
        &self.url
    }

    /// The readable text of the link, with whitespace collapsed.
    #[getter]
    fn text(&self) -> &str {
        &self.text
    }

    /// The raw `href` attribute as written in the markup.
    #[getter]
    fn href(&self) -> &str {
        &self.href
    }

    /// The link element (`<a>` or `<area>`).
    #[getter]
    fn element<'py>(&self, py: Python<'py>) -> Py<Element> {
        self.element.clone_ref(py)
    }

    fn __repr__(&self) -> String {
        format!("<Link url={:?} text={:?}>", self.url, self.text)
    }

    fn __str__(&self) -> &str {
        &self.url
    }
}

/// An embedded resource URL (`<img src>`, `<link href>`, `<script src>`, etc.).
#[pyclass(frozen, module = "domq")]
struct Resource {
    element: Py<Element>,
    url: String,
    attr: &'static str,
    value: String,
    tag: String,
}

#[pymethods]
impl Resource {
    /// The resolved resource URL.
    #[getter]
    fn url(&self) -> &str {
        &self.url
    }

    /// The attribute name (`"href"`, `"src"`, `"srcset"`, `"poster"`, or `"data"`).
    #[getter]
    fn attr(&self) -> &str {
        self.attr
    }

    /// The raw attribute value as written before resolution.
    #[getter]
    fn value(&self) -> &str {
        &self.value
    }

    /// The tag name of the element.
    #[getter]
    fn tag(&self) -> &str {
        &self.tag
    }

    /// The element carrying the resource URL.
    #[getter]
    fn element<'py>(&self, py: Python<'py>) -> Py<Element> {
        self.element.clone_ref(py)
    }

    fn __repr__(&self) -> String {
        format!(
            "<Resource tag={:?} attr={:?} url={:?}>",
            self.tag, self.attr, self.url
        )
    }

    fn __str__(&self) -> &str {
        &self.url
    }
}

/// A piece of a document's Markdown split at headings under a character budget.
#[pyclass(frozen, module = "domq")]
struct Chunk {
    markdown: String,
    heading_path: Vec<String>,
}

#[pymethods]
impl Chunk {
    /// The Markdown text of this chunk.
    #[getter]
    fn markdown(&self) -> &str {
        &self.markdown
    }

    /// Plain-text titles of the headings this chunk sits under, outermost first.
    #[getter]
    fn heading_path(&self) -> Vec<String> {
        self.heading_path.clone()
    }

    /// The heading path joined with `" > "` for RAG context.
    #[getter]
    fn context(&self) -> String {
        self.heading_path.join(" > ")
    }

    /// Byte size of the chunk Markdown.
    #[getter]
    fn byte_size(&self) -> usize {
        self.markdown.len()
    }

    fn __len__(&self) -> usize {
        self.markdown.len()
    }

    fn __str__(&self) -> &str {
        &self.markdown
    }

    fn __repr__(&self) -> String {
        let ctx = self.context();
        if ctx.is_empty() {
            format!("<Chunk len={}>", self.markdown.len())
        } else {
            format!("<Chunk context={ctx:?} len={}>", self.markdown.len())
        }
    }
}

/// An element in the semantic page outline.
#[pyclass(frozen, module = "domq")]
struct OutlineNode {
    inner: CoreOutlineNode,
}

#[pymethods]
impl OutlineNode {
    /// 1-based index reference assigned in outline order.
    #[getter]
    fn ref_id(&self) -> usize {
        self.inner.ref_id
    }

    /// Semantic role (`"banner"`, `"navigation"`, `"heading"`, `"link"`, `"button"`, etc.).
    #[getter]
    fn role(&self) -> &str {
        &self.inner.role
    }

    /// Accessible name, if available.
    #[getter]
    fn name(&self) -> Option<&str> {
        self.inner.name.as_deref()
    }

    /// Heading level (1..=6) for `role == "heading"`.
    #[getter]
    fn level(&self) -> Option<u8> {
        self.inner.level
    }

    /// Minimal, unique CSS selector resolving back to this element.
    #[getter]
    fn selector(&self) -> &str {
        &self.inner.selector
    }

    /// Target URL for links (`href`), if present.
    #[getter]
    fn href(&self) -> Option<&str> {
        self.inner.href.as_deref()
    }

    /// HTML tag name.
    #[getter]
    fn tag(&self) -> &str {
        &self.inner.tag
    }

    /// Hierarchy depth in the outline (0 for top-level landmarks).
    #[getter]
    fn depth(&self) -> usize {
        self.inner.depth
    }

    /// Byte range in the original HTML source, if known.
    #[getter]
    fn source_range(&self) -> Option<(usize, usize)> {
        self.inner.source_range
    }

    /// Render this outline node as a JSON string.
    fn to_json(&self) -> String {
        self.inner.to_json()
    }

    fn __repr__(&self) -> String {
        format!(
            "<OutlineNode ref={} role={:?} name={:?} selector={:?}>",
            self.inner.ref_id, self.inner.role, self.inner.name, self.inner.selector
        )
    }

    fn __str__(&self) -> String {
        self.inner.to_json()
    }
}

/// Semantic page outline of landmarks, headings, interactive controls, and forms.
#[pyclass(frozen, module = "domq")]
struct PageOutline {
    inner: CorePageOutline,
}

#[pymethods]
impl PageOutline {
    /// All outline nodes in document order.
    #[getter]
    fn nodes(&self) -> Vec<OutlineNode> {
        self.inner
            .nodes
            .iter()
            .cloned()
            .map(|node| OutlineNode { inner: node })
            .collect()
    }

    /// Render as a compact indented text outline for model prompts.
    fn to_text(&self) -> String {
        self.inner.to_text()
    }

    /// Render as a JSON array string.
    fn to_json(&self) -> String {
        self.inner.to_json()
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __getitem__(&self, index: isize) -> PyResult<OutlineNode> {
        let len = self.inner.len() as isize;
        let idx = if index < 0 { index + len } else { index };
        if idx < 0 || idx >= len {
            return Err(PyIndexError::new_err("index out of range"));
        }
        Ok(OutlineNode {
            inner: self.inner.nodes[idx as usize].clone(),
        })
    }

    fn __iter__<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyIterator>> {
        let py = slf.py();
        let nodes: Vec<OutlineNode> = slf
            .get()
            .inner
            .nodes
            .iter()
            .cloned()
            .map(|inner| OutlineNode { inner })
            .collect();
        let list = PyList::new(py, nodes)?;
        list.try_iter()
    }

    fn __str__(&self) -> String {
        self.inner.to_text()
    }

    fn __repr__(&self) -> String {
        format!("<PageOutline nodes={}>", self.inner.len())
    }
}

/// A parsed HTML document. Create one with `domq.parse(html)`.
#[pyclass(frozen, module = "domq")]
struct Document {
    dom: Dom,
}

impl Document {
    fn large(&self) -> bool {
        self.dom.len() >= DETACH_NODES
    }
}

fn parse_py_markdown_options(
    links: Option<&str>,
    images: Option<&str>,
    drop_data_uris: bool,
    base_url: Option<String>,
) -> PyResult<domq::MarkdownOptions> {
    let mut opts = domq::MarkdownOptions::default();
    if let Some(l) = links {
        opts.link_style = match l.to_ascii_lowercase().as_str() {
            "inline" => domq::LinkStyle::Inline,
            "reference" => domq::LinkStyle::Reference,
            "footer" => domq::LinkStyle::Footer,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown links style {other:?}; expected 'inline', 'reference' or 'footer'"
                )));
            }
        };
    }
    if let Some(img) = images {
        opts.image_style = match img.to_ascii_lowercase().as_str() {
            "keep" => domq::ImageStyle::Keep,
            "alt" => domq::ImageStyle::AltOnly,
            "drop" => domq::ImageStyle::Drop,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown images style {other:?}; expected 'keep', 'alt' or 'drop'"
                )));
            }
        };
    }
    opts.drop_data_uris = drop_data_uris;
    opts.base_url = base_url;
    Ok(opts)
}

#[pymethods]
impl Document {
    /// Every element matching a CSS selector, in document order.
    ///
    /// Accepts a CSS selector string or a precompiled `domq.Selector`.
    /// Raises `ValueError` if the selector is invalid or unsupported.
    fn select(slf: &Bound<'_, Self>, selector: &Bound<'_, PyAny>) -> PyResult<Vec<Element>> {
        let sel = to_selector(selector)?;
        let doc = slf.get();
        Ok(doc
            .dom
            .select(sel.as_selector())
            .map(|n| Element::new(slf, n.id()))
            .collect())
    }

    /// The first element matching a CSS selector, or `None`.
    ///
    /// Accepts a CSS selector string or a precompiled `domq.Selector`.
    /// Raises `ValueError` if the selector is invalid or unsupported.
    fn select_first(
        slf: &Bound<'_, Self>,
        selector: &Bound<'_, PyAny>,
    ) -> PyResult<Option<Element>> {
        let sel = to_selector(selector)?;
        Ok(slf
            .get()
            .dom
            .select_first(sel.as_selector())
            .map(|n| Element::new(slf, n.id())))
    }

    /// The element holding the page's main content ("reader mode"), or `None`.
    fn main_content(slf: &Bound<'_, Self>, py: Python<'_>) -> Option<Element> {
        let doc = slf.get();
        let target_id = maybe_detach(py, doc.large(), || doc.dom.main_content().map(|n| n.id()));
        target_id.map(|id| Element::new(slf, id))
    }

    /// Page metadata: title, description, canonical URL, language, OpenGraph,
    /// Twitter Cards, feeds, and JSON-LD blocks.
    #[pyo3(signature = (base_url = None))]
    fn metadata(&self, py: Python<'_>, base_url: Option<&str>) -> Metadata {
        let base = self.dom.base_url(base_url);
        let mut meta = maybe_detach(py, self.large(), || self.dom.metadata());
        if let Some(b) = base.as_deref() {
            if let Some(c) = meta.canonical.as_mut() {
                *c = domq::resolve_url(b, c);
            }
            if let Some(img) = meta.image.as_mut() {
                *img = domq::resolve_url(b, img);
            }
            for f in &mut meta.feeds {
                *f = domq::resolve_url(b, f);
            }
        }
        Metadata { inner: meta }
    }

    /// Every hyperlink (`<a href>` and `<area href>`) in document order.
    #[pyo3(signature = (base_url = None))]
    fn links(slf: &Bound<'_, Self>, py: Python<'_>, base_url: Option<&str>) -> PyResult<Vec<Link>> {
        let doc = slf.get();
        let raw = maybe_detach(py, doc.large(), || {
            doc.dom
                .links(base_url)
                .into_iter()
                .map(|l| (l.node.id(), l.url, l.text, l.href.to_string()))
                .collect::<Vec<_>>()
        });
        raw.into_iter()
            .map(|(id, url, text, href)| {
                let element = Py::new(py, Element::new(slf, id))?;
                Ok(Link {
                    element,
                    url,
                    text,
                    href,
                })
            })
            .collect()
    }

    /// Every embedded resource URL in document order.
    #[pyo3(signature = (base_url = None))]
    fn resources(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        base_url: Option<&str>,
    ) -> PyResult<Vec<Resource>> {
        let doc = slf.get();
        let raw = maybe_detach(py, doc.large(), || {
            doc.dom
                .resources(base_url)
                .into_iter()
                .map(|r| {
                    (
                        r.node.id(),
                        r.url,
                        r.attr,
                        r.value.to_string(),
                        r.node.tag_name().unwrap_or("").to_string(),
                    )
                })
                .collect::<Vec<_>>()
        });
        raw.into_iter()
            .map(|(id, url, attr, value, tag)| {
                let element = Py::new(py, Element::new(slf, id))?;
                Ok(Resource {
                    element,
                    url,
                    attr,
                    value,
                    tag,
                })
            })
            .collect()
    }

    /// Readable plain text: no `<script>`/`<style>`/`<head>`, whitespace
    /// collapsed, a line per block and a blank line between paragraphs.
    fn to_text(&self, py: Python<'_>) -> String {
        maybe_detach(py, self.large(), || self.dom.to_text())
    }

    /// CommonMark + GFM Markdown (headings, lists, links, images, code
    /// blocks, tables, quotes) — handy for LLM prompts and RAG indexes.
    #[pyo3(signature = (links = None, images = None, drop_data_uris = false, base_url = None))]
    fn to_markdown(
        &self,
        py: Python<'_>,
        links: Option<&str>,
        images: Option<&str>,
        drop_data_uris: bool,
        base_url: Option<String>,
    ) -> PyResult<String> {
        let opts = parse_py_markdown_options(links, images, drop_data_uris, base_url)?;
        Ok(maybe_detach(py, self.large(), || {
            self.dom.to_markdown_with(&opts)
        }))
    }

    /// CommonMark + GFM Markdown split at headings into chunks under `max_chars`.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (max_chars = None, max_bytes = None, links = None, images = None, drop_data_uris = false, base_url = None))]
    fn to_markdown_chunks(
        &self,
        py: Python<'_>,
        max_chars: Option<usize>,
        max_bytes: Option<usize>,
        links: Option<&str>,
        images: Option<&str>,
        drop_data_uris: bool,
        base_url: Option<String>,
    ) -> PyResult<Vec<Chunk>> {
        let budget = max_chars.or(max_bytes).unwrap_or(1500);
        let opts = parse_py_markdown_options(links, images, drop_data_uris, base_url)?;
        let chunks = maybe_detach(py, self.large(), || {
            self.dom.to_markdown_chunks_with(budget, &opts)
        });
        Ok(chunks
            .into_iter()
            .map(|c| Chunk {
                markdown: c.markdown,
                heading_path: c.heading_path,
            })
            .collect())
    }

    /// Extract rows of cell text for tables matching `selector` (default `"table"`).
    #[pyo3(signature = (selector = "table"))]
    fn tables(&self, py: Python<'_>, selector: &str) -> PyResult<Vec<Vec<Vec<String>>>> {
        let sel = compile(selector)?;
        Ok(maybe_detach(py, self.large(), || {
            self.dom
                .select(&sel)
                .filter(|n| n.tag_name() == Some("table"))
                .map(|t| t.table_rows())
                .collect()
        }))
    }

    /// Format matched tables as RFC 4180 CSV, separated by `\r\n`.
    #[pyo3(signature = (selector = "table"))]
    fn to_csv(&self, py: Python<'_>, selector: &str) -> PyResult<String> {
        let sel = compile(selector)?;
        Ok(maybe_detach(py, self.large(), || {
            let tables: Vec<String> = self
                .dom
                .select(&sel)
                .filter(|n| n.tag_name() == Some("table"))
                .map(|t| t.table_rows())
                .filter(|rows| !rows.is_empty())
                .map(domq::to_csv)
                .collect();
            tables.join("\r\n")
        }))
    }

    /// Extract a semantic page outline of landmarks, headings, interactive controls, and forms.
    fn outline(&self, py: Python<'_>) -> PageOutline {
        let outline = maybe_detach(py, self.large(), || self.dom.outline());
        PageOutline { inner: outline }
    }

    /// The document re-serialized as HTML.
    fn to_html(&self, py: Python<'_>) -> String {
        maybe_detach(py, self.large(), || self.dom.to_html())
    }

    /// The whole tree as a JSON string (`json.loads` it for nested dicts).
    fn to_json(&self, py: Python<'_>) -> String {
        maybe_detach(py, self.large(), || self.dom.to_json())
    }

    /// Recovery diagnostics as `(byte_offset, message)` tuples; empty for
    /// clean input. Offsets index the UTF-8 encoding of the source.
    #[getter]
    fn errors(&self) -> Vec<(usize, &'static str)> {
        self.dom
            .errors()
            .iter()
            .map(|e| (e.position, e.message()))
            .collect()
    }

    /// The first top-level element (`<html>` for a typical page), or `None`.
    #[getter]
    fn root(slf: &Bound<'_, Self>) -> Option<Element> {
        slf.get()
            .dom
            .roots()
            .find(NodeRef::is_element)
            .map(|n| Element::new(slf, n.id()))
    }

    /// All top-level elements, in document order (a fragment can have several).
    #[getter]
    fn roots(slf: &Bound<'_, Self>) -> Vec<Element> {
        slf.get()
            .dom
            .roots()
            .filter(NodeRef::is_element)
            .map(|n| Element::new(slf, n.id()))
            .collect()
    }

    fn __str__(&self, py: Python<'_>) -> String {
        self.to_html(py)
    }

    fn __repr__(&self) -> String {
        let roots: Vec<String> = self
            .dom
            .roots()
            .filter_map(|n| n.tag_name().map(|t| format!("<{t}>")))
            .collect();
        format!(
            "<Document roots=[{}] nodes={} errors={}>",
            roots.join(", "),
            self.dom.len(),
            self.dom.errors().len()
        )
    }
}

/// An HTML element inside a `Document`. Keeps its document alive.
#[pyclass(frozen, module = "domq")]
struct Element {
    doc: Py<Document>,
    id: NodeId,
}

impl Element {
    fn new(doc: &Bound<'_, Document>, id: NodeId) -> Element {
        Element {
            doc: doc.clone().unbind(),
            id,
        }
    }

    fn node(&self) -> PyResult<NodeRef<'_>> {
        // Ids come from the same (immutable) document, so this always succeeds.
        self.doc
            .get()
            .dom
            .get(self.id)
            .ok_or_else(|| PyRuntimeError::new_err("element no longer exists"))
    }

    fn wrap(&self, py: Python<'_>, node: NodeRef<'_>) -> Element {
        Element {
            doc: self.doc.clone_ref(py),
            id: node.id(),
        }
    }
}

#[pymethods]
impl Element {
    /// The lower-cased tag name, e.g. `"div"`.
    #[getter]
    fn tag(&self) -> PyResult<&str> {
        Ok(self.node()?.tag_name().unwrap_or(""))
    }

    /// The attributes as a new `dict` (for a duplicated attribute, the first
    /// occurrence wins — the same value `attr()` returns).
    #[getter]
    fn attrs<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (name, value) in self.node()?.attributes() {
            if !dict.contains(name)? {
                dict.set_item(name, value)?;
            }
        }
        Ok(dict)
    }

    /// The value of attribute `name`, or `default` if it is absent.
    #[pyo3(signature = (name, default = None))]
    fn attr<'py>(
        &self,
        py: Python<'py>,
        name: PyBackedStr,
        default: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(match self.node()?.attr(&name) {
            Some(value) => PyString::new(py, value).into_any(),
            None => default.unwrap_or_else(|| py.None().into_bound(py)),
        })
    }

    /// Readable text of this element (like `Document.to_text()`).
    #[getter]
    fn text(&self) -> PyResult<String> {
        Ok(self.node()?.to_text())
    }

    /// Readable plain text of this element: whitespace collapsed, a line per
    /// block and a blank line between paragraphs (like `Document.to_text()`).
    fn to_text(&self) -> PyResult<String> {
        self.text()
    }

    /// All descendant text concatenated, verbatim (DOM `textContent`).
    #[getter]
    fn text_content(&self) -> PyResult<String> {
        Ok(self.node()?.text_content())
    }

    /// This element as HTML, including its own tags (DOM `outerHTML`).
    #[getter]
    fn html(&self) -> PyResult<String> {
        Ok(self.node()?.outer_html())
    }

    /// The HTML of this element's children (DOM `innerHTML`).
    #[getter]
    fn inner_html(&self) -> PyResult<String> {
        Ok(self.node()?.inner_html())
    }

    /// Descendants matching a CSS selector, in document order. As with the
    /// DOM's `querySelectorAll`, the selector is matched against the whole
    /// document; use `:scope` to anchor it here (`":scope > li"`).
    fn select(&self, py: Python<'_>, selector: &Bound<'_, PyAny>) -> PyResult<Vec<Element>> {
        let sel = to_selector(selector)?;
        Ok(self
            .node()?
            .select(sel.as_selector())
            .map(|n| self.wrap(py, n))
            .collect())
    }

    /// The first descendant matching a CSS selector, or `None`.
    fn select_first(
        &self,
        py: Python<'_>,
        selector: &Bound<'_, PyAny>,
    ) -> PyResult<Option<Element>> {
        let sel = to_selector(selector)?;
        Ok(self
            .node()?
            .select_first(sel.as_selector())
            .map(|n| self.wrap(py, n)))
    }

    /// The nearest element matching `selector`, starting with this element
    /// itself and walking up its ancestors (DOM `Element.closest`).
    fn closest(&self, py: Python<'_>, selector: &Bound<'_, PyAny>) -> PyResult<Option<Element>> {
        let sel = to_selector(selector)?;
        Ok(self
            .node()?
            .closest(sel.as_selector())
            .map(|n| self.wrap(py, n)))
    }

    /// Whether this element matches a CSS selector (DOM `Element.matches`).
    fn matches(&self, selector: &Bound<'_, PyAny>) -> PyResult<bool> {
        let sel = to_selector(selector)?;
        Ok(self.node()?.matches(sel.as_selector()))
    }

    /// If this element is a `<table>`, rows of cell text with colspan/rowspan expanded.
    fn table_rows(&self) -> PyResult<Vec<Vec<String>>> {
        Ok(self.node()?.table_rows())
    }

    /// If this element is a `<table>`, its cell text formatted as RFC 4180 CSV.
    fn to_csv(&self) -> PyResult<String> {
        let rows = self.node()?.table_rows();
        Ok(if rows.is_empty() {
            String::new()
        } else {
            domq::to_csv(rows)
        })
    }

    /// Hyperlinks inside this element's subtree.
    #[pyo3(signature = (base_url = None))]
    fn links(&self, py: Python<'_>, base_url: Option<&str>) -> PyResult<Vec<Link>> {
        let this_id = self.id;
        let doc_bound = self.doc.bind(py);
        let doc = doc_bound.get();
        let raw = maybe_detach(py, doc.large(), || {
            doc.dom
                .links(base_url)
                .into_iter()
                .filter(|l| l.node.id() == this_id || l.node.ancestors().any(|a| a.id() == this_id))
                .map(|l| (l.node.id(), l.url, l.text, l.href.to_string()))
                .collect::<Vec<_>>()
        });
        raw.into_iter()
            .map(|(id, url, text, href)| {
                let element = Py::new(
                    py,
                    Element {
                        doc: self.doc.clone_ref(py),
                        id,
                    },
                )?;
                Ok(Link {
                    element,
                    url,
                    text,
                    href,
                })
            })
            .collect()
    }

    /// Embedded resource URLs inside this element's subtree.
    #[pyo3(signature = (base_url = None))]
    fn resources(&self, py: Python<'_>, base_url: Option<&str>) -> PyResult<Vec<Resource>> {
        let this_id = self.id;
        let doc_bound = self.doc.bind(py);
        let doc = doc_bound.get();
        let raw = maybe_detach(py, doc.large(), || {
            doc.dom
                .resources(base_url)
                .into_iter()
                .filter(|r| r.node.id() == this_id || r.node.ancestors().any(|a| a.id() == this_id))
                .map(|r| {
                    (
                        r.node.id(),
                        r.url,
                        r.attr,
                        r.value.to_string(),
                        r.node.tag_name().unwrap_or("").to_string(),
                    )
                })
                .collect::<Vec<_>>()
        });
        raw.into_iter()
            .map(|(id, url, attr, value, tag)| {
                let element = Py::new(
                    py,
                    Element {
                        doc: self.doc.clone_ref(py),
                        id,
                    },
                )?;
                Ok(Resource {
                    element,
                    url,
                    attr,
                    value,
                    tag,
                })
            })
            .collect()
    }

    /// The parent element, or `None` for a top-level element.
    #[getter]
    fn parent(&self, py: Python<'_>) -> PyResult<Option<Element>> {
        Ok(self
            .node()?
            .parent()
            .filter(NodeRef::is_element)
            .map(|n| self.wrap(py, n)))
    }

    /// The child elements (text and comment nodes are skipped).
    #[getter]
    fn children(&self, py: Python<'_>) -> PyResult<Vec<Element>> {
        Ok(self
            .node()?
            .child_elements()
            .map(|n| self.wrap(py, n))
            .collect())
    }

    /// The `Document` this element belongs to.
    #[getter]
    fn document(&self, py: Python<'_>) -> Py<Document> {
        self.doc.clone_ref(py)
    }

    /// This element as Markdown.
    #[pyo3(signature = (links = None, images = None, drop_data_uris = false, base_url = None))]
    fn to_markdown(
        &self,
        links: Option<&str>,
        images: Option<&str>,
        drop_data_uris: bool,
        base_url: Option<String>,
    ) -> PyResult<String> {
        let opts = parse_py_markdown_options(links, images, drop_data_uris, base_url)?;
        Ok(self.node()?.to_markdown_with(&opts))
    }

    /// CommonMark + GFM Markdown chunks for this element's subtree.
    #[pyo3(signature = (max_chars = None, max_bytes = None, links = None, images = None, drop_data_uris = false, base_url = None))]
    fn to_markdown_chunks(
        &self,
        max_chars: Option<usize>,
        max_bytes: Option<usize>,
        links: Option<&str>,
        images: Option<&str>,
        drop_data_uris: bool,
        base_url: Option<String>,
    ) -> PyResult<Vec<Chunk>> {
        let budget = max_chars.or(max_bytes).unwrap_or(1500);
        let opts = parse_py_markdown_options(links, images, drop_data_uris, base_url)?;
        let chunks = self.node()?.to_markdown_chunks_with(budget, &opts);
        Ok(chunks
            .into_iter()
            .map(|c| Chunk {
                markdown: c.markdown,
                heading_path: c.heading_path,
            })
            .collect())
    }

    /// Extract a semantic page outline for this element's subtree.
    fn outline(&self) -> PyResult<PageOutline> {
        Ok(PageOutline {
            inner: self.node()?.outline(),
        })
    }

    /// This element's subtree as a JSON string.
    fn to_json(&self) -> PyResult<String> {
        Ok(self.node()?.to_json())
    }

    fn __str__(&self) -> PyResult<String> {
        self.html()
    }

    fn __repr__(&self) -> PyResult<String> {
        const MAX: usize = 80;
        let node = self.node()?;
        let mut tag = format!("<{}", node.tag_name().unwrap_or(""));
        for (name, value) in node.attributes() {
            if value.is_empty() {
                tag.push_str(&format!(" {name}"));
            } else {
                tag.push_str(&format!(" {name}=\"{value}\""));
            }
        }
        if tag.chars().count() > MAX {
            tag = tag.chars().take(MAX).collect();
            tag.push('…');
        }
        Ok(format!("<Element {tag}>>"))
    }

    fn __eq__(&self, other: &Bound<'_, Element>) -> bool {
        let other = other.get();
        self.doc.as_ptr() == other.doc.as_ptr() && self.id == other.id
    }

    fn __hash__(&self) -> u64 {
        let mut h = DefaultHasher::new();
        (self.doc.as_ptr() as usize).hash(&mut h);
        self.id.hash(&mut h);
        h.finish()
    }
}

/// The native half of the `domq` package; import `domq` instead (its
/// `__init__.py` re-exports everything here, next to the type stubs).
#[pymodule(name = "_domq")]
mod py_domq {
    #[pymodule_export]
    use super::{
        Chunk, Document, Element, Link, Metadata, OutlineNode, PageOutline, Resource, Selector,
        parse, resolve_srcset, resolve_url,
    };

    /// The package version.
    #[pymodule_export]
    #[allow(non_upper_case_globals)]
    const __version__: &str = env!("CARGO_PKG_VERSION");
}
