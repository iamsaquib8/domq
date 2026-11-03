//! WebAssembly / JavaScript bindings for [`domq`](https://crates.io/crates/domq).
//!
//! Parse messy HTML, query it with CSS selectors, and get readable text,
//! Markdown, JSON or repaired HTML out — in the browser, on the edge (Workers,
//! Deno, Bun) or in Node, from one small `.wasm` with zero JS dependencies.
//!
//! Structured results (matches, the tree, errors) are serialized to JSON in
//! Rust and turned into plain JS objects with a single `JSON.parse`: one
//! string crossing per call, no `serde-wasm-bindgen`.
//!
//! The crate is `no_std` (plus `alloc`) so the module doesn't carry std's
//! panic machinery (~2.4 KB gzipped, measured). The parser never panics; if
//! anything ever did, the module traps (`RuntimeError: unreachable`).

#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use domq::{Dom, NodeRef, Selector};
use wasm_bindgen::prelude::*;

/// The same allocator Rust's `std` uses on `wasm32-unknown-unknown`.
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

/// Trap without formatting a message. (Tests link `std` and its handler. So
/// does building the library together with the tests, as `cargo clippy
/// --all-targets` does: the test-only dependencies enable wasm-bindgen's `std`.
/// Check the library and `--tests` separately instead.)
#[cfg(all(target_arch = "wasm32", not(test)))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/** An element in the parsed tree. */
export interface DomElement {
  type: "element";
  /** Lower-cased tag name, e.g. `"div"`. */
  tag: string;
  /** Attributes (names lower-cased, values entity-decoded; boolean attributes are `""`). */
  attrs: Record<string, string>;
  children: DomNode[];
}

/** Any node in the parsed tree. Text is already entity-decoded. */
export type DomNode =
  | DomElement
  | { type: "text"; value: string }
  | { type: "comment"; value: string }
  | { type: "doctype"; value: string };

/** The parsed document, as returned by `parse()` and `Document#tree()`. */
export interface DomDocument {
  type: "document";
  /** How many problems the parser recovered from (0 = clean input). See `errors()`. */
  errors: number;
  /** Top-level nodes, in document order. */
  children: DomNode[];
}

/** An element matched by a CSS selector. */
export interface Match {
  /** Lower-cased tag name. */
  tag: string;
  /** The element's attributes. */
  attrs: Record<string, string>;
  /** Readable text of the element (what `toText` gives: no scripts, collapsed whitespace). */
  text: string;
  /** The element's outer HTML (re-serialized, so it is well-formed). */
  html: string;
  /**
   * The element's position in a depth-first walk of the whole tree (0-based,
   * counting text, comment and doctype nodes too) — the order in which
   * `tree()` lists nodes. Use it to find a match in the tree.
   */
  nodeIndex: number;
}

/** A problem the parser recovered from. Parsing itself never fails. */
export interface ParseError {
  /** Index into the input string (a normal JS string index) where it happened. */
  position: number;
  /** Short description, e.g. `"element implicitly closed"`. */
  message: string;
}

/**
 * Thrown (as an `Error` with `name === "SelectorError"`) when a CSS selector is
 * invalid or unsupported, e.g. `"a:hover"` or `"div >"`.
 */
export interface SelectorError extends Error {
  name: "SelectorError";
  /** Index into the selector string where the problem is. */
  position: number;
}

/** The isolated core article content ("reader mode"). */
export interface MainContent {
  /** Readable text of the main content. */
  text: string;
  /** Markdown of the main content. */
  markdown: string;
  /** HTML of the main content. */
  html: string;
}

/** Page metadata from title, meta, link tags, OpenGraph, Twitter Cards, and JSON-LD. */
export interface Metadata {
  /** Page title (<title>, og:title, or twitter:title). */
  title: string | null;
  /** Meta description (description, og:description, or twitter:description). */
  description: string | null;
  /** Canonical URL (<link rel="canonical"> or og:url). */
  canonical: string | null;
  /** Page language (html[lang] or http-equiv="content-language"). */
  lang: string | null;
  /** Alias for `lang`. */
  language: string | null;
  /** Site name (og:site_name or meta[name="application-name"]). */
  siteName: string | null;
  /** Lead image (og:image, twitter:image, or link[rel="image_src"]). */
  image: string | null;
  /** Page author (meta[name="author"], article:author, or a[rel="author"]). */
  author: string | null;
  /** Publication date string (e.g. ISO 8601). */
  published: string | null;
  /** Keywords from meta[name="keywords"] or article:tag. */
  keywords: string[];
  /** OpenGraph key-value pairs in document order. */
  og: [string, string][];
  /** Twitter card key-value pairs in document order. */
  twitter: [string, string][];
  /** Raw JSON-LD blocks from <script type="application/ld+json">. */
  jsonLd: string[];
  /** Alternate feed URLs (RSS, Atom, JSON Feed). */
  feeds: string[];
}

