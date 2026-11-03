//! Semantic page outline for browser-free AI models ([`Dom::outline`]).
//!
//! Extracts structural landmarks (`banner`, `main`, `navigation`, `aside`,
//! `contentinfo`, `region`, `article`, `form`, `dialog`), headings (`h1`–`h6`),
//! and interactive controls (`button`, `link`, `textbox`, `searchbox`,
//! `combobox`, `checkbox`, `radio`), with accessible names and minimal unique
//! CSS selectors.

use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use super::{collapse, is_space};
use crate::NodeId;
use crate::serialize::walk;
use crate::tags::{self, Display};
use crate::tree::{Dom, NodeRef};

/// An element in the semantic page outline.
///
/// ```
/// let dom = domq::parse("<nav><a href='/'>Home</a></nav>");
/// let outline = dom.outline();
/// let link = &outline.nodes()[1];
/// assert_eq!((link.role.as_str(), link.name.as_deref()), ("link", Some("Home")));
/// assert_eq!(link.href.as_deref(), Some("/"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OutlineNode {
    /// 1-based index reference assigned in document outline order.
    pub ref_id: usize,
    /// Semantic role (e.g. `"banner"`, `"navigation"`, `"link"`, `"button"`, `"heading"`, `"textbox"`).
    pub role: String,
    /// Accessible name, if available (e.g. from `aria-label`, `<label>`, inner text, `alt`, `title`).
    pub name: Option<String>,
    /// Heading level (1..=6) for `role == "heading"`.
    pub level: Option<u8>,
    /// Minimal, unique CSS selector that resolves back to this element.
    pub selector: String,
    /// Target URL for links (`href`), if present.
    pub href: Option<String>,
    /// HTML tag name (e.g. `"a"`, `"button"`, `"h1"`, `"input"`).
    pub tag: String,
    /// Tree depth in the outline hierarchy (starting at 0 for top-level landmarks).
    pub depth: usize,
    /// Byte range in the original HTML source, if known.
    pub source_range: Option<(usize, usize)>,
}

impl OutlineNode {
    /// Render this outline node as a JSON object string.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\"ref\":");
        push_usize(&mut out, self.ref_id);
        out.push_str(",\"role\":");
        write_str(&mut out, &self.role);
        out.push_str(",\"name\":");
        match &self.name {
            Some(name) => write_str(&mut out, name),
            None => out.push_str("null"),
        }
        out.push_str(",\"level\":");
        match self.level {
            Some(lvl) => push_usize(&mut out, lvl as usize),
            None => out.push_str("null"),
        }
        out.push_str(",\"selector\":");
        write_str(&mut out, &self.selector);
        out.push_str(",\"href\":");
        match &self.href {
            Some(href) => write_str(&mut out, href),
            None => out.push_str("null"),
        }
        out.push_str(",\"tag\":");
        write_str(&mut out, &self.tag);
        out.push_str(",\"depth\":");
        push_usize(&mut out, self.depth);
        out.push_str(",\"source_range\":");
        match self.source_range {
            Some((start, end)) => {
                out.push('[');
                push_usize(&mut out, start);
                out.push(',');
                push_usize(&mut out, end);
                out.push(']');
            }
            None => out.push_str("null"),
        }
        out.push('}');
        out
    }
}

/// The semantic page outline of landmarks, headings, interactive controls, and forms.
///
/// ```
/// let dom = domq::parse("<main><h1>Title</h1><button>Send</button></main>");
/// let outline = dom.outline();
/// assert_eq!(outline.len(), 3);
/// assert!(!outline.is_empty());
/// assert_eq!(
///     outline.to_text(),
///     "[1] main\n[2]   heading 1 \"Title\"\n[3]   button \"Send\" -> main > button",
/// );
/// assert!(outline.to_json().starts_with("[{\"ref\":1,\"role\":\"main\""));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PageOutline {
    /// Outline items in document order.
    pub nodes: Vec<OutlineNode>,
}

