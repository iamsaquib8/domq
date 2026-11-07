//! The fields of a page's JSON-LD blocks that [`Dom::metadata`] reads:
//! `headline`, `author`, `datePublished` and `image` of the article.
//!
//! A small JSON reader, with no dependencies, built for hostile input: it is
//! iterative (no recursion, whatever the nesting), refuses nesting beyond
//! [`MAX_DEPTH`] and blocks with more than [`MAX_VALUES`] values, and does
//! linear work. It is lenient where real pages are (raw line breaks inside
//! strings) and strict otherwise: a block that is not JSON is skipped.

use alloc::string::String;
use alloc::vec::Vec;

use super::collapse;

/// Nesting deeper than this is not read.
const MAX_DEPTH: usize = 32;

/// A block with more values than this is not read.
const MAX_VALUES: usize = 200_000;

/// At most this many objects of a block are looked at (those at the top, in
/// an array there, or in `@graph`).
const MAX_OBJECTS: usize = 512;

/// A JSON value; numbers, booleans and `null` are not told apart (nothing
/// here reads them).
enum Json {
    Other,
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
}

/// What the article's JSON-LD says.
#[derive(Default)]
pub(crate) struct Fields {
    pub(crate) headline: Option<String>,
    pub(crate) author: Option<String>,
    pub(crate) published: Option<String>,
    pub(crate) image: Option<String>,
}

/// The fields of the blocks' article-like objects (`Article`, `NewsArticle`,
/// `BlogPosting`, …); for what those lack, any other object that has it. The
/// first value in document order wins.
pub(crate) fn fields(blocks: &[String]) -> Fields {
    let roots: Vec<Json> = blocks.iter().filter_map(|b| parse(b)).collect();
    let mut objects: Vec<&Json> = Vec::new();
    for root in &roots {
        collect(root, &mut objects);
    }
    // Article-like objects first, the rest after, each in document order.
    let (mut ordered, rest): (Vec<&Json>, Vec<&Json>) =
        objects.iter().partition(|o| article_like(o));
    ordered.extend(rest);
    let pick = |read: &dyn Fn(&Json) -> Option<String>| ordered.iter().find_map(|o| read(o));
    Fields {
        headline: pick(&|o| text(o.get("headline")?)),
        author: pick(&|o| author(o.get("author")?, &objects)),
        published: pick(&|o| text(o.get("datePublished")?)),
        image: pick(&|o| image(o.get("image")?)),
    }
}

/// The objects at the top of `root`, in an array there, or in `@graph`.
fn collect<'j>(root: &'j Json, out: &mut Vec<&'j Json>) {
    let mut stack = alloc::vec![root];
    while let Some(value) = stack.pop() {
        if out.len() >= MAX_OBJECTS {
            return;
        }
        match value {
            Json::Arr(items) => stack.extend(items.iter().rev()),
            Json::Obj(_) => {
                out.push(value);
                if let Some(graph) = value.get("@graph") {
                    stack.push(graph);
                }
            }
            _ => {}
        }
    }
}

/// Whether the object's `@type` (a name or a list) is an article or a post.
fn article_like(object: &Json) -> bool {
    let like = |t: &str| {
        let t = t.rsplit(['/', '#']).next().unwrap_or(t);
        t.ends_with("Article") || t.ends_with("Posting") || t == "Report"
    };
    match object.get("@type") {
        Some(Json::Str(t)) => like(t),
        Some(Json::Arr(types)) => types.iter().filter_map(Json::str).any(like),
        _ => false,
    }
}

fn text(value: &Json) -> Option<String> {
    Some(collapse(value.str()?)).filter(|t| !t.is_empty())
}

/// An author: a name, an object with a `name` (or an `@id` of one in the
/// graph), or a list of those, joined with `, `.
fn author(value: &Json, objects: &[&Json]) -> Option<String> {
    match value {
        Json::Str(_) => text(value),
        Json::Obj(_) => value.get("name").and_then(text).or_else(|| {
            let id = value.get("@id")?.str()?;
            let named = objects
                .iter()
                .find(|o| o.get("@id").and_then(Json::str) == Some(id))?;
            text(named.get("name")?)
        }),
        Json::Arr(items) => {
            let names: Vec<String> = items.iter().filter_map(|a| author(a, objects)).collect();
            Some(names.join(", ")).filter(|n| !n.is_empty())
        }
        Json::Other => None,
    }
}

/// An image URL: a string, an object with a `url`, or the first of a list.
fn image(value: &Json) -> Option<String> {
    match value {
        Json::Str(s) => Some(String::from(s.trim())).filter(|s| !s.is_empty()),
        Json::Obj(_) => image(value.get("url").or_else(|| value.get("contentUrl"))?),
        Json::Arr(items) => items.iter().find_map(image),
        Json::Other => None,
    }
}

