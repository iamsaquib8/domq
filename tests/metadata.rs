//! `Dom::metadata`: sources, fallbacks and robustness.

use domq::{Metadata, parse};

fn s(v: &str) -> Option<String> {
    Some(v.to_string())
}

fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn property_and_name_alias_same_key_keeps_single_entry() {
    let meta = parse(
        r#"<html lang="en"><head>
             <meta property="og:title" name="og:title" content="Shared title">
             <meta property="og:description" name="og:description" content="Shared desc">
             <meta property="twitter:card" name="twitter:card" content="summary_large_image">
             <meta name="description" content="Plain description">
             <meta property="og:image" content="/a.png">
             <meta property="og:image" content="/b.png">
           </head></html>"#,
    )
    .metadata();
    // Both attributes name the same key -> one entry, not two.
    let og_titles: Vec<_> = meta
        .og
        .iter()
        .filter(|(k, _)| k == "og:title")
        .map(|(_, v)| v.as_str())
        .collect();
    assert_eq!(og_titles, ["Shared title"]);
    let og_descs: Vec<_> = meta
        .og
        .iter()
        .filter(|(k, _)| k == "og:description")
        .map(|(_, v)| v.as_str())
        .collect();
    assert_eq!(og_descs, ["Shared desc"]);
    // Distinct properties still all survive (multi-image stays intact).
    assert_eq!(meta.og.iter().filter(|(k, _)| k == "og:image").count(), 2);
    assert_eq!(
        meta.twitter,
        pairs(&[("twitter:card", "summary_large_image")])
    );
    // And `name` lookup still sees the plain description.
    assert_eq!(meta.description, s("Plain description"));
}

/// Issue #36: one `<meta>` carrying both `property` and `name` for the same
/// key records one og/twitter entry, not two.
#[test]
fn og_and_twitter_alias_on_one_tag_is_recorded_once() {
    let meta = parse(
        r#"<meta property="og:title" name="og:title" content="Hello">
           <meta property="twitter:title" name="twitter:title" content="Hello">"#,
    )
    .metadata();
    assert_eq!(meta.og, pairs(&[("og:title", "Hello")]));
    assert_eq!(meta.twitter, pairs(&[("twitter:title", "Hello")]));
}

#[test]
fn og_alias_dedupe_is_case_insensitive_and_keeps_distinct_keys() {
    let meta = parse(
        r#"<meta property="OG:Title" name="og:title" content="A">
           <meta property="og:title" content="B">
           <meta property="og:site_name" name="og:title" content="C">
           <meta name="og:image" property="og:image" content="/i.png">
           <meta property="og:image" content="/j.png">"#,
    )
    .metadata();
    // Separate tags repeating a key stay; a differing `name` still counts.
    assert_eq!(
        meta.og,
        pairs(&[
            ("og:title", "A"),
            ("og:title", "B"),
            ("og:site_name", "C"),
            ("og:title", "C"),
            ("og:image", "/i.png"),
            ("og:image", "/j.png"),
        ])
    );
}