/** A hyperlink (`<a href>` or `<area href>`) with its resolved URL and readable text. */
export interface Link {
  /** The resolved URL. */
  url: string;
  /** The readable text of the link, with whitespace collapsed. */
  text: string;
  /** The raw href attribute as written in the markup. */
  href: string;
}

/** An embedded resource URL (`<img src>`, `<link href>`, `<script src>`, etc.). */
export interface Resource {
  /** The resolved resource URL. */
  url: string;
  /** The attribute name (e.g. "href", "src", "srcset", "poster", "data"). */
  attr: string;
  /** The raw attribute value as written before resolution. */
  value: string;
  /** Lower-cased tag name of the element carrying the resource. */
  tag: string;
}

/** A piece of a document's Markdown split at headings under a character/byte budget. */
export interface Chunk {
  /** The Markdown text of this chunk. */
  markdown: string;
  /** Alias for `markdown`. */
  text: string;
  /** Plain-text titles of the headings this chunk sits under, outermost first. */
  headingPath: string[];
  /** The heading path joined with " > " for RAG context. */
  context: string;
  /** Byte size of the chunk Markdown. */
  byteSize: number;
}

/** A table represented as rows of rectangularized cell strings. */
export type Table = string[][];
"#;

// --- the Document class ----------------------------------------------------

/// A parsed HTML document. Parse once, then query it as often as you like.
///
/// Free it with `free()` (or a `using` declaration) when you are done, or let
/// the garbage collector do it.
#[wasm_bindgen]
pub struct Document {
    dom: Dom,
    /// Parse errors with positions already converted to JS string indices.
    errors: Vec<(usize, &'static str)>,
}

#[wasm_bindgen]
impl Document {
    /// Parse `html` (a full page or a fragment). Never throws: broken markup is
    /// repaired and the repairs are listed by `errors()`.
    #[wasm_bindgen(constructor)]
    pub fn new(html: &str) -> Document {
        let dom = domq::parse(html);
        let errors = js_positions(html, &dom);
        Document { dom, errors }
    }

    /// Same as `new Document(html)`.
    pub fn parse(html: &str) -> Document {
        Document::new(html)
    }

    /// Every element matching a CSS `selector`, in document order. Throws a
    /// `SelectorError` if the selector is invalid.
    #[wasm_bindgen(unchecked_return_type = "Match[]")]
    pub fn select(&self, selector: &str) -> Result<JsValue, JsValue> {
        let sel = compile(selector)?;
        let mut out = String::from("[");
        for node in self.dom.select(&sel) {
            if out.len() > 1 {
                out.push(',');
            }
            push_match(&mut out, node);
        }
        out.push(']');
        Ok(json(&out))
    }

    /// The first element matching a CSS `selector`, or `undefined`. Throws a
    /// `SelectorError` if the selector is invalid.
    #[wasm_bindgen(js_name = selectFirst, unchecked_return_type = "Match | undefined")]
    pub fn select_first(&self, selector: &str) -> Result<JsValue, JsValue> {
        let sel = compile(selector)?;
        Ok(match self.dom.select_first(&sel) {
            Some(node) => {
                let mut out = String::new();
                push_match(&mut out, node);
                json(&out)
            }
            None => JsValue::UNDEFINED,
        })
    }

    /// Readable plain text: no `<script>`/`<style>`/`<head>`, whitespace
    /// collapsed, a line per block, a blank line between paragraphs. With a
    /// `selector`, the text of every match, separated by a blank line.
    #[wasm_bindgen(js_name = toText)]
    pub fn to_text(&self, selector: Option<String>) -> Result<String, JsValue> {
        self.each(selector, Dom::to_text, |n| n.to_text())
    }

    /// Markdown (CommonMark + GFM tables): headings, lists, links, images,
    /// code blocks, tables, quotes. With a `selector`, every match converted
    /// and separated by a blank line.
    #[wasm_bindgen(js_name = toMarkdown)]
    pub fn to_markdown(&self, selector: Option<String>) -> Result<String, JsValue> {
        self.each(selector, Dom::to_markdown, |n| n.to_markdown())
    }

