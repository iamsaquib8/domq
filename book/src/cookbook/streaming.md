# Process a huge dump without building a tree

`domq::events` streams a document as start, end, text, comment and doctype
events without building a tree. The events come *after* recovery, from the
same tree builder `parse` uses, so every start has exactly one end and
replaying them builds exactly the tree `parse` would. Memory grows with the
nesting depth, not the document size.

```rust
# extern crate domq;
use domq::Event;

let html = "<ul><li>Tom &amp; Jerry<li>Spike<br></ul>";
let mut items = Vec::new();
for (event, _range) in domq::events(html) {
    match event {
        Event::Start { name, .. } if name == "li" => items.push(String::new()),
        Event::Text(text) => {
            if let Some(item) = items.last_mut() {
                item.push_str(&text);
            }
        }
        _ => {}
    }
}
assert_eq!(items, ["Tom & Jerry", "Spike"]);
```

Each event carries its byte range in the input; implied end tags have an empty
range. For raw tokens before any recovery, use `domq::tokenize`.

Running CSS selectors directly over the event stream (for multi-gigabyte
dumps) is on the [roadmap](https://github.com/iamsaquib8/domq/blob/main/ROADMAP.md).
