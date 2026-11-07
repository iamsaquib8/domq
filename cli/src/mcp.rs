//! `domq mcp`: a Model Context Protocol server over stdio.
//!
//! The tools work on HTML the agent passes in (the `html` argument) and never
//! touch the network or the file system, so every call is a pure function of
//! its arguments: read-only, idempotent and closed-world.
//!
//! Both protocol eras are served on the same stdin/stdout:
//!
//! * **Modern** (2026-07-28): every request names its protocol version in
//!   `params._meta`, there is no handshake, and `server/discover` describes
//!   the server. Results carry `resultType` and the server's identity.
//! * **Legacy** (2025-11-25 and earlier): the client opens with `initialize`.
//!   Requests without a version in `_meta` are served this way, also before
//!   (or without) `initialize` — the tools need no session state.
//!
//! Messages are one JSON-RPC object per line. Notifications get no reply,
//! and batches (a JSON array, allowed by 2025-03-26) get an array of replies.

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use domq::{Dom, NodeRef};

use crate::extract::{self, Field};
use crate::json::{self, Value};
use crate::lint::{self, Level};

/// The current protocol revision (per-request metadata, no handshake).
const MODERN: &str = "2026-07-28";
/// Handshake-based revisions, newest first.
const LEGACY: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const VERSION_KEY: &str = "io.modelcontextprotocol/protocolVersion";
const CAPABILITIES_KEY: &str = "io.modelcontextprotocol/clientCapabilities";

// JSON-RPC and MCP error codes.
const PARSE_ERROR: i32 = -32700;
const INVALID_REQUEST: i32 = -32600;
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;
const UNSUPPORTED_VERSION: i32 = -32022;

/// How long clients may cache `server/discover` and `tools/list`: the tool
/// set only changes with the binary.
const TTL_MS: &str = "3600000";

const INSTRUCTIONS: &str = "domq parses HTML that you pass in the `html` \
argument; it never fetches URLs, so get the page first (with a fetch tool, \
for example) and pass its HTML. To read a page, use `markdown` (main=true \
keeps just the article). To understand page structure, use `outline`; \
to find elements, use `select`; to pull structured fields, such as one \
object per product card or table row, use `extract`. `chunks` prepares \
Markdown for embeddings; `links`, `metadata` and `tables` read those \
parts of a page; `lint` checks HTML for accessibility and markup \
problems. Invalid selectors come back as errors that point at the problem.";

/// Serve requests from stdin until it closes.
pub fn serve() -> ExitCode {
    let mut input = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    let mut line = Vec::new();
    loop {
        line.clear();
        match input.read_until(b'\n', &mut line) {
            Ok(0) => return ExitCode::SUCCESS,
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                warn!("domq mcp: reading stdin: {e}");
                return ExitCode::FAILURE;
            }
        }
        let Some(reply) = handle_line(&line) else {
            continue;
        };
        if writeln!(stdout, "{reply}")
            .and_then(|()| stdout.flush())
            .is_err()
        {
            // The client went away.
            return ExitCode::SUCCESS;
        }
    }
}

/// The reply to one line of input, if it needs one.
pub fn handle_line(line: &[u8]) -> Option<String> {
    let Ok(text) = std::str::from_utf8(line) else {
        return Some(error("null", PARSE_ERROR, "the message is not valid UTF-8"));
    };
    let text = text.trim_matches(|c: char| c.is_ascii_whitespace() || c == '\u{feff}');
    if text.is_empty() {
        return None;
    }
    match json::parse(text) {
        Err(e) => Some(error("null", PARSE_ERROR, &format!("invalid JSON: {e}"))),
        Ok(Value::Array(items)) if items.is_empty() => {
            Some(error("null", INVALID_REQUEST, "empty batch"))
        }
        Ok(Value::Array(items)) => {
            let replies: Vec<String> = items.iter().filter_map(handle).collect();
            (!replies.is_empty()).then(|| json::array(replies))
        }
        Ok(message) => handle(&message),
    }
}

/// The reply to one message: `None` for notifications (and for responses,
/// which a client should not send).
fn handle(message: &Value) -> Option<String> {
    if !matches!(message, Value::Object(_)) {
        return Some(error("null", INVALID_REQUEST, "expected a JSON-RPC object"));
    }
    let method = message.get("method");
    let is_response = message.get("result").is_some() || message.get("error").is_some();
    let id = match message.get("id") {
        None if method.is_some() || is_response => return None,
        None => return Some(error("null", INVALID_REQUEST, "missing \"method\"")),
        Some(id @ (Value::String(_) | Value::Number(_))) => id.to_json(),
        Some(_) => {
            return Some(error(
                "null",
                INVALID_REQUEST,
                "\"id\" must be a string or a number",
            ));
        }
    };
    if method.is_none() && is_response {
        return None;
    }
    if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Some(error(&id, INVALID_REQUEST, "\"jsonrpc\" must be \"2.0\""));
    }
    let Some(method) = method.and_then(Value::as_str) else {
        return Some(error(&id, INVALID_REQUEST, "\"method\" must be a string"));
    };
    let no_params = Value::Object(Vec::new());
    let params = match message.get("params") {
        None | Some(Value::Null) => &no_params,
        Some(params @ Value::Object(_)) => params,
        Some(_) => return Some(error(&id, INVALID_PARAMS, "\"params\" must be an object")),
    };
    Some(request(&id, method, params))
}

/// The reply to a request with a valid envelope.
fn request(id: &str, method: &str, params: &Value) -> String {
    let meta = params.get("_meta");
    let modern = match meta.and_then(|m| m.get(VERSION_KEY)) {
        None if method == "server/discover" => {
            return error(
                id,
                INVALID_PARAMS,
                &format!("missing _meta \"{VERSION_KEY}\""),
            );
        }
        None => {
            if let Some(caps) = meta.and_then(|m| m.get(CAPABILITIES_KEY))
                && !matches!(caps, Value::Object(_))
            {
                return error(
                    id,
                    INVALID_PARAMS,
                    &format!("_meta \"{CAPABILITIES_KEY}\" must be an object"),
                );
            }
            false
        }
        Some(Value::String(version)) if !supported(version) => {
            let mut data = json::Object::default();
            data.field("supported", &supported_versions())
                .field("requested", &json::string(version));
            return error_with(
                id,
                UNSUPPORTED_VERSION,
                "Unsupported protocol version",
                Some(&data.finish()),
            );
        }
        Some(Value::String(version)) if version == MODERN => {
            match meta.and_then(|m| m.get(CAPABILITIES_KEY)) {
                None => {
                    return error(
                        id,
                        INVALID_PARAMS,
                        &format!("missing _meta \"{CAPABILITIES_KEY}\""),
                    );
                }
                Some(Value::Object(_)) => true,
                Some(_) => {
                    return error(
                        id,
                        INVALID_PARAMS,
                        &format!("_meta \"{CAPABILITIES_KEY}\" must be an object"),
                    );
                }
            }
        }
        Some(Value::String(_)) => {
            // A supported legacy version in _meta is served with legacy semantics.
            if let Some(caps) = meta.and_then(|m| m.get(CAPABILITIES_KEY))
                && !matches!(caps, Value::Object(_))
            {
                return error(
                    id,
                    INVALID_PARAMS,
                    &format!("_meta \"{CAPABILITIES_KEY}\" must be an object"),
                );
            }
            false
        }
        Some(_) => {
            return error(
                id,
                INVALID_PARAMS,
                &format!("_meta \"{VERSION_KEY}\" must be a string"),
            );
        }
    };
    match method {
        "initialize" if !modern => initialize(id, params),
        "server/discover" => discover(id),
        "ping" => success(id, modern, &[]),
        "tools/list" => tools_list(id, modern),
        "tools/call" => tools_call(id, modern, params),
        _ => error(id, METHOD_NOT_FOUND, &format!("method not found: {method}")),
    }
}

