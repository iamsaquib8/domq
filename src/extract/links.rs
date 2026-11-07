//! Hyperlinks ([`Dom::links`]) and embedded resources ([`Dom::resources`]),
//! with URLs resolved against the page's base URL.

use alloc::string::String;
use alloc::vec::Vec;

use super::{collapse, document_order, non_empty};
use crate::node::{NodeId, NodeKind};
use crate::serialize::{is_space, walk};
use crate::tags::{self, Display};
use crate::tree::{Dom, NodeRef};
use crate::url::{is_javascript, normalize, resolve_url, srcset_candidates};

/// A hyperlink — an `<a href>` or `<area href>` — found by [`Dom::links`].
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Link<'a> {
    /// The `<a>` or `<area>` element.
    pub node: NodeRef<'a>,
    /// The `href` attribute exactly as written.
    pub href: &'a str,
    /// `href` resolved against the base URL (see [`Dom::base_url`]) with
    /// [`resolve_url`]. Without an absolute base this is `href` cleaned up the
    /// same way (surrounding whitespace removed), and may be relative.
    pub url: String,
    /// The readable link text, on one line: the element's visible text with
    /// whitespace collapsed. When that is empty (an icon link), the first of
    /// `aria-label`, `alt` (for `<area>`), the `alt` of a visible image
    /// inside, or `title`; otherwise empty.
    ///
    /// Visible means not inside a hidden element ([`Dom::to_text`] skips the
    /// same: `<script>`, `<template>`, …), nor inside an element of the link
    /// that is hidden by its `hidden` attribute or by an inline `style`
    /// declaring `display: none` or `visibility: hidden` (a simple check of
    /// the attribute: no CSS cascade). `aria-hidden` content is kept, since it
    /// is still shown on screen.
    pub text: String,
}

/// A URL-valued attribute that loads something into the page — an image,
/// script, stylesheet, media file or frame — found by [`Dom::resources`].
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Resource<'a> {
    /// The element carrying the URL.
    pub node: NodeRef<'a>,
    /// The attribute it came from: `"href"` (on `<link>`), `"src"`,
    /// `"srcset"`, `"poster"` or `"data"` (on `<object>`).
    pub attr: &'static str,
    /// The URL as written. For `srcset`, one candidate's URL (without its
    /// descriptor); a `srcset` yields one `Resource` per candidate.
    pub value: &'a str,
    /// `value` resolved against the base URL, as for [`Link::url`].
    pub url: String,
}

impl Dom {
    /// The URL relative links in this document resolve against.
    ///
    /// That is the `href` of the first `<base href>` element, resolved against
    /// `base` (typically the URL the page was fetched from); without a
    /// `<base>` element, `base` itself. `None` when neither is available.
    ///
    /// As in a browser, a `<base>` inside a `<template>` does not count (a
    /// template's content is not part of the document), and neither does one
    /// inside `<svg>` or `<math>` (it is not an HTML `<base>` there).
    ///
    /// ```
    /// let dom = domq::parse(r#"<head><base href="/docs/"></head>"#);
    /// assert_eq!(
    ///     dom.base_url(Some("https://example.com/index.html")).as_deref(),
    ///     Some("https://example.com/docs/"),
    /// );
    /// assert_eq!(dom.base_url(None).as_deref(), Some("/docs/"));
    /// assert_eq!(domq::parse("<p>x</p>").base_url(None), None);
    /// ```
    pub fn base_url(&self, base: Option<&str>) -> Option<String> {
        let mut tag = None;
        for root in self.roots() {
            // Once found, nothing more is entered: the walk only steps over
            // the remaining siblings on the way out.
            walk(root, |node, enter| match node.tag_name() {
                _ if !enter || tag.is_some() => false,
                Some("base") => {
                    tag = node.attr("href");
                    false
                }
                Some("template" | "svg" | "math") => false,
                Some(_) => true,
                None => false,
            });
            if tag.is_some() {
                break;
            }
        }
        match (tag, base) {
            (Some(href), Some(base)) => Some(resolve_url(base, href)),
            (Some(href), None) => Some(normalize(href)).filter(|h| !h.is_empty()),
            (None, Some(base)) => Some(normalize(base)),
            (None, None) => None,
        }
    }