impl PageOutline {
    /// Number of items in the outline.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the outline contains no items.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Slice of outline nodes.
    pub fn nodes(&self) -> &[OutlineNode] {
        &self.nodes
    }

    /// Render as a compact indented text outline for model prompts.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for node in &self.nodes {
            if !out.is_empty() {
                out.push('\n');
            }
            // [ref]
            out.push('[');
            push_usize(&mut out, node.ref_id);
            out.push(']');

            // 2 spaces per depth level
            out.push(' ');
            for _ in 0..node.depth {
                out.push_str("  ");
            }

            let is_control = matches!(
                node.role.as_str(),
                "button"
                    | "link"
                    | "textbox"
                    | "searchbox"
                    | "checkbox"
                    | "radio"
                    | "combobox"
                    | "spinbutton"
                    | "slider"
                    | "switch"
            );

            if let Some(level) = node.level {
                out.push_str("heading ");
                push_usize(&mut out, level as usize);
                if let Some(name) = &node.name {
                    out.push_str(" \"");
                    out.push_str(name);
                    out.push('"');
                }
            } else if is_control {
                out.push_str(&node.role);
                if let Some(name) = &node.name {
                    out.push_str(" \"");
                    out.push_str(name);
                    out.push('"');
                }
                out.push_str(" -> ");
                out.push_str(&node.selector);
            } else {
                // Landmark / form / container
                out.push_str(&node.role);
                if node.selector.starts_with('#') {
                    out.push_str(&node.selector);
                }
                if let Some(name) = &node.name {
                    out.push_str(" \"");
                    out.push_str(name);
                    out.push('"');
                }
            }
        }
        out
    }

    /// Render as a JSON array string.
    pub fn to_json(&self) -> String {
        let mut out = String::from("[");
        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&node.to_json());
        }
        out.push(']');
        out
    }
}

impl core::fmt::Display for PageOutline {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.to_text())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for OutlineNode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("OutlineNode", 9)?;
        st.serialize_field("ref", &self.ref_id)?;
        st.serialize_field("role", &self.role)?;
        st.serialize_field("name", &self.name)?;
        st.serialize_field("level", &self.level)?;
        st.serialize_field("selector", &self.selector)?;
        st.serialize_field("href", &self.href)?;
        st.serialize_field("tag", &self.tag)?;
        st.serialize_field("depth", &self.depth)?;
        st.serialize_field("source_range", &self.source_range)?;
        st.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PageOutline {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.nodes.serialize(s)
    }
}

impl Dom {
    /// Generate a semantic page outline of landmarks, headings, controls, and forms.
    ///
    /// ```
    /// let dom = domq::parse(r#"
    ///   <header><nav><a href="/">Home</a></nav></header>
    ///   <main>
    ///     <h1>Welcome</h1>
    ///     <form id="search"><input type="search" name="q"><button>Go</button></form>
    ///   </main>
    /// "#);
    /// let outline = dom.outline();
    /// assert_eq!(
    ///     outline.to_text(),
    ///     "[1] banner\n\
    ///      [2]   navigation\n\
    ///      [3]     link \"Home\" -> header > nav > a\n\
    ///      [4] main\n\
    ///      [5]   heading 1 \"Welcome\"\n\
    ///      [6]   form#search\n\
    ///      [7]     searchbox -> input[name=\"q\"]\n\
    ///      [8]     button \"Go\" -> #search > button"
    /// );
    /// ```
    pub fn outline(&self) -> PageOutline {
        let mut nodes = Vec::new();
        let mut container_stack: Vec<NodeId> = Vec::new();

        for root in self.roots() {
            extract_outline_tree(root, self, &mut nodes, &mut container_stack);
        }

        PageOutline { nodes }
    }
}

