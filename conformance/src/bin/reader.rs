//! Score domq's reader mode against Mozilla Readability's labelled pages.
//!
//! Each page in Readability's `test/test-pages/` has the original page
//! (`source.html`), the article Readability extracts from it
//! (`expected.html`) and the metadata it finds (`expected-metadata.json`);
//! `fetch.sh` downloads them at the commit pinned in `READABILITY_COMMIT`.
//!
//! * **Main content**: the words of `Dom::main_content()` (the whole page when
//!   it returns `None`, as `domq --main` does) against the words of the
//!   expected article, both as `to_text`. Words are runs of letters and
//!   digits, lower-cased, compared as bags: precision is the share of domq's
//!   words that are in the article, recall the share of the article domq
//!   kept. The same score for the whole page shows what reader mode adds.
//! * **Metadata**: `Dom::metadata()` against Readability's title, byline,
//!   site name, language and published time, compared after collapsing
//!   whitespace, on the pages where Readability found a value.
//!
//! The reference is Readability's output, so this measures agreement with
//! Firefox's reader view, not ground truth.
//!
//! ```text
//! sh conformance/fetch.sh
//! cargo run --release --manifest-path conformance/Cargo.toml --bin reader
//! cargo run --release --manifest-path conformance/Cargo.toml --bin reader -- --write docs/reader-mode.md
//! cargo run --release --manifest-path conformance/Cargo.toml --bin reader -- --check docs/reader-mode.md
//! ```

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

fn main() -> ExitCode {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = here.join("readability/test/test-pages");
    let mut write = None;
    let mut check = None;
    let mut explain = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--write" => write = args.next().map(PathBuf::from),
            "--check" => check = args.next().map(PathBuf::from),
            "--explain" => explain = args.next(),
            "-h" | "--help" => {
                println!("usage: reader [--write FILE | --check FILE | --explain PAGE]");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("reader: unknown argument '{other}'");
                return ExitCode::from(2);
            }
        }
    }
    if let Some(page) = explain {
        return explain_page(&dir.join(page));
    }
    let commit = fs::read_to_string(here.join("READABILITY_COMMIT"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let mut pages: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.join("source.html").is_file() && p.join("expected.html").is_file())
            .collect(),
        Err(e) => {
            eprintln!(
                "reader: reading {}: {e}\nRun `sh conformance/fetch.sh` first.",
                dir.display()
            );
            return ExitCode::from(2);
        }
    };
    pages.sort();

    let scores: Vec<Page> = pages.iter().filter_map(|p| score(p)).collect();
    let report = report(&scores, &commit);
    if let Some(path) = write {
        if let Err(e) = fs::write(&path, &report) {
            eprintln!("reader: writing {}: {e}", path.display());
            return ExitCode::from(2);
        }
        println!("wrote {}", path.display());
    } else if let Some(path) = check {
        if fs::read_to_string(&path).unwrap_or_default() != report {
            eprintln!(
                "reader: {} is out of date. Reader mode's results changed; regenerate it with\n  \
                 cargo run --release --manifest-path conformance/Cargo.toml --bin reader -- --write {}\n\
                 and review the difference.\n\n--- new report ---\n{report}",
                path.display(),
                path.display()
            );
            return ExitCode::FAILURE;
        }
        println!("{} is up to date", path.display());
    } else {
        print!("{}", summary(&scores));
    }
    ExitCode::SUCCESS
}

/// `--explain`: which element `main_content()` picked on one page, its
/// ancestors, and the words it missed or added most.
fn explain_page(dir: &Path) -> ExitCode {
    let (Ok(source), Ok(expected)) = (
        fs::read_to_string(dir.join("source.html")),
        fs::read_to_string(dir.join("expected.html")),
    ) else {
        eprintln!(
            "reader: no source.html / expected.html in {}",
            dir.display()
        );
        return ExitCode::from(2);
    };
    let dom = domq::parse(&source);
    let reference = words(&domq::parse(&expected).to_text());
    let describe = |n: domq::NodeRef<'_>| {
        let mut s = format!("<{}", n.tag_name().unwrap_or("?"));
        for key in ["id", "class", "role", "itemprop"] {
            if let Some(v) = n.attr(key) {
                let _ = write!(s, " {key}=\"{}\"", v.chars().take(60).collect::<String>());
            }
        }
        let words: usize = words(&n.to_text()).values().sum();
        format!("{s}> ({words} words)")
    };
    let Some(main) = dom.main_content() else {
        println!("main_content(): None (the whole page is used)");
        return ExitCode::SUCCESS;
    };
    println!("picked: {}", describe(main));
    for (depth, a) in main.ancestors().enumerate().take(6) {
        println!("  {}parent: {}", "  ".repeat(depth), describe(a));
    }
    let got = words(&main.to_text());
    let s = prf(&got, &reference);
    println!(
        "precision {} recall {} F1 {}",
        pct(s.precision),
        pct(s.recall),
        pct(s.f1)
    );
    let mut missing: Vec<(&String, usize)> = reference
        .iter()
        .map(|(w, n)| (w, n.saturating_sub(got.get(w).copied().unwrap_or(0))))
        .filter(|(_, n)| *n > 0)
        .collect();
    missing.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let total_missing: usize = missing.iter().map(|(_, n)| n).sum();
    println!(
        "missed {total_missing} words, e.g. {:?}",
        missing
            .iter()
            .take(12)
            .map(|(w, _)| w.as_str())
            .collect::<Vec<_>>()
    );
    ExitCode::SUCCESS
}