/// Read one JSON value, or `None` if `text` is not JSON, is nested deeper
/// than [`MAX_DEPTH`] or holds more than [`MAX_VALUES`] values.
fn parse(text: &str) -> Option<Json> {
    enum Frame {
        Arr(Vec<Json>),
        /// The fields so far, and the key whose value is being read.
        Obj(Vec<(String, Json)>, String),
    }
    let b = text.as_bytes();
    let mut i = 0;
    let mut stack: Vec<Frame> = Vec::new();
    let mut values = 0usize;
    'value: loop {
        skip_space(b, &mut i);
        let mut done = match b.get(i)? {
            b'{' => {
                i += 1;
                if stack.len() >= MAX_DEPTH {
                    return None;
                }
                skip_space(b, &mut i);
                if b.get(i) == Some(&b'}') {
                    i += 1;
                    Json::Obj(Vec::new())
                } else {
                    let key = key(text, &mut i)?;
                    stack.push(Frame::Obj(Vec::new(), key));
                    continue 'value;
                }
            }
            b'[' => {
                i += 1;
                if stack.len() >= MAX_DEPTH {
                    return None;
                }
                skip_space(b, &mut i);
                if b.get(i) == Some(&b']') {
                    i += 1;
                    Json::Arr(Vec::new())
                } else {
                    stack.push(Frame::Arr(Vec::new()));
                    continue 'value;
                }
            }
            b'"' => Json::Str(string(text, &mut i)?),
            b't' => literal(b, &mut i, b"true")?,
            b'f' => literal(b, &mut i, b"false")?,
            b'n' => literal(b, &mut i, b"null")?,
            b'-' | b'0'..=b'9' => {
                while b.get(i).is_some_and(|c| {
                    c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E')
                }) {
                    i += 1;
                }
                Json::Other
            }
            _ => return None,
        };
        // Attach the value to its parent; close the containers that end here.
        loop {
            values += 1;
            if values > MAX_VALUES {
                return None;
            }
            match stack.pop() {
                None => {
                    skip_space(b, &mut i);
                    return (i == b.len()).then_some(done);
                }
                Some(Frame::Arr(mut items)) => {
                    items.push(done);
                    skip_space(b, &mut i);
                    match b.get(i)? {
                        b',' => {
                            i += 1;
                            stack.push(Frame::Arr(items));
                            continue 'value;
                        }
                        b']' => {
                            i += 1;
                            done = Json::Arr(items);
                        }
                        _ => return None,
                    }
                }
                Some(Frame::Obj(mut fields, k)) => {
                    fields.push((k, done));
                    skip_space(b, &mut i);
                    match b.get(i)? {
                        b',' => {
                            i += 1;
                            skip_space(b, &mut i);
                            let next = key(text, &mut i)?;
                            stack.push(Frame::Obj(fields, next));
                            continue 'value;
                        }
                        b'}' => {
                            i += 1;
                            done = Json::Obj(fields);
                        }
                        _ => return None,
                    }
                }
            }
        }
    }
}

fn skip_space(b: &[u8], i: &mut usize) {
    while b
        .get(*i)
        .is_some_and(|c| matches!(c, b' ' | b'\t' | b'\n' | b'\r'))
    {
        *i += 1;
    }
}

fn literal(b: &[u8], i: &mut usize, word: &[u8]) -> Option<Json> {
    b.get(*i..)?.starts_with(word).then(|| {
        *i += word.len();
        Json::Other
    })
}

/// An object key and the `:` after it.
fn key(text: &str, i: &mut usize) -> Option<String> {
    let b = text.as_bytes();
    if b.get(*i) != Some(&b'"') {
        return None;
    }
    let key = string(text, i)?;
    skip_space(b, i);
    (b.get(*i) == Some(&b':')).then(|| *i += 1)?;
    Some(key)
}

/// The string starting at the `"` at `*i`, unescaped; `*i` ends after the
/// closing quote. Raw line breaks inside are kept.
fn string(text: &str, i: &mut usize) -> Option<String> {
    let rest = text.get(*i + 1..)?;
    let mut out = String::new();
    let mut chars = rest.char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => {
                *i += 1 + at + 1;
                return Some(out);
            }
            '\\' => match chars.next()?.1 {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                'u' => {
                    let hi = hex4(&mut chars)?;
                    let code = if (0xD800..0xDC00).contains(&hi) {
                        // A surrogate pair; a lone one is U+FFFD.
                        let mut ahead = chars.clone();
                        match (ahead.next(), ahead.next()) {
                            (Some((_, '\\')), Some((_, 'u'))) => match hex4(&mut ahead) {
                                Some(lo) if (0xDC00..0xE000).contains(&lo) => {
                                    chars = ahead;
                                    0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                                }
                                _ => 0xFFFD,
                            },
                            _ => 0xFFFD,
                        }
                    } else {
                        hi
                    };
                    out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                }
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

/// Four hex digits.
fn hex4(chars: &mut core::str::CharIndices<'_>) -> Option<u32> {
    let mut n = 0;
    for _ in 0..4 {
        n = n * 16 + chars.next()?.1.to_digit(16)?;
    }
    Some(n)
}
