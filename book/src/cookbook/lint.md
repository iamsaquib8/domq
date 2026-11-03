# Lint HTML in CI

`domq lint` checks HTML for common accessibility and markup problems and
prints one line per finding, in the `file:line:col` format editors and CI
problem matchers understand:

```console
$ domq lint public/**/*.html
public/post.html:96:34: error: <a> link has no text or accessible name [empty-link]
public/post.html:290:13: warning: heading level skipped: <h4> after <h2> [heading-order]
```

It exits with 1 when it finds errors or warnings (notes never fail a run),
and 2 on bad usage or an unreadable file.

| Rule | Level | Finds |
|---|---|---|
| `img-alt` | error | `<img>` without `alt` (`alt=""` marks a decorative image) |
| `empty-link` | error | a link without text, `aria-label`, `title` or image `alt` |
| `empty-button` | error | a button without an accessible name |
| `duplicate-id` | error | an `id` used twice |
| `heading-order` | warning | a skipped heading level |
| `unclosed-element` | warning | an element whose required end tag is missing |
| `mismatched-end-tag` | warning | an end tag that closed mis-nested elements |
| `stray-end-tag`, `unterminated-tag`, `bogus-comment`, … | warning | other markup the parser recovered from |
| `implicitly-closed` | note | an omitted optional end tag (valid HTML) |
| `unknown-entity` | note | an `&name;` that is no character reference |
| `missing-semicolon` | note | a character reference without its `;` (often a URL pasted into text) |

`-q` hides notes; `--format json` prints a JSON array of
`{file, line, column, level, rule, message}`.

## GitHub Actions

Turn the JSON report into annotations on the pull request's lines:

```yaml
- name: Lint the built site
  shell: bash   # with pipefail, so findings fail the step
  run: |
    curl -sSfL https://github.com/iamsaquib8/domq/releases/download/v0.6.0/domq-v0.6.0-x86_64-unknown-linux-musl.tar.gz | tar xz
    ./domq-v0.6.0-x86_64-unknown-linux-musl/domq lint -q --format json public/*.html \
      | jq -r '.[] | "::\(if .level == "error" then "error" else "warning" end) file=\(.file),line=\(.line),col=\(.column)::\(.message) [\(.rule)]"'
```

A reusable action, SARIF output and per-rule `--allow`/`--deny` are on the
[roadmap](https://github.com/iamsaquib8/domq/blob/main/ROADMAP.md).
