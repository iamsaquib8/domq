# Measured: conformance and reader mode

domq publishes numbers instead of adjectives. Two reports are generated from
external test suites and checked in CI, so every change to the parser or the
reader-mode heuristic shows its effect in the pull request.

## How close the trees are to a browser's

[docs/conformance.md](https://github.com/iamsaquib8/domq/blob/main/docs/conformance.md)
runs the **1,959 HTML tree-construction tests** (the html5lib tests, now
maintained in web-platform-tests) against domq. Each test is scored twice:

- **exact**: the tree, dumped in the html5lib format, matches line for line.
  This is low by design, since most expected trees start with the `<html>`,
  `<head>` and `<body>` elements browsers insert and domq does not;
- **same structure**: the trees match once those implied wrappers,
  namespaces and whitespace-only text are set aside. A failure here is a real
  difference from a browser.

There are **0 panics** on the whole suite. The report breaks the results down
by area (character references, tables, mis-nested formatting, foreign
content, …) and explains the gaps that are there by design.

## How well reader mode finds the article

[docs/reader-quality.md](https://github.com/iamsaquib8/domq/blob/main/docs/reader-quality.md)
scores `main_content()` against **human-labelled ground truth** on the two
public benchmarks other extractors publish: the Zyte/ScrapingHub article
extraction benchmark (AEB, 181 articles) and WCXB (2,008 pages in seven page
types, with a held-out test split), each with its own scoring, next to the
published scores of trafilatura, Readability.js, dom_smoothie and others.
Today: **0.900 F1 on AEB** and **0.792 / 0.830 on WCXB** (dev / test), at a
few milliseconds per page.

[docs/reader-mode.md](https://github.com/iamsaquib8/domq/blob/main/docs/reader-mode.md)
measures something else: *agreement* with Readability (Firefox's reader view)
on Readability's own 130 test pages, plus `metadata()` field by field.

## Run them yourself

```sh
sh conformance/fetch.sh     # pinned WPT, Readability, AEB and WCXB commits, sparse fetch
cargo run --release --manifest-path conformance/Cargo.toml
cargo run --release --manifest-path conformance/Cargo.toml -- --failures tables01.dat
cargo run --release --manifest-path conformance/Cargo.toml --bin reader -- --explain engadget
```

## Other checks

Property tests ("never panics" and tree invariants), snapshot tests over a
corpus of messy pages, fuzzing (`parse`, `select`, `markdown`, and a
differential target against html5ever on the subset domq claims to handle like
a browser), `no_std` and MSRV builds, a zero-dependency check, and a size
budget for the wasm module.
