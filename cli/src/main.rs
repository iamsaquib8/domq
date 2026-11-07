//! `domq` — query HTML from the command line with CSS selectors.
//!
//! A thin, zero-dependency wrapper over the `domq` library. Flags mirror
//! `htmlq` where they overlap; see `domq --help`.

#![warn(clippy::print_stderr)]

macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::emit_warn(format_args!($($arg)*))
    };
}

mod args;
mod extract;
mod json;
mod lint;
mod mcp;
mod pretty;

use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use domq::{Dom, NodeRef, Selector};

use args::{Command, Mode, Options};

/// Write a message to stderr, ignoring any write failure (such as a closed
/// pipe).
pub(crate) fn emit_warn(args: std::fmt::Arguments<'_>) {
    let mut stderr = io::stderr().lock();
    let _ = stderr
        .write_fmt(args)
        .and_then(|()| stderr.write_all(b"\n"))
        .and_then(|()| stderr.flush());
}

/// Attributes holding a single URL, rewritten by `--base`.
const URL_ATTRS: &[&str] = &["href", "src", "action", "cite", "poster", "formaction"];

struct Output {
    text: String,
    matched: bool,
}

fn main() -> ExitCode {
    let opts = match args::parse(std::env::args_os().skip(1)) {
        Ok(Command::Run(opts)) => opts,
        Ok(Command::Lint(opts)) => {
            let (report, code) = lint::run(&opts);
            return match write_output(None, &report) {
                Ok(()) => ExitCode::from(code),
                Err(e) => {
                    warn!("domq: {e}");
                    ExitCode::from(2)
                }
            };
        }
        Ok(Command::Mcp) => return mcp::serve(),
        Ok(Command::Help) => return emit(None, args::HELP),
        Ok(Command::LintHelp) => return emit(None, args::LINT_HELP),
        Ok(Command::McpHelp) => return emit(None, args::MCP_HELP),
        Ok(Command::Version) => {
            return emit(None, &format!("domq {}\n", env!("CARGO_PKG_VERSION")));
        }
        Err(e) => {
            warn!("domq: {e}\nTry '{}' for more information.", e.help);
            return ExitCode::from(2);
        }
    };
    match run(&opts) {
        Ok(out) => match write_output(opts.output.as_deref(), &out.text) {
            Ok(()) if opts.exit_status && !out.matched => ExitCode::FAILURE,
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                warn!("domq: {e}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            warn!("domq: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(opts: &Options) -> Result<Output, String> {
    let html = read_input(opts.input.as_deref(), opts.encoding.as_deref())?;
    let mut dom = domq::parse(&html);

    for remove in &opts.remove {
        let sel = compile(remove)?;
        let ids: Vec<_> = dom.select(&sel).map(|n| n.id()).collect();
        for id in ids {
            dom.remove(id);
        }
    }

    let base = base_url(&dom, opts);
    if let Some(base) = &base {
        absolutize(&mut dom, base, opts.base.as_deref());
    }

    if opts.meta {
        // Page-level, so read before `--main` narrows the document.
        return Ok(Output {
            text: meta_output(&dom, base.as_deref()),
            matched: true,
        });
    }
    if opts.main
        && let Some(main) = dom.main_content().map(|m| m.to_html())
    {
        dom = domq::parse(&main);
    }

    let scope = match opts.selector.as_str() {
        "" => None,
        s => Some(compile(s)?),
    };
    if !opts.extract.is_empty() {
        return extract_output(&dom, scope.as_ref(), opts);
    }
    if opts.links {
        return Ok(links_output(&dom, scope.as_ref(), base.as_deref(), opts));
    }
    if opts.outline {
        return Ok(outline_output(&dom, scope.as_ref(), opts));
    }

    let selector = match (scope, &opts.attribute, opts.mode) {
        (Some(sel), _, _) => Some(sel),
        (None, _, Mode::Csv) => Some(compile("table")?),
        (None, Some(_), _) | (None, None, Mode::Count) => Some(compile("*")?),
        (None, None, _) => None,
    };

    let Some(selector) = selector else {
        return Ok(Output {
            text: document_output(&dom, opts),
            matched: true,
        });
    };
    let matches: Vec<NodeRef<'_>> = dom.select(&selector).collect();
    if opts.mode == Mode::Csv {
        return Ok(csv_output(&matches, opts.first));
    }
    Ok(match_output(&matches, opts))
}

/// Output for the whole document (no selector given).
fn document_output(dom: &Dom, opts: &Options) -> String {
    let out = match opts.mode {
        Mode::Html if opts.pretty => {
            let mut out = String::new();
            for root in dom.roots() {
                pretty::write(root, &mut out);
            }
            return out;
        }
        Mode::Html => dom.to_html(),
        Mode::Text => dom.to_text(),
        Mode::Markdown => dom.to_markdown_with(&opts.markdown_options()),
        Mode::Json | Mode::Jsonl => dom.to_json(),
        // Both always have a selector (see `run`).
        Mode::Count | Mode::Csv => String::new(),
    };
    with_newline(out)
}

/// Output for a list of selected elements.
fn match_output(matches: &[NodeRef<'_>], opts: &Options) -> Output {
    if let Some(name) = &opts.attribute {
        let mut values: Vec<&str> = matches
            .iter()
            .filter_map(|m| m.attr(name))
            .filter(|v| !(opts.ignore_whitespace && v.trim().is_empty()))
            .collect();
        if opts.first {
            values.truncate(1);
        }
        let matched = !values.is_empty();
        let text = match opts.mode {
            Mode::Count => format!("{}\n", values.len()),
            Mode::Json => format!("{}\n", json::string_array(values)),
            Mode::Jsonl => lines(values.iter().map(|v| json::string(v)), "\n"),
            _ => lines(values.iter().map(|v| v.to_string()), "\n"),
        };
        return Output { text, matched };
    }
    let (text, matched) = match opts.mode {
        Mode::Count => {
            let count = if opts.ignore_whitespace {
                matches
                    .iter()
                    .filter(|m| !m.to_text().trim().is_empty())
                    .take(if opts.first { 1 } else { usize::MAX })
                    .count()
            } else if opts.first {
                matches.len().min(1)
            } else {
                matches.len()
            };
            (format!("{count}\n"), count > 0)
        }
        Mode::Text => {
            let mut values: Vec<String> =
                non_blank(matches.iter().map(|m| m.to_text()), opts).collect();
            if opts.first {
                values.truncate(1);
            }
            let matched = !values.is_empty();
            (lines(values.into_iter(), "\n"), matched)
        }
        Mode::Markdown => {
            let md_opts = opts.markdown_options();
            let mut values: Vec<String> =
                non_blank(matches.iter().map(|m| m.to_markdown_with(&md_opts)), opts).collect();
            if opts.first {
                values.truncate(1);
            }
            let matched = !values.is_empty();
            (lines(values.into_iter(), "\n\n"), matched)
        }
        _ => {
            let selected = if opts.first {
                &matches[..matches.len().min(1)]
            } else {
                matches
            };
            let matched = !selected.is_empty();
            let text = match opts.mode {
                Mode::Json => format!("{}\n", json::array(selected.iter().map(|m| m.to_json()))),
                Mode::Jsonl => lines(selected.iter().map(|m| m.to_json()), "\n"),
                Mode::Html if opts.pretty => {
                    let mut out = String::new();
                    for m in selected {
                        pretty::write(*m, &mut out);
                    }
                    out
                }
                Mode::Html | Mode::Csv => lines(selected.iter().map(|m| m.to_html()), "\n"),
                // Handled above.
                Mode::Count | Mode::Text | Mode::Markdown => String::new(),
            };
            (text, matched)
        }
    };
    Output { text, matched }
}

/// `--csv`: each matched `<table>` with rows as RFC 4180 CSV, separated by a
/// blank line. For non-table matches, the outermost tables inside them are used.
fn csv_output(matches: &[NodeRef<'_>], first: bool) -> Output {
    let tables: Vec<String> = matched_tables(matches)
        .into_iter()
        .map(|t| t.table_rows())
        .filter(|rows| !rows.is_empty())
        .take(if first { 1 } else { usize::MAX })
        .map(domq::to_csv)
        .collect();
    Output {
        matched: !tables.is_empty(),
        text: tables.join("\r\n"),
    }
}

/// The tables to extract from a list of matched elements: for each element,
/// the element itself if it is a `<table>`, or the outermost `<table>`s
/// inside it in document order.
pub(crate) fn matched_tables<'a>(matches: &[NodeRef<'a>]) -> Vec<NodeRef<'a>> {
    let mut tables = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for &m in matches {
        for table in outermost_tables(m) {
            if seen.insert(table.id()) {
                tables.push(table);
            }
        }
    }
    tables
}

/// The element itself if it is a `<table>`, or every outermost `<table>` descendant.
/// Does not descend into nested `<table>`s.
fn outermost_tables<'a>(node: NodeRef<'a>) -> Vec<NodeRef<'a>> {
    if node.tag_name() == Some("table") {
        return vec![node];
    }
    let mut tables = Vec::new();
    let mut stack: Vec<NodeRef<'a>> = node.child_elements().collect();
    stack.reverse();
    while let Some(current) = stack.pop() {
        if current.tag_name() == Some("table") {
            tables.push(current);
        } else {
            let mut children: Vec<_> = current.child_elements().collect();
            children.reverse();
            stack.extend(children);
        }
    }
    tables
}

/// `-e`: one object, or one per match of the scope selector.
fn extract_output(dom: &Dom, scope: Option<&Selector>, opts: &Options) -> Result<Output, String> {
    let fields = extract::compile(&opts.extract)?;
    let ws = opts.ignore_whitespace;
    let Some(scope) = scope else {
        return Ok(Output {
            text: format!("{}\n", extract::object(dom, &fields, ws)),
            matched: true,
        });
    };
    let mut scopes: Vec<NodeRef<'_>> = dom.select(scope).collect();
    if opts.first {
        scopes.truncate(1);
    }
    let objects = extract::scoped_objects(dom, &scopes, &fields, ws).into_iter();
    Ok(Output {
        text: if opts.mode == Mode::Jsonl {
            lines(objects, "\n")
        } else {
            format!("{}\n", json::array(objects))
        },
        matched: !scopes.is_empty(),
    })
}

/// `--links`: every link (inside a match of `scope`, if given), its URL
/// resolved as `--base`/`-B` say — so with neither, URLs print as written.
/// With `-w`, links without text are skipped.
fn links_output(dom: &Dom, scope: Option<&Selector>, base: Option<&str>, opts: &Options) -> Output {
    let inside = scope.map(|sel| inside_matches(dom, sel));
    let in_scope = |node: NodeRef<'_>| {
        inside
            .as_ref()
            .is_none_or(|marks| marks.get(node.id().index()).copied().unwrap_or(false))
    };
    let mut links: Vec<(String, String)> = dom
        .links(None)
        .into_iter()
        .filter(|l| in_scope(l.node))
        .filter(|l| !(opts.ignore_whitespace && l.text.trim().is_empty()))
        .map(|l| (domq::resolve_url(base.unwrap_or(""), l.href), l.text))
        .collect();
    if opts.first {
        links.truncate(1);
    }
    let object = |(url, text): &(String, String)| {
        let mut obj = json::Object::default();
        obj.field("url", &json::string(url))
            .field("text", &json::string(text));
        obj.finish()
    };
    let matched = !links.is_empty();
    let text = match opts.mode {
        Mode::Count => format!("{}\n", links.len()),
        Mode::Json => format!("{}\n", json::array(links.iter().map(object))),
        Mode::Jsonl => lines(links.iter().map(object), "\n"),
        _ => lines(
            links
                .iter()
                .map(|(url, text)| format!("{}\t{}", one_field(url), one_field(text))),
            "\n",
        ),
    };
    Output { text, matched }
}

