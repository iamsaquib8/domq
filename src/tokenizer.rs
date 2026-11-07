//! The streaming HTML tokenizer.
//!
//! Byte-driven and pull-based: it implements [`Iterator`] and yields borrowed
//! [`Token`]s without allocating anything except each tag's attribute list. All
//! cursor access goes through `get()`/iterators, so it never panics on
//! malformed or truncated input — the whole point of the crate.
//!
//! The tree builder uses the crate-internal [`Tokenizer::next_token`], which
//! leaves a start tag's attributes in a buffer that is reused for every tag
//! ([`Tokenizer::attrs`]), so parsing allocates nothing per tag at all. The
//! scans for the next `<`, quote, `-->` or `>` go through [`crate::scan`].

use alloc::vec::Vec;

use crate::error::{ErrorKind, ParseError};
use crate::scan;
use crate::token::{AttrValue, Token};

/// A streaming HTML tokenizer. Construct with [`Tokenizer::new`] (or the
/// top-level [`crate::tokenize`]) and iterate to get [`Token`]s.
pub struct Tokenizer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    /// When set, the next token is the raw-text content up to this element's
    /// matching end tag (used for `<script>`/`<style>`/`<title>`/`<textarea>`).
    raw_text_end: Option<&'a str>,
    /// The attributes of the start tag read last (see [`Tokenizer::attrs`]).
    attrs: Vec<(&'a str, AttrValue<'a>)>,
    pub(crate) errors: Vec<ParseError>,
}

impl<'a> Tokenizer<'a> {
    /// Create a tokenizer over `src`.
    pub fn new(src: &'a str) -> Self {
        Tokenizer {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            raw_text_end: None,
            attrs: Vec::new(),
            errors: Vec::new(),
        }
    }

    // --- cursor helpers (all panic-free) ---------------------------------

    #[inline]
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    #[inline]
    fn peek_at(&self, off: usize) -> Option<u8> {
        self.bytes.get(self.pos + off).copied()
    }

