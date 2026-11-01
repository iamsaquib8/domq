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
    assert_eq!(dom.select(&btn.selector).count(), 1);

    let input = &outline.nodes()[1];
    assert_eq!(input.selector, "input[name=\"email_address\"]");
    assert_eq!(dom.select(&input.selector).count(), 1);

    let submit = &outline.nodes()[2];
    assert_eq!(submit.selector, ".primary-btn");
    assert_eq!(dom.select(&submit.selector).count(), 1);

    let link_b = &outline.nodes()[5];
    assert_eq!(link_b.selector, "#s1 > a:nth-of-type(2)");
    assert_eq!(dom.select(&link_b.selector).count(), 1);
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
