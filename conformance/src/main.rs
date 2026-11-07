//! Run the HTML tree-construction tests against domq and report pass rates.
//!
//! The tests are the html5lib `.dat` files, now maintained in
//! web-platform-tests (`html/syntax/parsing/resources/`); `fetch.sh` downloads
//! them at the commit pinned in `WPT_COMMIT`. Each test is a document (or a
//! fragment) and the tree a spec-conforming parser builds for it.
//!
//! Every test is scored twice:
//!
//! * **exact**: domq's tree, dumped in the html5lib format, matches the
//!   expected dump line for line.
//! * **structure**: the trees match in domq's model of HTML, which is not the
//!   browser's: implied `<html>`, `<head>`, `<body>` and `<tbody>` wrappers
//!   and template `content` are unwrapped, namespaces and the case of SVG
//!   names are ignored, whitespace-only text is dropped (adjacent text is then
//!   merged), and doctypes compare by name only.
//!
//! ```text
//! sh conformance/fetch.sh
//! cargo run --release --manifest-path conformance/Cargo.toml              # summary
//! cargo run --release --manifest-path conformance/Cargo.toml -- --write docs/conformance.md
//! cargo run --release --manifest-path conformance/Cargo.toml -- --check docs/conformance.md
//! cargo run --release --manifest-path conformance/Cargo.toml -- --failures adoption01.dat
//! ```

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs, panic};

use domq::{Dom, NodeKind, NodeRef};

fn main() -> ExitCode {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut dir = here.join("wpt/html/syntax/parsing/resources");
    let mut write = None;
    let mut check = None;
    let mut failures = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--write" => write = args.next().map(PathBuf::from),
            "--check" => check = args.next().map(PathBuf::from),
            "--failures" => failures = Some(args.next().unwrap_or_default()),
            "--dir" => dir = args.next().map(PathBuf::from).unwrap_or(dir),
            "-h" | "--help" => {
                println!(
                    "usage: conformance [--dir DIR] [--write FILE | --check FILE | --failures [FILE|AREA]]"
                );
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("conformance: unknown argument '{other}'");
                return ExitCode::from(2);
            }
        }
    }
    let commit = fs::read_to_string(here.join("WPT_COMMIT"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let mut files: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "dat"))
            .collect(),
        Err(e) => {
            eprintln!(
                "conformance: reading {}: {e}\nRun `sh conformance/fetch.sh` first.",
                dir.display()
            );
            return ExitCode::from(2);
        }
    };
    files.sort();

    let mut results = Vec::new();
    for path in &files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("conformance: reading {}: {e}", path.display());
                return ExitCode::from(2);
            }
        };
        for test in parse_dat(&text) {
            results.push(run(&name, test));
        }
    }

    if let Some(filter) = failures {
        print_failures(&results, &filter);
        return ExitCode::SUCCESS;
    }
    let report = report(&results, &commit);
    if let Some(path) = write {
        if let Err(e) = fs::write(&path, &report) {
            eprintln!("conformance: writing {}: {e}", path.display());
            return ExitCode::from(2);
        }
        println!("wrote {}", path.display());
    } else if let Some(path) = check {
        let current = fs::read_to_string(&path).unwrap_or_default();
        if current != report {
            eprintln!(
                "conformance: {} is out of date. The parser's results changed; regenerate it with\n  \
                 cargo run --release --manifest-path conformance/Cargo.toml -- --write {}\n\
                 and review the difference.\n\n--- new report ---\n{report}",
                path.display(),
                path.display()
            );
            return ExitCode::FAILURE;
        }
        println!("{} is up to date", path.display());
    } else {
        print!("{}", summary(&results));
    }
    ExitCode::SUCCESS
}

/// One test from a `.dat` file.
struct Test {
    /// Line of its `#data` in the file.
    line: usize,
    data: String,
    /// The context element, for fragment tests.
    fragment: Option<String>,
    /// `Some(true)` for `#script-on`, `Some(false)` for `#script-off`.
    scripting: Option<bool>,
    /// The expected tree, as dump entries.
    expected: Vec<Entry>,
}

/// One node of a tree dump: its depth and its line (a text node's line may
/// hold newlines).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    depth: usize,
    line: String,
}

