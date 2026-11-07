# Replace htmlq and pup in shell scripts

**Goal.** Swap `htmlq` or `pup` for `domq` in a script, keep the script working,
and use what `domq` adds: field extraction as JSON, Markdown, a count, and
exit codes you can test.

**Input.** [shop.html](../sample/shop.html). The examples read it from stdin;
in a script it is usually `curl -sL "$url" | domq ...` or `domq -f page.html ...`.

## The same job, three tools

| Job | htmlq | pup | domq |
|---|---|---|---|
| An attribute | `htmlq -a href a` | `pup 'a attr{href}'` | `domq -a href a` |
| Text | `htmlq -t h2` | `pup 'h2 text{}'` | `domq -t h2` |
| Pretty-printed HTML | `htmlq -p article` | `pup 'article'` | `domq -p article` |
| Remove nodes first | `htmlq -r script body` | - | `domq -r script body` |
| Absolute URLs | `htmlq -b URL -a href a` | - | `domq -b URL -a href a` |
| JSON | - | `pup 'li json{}'` | `domq -j li` |
| Read a file, write a file | `htmlq -f in.html -o out.txt` | - | `domq -f in.html -o out.txt` |

The `htmlq` column is `htmlq`'s own flags, and `domq` takes them unchanged
([From htmlq](../migrating/htmlq.md) has the details). The `pup` column is
`pup`'s documented syntax: `pup` is not installed where these pages were
written, so only the `domq` column was run.

## What a script does with it

Each command below was run on `shop.html`.

**Every product link, as an absolute URL:**

```sh
domq -a href -b https://shop.example/gadgets '.product a' < shop.html
```

```text
https://shop.example/gadgets/widget
https://shop.example/gadgets/gizmo
https://shop.example/gadgets/doohickey
```

**Fields as JSON Lines, one object per product (`-e`):**

```sh
domq '.product' -e name=h2 -e price=.price@data-value --jsonl < shop.html
```

```text
{"name":"Widget","price":"9.5"}
{"name":"Gizmo","price":"12"}
{"name":"Doohickey","price":"24.99"}
```

Each `-e NAME=SPEC` is a selector (the text of its first match),
`selector@attr` (an attribute), or either with a trailing `[]` for every match.
An empty selector means the scope element itself, so `-e sku=@data-sku` reads
the product's own attribute. The selector argument (`.product`) is the scope:
one object per match.

**Fields as CSV, with `jq`:**

```sh
domq '.product' -e sku=@data-sku -e name=h2 -e price=.price@data-value --jsonl < shop.html \
  | jq -r '[.sku, .name, .price] | @csv'
```

```text
"W-1","Widget","9.5"
"G-2","Gizmo","12"
"D-3","Doohickey","24.99"
```

**A loop over the results:**

```sh
domq -a href '.product a' < shop.html | while read -r href; do
  echo "would fetch https://shop.example$href"
done
```

```text
would fetch https://shop.example/gadgets/widget
would fetch https://shop.example/gadgets/gizmo
would fetch https://shop.example/gadgets/doohickey
```

Avoid naming that variable `path` in a script that may run under `zsh`: there,
`path` is tied to `PATH`.

**A count, and whether anything matched:**

```sh
domq -c '.product' < shop.html
# 3
if domq --exit-status -c '.out-of-stock' < shop.html > /dev/null; then
  echo "some out of stock"
fi
# some out of stock
if ! domq --exit-status '.coupon' < shop.html > /dev/null; then
  echo "no coupon"
fi
# no coupon
```

**The main part as Markdown, with the navigation removed first:**

```sh
domq -r script -r nav -1 -m main < shop.html
```

```text
# Gadgets

- [![Widget](/img/widget.jpg)](/gadgets/widget)

  ## Widget

  $9.50 new blue
- [![Gizmo](/img/gizmo.jpg)](/gadgets/gizmo)

  ## Gizmo

  $12
- [![](/img/doohickey.jpg)](/gadgets/doohickey)

  ## Doohickey

  $24.99 sale

## Specifications

| Product | Weight (g) | Warranty |
| --- | --- | --- |
| Widget | 120 | 2 years |
| Gizmo | 85 | |
| Doohickey | 300 | 1 year |
```

(In Markdown the cell a `rowspan` covers is left empty; [`--csv`](crawl-to-table.md)
repeats the value.)

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success. This includes **no match**: `domq '.coupon' < shop.html` prints nothing and exits 0, and `domq -c '.coupon'` prints `0`. |
| 1 | An invalid or unsupported selector, an I/O error (unreadable file), or no match when `--exit-status` is set. |
| 2 | Bad usage, such as an unknown flag. |

```text
$ domq 'a:hover' < shop.html
domq: invalid selector: unsupported pseudo-class ':hover'
  a:hover
   ^
$ echo $?
1
$ domq --bogus < shop.html
domq: unknown option '--bogus'
Try 'domq --help' for more information.
$ echo $?
2
```

`--exit-status` is for `if domq --exit-status SELECTOR`. Without it, an empty
result is not a failure, so a script that must stop when a selector finds
nothing needs the flag. An unsupported selector is an error, never a
silently empty result, so a typo does not look like "nothing found".

In a pipeline (`curl ... | domq ...`), set `set -o pipefail` if you want a
failing `curl` or `domq` to fail the script.

## What can go wrong

- **`-t` is readable text, not raw text nodes.** Paragraphs sit on their own
  lines, `<script>` and `<style>` are dropped, whitespace is collapsed.
  A script that parsed `htmlq -t` output by its exact spacing may need a
  second look.
- **No selector means the whole document**, where `htmlq` means the `html`
  element. Fragments such as `<li>a<li>b` work.
- **`pup` selectors are not all CSS.** Its `:parent-of()` and similar filters
  have no equivalent here; most plain CSS works unchanged. Selectors domq
  does not support (`:hover`, `:focus`, `::before`) are an error with the
  position.
- **Pages that need JavaScript** to show their data have nothing in the HTML
  for domq to read.
- **Encoding.** Input is decoded like a browser does it: byte-order mark,
  `<meta charset>`, then UTF-8 if valid, then Windows-1252. Force one with
  `--encoding utf-8` when a page lies.
- **An element called `<mcp>`.** `domq mcp` starts the MCP server; write
  `domq -- mcp` to select the element.

## Reference

[Command line](../cli.md) (every flag), [Selectors](../selectors.md),
[From htmlq](../migrating/htmlq.md), [Pull fields from a list of
items](extracting.md).