    #[inline]
    fn slice(&self, start: usize, end: usize) -> &'a str {
        self.src.get(start..end).unwrap_or("")
    }

    #[inline]
    fn rest(&self) -> &'a [u8] {
        self.bytes.get(self.pos..).unwrap_or(&[])
    }

    fn starts_with(&self, s: &[u8]) -> bool {
        self.rest().starts_with(s)
    }

    fn starts_with_ci(&self, s: &[u8]) -> bool {
        self.rest()
            .get(..s.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(s))
    }

    /// Advance to the next `needle` byte, or to the end of the input.
    #[inline]
    fn skip_to(&mut self, needle: u8) {
        match scan::find_byte(needle, self.rest()) {
            Some(rel) => self.pos += rel,
            None => self.pos = self.bytes.len(),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn error(&mut self, pos: usize, kind: ErrorKind) {
        self.errors.push(ParseError::new(pos, kind));
    }

    /// The byte offset just past the last token returned (its end).
    #[inline]
    pub(crate) fn offset(&self) -> usize {
        self.pos
    }

    // --- token production ------------------------------------------------

    /// The attributes of the start tag returned last by
    /// [`Tokenizer::next_token`], in source order.
    #[inline]
    pub(crate) fn attrs(&self) -> &[(&'a str, AttrValue<'a>)] {
        &self.attrs
    }

    /// Like `Iterator::next`, but also returns the byte offset the token
    /// started at, and a [`Token::StartTag`]'s `attrs` is always empty: the
    /// attributes are in [`Tokenizer::attrs`] until the next call.
    pub(crate) fn next_token(&mut self) -> Option<(Token<'a>, usize)> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        let start = self.pos;
        if let Some(end_name) = self.raw_text_end.take() {
            return Some((self.read_raw_text(end_name), start));
        }
        let tok = match self.peek() {
            Some(b'<') => self.read_markup(),
            _ => self.read_text(),
        };
        Some((tok, start))
    }

    fn read_text(&mut self) -> Token<'a> {
        let start = self.pos;
        // Scan to the next '<' in one shot, not a byte-at-a-time `peek` loop.
        self.skip_to(b'<');
        Token::Text(self.slice(start, self.pos))
    }

    fn read_markup(&mut self) -> Token<'a> {
        match self.peek_at(1) {
            Some(b'/') => match self.peek_at(2) {
                Some(c) if c.is_ascii_alphabetic() => self.read_end_tag(),
                // `</>` is dropped, as in browsers.
                Some(b'>') => {
                    let s = self.slice(self.pos, self.pos);
                    self.error(self.pos, ErrorKind::StrayEndTag);
                    self.pos += 3;
                    Token::Text(s)
                }
                // `</` at the end of the input is text.
                None => {
                    let s = self.slice(self.pos, self.pos + 2);
                    self.pos += 2;
                    Token::Text(s)
                }
                // `</ x>`, `</#>`: a bogus comment.
                Some(_) => self.read_bogus_comment(),
            },
            Some(b'!') => self.read_bang(),
            Some(b'?') => self.read_bogus_comment(),
            Some(c) if c.is_ascii_alphabetic() => self.read_start_tag(),
            _ => {
                // A lone '<' that doesn't begin markup is literal text.
                let s = self.slice(self.pos, self.pos + 1);
                self.pos += 1;
                Token::Text(s)
            }
        }
    }

    fn read_tag_name(&mut self) -> &'a str {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b':' | b'_' | b'.') {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.slice(start, self.pos)
    }

    fn read_start_tag(&mut self) -> Token<'a> {
        let open = self.pos;
        self.pos += 1; // consume '<'
        let name = self.read_tag_name();
        self.attrs.clear();
        let mut self_closing = false;

        loop {
            self.skip_whitespace();
            match self.peek() {
                None => {
                    self.error(open, ErrorKind::UnterminatedTag);
                    break;
                }
                Some(b'>') => {
                    self.pos += 1;
                    break;
                }
                Some(b'/') => {
                    if self.peek_at(1) == Some(b'>') {
                        self_closing = true;
                        self.pos += 2;
                        break;
                    }
                    self.pos += 1; // stray slash
                }
                Some(_) => match self.read_attribute() {
                    Some(attr) => self.attrs.push(attr),
                    None => self.pos += 1, // couldn't form an attr; make progress
                },
            }
        }

        if !self_closing && is_rawish_name(name) {
            self.raw_text_end = Some(name);
        }

        Token::StartTag {
            name,
            attrs: Vec::new(),
            self_closing,
        }
    }

    fn read_attribute(&mut self) -> Option<(&'a str, AttrValue<'a>)> {
        let nstart = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() || matches!(b, b'=' | b'>' | b'/') {
                break;
            }
            self.pos += 1;
        }
        let name = self.slice(nstart, self.pos);
        if name.is_empty() {
            return None;
        }
        self.skip_whitespace();
        if self.peek() == Some(b'=') {
            self.pos += 1;
            self.skip_whitespace();
            Some((name, self.read_attr_value()))
        } else {
            Some((name, AttrValue::Empty))
        }
    }

    fn read_attr_value(&mut self) -> AttrValue<'a> {
        match self.peek() {
            Some(q @ (b'"' | b'\'')) => {
                self.pos += 1; // opening quote
                let start = self.pos;
                self.skip_to(q);
                let val = self.slice(start, self.pos);
                if self.peek() == Some(q) {
                    self.pos += 1; // closing quote
                }
                AttrValue::Quoted(val)
            }
            _ => {
                let start = self.pos;
                while let Some(b) = self.peek() {
                    if b.is_ascii_whitespace() || b == b'>' {
                        break;
                    }
                    self.pos += 1;
                }
                AttrValue::Unquoted(self.slice(start, self.pos))
            }
        }
    }

    fn read_end_tag(&mut self) -> Token<'a> {
        let open = self.pos;
        self.pos += 2; // consume '</'
        let name = self.read_tag_name();
        self.skip_to(b'>');
        if self.peek() == Some(b'>') {
            self.pos += 1;
        } else {
            self.error(open, ErrorKind::UnterminatedTag);
        }
        Token::EndTag { name }
    }

    fn read_bang(&mut self) -> Token<'a> {
        if self.starts_with(b"<!--") {
            self.read_comment()
        } else if self.starts_with(b"<![CDATA[") {
            self.read_cdata()
        } else if self.starts_with_ci(b"<!doctype") {
            self.read_doctype()
        } else {
            self.read_bogus_comment()
        }
    }

    /// A comment, recovered as in browsers: `<!-->` and `<!--->` are empty
    /// comments, `--!>` also ends one, and one cut off by the end of input
    /// drops the `-`/`--` it ended in.
    fn read_comment(&mut self) -> Token<'a> {
        let open = self.pos;
        self.pos += 4; // consume '<!--'
        let start = self.pos;
        for abrupt in [&b">"[..], b"->"] {
            if self.starts_with(abrupt) {
                self.pos += abrupt.len();
                self.error(open, ErrorKind::BogusComment);
                return Token::Comment(self.slice(start, start));
            }
        }
        // Each `--` found is either the end or skipped by one byte, so the
        // scan stays linear.
        let mut from = start;
        while let Some(rel) = self
            .bytes
            .get(from..)
            .and_then(|rest| scan::find(b"--", rest))
        {
            let dashes = from + rel;
            let close = match self.bytes.get(dashes + 2..) {
                Some([b'>', ..]) => 3,
                Some([b'!', b'>', ..]) => 4,
                _ => {
                    from = dashes + 1;
                    continue;
                }
            };
            if close == 4 {
                self.error(open, ErrorKind::BogusComment);
            }
            let s = self.slice(start, dashes);
            self.pos = dashes + close;
            return Token::Comment(s);
        }
        self.pos = self.bytes.len();
        self.error(open, ErrorKind::UnterminatedComment);
        let s = self.slice(start, self.pos);
        let s = s
            .strip_suffix("--")
            .or_else(|| s.strip_suffix('-'))
            .unwrap_or(s);
        Token::Comment(s)
    }

    fn read_cdata(&mut self) -> Token<'a> {
        let open = self.pos;
        self.pos += 9; // consume '<![CDATA['
        let start = self.pos;
        if let Some(rel) = scan::find(b"]]>", self.rest()) {
            self.pos += rel;
            let s = self.slice(start, self.pos);
            self.pos += 3;
            return Token::Cdata(s);
        }
        self.pos = self.bytes.len();
        self.error(open, ErrorKind::UnterminatedCdata);
        Token::Cdata(self.slice(start, self.pos))
    }

    fn read_doctype(&mut self) -> Token<'a> {
        let open = self.pos;
        self.pos += 9; // consume '<!doctype'
        self.skip_whitespace();
        let start = self.pos;
        self.skip_to(b'>');
        let s = self.slice(start, self.pos);
        if self.peek() == Some(b'>') {
            self.pos += 1;
        } else {
            self.error(open, ErrorKind::UnterminatedTag);
        }
        Token::Doctype(s)
    }

    /// `<! ... >` (not comment/doctype/cdata), `<? ... >` and `</` followed
    /// by something other than a letter are treated as bogus comments,
    /// matching browser recovery: the comment is what follows `<!` or `</`,
    /// or everything after the `<` of `<?` (`<?xml …>` is the comment
    /// `?xml …`).
    fn read_bogus_comment(&mut self) -> Token<'a> {
        let open = self.pos;
        self.pos += 1; // consume '<'
        if matches!(self.peek(), Some(b'!' | b'/')) {
            self.pos += 1;
        }
        let start = self.pos;
        self.skip_to(b'>');
        let s = self.slice(start, self.pos);
        if self.peek() == Some(b'>') {
            self.pos += 1;
        }
        self.error(open, ErrorKind::BogusComment);
        Token::Comment(s)
    }

    /// Scan raw-text content up to (but not consuming) the matching end tag.
    /// `<plaintext>` has none: everything after it is text.
    fn read_raw_text(&mut self, end_name: &str) -> Token<'a> {
        let start = self.pos;
        if end_name.eq_ignore_ascii_case("plaintext") {
            self.pos = self.bytes.len();
        }
        if end_name.eq_ignore_ascii_case("script") {
            self.pos = ScriptData::default().find_end(self.bytes, self.pos, true);
            return Token::Text(self.slice(start, self.pos));
        }
        while self.pos < self.bytes.len() {
            match scan::find_byte(b'<', self.rest()) {
                None => {
                    self.pos = self.bytes.len();
                    break;
                }
                Some(rel) => {
                    let lt = self.pos + rel;
                    if self.is_close_tag_at(lt, end_name) {
                        self.pos = lt;
                        break;
                    }
                    self.pos = lt + 1;
                }
            }
        }
        Token::Text(self.slice(start, self.pos))
    }

    fn is_close_tag_at(&self, lt: usize, name: &str) -> bool {
        if self.bytes.get(lt) != Some(&b'<') || self.bytes.get(lt + 1) != Some(&b'/') {
            return false;
        }
        let after = lt + 2;
        let nm = name.as_bytes();
        match self.bytes.get(after..after + nm.len()) {
            Some(slice) if slice.eq_ignore_ascii_case(nm) => {
                match self.bytes.get(after + nm.len()) {
                    None => true,
                    Some(b) => b.is_ascii_whitespace() || matches!(b, b'>' | b'/'),
                }
            }
            _ => false,
        }
    }
}

