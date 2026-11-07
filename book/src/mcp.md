# AI agents (MCP)

`domq mcp` runs the CLI as a [Model Context Protocol](https://modelcontextprotocol.io)
server over stdio. An agent that has fetched a page hands its HTML to domq
and gets back what it needs (the article as Markdown, a list of fields, the
links) instead of spending its context on raw markup.

## Set up

```sh
claude mcp add domq -- domq mcp        # Claude Code
codex mcp add domq -- domq mcp         # Codex CLI
```

For Claude Desktop, Cursor, VS Code, Windsurf and most other clients, add it
to the MCP configuration:

```json
{ "mcpServers": { "domq": { "command": "domq", "args": ["mcp"] } } }
```

The WASI build works too: `"command": "wasmtime", "args": ["domq.wasm", "mcp"]`.

## Tools

| Tool | Arguments (besides `html`) | Returns |
|---|---|---|
| `markdown` | `selector`, `main`, `base_url`, `links`, `images`, `drop_data_uris` | the page, or each match, as Markdown |
| `text` | `selector`, `main` | readable text |
| `outline` | `selector` | `{count, outline: [{ref, role, name, level, selector, href, tag, depth, source_range}]}` |
| `select` | `selector`, `limit` (50), `include_html`, `max_chars` (1000) | `{count, truncated, matches: [{tag, attrs, text, html?}]}` |
| `extract` | `fields` (name → spec, as in `domq -e`), `selector`, `base_url` | `{results: [{…}]}`, one object per match of `selector` |
| `chunks` | `max_chars` (2000), `selector`, `main`, `base_url`, `links`, `images`, `drop_data_uris` | `{chunks: [{context, heading_path, markdown}]}` |
| `links` | `selector`, `base_url` | `{count, links: [{url, text}]}` |
| `metadata` | `base_url` | title, description, canonical, OpenGraph, JSON-LD, feeds, … |
| `tables` | `selector` (default `table`) | `{tables: [{rows: [[cell, …]]}]}` |
| `lint` | `include_notes` | `{errors, warnings, notes, findings: [{line, column, level, rule, message}]}` |

`main: true` keeps only the article ([reader mode](cookbook/reader-mode.md));
`base_url` makes relative links absolute. A worked example, from `outline` to
`extract`, is in [Give an agent a page](cookbook/agent-page.md).

## Safety

- Every tool is **read-only** and **closed-world**: it works on the `html`
  argument alone and never fetches URLs, reads files or runs scripts.
- Text and Markdown output leave out content a browser never shows:
  elements with `hidden`, inline `display: none` or `visibility: hidden`,
  `<template>`, comments, scripts and styles. Text hidden in other ways
  (CSS classes, `opacity: 0`, off-screen positioning, a zero font size) is
  **not** detected, so treat page text as untrusted input either way.
- Invalid selectors and arguments come back as tool errors whose message
  points at the problem, so the model can correct itself.

## Protocol

The server speaks MCP **2026-07-28** (per-request `_meta`, `server/discover`,
no handshake) and the handshake-based revisions **2025-11-25**, 2025-06-18,
2025-03-26 and 2024-11-05, on the same connection. JSON results are returned
as `structuredContent` with an `outputSchema`, and as text for clients that
only read text. It has no dependencies: the CLI carries its own small JSON
parser.

To query an element named `<mcp>` from the shell, write `domq -- mcp`.
