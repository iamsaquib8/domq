//! Tests for the JS API, run in Node with: `wasm-pack test --node`.
//!
//! This file is compiled as a module of the library (see the bottom of
//! `src/lib.rs`), so it calls the exported functions directly and inspects the
//! JS values they return. `tests/node.mjs` covers the built npm package itself.

#![cfg(target_arch = "wasm32")]

use crate::{
    Document, decode, errors, parse, parse_bytes, parse_json, resolve_srcset, resolve_url, select,
    select_first, to_html, to_markdown, to_text,
};
use js_sys::{JSON, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::wasm_bindgen_test;

/// `JSON.stringify(value)`; `"undefined"` for `undefined`.
fn js(value: &JsValue) -> String {
    if value.is_undefined() {
        return "undefined".into();
    }
    JSON::stringify(value).unwrap().as_string().unwrap()
}

fn get(value: &JsValue, key: &str) -> JsValue {
    Reflect::get(value, &JsValue::from_str(key)).unwrap()
}

/// Assert `err` is the `SelectorError` we throw, and return (message, position).
fn selector_error(err: JsValue) -> (String, f64) {
    assert!(
        err.is_instance_of::<js_sys::Error>(),
        "not an Error: {err:?}"
    );
    assert_eq!(get(&err, "name").as_string().unwrap(), "SelectorError");
    (
        get(&err, "message").as_string().unwrap(),
        get(&err, "position").as_f64().unwrap(),
    )
}

const LIST: &str = "<ul id=list><li class=item>One<li>Two &amp; more</ul>";

#[wasm_bindgen_test]
fn parse_returns_a_js_object() {
    let doc = parse(&JsValue::from_str("<p class='x'>hi &amp; bye</p>")).unwrap();
    assert_eq!(
        js(&doc),
        r#"{"type":"document","errors":0,"children":[{"type":"element","tag":"p","attrs":{"class":"x"},"children":[{"type":"text","value":"hi & bye"}]}]}"#
    );
    // Recovered problems are counted.
    assert!(
        get(&parse(&JsValue::from_str("<b><i>x</b>")).unwrap(), "errors")
            .as_f64()
            .unwrap()
            > 0.0
    );
}

#[wasm_bindgen_test]
fn parse_json_returns_the_same_tree_as_a_string() {
    let html = "<!doctype html><!-- c --><a href=/x>link</a>";
    assert_eq!(
        parse_json(html),
        js(&parse(&JsValue::from_str(html)).unwrap())
    );
    assert_eq!(
        parse_json("<br>"),
        r#"{"type":"document","errors":0,"children":[{"type":"element","tag":"br","attrs":{},"children":[]}]}"#
    );
}

#[wasm_bindgen_test]
fn select_returns_matches() {
    let hits = select(LIST, "li").unwrap();
    assert_eq!(
        js(&hits),
        concat!(
            r#"[{"tag":"li","attrs":{"class":"item"},"text":"One","html":"<li class=\"item\">One</li>","nodeIndex":1},"#,
            r#"{"tag":"li","attrs":{},"text":"Two & more","html":"<li>Two &amp; more</li>","nodeIndex":3}]"#
        )
    );
    assert_eq!(js(&select(LIST, "table").unwrap()), "[]");
}

#[wasm_bindgen_test]
fn node_index_is_the_depth_first_position_in_the_tree() {
    // Mis-nested and unclosed on purpose.
    let html = "<!doctype html><div><p>a<b>b<i>c</b>d</i><p>e</div><span>f";
    let tree = parse(&JsValue::from_str(html)).unwrap();
    let mut order = Vec::new(); // tag (or "#text" etc.) in depth-first order
    let mut stack = vec![get(&tree, "children")];
    let mut cursors = vec![0u32];
    while let Some(list) = stack.last().cloned() {
        let array: js_sys::Array = list.into();
        let i = cursors.last().copied().unwrap();
        if i >= array.length() {
            stack.pop();
            cursors.pop();
            continue;
        }
        *cursors.last_mut().unwrap() += 1;
        let node = array.get(i);
        let kind = get(&node, "type").as_string().unwrap();
        if kind == "element" {
            order.push(get(&node, "tag").as_string().unwrap());
            stack.push(get(&node, "children"));
            cursors.push(0);
        } else {
            order.push(format!("#{kind}"));
        }
    }
    let hits: js_sys::Array = select(html, "*").unwrap().into();
    assert!(hits.length() > 5);
    for hit in hits.iter() {
        let index = get(&hit, "nodeIndex").as_f64().unwrap() as usize;
        assert_eq!(order[index], get(&hit, "tag").as_string().unwrap());
    }
}

#[wasm_bindgen_test]
fn invalid_selectors_throw_a_selector_error() {
    let (message, position) = selector_error(select(LIST, "div > p)").unwrap_err());
    assert_eq!(message, "unexpected ')' at byte 7");
    assert_eq!(position, 7.0);

    let (message, position) = selector_error(select(LIST, "a:hover").unwrap_err());
    assert_eq!(message, "unsupported pseudo-class ':hover' at byte 1");
    assert_eq!(position, 1.0);

    // `position` is a JS string index (UTF-16), not a byte offset.
    let (message, position) = selector_error(select(LIST, "p.é)").unwrap_err());
    assert_eq!(message, "unexpected ')' at byte 4");
    assert_eq!(position, 3.0);

    // Every function that takes a selector throws the same way.
    selector_error(select_first(LIST, "").unwrap_err());
    selector_error(to_text(LIST, Some("li >".into())).unwrap_err());
    selector_error(to_markdown(LIST, Some("::before".into())).unwrap_err());
    let doc = Document::new(&JsValue::from_str(LIST)).unwrap();
    selector_error(doc.select("[").unwrap_err());
    selector_error(doc.select_first("li:nope").unwrap_err());
    selector_error(doc.to_text(Some(",".into())).unwrap_err());
    selector_error(doc.to_markdown(Some("a >> b".into())).unwrap_err());
}

#[wasm_bindgen_test]
fn select_first_returns_a_match_or_undefined() {
    let hit = select_first(LIST, "li:last-child").unwrap();
    assert_eq!(
        js(&hit),
        r#"{"tag":"li","attrs":{},"text":"Two & more","html":"<li>Two &amp; more</li>","nodeIndex":3}"#
    );
    assert!(select_first(LIST, "table").unwrap().is_undefined());
}

#[wasm_bindgen_test]
fn to_text_whole_document_or_matches() {
    let html = "<head><title>T</title></head><h1>Hello</h1><p>Some   <b>bold</b>\n text.</p>\
                <ul><li>a</li><li></li><li>b</li></ul><script>x()</script>";
    assert_eq!(
        to_text(html, None).unwrap(),
        "Hello\n\nSome bold text.\n\na\nb"
    );
    // Matches joined by a blank line; empty ones skipped.
    assert_eq!(to_text(html, Some("li".into())).unwrap(), "a\n\nb");
    assert_eq!(to_text(html, Some("table".into())).unwrap(), "");
}

#[wasm_bindgen_test]
fn to_markdown_whole_document_or_matches() {
    let html = "<h1>Title</h1><p>Some <b>bold</b> and a <a href='/x'>link</a>.</p>\
                <ul><li>one</li><li>two</li></ul>";
    assert_eq!(
        to_markdown(html, None).unwrap(),
        "# Title\n\nSome **bold** and a [link](/x).\n\n- one\n- two"
    );
    assert_eq!(
        to_markdown(html, Some("h1, li".into())).unwrap(),
        "# Title\n\n- one\n\n- two"
    );
}

#[wasm_bindgen_test]
fn to_html_repairs_markup() {
    assert_eq!(
        to_html("<ul><li>a<li>b</ul><p class=x>1 < 2"),
        r#"<ul><li>a</li><li>b</li></ul><p class="x">1 &lt; 2</p>"#
    );
}

#[wasm_bindgen_test]
fn errors_lists_positions_and_messages() {
    assert_eq!(js(&errors("<p>clean</p>")), "[]");
    assert_eq!(
        js(&errors("<b><i>x</b></i>")),
        r#"[{"position":7,"message":"end tag closed mis-nested elements"},{"position":11,"message":"stray end tag with no matching open element"}]"#
    );
    // Positions are JS string indices: "é" is 2 bytes but 1 UTF-16 unit, and
    // "😀" is 4 bytes but 2 units.
    let html = "<p>é😀</x>";
    let stray = js_sys::Array::from(&errors(html))
        .iter()
        .find(|e| get(e, "message").as_string().unwrap().starts_with("stray"))
        .unwrap();
    assert_eq!(get(&stray, "position").as_f64().unwrap(), 6.0); // html.indexOf("</x>") in JS
}

#[wasm_bindgen_test]
fn document_parses_once_and_queries_many_times() {
    let doc = Document::parse(&JsValue::from_str(LIST)).unwrap();
    assert_eq!(
        js(&doc.select("li").unwrap()),
        js(&select(LIST, "li").unwrap())
    );
    assert_eq!(
        js(&doc.select_first("#list > .item").unwrap()),
        js(&select_first(LIST, "#list > .item").unwrap())
    );
    assert!(doc.select_first("p").unwrap().is_undefined());
    assert_eq!(doc.to_text(None).unwrap(), "One\nTwo & more");
    assert_eq!(doc.to_text(Some("li".into())).unwrap(), "One\n\nTwo & more");
    assert_eq!(doc.to_markdown(None).unwrap(), "- One\n- Two & more");
    assert_eq!(doc.to_markdown(Some(".item".into())).unwrap(), "- One");
    assert_eq!(
        doc.to_html(),
        r#"<ul id="list"><li class="item">One</li><li>Two &amp; more</li></ul>"#
    );
    assert_eq!(js(&doc.tree()), parse_json(LIST));
    assert_eq!(js(&doc.to_json()), parse_json(LIST));
    assert_eq!(js(&doc.errors()), js(&errors(LIST)));
    assert_eq!(
        js(&Document::new(&JsValue::from_str("<p>x")).unwrap().errors()),
        r#"[{"position":4,"message":"element implicitly closed"}]"#
    );
}

#[wasm_bindgen_test]
fn strings_survive_the_json_round_trip() {
    let text = "quote \" backslash \\ tab \t nl \n ctrl \u{1} \u{1f} del \u{7f} \u{2028} 😀 é";
    let html = format!("<p title='{text}'>{text}</p>");
    let tree = parse(&JsValue::from_str(&html)).unwrap();
    let p = js_sys::Array::from(&get(&tree, "children")).get(0);
    assert_eq!(get(&get(&p, "attrs"), "title").as_string().unwrap(), text);
    let hit = select_first(&html, "p").unwrap();
    assert_eq!(get(&get(&hit, "attrs"), "title").as_string().unwrap(), text);
    let tab = js_sys::Array::from(&get(&p, "children")).get(0);
    assert_eq!(get(&tab, "value").as_string().unwrap(), text);
}

#[wasm_bindgen_test]
fn never_throws_on_garbage() {
    for html in [
        "",
        "<",
        "</",
        "<<>>",
        "<a b='",
        "&#xffffffff;",
        "<!--",
        "\u{0}<p\u{0}>",
    ] {
        let _ = parse(&JsValue::from_str(html));
        let _ = Document::new(&JsValue::from_str(html)).map(|d| d.errors());
        let _ = to_text(html, None).unwrap();
        let _ = to_markdown(html, None).unwrap();
        let _ = select(html, "*").unwrap();
        let _ = to_html(html);
    }
}

/// The extraction API only exists in the `extract` build.
#[cfg(feature = "extract")]
mod extract {
    use super::{get, js};
    use crate::{
        Document, links, main_content, metadata, resources, tables, to_csv, to_markdown_chunks,
    };
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn main_content_finds_article_and_returns_undefined_when_missing() {
        let html = "<html><head><title>Page</title></head><body>\
                    <nav><a href='/'>Home</a></nav>\
                    <article><h1>Release Notes</h1><p>domq now supports reader mode in WASM with full parity.</p></article>\
                    <footer>Footer text</footer></body></html>";
        let main = main_content(html);
        assert!(!main.is_undefined());
        assert!(
            get(&main, "text")
                .as_string()
                .unwrap()
                .contains("Release Notes")
        );
        assert!(
            get(&main, "markdown")
                .as_string()
                .unwrap()
                .contains("# Release Notes")
        );
        assert!(
            get(&main, "html")
                .as_string()
                .unwrap()
                .contains("<article>")
        );

        let doc = Document::new(&JsValue::from_str(html)).unwrap();
        let doc_main = doc.main_content();
        assert_eq!(js(&main), js(&doc_main));

        assert!(main_content("").is_undefined());
    }

    #[wasm_bindgen_test]
    fn metadata_extracts_fields_and_resolves_urls() {
        let html = "<html lang=\"en\"><head>\
                    <title>Document Title</title>\
                    <meta name=\"description\" content=\"Meta description text\">\
                    <link rel=\"canonical\" href=\"doc.html\">\
                    <meta property=\"og:title\" content=\"OG Title\">\
                    <meta name=\"twitter:card\" content=\"summary_large_image\">\
                    <script type=\"application/ld+json\">{\"@type\":\"Article\"}</script>\
                    <link rel=\"alternate\" type=\"application/rss+xml\" href=\"feed.xml\">\
                    </head><body><p>Body</p></body></html>";
        let base = "https://example.com/blog/";
        let meta = metadata(html, Some(base.into()));
        assert_eq!(get(&meta, "title").as_string().unwrap(), "Document Title");
        assert_eq!(
            get(&meta, "description").as_string().unwrap(),
            "Meta description text"
        );
        assert_eq!(
            get(&meta, "canonical").as_string().unwrap(),
            "https://example.com/blog/doc.html"
        );
        assert_eq!(get(&meta, "lang").as_string().unwrap(), "en");
        assert_eq!(get(&meta, "language").as_string().unwrap(), "en");

        let feeds = js_sys::Array::from(&get(&meta, "feeds"));
        assert_eq!(feeds.length(), 1);
        assert_eq!(
            feeds.get(0).as_string().unwrap(),
            "https://example.com/blog/feed.xml"
        );

        let json_ld = js_sys::Array::from(&get(&meta, "jsonLd"));
        assert_eq!(json_ld.length(), 1);
        assert_eq!(
            json_ld.get(0).as_string().unwrap(),
            "{\"@type\":\"Article\"}"
        );

        let doc = Document::new(&JsValue::from_str(html)).unwrap();
        assert_eq!(js(&meta), js(&doc.metadata(Some(base.into()))));
    }

    #[wasm_bindgen_test]
    fn links_and_resources_extract_and_resolve() {
        let html = "<div>\
                    <a href=\"sub/page.html\">Read more</a>\
                    <img src=\"images/logo.png\" alt=\"Logo\">\
                    <script src=\"/bundle.js\"></script>\
                    </div>";
        let base = "https://example.com/news/";
        let l = links(html, Some(base.into()));
        let link_arr = js_sys::Array::from(&l);
        assert_eq!(link_arr.length(), 1);
        let link0 = link_arr.get(0);
        assert_eq!(
            get(&link0, "url").as_string().unwrap(),
            "https://example.com/news/sub/page.html"
        );
        assert_eq!(get(&link0, "text").as_string().unwrap(), "Read more");
        assert_eq!(get(&link0, "href").as_string().unwrap(), "sub/page.html");

        let r = resources(html, Some(base.into()));
        let res_arr = js_sys::Array::from(&r);
        assert_eq!(res_arr.length(), 2);
        let img = res_arr.get(0);
        assert_eq!(
            get(&img, "url").as_string().unwrap(),
            "https://example.com/news/images/logo.png"
        );
        assert_eq!(get(&img, "attr").as_string().unwrap(), "src");
        assert_eq!(get(&img, "tag").as_string().unwrap(), "img");
        assert_eq!(get(&img, "value").as_string().unwrap(), "images/logo.png");

        let doc = Document::new(&JsValue::from_str(html)).unwrap();
        assert_eq!(js(&l), js(&doc.links(Some(base.into()))));
        assert_eq!(js(&r), js(&doc.resources(Some(base.into()))));
    }

    #[wasm_bindgen_test]
    fn to_markdown_chunks_splits_by_budget() {
        let html = "<h1>Main Heading</h1><p>First paragraph here.</p>\
                    <h2>Sub Heading</h2><p>Second paragraph with more detail.</p>";
        let chunks = to_markdown_chunks(html, Some(40));
        let arr = js_sys::Array::from(&chunks);
        assert!(arr.length() >= 2);
        let c0 = arr.get(0);
        assert!(!get(&c0, "markdown").as_string().unwrap().is_empty());
        assert!(!get(&c0, "text").as_string().unwrap().is_empty());
        assert!(get(&c0, "byteSize").as_f64().unwrap() > 0.0);

        let doc = Document::new(&JsValue::from_str(html)).unwrap();
        assert_eq!(js(&chunks), js(&doc.to_markdown_chunks(Some(40))));
    }

    #[wasm_bindgen_test]
    fn tables_and_csv_extraction() {
        let html = "<table>\
                    <thead><tr><th>Col A</th><th>Col B</th></tr></thead>\
                    <tbody><tr><td>1</td><td>2</td></tr></tbody>\
                    </table>";
        let t = tables(html, None).unwrap();
        assert_eq!(js(&t), r#"[[["Col A","Col B"],["1","2"]]]"#);

        let csv = to_csv(html, None).unwrap();
        assert_eq!(csv, "Col A,Col B\r\n1,2\r\n");

        let doc = Document::new(&JsValue::from_str(html)).unwrap();
        assert_eq!(js(&t), js(&doc.tables(None).unwrap()));
        assert_eq!(csv, doc.to_csv(None).unwrap());

        // Invalid selector throws SelectorError
        assert!(tables(html, Some("table >".into())).is_err());
        assert!(doc.tables(Some("table >".into())).is_err());
        assert!(to_csv(html, Some("table >".into())).is_err());
        assert!(doc.to_csv(Some("table >".into())).is_err());
    }
}

#[wasm_bindgen_test]
fn resolve_url_and_srcset_helpers() {
    assert_eq!(
        resolve_url("https://example.com/dir/", "page.html"),
        "https://example.com/dir/page.html"
    );
    assert_eq!(
        resolve_srcset("https://example.com/base/", "a.png 1x, b.png 2x"),
        "https://example.com/base/a.png 1x, https://example.com/base/b.png 2x"
    );
}

#[wasm_bindgen_test]
fn parse_accepts_bytes_and_uint8array() {
    let bytes = b"<p class='x'>hello bytes</p>";
    let arr = js_sys::Uint8Array::from(&bytes[..]);
    let tree = parse(&arr.into()).unwrap();
    assert_eq!(
        js(&tree),
        r#"{"type":"document","errors":0,"children":[{"type":"element","tag":"p","attrs":{"class":"x"},"children":[{"type":"text","value":"hello bytes"}]}]}"#
    );

    let doc = Document::new(&js_sys::Uint8Array::from(&bytes[..]).into()).unwrap();
    assert_eq!(doc.to_text(None).unwrap(), "hello bytes");

    let doc2 = Document::parse_bytes(bytes);
    assert_eq!(doc2.to_text(None).unwrap(), "hello bytes");

    let tree2 = parse_bytes(bytes);
    assert_eq!(js(&tree2), js(&tree));
}

#[wasm_bindgen_test]
fn decode_returns_js_object() {
    let res = decode(b"<p>caf\xc3\xa9</p>", None);
    assert_eq!(get(&res, "text").as_string().unwrap(), "<p>café</p>");
    assert_eq!(get(&res, "encoding").as_string().unwrap(), "utf-8");
    assert!(get(&res, "warning").is_null());

    let res_win = decode(b"<meta charset=latin1>\xe9", None);
    assert_eq!(
        get(&res_win, "encoding").as_string().unwrap(),
        "windows-1252"
    );

    let res_stray = decode(b"<p>caf\xc3\xa9</p>\xa0", None);
    assert_eq!(get(&res_stray, "encoding").as_string().unwrap(), "utf-8");
    assert_eq!(
        get(&res_stray, "warning").as_string().unwrap(),
        "1 invalid UTF-8 sequence replaced with U+FFFD"
    );

    let res_latin1 = decode(b"<p>caf\xe9</p>", None);
    assert_eq!(
        get(&res_latin1, "encoding").as_string().unwrap(),
        "windows-1252"
    );
    assert!(
        get(&res_latin1, "warning")
            .as_string()
            .unwrap()
            .contains("windows-1252")
    );
}
