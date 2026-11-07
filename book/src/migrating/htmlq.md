# From htmlq

`domq` is a drop-in replacement for [`htmlq`](https://github.com/mgdm/htmlq):
the shared flags have the same names and meaning, so existing scripts keep
working.

```sh
# htmlq                                   # domq
htmlq -a href 'a'          < page.html    domq -a href 'a'          < page.html
htmlq -t '.title'          < page.html    domq -t '.title'          < page.html
htmlq -p 'article'         < page.html    domq -p 'article'         < page.html
htmlq -r 'script' 'body'   < page.html    domq -r 'script' 'body'   < page.html
htmlq -b https://x.com -a href a          domq -b https://x.com -a href a
```

| htmlq | domq | Notes |
|---|---|---|
| `-a`, `--attribute` | same | |
| `-t`, `--text` | same | prints *readable* text: paragraphs on their own lines, scripts and styles dropped |
| `-p`, `--pretty` | same | |
| `-w`, `--ignore-whitespace` | same | |
| `-r`, `--remove-nodes` | same | |
| `-b`, `--base`, `-B`, `--detect-base` | same | also resolves `srcset` |
| `-f`, `--filename`, `-o`, `--output` | same | |
| no selector: the `html` element | the whole document | so fragments work |

## What you gain

```sh
domq -m 'article'           # Markdown
domq -j 'li' / --jsonl      # JSON / JSON Lines
domq --csv -1 'table'       # tables as CSV
domq -c 'img:not([alt])'    # count
domq -1 'h1'                # first match only
domq '.card' -e title=h2 -e url=a@href   # fields as JSON
domq --main --markdown      # reader mode
domq --links / --meta       # links, page metadata
domq lint *.html            # accessibility and markup checks
domq mcp                    # MCP server for AI agents
```

Input encodings are detected like a browser does (BOM, `<meta charset>`,
UTF-8, then Windows-1252), or set with `--encoding`. Selectors add `:has()`,
`:is()`, `:where()`, `:nth-child(An+B of S)` and `:contains()`, and an
unsupported selector is an error instead of an empty result.

The binary is a single static file with no dependencies, also available as a
WASI module.

For scripts, with exit codes and extraction side by side, see [Replace htmlq
and pup in shell scripts](../cookbook/shell-scripts.md).
