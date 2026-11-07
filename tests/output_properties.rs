//! Property and robustness tests for the content serializers: `to_text`,
//! `to_markdown` and `inner_html` must never panic, keep their whitespace
//! promises, and must not overflow the stack on deeply nested input.

use domq::{Dom, parse};
use proptest::prelude::*;

/// Markup soup built from fragments that exercise the serializers' structure
/// (lists, quotes, tables, code, inline marks, links, hidden elements) plus
/// Markdown-significant characters and whitespace.
fn html_soup() -> impl Strategy<Value = String> {
    let pieces = vec![
        "<p>",
        "</p>",
        "<div>",
        "</div>",
        "<ul>",
        "<ol start=9>",
        "</ul>",
        "</ol>",
        "<li>",
        "</li>",
        "<blockquote>",
        "</blockquote>",
        "<table>",
        "</table>",
        "<tr>",
        "<td>",
        "<th>",
        "</td>",
        "<thead>",
        "<caption>",
        "<pre>",
        "</pre>",
        "<code>",
        "</code>",
        "<b>",
        "</b>",
        "<i>",
        "</i>",
        "<del>",
        "<a href='/x y'>",
        "<a href=u>",
        "<a href='ééé'>",
        "<a href='data:text/plain,hi'>",
        "</a>",
        "<img src=i alt='a*'>",
        "<img src='ééé' alt='m'>",
        "<img src='data:image/png;base64,xx' alt='d'>",
        "<br>",
        "<hr>",
        "<h1>",
        "</h1>",
        "<h3>",
        "<dl>",
        "<dt>",
        "<dd>",
        "<script>",
        "</script>",
        "<style>x</style>",
        "<textarea>",
        "<!-- c -->",
        "&amp;",
        "&#150;",
        "&nbsp;",
        "text",
        "1.",
        "1",
        ".",
        ")",
        "<span>",
        "</span>",
        "<!---->",
        " ",
        "\n",
        "\t",
        "#",
        "-",
        "*",
        "_",
        "`",
        "|",
        "[",
        "]",
        ">",
        "<",
        "\\",
        "~",
        "&",
        "\r",
    ];
    proptest::collection::vec(proptest::sample::select(pieces), 0..80).prop_map(|v| v.concat())
}

/// Raw characters, as in `tests/properties.rs`.
fn char_soup() -> impl Strategy<Value = String> {
    let alphabet = vec![
        '<', '>', '/', '=', '"', '\'', ' ', '\n', '!', '-', '?', '&', ';', '#', '[', ']', 'a', 'b',
        'p', 'l', 'i', '1', '\u{0}', '\u{a0}', '🦀',
    ];
    proptest::collection::vec(proptest::sample::select(alphabet), 0..300)
        .prop_map(|v| v.into_iter().collect())
}

/// Serialize every node (and the document) every way; return the document's
/// text and Markdown.
fn serialize_all(dom: &Dom) -> (String, String) {
    for node in dom.nodes() {
        let _ = node.inner_html();
        let _ = node.to_text();
        #[cfg(feature = "markdown")]
        let _ = node.to_markdown();
    }
    #[cfg(feature = "markdown")]
    {
        use domq::{ImageStyle, LinkStyle, MarkdownOptions};
        // Exercise option combinations, including drop_data_uris on non-ASCII
        // and data: URLs from the soup generator (#88).
        for drop in [false, true] {
            for link in [LinkStyle::Inline, LinkStyle::Reference, LinkStyle::Footer] {
                for image in [ImageStyle::Keep, ImageStyle::AltOnly, ImageStyle::Drop] {
                    let opts = MarkdownOptions::default()
                        .drop_data_uris(drop)
                        .link_style(link)
                        .image_style(image);
                    let _ = dom.to_markdown_with(&opts);
                }
            }
        }
    }
    #[cfg(feature = "markdown")]
    let md = dom.to_markdown();
    #[cfg(not(feature = "markdown"))]
    let md = String::new();
    (dom.to_text(), md)
}

fn trimmed(s: &str) -> bool {
    !s.starts_with(['\n', '\r']) && s == s.trim_end()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2048, ..ProptestConfig::default() })]

    #[test]
    fn outputs_never_panic_on_markup(s in html_soup()) {
        let dom = parse(&s);
        let (text, md) = serialize_all(&dom);
        prop_assert!(trimmed(&text), "{:?}", text);
        prop_assert!(trimmed(&md), "{:?}", md);
        // Outside verbatim blocks: no trailing whitespace, no blank-line runs.
        if !s.contains("<pre>") && !s.contains("<textarea>") {
            prop_assert!(text.lines().all(|l| l == l.trim_end()), "{:?}", text);
            prop_assert!(!text.contains("\n\n\n"), "{:?}", text);
            prop_assert!(md.lines().all(|l| l == l.trim_end()), "{:?}", md);
            prop_assert!(!md.contains("\n\n\n"), "{:?}", md);
        }
    }

    #[test]
    fn outputs_never_panic_on_chars(s in char_soup()) {
        let dom = parse(&s);
        let (text, md) = serialize_all(&dom);
        prop_assert!(trimmed(&text));
        prop_assert!(trimmed(&md));
    }

    #[test]
    fn outputs_never_panic_on_any_text(s in ".{0,200}") {
        let _ = serialize_all(&parse(&s));
    }
}