    /// Every hyperlink in the document — each `<a href>` and `<area href>` —
    /// in document order, with its URL resolved and its readable text.
    ///
    /// URLs resolve against [`Dom::base_url`]`(base)`, so a `<base href>` in
    /// the page is honoured. `javascript:` links are skipped: they run code
    /// rather than navigate. Fragment-only (`#top`), `mailto:` and other links
    /// are kept; filter on [`Link::url`] as needed. Embedded resources (`<img
    /// src>`, `<link href>`, `<script src>`, …) are not links; see
    /// [`Dom::resources`].
    ///
    /// Link text comes from one pass over the document; in invalid nested
    /// links (`<a><a>…</a></a>`), the inner text belongs to the inner link
    /// only.
    ///
    /// ```
    /// let dom = domq::parse(
    ///     r#"<nav><a href="/docs/">Docs</a> <a href="javascript:void(0)">Menu</a></nav>
    ///        <p>See <a href="../faq.html#top">the
    ///        FAQ</a> or <a href="https://example.org"><img src=logo.png alt="Example"></a>.</p>"#,
    /// );
    /// let links = dom.links(Some("https://example.com/en/guide/"));
    /// let found: Vec<_> = links.iter().map(|l| (l.url.as_str(), l.text.as_str())).collect();
    /// assert_eq!(found, [
    ///     ("https://example.com/docs/", "Docs"),
    ///     ("https://example.com/en/faq.html#top", "the FAQ"),
    ///     ("https://example.org", "Example"),
    /// ]);
    /// assert_eq!(links[1].href, "../faq.html#top");
    /// ```
    pub fn links(&self, base: Option<&str>) -> Vec<Link<'_>> {
        let base = self.base_url(base);
        collect_links(self, None, base.as_deref().unwrap_or(""))
    }

    /// Every embedded resource URL in the document, in document order:
    /// `<link href>` (stylesheets, icons, feeds, …), every `src` (images,
    /// scripts, frames, media, …), each candidate of a `srcset`, `poster` and
    /// `<object data>`, resolved against [`Dom::base_url`]`(base)`.
    ///
    /// Empty values and `javascript:` URLs are skipped. For each element the
    /// attributes come in the order `href`, `src`, `srcset`, `poster`, `data`;
    /// if an attribute is repeated, the first one counts (as with
    /// [`NodeRef::attr`]).
    ///
    /// ```
    /// let dom = domq::parse(
    ///     r#"<link rel=stylesheet href=/main.css>
    ///        <img src="a.png" srcset="a.png 1x, a@2x.png 2x">
    ///        <script src="https://cdn.example.com/x.js"></script>"#,
    /// );
    /// let urls: Vec<_> = dom
    ///     .resources(Some("https://example.com/blog/"))
    ///     .into_iter()
    ///     .map(|r| (r.attr, r.url))
    ///     .collect();
    /// assert_eq!(urls, [
    ///     ("href", "https://example.com/main.css".to_string()),
    ///     ("src", "https://example.com/blog/a.png".to_string()),
    ///     ("srcset", "https://example.com/blog/a.png".to_string()),
    ///     ("srcset", "https://example.com/blog/a@2x.png".to_string()),
    ///     ("src", "https://cdn.example.com/x.js".to_string()),
    /// ]);
    /// ```
    pub fn resources(&self, base: Option<&str>) -> Vec<Resource<'_>> {
        let base = self.base_url(base);
        collect_resources(document_order(self), base.as_deref().unwrap_or(""))
    }
}

