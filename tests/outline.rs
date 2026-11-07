use domq::parse;

#[test]
fn basic_page_outline() {
    let html = r#"<!DOCTYPE html>
<html>
<head><title>Test Page</title></head>
<body>
  <header>
    <nav aria-label="Main menu">
      <a href="/" id="nav-home">Home</a>
      <a href="/pricing" id="nav-pricing">Pricing</a>
    </nav>
  </header>
  <main>
    <h1>Product Overview</h1>
    <section aria-label="Features">
      <h2>Key Features</h2>
      <p>Fast and light.</p>
    </section>
    <form id="search" action="/search">
      <label for="q">Search query</label>
      <input type="search" id="q" name="q" placeholder="Search docs">
      <button type="submit">Search</button>
    </form>
  </main>
  <footer>
    <p>&copy; 2026 Acme Corp</p>
    <a href="/terms">Terms</a>
  </footer>
</body>
</html>"#;

    let dom = parse(html);
    let outline = dom.outline();

    // Verify all selectors uniquely resolve back to their element
    #[cfg(feature = "query")]
    for node in outline.nodes() {
        let matches: Vec<_> = dom.select(&node.selector).collect();
        assert_eq!(
            matches.len(),
            1,
            "selector '{}' for node [ref: {}] role: '{}' matched {} elements (expected 1)",
            node.selector,
            node.ref_id,
            node.role,
            matches.len()
        );
    }

    let text = outline.to_text();
    assert!(text.contains("[1] banner"), "{text}");
    assert!(text.contains("[2]   navigation \"Main menu\""), "{text}");
    assert!(
        text.contains("[3]     link \"Home\" -> #nav-home"),
        "{text}"
    );
    assert!(
        text.contains("[4]     link \"Pricing\" -> #nav-pricing"),
        "{text}"
    );
    assert!(text.contains("[5] main"), "{text}");
    assert!(
        text.contains("[6]   heading 1 \"Product Overview\""),
        "{text}"
    );
    assert!(text.contains("[7]   region \"Features\""), "{text}");
    assert!(
        text.contains("[8]     heading 2 \"Key Features\""),
        "{text}"
    );
    assert!(text.contains("[9]   form#search"), "{text}");
    assert!(text.contains("searchbox \"Search query\" -> #q"), "{text}");
    assert!(
        text.contains("button \"Search\" -> button[type=\"submit\"]"),
        "{text}"
    );
    assert!(text.contains("contentinfo"), "{text}");
    assert!(text.contains("link \"Terms\" -> footer > a"), "{text}");
}

