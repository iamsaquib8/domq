# Command line

`domq` reads HTML from stdin (or `-f FILE`) and prints what a CSS selector
matches. Its flags are compatible with [`htmlq`](migrating/htmlq.md), with
Markdown, JSON Lines, CSV, field extraction, reader mode, links, metadata,
encoding detection, a linter and an MCP server on top.

```sh
curl -s https://example.com | domq 'a' -a href
domq -f page.html -t 'article h2'
curl -sL https://example.com/post | domq --main --markdown
domq '.card' -e name=h2 -e url=a@href --jsonl < list.html
domq --csv -1 < report.html > report.csv
domq lint index.html docs/*.html
claude mcp add domq -- domq mcp
```

This page is the output of `domq --help`, `domq lint --help` and
`domq mcp --help`.

## `domq`

```text
domq — query HTML with CSS selectors

USAGE:
    domq [OPTIONS] [SELECTOR]...
    domq lint [OPTIONS] [FILE]...    (see 'domq lint --help')
    domq mcp                         MCP server for AI agents (see 'domq mcp --help')

    Reads HTML from stdin (or --filename) and prints every element matching
    SELECTOR. Several SELECTOR arguments are joined with spaces. With no
    selector, the whole document is used.

EXAMPLES:
    curl -s https://example.com | domq 'a' -a href
    domq -f page.html -t 'article h2'
    curl -sL https://example.com/post | domq --main --markdown
    domq -e title=h1 -e price=.price -e 'tags=.tag[]' < product.html
    domq '.card' -e name=h2 -e url=a@href --jsonl < list.html
    domq --csv -1 < report.html > report.csv
    domq -c 'img:not([alt])' < page.html
    domq lint index.html docs/*.html

OUTPUT (default: matching elements as HTML, one per line; -t, -m, -j, --jsonl,
--csv and -c each pick the format, so give at most one):
    -a, --attribute <NAME>      Print this attribute of each match (skips matches
                                without it). With no selector, searches all elements.
    -t, --text                  Print readable text of each match
    -m, --markdown              Print each match as Markdown
    -j, --json                  Print matches as a JSON array (document JSON with
                                no selector)
        --jsonl                 Print one JSON value per line (JSON Lines)
        --csv                   Print matched <table>s as CSV (default selector:
                                table), separated by a blank line
    -c, --count                 Print the number of matches
    -p, --pretty                Pretty-print HTML output
    -1, --first                 Only use the first match
    -w, --ignore-whitespace     Skip matches whose output is empty or whitespace
                                (with --links, links without text)
        --exit-status           Exit 1 when nothing matches

EXTRACT:
    -e, --extract <NAME=SPEC>   Print a JSON object with a NAME key (repeat it);
                                SPEC is SELECTOR (text of the first match) or
                                SELECTOR@ATTR (its attribute); end it with []
                                for an array of all matches. An empty SELECTOR
                                is the scope element itself. With a SELECTOR
                                argument, prints one object per match of it, as
                                a JSON array (or JSON Lines with --jsonl).
        --links                 Print every link as URL<TAB>text (JSON objects
                                with --json / --jsonl); a SELECTOR keeps the
                                links inside its matches
        --meta                  Print the page metadata (title, description,
                                canonical, OpenGraph, JSON-LD, ...) as JSON

DOCUMENT:
    -r, --remove-nodes <SEL>    Remove nodes matching SEL before selecting
                                (repeatable)
        --main                  Keep only the main content (the article, without
                                navigation, sidebars and footer)
    -b, --base <URL>            Resolve relative href/src/... URLs against URL
    -B, --detect-base           Use the document's <base href>, itself resolved
                                against --base (falls back to --base)

INPUT / OUTPUT:
    -f, --filename <FILE>       Read from FILE instead of stdin ('-' = stdin)
    -o, --output <FILE>         Write to FILE instead of stdout
        --encoding <LABEL>      Decode the input as LABEL: utf-8, utf-16le,
                                utf-16be or windows-1252 (default: detected from
                                a BOM, then <meta charset>, then UTF-8 validity,
                                falling back to windows-1252)

    -h, --help                  Print this help
    -V, --version               Print the version

Selectors: type, .class, #id, [attr op value], descendant/child/sibling
combinators, :nth-child(), :not(), :is(), :has(), :contains("text"), and more.
In --extract SPECs only the last @NAME and a trailing [] are special; write \@
for a literal @ in a selector.

Exit codes: 0 success (also when nothing matches, unless --exit-status is set),
1 invalid selector, I/O error or no matches with --exit-status, 2 bad usage.
Docs: https://github.com/iamsaquib8/domq
```