impl<'a> NodeRef<'a> {
    /// The hyperlinks inside this node's subtree (the node itself included),
    /// in document order: what [`Dom::links`] returns for the links under
    /// this node, with the same URLs and text, found by walking only the
    /// subtree, so the cost follows its size and not the document's.
    ///
    /// URLs resolve against [`Dom::base_url`]`(base)`. Finding a `<base>`
    /// element scans the document, so calling this for many nodes of a big
    /// document costs one scan per call; compute [`Dom::base_url`] once and
    /// use [`NodeRef::links_resolved`] instead.
    ///
    /// ```
    /// let dom = domq::parse(
    ///     r#"<div id=a><a href="/x">X</a></div><div id=b><a href="y">Y</a></div>"#,
    /// );
    /// let b = dom.find_by_tag("div").nth(1).unwrap();
    /// let links = b.links(Some("https://example.com/dir/"));
    /// assert_eq!(links.len(), 1);
    /// assert_eq!(links[0].url, "https://example.com/dir/y");
    /// assert_eq!(links[0].text, "Y");
    /// ```
    pub fn links(&self, base: Option<&str>) -> Vec<Link<'a>> {
        let base = self.dom().base_url(base);
        self.links_resolved(base.as_deref())
    }

    /// [`NodeRef::links`] with the base URL already worked out: `base` is the
    /// result of [`Dom::base_url`] (`None` when there is none), used as is.
    /// Nothing outside the subtree is scanned, apart from the node's
    /// ancestors, which decide whether it is inside hidden content.
    ///
    /// ```
    /// let dom = domq::parse(r#"<base href=/docs/><ul><li><a href=a.html>A</a></li></ul>"#);
    /// let base = dom.base_url(Some("https://example.com/"));
    /// let li = dom.find_by_tag("li").next().unwrap();
    /// let links = li.links_resolved(base.as_deref());
    /// assert_eq!(links[0].url, "https://example.com/docs/a.html");
    /// ```
    pub fn links_resolved(&self, base: Option<&str>) -> Vec<Link<'a>> {
        collect_links(self.dom(), Some(*self), base.unwrap_or(""))
    }

    /// The embedded resource URLs inside this node's subtree (the node itself
    /// included), in document order: what [`Dom::resources`] returns for the
    /// resources under this node, found by walking only the subtree. The
    /// base URL is found as in [`NodeRef::links`]; use
    /// [`NodeRef::resources_resolved`] to skip that scan.
    ///
    /// ```
    /// let dom = domq::parse(r#"<p id=a><img src=a.png></p><p id=b><img src=b.png></p>"#);
    /// let b = dom.find_by_tag("p").nth(1).unwrap();
    /// let found = b.resources(Some("https://example.com/"));
    /// assert_eq!(found.len(), 1);
    /// assert_eq!(found[0].url, "https://example.com/b.png");
    /// ```
    pub fn resources(&self, base: Option<&str>) -> Vec<Resource<'a>> {
        let base = self.dom().base_url(base);
        self.resources_resolved(base.as_deref())
    }

    /// [`NodeRef::resources`] with the base URL already worked out: `base` is
    /// the result of [`Dom::base_url`], used as is.
    ///
    /// ```
    /// let dom = domq::parse(r#"<p><img src=a.png></p>"#);
    /// let base = dom.base_url(Some("https://example.com/"));
    /// let p = dom.find_by_tag("p").next().unwrap();
    /// assert_eq!(p.resources_resolved(base.as_deref())[0].url, "https://example.com/a.png");
    /// ```
    pub fn resources_resolved(&self, base: Option<&str>) -> Vec<Resource<'a>> {
        collect_resources(
            core::iter::once(*self).chain(self.descendants()),
            base.unwrap_or(""),
        )
    }
}

/// The links of the whole document (`under` is `None`) or of the subtree of
/// `under`, with the text and state the document-wide walk would have: the
/// subtree's ancestors are entered first, in order, so a link nested in
/// another link or in hidden content reads as it does there.
fn collect_links<'a>(dom: &'a Dom, under: Option<NodeRef<'a>>, base: &str) -> Vec<Link<'a>> {
    let mut links: Vec<Link<'a>> = Vec::new();
    let mut collector = TextCollector {
        noscript: dom.noscript_is_content(),
        ..TextCollector::default()
    };
    let push = |links: &mut Vec<Link<'a>>, node: NodeRef<'a>, href: &'a str| {
        links.push(Link {
            node,
            href,
            url: resolve_url(base, href),
            text: String::new(),
        });
        links.len() - 1
    };
    let mut skip = 0;
    match under {
        None => {
            for root in dom.roots() {
                walk(root, |node, enter| {
                    collector.visit(node, enter, |node, href| push(&mut links, node, href))
                });
            }
        }
        Some(node) => {
            let ancestors: Vec<NodeRef<'a>> = node.ancestors().collect();
            for ancestor in ancestors.into_iter().rev() {
                let open =
                    collector.visit(ancestor, true, |node, href| push(&mut links, node, href));
                if !open {
                    // Inside an element whose content is not walked.
                    return Vec::new();
                }
            }
            // The ancestors that are links are not part of the subtree.
            skip = links.len();
            walk(node, |node, enter| {
                collector.visit(node, enter, |node, href| push(&mut links, node, href))
            });
        }
    }
    for (i, state) in collector.done.into_iter().enumerate().skip(skip) {
        if let Some(link) = links.get_mut(i) {
            link.text = state.finish(link.node);
        }
    }
    links.drain(..skip);
    links
}