/// The outline of every match of `sel`, renumbered from 1. Matches inside
/// another match are skipped, so nested matches (`div` on nested `<div>`s)
/// list each control once instead of once per enclosing match.
pub(crate) fn scoped_outline(dom: &Dom, sel: &Selector) -> domq::PageOutline {
    let mut marks = vec![false; dom.len()];
    let matches: Vec<_> = dom.select(sel).collect();
    for m in &matches {
        if let Some(mark) = marks.get_mut(m.id().index()) {
            *mark = true;
        }
    }
    // Parents come before their children in `nodes()`, so one pass marks
    // everything inside a match.
    for node in dom.nodes() {
        let parent_inside = node
            .parent()
            .is_some_and(|p| marks.get(p.id().index()).copied().unwrap_or(false));
        if let Some(mark) = marks.get_mut(node.id().index()) {
            *mark |= parent_inside;
        }
    }
    let mut combined = domq::PageOutline::default();
    for m in matches {
        let nested = m
            .parent()
            .is_some_and(|p| marks.get(p.id().index()).copied().unwrap_or(false));
        if !nested {
            combined.nodes.extend(m.outline().nodes);
        }
    }
    for (i, node) in combined.nodes.iter_mut().enumerate() {
        node.ref_id = i + 1;
    }
    combined
}

