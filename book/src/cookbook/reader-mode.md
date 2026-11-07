# Turn a page into Markdown for an LLM

Raw HTML wastes a model's context on markup, navigation and scripts. domq
turns a page into the article as clean Markdown, or into chunks ready for an
embedding model.

## The article, as Markdown

```rust
# extern crate domq;
let dom = domq::parse(r#"<body>
  <nav><a href="/">Home</a> <a href="/blog">Blog</a></nav>
  <article>
    <h1>Arena trees</h1>
    <p>An arena stores every node in one vector, and links them by index.</p>
    <p>Traversal is cheap, removal is a flag, and nothing is reference-counted.</p>
  </article>
  <footer>© 2026</footer>
</body>"#);

let article = dom.main_content().expect("an article");
let md = article.to_markdown();
assert!(md.starts_with("# Arena trees"));
assert!(!md.contains("Home"));
```

`main_content()` is a small, deterministic cousin of Mozilla's Readability:
semantic hints first (a Schema.org `articleBody`, a single `<article>` or
`<main>`), then paragraph scoring with boilerplate (navigation, sidebars,
comments, share bars) discounted. It returns `None` when no container stands
out; use the whole document then. Its agreement with Firefox's reader view is
[measured](../measured.md) on 130 real pages.

From the shell, and in JavaScript:

```sh
curl -sL https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/ | domq --main --markdown
```

```js
import { toMarkdown } from "domq-wasm";
toMarkdown(html, "article");   // or toMarkdown(html) for the whole page
```

### Configuring Markdown output

Markdown output can be tuned with `MarkdownOptions`:

```rust
# extern crate domq;
# let dom = domq::parse("<article><a href=\"/docs\">Docs</a></article>");
let opts = domq::MarkdownOptions::default()
    .link_style(domq::LinkStyle::Reference)
    .image_style(domq::ImageStyle::AltOnly)
    .drop_data_uris(true);
let md = dom.to_markdown_with(&opts);
```

Link styles include `LinkStyle::Inline` (default), `LinkStyle::Reference` (`[text][1]`), and `LinkStyle::Footer` (`### Links` list at document end). Image styles include `ImageStyle::Keep`, `ImageStyle::AltOnly`, and `ImageStyle::Drop`.

## The article, as plain text

When you want the words and nothing else (to search them, count them or show
them), ask reader mode for text instead of Markdown. With the page above in
`arena.html`:

<div class="domq-tabs">

**Rust**

```rust
# extern crate domq;
# let dom = domq::parse(r#"<body>
#   <nav><a href="/">Home</a> <a href="/blog">Blog</a></nav>
#   <article>
#     <h1>Arena trees</h1>
#     <p>An arena stores every node in one vector, and links them by index.</p>
#     <p>Traversal is cheap, removal is a flag, and nothing is reference-counted.</p>
#   </article>
#   <footer>© 2026</footer>
# </body>"#);
let text = dom.main_content().map(|a| a.to_text()).unwrap_or_else(|| dom.to_text());
assert_eq!(
    text,
    "Arena trees\n\nAn arena stores every node in one vector, and links them by index.\n\n\
     Traversal is cheap, removal is a flag, and nothing is reference-counted."
);
```

**CLI**

```sh
domq --main -t < arena.html
```

**JavaScript**

```js
import { mainContent } from "domq-wasm/extract";

mainContent(html)?.text;
```

**Python**

```python
import domq

article = domq.parse(html).main_content()
article.to_text() if article else None
```

</div>

All four give this text:

```text
Arena trees

An arena stores every node in one vector, and links them by index.

Traversal is cheap, removal is a flag, and nothing is reference-counted.
```

When no container stands out, `main_content()` gives `None` (`undefined` in
JavaScript, and the CLI keeps the whole page): fall back to the whole
document, as the Rust line above does.

## Chunks for embeddings

```rust
# extern crate domq;
let dom = domq::parse(
    "<h1>Guide</h1><p>Welcome.</p><h2>Install</h2><p>Run the installer.</p>",
);
for chunk in dom.to_markdown_chunks(500) {
    // `context()` is the heading path, e.g. "Guide > Install": prepend it
    // when embedding so every chunk stands on its own.
    println!("{} | {}", chunk.context(), chunk.markdown);
}
```

Chunks split at blank lines (never inside a code block), start new sections at
headings, and carry the headings they sit under.

## What gets dropped

`to_text` and `to_markdown` leave out what a reader would not see: `<head>`,
`<script>`, `<style>`, `<template>`, and elements hidden with the `hidden`
attribute or an inline `display: none` / `visibility: hidden`. Hidden text is
a favourite place for prompt injections aimed at LLM agents, so it never
reaches your model.

## Agents

An agent with the MCP server calls `markdown` with `main: true`, or `chunks`,
on the HTML it fetched. See [AI agents (MCP)](../mcp.md).