impl NodeRef<'_> {
    /// Generate a semantic outline for this node and its subtree.
    pub fn outline(&self) -> PageOutline {
        let mut nodes = Vec::new();
        let mut container_stack: Vec<NodeId> = Vec::new();

        extract_outline_tree(*self, self.dom(), &mut nodes, &mut container_stack);

        PageOutline { nodes }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElementRole<'a> {
    Landmark(&'a str),
    Heading(u8),
    Control(&'a str),
    Form,
    None,
}

fn classify_element<'a>(node: NodeRef<'a>) -> ElementRole<'a> {
    if let Some(r) = node.attr("role") {
        let first_token = r.split(is_space).find(|t| !t.is_empty()).unwrap_or("");
        match first_token.to_ascii_lowercase().as_str() {
            "none" | "presentation" => return ElementRole::None,
            "banner" | "navigation" | "main" | "complementary" | "contentinfo" | "region"
            | "article" | "dialog" | "search" | "feed" => {
                return ElementRole::Landmark(first_token);
            }
            "button" | "link" | "checkbox" | "radio" | "textbox" | "searchbox" | "combobox"
            | "spinbutton" | "slider" | "switch" | "tab" | "tabpanel" | "menuitem" => {
                return ElementRole::Control(first_token);
            }
            "form" => return ElementRole::Form,
            "heading" => {
                let level = node
                    .attr("aria-level")
                    .and_then(|l| l.trim().parse::<u8>().ok())
                    .filter(|&l| (1..=6).contains(&l))
                    .unwrap_or(2);
                return ElementRole::Heading(level);
            }
            _ => {}
        }
    }

    let tag = node.tag_name().unwrap_or("");
    match tag {
        "header" => {
            if in_sectioning_content(node) {
                ElementRole::Landmark("header")
            } else {
                ElementRole::Landmark("banner")
            }
        }
        "footer" => {
            if in_sectioning_content(node) {
                ElementRole::Landmark("footer")
            } else {
                ElementRole::Landmark("contentinfo")
            }
        }
        "main" => ElementRole::Landmark("main"),
        "nav" => ElementRole::Landmark("navigation"),
        "aside" => ElementRole::Landmark("complementary"),
        "article" => ElementRole::Landmark("article"),
        "section" => ElementRole::Landmark("region"),
        "dialog" => ElementRole::Landmark("dialog"),
        "form" => ElementRole::Form,
        "h1" => ElementRole::Heading(1),
        "h2" => ElementRole::Heading(2),
        "h3" => ElementRole::Heading(3),
        "h4" => ElementRole::Heading(4),
        "h5" => ElementRole::Heading(5),
        "h6" => ElementRole::Heading(6),
        "button" => ElementRole::Control("button"),
        "summary" => ElementRole::Control("button"),
        "select" => ElementRole::Control("combobox"),
        "textarea" => ElementRole::Control("textbox"),
        "a" | "area" => {
            if node.has_attr("href") {
                ElementRole::Control("link")
            } else {
                ElementRole::None
            }
        }
        "input" => {
            let input_type = node.attr("type").map(str::trim).unwrap_or("text");
            let input_type_lower = input_type.to_ascii_lowercase();
            match input_type_lower.as_str() {
                "hidden" => ElementRole::None,
                "button" | "submit" | "reset" | "image" => ElementRole::Control("button"),
                "checkbox" => ElementRole::Control("checkbox"),
                "radio" => ElementRole::Control("radio"),
                "search" => ElementRole::Control("searchbox"),
                "number" => ElementRole::Control("spinbutton"),
                "range" => ElementRole::Control("slider"),
                _ => ElementRole::Control("textbox"),
            }
        }
        _ => ElementRole::None,
    }
}

fn in_sectioning_content(node: NodeRef<'_>) -> bool {
    let mut cur = node.parent();
    while let Some(p) = cur {
        if matches!(
            p.tag_name(),
            Some("article" | "aside" | "main" | "nav" | "section")
        ) {
            return true;
        }
        cur = p.parent();
    }
    false
}