/// The resources of `nodes`, in order, resolved against `base`.
fn collect_resources<'a>(
    nodes: impl Iterator<Item = NodeRef<'a>>,
    base: &str,
) -> Vec<Resource<'a>> {
    let mut out = Vec::new();
    for node in nodes {
        let Some(tag) = node.tag_name() else {
            continue;
        };
        let mut attrs: [Option<&'static str>; 5] =
            [None, Some("src"), Some("srcset"), Some("poster"), None];
        if tag == "link" {
            attrs[0] = Some("href");
        }
        if tag == "object" {
            attrs[4] = Some("data");
        }
        for attr in attrs.into_iter().flatten() {
            let Some(value) = node.attr(attr) else {
                continue;
            };
            if attr == "srcset" {
                for (url, _) in srcset_candidates(value) {
                    add_resource(&mut out, base, node, attr, url);
                }
            } else {
                add_resource(&mut out, base, node, attr, value);
            }
        }
    }
    out
}

fn add_resource<'a>(
    out: &mut Vec<Resource<'a>>,
    base: &str,
    node: NodeRef<'a>,
    attr: &'static str,
    value: &'a str,
) {
    if non_empty(value).is_some() && !is_javascript(value) {
        out.push(Resource {
            node,
            attr,
            value,
            url: resolve_url(base, value),
        });
    }
}

/// Collects the readable text of every link in one walk (so nested links cost
/// no extra passes): text goes to the innermost open link.
#[derive(Default)]
struct TextCollector<'a> {
    /// Open links: (element, index into `done`).
    open: Vec<(NodeRef<'a>, usize)>,
    done: Vec<LinkText<'a>>,
    /// The hidden element inside a link that the walk is in, if any: nothing
    /// in it counts toward any link's text (links in it are still found).
    hidden: Option<NodeId>,
    /// `<noscript>` holds the document's content: walk into it.
    noscript: bool,
}

#[derive(Default)]
struct LinkText<'a> {
    text: String,
    space: bool,
    /// The `alt` of the first image inside, as a fallback.
    alt: Option<&'a str>,
}

impl<'a> TextCollector<'a> {
    /// Visit a node during `walk`; `found` registers a new link and returns its
    /// index. Returns whether to descend.
    fn visit(
        &mut self,
        node: NodeRef<'a>,
        enter: bool,
        mut found: impl FnMut(NodeRef<'a>, &'a str) -> usize,
    ) -> bool {
        let tag = match node.kind() {
            NodeKind::Element { tag } => tag,
            NodeKind::Text(t) => {
                if self.hidden.is_none()
                    && let Some(link) = self.current()
                {
                    link.push(t);
                }
                return false;
            }
            NodeKind::Comment(_) | NodeKind::Doctype(_) => return false,
        };
        if !enter {
            if self.open.last().is_some_and(|(n, _)| n.id() == node.id()) {
                self.open.pop();
            }
            if self.hidden.is_some() {
                if self.hidden == Some(node.id()) {
                    self.hidden = None;
                }
                return false;
            }
            if tags::display(tag) != Display::Inline
                && let Some(link) = self.current()
            {
                link.space = true;
            }
            return false;
        }
        let display = tags::display(tag);
        if display == Display::Hidden && !(tag == "noscript" && self.noscript) {
            return false;
        }
        if self.hidden.is_none() && !self.open.is_empty() && node.hides_itself() {
            self.hidden = Some(node.id());
        }
        if self.hidden.is_none()
            && let Some(link) = self.current()
        {
            if display != Display::Inline || tag == "br" {
                link.space = true;
            }
            if tag == "img" && link.alt.is_none() {
                link.alt = node.attr("alt").and_then(non_empty);
            }
        }
        if matches!(tag, "a" | "area")
            && let Some(href) = node.attr("href").filter(|h| !is_javascript(h))
        {
            let index = found(node, href);
            // `found` pushes links in order, so `index == done.len()`.
            self.done.push(LinkText::default());
            self.open.push((node, index));
        }
        true
    }

    fn current(&mut self) -> Option<&mut LinkText<'a>> {
        let &(_, index) = self.open.last()?;
        self.done.get_mut(index)
    }
}

impl<'a> LinkText<'a> {
    fn push(&mut self, text: &str) {
        for (i, word) in text.split(is_space).enumerate() {
            if i > 0 {
                self.space = true;
            }
            if !word.is_empty() {
                if self.space && !self.text.is_empty() {
                    self.text.push(' ');
                }
                self.space = false;
                self.text.push_str(word);
            }
        }
    }

    fn finish(self, node: NodeRef<'a>) -> String {
        if !self.text.is_empty() {
            return self.text;
        }
        let attr = |name| node.attr(name).and_then(non_empty);
        attr("aria-label")
            .or_else(|| attr("alt"))
            .or(self.alt)
            .or_else(|| attr("title"))
            .map(collapse)
            .unwrap_or_default()
    }
}