#[test]
fn blog_fixture() {
    let meta = parse(include_str!("pages/reader/blog-comments.html")).metadata();
    // The JSON-LD `headline`, without the site name `<title>` adds.
    assert_eq!(meta.title, s("Why we rewrote the importer in Rust"));
    assert_eq!(
        meta.description,
        s("Three months, one importer, and what changed when the hot path stopped allocating.")
    );
    assert_eq!(
        meta.canonical,
        s("https://fieldnotes.example/2026/03/why-we-rewrote-the-importer/")
    );
    assert_eq!(meta.lang, s("en-GB"));
    assert_eq!(meta.site_name, s("Field Notes"));
    assert_eq!(
        meta.image,
        s("https://fieldnotes.example/wp-content/uploads/2026/03/flame.png")
    );
    // `article:author` is a URL here, so the byline link's text wins.
    assert_eq!(meta.author, s("Maya Lindqvist"));
    assert_eq!(meta.published, s("2026-03-14T09:30:00+00:00"));
    assert_eq!(meta.keywords, ["rust", "performance"]); // from article:tag
    assert_eq!(meta.og.len(), 8);
    assert_eq!(meta.og[0], ("og:type".to_string(), "article".to_string()));
    assert_eq!(
        meta.twitter,
        pairs(&[
            ("twitter:card", "summary_large_image"),
            ("twitter:creator", "@maya")
        ])
    );
    assert_eq!(meta.json_ld.len(), 1);
    assert!(
        meta.json_ld[0].starts_with(r#"{"@context":"https://schema.org","@type":"BlogPosting""#)
    );
    // Two RSS feeds; the `application/json` REST link is not a feed.
    assert_eq!(
        meta.feeds,
        [
            "https://fieldnotes.example/feed/",
            "https://fieldnotes.example/comments/feed/"
        ]
    );
}

#[test]
fn news_fixture() {
    let meta = parse(include_str!("pages/reader/news.html")).metadata();
    assert_eq!(meta.title, s("City council approves night-bus pilot"));
    assert_eq!(meta.author, s("Lena Okafor"));
    assert_eq!(meta.keywords, ["transport", "night bus", "city council"]);
    // No canonical link: og:url.
    assert_eq!(
        meta.canonical,
        s("https://tribune.example/local/2026/04/02/night-bus-pilot")
    );
    // No article:published_time: the <time> in the <article>.
    assert_eq!(meta.published, s("2026-04-02T18:15:00Z"));
    assert_eq!(meta.json_ld.len(), 1);
    assert!(meta.json_ld[0].starts_with('{') && meta.json_ld[0].ends_with('}'));
    assert!(meta.feeds.is_empty());
}

#[test]
fn empty_documents() {
    assert_eq!(parse("").metadata(), Metadata::default());
    assert_eq!(parse("<p>just text</p>").metadata(), Metadata::default());
}

#[test]
fn title_fallbacks() {
    let og = r#"<meta property="og:title" content="OG title"><meta name="twitter:title" content="TW title">"#;
    assert_eq!(parse(og).metadata().title, s("OG title"));
    assert_eq!(
        parse(r#"<meta name="twitter:title" content="TW title">"#)
            .metadata()
            .title,
        s("TW title")
    );
    // An empty <title> falls through to the next one / og:title.
    assert_eq!(
        parse(&format!("<title>  </title>{og}")).metadata().title,
        s("OG title")
    );
    // <title> inside <svg> is an SVG tooltip, not the page title.
    assert_eq!(
        parse("<body><svg><title>Icon</title></svg><title>Page</title></body>")
            .metadata()
            .title,
        s("Page")
    );
    assert_eq!(
        parse("<title>First</title><title>Second</title>")
            .metadata()
            .title,
        s("First")
    );
    assert_eq!(
        parse("<title>\n  Spread \t over\n lines &amp; entities </title>")
            .metadata()
            .title,
        s("Spread over lines & entities")
    );
}

#[test]
fn description_lang_site_image_fallbacks() {
    let meta = parse(
        r#"<meta property="og:description" content="From OG">
           <meta http-equiv="Content-Language" content="de">
           <meta name="application-name" content="My App">
           <meta name="twitter:image:src" content="/tw.png">"#,
    )
    .metadata();
    assert_eq!(meta.description, s("From OG"));
    assert_eq!(meta.lang, s("de"));
    assert_eq!(meta.site_name, s("My App"));
    assert_eq!(meta.image, s("/tw.png"));

    let meta = parse(r#"<link rel="image_src" href="/thumb.jpg"><html lang=" fr-CA ">"#).metadata();
    assert_eq!(meta.image, s("/thumb.jpg"));
    assert_eq!(meta.lang, s("fr-CA"));

    let meta =
        parse(r#"<meta property="og:image:secure_url" content="https://x/s.png">"#).metadata();
    assert_eq!(meta.image, s("https://x/s.png"));
}

#[test]
fn names_and_properties_are_case_insensitive() {
    let meta = parse(
        r#"<META NAME="Description" CONTENT="Upper">
           <meta property="OG:Site_Name" content="Site">
           <meta name="Twitter:Card" content="summary">
           <LINK REL="Canonical Alternate" HREF="/c">"#,
    )
    .metadata();
    assert_eq!(meta.description, s("Upper"));
    assert_eq!(meta.site_name, s("Site"));
    assert_eq!(meta.og, pairs(&[("og:site_name", "Site")]));
    assert_eq!(meta.twitter, pairs(&[("twitter:card", "summary")]));
    assert_eq!(meta.canonical, s("/c"));
}

#[test]
fn first_value_wins_and_empty_values_are_ignored() {
    let meta = parse(
        r#"<meta name="description" content="  ">
           <meta name="description" content="Second">
           <meta name="description" content="Third">
           <link rel=canonical href=""><link rel=canonical href="/real">"#,
    )
    .metadata();
    assert_eq!(meta.description, s("Second"));
    assert_eq!(meta.canonical, s("/real"));
}

#[test]
fn og_and_twitter_collect_every_pair() {
    let meta = parse(
        r#"<meta property="og:image" content="/1.png">
           <meta property="og:image" content="/2.png">
           <meta property="article:section" content="Tech">
           <meta property="book:isbn" content="123">
           <meta property="fb:app_id" content="999">
           <meta name="og:locale" content="en_US">
           <meta property="twitter:site" content="@a">
           <meta name="twitter:site" content="@b">"#,
    )
    .metadata();
    assert_eq!(
        meta.og,
        pairs(&[
            ("og:image", "/1.png"),
            ("og:image", "/2.png"),
            ("article:section", "Tech"),
            ("book:isbn", "123"),
            ("og:locale", "en_US"),
        ])
    );
    assert_eq!(
        meta.twitter,
        pairs(&[("twitter:site", "@a"), ("twitter:site", "@b")])
    );
    assert_eq!(meta.image, s("/1.png"));
}

#[test]
fn author_sources() {
    assert_eq!(
        parse(r#"<meta name=author content="Ann"><a rel=author>Bob</a>"#)
            .metadata()
            .author,
        s("Ann")
    );
    assert_eq!(
        parse(r#"<meta property="article:author" content="Cy Young">"#)
            .metadata()
            .author,
        s("Cy Young")
    );
    assert_eq!(
        parse(r#"<meta property="article:author" content="https://x/cy">"#)
            .metadata()
            .author,
        s("https://x/cy")
    );
    assert_eq!(
        parse(r#"<p>By <a href=/d rel="author external"> Dee  Dee </a></p>"#)
            .metadata()
            .author,
        s("Dee Dee")
    );
}

#[test]
fn published_sources() {
    let date = |html: &str| parse(html).metadata().published;
    assert_eq!(
        date(
            r#"<meta property="article:published_time" content="2026-01-01"><meta name=date content="2020">"#
        ),
        s("2026-01-01")
    );
    assert_eq!(
        date(r#"<meta itemprop="datePublished" content="2026-02-02">"#),
        s("2026-02-02")
    );
    assert_eq!(
        date(r#"<meta name="DC.date.issued" content="2026-03-03">"#),
        s("2026-03-03")
    );
    assert_eq!(
        date(r#"<meta name="pubdate" content="20260404">"#),
        s("20260404")
    );
    // <time>: only inside an <article>, or marked as the publication date.
    assert_eq!(
        date(r#"<footer><time datetime="1999">x</time></footer>"#),
        None
    );
    assert_eq!(
        date(
            r#"<time datetime="1999">x</time><article><time datetime="2026-05-05">May</time></article>"#
        ),
        s("2026-05-05")
    );
    assert_eq!(
        date(r#"<time pubdate datetime="2026-06-06">x</time>"#),
        s("2026-06-06")
    );
    assert_eq!(
        date(r#"<span><time itemprop="datePublished" datetime="2026-07-07">x</time></span>"#),
        s("2026-07-07")
    );
}

#[test]
fn keywords() {
    assert_eq!(
        parse(r#"<meta name=keywords content=" a, b ,,c , ">"#)
            .metadata()
            .keywords,
        ["a", "b", "c"]
    );
    assert_eq!(
        parse(r#"<meta name=news_keywords content="x,y">"#)
            .metadata()
            .keywords,
        ["x", "y"]
    );
}

#[test]
fn json_ld_blocks() {
    let meta = parse(
        r#"<script type="application/ld+json">
             {"@type": "Person"}
           </script>
           <script type="Application/LD+JSON; charset=utf-8">[1]</script>
           <script type="application/ld+json">   </script>
           <script type="application/json">{"not": "ld"}</script>
           <script>var x = 1;</script>"#,
    )
    .metadata();
    assert_eq!(meta.json_ld, [r#"{"@type": "Person"}"#, "[1]"]);
}

#[test]
fn feeds() {
    let meta = parse(
        r#"<link rel=alternate type="application/atom+xml" href="/atom.xml">
           <link rel="alternate" type="application/feed+json" href="/feed.json">
           <link rel="alternate" type="APPLICATION/RSS+XML; charset=utf-8" href="/rss">
           <link rel="alternate" hreflang="de" href="/de/">
           <link rel="stylesheet" type="application/rss+xml" href="/odd">"#,
    )
    .metadata();
    assert_eq!(meta.feeds, ["/atom.xml", "/feed.json", "/rss"]);
}

#[test]
fn metadata_outside_head_and_in_templates() {
    // This parser keeps misplaced tags where they are: they still count. But
    // <template> content is inert.
    let meta = parse(
        r#"<body><meta name=description content="In body">
           <template><title>Template</title><meta name=author content="T"></template></body>"#,
    )
    .metadata();
    assert_eq!(meta.description, s("In body"));
    assert_eq!(meta.title, None);
    assert_eq!(meta.author, None);
}

#[test]
fn metadata_is_owned() {
    let meta = {
        let dom = parse("<title>Owned</title>");
        dom.metadata()
    };
    assert_eq!(meta.title, s("Owned"));
}

#[test]
fn many_meta_tags_stay_linear() {
    let html = r#"<meta name="x-custom" content="v"><meta property="og:image" content="/i">"#
        .repeat(50_000);
    let meta = parse(&html).metadata();
    assert_eq!(meta.og.len(), 50_000);
    assert_eq!(meta.description, None);
}

#[test]
fn deep_svg_and_articles() {
    let depth = 100_000;
    let html = format!(
        "{}<title>t</title>{}",
        "<svg>".repeat(depth),
        "<article><time datetime=d>".repeat(depth)
    );
    let meta = parse(&html).metadata();
    assert_eq!(meta.title, None);
}

#[cfg(feature = "serde")]
#[test]
fn serializes_with_serde() {
    let meta =
        parse(r#"<html lang=en><title>T</title><meta property="og:type" content="website">"#)
            .metadata();
    let json = serde_json::to_value(&meta).unwrap();
    assert_eq!(json["title"], "T");
    assert_eq!(json["lang"], "en");
    assert_eq!(json["description"], serde_json::Value::Null);
    assert_eq!(json["og"][0][0], "og:type");
    assert_eq!(json["og"][0][1], "website");
    assert_eq!(json["feeds"], serde_json::json!([]));
    assert_eq!(json.as_object().unwrap().len(), 13);
}

#[test]
fn json_ld_wrappers_are_stripped() {
    let json = r#"{"@context":"https://schema.org","@type":"Article","headline":"A \u2014 B"}"#;
    for wrapped in [
        format!("<![CDATA[\n  {json}\n  ]]>"),
        format!("//<![CDATA[\n{json}\n//]]>"),
        format!("/*<![CDATA[*/\n{json}\n/*]]>*/"),
        format!("<!--\n{json}\n-->"),
        format!("<!-- <![CDATA[ {json} ]]> -->"),
        format!("  \n{json}\n  "),
        json.to_string(),
    ] {
        let dom = parse(&format!(
            "<script type=\"application/ld+json\">{wrapped}</script>"
        ));
        let meta = dom.metadata();
        assert_eq!(meta.json_ld, [json], "{wrapped}");
        let value: serde_json::Value = serde_json::from_str(&meta.json_ld[0]).unwrap();
        assert_eq!(value["headline"], "A \u{2014} B");
    }
}

#[test]
fn json_ld_that_is_not_wrapped_is_left_alone() {
    for (block, want) in [
        // Only one half of a wrapper.
        ("<![CDATA[ {\"a\":1}", Some("<![CDATA[ {\"a\":1}")),
        ("{\"a\":1} ]]>", Some("{\"a\":1} ]]>")),
        ("<!-- {\"a\":1}", Some("<!-- {\"a\":1}")),
        // A wrapper round nothing is no block.
        ("<![CDATA[ ]]>", None),
        ("<!-- -->", None),
        ("//<![CDATA[ //]]>", None),
        // Text that merely contains them.
        (r#"{"a":"<![CDATA[x]]>"}"#, Some(r#"{"a":"<![CDATA[x]]>"}"#)),
        (r#"[{"a":"-->"}]"#, Some(r#"[{"a":"-->"}]"#)),
    ] {
        let dom = parse(&format!(
            "<script type=\"application/ld+json\">{block}</script>"
        ));
        let meta = dom.metadata();
        assert_eq!(meta.json_ld.first().map(String::as_str), want, "{block}");
    }
    // Nested wrappers do not loop.
    let deep = format!("{}{{}}{}", "<!--".repeat(1000), "-->".repeat(1000));
    let dom = parse(&format!("<script type=ld+json>{deep}</script>"));
    assert!(dom.metadata().json_ld.len() <= 1);
}

// JSON-LD as a source (#100).

fn ld(json: &str) -> Metadata {
    parse(&format!(
        "<head><title>Page | Site</title>\
         <meta name=author content='Meta Author'>\
         <meta property=article:published_time content='2000-01-01'>\
         <meta property=og:image content='https://m.example/og.png'>\
         <script type='application/ld+json'>{json}</script></head>"
    ))
    .metadata()
}

#[test]
fn json_ld_article_fields_come_first() {
    let meta = ld(r#"{
        "@context": "https://schema.org", "@type": "NewsArticle",
        "headline": "  The   real headline ",
        "datePublished": "2024-05-06T07:08:09+00:00",
        "author": {"@type": "Person", "name": "Jane Doe"},
        "image": {"@type": "ImageObject", "url": "https://ld.example/a.jpg"}
    }"#);
    assert_eq!(meta.title.as_deref(), Some("The real headline"));
    assert_eq!(meta.published.as_deref(), Some("2024-05-06T07:08:09+00:00"));
    assert_eq!(meta.author.as_deref(), Some("Jane Doe"));
    // `og:image` is an explicit choice of the page: it stays first.
    assert_eq!(meta.image.as_deref(), Some("https://m.example/og.png"));
    // The block itself is still there.
    assert_eq!(meta.json_ld.len(), 1);
}

#[test]
fn json_ld_forms_of_author_and_image() {
    let author = |json: &str| ld(json).author;
    assert_eq!(
        author(r#"{"author": "Ann Lee"}"#).as_deref(),
        Some("Ann Lee")
    );
    assert_eq!(
        author(r#"{"author": [{"name": "A"}, "B", {"@type": "Person", "name": " C  D "}]}"#)
            .as_deref(),
        Some("A, B, C D")
    );
    // Empty or non-text authors fall back to the meta tag.
    assert_eq!(author(r#"{"author": {}}"#).as_deref(), Some("Meta Author"));
    assert_eq!(author(r#"{"author": 5}"#).as_deref(), Some("Meta Author"));
    assert_eq!(author(r#"{"author": ""}"#).as_deref(), Some("Meta Author"));
    // An `@id` reference into the graph, as WordPress writes it.
    assert_eq!(
        author(
            r##"{"@graph": [
                {"@type": "WebSite", "name": "Site"},
                {"@type": "Article", "headline": "H", "author": {"@id": "https://x.example/#/schema/person/1"}},
                {"@type": "Person", "@id": "https://x.example/#/schema/person/1", "name": "Pat"}
            ]}"##
        )
        .as_deref(),
        Some("Pat")
    );
    // Image forms, without an og:image.
    let image = |json: &str| {
        parse(&format!(
            "<script type='application/ld+json'>{json}</script>"
        ))
        .metadata()
        .image
    };
    assert_eq!(
        image(r#"{"image": "https://i/a.png"}"#).as_deref(),
        Some("https://i/a.png")
    );
    assert_eq!(
        image(r#"{"image": ["https://i/1.png", "https://i/2.png"]}"#).as_deref(),
        Some("https://i/1.png")
    );
    assert_eq!(
        image(r#"{"image": [{"url": "https://i/3.png"}]}"#).as_deref(),
        Some("https://i/3.png")
    );
    assert_eq!(
        image(r#"{"image": {"contentUrl": "https://i/4.png"}}"#).as_deref(),
        Some("https://i/4.png")
    );
    assert_eq!(image(r#"{"image": 7}"#), None);
}

#[test]
fn json_ld_prefers_article_objects_and_walks_graphs_and_arrays() {
    // The page's own entity comes before the article in the graph, and has
    // a date; the article's date wins.
    let meta = ld(r#"{"@graph": [
        {"@type": "WebPage", "datePublished": "1999-01-01", "headline": "Page headline"},
        {"@type": ["Thing", "BlogPosting"], "datePublished": "2024-02-03", "headline": "Post"}
    ]}"#);
    assert_eq!(meta.published.as_deref(), Some("2024-02-03"));
    assert_eq!(meta.title.as_deref(), Some("Post"));
    // A top-level array, several blocks, and a non-article for what the
    // article lacks.
    let meta = parse(
        r#"<script type="application/ld+json">[{"@type":"Organization","name":"Acme"}]</script>
           <script type="application/ld+json">[{"@type":"Article","headline":"H"},
                                               {"@type":"WebPage","datePublished":"2023-01-02"}]</script>"#,
    )
    .metadata();
    assert_eq!(meta.title.as_deref(), Some("H"));
    assert_eq!(meta.published.as_deref(), Some("2023-01-02"));
    // A type given as a URL.
    let meta = ld(
        r#"{"@type": "https://schema.org/Article", "headline": "U", "datePublished": "2022-01-01"}"#,
    );
    assert_eq!(meta.title.as_deref(), Some("U"));
}

#[test]
fn json_ld_that_is_not_usable_changes_nothing() {
    for json in [
        "",
        "not json",
        "{",
        r#"{"headline": "x""#,
        r#"{"headline": "x"} trailing"#,
        r#"{"headline": 5, "datePublished": null, "author": [], "image": {}}"#,
        r#"[1, 2, {"a": [true, false, null, -1.5e3]}]"#,
    ] {
        let meta = ld(json);
        assert_eq!(meta.title.as_deref(), Some("Page | Site"), "{json}");
        assert_eq!(meta.author.as_deref(), Some("Meta Author"), "{json}");
        assert_eq!(meta.published.as_deref(), Some("2000-01-01"), "{json}");
    }
}

#[test]
fn json_ld_strings_are_unescaped() {
    let meta = ld(
        r#"{"@type":"Article","headline":"Café 🦀 \"quoted\" \/ \\ \n tab\tend","author":"Zoë"}"#,
    );
    assert_eq!(
        meta.title.as_deref(),
        Some("Café \u{1f980} \"quoted\" / \\ tab end")
    );
    assert_eq!(meta.author.as_deref(), Some("Zoë"));
    // A lone surrogate is U+FFFD; a raw line break inside a string is kept
    // (and collapsed in the text field).
    let meta = ld("{\"@type\":\"Article\",\"headline\":\"a\\ud800b\nc\"}");
    assert_eq!(meta.title.as_deref(), Some("a\u{fffd}b c"));
}

#[test]
fn json_ld_reading_is_bounded() {
    // Deep nesting is refused, not recursed into: the fields come from
    // the meta tags.
    for open in ["[", "{\"a\":"] {
        let close = if open == "[" { "]" } else { "}" };
        let deep = format!("{}1{}", open.repeat(100_000), close.repeat(100_000));
        let meta = ld(&deep);
        assert_eq!(meta.title.as_deref(), Some("Page | Site"));
    }
    // Moderate nesting is fine.
    let ok = format!(
        r#"{{"@type":"Article","headline":"H","x":{}1{}}}"#,
        "[".repeat(20),
        "]".repeat(20)
    );
    assert_eq!(ld(&ok).title.as_deref(), Some("H"));
    // A huge block is refused in linear time.
    let many = format!("[{}]", "0,".repeat(1_000_000) + "0");
    let start = std::time::Instant::now();
    assert_eq!(ld(&many).title.as_deref(), Some("Page | Site"));
    assert!(start.elapsed().as_secs() < 5, "took {:?}", start.elapsed());
    // Many objects, each looked at: bounded too.
    let objects = format!(
        "[{}]",
        r#"{"@type":"Thing"},"#.repeat(200_000) + r#"{"@type":"Article","headline":"late"}"#
    );
    let start = std::time::Instant::now();
    let _ = ld(&objects);
    assert!(start.elapsed().as_secs() < 5, "took {:?}", start.elapsed());
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config { cases: 512, ..Default::default() })]

    #[test]
    fn json_ld_reading_never_panics(text in proptest::collection::vec(
        proptest::sample::select(vec![
            "{", "}", "[", "]", ",", ":", "\"", "\\", "\\u", "d800", "dc00", "@graph", "@type",
            "Article", "headline", "author", "name", "@id", "image", "url", "datePublished",
            "true", "null", "-1.5e+3", "é", "🦀", "\n", " ", "<", "a",
        ]),
        0..120,
    )) {
        let text: String = text.concat();
        let meta = parse(&format!("<script type=application/ld+json>{text}</script>")).metadata();
        let _ = (meta.title, meta.author, meta.published, meta.image);
    }
}