## `domq lint`

See also [Lint HTML in CI](cookbook/lint.md).

```text
domq lint — check HTML for common accessibility and markup problems

USAGE:
    domq lint [OPTIONS] [FILE]...

    Checks each FILE (stdin when none are given, or for '-') and prints one
    line per finding, in document order:

        FILE:LINE:COLUMN: LEVEL: MESSAGE [RULE]

RULES:
    img-alt             error    <img> without alt (alt="" marks a decorative
                                 image; aria-label, title or role=presentation
                                 also count)
    empty-link          error    <a href> / <area href> without text,
                                 aria-label, title or image alt
    empty-button        error    <button> / <input type=button> without a name
    duplicate-id        error    an id that is already used (reported where it
                                 repeats, naming the first use)
    heading-order       warning  a skipped heading level (<h4> after <h2>)
    unclosed-element    warning  an element whose end tag HTML requires was
                                 closed by another tag, or never closed
    mismatched-end-tag  warning  an end tag that also closed such elements
                                 (<b><i>x</b>)
    stray-end-tag, unterminated-tag, unterminated-comment, unterminated-cdata,
    unexpected-eof, bogus-comment
                        warning  other markup the parser had to recover from
    implicitly-closed   note     an omitted optional end tag (</li>, </p>,
                                 </td>, ...): valid HTML
    unknown-entity      note     an &name; or &#... that is no character
                                 reference, left as written
    missing-semicolon   note     a character reference without its ';', decoded
                                 anyway (&copy 2026; also a bare & in a URL
                                 in text: ?a=1&copy=2 reads as ?a=1©=2)

OPTIONS:
        --format <FORMAT>       text (default) or json: one JSON array of
                                {file, line, column, level, rule, message}
    -q, --quiet                 Do not print notes
        --encoding <LABEL>      Decode the input as LABEL (see 'domq --help')
    -h, --help                  Print this help

EXIT CODES:
    0  no errors or warnings (notes do not count)
    1  errors or warnings found
    2  bad usage, or a FILE could not be read
```

## `domq mcp`

See also [AI agents (MCP)](mcp.md).

```text
domq mcp — an MCP server for AI agents, over stdio

USAGE:
    domq mcp

    Speaks the Model Context Protocol on stdin/stdout, so an agent can parse
    and query HTML it already has: the tools take the HTML as an argument and
    never fetch URLs or touch files. Supports protocol 2026-07-28 and the
    handshake-based revisions 2025-11-25 back to 2024-11-05.

TOOLS (all read-only):
    markdown    the page (or each match of a selector) as Markdown
    text        readable text
    select      elements matching a CSS selector: tag, attributes, text
    extract     named fields as JSON, one object per item (like -e)
    chunks      Markdown chunks with heading paths, for RAG
    links       links with resolved URLs and text
    metadata    title, description, OpenGraph, JSON-LD, feeds, ...
    tables      tables as rows of cells
    lint        accessibility and markup problems, with positions

SETUP:
    Claude Code:   claude mcp add domq -- domq mcp
    Codex CLI:     codex mcp add domq -- domq mcp
    JSON config (Claude Desktop, Cursor, VS Code, Windsurf, ...):
        { "mcpServers": { "domq": { "command": "domq", "args": ["mcp"] } } }

    To query an element named <mcp>, write 'domq -- mcp'.
```