/// Where the scan of a `<script>`'s content is, in the HTML tokenizer's
/// script data states: after `<!--` the content is "escaped", and there
/// `<script` starts a "double escaped" part in which `</script>` does not end
/// the element (it only leaves the double escaped part). `-->` goes back to
/// plain script data. Used by the tokenizer, and by the serializer to find
/// the `</script` it must neutralize.
#[derive(Clone, Copy, Default)]
pub(crate) struct ScriptData {
    state: ScriptState,
    /// How many `-` just before the scan position, within an escaped part
    /// (`-->` ends it; `<!-->` is `<!--` then `>`).
    dashes: u32,
    /// Where the `<` of the `<script` that opened the double escaped part
    /// the scan is in, if it is in one.
    double_at: usize,
}

#[derive(Clone, Copy, Default, PartialEq)]
enum ScriptState {
    #[default]
    Data,
    Escaped,
    Double,
}

impl ScriptData {
    /// The index in `bytes` of the `<` of the next `</script` that ends the
    /// element, scanning from `from` (`bytes.len()` if none), and leaving the
    /// scan state to resume after it. `eof_ends`: whether a `</script` cut
    /// off by the very end of `bytes` is an end tag. Each byte is looked at a
    /// bounded number of times, so this is linear.
    pub(crate) fn find_end(&mut self, bytes: &[u8], from: usize, eof_ends: bool) -> usize {
        let mut pos = from;
        while let Some(&b) = bytes.get(pos) {
            if self.state == ScriptState::Data {
                // Plain script data: only `<` matters; scan to it in one shot.
                match scan::find_byte(b'<', bytes.get(pos..).unwrap_or(&[])) {
                    None => return bytes.len(),
                    Some(rel) => pos += rel,
                }
                if Self::end_tag_at(bytes, pos, eof_ends) {
                    return pos;
                }
                if bytes.get(pos..).is_some_and(|r| r.starts_with(b"<!--")) {
                    // `<!--` is also the first two dashes of `-->`.
                    pos += 4;
                    self.state = ScriptState::Escaped;
                    self.dashes = 2;
                } else {
                    pos += 1;
                }
                continue;
            }
            match b {
                b'-' => {
                    self.dashes = self.dashes.saturating_add(1);
                    pos += 1;
                }
                b'>' => {
                    pos += 1;
                    if self.dashes >= 2 {
                        self.state = ScriptState::Data;
                    }
                    self.dashes = 0;
                }
                b'<' => {
                    self.dashes = 0;
                    let closing = bytes.get(pos + 1) == Some(&b'/');
                    if closing
                        && self.state == ScriptState::Escaped
                        && Self::end_tag_at(bytes, pos, eof_ends)
                    {
                        return pos;
                    }
                    // `<script` opens a double escaped part; `</script`
                    // closes one. The delimiter is consumed with it.
                    let name_at = pos + if closing { 2 } else { 1 };
                    if (self.state == ScriptState::Escaped) != closing
                        && let Some(delimiter) = Self::script_name_at(bytes, name_at)
                    {
                        self.state = if closing {
                            ScriptState::Escaped
                        } else {
                            self.double_at = pos;
                            ScriptState::Double
                        };
                        pos = delimiter + 1;
                    } else {
                        pos += 1;
                    }
                }
                _ => {
                    self.dashes = 0;
                    pos += 1;
                }
            }
        }
        bytes.len()
    }