/// Split a `.dat` file into tests (format: the README next to the files).
fn parse_dat(text: &str) -> Vec<Test> {
    let lines: Vec<&str> = text.split('\n').collect();
    let header = |l: &str| {
        matches!(
            l,
            "#errors"
                | "#new-errors"
                | "#document-fragment"
                | "#script-off"
                | "#script-on"
                | "#document"
        )
    };
    let mut tests = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].is_empty() {
            i += 1;
            continue;
        }
        if lines[i] == "#description" {
            while i < lines.len() && lines[i] != "#data" {
                i += 1;
            }
        }
        if i >= lines.len() || lines[i] != "#data" {
            // Not a test start; skip the line rather than loop forever.
            i += 1;
            continue;
        }
        let line = i + 1;
        i += 1;
        let start = i;
        while i < lines.len() && lines[i] != "#errors" {
            i += 1;
        }
        let data = lines[start..i.min(lines.len())].join("\n");
        let (mut fragment, mut scripting, mut expected) = (None, None, Vec::new());
        // Skip the error lines, then read the remaining sections.
        i += 1;
        while i < lines.len() && !header(lines[i]) {
            i += 1;
        }
        while i < lines.len() {
            match lines[i] {
                "#new-errors" => {
                    i += 1;
                    while i < lines.len() && !header(lines[i]) {
                        i += 1;
                    }
                }
                "#document-fragment" => {
                    fragment = lines.get(i + 1).map(|s| s.to_string());
                    i += 2;
                }
                "#script-off" => {
                    scripting = Some(false);
                    i += 1;
                }
                "#script-on" => {
                    scripting = Some(true);
                    i += 1;
                }
                "#document" => {
                    i += 1;
                    let start = i;
                    // The dump ends at a blank line followed by the next test
                    // (or the end): text nodes may contain blank lines.
                    while i < lines.len() {
                        let next_test = lines[i + 1..]
                            .iter()
                            .find(|l| !l.is_empty())
                            .is_none_or(|l| *l == "#data" || *l == "#description");
                        if lines[i].is_empty() && next_test {
                            break;
                        }
                        i += 1;
                    }
                    expected = parse_dump(&lines[start..i]);
                    break;
                }
                _ => i += 1,
            }
        }
        tests.push(Test {
            line,
            data,
            fragment,
            scripting,
            expected,
        });
    }
    tests
}

/// Dump lines (`| ` + two spaces per level) to entries; a line without the
/// `| ` prefix continues the previous text node.
fn parse_dump(lines: &[&str]) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    for line in lines {
        match line.strip_prefix("| ") {
            Some(rest) => {
                let content = rest.trim_start_matches(' ');
                entries.push(Entry {
                    depth: (rest.len() - content.len()) / 2,
                    line: content.to_string(),
                });
            }
            None => {
                if let Some(last) = entries.last_mut() {
                    last.line.push('\n');
                    last.line.push_str(line);
                }
            }
        }
    }
    entries
}

/// domq's tree in the html5lib dump format.
fn dump(dom: &Dom) -> Vec<Entry> {
    fn walk(node: NodeRef<'_>, depth: usize, out: &mut Vec<Entry>) {
        let entry = |line: String| Entry { depth, line };
        match node.kind() {
            NodeKind::Element { tag } => {
                out.push(entry(format!("<{tag}>")));
                let mut attrs: Vec<(&str, &str)> = Vec::new();
                for (name, value) in node.attributes() {
                    // A repeated attribute is dropped, as in a browser.
                    if !attrs.iter().any(|(n, _)| *n == name) {
                        attrs.push((name, value));
                    }
                }
                attrs.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
                for (name, value) in attrs {
                    out.push(Entry {
                        depth: depth + 1,
                        line: format!("{name}=\"{value}\""),
                    });
                }
                for child in node.children() {
                    walk(child, depth + 1, out);
                }
            }
            NodeKind::Text(text) => out.push(entry(format!("\"{text}\""))),
            NodeKind::Comment(text) => out.push(entry(format!("<!-- {text} -->"))),
            NodeKind::Doctype(body) => out.push(entry(doctype(body))),
        }
    }
    let mut out = Vec::new();
    for root in dom.roots() {
        walk(root, 0, &mut out);
    }
    out
}

