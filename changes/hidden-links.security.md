`Dom::links` and `NodeRef::links` no longer return links hidden by their own
`hidden` attribute or inline `display: none` / `visibility: hidden` style,
or links beneath a hidden ancestor. This keeps hidden navigation and
potential prompt-injection links out of CLI, MCP and binding results while
preserving visible links (including `aria-hidden` elements).
