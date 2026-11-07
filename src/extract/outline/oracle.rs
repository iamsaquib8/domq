//! The outline algorithm as it was before it became linear (#90), kept as the
//! oracle for the differential test in `super::tests`: quadratic, but simple
//! enough to trust. Do not "improve" this file.

use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use super::super::{collapse, is_space};
use super::{OutlineNode, PageOutline};
use crate::NodeId;
use crate::serialize::walk;
use crate::tags::{self, Display};
use crate::tree::{Dom, NodeRef};

pub(super) fn outline(dom: &Dom) -> PageOutline {
    let mut nodes = Vec::new();
    let mut container_stack: Vec<NodeId> = Vec::new();
    for root in dom.roots() {
        extract_outline_tree(root, dom, &mut nodes, &mut container_stack);
    }
    PageOutline { nodes }
}

pub(super) fn node_outline(node: NodeRef<'_>) -> PageOutline {
    let mut nodes = Vec::new();
    let mut container_stack: Vec<NodeId> = Vec::new();
    extract_outline_tree(node, node.dom(), &mut nodes, &mut container_stack);
    PageOutline { nodes }
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
                    ElementRole::None => (String::new(), None), // (not reached: checked above)
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