    /// The document re-serialized as HTML: every tag closed and nested
    /// properly, attributes quoted, text escaped.
    #[wasm_bindgen(js_name = toHtml)]
    pub fn to_html(&self) -> String {
        self.dom.to_html()
    }

    /// The whole tree as plain JS objects.
    #[wasm_bindgen(unchecked_return_type = "DomDocument")]
    pub fn tree(&self) -> JsValue {
        json(&self.dom.to_json())
    }

    /// Same as `tree()`, so `JSON.stringify(doc)` gives the JSON tree.
    #[wasm_bindgen(js_name = toJSON, unchecked_return_type = "DomDocument")]
    pub fn to_json(&self) -> JsValue {
        self.tree()
    }

    /// The problems the parser recovered from (empty for clean input).
    #[wasm_bindgen(unchecked_return_type = "ParseError[]")]
    pub fn errors(&self) -> JsValue {
        let mut out = String::from("[");
        for &(position, message) in &self.errors {
            if out.len() > 1 {
                out.push(',');
            }
            out.push_str("{\"position\":");
            push_uint(&mut out, position);
            out.push_str(",\"message\":");
            push_str(&mut out, message);
            out.push('}');
        }
        out.push(']');
        json(&out)
    }

    /// The element holding the page's main content ("reader mode"), or `undefined`.
    #[wasm_bindgen(js_name = mainContent, unchecked_return_type = "MainContent | undefined")]
    pub fn main_content(&self) -> JsValue {
        match self.dom.main_content() {
            Some(node) => {
                let mut out = String::from("{\"text\":");
                push_str(&mut out, &node.to_text());
                out.push_str(",\"markdown\":");
                push_str(&mut out, &node.to_markdown());
                out.push_str(",\"html\":");
                push_str(&mut out, &node.outer_html());
                out.push('}');
                json(&out)
            }
            None => JsValue::UNDEFINED,
        }
    }

    /// Page metadata: title, description, canonical URL, language, OpenGraph,
    /// Twitter Cards, feeds, and JSON-LD blocks.
    #[wasm_bindgen(unchecked_return_type = "Metadata")]
    pub fn metadata(&self, base_url: Option<String>) -> JsValue {
        let base = self.dom.base_url(base_url.as_deref());
        let mut meta = self.dom.metadata();
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
        let mut out = String::new();
        push_metadata(&mut out, &meta);
        json(&out)
    }

    /// Every hyperlink (`<a href>` and `<area href>`) in document order.
    #[wasm_bindgen(unchecked_return_type = "Link[]")]
    pub fn links(&self, base_url: Option<String>) -> JsValue {
        let raw = self.dom.links(base_url.as_deref());
        let mut out = String::from("[");
        for (i, link) in raw.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            push_link(&mut out, link);
        }
        out.push(']');
        json(&out)
    }

    /// Every embedded resource URL in document order.
    #[wasm_bindgen(unchecked_return_type = "Resource[]")]
    pub fn resources(&self, base_url: Option<String>) -> JsValue {
        let raw = self.dom.resources(base_url.as_deref());
        let mut out = String::from("[");
        for (i, res) in raw.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            push_resource(&mut out, res);
        }
        out.push(']');
        json(&out)
    }

    /// CommonMark + GFM Markdown split at headings into chunks under `max_chars`.
    #[wasm_bindgen(js_name = toMarkdownChunks, unchecked_return_type = "Chunk[]")]
    pub fn to_markdown_chunks(&self, max_chars: Option<usize>) -> JsValue {
        let budget = max_chars.unwrap_or(1500);
        let chunks = self.dom.to_markdown_chunks(budget);
        let mut out = String::from("[");
        for (i, chunk) in chunks.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            push_chunk(&mut out, chunk);
        }
        out.push(']');
        json(&out)
    }

    /// Extract rows of cell text for tables matching `selector` (default `"table"`).
    #[wasm_bindgen(unchecked_return_type = "Table[]")]
    pub fn tables(&self, selector: Option<String>) -> Result<JsValue, JsValue> {
        let sel_str = selector.as_deref().unwrap_or("table");
        let sel = compile(sel_str)?;
        let mut out = String::from("[");
        let mut first = true;
        for node in self
            .dom
            .select(&sel)
            .filter(|n| n.tag_name() == Some("table"))
        {
            if !first {
                out.push(',');
            }
            first = false;
            let rows = node.table_rows();
            push_table(&mut out, &rows);
        }
        out.push(']');
        Ok(json(&out))
    }

    /// Format matched tables as RFC 4180 CSV, separated by `\r\n`.
    #[wasm_bindgen(js_name = toCsv)]
    pub fn to_csv(&self, selector: Option<String>) -> Result<String, JsValue> {
        let sel_str = selector.as_deref().unwrap_or("table");
        let sel = compile(sel_str)?;
        let mut out = String::new();
        for node in self
            .dom
            .select(&sel)
            .filter(|n| n.tag_name() == Some("table"))
        {
            let rows = node.table_rows();
            if rows.is_empty() {
                continue;
            }
            if !out.is_empty() {
                out.push_str("\r\n");
            }
            out.push_str(&domq::to_csv(rows));
        }
        Ok(out)
    }
}