// --- deep nesting: iterative traversal, bounded stack -------------------------

const DEPTH: usize = 100_000;

fn nested(open: &str, inner: &str) -> String {
    let mut html = open.repeat(DEPTH);
    html.push_str(inner);
    html
}

#[test]
fn deep_divs() {
    let dom = parse(&nested("<div>", "x"));
    assert_eq!(dom.to_text(), "x");
    #[cfg(feature = "markdown")]
    assert_eq!(dom.to_markdown(), "x");
    let root = dom.root().unwrap();
    assert!(root.inner_html().len() > DEPTH);
    assert_eq!(root.outer_html(), root.to_html());
}

#[test]
fn deep_lists() {
    let dom = parse(&nested("<ul><li>", "x"));
    assert_eq!(dom.to_text(), "x");
    #[cfg(feature = "markdown")]
    {
        // Nesting past the cap is flattened, so the output stays small.
        let md = dom.to_markdown();
        assert!(md.starts_with("- - - ") && md.ends_with(" x"), "{md:?}");
        assert!(md.len() < 1_000, "{} bytes", md.len());
    }
}

#[test]
fn deep_lists_with_text_at_every_level() {
    let dom = parse(&nested("<ol><li>item", ""));
    let text = dom.to_text();
    assert_eq!(text.lines().count(), DEPTH);
    #[cfg(feature = "markdown")]
    {
        let md = dom.to_markdown();
        // Every item survives; indentation is bounded.
        assert_eq!(md.matches("item").count(), DEPTH);
        assert!(md.lines().all(|l| l.len() < 200));
    }
}

#[test]
fn deep_blockquotes() {
    let dom = parse(&nested("<blockquote>", "x"));
    assert_eq!(dom.to_text(), "x");
    #[cfg(feature = "markdown")]
    {
        let md = dom.to_markdown();
        assert!(md.starts_with("> > > ") && md.ends_with("> x"), "{md:?}");
        assert!(md.len() < 1_000, "{} bytes", md.len());
    }
}

#[test]
fn deep_mixed_structures() {
    let chunk = "<div><b><a href=/l><blockquote><ul><li>t<pre>p</pre><table><tr><td><h2>h</h2>";
    let dom = parse(&chunk.repeat(DEPTH / 10));
    assert_eq!(dom.to_text().matches('p').count(), DEPTH / 10);
    assert!(dom.root().unwrap().inner_html().len() > DEPTH);
    #[cfg(feature = "markdown")]
    {
        let md = dom.to_markdown();
        assert!(
            md.contains("```\n") && md.contains("| --- |"),
            "{}",
            &md[..200]
        );
        assert_eq!(md.matches('p').count(), DEPTH / 10);
    }
}

#[test]
fn deep_inline_marks() {
    // (Not `<a>`: a link inside a link closes it, so they would not nest.)
    let dom = parse(&nested("<b><i><del><span>", "x"));
    assert_eq!(dom.to_text(), "x");
    #[cfg(feature = "markdown")]
    assert_eq!(dom.to_markdown(), "***~~x~~***");
}

#[test]
fn blank_runs_in_pre_stay_linear() {
    // Each block inside the `<pre>` owes a line break right after a growing
    // run of newlines; checking that run must not rescan it every time
    // (this took ~70 s when it was quadratic).
    let html = format!("<pre>x{}y</pre>", "<p></p>\n".repeat(DEPTH));
    let text = parse(&html).to_text();
    assert!(text.starts_with('x') && text.ends_with('y'));
    // The first `<p>` owes a blank line; after that the verbatim newlines are
    // already enough.
    assert_eq!(text.matches('\n').count(), DEPTH + 2);
}

#[cfg(feature = "markdown")]
#[test]
fn wide_ragged_tables_stay_linear() {
    // One very wide row and many one-cell rows: padding every row to the
    // widest would be quadratic.
    let n = 5_000;
    let mut html = String::from("<table><tr>");
    html.push_str(&"<td>w</td>".repeat(n));
    html.push_str(&"<tr><td>r</td>".repeat(n));
    html.push_str("</table>");
    let md = parse(&html).to_markdown();
    assert!(md.len() < 20 * n * 3, "{} bytes", md.len());
    assert_eq!(md.matches('r').count(), n);
}

#[cfg(feature = "markdown")]
#[test]
fn many_distinct_links_stay_linear_in_every_link_style() {
    // `Reference` and `Footer` look each link up among the definitions so far;
    // a linear search made 40,000 distinct links take seconds (#95). The
    // ceiling is far above a slow machine and far below the quadratic cost.
    use domq::{LinkStyle, MarkdownOptions};
    let n = 40_000;
    let html: String = (0..n)
        .map(|i| format!("<p><a href=\"https://example.com/articles/page-{i}\">link {i}</a></p>"))
        .collect();
    let dom = parse(&html);
    for style in [LinkStyle::Reference, LinkStyle::Footer] {
        let start = std::time::Instant::now();
        let md = dom.to_markdown_with(&MarkdownOptions::new().link_style(style));
        assert!(
            start.elapsed().as_secs() < 5,
            "{style:?} took {:?}",
            start.elapsed()
        );
        // Every link got its own definition, numbered in order of use.
        assert!(md.contains(&format!("{n}")), "{style:?}");
        assert_eq!(md.matches("https://example.com/articles/page-").count(), n);
    }
}
