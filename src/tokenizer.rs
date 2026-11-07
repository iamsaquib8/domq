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
        // `</>` is dropped, so it has no token: read on.
        let mut start = start;
        loop {
            let tok = match self.peek() {
                Some(b'<') => self.read_markup(),
                _ => Some(self.read_text()),
            };
            if let Some(tok) = tok {
                return Some((tok, start));
            }
            if self.pos >= self.bytes.len() {
                return None;
            }
            start = self.pos;
        }
    }

    /// A text run: up to the next `<` that starts markup. A `<` that cannot
    /// (`a < b`, `<3`, `<-`, and `<` or `</` at the very end) is literal text
    /// and stays in the run, so a browser's one text node is one token.
    fn read_text(&mut self) -> Token<'a> {
        let start = self.pos;
        loop {
            // Scan to the next '<' in one shot, not a byte-at-a-time `peek`
            // loop.
            self.skip_to(b'<');
            let literal = match self.peek_at(1) {
                None => true,
                Some(b'/') => self.peek_at(2).is_none(),
                Some(c) => !(c.is_ascii_alphabetic() || matches!(c, b'!' | b'?')),
            };
            if !literal || self.pos >= self.bytes.len() {
                break;
            }
            self.pos += 1; // the '<'
        }
        Token::Text(self.slice(start, self.pos))
    }

    /// The markup at a `<`; `None` for a `</>`, which is dropped.
    fn read_markup(&mut self) -> Option<Token<'a>> {
        Some(match self.peek_at(1) {
            Some(b'/') => match self.peek_at(2) {
                Some(c) if c.is_ascii_alphabetic() => self.read_end_tag(),
                // `</>` is dropped, as in browsers.
                Some(b'>') => {
                    self.error(self.pos, ErrorKind::StrayEndTag);
                    self.pos += 3;
                    return None;
                }
                // `</` at the end of the input is text.
                None => return Some(self.read_text()),
                // `</ x>`, `</#>`: a bogus comment.
                Some(_) => self.read_bogus_comment(),
            },
            Some(b'!') => self.read_bang(),
            Some(b'?') => self.read_bogus_comment(),
            Some(c) if c.is_ascii_alphabetic() => self.read_start_tag(),
            // A lone '<' that doesn't begin markup is literal text; only a
            // run that starts with one gets here (`read_text` keeps the rest
            // of them in the run).
            _ => return Some(self.read_text()),
        })
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