/// A doctype body (`html PUBLIC "…" "…"`) as `<!DOCTYPE name "public" "system">`.
fn doctype(body: &str) -> String {
    let body = body.trim();
    let name_end = body
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(body.len());
    let name = body[..name_end].to_ascii_lowercase();
    let rest = body[name_end..].trim_start();
    let quoted = |s: &str| -> Vec<String> {
        let mut out = Vec::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '"' || c == '\'' {
                out.push(chars.by_ref().take_while(|&d| d != c).collect());
            }
        }
        out
    };
    let keyword = rest.get(..6).map(str::to_ascii_uppercase);
    let (public, system) = match keyword.as_deref() {
        Some("PUBLIC") => {
            let ids = quoted(&rest[6..]);
            (ids.first().cloned(), ids.get(1).cloned())
        }
        Some("SYSTEM") => (None, quoted(&rest[6..]).first().cloned()),
        _ => (None, None),
    };
    match (public, system) {
        (None, None) => format!("<!DOCTYPE {name}>"),
        (p, s) => format!(
            "<!DOCTYPE {name} \"{}\" \"{}\">",
            p.unwrap_or_default(),
            s.unwrap_or_default()
        ),
    }
}

/// A dump entry's node kind, read from its line.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Element,
    Attribute,
    Text,
    Comment,
    Doctype,
    Instruction,
    TemplateContent,
}

fn kind(line: &str) -> Kind {
    if line.starts_with('"') {
        Kind::Text
    } else if line.starts_with("<!-- ") {
        Kind::Comment
    } else if line.starts_with("<!DOCTYPE") {
        Kind::Doctype
    } else if line.starts_with("<?") {
        Kind::Instruction
    } else if line.starts_with('<') {
        Kind::Element
    } else if line == "content" {
        Kind::TemplateContent
    } else {
        Kind::Attribute
    }
}

struct Node {
    line: String,
    kind: Kind,
    children: Vec<Node>,
}

fn tree(entries: &[Entry]) -> Vec<Node> {
    fn build(entries: &[Entry], i: &mut usize, depth: usize) -> Vec<Node> {
        let mut nodes = Vec::new();
        while let Some(e) = entries.get(*i) {
            if e.depth < depth {
                break;
            }
            *i += 1;
            let children = build(entries, i, e.depth + 1);
            nodes.push(Node {
                kind: kind(&e.line),
                line: e.line.clone(),
                children,
            });
        }
        nodes
    }
    build(entries, &mut 0, 0)
}

/// Elements a browser inserts on its own, which domq does not.
const IMPLIED: &[&str] = &["html", "head", "body", "tbody"];

/// The tree in domq's model (see the module docs), as dump entries.
fn structure(entries: &[Entry]) -> Vec<Entry> {
    fn text_of(line: &str) -> &str {
        line.get(1..line.len().saturating_sub(1)).unwrap_or("")
    }
    fn norm(nodes: &[Node], out: &mut Vec<Node>) {
        for node in nodes {
            match node.kind {
                Kind::Element => {
                    let name = text_of(&node.line);
                    let name = name
                        .strip_prefix("svg ")
                        .or_else(|| name.strip_prefix("math "))
                        .unwrap_or(name)
                        .to_ascii_lowercase();
                    let (attrs, kids): (Vec<&Node>, Vec<&Node>) = node
                        .children
                        .iter()
                        .partition(|c| c.kind == Kind::Attribute);
                    let kids: Vec<Node> = kids.into_iter().map(clone).collect();
                    if IMPLIED.contains(&name.as_str()) {
                        norm(&kids, out);
                        continue;
                    }
                    let mut children: Vec<Node> = attrs
                        .iter()
                        .map(|a| {
                            let (n, v) = a.line.split_once('=').unwrap_or((&a.line, ""));
                            Node {
                                line: format!(
                                    "{}={v}",
                                    n.replacen(' ', ":", 1).to_ascii_lowercase()
                                ),
                                kind: Kind::Attribute,
                                children: Vec::new(),
                            }
                        })
                        .collect();
                    children.sort_by(|a, b| a.line.cmp(&b.line));
                    norm(&kids, &mut children);
                    out.push(Node {
                        line: format!("<{name}>"),
                        kind: Kind::Element,
                        children,
                    });
                }
                Kind::TemplateContent => norm(&node.children, out),
                Kind::Text => {
                    let text = text_of(&node.line);
                    if text.trim().is_empty() {
                        continue;
                    }
                    match out.last_mut() {
                        Some(last) if last.kind == Kind::Text => {
                            last.line = format!("\"{}{text}\"", text_of(&last.line));
                        }
                        _ => out.push(clone(node)),
                    }
                }
                Kind::Doctype => {
                    let name = node.line["<!DOCTYPE".len()..]
                        .trim_start()
                        .split([' ', '>'])
                        .next()
                        .unwrap_or("");
                    out.push(Node {
                        line: format!("<!DOCTYPE {name}>"),
                        kind: Kind::Doctype,
                        children: Vec::new(),
                    });
                }
                Kind::Comment | Kind::Instruction | Kind::Attribute => out.push(clone(node)),
            }
        }
    }
    fn clone(node: &Node) -> Node {
        Node {
            line: node.line.clone(),
            kind: node.kind,
            children: node.children.iter().map(clone).collect(),
        }
    }
    fn flatten(nodes: &[Node], depth: usize, out: &mut Vec<Entry>) {
        for node in nodes {
            out.push(Entry {
                depth,
                line: node.line.clone(),
            });
            flatten(&node.children, depth + 1, out);
        }
    }
    let mut normalized = Vec::new();
    norm(&tree(entries), &mut normalized);
    let mut out = Vec::new();
    flatten(&normalized, 0, &mut out);
    out
}

