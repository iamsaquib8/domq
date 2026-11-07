After edits, `to_text()`, `to_markdown()`, `links()` and `main_content()` decide
whether `<noscript>` holds the page's content from the document as it is now. The
decision was made at parse time and went stale: a document edited into some shape
(`insert_html_after`, `append_html`, `create_element("noscript")`, `rename`,
`set_text`, `remove`, moves) read differently from `parse(edited.to_html())`. Each
edit that created a `<noscript>` also walked the whole document; creating 1,000 of
them on a 644 KB document took 1.2 s and now takes under a millisecond
([#70](https://github.com/iamsaquib8/domq/issues/70)). Parsed documents are unaffected.
