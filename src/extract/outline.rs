//! Semantic page outline for browser-free AI models ([`Dom::outline`]).
//!
//! Extracts structural landmarks (`banner`, `main`, `navigation`, `aside`,
//! `contentinfo`, `region`, `article`, `form`, `dialog`), headings (`h1`–`h6`),
//! and interactive controls (`button`, `link`, `textbox`, `searchbox`,
//! `combobox`, `checkbox`, `radio`), with accessible names and minimal unique
//! CSS selectors.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec;
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
        let mut cx = Cx::new(self);

        for root in self.roots() {
            extract_outline_tree(root, &mut cx, &mut nodes, &mut container_stack);
        }

        PageOutline { nodes }
    }
}

impl<'a> NodeRef<'a> {
    /// Generate a semantic outline for this node and its subtree.
    pub fn outline(&self) -> PageOutline {
        let mut nodes = Vec::new();
        let mut container_stack: Vec<NodeId> = Vec::new();
        let mut cx = Cx::new(self.dom());

        extract_outline_tree(*self, &mut cx, &mut nodes, &mut container_stack);

        PageOutline { nodes }
    }
}

/// Longest accessible name, in characters; longer ones are cut and end in `…`.
const MAX_NAME_LEN: usize = 100;

/// Something an element can be looked up by, to tell whether it is the only
/// one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Key<'a> {
    Id(&'a str),
    /// An attribute that names an element for tests: its name and value.
    Attr(&'a str, &'a str),
    /// A tag with an attribute: tag, attribute name, value.
    TagAttr(&'a str, &'a str, &'a str),
    Class(&'a str),
    TagClass(&'a str, &'a str),
}

/// What the outline asks of the whole document, answered from one pass over it
/// so that no outline node has to scan the document (or its siblings, or its
/// ancestors) again (#90).
struct Cx<'a> {
    dom: &'a Dom,
    /// How many elements match each key, and the first one in document order.
    counts: BTreeMap<Key<'a>, (usize, NodeId)>,
    /// The `<label>`s with a `for` attribute, by its value, in document order.
    labels_for: BTreeMap<&'a str, Vec<NodeId>>,
    /// Per node (by arena index): its position among the siblings of its tag,
    /// from 1, and how many there are. `(1, 0)` for a top-level node.
    siblings: Vec<(u32, u32)>,
    /// Names already worked out: the head of an element's text, by node, and
    /// the name a `for` value gives, so that many controls pointing at one
    /// label or one big element read it once.
    texts: BTreeMap<usize, String>,
    label_names: BTreeMap<&'a str, Option<String>>,
    /// Open elements of the walk, kept as it goes: how many are sectioning
    /// content (plus those above where it started), and the `<label>`s.
    sectioning: usize,
    open_labels: Vec<NodeId>,
}

impl<'a> Cx<'a> {
    fn new(dom: &'a Dom) -> Self {
        let mut cx = Cx {
            dom,
            counts: BTreeMap::new(),
            labels_for: BTreeMap::new(),
            siblings: vec![(1, 0); dom.nodes.len()],
            texts: BTreeMap::new(),
            label_names: BTreeMap::new(),
            sectioning: 0,
            open_labels: Vec::new(),
        };
        let mut classes: Vec<&str> = Vec::new();
        let mut tags: BTreeMap<&str, u32> = BTreeMap::new();
        for node in dom.nodes() {
            for child in node.child_elements() {
                let seen = tags.entry(child.tag_name().unwrap_or("")).or_insert(0);
                *seen += 1;
                if let Some(slot) = cx.siblings.get_mut(child.id().index()) {
                    slot.0 = *seen;
                }
            }
            for child in node.child_elements() {
                let total = tags.get(child.tag_name().unwrap_or("")).copied();
                if let Some(slot) = cx.siblings.get_mut(child.id().index()) {
                    slot.1 = total.unwrap_or(0);
                }
            }
            tags.clear();

            let Some(tag) = node.tag_name() else {
                continue;
            };
            if let Some(id) = node.attr("id") {
                cx.count(Key::Id(id), node);
            }
            for name in ["data-testid", "data-test-id"] {
                if let Some(v) = node.attr(name) {
                    cx.count(Key::Attr(name, v), node);
                }
            }
            for name in ["name", "type"] {
                if let Some(v) = node.attr(name) {
                    cx.count(Key::TagAttr(tag, name, v), node);
                }
            }
            // An element counts once for a class it names twice.
            classes.clear();
            classes.extend(node.classes());
            classes.sort_unstable();
            classes.dedup();
            for &c in &classes {
                cx.count(Key::Class(c), node);
                cx.count(Key::TagClass(tag, c), node);
            }
            if tag.eq_ignore_ascii_case("label")
                && let Some(target) = node.attr("for")
            {
                cx.labels_for.entry(target).or_default().push(node.id());
            }
        }
        cx
    }

    fn count(&mut self, key: Key<'a>, node: NodeRef<'a>) {
        self.counts.entry(key).or_insert((0, node.id())).0 += 1;
    }

    /// Whether exactly one element has `key`.
    fn unique(&self, key: Key<'_>) -> bool {
        self.counts.get(&key).is_some_and(|&(n, _)| n == 1)
    }

    /// The text of `node` as far as a name can use it: collapsed, and cut at
    /// one character more than the longest name, so that cutting it to a name
    /// gives what the whole text would.
    fn text_head(&mut self, node: NodeRef<'a>) -> String {
        if let Some(text) = self.texts.get(&node.id().index()) {
            return text.clone();
        }
        let text = head_text(node);
        self.texts.insert(node.id().index(), text.clone());
        text
    }

    /// The name the first `<label for=id>` with text gives.
    fn label_for(&mut self, id: &'a str) -> Option<String> {
        if let Some(name) = self.label_names.get(id) {
            return name.clone();
        }
        let labels = self.labels_for.get(id).cloned().unwrap_or_default();
        let mut name = None;
        for label in labels {
            let Some(label) = self.dom.get(label) else {
                continue;
            };
            let text = self.text_head(label);
            if !text.is_empty() {
                name = Some(truncate_name(text));
                break;
            }
        }
        self.label_names.insert(id, name.clone());
        name
    }
}

/// `collapse(node.to_text())`, cut after `MAX_NAME_LEN + 1` characters.
fn head_text(node: NodeRef<'_>) -> String {
    let mut text = collapse(&node.text_head(MAX_NAME_LEN + 1));
    if let Some((cut, _)) = text.char_indices().nth(MAX_NAME_LEN + 1) {
        text.truncate(cut);
    }
    text
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElementRole<'a> {
    Landmark(&'a str),
    Heading(u8),
    Control(&'a str),
    Form,
    None,
}

/// `sectioned`: the element is inside `article`, `aside`, `main`, `nav` or
/// `section`, which makes a `header` or `footer` a plain one, not a landmark.
fn classify_element<'a>(node: NodeRef<'a>, sectioned: bool) -> ElementRole<'a> {
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
            if sectioned {
                ElementRole::Landmark("header")
            } else {
                ElementRole::Landmark("banner")
            }
        }
        "footer" => {
            if sectioned {
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
    node.ancestors().any(|p| is_sectioning(p.tag_name()))
}

fn is_sectioning(tag: Option<&str>) -> bool {
    matches!(tag, Some("article" | "aside" | "main" | "nav" | "section"))
}

fn is_container_landmark(role: &ElementRole<'_>) -> bool {
    matches!(role, ElementRole::Landmark(_) | ElementRole::Form)
}

fn is_hidden_element(node: NodeRef<'_>) -> bool {
    let tag = node.tag_name().unwrap_or("");
    (tags::display(tag) == Display::Hidden
        && !(tag == "noscript" && node.dom().noscript_is_content()))
        || node.hides_itself()
        || node
            .attr("aria-hidden")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

fn extract_outline_tree<'a>(
    root: NodeRef<'a>,
    cx: &mut Cx<'a>,
    nodes: &mut Vec<OutlineNode>,
    container_stack: &mut Vec<NodeId>,
) {
    // What is open above `root` counts too.
    cx.sectioning = usize::from(in_sectioning_content(root));
    cx.open_labels.clear();
    cx.open_labels.extend(
        root.ancestors()
            .find(|p| p.tag_name() == Some("label"))
            .map(|p| p.id()),
    );

    walk(root, |node, enter| {
        if !node.is_element() {
            return false;
        }

        if is_hidden_element(node) {
            return false;
        }

        let role = classify_element(node, cx.sectioning > 0);
        let is_container = is_container_landmark(&role);
        let tag = node.tag_name();

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

                let name = compute_accessible_name(node, cx);
                let selector = synthesize_selector(node, cx);
                let href = if matches!(role, ElementRole::Control("link")) {
                    node.attr("href").map(String::from)
                } else {
                    None
                };
                let tag = tag.unwrap_or("").to_string();
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
            // Open for the element's descendants (see `Cx`).
            if is_sectioning(tag) {
                cx.sectioning += 1;
            }
            if tag == Some("label") {
                cx.open_labels.push(node.id());
            }
        } else {
            if is_container && container_stack.last() == Some(&node.id()) {
                container_stack.pop();
            }
            if is_sectioning(tag) {
                cx.sectioning = cx.sectioning.saturating_sub(1);
            }
            if tag == Some("label") {
                cx.open_labels.pop();
            }
        }

        true
    });
}

fn compute_accessible_name<'a>(node: NodeRef<'a>, cx: &mut Cx<'a>) -> Option<String> {
    // 1. aria-labelledby
    if let Some(ids) = node.attr("aria-labelledby") {
        let mut parts = Vec::new();
        for id in ids.split(is_space).filter(|s| !s.is_empty()) {
            if let Some(&(_, first)) = cx.counts.get(&Key::Id(id))
                && let Some(target) = cx.dom.get(first)
            {
                let t = cx.text_head(target);
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
        if let Some(id) = node.attr("id")
            && let Some(name) = cx.label_for(id)
        {
            return Some(name);
        }

        // Ancestor label
        if let Some(label) = cx.open_labels.last().and_then(|&id| cx.dom.get(id)) {
            let t = cx.text_head(label);
            if !t.is_empty() {
                return Some(truncate_name(t));
            }
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
        let t = head_text(node);
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
    const MAX_LEN: usize = MAX_NAME_LEN;
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

fn synthesize_selector(node: NodeRef<'_>, cx: &Cx<'_>) -> String {
    let tag = node.tag_name().unwrap_or("");

    // 1. Unique ID
    if let Some(id) = node.attr("id") {
        let id = id.trim();
        if !id.is_empty() && cx.unique(Key::Id(id)) {
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
            if !tid.is_empty() && cx.unique(Key::Attr(test_attr, tid)) {
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
        if !name.is_empty() && cx.unique(Key::TagAttr(tag, "name", name)) {
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
        if matches!(ty, "search" | "submit" | "reset") && cx.unique(Key::TagAttr(tag, "type", ty)) {
            return format!("{tag}[type=\"{ty}\"]");
        }
    }

    // 5. Unique class name or tag.class
    if let Some(cls) = node.attr("class") {
        for c in cls.split(is_space).filter(|s| !s.is_empty()) {
            if is_clean_css_ident(c) {
                if cx.unique(Key::Class(c)) {
                    return format!(".{c}");
                }
                if cx.unique(Key::TagClass(tag, c)) {
                    return format!("{tag}.{c}");
                }
            }
        }
    }

    // 6. Structural hierarchy path
    build_path_selector(node, cx)
}

/// One step of a path selector, from the element up.
enum Step<'a> {
    Id(&'a str),
    Tag(&'a str),
    Nth(&'a str, u32),
}

fn build_path_selector(node: NodeRef<'_>, cx: &Cx<'_>) -> String {
    let mut steps = Vec::new();
    let mut cur = Some(node);

    while let Some(el) = cur {
        let tag = el.tag_name().unwrap_or("");
        if tag.is_empty() || tag == "html" || tag == "body" {
            // Stop at root containers
            if steps.is_empty() {
                steps.push(Step::Tag(tag));
            }
            break;
        }

        // If this ancestor has a clean unique ID, stop path here
        if let Some(id) = el.attr("id") {
            let id = id.trim();
            if is_clean_css_ident(id) && cx.unique(Key::Id(id)) {
                steps.push(Step::Id(id));
                break;
            }
        }

        // Position among same-tag siblings under the parent
        let (index, same_tag_count) = cx.siblings.get(el.id().index()).copied().unwrap_or((1, 0));

        steps.push(if same_tag_count > 1 {
            Step::Nth(tag, index)
        } else {
            Step::Tag(tag)
        });
        cur = el.parent();
    }

    // The path can be as long as the tree is deep: write it in one string.
    let mut out = String::new();
    for (i, step) in steps.iter().rev().enumerate() {
        if i > 0 {
            out.push_str(" > ");
        }
        match *step {
            Step::Id(id) => {
                out.push('#');
                out.push_str(id);
            }
            Step::Tag(tag) => out.push_str(tag),
            Step::Nth(tag, index) => {
                out.push_str(tag);
                out.push_str(":nth-of-type(");
                push_usize(&mut out, index as usize);
                out.push(')');
            }
        }
    }
    out
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

#[cfg(test)]
mod oracle;
#[cfg(test)]
mod tests;