fn supported(version: &str) -> bool {
    version == MODERN || LEGACY.contains(&version)
}

fn supported_versions() -> String {
    json::string_array(std::iter::once(MODERN).chain(LEGACY.iter().copied()))
}

fn server_info() -> String {
    let mut info = json::Object::default();
    info.field("name", "\"domq\"")
        .field("title", "\"domq\"")
        .field("version", &json::string(env!("CARGO_PKG_VERSION")))
        .field("websiteUrl", "\"https://github.com/iamsaquib8/domq\"");
    info.finish()
}

fn capabilities() -> &'static str {
    r#"{"tools":{}}"#
}

/// Legacy `initialize`: answer with the client's revision if it is one we
/// know, else the newest legacy one.
fn initialize(id: &str, params: &Value) -> String {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    let version = requested
        .filter(|v| LEGACY.contains(v))
        .unwrap_or(LEGACY[0]);
    success(
        id,
        false,
        &[
            ("protocolVersion", json::string(version)),
            ("capabilities", capabilities().to_string()),
            ("serverInfo", server_info()),
            ("instructions", json::string(INSTRUCTIONS)),
        ],
    )
}

fn discover(id: &str) -> String {
    success(
        id,
        true,
        &[
            ("supportedVersions", supported_versions()),
            ("capabilities", capabilities().to_string()),
            ("instructions", json::string(INSTRUCTIONS)),
            ("ttlMs", TTL_MS.to_string()),
            ("cacheScope", "\"public\"".to_string()),
        ],
    )
}

fn tools_list(id: &str, modern: bool) -> String {
    let tools = json::array(TOOLS.iter().map(Tool::definition));
    let mut fields = vec![("tools", tools)];
    if modern {
        fields.push(("ttlMs", TTL_MS.to_string()));
        fields.push(("cacheScope", "\"public\"".to_string()));
    }
    success(id, modern, &fields)
}

fn tools_call(id: &str, modern: bool, params: &Value) -> String {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error(id, INVALID_PARAMS, "\"name\" must be a string");
    };
    let Some(tool) = TOOLS.iter().find(|t| t.name == name) else {
        return error(id, INVALID_PARAMS, &format!("unknown tool: {name}"));
    };
    let members = match params.get("arguments") {
        None | Some(Value::Null) => &[][..],
        Some(Value::Object(members)) => &members[..],
        Some(_) => return error(id, INVALID_PARAMS, "\"arguments\" must be an object"),
    };
    let args = Args { members };
    let outcome = args.check_known(tool).and_then(|()| (tool.run)(&args));
    let (output, is_error) = match outcome {
        Ok(output) => (output, false),
        Err(message) => (Output::text(message), true),
    };
    let mut text = json::Object::default();
    text.field("type", "\"text\"")
        .field("text", &json::string(&output.text));
    let mut fields = vec![("content", json::array([text.finish()]))];
    if let Some(structured) = output.structured {
        fields.push(("structuredContent", structured));
    }
    fields.push(("isError", is_error.to_string()));
    success(id, modern, &fields)
}

/// A result response. Modern results also carry `resultType` and the
/// server's identity in `_meta`.
fn success(id: &str, modern: bool, fields: &[(&str, String)]) -> String {
    let mut result = json::Object::default();
    if modern {
        result.field("resultType", "\"complete\"");
    }
    for (key, value) in fields {
        result.field(key, value);
    }
    if modern {
        let mut meta = json::Object::default();
        meta.field("io.modelcontextprotocol/serverInfo", &server_info());
        result.field("_meta", &meta.finish());
    }
    let mut response = json::Object::default();
    response
        .field("jsonrpc", "\"2.0\"")
        .field("id", id)
        .field("result", &result.finish());
    response.finish()
}

fn error(id: &str, code: i32, message: &str) -> String {
    error_with(id, code, message, None)
}

fn error_with(id: &str, code: i32, message: &str, data: Option<&str>) -> String {
    let mut err = json::Object::default();
    err.field("code", &code.to_string())
        .field("message", &json::string(message));
    if let Some(data) = data {
        err.field("data", data);
    }
    let mut response = json::Object::default();
    response
        .field("jsonrpc", "\"2.0\"")
        .field("id", id)
        .field("error", &err.finish());
    response.finish()
}

/// What a tool returns: text for the model and, for JSON results, the same
/// value as `structuredContent`.
struct Output {
    text: String,
    structured: Option<String>,
}

impl Output {
    fn text(text: String) -> Output {
        Output {
            text,
            structured: None,
        }
    }

    /// A JSON result, also sent as text for clients that only read text.
    fn json(value: String) -> Output {
        Output {
            text: value.clone(),
            structured: Some(value),
        }
    }
}

/// A tool call's arguments, read with type checks. Input mistakes are tool
/// errors (not protocol errors), so the model sees them and can retry.
struct Args<'a> {
    members: &'a [(String, Value)],
}