    /// The index of the `<` of the `<script` that opened the double escaped
    /// part the scan is in. `Some` means no `</script` can end the element
    /// from here on, not even one written after the content.
    pub(crate) fn double_escaped_at(&self) -> Option<usize> {
        (self.state == ScriptState::Double).then_some(self.double_at)
    }

    /// If `script` starts at `at` and ends there (at whitespace, `/` or `>`),
    /// the index of that delimiter.
    fn script_name_at(bytes: &[u8], at: usize) -> Option<usize> {
        let name = bytes.get(at..at + 6)?;
        let delimiter = bytes.get(at + 6)?;
        (name.eq_ignore_ascii_case(b"script")
            && (delimiter.is_ascii_whitespace() || matches!(delimiter, b'/' | b'>')))
        .then_some(at + 6)
    }

    /// Whether the `<` at `lt` starts `</script` and ends it: with a
    /// delimiter, or `</script` itself ending `bytes` and `eof_ends`.
    fn end_tag_at(bytes: &[u8], lt: usize, eof_ends: bool) -> bool {
        if bytes.get(lt + 1) != Some(&b'/') {
            return false;
        }
        match bytes.get(lt + 2..lt + 8) {
            Some(name) if name.eq_ignore_ascii_case(b"script") => match bytes.get(lt + 8) {
                None => eof_ends,
                Some(b) => b.is_ascii_whitespace() || matches!(b, b'/' | b'>'),
            },
            _ => false,
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        let (mut tok, _) = self.next_token()?;
        if let Token::StartTag { attrs, .. } = &mut tok {
            // Hand the buffer over: the next tag starts a fresh one.
            *attrs = core::mem::take(&mut self.attrs);
        }
        Some(tok)
    }
}

/// Whether an element's content is treated as raw text by the tokenizer
/// (no nested tags). Entity-decoding of that text is decided later, per element.
fn is_rawish_name(name: &str) -> bool {
    [
        "script",
        "style",
        "title",
        "textarea",
        "xmp",
        "iframe",
        "noembed",
        "noframes",
        "plaintext",
    ]
    .iter()
    .any(|t| name.eq_ignore_ascii_case(t))
}