fn outline_output(dom: &Dom, scope: Option<&Selector>, opts: &Options) -> Output {
    let mut outline = match scope {
        Some(sel) => scoped_outline(dom, sel),
        None => dom.outline(),
    };
    if opts.first {
        outline.nodes.truncate(1);
    }
    let matched = !outline.nodes.is_empty();
    let text = match opts.mode {
        Mode::Count => format!("{}\n", outline.nodes.len()),
        Mode::Json => with_newline(outline.to_json()),
        Mode::Jsonl => {
            let mut out = String::new();
            for node in &outline.nodes {
                out.push_str(&node.to_json());
                out.push('\n');
            }
            out
        }
        _ => with_newline(outline.to_text()),
    };
    Output { text, matched }
}

/// Which nodes are a match of `sel` or inside one, indexed by
/// `NodeId::index()`: one pass in document order, where parents come before
/// their children (rather than a walk up the ancestors of every node, which
/// is quadratic in deep documents).
fn inside_matches(dom: &Dom, sel: &Selector) -> Vec<bool> {
    let mut marks = vec![false; dom.len()];
    for m in dom.select(sel) {
        if let Some(mark) = marks.get_mut(m.id().index()) {
            *mark = true;
        }
    }
    for node in dom.nodes() {
        let parent_inside = node
            .parent()
            .is_some_and(|p| marks.get(p.id().index()).copied().unwrap_or(false));
        if let Some(mark) = marks.get_mut(node.id().index()) {
            *mark |= parent_inside;
        }
    }
    marks
}