fn is_container_landmark(role: &ElementRole<'_>) -> bool {
    matches!(role, ElementRole::Landmark(_) | ElementRole::Form)
}

fn is_hidden_element(node: NodeRef<'_>) -> bool {
    let tag = node.tag_name().unwrap_or("");
    (tags::display(tag) == Display::Hidden
        && !(tag == "noscript" && node.dom().noscript_is_content))
        || node.hides_itself()
        || node
            .attr("aria-hidden")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

fn extract_outline_tree(
    root: NodeRef<'_>,
    dom: &Dom,
    nodes: &mut Vec<OutlineNode>,
    container_stack: &mut Vec<NodeId>,
) {
    walk(root, |node, enter| {
        if !node.is_element() {
            return false;
        }

        if is_hidden_element(node) {
            return false;
        }

        let role = classify_element(node);
        let is_container = is_container_landmark(&role);

        if enter {
            if role != ElementRole::None {
                let depth = container_stack.len();
                let ref_id = nodes.len() + 1;
                let (role_str, level) = match role {
                    ElementRole::Landmark(r) => (r.to_string(), None),
                    ElementRole::Heading(lvl) => ("heading".to_string(), Some(lvl)),
                    ElementRole::Control(c) => (c.to_string(), None),
                    ElementRole::Form => ("form".to_string(), None),
                    ElementRole::None => unreachable!(),
                };

                let name = compute_accessible_name(node, dom);
                let selector = synthesize_selector(node, dom);
                let href = if matches!(role, ElementRole::Control("link")) {
                    node.attr("href").map(String::from)
                } else {
                    None
                };
                let tag = node.tag_name().unwrap_or("").to_string();
                let source_range = node.source_range().map(|r| (r.start, r.end));

                nodes.push(OutlineNode {
                    ref_id,
                    role: role_str,
                    name,
                    level,
                    selector,
                    href,
                    tag,
                    depth,
                    source_range,
                });
            }

            if is_container {
                container_stack.push(node.id());
            }
        } else if is_container && container_stack.last() == Some(&node.id()) {
            container_stack.pop();
        }

        true
    });
}

fn compute_accessible_name(node: NodeRef<'_>, dom: &Dom) -> Option<String> {
    // 1. aria-labelledby
    if let Some(ids) = node.attr("aria-labelledby") {
        let mut parts = Vec::new();
        for id in ids.split(is_space).filter(|s| !s.is_empty()) {
            if let Some(target) = dom.find_by_id(id) {
                let t = collapse(&target.to_text());
                if !t.is_empty() {
                    parts.push(t);
                }
            }
        }
        if !parts.is_empty() {
            return Some(truncate_name(parts.join(" ")));
        }
    }

    // 2. aria-label
    if let Some(label) = node.attr("aria-label") {
        let c = collapse(label);
        if !c.is_empty() {
            return Some(truncate_name(c));
        }
    }

    let tag = node.tag_name().unwrap_or("");

    // 3. Form controls (<input>, <select>, <textarea>)
    if matches!(tag, "input" | "select" | "textarea") {
        // Associated label by id
        if let Some(id) = node.attr("id") {
            for label in dom.find_by_tag("label") {
                if label.attr("for") == Some(id) {
                    let t = collapse(&label.to_text());
                    if !t.is_empty() {
                        return Some(truncate_name(t));
                    }
                }
            }
        }

        // Ancestor label
        let mut cur = node.parent();
        while let Some(p) = cur {
            if p.tag_name() == Some("label") {
                let t = collapse(&p.to_text());
                if !t.is_empty() {
                    return Some(truncate_name(t));
                }
                break;
            }
            cur = p.parent();
        }

        // Placeholder
        if let Some(ph) = node.attr("placeholder") {
            let c = collapse(ph);
            if !c.is_empty() {
                return Some(truncate_name(c));
            }
        }

        // Value attribute for buttons
        if let Some(val) = node.attr("value") {
            let c = collapse(val);
            if !c.is_empty() {
                return Some(truncate_name(c));
            }
        } else if node
            .attr("type")
            .map(str::trim)
            .is_some_and(|t| t.eq_ignore_ascii_case("submit"))
        {
            return Some("Submit".to_string());
        }

        // Alt attribute for image inputs
        if let Some(alt) = node.attr("alt") {
            let c = collapse(alt);
            if !c.is_empty() {
                return Some(truncate_name(c));
            }
        }
    }

    // 4. Inner readable text (headings, buttons, links, summaries)
    let is_text_element = matches!(
        tag,
        "a" | "button" | "summary" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
    ) || node.attr("role").is_some_and(|r| {
        matches!(
            r.trim().to_ascii_lowercase().as_str(),
            "heading" | "button" | "link" | "tab" | "menuitem"
        )
    });
    if is_text_element {
        let t = collapse(&node.to_text());
        if !t.is_empty() {
            return Some(truncate_name(t));
        }

        // Image with alt inside link/button
        for desc in node.descendants() {
            if desc.tag_name() == Some("img")
                && let Some(alt) = desc.attr("alt")
            {
                let c = collapse(alt);
                if !c.is_empty() {
                    return Some(truncate_name(c));
                }
            }
        }
    }

    // 5. Fallback title attribute
    if let Some(title) = node.attr("title") {
        let c = collapse(title);
        if !c.is_empty() {
            return Some(truncate_name(c));
        }
    }

    None
}

fn truncate_name(mut s: String) -> String {
    const MAX_LEN: usize = 100;
    if s.chars().count() > MAX_LEN {
        let cut = s
            .char_indices()
            .nth(MAX_LEN)
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        s.truncate(cut);
        s.push('…');
    }
    s
}

fn synthesize_selector(node: NodeRef<'_>, dom: &Dom) -> String {
    let tag = node.tag_name().unwrap_or("");

    // 1. Unique ID
    if let Some(id) = node.attr("id") {
        let id = id.trim();
        if !id.is_empty() && is_unique_id(dom, id) {
            if is_clean_css_ident(id) {
                return format!("#{id}");
            } else {
                let mut out = String::from("[id=\"");
                escape_css_attr_val(&mut out, id);
                out.push_str("\"]");
                return out;
            }
        }
    }

    // 2. Unique data-testid / data-test-id
    for test_attr in &["data-testid", "data-test-id"] {
        if let Some(tid) = node.attr(test_attr) {
            let tid = tid.trim();
            if !tid.is_empty() && is_unique_attr(dom, test_attr, tid) {
                let mut out = format!("[{test_attr}=\"");
                escape_css_attr_val(&mut out, tid);
                out.push_str("\"]");
                return out;
            }
        }
    }

    // 3. Unique name attribute (e.g. input[name="q"])
    if let Some(name) = node.attr("name") {
        let name = name.trim();
        if !name.is_empty() && is_unique_tag_attr(dom, tag, "name", name) {
            let mut out = format!("{tag}[name=\"");
            escape_css_attr_val(&mut out, name);
            out.push_str("\"]");
            return out;
        }
    }

    // 4. Unique type attribute for search / submit / reset
    if matches!(tag, "input" | "button")
        && let Some(ty) = node.attr("type")
    {
        let ty = ty.trim();
        if matches!(ty, "search" | "submit" | "reset") && is_unique_tag_attr(dom, tag, "type", ty) {
            return format!("{tag}[type=\"{ty}\"]");
        }
    }

    // 5. Unique class name or tag.class
    if let Some(cls) = node.attr("class") {
        for c in cls.split(is_space).filter(|s| !s.is_empty()) {
            if is_clean_css_ident(c) {
                if is_unique_class(dom, c) {
                    return format!(".{c}");
                }
                if is_unique_tag_class(dom, tag, c) {
                    return format!("{tag}.{c}");
                }
            }
        }
    }

    // 6. Structural hierarchy path
    build_path_selector(node, dom)
}

fn build_path_selector(node: NodeRef<'_>, dom: &Dom) -> String {
    let mut segments = Vec::new();
    let mut cur = Some(node);

    while let Some(el) = cur {
        let tag = el.tag_name().unwrap_or("");
        if tag.is_empty() || tag == "html" || tag == "body" {
            // Stop at root containers
            if segments.is_empty() {
                segments.push(tag.to_string());
            }
            break;
        }

        // If this ancestor has a clean unique ID, stop path here
        if let Some(id) = el.attr("id") {
            let id = id.trim();
            if is_clean_css_ident(id) && is_unique_id(dom, id) {
                segments.push(format!("#{id}"));
                break;
            }
        }

        // Count index among same-tag siblings under parent
        let mut index = 1;
        let mut same_tag_count = 0;
        if let Some(parent) = el.parent() {
            for sibling in parent.child_elements() {
                if sibling.tag_name() == el.tag_name() {
                    same_tag_count += 1;
                    if sibling.id() == el.id() {
                        index = same_tag_count;
                    }
                }
            }
        }

        let segment = if same_tag_count > 1 {
            format!("{tag}:nth-of-type({index})")
        } else {
            tag.to_string()
        };

        segments.push(segment);
        cur = el.parent();
    }

    segments.reverse();
    segments.join(" > ")
}

fn is_unique_id(dom: &Dom, id: &str) -> bool {
    let mut count = 0;
    for n in dom.nodes() {
        if n.attr("id") == Some(id) {
            count += 1;
            if count > 1 {
                return false;
            }
        }
    }
    count == 1
}

fn is_unique_attr(dom: &Dom, attr: &str, val: &str) -> bool {
    let mut count = 0;
    for n in dom.nodes() {
        if n.attr(attr) == Some(val) {
            count += 1;
            if count > 1 {
                return false;
            }
        }
    }
    count == 1
}

fn is_unique_tag_attr(dom: &Dom, tag: &str, attr: &str, val: &str) -> bool {
    let mut count = 0;
    for n in dom.nodes() {
        if n.tag_name() == Some(tag) && n.attr(attr) == Some(val) {
            count += 1;
            if count > 1 {
                return false;
            }
        }
    }
    count == 1
}

fn is_unique_class(dom: &Dom, class: &str) -> bool {
    let mut count = 0;
    for n in dom.nodes() {
        if n.has_class(class) {
            count += 1;
            if count > 1 {
                return false;
            }
        }
    }
    count == 1
}

fn is_unique_tag_class(dom: &Dom, tag: &str, class: &str) -> bool {
    let mut count = 0;
    for n in dom.nodes() {
        if n.tag_name() == Some(tag) && n.has_class(class) {
            count += 1;
            if count > 1 {
                return false;
            }
        }
    }
    count == 1
}

fn is_clean_css_ident(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first.is_ascii_digit() {
        return false;
    }
    if first == '-' {
        if let Some(second) = chars.next() {
            if second.is_ascii_digit() || second == '-' {
                return false;
            }
        } else {
            return false;
        }
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn escape_css_attr_val(out: &mut String, val: &str) {
    for c in val.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
}

fn push_usize(out: &mut String, mut n: usize) {
    if n == 0 {
        out.push('0');
        return;
    }
    let mut buf = [0u8; 20];
    let mut i = 0;
    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        out.push(buf[i] as char);
    }
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str("\\u");
                const HEX: [u8; 16] = *b"0123456789abcdef";
                for shift in [12u32, 8, 4, 0] {
                    let nib = (((c as u32) >> shift) & 0xf) as usize;
                    out.push(HEX[nib] as char);
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}
