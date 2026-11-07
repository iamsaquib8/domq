# Changelog fragments

Each user-visible change adds one file here, so pull requests never edit the
same lines of `CHANGELOG.md` and stop conflicting with each other. The
maintainer assembles the fragments into the changelog when a release is cut.

Name the file `<slug>.<kind>.md`, for example `parser-stray-lt-text.output.md`.
The slug is a few words about the change (an issue number works too). The kind
is one of:

| kind | heading | use it for |
|---|---|---|
| `added` | Added | new API, flags, tools |
| `changed` | Changed | behaviour you must know about that is not an output change |
| `output` | Output changes | anything that changes `to_text`, `to_markdown`, chunks, what `main_content()` picks, or the parsed tree: re-embed if exact text matters |
| `deprecated` | Deprecated | things that will be removed |
| `removed` | Removed | things that were removed |
| `fixed` | Fixed | bug fixes |
| `performance` | Performance | speed and memory |
| `security` | Security | vulnerabilities and hardening |

The file holds the entry's text: plain sentences about what a user sees,
without a leading `- ` and without headings. It may span several lines.

```text
CLI: `-1` keeps the first result that survives `-a` and `-w` filtering in
every mode, and `-c` honours `-w` ([#144](https://github.com/iamsaquib8/domq/issues/144)).
```

Not every change needs one: tests, refactors, CI and docs changes with no
effect on users do not.

```sh
python3 scripts/changelog.py check                     # validate the fragments
python3 scripts/changelog.py preview                   # the next release's notes
python3 scripts/changelog.py release 0.7.0 2026-12-01  # write the section, delete the fragments
```

`release` puts `## [0.7.0] — 2026-12-01` below `## [Unreleased]`, grouped in the
order above. Add the release's summary paragraph (and the "Upgrading from"
note) under the new heading by hand, then commit.