impl Args<'_> {
    /// The argument `key`; `null` counts as absent.
    fn get(&self, key: &str) -> Option<&Value> {
        self.members
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
            .filter(|v| **v != Value::Null)
    }

    /// Reject arguments the tool's input schema does not list.
    fn check_known(&self, tool: &Tool) -> Result<(), String> {
        let schema = json::parse(tool.input).unwrap_or(Value::Null);
        let known = schema.get("properties");
        match self
            .members
            .iter()
            .find(|(k, _)| known.and_then(|p| p.get(k)).is_none())
        {
            Some((key, _)) => Err(format!(
                "unknown argument `{key}` for {}; expected: {}",
                tool.name,
                match known {
                    Some(Value::Object(props)) => props
                        .iter()
                        .map(|(k, _)| k.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    _ => String::new(),
                }
            )),
            None => Ok(()),
        }
    }

    fn str(&self, key: &str) -> Result<Option<&str>, String> {
        match self.get(key) {
            None => Ok(None),
            Some(Value::String(s)) => Ok(Some(s)),
            Some(_) => Err(format!("`{key}` must be a string")),
        }
    }

    fn required_str(&self, key: &str) -> Result<&str, String> {
        self.str(key)?
            .ok_or_else(|| format!("missing required argument `{key}`"))
    }

    fn base_url(&self) -> Result<Option<&str>, String> {
        let Some(url) = self.str("base_url")? else {
            return Ok(None);
        };
        if !crate::args::is_absolute_url(url) {
            return Err("`base_url` must be an absolute URL (scheme://…)".into());
        }
        Ok(Some(url))
    }

    /// A selector argument; blank counts as absent.
    fn selector(&self, key: &str) -> Result<Option<&str>, String> {
        Ok(self.str(key)?.filter(|s| !s.trim().is_empty()))
    }

    fn flag(&self, key: &str) -> Result<bool, String> {
        match self.get(key) {
            None => Ok(false),
            Some(Value::Bool(b)) => Ok(*b),
            Some(_) => Err(format!("`{key}` must be true or false")),
        }
    }

    fn int(&self, key: &str, default: usize, min: usize, max: usize) -> Result<usize, String> {
        let bad = || format!("`{key}` must be an integer from {min} to {max}");
        match self.get(key) {
            None => Ok(default),
            Some(Value::Number(n)) => {
                let n = n.parse::<u64>().ok().or_else(|| {
                    // `2000.0` or `2e3` from a model that writes floats.
                    n.parse::<f64>()
                        .ok()
                        .filter(|f| f.fract() == 0.0 && (0.0..=1e15).contains(f))
                        .map(|f| f as u64)
                });
                n.and_then(|n| usize::try_from(n).ok())
                    .filter(|n| (min..=max).contains(n))
                    .ok_or_else(bad)
            }
            Some(_) => Err(bad()),
        }
    }
}