/// Precision, recall and F1 of one extraction.
#[derive(Clone, Copy, Default)]
struct Prf {
    precision: f64,
    recall: f64,
    f1: f64,
}

/// The metadata fields compared, as (Readability's key, label).
const FIELDS: &[(&str, &str)] = &[
    ("title", "title"),
    ("byline", "author (byline)"),
    ("siteName", "site name"),
    ("lang", "language"),
    ("publishedTime", "published time"),
];

struct Page {
    name: String,
    main: Prf,
    whole: Prf,
    /// `main_content()` returned `None`.
    fallback: bool,
    /// Per [`FIELDS`] entry: `None` when Readability found no value, else
    /// whether domq's matches.
    fields: Vec<Option<bool>>,
    /// `metadata().json_ld`: how many blocks, and how many parse as JSON.
    json_ld: (usize, usize),
}

fn score(dir: &Path) -> Option<Page> {
    let source = fs::read_to_string(dir.join("source.html")).ok()?;
    let expected = fs::read_to_string(dir.join("expected.html")).ok()?;
    let reference = words(&domq::parse(&expected).to_text());
    let dom = domq::parse(&source);
    let whole_text = dom.to_text();
    let (main_text, fallback) = match dom.main_content() {
        Some(main) => (main.to_text(), false),
        None => (whole_text.clone(), true),
    };
    let metadata: serde_json::Value = fs::read_to_string(dir.join("expected-metadata.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let m = dom.metadata();
    let ours = |key: &str| -> Option<String> {
        match key {
            "title" => m.title.clone(),
            "byline" => m.author.clone(),
            "siteName" => m.site_name.clone(),
            "lang" => m.lang.clone(),
            "publishedTime" => m.published.clone(),
            _ => None,
        }
    };
    let fields = FIELDS
        .iter()
        .map(|(key, _)| {
            let theirs = metadata.get(key)?.as_str().map(normalize)?;
            if theirs.is_empty() {
                return None;
            }
            Some(ours(key).map(|o| normalize(&o)) == Some(theirs))
        })
        .collect();
    let parsed = m
        .json_ld
        .iter()
        .filter(|b| serde_json::from_str::<serde_json::Value>(b).is_ok())
        .count();
    Some(Page {
        name: dir.file_name()?.to_str()?.to_string(),
        main: prf(&words(&main_text), &reference),
        whole: prf(&words(&whole_text), &reference),
        fallback,
        fields,
        json_ld: (m.json_ld.len(), parsed),
    })
}

fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The bag of lower-cased words (runs of letters and digits) in `text`.
fn words(text: &str) -> HashMap<String, usize> {
    let mut bag = HashMap::new();
    for word in text.split(|c: char| !c.is_alphanumeric()) {
        if !word.is_empty() {
            *bag.entry(word.to_lowercase()).or_insert(0) += 1;
        }
    }
    bag
}

fn prf(got: &HashMap<String, usize>, want: &HashMap<String, usize>) -> Prf {
    let total = |bag: &HashMap<String, usize>| bag.values().sum::<usize>();
    let common: usize = got
        .iter()
        .map(|(w, n)| (*n).min(want.get(w).copied().unwrap_or(0)))
        .sum();
    let (g, w) = (total(got), total(want));
    let precision = if g == 0 {
        0.0
    } else {
        common as f64 / g as f64
    };
    let recall = if w == 0 {
        1.0
    } else {
        common as f64 / w as f64
    };
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    Prf {
        precision,
        recall,
        f1,
    }
}

fn mean(scores: &[Page], pick: impl Fn(&Page) -> f64) -> f64 {
    if scores.is_empty() {
        return 0.0;
    }
    scores.iter().map(pick).sum::<f64>() / scores.len() as f64
}

fn pct(x: f64) -> String {
    format!("{:.1}%", x * 100.0)
}

fn summary(scores: &[Page]) -> String {
    format!(
        "{} pages\nreader mode: precision {} recall {} F1 {}\nwhole page:  precision {} recall {} F1 {}\n",
        scores.len(),
        pct(mean(scores, |p| p.main.precision)),
        pct(mean(scores, |p| p.main.recall)),
        pct(mean(scores, |p| p.main.f1)),
        pct(mean(scores, |p| p.whole.precision)),
        pct(mean(scores, |p| p.whole.recall)),
        pct(mean(scores, |p| p.whole.f1)),
    )
}

fn report(scores: &[Page], commit: &str) -> String {
    let short = commit.get(..7).unwrap_or(commit);
    let n = scores.len();
    let count = |f: &dyn Fn(&Page) -> bool| scores.iter().filter(|p| f(p)).count();
    let mut out = String::new();
    let _ = write!(
        out,
        "# Reader mode: agreement with Readability

How closely `Dom::main_content()` (`domq --main`, the `main` option of the MCP
tools) and `Dom::metadata()` agree with
[Mozilla Readability](https://github.com/mozilla/readability) (Firefox's
reader view) on Readability's {n} test pages at commit
[`{short}`](https://github.com/mozilla/readability/tree/{commit}/test/test-pages).
The reference is Readability's own output, so these numbers measure
**agreement**, not quality, and are not comparable with other extractors'
benchmark scores. For quality against human labels, see
[reader-quality.md](reader-quality.md) (the AEB and WCXB benchmarks).

<!-- Generated by conformance/ (`cargo run --release --manifest-path
conformance/Cargo.toml --bin reader -- --write docs/reader-mode.md`). Do not
edit: CI regenerates it and fails when it differs. -->

## Main content

Words of domq's pick against the words of Readability's article, both as
`to_text`, compared as bags of lower-cased words. **Precision**: how much of
what domq kept is article; **recall**: how much of the article domq kept.
Means over the pages.

| Extraction | Precision | Recall | F1 |
|---|---:|---:|---:|
| `main_content()` | {mp} | {mr} | {mf} |
| Whole page (no reader mode) | {wp} | {wr} | {wf} |

| Pages where reader mode's F1 is… | Pages |
|---|---:|
| at least 90% | {f90} |
| 50–90% | {f50} |
| below 50% | {f0} |

On {fallback} pages `main_content()` found no container and the whole page
was used.

## Metadata

`Dom::metadata()` against Readability's metadata, on the pages where
Readability found a value; equal after collapsing whitespace.

| Field | Pages | Same |
|---|---:|---:|
",
        mp = pct(mean(scores, |p| p.main.precision)),
        mr = pct(mean(scores, |p| p.main.recall)),
        mf = pct(mean(scores, |p| p.main.f1)),
        wp = pct(mean(scores, |p| p.whole.precision)),
        wr = pct(mean(scores, |p| p.whole.recall)),
        wf = pct(mean(scores, |p| p.whole.f1)),
        f90 = count(&|p| p.main.f1 >= 0.9),
        f50 = count(&|p| p.main.f1 >= 0.5 && p.main.f1 < 0.9),
        f0 = count(&|p| p.main.f1 < 0.5),
        fallback = count(&|p| p.fallback),
    );
    for (i, (_, label)) in FIELDS.iter().enumerate() {
        let known: Vec<bool> = scores.iter().filter_map(|p| p.fields[i]).collect();
        let same = known.iter().filter(|s| **s).count();
        let rate = if known.is_empty() {
            "—".to_string()
        } else {
            pct(same as f64 / known.len() as f64)
        };
        let _ = writeln!(out, "| {label} | {} | {rate} |", known.len());
    }
    let blocks: usize = scores.iter().map(|p| p.json_ld.0).sum();
    let parsed: usize = scores.iter().map(|p| p.json_ld.1).sum();
    let with = count(&|p| p.json_ld.0 > 0);
    let _ = write!(
        out,
        "
`Metadata::json_ld` holds the page's JSON-LD blocks for a JSON parser: on {with} pages, {blocks} blocks, of which {parsed} parse with `serde_json` as returned.
"
    );
    out.push_str(
        "
Readability's byline also comes from the page text (\"By Jane Doe\"), and its
title drops a trailing site name (\"Article | Site\"); domq reads both from
markup only, so those two rows count differences of convention too.

## Hardest pages

The 15 pages with the lowest reader-mode F1, for tuning the heuristic:

| Page | Precision | Recall | F1 |
|---|---:|---:|---:|
",
    );
    let mut worst: Vec<&Page> = scores.iter().collect();
    worst.sort_by(|a, b| a.main.f1.total_cmp(&b.main.f1).then(a.name.cmp(&b.name)));
    for page in worst.iter().take(15) {
        let _ = writeln!(
            out,
            "| `{}`{} | {} | {} | {} |",
            page.name,
            if page.fallback { " (whole page)" } else { "" },
            pct(page.main.precision),
            pct(page.main.recall),
            pct(page.main.f1)
        );
    }
    out.push_str(
        "
The test pages are © their publishers, collected by the Readability project
(Apache-2.0); they are downloaded by `conformance/fetch.sh`, not stored in
this repository.
",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_bags_of_words() {
        let s = prf(&words("The cat, the hat."), &words("the cat sat"));
        assert!(
            (s.precision - 0.5).abs() < 1e-9,
            "2 of 4 words kept are article"
        );
        assert!((s.recall - 2.0 / 3.0).abs() < 1e-9);
        let perfect = prf(&words("A b"), &words("a B"));
        assert!((perfect.f1 - 1.0).abs() < 1e-9);
        assert_eq!(prf(&words(""), &words("x")).f1, 0.0);
    }
}