/// `s` with tabs and line breaks turned into spaces, for one TSV field.
fn one_field(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

/// `--meta`: [`meta_json`] on one line.
fn meta_output(dom: &Dom, base: Option<&str>) -> String {
    format!("{}\n", meta_json(dom, base))
}

/// [`Dom::metadata`] as a JSON object, keys as in the library's `serde` form.
/// With a base URL, `canonical`, `image` and `feeds` are made absolute.
fn meta_json(dom: &Dom, base: Option<&str>) -> String {
    let m = dom.metadata();
    let url = |u: &str| match base {
        Some(base) => domq::resolve_url(base, u),
        None => u.to_string(),
    };
    let opt = |v: &Option<String>| json::opt_string(v.as_deref());
    let opt_url = |v: &Option<String>| json::opt_string(v.as_deref().map(url).as_deref());
    let pairs = |v: &[(String, String)]| {
        json::array(
            v.iter()
                .map(|(k, v)| json::string_array([k.as_str(), v.as_str()])),
        )
    };
    let feeds: Vec<String> = m.feeds.iter().map(|f| url(f)).collect();
    let mut obj = json::Object::default();
    obj.field("title", &opt(&m.title))
        .field("description", &opt(&m.description))
        .field("canonical", &opt_url(&m.canonical))
        .field("lang", &opt(&m.lang))
        .field("site_name", &opt(&m.site_name))
        .field("image", &opt_url(&m.image))
        .field("author", &opt(&m.author))
        .field("published", &opt(&m.published))
        .field(
            "keywords",
            &json::string_array(m.keywords.iter().map(String::as_str)),
        )
        .field("og", &pairs(&m.og))
        .field("twitter", &pairs(&m.twitter))
        .field(
            "json_ld",
            &json::string_array(m.json_ld.iter().map(String::as_str)),
        )
        .field(
            "feeds",
            &json::string_array(feeds.iter().map(String::as_str)),
        );
    obj.finish()
}

/// Drop blank outputs when `--ignore-whitespace` is set.
fn non_blank<'o>(
    items: impl Iterator<Item = String> + 'o,
    opts: &'o Options,
) -> impl Iterator<Item = String> + 'o {
    items.filter(move |s| !(opts.ignore_whitespace && s.trim().is_empty()))
}