/// A scored test.
struct Outcome {
    file: String,
    test: Test,
    /// `None`: skipped (it needs scripting enabled).
    actual: Option<Vec<Entry>>,
    exact: bool,
    structure: bool,
    panicked: bool,
}

fn run(file: &str, test: Test) -> Outcome {
    let skip = test.scripting == Some(true);
    let mut outcome = Outcome {
        file: file.to_string(),
        actual: None,
        exact: false,
        structure: false,
        panicked: false,
        test,
    };
    if skip {
        return outcome;
    }
    let data = outcome.test.data.clone();
    match panic::catch_unwind(|| dump(&domq::parse(&data))) {
        Ok(actual) => {
            outcome.exact = actual == outcome.test.expected;
            outcome.structure = structure(&actual) == structure(&outcome.test.expected);
            outcome.actual = Some(actual);
        }
        Err(_) => outcome.panicked = true,
    }
    outcome
}

/// What a file's tests are mostly about.
fn area(file: &str) -> &'static str {
    let stem = file.trim_end_matches(".dat");
    match stem {
        "adoption01"
        | "adoption02"
        | "scripted_adoption01"
        | "scripted_ark"
        | "tests22"
        | "tests23"
        | "tests24"
        | "tests25"
        | "tests26" => "Mis-nested formatting (adoption agency)",
        "tables01" | "scripted_foster01" | "tests7" | "tests8" => "Tables and foster parenting",
        "template" => "Templates",
        "svg"
        | "math"
        | "foreign-fragment"
        | "namespace-sensitivity"
        | "tests9"
        | "tests10"
        | "tests11"
        | "tests12"
        | "tests21" => "Foreign content (SVG, MathML)",
        "entities01" | "entities02" => "Character references",
        "comments01" | "doctype01" | "quirks01" | "html5test-com" => "Comments, doctypes, quirks",
        "processing-instructions" => "Processing instructions (new in the spec)",
        "noscript01" | "scriptdata01" | "plain-text-unsafe" | "domjs-unsafe" | "tests5"
        | "tests15" | "tests16" | "tests17" => "Raw text, script data, unsafe text",
        "tests_innerHTML_1" | "tests4" | "tests6" => "Fragments and context",
        _ => "General tree building",
    }
}

const AREAS: &[&str] = &[
    "General tree building",
    "Comments, doctypes, quirks",
    "Character references",
    "Raw text, script data, unsafe text",
    "Mis-nested formatting (adoption agency)",
    "Tables and foster parenting",
    "Foreign content (SVG, MathML)",
    "Templates",
    "Fragments and context",
    "Processing instructions (new in the spec)",
];

#[derive(Default, Clone, Copy)]
struct Tally {
    run: usize,
    exact: usize,
    structure: usize,
}

impl Tally {
    fn add(&mut self, o: &Outcome) {
        if o.actual.is_some() || o.panicked {
            self.run += 1;
            self.exact += o.exact as usize;
            self.structure += o.structure as usize;
        }
    }
}

fn pct(n: usize, of: usize) -> String {
    if of == 0 {
        "—".to_string()
    } else {
        format!("{:.1}%", n as f64 * 100.0 / of as f64)
    }
}

fn summary(results: &[Outcome]) -> String {
    let mut total = Tally::default();
    for o in results {
        total.add(o);
    }
    let panics = results.iter().filter(|o| o.panicked).count();
    format!(
        "{} tests run, {} skipped (scripting on), {} panics\nexact:     {} ({})\nstructure: {} ({})\n",
        total.run,
        results.len() - total.run,
        panics,
        total.exact,
        pct(total.exact, total.run),
        total.structure,
        pct(total.structure, total.run),
    )
}

