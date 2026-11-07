# Edit and serialize

A parsed `Dom` can be rewritten in place and serialized back to HTML. Nodes
are addressed by `NodeId` (`node.id()`); removing a node detaches it, so ids
stay valid.

```rust
# extern crate domq;
let mut dom = domq::parse(r#"<div id="post"><p class="ad">Buy!</p><p>Text <b>bold</b></p>
  <a href="/x">link</a></div>"#);

// Remove every ad.
let ads: Vec<_> = dom.select(".ad").map(|n| n.id()).collect();
for id in ads {
    dom.remove(id);
}

// Rewrite attributes and text, rename and wrap elements.
let link = dom.select_first("a").unwrap().id();
dom.set_attr(link, "rel", "nofollow");
let b = dom.select_first("b").unwrap().id();
dom.rename(b, "strong");

// Insert parsed HTML.
let post = dom.find_by_id("post").unwrap().id();
dom.append_html(post, "<footer>Updated</footer>");

assert_eq!(
    dom.to_html(),
    "<div id=\"post\"><p>Text <strong>bold</strong></p>\n  \
     <a href=\"/x\" rel=\"nofollow\">link</a><footer>Updated</footer></div>"
);
```

Also available: `remove_attr`, `set_text`, `create_element`, `create_text`,
`append_child`, `insert_before`, `wrap`, `unwrap`, `prepend_html`,
`insert_html_before` and `insert_html_after`.

Serialization: `dom.to_html()` (the repaired document: every element closed
and properly nested), `node.outer_html()` and `node.inner_html()`,
`dom.to_json()` for a JSON tree. Text inside `<script>`, `<style>` and other
raw-text elements is written so it cannot close its element early.

From the shell, `domq -r 'script, .ad' -p` removes nodes and pretty-prints the
rest. A complete cleaning pass (remove, absolute URLs, strip attributes, round
trip) is in [Clean HTML before storing it](clean-html.md).