impl Document {
    /// The whole document through `whole`, or every match of `selector`
    /// through `part`, joined by blank lines (empty results are skipped).
    fn each(
        &self,
        selector: Option<String>,
        // Function pointers, not generics: one copy of this code, not two.
        whole: fn(&Dom) -> String,
        part: fn(NodeRef<'_>) -> String,
    ) -> Result<String, JsValue> {
        let Some(selector) = selector else {
            return Ok(whole(&self.dom));
        };
        let sel = compile(&selector)?;
        let mut out = String::new();
        for node in self.dom.select(&sel) {
            let piece = part(node);
            if piece.is_empty() {
                continue;
            }
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(&piece);
        }
        Ok(out)
    }
}

// --- one-shot functions ----------------------------------------------------

/// Parse `html` into a tree of plain JS objects (see `DomDocument`).
#[wasm_bindgen(unchecked_return_type = "DomDocument")]
pub fn parse(html: &str) -> JsValue {
    json(&domq::parse(html).to_json())
}

/// Parse `html` and return the tree as a JSON string (the same shape as
/// `parse()`), e.g. to send it over the network without re-encoding.
#[wasm_bindgen(js_name = parseJson)]
pub fn parse_json(html: &str) -> String {
    domq::parse(html).to_json()
}

/// Every element of `html` matching a CSS `selector`. Throws a
/// `SelectorError` if the selector is invalid.
#[wasm_bindgen(unchecked_return_type = "Match[]")]
pub fn select(html: &str, selector: &str) -> Result<JsValue, JsValue> {
    compile(selector)?;
    Document::new(html).select(selector)
}

/// The first element of `html` matching a CSS `selector`, or `undefined`.
/// Throws a `SelectorError` if the selector is invalid.
#[wasm_bindgen(js_name = selectFirst, unchecked_return_type = "Match | undefined")]
pub fn select_first(html: &str, selector: &str) -> Result<JsValue, JsValue> {
    compile(selector)?;
    Document::new(html).select_first(selector)
}

/// `html` as readable plain text — or, with a `selector`, the text of every
/// match separated by a blank line.
#[wasm_bindgen(js_name = toText)]
pub fn to_text(html: &str, selector: Option<String>) -> Result<String, JsValue> {
    Document::new(html).to_text(selector)
}

/// `html` as Markdown — or, with a `selector`, every match converted and
/// separated by a blank line.
#[wasm_bindgen(js_name = toMarkdown)]
pub fn to_markdown(html: &str, selector: Option<String>) -> Result<String, JsValue> {
    Document::new(html).to_markdown(selector)
}

/// `html` parsed and re-serialized: the repaired, well-formed version.
#[wasm_bindgen(js_name = toHtml)]
pub fn to_html(html: &str) -> String {
    domq::parse(html).to_html()
}

/// The problems the parser recovered from while parsing `html`.
#[wasm_bindgen(unchecked_return_type = "ParseError[]")]
pub fn errors(html: &str) -> JsValue {
    Document::new(html).errors()
}

/// The element holding the page's main content ("reader mode"), or `undefined`.
#[wasm_bindgen(js_name = mainContent, unchecked_return_type = "MainContent | undefined")]
pub fn main_content(html: &str) -> JsValue {
    Document::new(html).main_content()
}

/// Page metadata: title, description, canonical URL, language, OpenGraph,
/// Twitter Cards, feeds, and JSON-LD blocks.
#[wasm_bindgen(unchecked_return_type = "Metadata")]
pub fn metadata(html: &str, base_url: Option<String>) -> JsValue {
    Document::new(html).metadata(base_url)
}

/// Every hyperlink (`<a href>` and `<area href>`) in document order.
#[wasm_bindgen(unchecked_return_type = "Link[]")]
pub fn links(html: &str, base_url: Option<String>) -> JsValue {
    Document::new(html).links(base_url)
}

/// Every embedded resource URL in document order.
#[wasm_bindgen(unchecked_return_type = "Resource[]")]
pub fn resources(html: &str, base_url: Option<String>) -> JsValue {
    Document::new(html).resources(base_url)
}

/// CommonMark + GFM Markdown split at headings into chunks under `max_chars`.
#[wasm_bindgen(js_name = toMarkdownChunks, unchecked_return_type = "Chunk[]")]
pub fn to_markdown_chunks(html: &str, max_chars: Option<usize>) -> JsValue {
    Document::new(html).to_markdown_chunks(max_chars)
}

/// Extract rows of cell text for tables matching `selector` (default `"table"`).
#[wasm_bindgen(unchecked_return_type = "Table[]")]
pub fn tables(html: &str, selector: Option<String>) -> Result<JsValue, JsValue> {
    Document::new(html).tables(selector)
}

/// Format matched tables as RFC 4180 CSV, separated by `\r\n`.
#[wasm_bindgen(js_name = toCsv)]
pub fn to_csv(html: &str, selector: Option<String>) -> Result<String, JsValue> {
    Document::new(html).to_csv(selector)
}

/// Resolve `relative` against `base`, following RFC 3986 §5.2.
#[wasm_bindgen(js_name = resolveUrl)]
pub fn resolve_url(base: &str, relative: &str) -> String {
    domq::resolve_url(base, relative)
}

/// Resolve URLs in a `srcset` attribute string against `base`.
#[wasm_bindgen(js_name = resolveSrcset)]
pub fn resolve_srcset(base: &str, srcset: &str) -> String {
    domq::resolve_srcset(base, srcset)
}

// --- helpers ---------------------------------------------------------------

// The three JS built-ins we need, bound directly (cheaper than `js-sys`).
#[wasm_bindgen]
extern "C" {
    /// `JSON.parse`. Only ever called on JSON we built, so it cannot throw.
    #[wasm_bindgen(js_namespace = JSON, js_name = parse)]
    fn json(text: &str) -> JsValue;

    type Error;

    #[wasm_bindgen(constructor)]
    fn new(message: &str) -> Error;

    #[wasm_bindgen(method, setter)]
    fn set_name(this: &Error, name: &str);

    #[wasm_bindgen(method, setter)]
    fn set_position(this: &Error, position: u32);
}

/// Parse a selector, or build the JS `SelectorError` to throw.
fn compile(selector: &str) -> Result<Selector, JsValue> {
    Selector::parse(selector).map_err(|e| {
        // Same text as `SelectorError`'s `Display`, without `core::fmt`.
        let mut message = String::from(e.message());
        message.push_str(" at byte ");
        push_uint(&mut message, e.position());
        let err = Error::new(&message);
        err.set_name("SelectorError");
        err.set_position(utf16_len(selector.get(..e.position()).unwrap_or(selector)) as u32);
        err.into()
    })
}

fn push_opt_str(out: &mut String, val: Option<&str>) {
    match val {
        Some(s) => push_str(out, s),
        None => out.push_str("null"),
    }
}

fn push_str_array(out: &mut String, list: &[String]) {
    out.push('[');
    for (i, s) in list.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_str(out, s);
    }
    out.push(']');
}