fn lines(items: impl Iterator<Item = String>, sep: &str) -> String {
    with_newline(items.collect::<Vec<_>>().join(sep))
}

fn with_newline(mut s: String) -> String {
    if !s.is_empty() && !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// Parse a selector, turning errors into a message that points at the problem.
fn compile(selector: &str) -> Result<Selector, String> {
    Selector::parse(selector).map_err(|e| {
        let col = selector
            .get(..e.position())
            .map_or(0, |s| s.chars().count());
        format!(
            "invalid selector: {}\n  {selector}\n  {}^",
            e.message(),
            " ".repeat(col)
        )
    })
}

/// The base URL to resolve links against: `<base href>` (with `-B`), resolved
/// against `--base` when both are present.
fn base_url(dom: &Dom, opts: &Options) -> Option<String> {
    if opts.detect_base {
        dom.base_url(opts.base.as_deref())
    } else {
        opts.base.clone()
    }
}

/// Rewrite every relative URL attribute in the document to an absolute one,
/// resolved against `base`.
///
/// A `<base href>` is the exception: it is relative to the document's own
/// URL (`--base`, if given), not to the base URL it defines — so with `-B`
/// it is resolved once, as in [`Dom::base_url`], instead of against itself.
///
/// A repeated attribute (`<a href=a href=b>`) is read and rewritten through
/// its first occurrence only — the one `NodeRef::attr` and `Dom::set_attr`
/// use — so the value that counts is the one resolved.
fn absolutize(dom: &mut Dom, base: &str, document_url: Option<&str>) {
    let mut edits = Vec::new();
    for node in dom.nodes() {
        let base = match node.tag_name() {
            Some("base") => match document_url {
                Some(url) => url,
                None => continue,
            },
            _ => base,
        };
        for &name in URL_ATTRS.iter().chain(&["srcset"]) {
            let Some(value) = node.attr(name) else {
                continue;
            };
            let resolved = if name == "srcset" {
                domq::resolve_srcset(base, value)
            } else {
                domq::resolve_url(base, value)
            };
            if resolved != value {
                edits.push((node.id(), name, resolved));
            }
        }
    }
    for (id, name, value) in edits {
        dom.set_attr(id, name, &value);
    }
}

/// Read the input and decode it (see `encoding.rs`).
fn read_input(path: Option<&str>, encoding: Option<&str>) -> Result<String, String> {
    let bytes = match path {
        None | Some("-") => {
            let mut buf = Vec::new();
            io::stdin()
                .read_to_end(&mut buf)
                .map_err(|e| format!("reading stdin: {e}"))?;
            buf
        }
        Some(p) => fs::read(p).map_err(|e| format!("reading {p}: {e}"))?,
    };
    let decoded = domq::decode_with(&bytes, encoding);
    if let Some(warning) = &decoded.warning {
        let name = match path {
            None | Some("-") => "<stdin>",
            Some(p) => p,
        };
        warn!("domq: warning: {name}: {warning}");
    }
    Ok(decoded.text)
}

/// Write `out` to a file or stdout. A closed pipe (`domq … | head`) is not
/// an error.
fn write_output(path: Option<&str>, out: &str) -> Result<(), String> {
    match path {
        Some(p) if p != "-" => fs::write(p, out).map_err(|e| format!("writing {p}: {e}")),
        _ => {
            let mut stdout = io::stdout().lock();
            match stdout
                .write_all(out.as_bytes())
                .and_then(|()| stdout.flush())
            {
                Err(e) if !is_closed_pipe(&e) => Err(format!("writing: {e}")),
                _ => Ok(()),
            }
        }
    }
}

/// [`write_output`], as the exit code.
fn emit(path: Option<&str>, out: &str) -> ExitCode {
    match write_output(path, out) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            warn!("domq: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Whether a stdout write failed because the reader went away.
#[cfg(not(target_os = "wasi"))]
fn is_closed_pipe(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::BrokenPipe
}

/// Whether a stdout write failed because the reader went away. wasmtime
/// reports a closed pipe as `EIO` (WASI errno 29) rather than `EPIPE`.
#[cfg(target_os = "wasi")]
fn is_closed_pipe(e: &io::Error) -> bool {
    const WASI_EIO: i32 = 29;
    e.kind() == io::ErrorKind::BrokenPipe || e.raw_os_error() == Some(WASI_EIO)
}