/// One tool: its listing and its implementation.
struct Tool {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    /// The input JSON Schema; its `properties` are the accepted arguments.
    input: &'static str,
    output: Option<&'static str>,
    run: fn(&Args<'_>) -> Result<Output, String>,
}

impl Tool {
    fn definition(&self) -> String {
        let mut def = json::Object::default();
        def.field("name", &json::string(self.name))
            .field("title", &json::string(self.title))
            .field("description", &json::string(self.description))
            .field("inputSchema", self.input);
        if let Some(output) = self.output {
            def.field("outputSchema", output);
        }
        def.field(
            "annotations",
            r#"{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}"#,
        );
        def.finish()
    }
}

// Argument schemas shared by several tools.
macro_rules! html_arg {
    () => {
        r#""html":{"type":"string","description":"The HTML document or fragment. Any quality: broken markup is repaired, never rejected."}"#
    };
}
macro_rules! selector_arg {
    () => {
        r#""selector":{"type":"string","description":"CSS selector. When given, only the elements it matches are used."}"#
    };
}
macro_rules! main_arg {
    () => {
        r#""main":{"type":"boolean","default":false,"description":"First keep only the main content: the article, without navigation, sidebars and footer."}"#
    };
}
macro_rules! base_arg {
    () => {
        r#""base_url":{"type":"string","description":"The page's URL. Relative links are resolved against it (and against the page's <base href>)."}"#
    };
}
macro_rules! markdown_options_args {
    () => {
        concat!(
            r#""links":{"type":"string","enum":["inline","reference","footer"],"default":"inline","description":"Link formatting style: 'inline' ([text](url)), 'reference' ([text][1]), or 'footer' (summary list at end of document). Ignored by `chunks`, which always writes links inline so each chunk stands alone."},"#,
            r#""images":{"type":"string","enum":["keep","alt","drop"],"default":"keep","description":"Image formatting style: 'keep' (![alt](url)), 'alt' (only alt text), or 'drop' (omit images)."},"#,
            r#""drop_data_uris":{"type":"boolean","default":false,"description":"Strip data: URIs in links and images to reduce token usage."}"#
        )
    };
}

const TOOLS: &[Tool] = &[
    Tool {
        name: "markdown",
        title: "HTML to Markdown",
        description: "Convert the page, or each element matching `selector`, to clean \
            CommonMark/GFM Markdown: headings, lists, links, tables, code blocks. The \
            best way to read a page; set main=true for articles and docs.",
        input: concat!(
            r#"{"type":"object","properties":{"#,
            html_arg!(),
            ",",
            selector_arg!(),
            ",",
            main_arg!(),
            ",",
            base_arg!(),
            ",",
            markdown_options_args!(),
            r#"},"required":["html"],"additionalProperties":false}"#
        ),
        output: None,
        run: markdown,
    },
    Tool {
        name: "text",
        title: "HTML to text",
        description: "Readable text of the page, or of each element matching `selector` \
            (separated by blank lines): no tags, scripts or styles, with block \
            structure kept as line breaks.",
        input: concat!(
            r#"{"type":"object","properties":{"#,
            html_arg!(),
            ",",
            selector_arg!(),
            ",",
            main_arg!(),
            r#"},"required":["html"],"additionalProperties":false}"#
        ),
        output: None,
        run: text,
    },
    Tool {
        name: "outline",
        title: "Page outline",
        description: "Semantic page outline of landmarks, headings, interactive controls \
            and forms, with synthesized unique selectors and accessible names. Returns \
            both a compact indented text outline and structured nodes.",
        input: concat!(
            r#"{"type":"object","properties":{"#,
            html_arg!(),
            ",",
            selector_arg!(),
            r#"},"required":["html"],"additionalProperties":false}"#
        ),
        output: Some(
            r#"{"type":"object","properties":{"count":{"type":"integer"},"outline":{"type":"array","items":{"type":"object","properties":{"ref":{"type":"integer"},"role":{"type":"string"},"name":{"type":["string","null"]},"level":{"type":["integer","null"]},"selector":{"type":"string"},"href":{"type":["string","null"]},"tag":{"type":"string"},"depth":{"type":"integer"},"source_range":{"type":["array","null"],"items":{"type":"integer"}}},"required":["ref","role","selector","tag","depth"]}}},"required":["count","outline"]}"#,
        ),
        run: outline,
    },
    Tool {
        name: "select",
        title: "Select elements",
        description: "Find the elements matching a CSS selector. Returns how many \
            matched and, for each (up to `limit`), its tag, attributes and readable \
            text, plus its outer HTML with include_html. Supports combinators, \
            attribute operators, :nth-child(), :not(), :is(), :has() and \
            :contains(\"text\").",
        input: concat!(
            r#"{"type":"object","properties":{"#,
            html_arg!(),
            r#","selector":{"type":"string","description":"CSS selector, e.g. \"article h2 > a[href]\"."}"#,
            r#","limit":{"type":"integer","minimum":1,"maximum":1000,"default":50,"description":"Return at most this many matches; count still reports all of them."}"#,
            r#","include_html":{"type":"boolean","default":false,"description":"Also return each match's outer HTML."}"#,
            r#","max_chars":{"type":"integer","minimum":0,"default":1000,"description":"Cut each match's text and HTML to this many characters (0 = no limit)."}"#,
            r#"},"required":["html","selector"],"additionalProperties":false}"#
        ),
        output: Some(
            r#"{"type":"object","properties":{"count":{"type":"integer"},"truncated":{"type":"boolean"},"matches":{"type":"array","items":{"type":"object","properties":{"tag":{"type":"string"},"attrs":{"type":"object","additionalProperties":{"type":"string"}},"text":{"type":"string"},"html":{"type":"string"}},"required":["tag","attrs","text"]}}},"required":["count","truncated","matches"]}"#,
        ),
        run: select,
    },
    Tool {
        name: "extract",
        title: "Extract fields",
        description: "Extract named fields as JSON. Each field's spec is a CSS selector \
            (the text of its first match), selector@attr (that match's attribute), or \
            either with a trailing [] (every match, as an array); an empty selector \
            means the element itself. With `selector`, returns one object per matching \
            element (one per product card, row, ...); otherwise one object for the page.",
        input: concat!(
            r#"{"type":"object","properties":{"#,
            html_arg!(),
            r#","fields":{"type":"object","additionalProperties":{"type":"string"},"minProperties":1,"description":"Field name to spec, e.g. {\"title\": \"h1\", \"url\": \"a@href\", \"tags\": \".tag[]\"}."}"#,
            r#","selector":{"type":"string","description":"CSS selector for the repeated items; each match gives one object, with the fields looked up inside it."}"#,
            ",",
            base_arg!(),
            r#"},"required":["html","fields"],"additionalProperties":false}"#
        ),
        output: Some(
            r#"{"type":"object","properties":{"results":{"type":"array","items":{"type":"object"}}},"required":["results"]}"#,
        ),
        run: extract_fields,
    },
    Tool {
        name: "chunks",
        title: "Markdown chunks",
        description: "Split the page's Markdown into chunks of at most max_chars \
            characters, for embeddings and RAG. Splits at blank lines (never inside a \
            code block) and starts sections at headings; each chunk carries the \
            headings it sits under.",
        input: concat!(
            r#"{"type":"object","properties":{"#,
            html_arg!(),
            r#","max_chars":{"type":"integer","minimum":1,"maximum":1000000,"default":2000,"description":"The largest chunk, in characters."}"#,
            ",",
            selector_arg!(),
            ",",
            main_arg!(),
            ",",
            base_arg!(),
            ",",
            markdown_options_args!(),
            r#"},"required":["html"],"additionalProperties":false}"#
        ),
        output: Some(
            r#"{"type":"object","properties":{"chunks":{"type":"array","items":{"type":"object","properties":{"context":{"type":"string","description":"The heading path joined with \" > \"."},"heading_path":{"type":"array","items":{"type":"string"}},"markdown":{"type":"string"}},"required":["context","heading_path","markdown"]}}},"required":["chunks"]}"#,
        ),
        run: chunks,
    },
    Tool {
        name: "links",
        title: "List links",
        description: "List the page's links (<a href> and <area href>) with their \
            readable text, in document order. URLs are resolved against base_url and \
            the page's <base href>.",
        input: concat!(
            r#"{"type":"object","properties":{"#,
            html_arg!(),
            r#","selector":{"type":"string","description":"Only the links inside elements matching this CSS selector."}"#,
            ",",
            base_arg!(),
            r#"},"required":["html"],"additionalProperties":false}"#
        ),
        output: Some(
            r#"{"type":"object","properties":{"count":{"type":"integer"},"links":{"type":"array","items":{"type":"object","properties":{"url":{"type":"string"},"text":{"type":"string"}},"required":["url","text"]}}},"required":["count","links"]}"#,
        ),
        run: links,
    },
    Tool {
        name: "metadata",
        title: "Page metadata",
        description: "The page's metadata: title, description, canonical URL, \
            language, site name, image, author, published date, keywords, OpenGraph \
            and Twitter tags, JSON-LD blocks and feeds.",
        input: concat!(
            r#"{"type":"object","properties":{"#,
            html_arg!(),
            ",",
            base_arg!(),
            r#"},"required":["html"],"additionalProperties":false}"#
        ),
        output: Some(
            r#"{"type":"object","properties":{"title":{"type":["string","null"]},"description":{"type":["string","null"]},"canonical":{"type":["string","null"]},"lang":{"type":["string","null"]},"site_name":{"type":["string","null"]},"image":{"type":["string","null"]},"author":{"type":["string","null"]},"published":{"type":["string","null"]},"keywords":{"type":"array","items":{"type":"string"}},"og":{"type":"array","items":{"type":"array","items":{"type":"string"}}},"twitter":{"type":"array","items":{"type":"array","items":{"type":"string"}}},"json_ld":{"type":"array","items":{"type":"string"}},"feeds":{"type":"array","items":{"type":"string"}}}}"#,
        ),
        run: metadata,
    },
    Tool {
        name: "tables",
        title: "Read tables",
        description: "Read <table> elements as rows of cell text, with colspan and \
            rowspan expanded. All tables by default, or those matching `selector`.",
        input: concat!(
            r#"{"type":"object","properties":{"#,
            html_arg!(),
            r#","selector":{"type":"string","description":"CSS selector for the tables to read (default: table)."}"#,
            r#"},"required":["html"],"additionalProperties":false}"#
        ),
        output: Some(
            r#"{"type":"object","properties":{"tables":{"type":"array","items":{"type":"object","properties":{"rows":{"type":"array","items":{"type":"array","items":{"type":"string"}}}},"required":["rows"]}}},"required":["tables"]}"#,
        ),
        run: tables,
    },
    Tool {
        name: "lint",
        title: "Lint HTML",
        description: "Check HTML for accessibility and markup problems: images without \
            alt, empty links and buttons, duplicate ids, skipped heading levels, and \
            unclosed or mis-nested tags. Each finding has a line, column, level, rule \
            and message.",
        input: concat!(
            r#"{"type":"object","properties":{"#,
            html_arg!(),
            r#","include_notes":{"type":"boolean","default":false,"description":"Also report notes: omitted optional end tags and unknown entities, which are valid HTML."}"#,
            r#"},"required":["html"],"additionalProperties":false}"#
        ),
        output: Some(
            r#"{"type":"object","properties":{"errors":{"type":"integer"},"warnings":{"type":"integer"},"notes":{"type":"integer"},"findings":{"type":"array","items":{"type":"object","properties":{"line":{"type":"integer"},"column":{"type":"integer"},"level":{"type":"string","enum":["error","warning","note"]},"rule":{"type":"string"},"message":{"type":"string"}},"required":["line","column","level","rule","message"]}}},"required":["errors","warnings","notes","findings"]}"#,
        ),
        run: lint_html,
    },
];

/// The parsed `html`: URLs made absolute when `base_url` is given, then
/// narrowed to the main content with `main`.
fn document(args: &Args<'_>) -> Result<Dom, String> {
    let mut dom = domq::parse(args.required_str("html")?);
    if let Some(page) = args.base_url()?
        && let Some(base) = dom.base_url(Some(page))
    {
        crate::absolutize(&mut dom, &base, Some(page));
    }
    if args.flag("main")?
        && let Some(main) = dom.main_content().map(|m| m.to_html())
    {
        dom = domq::parse(&main);
    }
    Ok(dom)
}

/// The elements matching the `selector` argument, or `None` without one.
fn selected<'d>(dom: &'d Dom, args: &Args<'_>) -> Result<Option<Vec<NodeRef<'d>>>, String> {
    match args.selector("selector")? {
        None => Ok(None),
        Some(s) => Ok(Some(dom.select(&crate::compile(s)?).collect())),
    }
}

fn no_match() -> Output {
    Output::text("No elements match the selector.".to_string())
}

/// The non-blank parts, separated by a blank line.
fn join_blocks(parts: impl Iterator<Item = String>) -> String {
    parts
        .filter(|p| !p.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn parse_markdown_options(args: &Args<'_>) -> Result<domq::MarkdownOptions, String> {
    let mut opts = domq::MarkdownOptions::default();
    if let Some(links) = args.str("links")? {
        opts.link_style = match links.to_ascii_lowercase().as_str() {
            "inline" => domq::LinkStyle::Inline,
            "reference" => domq::LinkStyle::Reference,
            "footer" => domq::LinkStyle::Footer,
            other => {
                return Err(format!(
                    "unknown links style `{other}`; expected inline, reference or footer"
                ));
            }
        };
    }
    if let Some(images) = args.str("images")? {
        opts.image_style = match images.to_ascii_lowercase().as_str() {
            "keep" => domq::ImageStyle::Keep,
            "alt" => domq::ImageStyle::AltOnly,
            "drop" => domq::ImageStyle::Drop,
            other => {
                return Err(format!(
                    "unknown images style `{other}`; expected keep, alt or drop"
                ));
            }
        };
    }
    opts.drop_data_uris = args.flag("drop_data_uris")?;
    if let Some(base) = args.base_url()? {
        opts.base_url = Some(base.to_string());
    }
    Ok(opts)
}

fn markdown(args: &Args<'_>) -> Result<Output, String> {
    let dom = document(args)?;
    let opts = parse_markdown_options(args)?;
    Ok(match selected(&dom, args)? {
        None => Output::text(dom.to_markdown_with(&opts)),
        Some(m) if m.is_empty() => no_match(),
        Some(m) => Output::text(join_blocks(m.iter().map(|n| n.to_markdown_with(&opts)))),
    })
}

fn text(args: &Args<'_>) -> Result<Output, String> {
    let dom = document(args)?;
    Ok(match selected(&dom, args)? {
        None => Output::text(dom.to_text()),
        Some(m) if m.is_empty() => no_match(),
        Some(m) => Output::text(join_blocks(m.iter().map(|n| n.to_text()))),
    })
}

fn outline(args: &Args<'_>) -> Result<Output, String> {
    let dom = domq::parse(args.required_str("html")?);
    // Render the text from the renumbered nodes, so the refs the model reads
    // are the refs in `structuredContent`.
    let outline = match args.selector("selector")? {
        None => dom.outline(),
        Some(s) => crate::scoped_outline(&dom, &crate::compile(s)?),
    };
    let mut out = json::Object::default();
    out.field("count", &outline.len().to_string())
        .field("outline", &outline.to_json());
    let text = outline.to_text();
    Ok(Output {
        text,
        structured: Some(out.finish()),
    })
}

/// `s` cut to `max` characters (0 = no limit), marked with `…` when cut.
fn cut(s: String, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((at, _)) if max > 0 => format!("{}…", &s[..at]),
        _ => s,
    }
}

fn select(args: &Args<'_>) -> Result<Output, String> {
    let selector = crate::compile(args.required_str("selector")?)?;
    let limit = args.int("limit", 50, 1, 1000)?;
    let max_chars = args.int("max_chars", 1000, 0, usize::MAX)?;
    let include_html = args.flag("include_html")?;
    let dom = document(args)?;
    let matches: Vec<NodeRef<'_>> = dom.select(&selector).collect();
    let items = matches.iter().take(limit).map(|m| {
        let mut attrs = json::Object::default();
        let mut seen: Vec<&str> = Vec::new();
        for (name, value) in m.attributes() {
            // A repeated attribute counts once, as its first occurrence.
            if !seen.contains(&name) {
                seen.push(name);
                attrs.field(name, &json::string(value));
            }
        }
        let mut item = json::Object::default();
        item.field("tag", &json::opt_string(m.tag_name()))
            .field("attrs", &attrs.finish())
            .field("text", &json::string(&cut(m.to_text(), max_chars)));
        if include_html {
            item.field("html", &json::string(&cut(m.to_html(), max_chars)));
        }
        item.finish()
    });
    let mut out = json::Object::default();
    out.field("count", &matches.len().to_string())
        .field("truncated", &(matches.len() > limit).to_string())
        .field("matches", &json::array(items));
    Ok(Output::json(out.finish()))
}

fn extract_fields(args: &Args<'_>) -> Result<Output, String> {
    let fields: Vec<Field> = match args.get("fields") {
        Some(Value::Object(members)) if !members.is_empty() => members
            .iter()
            .map(|(name, spec)| match spec {
                Value::String(spec) => Ok(Field::new(name, spec)),
                _ => Err(format!("field `{name}`: the spec must be a string")),
            })
            .collect::<Result<_, _>>()?,
        Some(Value::Object(_)) => return Err("`fields` needs at least one field".to_string()),
        Some(_) => return Err("`fields` must be an object of name to spec".to_string()),
        None => return Err("missing required argument `fields`".to_string()),
    };
    let compiled = extract::compile(&fields).map_err(|e| e.replacen("--extract ", "field ", 1))?;
    let dom = document(args)?;
    let results = match selected(&dom, args)? {
        None => vec![extract::object(&dom, &compiled, false)],
        Some(scopes) => extract::scoped_objects(&dom, &scopes, &compiled, false),
    };
    let mut out = json::Object::default();
    out.field("results", &json::array(results));
    Ok(Output::json(out.finish()))
}

fn chunks(args: &Args<'_>) -> Result<Output, String> {
    let max_chars = args.int("max_chars", 2000, 1, 1_000_000)?;
    let dom = document(args)?;
    let opts = parse_markdown_options(args)?;
    let chunks = match selected(&dom, args)? {
        None => dom.to_markdown_chunks_with(max_chars, &opts),
        Some(m) if m.is_empty() => return Ok(no_match()),
        Some(m) => m
            .iter()
            .flat_map(|n| n.to_markdown_chunks_with(max_chars, &opts))
            .collect(),
    };
    let items = chunks.iter().map(|c| {
        let mut item = json::Object::default();
        item.field("context", &json::string(&c.context()))
            .field(
                "heading_path",
                &json::string_array(c.heading_path.iter().map(String::as_str)),
            )
            .field("markdown", &json::string(&c.markdown));
        item.finish()
    });
    let mut out = json::Object::default();
    out.field("chunks", &json::array(items));
    Ok(Output::json(out.finish()))
}

fn links(args: &Args<'_>) -> Result<Output, String> {
    let dom = domq::parse(args.required_str("html")?);
    let inside = match args.selector("selector")? {
        Some(s) => Some(crate::inside_matches(&dom, &crate::compile(s)?)),
        None => None,
    };
    let links: Vec<String> = dom
        .links(args.base_url()?)
        .into_iter()
        .filter(|l| {
            inside
                .as_ref()
                .is_none_or(|marks| marks.get(l.node.id().index()).copied().unwrap_or(false))
        })
        .map(|l| {
            let mut link = json::Object::default();
            link.field("url", &json::string(&l.url))
                .field("text", &json::string(&l.text));
            link.finish()
        })
        .collect();
    let mut out = json::Object::default();
    out.field("count", &links.len().to_string())
        .field("links", &json::array(links));
    Ok(Output::json(out.finish()))
}

fn metadata(args: &Args<'_>) -> Result<Output, String> {
    let dom = domq::parse(args.required_str("html")?);
    let base = args.base_url()?.and_then(|page| dom.base_url(Some(page)));
    Ok(Output::json(crate::meta_json(&dom, base.as_deref())))
}

fn tables(args: &Args<'_>) -> Result<Output, String> {
    let dom = domq::parse(args.required_str("html")?);
    let tables = match selected(&dom, args)? {
        Some(m) if m.is_empty() => return Ok(no_match()),
        Some(m) => crate::matched_tables(&m),
        None => dom.select("table").collect(),
    };
    let tables = tables
        .into_iter()
        .map(|t| t.table_rows())
        .filter(|rows| !rows.is_empty())
        .map(|rows| {
            let rows = rows
                .iter()
                .map(|row| json::string_array(row.iter().map(String::as_str)));
            let mut table = json::Object::default();
            table.field("rows", &json::array(rows));
            table.finish()
        });
    let mut out = json::Object::default();
    out.field("tables", &json::array(tables));
    Ok(Output::json(out.finish()))
}

fn lint_html(args: &Args<'_>) -> Result<Output, String> {
    let findings = lint::check(args.required_str("html")?);
    let include_notes = args.flag("include_notes")?;
    let count = |level| findings.iter().filter(|f| f.level == level).count();
    let items = findings
        .iter()
        .filter(|f| include_notes || f.level != Level::Note)
        .map(|f| {
            let mut item = json::Object::default();
            item.field("line", &f.line.to_string())
                .field("column", &f.column.to_string())
                .field("level", &json::string(f.level.as_str()))
                .field("rule", &json::string(f.rule))
                .field("message", &json::string(&f.message));
            item.finish()
        });
    let mut out = json::Object::default();
    out.field("errors", &count(Level::Error).to_string())
        .field("warnings", &count(Level::Warning).to_string())
        .field("notes", &count(Level::Note).to_string())
        .field("findings", &json::array(items));
    Ok(Output::json(out.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Send one message, parse the reply.
    fn send(message: &str) -> Value {
        let reply = handle_line(message.as_bytes()).expect("a reply");
        assert!(!reply.contains('\n'), "replies are one line: {reply}");
        json::parse(&reply).expect("the reply is JSON")
    }

    fn modern(id: u32, method: &str, params: &str) -> String {
        let meta = format!(
            r#""_meta":{{"{VERSION_KEY}":"{MODERN}","{CAPABILITIES_KEY}":{{}},"io.modelcontextprotocol/clientInfo":{{"name":"test","version":"1"}}}}"#
        );
        let params = if params.is_empty() {
            format!("{{{meta}}}")
        } else {
            format!("{{{params},{meta}}}")
        };
        format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#)
    }

    fn call(tool: &str, arguments: &str) -> Value {
        let reply = send(&modern(
            9,
            "tools/call",
            &format!(r#""name":"{tool}","arguments":{arguments}"#),
        ));
        reply.get("result").cloned().expect("a result")
    }

    fn text_of(result: &Value) -> String {
        match result.get("content") {
            Some(Value::Array(items)) => items[0]
                .get("text")
                .and_then(Value::as_str)
                .unwrap()
                .to_string(),
            _ => panic!("no content: {result:?}"),
        }
    }

    fn is_error(result: &Value) -> bool {
        result.get("isError") == Some(&Value::Bool(true))
    }

    #[test]
    fn every_schema_is_valid_json() {
        for tool in TOOLS {
            let input = json::parse(tool.input).unwrap_or_else(|e| panic!("{}: {e}", tool.name));
            assert_eq!(input.get("type").and_then(Value::as_str), Some("object"));
            assert!(
                input
                    .get("properties")
                    .and_then(|p| p.get("html"))
                    .is_some()
            );
            if let Some(output) = tool.output {
                let output = json::parse(output).unwrap_or_else(|e| panic!("{}: {e}", tool.name));
                assert_eq!(output.get("type").and_then(Value::as_str), Some("object"));
            }
            assert!(json::parse(&tool.definition()).is_ok());
        }
    }

    #[test]
    fn discover() {
        let result = send(&modern(1, "server/discover", ""))
            .get("result")
            .cloned()
            .unwrap();
        assert_eq!(
            result.get("resultType").and_then(Value::as_str),
            Some("complete")
        );
        let versions = result.get("supportedVersions").unwrap().to_json();
        assert!(
            versions.starts_with(r#"["2026-07-28","2025-11-25""#),
            "{versions}"
        );
        assert_eq!(
            result.get("cacheScope").and_then(Value::as_str),
            Some("public")
        );
        let info = result
            .get("_meta")
            .and_then(|m| m.get("io.modelcontextprotocol/serverInfo"))
            .unwrap();
        assert_eq!(info.get("name").and_then(Value::as_str), Some("domq"));
    }

    #[test]
    fn version_errors() {
        let reply = send(
            r#"{"jsonrpc":"2.0","id":"a","method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"1900-01-01","io.modelcontextprotocol/clientCapabilities":{}}}}"#,
        );
        assert_eq!(reply.get("id"), Some(&Value::String("a".into())));
        let err = reply.get("error").unwrap();
        assert_eq!(err.get("code"), Some(&Value::Number("-32022".into())));
        assert_eq!(
            err.get("data")
                .and_then(|d| d.get("requested"))
                .and_then(Value::as_str),
            Some("1900-01-01")
        );

        let missing_caps = send(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}}"#,
        );
        assert_eq!(
            missing_caps.get("error").and_then(|e| e.get("code")),
            Some(&Value::Number("-32602".into()))
        );

        let non_object_caps = send(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":"nope"}}}"#,
        );
        assert_eq!(
            non_object_caps.get("error").and_then(|e| e.get("code")),
            Some(&Value::Number("-32602".into()))
        );

        let non_object_legacy_caps = send(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2025-06-18","io.modelcontextprotocol/clientCapabilities":"nope"}}}"#,
        );
        assert_eq!(
            non_object_legacy_caps
                .get("error")
                .and_then(|e| e.get("code")),
            Some(&Value::Number("-32602".into()))
        );

        let discover_without_meta = send(r#"{"jsonrpc":"2.0","id":1,"method":"server/discover"}"#);
        assert!(discover_without_meta.get("error").is_some());
    }

    #[test]
    fn legacy_handshake() {
        let reply = send(
            r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"c","version":"1"}}}"#,
        );
        let result = reply.get("result").unwrap();
        assert_eq!(
            result.get("protocolVersion").and_then(Value::as_str),
            Some("2025-06-18")
        );
        assert!(result.get("resultType").is_none());
        assert!(
            result
                .get("capabilities")
                .and_then(|c| c.get("tools"))
                .is_some()
        );

        // A legacy initialize request carrying _meta is served with legacy semantics.
        let legacy_with_meta = send(
            r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"_meta":{"io.modelcontextprotocol/protocolVersion":"2025-06-18","io.modelcontextprotocol/clientCapabilities":{}}}}"#,
        );
        let l_res = legacy_with_meta.get("result").unwrap();
        assert_eq!(
            l_res.get("protocolVersion").and_then(Value::as_str),
            Some("2025-06-18")
        );
        assert!(l_res.get("resultType").is_none());

        // A legacy request declaring a legacy version in _meta has no modern result fields.
        let legacy_tools = send(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2025-06-18","io.modelcontextprotocol/clientCapabilities":{}}}}"#,
        );
        let t_res = legacy_tools.get("result").unwrap();
        assert!(t_res.get("resultType").is_none());
        assert!(t_res.get("_meta").is_none());
        assert!(t_res.get("tools").is_some());

        let newer = send(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2099-01-01"}}"#,
        );
        assert_eq!(
            newer
                .get("result")
                .and_then(|r| r.get("protocolVersion"))
                .and_then(Value::as_str),
            Some("2025-11-25")
        );

        assert_eq!(
            handle_line(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
            None
        );
        let list = send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
        match list.get("result").and_then(|r| r.get("tools")) {
            Some(Value::Array(tools)) => assert_eq!(tools.len(), TOOLS.len()),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn envelope_errors() {
        let code = |reply: Value| reply.get("error").and_then(|e| e.get("code")).cloned();
        assert_eq!(code(send("{nope")), Some(Value::Number("-32700".into())));
        assert_eq!(code(send("[]")), Some(Value::Number("-32600".into())));
        assert_eq!(
            code(send(r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#)),
            Some(Value::Number("-32600".into()))
        );
        assert_eq!(
            code(send(r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#)),
            Some(Value::Number("-32600".into()))
        );
        assert_eq!(
            code(send(r#"{"jsonrpc":"2.0","id":1,"method":"nope"}"#)),
            Some(Value::Number("-32601".into()))
        );
        assert_eq!(
            code(send(
                r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":[]}"#
            )),
            Some(Value::Number("-32602".into()))
        );
        let unknown = send(&modern(3, "tools/call", r#""name":"fetch","arguments":{}"#));
        assert_eq!(code(unknown), Some(Value::Number("-32602".into())));
        assert_eq!(handle_line(b"   \r\n"), None);
        assert_eq!(
            handle_line(br#"{"jsonrpc":"2.0","id":5,"result":{}}"#),
            None
        );
        assert!(handle_line(b"\xff").unwrap().contains("-32700"));
    }

    #[test]
    fn batches() {
        let reply = handle_line(
            br#"[{"jsonrpc":"2.0","id":1,"method":"ping"},{"jsonrpc":"2.0","method":"notifications/initialized"},{"jsonrpc":"2.0","id":2,"method":"ping"}]"#,
        )
        .unwrap();
        assert_eq!(
            reply,
            r#"[{"jsonrpc":"2.0","id":1,"result":{}},{"jsonrpc":"2.0","id":2,"result":{}}]"#
        );
    }

    const PAGE: &str = r#"<html lang=en><head><title>Shop</title><meta name=description content="Things"><base href="/shop/"></head><body>
        <nav><a href="/">Home</a></nav>
        <main><h1>Gadgets</h1>
          <div class=card><h2>Widget</h2><span class=price data-value=9.5>$9.50</span><a href="widget">More</a><span class=tag>new</span><span class=tag>blue</span></div>
          <div class=card><h2>Gizmo</h2><span class=price data-value=12>$12</span><a href="gizmo">More</a></div>
          <table><tr><th>A<th>B<tr><td colspan=2>x</table>
          <img src=a.png>
        </main></body></html>"#;

    fn page_args(extra: &str) -> String {
        format!(r#"{{"html":{}{extra}}}"#, json::string(PAGE))
    }

    #[test]
    fn select_tool() {
        let result = call(
            "select",
            &page_args(r#","selector":".card h2","limit":1,"include_html":true"#),
        );
        assert!(!is_error(&result));
        let s = result.get("structuredContent").unwrap();
        assert_eq!(s.get("count"), Some(&Value::Number("2".into())));
        assert_eq!(s.get("truncated"), Some(&Value::Bool(true)));
        assert_eq!(text_of(&result), s.to_json());
        let first = match s.get("matches") {
            Some(Value::Array(m)) => m[0].clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(first.get("text").and_then(Value::as_str), Some("Widget"));
        assert_eq!(
            first.get("html").and_then(Value::as_str),
            Some("<h2>Widget</h2>")
        );

        let cut = call("select", &page_args(r#","selector":"h1","max_chars":3"#));
        assert!(
            text_of(&cut).contains(r#""text":"Gad…""#),
            "{}",
            text_of(&cut)
        );
    }

    #[test]
    fn selector_errors_are_tool_errors() {
        let result = call("select", &page_args(r#","selector":"a:hover""#));
        assert!(is_error(&result));
        assert!(
            text_of(&result).contains("invalid selector"),
            "{}",
            text_of(&result)
        );
        let missing = call("select", &page_args(""));
        assert!(is_error(&missing));
        assert!(text_of(&missing).contains("missing required argument `selector`"));
        let unknown = call("text", &page_args(r#","selecter":"h1""#));
        assert!(
            text_of(&unknown).contains("unknown argument `selecter`"),
            "{}",
            text_of(&unknown)
        );
        let wrong_type = call("select", &page_args(r#","selector":"h1","limit":"5""#));
        assert!(text_of(&wrong_type).contains("`limit` must be an integer from 1 to 1000"));
    }

    #[test]
    fn base_url_errors_are_tool_errors() {
        for bad in ["not a url", "/relative/", "htps://", "https://"] {
            for tool in ["links", "metadata", "markdown", "chunks", "extract"] {
                let args = match tool {
                    "extract" => {
                        format!(r#"{{"html":"<p>x</p>","fields":{{"t":"p"}},"base_url":"{bad}"}}"#)
                    }
                    _ => format!(r#"{{"html":"<a href=/a>A</a>","base_url":"{bad}"}}"#),
                };
                let res = call(tool, &args);
                assert!(is_error(&res), "{tool} should error on base_url={bad}");
                assert!(
                    text_of(&res).contains("`base_url` must be an absolute URL (scheme://…)"),
                    "unexpected error message for {tool}: {}",
                    text_of(&res)
                );
            }
        }
    }

    #[test]
    fn markdown_and_text_tools() {
        let md = call(
            "markdown",
            &page_args(r#","main":true,"base_url":"https://example.com/""#),
        );
        let md = text_of(&md);
        assert!(md.starts_with("# Gadgets"), "{md}");
        assert!(
            md.contains("[More](https://example.com/shop/widget)"),
            "{md}"
        );
        assert!(!md.contains("Home"), "{md}");

        let text = call("text", &page_args(r#","selector":".card h2""#));
        assert_eq!(text_of(&text), "Widget\n\nGizmo");
        let none = call("text", &page_args(r#","selector":"video""#));
        assert_eq!(text_of(&none), "No elements match the selector.");
        let none_chunks = call("chunks", &page_args(r#","selector":"video""#));
        assert_eq!(text_of(&none_chunks), "No elements match the selector.");
        let none_tables = call("tables", &page_args(r#","selector":"video""#));
        assert_eq!(text_of(&none_tables), "No elements match the selector.");
    }

    #[test]
    fn outline_tool() {
        let result = call("outline", &page_args(""));
        assert!(!is_error(&result));
        let text = text_of(&result);
        assert!(text.contains("[1] navigation"));
        assert!(text.contains("link \"Home\" -> nav > a"));
        assert!(text.contains("heading 1 \"Gadgets\""));
        assert!(text.contains("heading 2 \"Widget\""));

        let structured = result.get("structuredContent").unwrap();
        assert_eq!(structured.get("count"), Some(&Value::Number("8".into())));
        let outline = structured.get("outline").unwrap();
        match outline {
            Value::Array(items) => {
                assert_eq!(items.len(), 8);
                assert_eq!(items[0].get("ref"), Some(&Value::Number("1".into())));
                assert_eq!(
                    items[0].get("role").and_then(Value::as_str),
                    Some("navigation")
                );
                assert_eq!(items[1].get("ref"), Some(&Value::Number("2".into())));
                assert_eq!(items[1].get("role").and_then(Value::as_str), Some("link"));
                assert_eq!(items[1].get("name").and_then(Value::as_str), Some("Home"));
            }
            other => panic!("{other:?}"),
        }

        // Scoped outline
        let scoped = call("outline", &page_args(r#","selector":"nav""#));
        assert!(!is_error(&scoped));
        let s_text = text_of(&scoped);
        assert!(s_text.contains("navigation"));
        assert!(s_text.contains("link \"Home\""));
        assert!(!s_text.contains("Gadgets"));
        let s_structured = scoped.get("structuredContent").unwrap();
        assert_eq!(s_structured.get("count"), Some(&Value::Number("2".into())));
        assert_eq!(s_structured.get("nodes"), None);

        // Nested matches: each control once, and the text refs are the
        // structured refs.
        let nested = call(
            "outline",
            r#"{"html":"<nav><a href=/>Home</a><a href=/b>B</a></nav>","selector":"nav, nav a"}"#,
        );
        assert_eq!(
            text_of(&nested),
            "[1] navigation\n[2]   link \"Home\" -> nav > a:nth-of-type(1)\n[3]   link \"B\" -> nav > a:nth-of-type(2)"
        );
        let n_structured = nested.get("structuredContent").unwrap();
        assert_eq!(n_structured.get("count"), Some(&Value::Number("3".into())));

        // Invalid selector
        let bad = call("outline", &page_args(r#","selector":"a:hover""#));
        assert!(is_error(&bad));
    }

    #[test]
    fn extract_tool() {
        let result = call(
            "extract",
            &page_args(
                r#","selector":".card","fields":{"name":"h2","price":".price@data-value","tags":".tag[]","url":"a@href"},"base_url":"https://example.com/""#,
            ),
        );
        assert!(!is_error(&result), "{}", text_of(&result));
        assert_eq!(
            text_of(&result),
            r#"{"results":[{"name":"Widget","price":"9.5","tags":["new","blue"],"url":"https://example.com/shop/widget"},{"name":"Gizmo","price":"12","tags":[],"url":"https://example.com/shop/gizmo"}]}"#
        );
        let bad = call("extract", &page_args(r#","fields":{"x":"a:hover"}"#));
        assert!(is_error(&bad));
        assert!(
            text_of(&bad).starts_with("field x: invalid selector"),
            "{}",
            text_of(&bad)
        );
    }

    #[test]
    fn links_metadata_tables_chunks_lint() {
        let links = call(
            "links",
            &page_args(r#","selector":"main","base_url":"https://example.com/""#),
        );
        assert_eq!(
            text_of(&links),
            r#"{"count":2,"links":[{"url":"https://example.com/shop/widget","text":"More"},{"url":"https://example.com/shop/gizmo","text":"More"}]}"#
        );

        let meta = call("metadata", &page_args(""));
        let meta = meta.get("structuredContent").unwrap();
        assert_eq!(meta.get("title").and_then(Value::as_str), Some("Shop"));
        assert_eq!(meta.get("lang").and_then(Value::as_str), Some("en"));

        let tables = call("tables", &page_args(""));
        assert_eq!(
            text_of(&tables),
            r#"{"tables":[{"rows":[["A","B"],["x","x"]]}]}"#
        );
        let wrapper = call(
            "tables",
            r##"{"html":"<div id=r><table><tr><td>1</table></div>","selector":"#r"}"##,
        );
        assert_eq!(text_of(&wrapper), r#"{"tables":[{"rows":[["1"]]}]}"#);

        let chunks = call("chunks", &page_args(r#","main":true,"max_chars":40"#));
        let chunks = chunks
            .get("structuredContent")
            .and_then(|s| s.get("chunks"))
            .unwrap();
        match chunks {
            Value::Array(items) => {
                assert!(items.len() > 1);
                assert_eq!(
                    items[1].get("context").and_then(Value::as_str),
                    Some("Gadgets > Widget")
                );
            }
            other => panic!("{other:?}"),
        }

        let lint = call("lint", &page_args(""));
        let lint = lint.get("structuredContent").unwrap();
        assert_eq!(
            lint.get("errors"),
            Some(&Value::Number("1".into())),
            "{}",
            lint.to_json()
        );
        assert!(lint.to_json().contains("img-alt"));
    }

    #[test]
    fn legacy_tool_calls_have_no_modern_fields() {
        let reply = send(&format!(
            r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"text","arguments":{}}}}}"#,
            r#"{"html":"<p>Hi</p>"}"#
        ));
        let result = reply.get("result").unwrap();
        assert!(result.get("resultType").is_none() && result.get("_meta").is_none());
        assert_eq!(text_of(result), "Hi");
    }

    #[test]
    fn cut_counts_characters() {
        assert_eq!(cut("héllo".to_string(), 2), "hé…");
        assert_eq!(cut("hé".to_string(), 2), "hé");
        assert_eq!(cut("abc".to_string(), 0), "abc");
    }
}