#[test]
fn accessible_name_priorities() {
    // 1. aria-label overrides inner text
    let dom = parse(r#"<button aria-label="Close dialog">X</button>"#);
    let outline = dom.outline();
    assert_eq!(outline.nodes()[0].name.as_deref(), Some("Close dialog"));

    // 2. aria-labelledby overrides aria-label and text
    let dom = parse(
        r#"<span id="title">User Settings</span><div role="region" aria-labelledby="title" aria-label="Ignored"></div>"#,
    );
    let outline = dom.outline();
    assert_eq!(outline.nodes()[0].name.as_deref(), Some("User Settings"));

    // 3. Image alt inside button / link
    let dom = parse(r#"<a href="/"><img src="logo.png" alt="Company Logo"></a>"#);
    let outline = dom.outline();
    assert_eq!(outline.nodes()[0].name.as_deref(), Some("Company Logo"));

    // 4. Input with label for
    let dom = parse(r#"<label for="u">Username</label><input id="u" type="text">"#);
    let outline = dom.outline();
    assert_eq!(outline.nodes()[0].name.as_deref(), Some("Username"));

    // 5. Input inside label
    let dom = parse(r#"<label>Password <input type="password"></label>"#);
    let outline = dom.outline();
    assert_eq!(outline.nodes()[0].name.as_deref(), Some("Password"));

    // 6. Placeholder fallback
    let dom = parse(r#"<input type="text" placeholder="Enter email">"#);
    let outline = dom.outline();
    assert_eq!(outline.nodes()[0].name.as_deref(), Some("Enter email"));

    // 7. Submit input value
    let dom = parse(r#"<input type="submit" value="Register Now">"#);
    let outline = dom.outline();
    assert_eq!(outline.nodes()[0].name.as_deref(), Some("Register Now"));
}

#[test]
fn hidden_elements_are_excluded() {
    let html = r#"<body>
  <nav>
    <a href="/public">Visible</a>
    <a href="/secret" hidden>Hidden</a>
    <a href="/secret2" style="display: none">Hidden2</a>
    <a href="/secret3" style="visibility: hidden">Hidden3</a>
    <a href="/secret4" aria-hidden="true">Hidden4</a>
    <input type="hidden" name="token" value="xyz">
  </nav>
  <div style="display: none">
    <button>Hidden button</button>
  </div>
</body>"#;

    let dom = parse(html);
    let outline = dom.outline();

    assert_eq!(outline.len(), 2); // nav and the 1 visible link
    assert_eq!(outline.nodes()[0].role, "navigation");
    assert_eq!(outline.nodes()[1].role, "link");
    assert_eq!(outline.nodes()[1].name.as_deref(), Some("Visible"));
}

#[test]
fn unique_selector_synthesis_types() {
    let html = r#"<body>
  <div data-testid="checkout-btn" role="button">Checkout</div>
  <input name="email_address" type="email">
  <button class="primary-btn">Submit</button>
  <section id="s1">
    <a href="/a">Link A</a>
    <a href="/b">Link B</a>
  </section>
</body>"#;

    let dom = parse(html);
    let outline = dom.outline();

    let btn = &outline.nodes()[0];
    assert_eq!(btn.selector, "[data-testid=\"checkout-btn\"]");

    let input = &outline.nodes()[1];
    assert_eq!(input.selector, "input[name=\"email_address\"]");

    let submit = &outline.nodes()[2];
    assert_eq!(submit.selector, ".primary-btn");

    let link_b = &outline.nodes()[5];
    assert_eq!(link_b.selector, "#s1 > a:nth-of-type(2)");

    #[cfg(feature = "query")]
    {
        assert_eq!(dom.select(&btn.selector).count(), 1);
        assert_eq!(dom.select(&input.selector).count(), 1);
        assert_eq!(dom.select(&submit.selector).count(), 1);
        assert_eq!(dom.select(&link_b.selector).count(), 1);
    }
}

#[test]
fn json_serialization() {
    let html = r#"<main><h1>Docs</h1><a href="/page">Read</a></main>"#;
    let dom = parse(html);
    let outline = dom.outline();
    let json = outline.to_json();

    assert!(json.starts_with('['));
    assert!(json.ends_with(']'));
    assert!(json.contains("\"role\":\"main\""));
    assert!(json.contains("\"role\":\"heading\""));
    assert!(json.contains("\"level\":1"));
    assert!(json.contains("\"name\":\"Docs\""));
    assert!(json.contains("\"role\":\"link\""));
    assert!(json.contains("\"href\":\"/page\""));
    assert!(json.contains("\"source_range\":["));
}

#[test]
fn subtree_outline() {
    let html =
        r#"<div><header><a href="/">Home</a></header><main><button>Click</button></main></div>"#;
    let dom = parse(html);
    let main = dom.find_by_tag("main").next().unwrap();

    let outline = main.outline();
    assert_eq!(outline.len(), 2);
    assert_eq!(outline.nodes()[0].role, "main");
    assert_eq!(outline.nodes()[0].depth, 0);
    assert_eq!(outline.nodes()[1].role, "button");
    assert_eq!(outline.nodes()[1].depth, 1);
}

#[test]
fn explicit_roles_and_aria_levels() {
    let html = r#"<div role="navigation"><div role="heading" aria-level="3">Sub Section</div><div role="button">Action</div></div>"#;
    let dom = parse(html);
    let outline = dom.outline();

    assert_eq!(outline.len(), 3);
    assert_eq!(outline.nodes()[0].role, "navigation");
    assert_eq!(outline.nodes()[1].role, "heading");
    assert_eq!(outline.nodes()[1].level, Some(3));
    assert_eq!(outline.nodes()[1].name.as_deref(), Some("Sub Section"));
    assert_eq!(outline.nodes()[2].role, "button");
}

// --- scaling (#90) -------------------------------------------------------------
//
// Each case used to take seconds to minutes at these sizes (every outline node
// scanned the whole document). The ceiling is far above a slow or loaded
// machine and far below that, so it guards the complexity, not the speed.

/// Outline `html` within the time limit, expecting `nodes` nodes.
fn scales(what: &str, html: &str, nodes: usize) -> domq::PageOutline {
    let dom = parse(html);
    let start = std::time::Instant::now();
    let outline = dom.outline();
    assert!(
        start.elapsed().as_secs() < 5,
        "{what}: took {:?}",
        start.elapsed()
    );
    assert_eq!(outline.len(), nodes, "{what}");
    outline
}

#[test]
fn many_links_scale() {
    let n = 20_000;
    let html = format!(
        "<ul>{}</ul>",
        (0..n)
            .map(|i| format!("<li><a href=/p{i}>link {i}</a></li>"))
            .collect::<String>()
    );
    let outline = scales("links", &html, n);
    assert_eq!(
        outline.nodes()[n - 1].selector,
        "ul > li:nth-of-type(20000) > a"
    );
}

#[test]
fn many_buttons_with_one_class_scale() {
    let n = 20_000;
    let html = format!("<div>{}</div>", "<button class=btn-i>go</button>".repeat(n));
    let outline = scales("buttons", &html, n);
    assert_eq!(outline.nodes()[7].selector, "div > button:nth-of-type(8)");
}

#[test]
fn many_labels_and_inputs_scale() {
    let n = 20_000;
    let html: String = (0..n)
        .map(|i| format!("<label for=i{i}>Label {i}</label><input id=i{i}>"))
        .collect();
    let outline = scales("labels", &html, n);
    let last = &outline.nodes()[n - 1];
    assert_eq!(last.name.as_deref(), Some("Label 19999"));
    assert_eq!(last.selector, "#i19999");
}

#[test]
fn many_inputs_in_one_label_scale() {
    // Every input asks the same wrapping label for its name.
    let n = 20_000;
    let html = format!(
        "<label>Pick one {}</label>",
        "<input type=checkbox>".repeat(n)
    );
    let outline = scales("inputs in a label", &html, n);
    assert_eq!(outline.nodes()[0].name.as_deref(), Some("Pick one"));
}

#[test]
fn many_controls_naming_one_big_element_scale() {
    let n = 20_000;
    let html = format!(
        "<div id=big>{}</div>{}",
        "word ".repeat(n),
        "<button aria-labelledby=big>b</button>".repeat(n)
    );
    let outline = scales("labelledby", &html, n);
    let name = outline.nodes()[0].name.as_deref().unwrap_or("");
    assert!(
        name.starts_with("word word") && name.ends_with('…'),
        "{name}"
    );
}

#[test]
fn deeply_nested_buttons_and_sections_with_short_selectors_scale() {
    // A path selector is as long as the tree is deep, so an outline of nested
    // elements without ids is quadratic in size on its own; a unique id ends
    // the path, which leaves the names and the sectioning to be linear.
    let n = 5_000;
    let buttons: String = (0..n)
        .map(|i| format!("<div role=button id=b{i}>text "))
        .collect();
    let outline = scales("nested buttons", &buttons, n);
    let name = outline.nodes()[3].name.as_deref().unwrap_or("");
    assert!(
        name.starts_with("text text") && name.ends_with('…'),
        "{name}"
    );
    assert_eq!(outline.nodes()[3].selector, "#b3");
    let sections: String = (0..n)
        .map(|i| format!("<section id=s{i}><header>h "))
        .collect();
    let outline = scales("nested sections", &sections, 2 * n);
    // A header in a section is a plain header, not a banner.
    assert_eq!(outline.nodes()[1].role, "header");
}

#[test]
fn nested_elements_without_ids_take_time_in_proportion_to_their_selectors() {
    // The case from the issue. The output is quadratic here (every selector
    // lists every ancestor), so this is no guard against a slow algorithm,
    // only a check that such a page still gets its outline.
    let n = 1_000;
    let buttons = "<div role=button>t ".repeat(n);
    let outline = scales("nested role=button", &buttons, n);
    assert_eq!(
        outline.nodes()[n - 1].selector.matches(" > ").count(),
        n - 1
    );
    let sections = "<section><header>h ".repeat(n);
    scales("nested section/header", &sections, 2 * n);
}