fn report(results: &[Outcome], commit: &str) -> String {
    let mut total = Tally::default();
    let mut by_area = vec![Tally::default(); AREAS.len()];
    let mut by_file: Vec<(String, Tally)> = Vec::new();
    for o in results {
        total.add(o);
        if let Some(i) = AREAS.iter().position(|a| *a == area(&o.file)) {
            by_area[i].add(o);
        }
        match by_file.last_mut() {
            Some((file, tally)) if *file == o.file => tally.add(o),
            _ => {
                let mut tally = Tally::default();
                tally.add(o);
                by_file.push((o.file.clone(), tally));
            }
        }
    }
    let skipped = results.len() - total.run;
    let panics = results.iter().filter(|o| o.panicked).count();
    let short = commit.get(..7).unwrap_or(commit);
    let mut out = String::new();
    let _ = write!(
        out,
        "# HTML parsing conformance

How closely domq's trees match a browser's, measured with the HTML
tree-construction tests (the html5lib tests, maintained in web-platform-tests)
at commit [`{short}`](https://github.com/web-platform-tests/wpt/tree/{commit}/html/syntax/parsing/resources).

<!-- Generated by conformance/ (`cargo run --release --manifest-path
conformance/Cargo.toml -- --write docs/conformance.md`). Do not edit: CI
regenerates it and fails when it differs. -->

**{run} tests run**, {skipped} skipped (they need scripting enabled), **{panics} panics**.

| Measure | Passing | Rate |
|---|---:|---:|
| Exact tree: the html5lib dump, line for line | {exact} | {exact_pct} |
| Same structure, in domq's model of HTML | {structure} | {structure_pct} |

domq is a forgiving parser, not a spec-compliant one (see the
[README](../README.md#what-it-handles)), so the exact rate is low by design:
most expected trees start with `<html>`, `<head>` and `<body>` elements that a
browser inserts and domq does not. **Same structure** compares the trees after
removing what domq deliberately does differently:

- implied `<html>`, `<head>`, `<body>` and `<tbody>` are unwrapped (their
  children take their place) in both trees, and so is a template's `content`;
- namespaces and the case of SVG and MathML names are ignored
  (`<svg foreignObject>` and `<foreignobject>` are equal);
- whitespace-only text nodes are dropped, and adjacent text is merged;
- doctypes compare by name only.

Everything else counts: which elements exist, how they nest, their attributes,
text and comments. A test that fails here is a real difference from a browser.

## By area

| Area | Tests | Exact | Structure |
|---|---:|---:|---:|
",
        run = total.run,
        exact = total.exact,
        exact_pct = pct(total.exact, total.run),
        structure = total.structure,
        structure_pct = pct(total.structure, total.run),
    );
    for (name, tally) in AREAS.iter().zip(&by_area) {
        let _ = writeln!(
            out,
            "| {name} | {} | {} | {} |",
            tally.run,
            pct(tally.exact, tally.run),
            pct(tally.structure, tally.run)
        );
    }
    out.push_str(
        "
### Where the gaps are by design

- **Mis-nested formatting**: browsers run the adoption agency algorithm on
  `<b><p>x</b>y</p>`, cloning formatting elements across blocks, and reopen
  formatting elements a block closed; domq closes the formatting element and
  moves on. A bounded version is the next item on the
  [roadmap](../ROADMAP.md).
- **Tables**: browsers move stray content out of tables (foster parenting)
  and insert `<tbody>`; domq keeps content where it appears. Optional
  `<tbody>` insertion is planned for 0.10.
- **Foreign content**: domq has no namespaces, and lower-cases every tag name.
- **Templates**: a template's children stay its children (browsers move them
  into a separate document fragment, `content`); the structure score unwraps
  `content` for this reason.
- **Fragments**: domq parses a fragment like a document, without the context
  element a test names (`<td>` parsed inside a `<tr>`). Context-aware
  fragment parsing is planned for 0.10.
- **Scripting**: domq never runs scripts, so tests that need scripting enabled
  are skipped; `<noscript>` content is parsed as markup.
- **Quirks mode**: domq always builds the tree as in standards mode (a
  `<table>` closes an open `<p>`, as with `<!DOCTYPE html>`).
- **Processing instructions**: a recent change to the HTML standard keeps
  `<?target data>` as a processing-instruction node; domq, like the browsers
  that predate it, turns it into a comment.

## By file

| File | Area | Tests | Exact | Structure |
|---|---|---:|---:|---:|
",
    );
    for (file, tally) in &by_file {
        let _ = writeln!(
            out,
            "| `{file}` | {} | {} | {} | {} |",
            area(file),
            tally.run,
            pct(tally.exact, tally.run),
            pct(tally.structure, tally.run)
        );
    }
    out.push_str(
        "
To see the failures of one file or area:
`cargo run --release --manifest-path conformance/Cargo.toml -- --failures tables01.dat`.
The test data is © web-platform-tests contributors, under the
[3-Clause BSD License](https://github.com/web-platform-tests/wpt/blob/master/LICENSE.md);
it is downloaded by `conformance/fetch.sh`, not stored in this repository.
",
    );
    out
}

fn print_failures(results: &[Outcome], filter: &str) {
    let render = |entries: &[Entry]| {
        entries
            .iter()
            .map(|e| format!("| {}{}", "  ".repeat(e.depth), e.line))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut shown = 0;
    for o in results {
        if o.structure || o.actual.is_none() && !o.panicked {
            continue;
        }
        if !filter.is_empty() && o.file != filter && area(&o.file) != filter {
            continue;
        }
        shown += 1;
        let context = o
            .test
            .fragment
            .as_deref()
            .map(|c| format!(" (fragment in <{c}>)"))
            .unwrap_or_default();
        println!("== {}:{}{context}\n{}\n", o.file, o.test.line, o.test.data);
        if o.panicked {
            println!("-- PANICKED\n");
            continue;
        }
        println!(
            "-- expected (structure)\n{}\n-- domq (structure)\n{}\n",
            render(&structure(&o.test.expected)),
            render(&structure(o.actual.as_deref().unwrap_or(&[])))
        );
    }
    println!("{shown} failing tests");
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAT: &str = "#data\n<p>One<p>Two\n#errors\n(1,3): x\n#document\n| <html>\n|   <head>\n|   <body>\n|     <p>\n|       \"One\"\n|     <p>\n|       \"Two\"\n\n#data\n<pre>\n\nx</pre>\n#errors\n#document-fragment\ntd\n#script-off\n#document\n| <pre>\n|   \"\n\nx\"\n";

    #[test]
    fn parses_tests() {
        let tests = parse_dat(DAT);
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].data, "<p>One<p>Two");
        assert_eq!(tests[0].expected.len(), 7);
        assert_eq!(tests[1].fragment.as_deref(), Some("td"));
        assert_eq!(tests[1].scripting, Some(false));
        assert_eq!(tests[1].expected[1].line, "\"\n\nx\"");
    }

    #[test]
    fn scores_both_ways() {
        let tests = parse_dat(DAT);
        let first = run("t.dat", tests.into_iter().next().unwrap());
        assert!(!first.exact, "no implied <html>");
        assert!(first.structure);
    }

    #[test]
    fn structure_normalizes() {
        let e = |depth, line: &str| Entry {
            depth,
            line: line.to_string(),
        };
        let browser = [
            e(0, "<!DOCTYPE html \"\" \"about:legacy-compat\">"),
            e(0, "<html>"),
            e(1, "<body>"),
            e(2, "<svg svg>"),
            e(3, "xlink href=\"#a\""),
            e(3, "<svg foreignObject>"),
            e(2, "\"a\""),
            e(2, "\"  \""),
            e(2, "\"b\""),
        ];
        let domq = [
            e(0, "<!DOCTYPE html>"),
            e(0, "<svg>"),
            e(1, "xlink:href=\"#a\""),
            e(1, "<foreignobject>"),
            e(0, "\"ab\""),
        ];
        assert_eq!(structure(&browser), structure(&domq));
    }

    #[test]
    fn doctypes() {
        assert_eq!(doctype("html"), "<!DOCTYPE html>");
        assert_eq!(
            doctype("HTML PUBLIC \"-//W3C//DTD HTML 4.01//EN\" 'http://x'"),
            "<!DOCTYPE html \"-//W3C//DTD HTML 4.01//EN\" \"http://x\">"
        );
        assert_eq!(
            doctype("html SYSTEM \"about:legacy-compat\""),
            "<!DOCTYPE html \"\" \"about:legacy-compat\">"
        );
    }
}