fn push_str_pair_array(out: &mut String, list: &[(String, String)]) {
    out.push('[');
    for (i, (k, v)) in list.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('[');
        push_str(out, k);
        out.push(',');
        push_str(out, v);
        out.push(']');
    }
    out.push(']');
}

fn push_metadata(out: &mut String, meta: &domq::Metadata) {
    out.push_str("{\"title\":");
    push_opt_str(out, meta.title.as_deref());
    out.push_str(",\"description\":");
    push_opt_str(out, meta.description.as_deref());
    out.push_str(",\"canonical\":");
    push_opt_str(out, meta.canonical.as_deref());
    out.push_str(",\"lang\":");
    push_opt_str(out, meta.lang.as_deref());
    out.push_str(",\"language\":");
    push_opt_str(out, meta.lang.as_deref());
    out.push_str(",\"siteName\":");
    push_opt_str(out, meta.site_name.as_deref());
    out.push_str(",\"image\":");
    push_opt_str(out, meta.image.as_deref());
    out.push_str(",\"author\":");
    push_opt_str(out, meta.author.as_deref());
    out.push_str(",\"published\":");
    push_opt_str(out, meta.published.as_deref());
    out.push_str(",\"keywords\":");
    push_str_array(out, &meta.keywords);
    out.push_str(",\"og\":");
    push_str_pair_array(out, &meta.og);
    out.push_str(",\"twitter\":");
    push_str_pair_array(out, &meta.twitter);
    out.push_str(",\"jsonLd\":");
    push_str_array(out, &meta.json_ld);
    out.push_str(",\"feeds\":");
    push_str_array(out, &meta.feeds);
    out.push('}');
}

fn push_link(out: &mut String, link: &domq::Link<'_>) {
    out.push_str("{\"url\":");
    push_str(out, &link.url);
    out.push_str(",\"text\":");
    push_str(out, &link.text);
    out.push_str(",\"href\":");
    push_str(out, link.href);
    out.push('}');
}

fn push_resource(out: &mut String, res: &domq::Resource<'_>) {
    out.push_str("{\"url\":");
    push_str(out, &res.url);
    out.push_str(",\"attr\":");
    push_str(out, res.attr);
    out.push_str(",\"value\":");
    push_str(out, res.value);
    out.push_str(",\"tag\":");
    push_str(out, res.node.tag_name().unwrap_or(""));
    out.push('}');
}

fn push_chunk(out: &mut String, chunk: &domq::Chunk) {
    out.push_str("{\"markdown\":");
    push_str(out, &chunk.markdown);
    out.push_str(",\"text\":");
    push_str(out, &chunk.markdown);
    out.push_str(",\"headingPath\":");
    push_str_array(out, &chunk.heading_path);
    out.push_str(",\"context\":");
    push_str(out, &chunk.context());
    out.push_str(",\"byteSize\":");
    push_uint(out, chunk.markdown.len());
    out.push('}');
}

fn push_table(out: &mut String, rows: &[Vec<String>]) {
    out.push('[');
    for (r, row) in rows.iter().enumerate() {
        if r > 0 {
            out.push(',');
        }
        out.push('[');
        for (c, cell) in row.iter().enumerate() {
            if c > 0 {
                out.push(',');
            }
            push_str(out, cell);
        }
        out.push(']');
    }
    out.push(']');
}

/// Append `{"tag":…,"attrs":{…},"text":…,"html":…,"nodeIndex":N}`.
fn push_match(out: &mut String, node: NodeRef<'_>) {
    out.push_str("{\"tag\":");
    push_str(out, node.tag_name().unwrap_or(""));
    out.push_str(",\"attrs\":{");
    for (i, (k, v)) in node.attributes().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_str(out, k);
        out.push(':');
        push_str(out, v);
    }
    out.push_str("},\"text\":");
    push_str(out, &node.to_text());
    out.push_str(",\"html\":");
    push_str(out, &node.outer_html());
    out.push_str(",\"nodeIndex\":");
    // The arena is allocated in document order (and this crate never removes
    // nodes), so the arena index is the pre-order position.
    push_uint(out, node.id().index());
    out.push('}');
}

/// Append `s` as a JSON string literal.
fn push_str(out: &mut String, s: &str) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str("\\u00");
                out.push(HEX[(c as usize) >> 4] as char);
                out.push(HEX[(c as usize) & 0xf] as char);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Append a non-negative integer without going through `core::fmt`.
fn push_uint(out: &mut String, mut n: usize) {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    for &b in &buf[i..] {
        out.push(b as char);
    }
}

/// Length of `s` in UTF-16 code units (the unit of JS string indices).
fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// The document's parse errors, with byte offsets turned into JS string
/// indices. Errors come (almost) in source order, so a running cursor keeps
/// this linear in the input size.
fn js_positions(html: &str, dom: &Dom) -> Vec<(usize, &'static str)> {
    let ascii = html.is_ascii();
    let (mut byte, mut index) = (0, 0);
    let mut out = Vec::with_capacity(dom.errors().len());
    for e in dom.errors() {
        let mut at = e.position.min(html.len());
        if !ascii {
            while !html.is_char_boundary(at) {
                at -= 1;
            }
            if at < byte {
                (byte, index) = (0, 0);
            }
            index += utf16_len(html.get(byte..at).unwrap_or(""));
            byte = at;
            at = index;
        }
        out.push((at, e.message()));
    }
    out
}

// The tests live in tests/web.rs but are compiled into this crate: the
// cdylib-only crate type (see Cargo.toml) can't be linked by an integration
// test. Run them with `wasm-pack test --node`.
#[cfg(test)]
#[path = "../tests/web.rs"]
mod web;
