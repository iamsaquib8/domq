//! Input decoding: bytes of an HTML page → a `String`.
//!
//! Follows the WHATWG encoding-sniffing algorithm:
//!
//! 1. A byte-order mark (UTF-8, UTF-16LE, UTF-16BE) — always wins;
//! 2. An explicit encoding label override if provided;
//! 3. `<meta charset=…>` or `<meta http-equiv=content-type content="…; charset=…">`
//!    in the first 1024 bytes (the WHATWG prescan);
//! 4. UTF-8, if the bytes are valid UTF-8 (including an incomplete sequence cut off
//!    at the very end of the stream), or contain valid multibyte UTF-8 sequences
//!    with rare invalid sequences (decoded lossy with a warning);
//! 5. Otherwise Windows-1252, which maps every byte to a character.
//!
//! Supported encodings: UTF-8, UTF-16LE, UTF-16BE, and Windows-1252, under their
//! WHATWG labels — so `iso-8859-1`, `latin1`, and `ascii` map to Windows-1252,
//! as in browsers. Decoding never fails: unmapped or invalid sequences become
//! replacement characters (U+FFFD).

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ops::Deref;

/// An encoding detected or supported by [`decode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// UTF-8 (the default for the web).
    Utf8,
    /// UTF-16 little-endian.
    Utf16Le,
    /// UTF-16 big-endian.
    Utf16Be,
    /// Windows-1252 (the WHATWG fallback for legacy single-byte encodings).
    Windows1252,
}

impl Encoding {
    /// The canonical WHATWG name of this encoding (`"utf-8"`, `"utf-16le"`, `"utf-16be"`, or `"windows-1252"`).
    pub fn name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "utf-8",
            Encoding::Utf16Le => "utf-16le",
            Encoding::Utf16Be => "utf-16be",
            Encoding::Windows1252 => "windows-1252",
        }
    }
}

/// The result of [`decode`] or [`decode_with`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoded {
    /// The decoded HTML text.
    pub text: String,
    /// The character encoding used to decode the bytes.
    pub encoding: Encoding,
    /// A diagnostic warning if the declared encoding was unknown or unsupported.
    pub warning: Option<String>,
}

impl Decoded {
    /// The decoded text as a string slice.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Consume the decoded result and return the owned string.
    pub fn into_text(self) -> String {
        self.text
    }
}

impl Deref for Decoded {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.text
    }
}

impl AsRef<str> for Decoded {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

/// How far into the input the `<meta>` prescan looks.
const PRESCAN_BYTES: usize = 1024;

/// Decode `bytes` into a string using the WHATWG encoding-sniffing algorithm.
///
/// Sniffs BOM, `<meta charset>`, valid UTF-8, and falls back to Windows-1252.
/// Decoding never fails: unmapped or invalid sequences become U+FFFD.
pub fn decode(bytes: &[u8]) -> Decoded {
    decode_with(bytes, None)
}

/// Decode `bytes`, with `label` overriding auto-detection when valid.
pub fn decode_with(bytes: &[u8], label: Option<&str>) -> Decoded {
    let done = |text, encoding, warning| Decoded {
        text,
        encoding,
        warning,
    };
    // 1. A byte-order mark.
    if let Some((encoding, len)) = bom(bytes) {
        let body = bytes.get(len..).unwrap_or_default();
        return done(decode_as(body, encoding), encoding, None);
    }
    // 2. Explicit label override.
    if let Some(label) = label {
        return match lookup_encoding(label) {
            Some(encoding) => done(decode_as(bytes, encoding), encoding, None),
            None => done(
                decode_as(bytes, Encoding::Utf8),
                Encoding::Utf8,
                Some(format!(
                    "unknown or unsupported encoding '{label}' (supported: utf-8, \
                     utf-16le, utf-16be, windows-1252); decoding as UTF-8"
                )),
            ),
        };
    }
    // 3. `<meta charset>`.
    let head = bytes.get(..PRESCAN_BYTES).unwrap_or(bytes);
    let declared = prescan(head);
    if let Prescan::Found(encoding) = declared {
        return done(decode_as(bytes, encoding), encoding, None);
    }
    // 4. UTF-8 (strictly valid, or with rare invalid sequences).
    if is_utf8(bytes) {
        return done(decode_as(bytes, Encoding::Utf8), Encoding::Utf8, None);
    }
    let scan = scan_utf8(bytes);
    let non_ascii = scan.valid_multibyte + scan.invalid_sequences;
    if scan.valid_multibyte > 0
        && (scan.invalid_sequences <= 16 || scan.invalid_sequences * 100 <= non_ascii)
    {
        let plural = if scan.invalid_sequences == 1 { "" } else { "s" };
        let warning = format!(
            "{} invalid UTF-8 sequence{} replaced with U+FFFD",
            scan.invalid_sequences, plural
        );
        return done(
            decode_as(bytes, Encoding::Utf8),
            Encoding::Utf8,
            Some(warning),
        );
    }
    // 5. Fallback to Windows-1252.
    let warning = match declared {
        Prescan::Unsupported(label) => Some(format!(
            "the page declares charset '{label}', which is not supported; \
             decoding as windows-1252"
        )),
        _ => Some(
            "no charset declared and input is not valid UTF-8; \
             decoding as windows-1252 (use --encoding to override)"
                .to_string(),
        ),
    };
    done(
        decode_as(bytes, Encoding::Windows1252),
        Encoding::Windows1252,
        warning,
    )
}

/// The encoding a WHATWG label names, if it is one this module supports.
/// Labels are matched ASCII case-insensitively, ignoring surrounding whitespace.
pub fn lookup_encoding(label: &str) -> Option<Encoding> {
    let label = label
        .trim_matches(|c: char| c.is_ascii_whitespace())
        .to_ascii_lowercase();
    Some(match label.as_str() {
        "unicode-1-1-utf-8" | "unicode11utf8" | "unicode20utf8" | "utf-8" | "utf8"
        | "x-unicode20utf8" => Encoding::Utf8,
        "csunicode" | "iso-10646-ucs-2" | "ucs-2" | "unicode" | "unicodefeff" | "utf-16"
        | "utf-16le" => Encoding::Utf16Le,
        "unicodefffe" | "utf-16be" => Encoding::Utf16Be,
        "ansi_x3.4-1968" | "ascii" | "cp1252" | "cp819" | "csisolatin1" | "ibm819"
        | "iso-8859-1" | "iso-ir-100" | "iso8859-1" | "iso88591" | "iso_8859-1"
        | "iso_8859-1:1987" | "l1" | "latin1" | "us-ascii" | "windows-1252" | "x-cp1252" => {
            Encoding::Windows1252
        }
        _ => return None,
    })
}

fn bom(bytes: &[u8]) -> Option<(Encoding, usize)> {
    match bytes {
        [0xEF, 0xBB, 0xBF, ..] => Some((Encoding::Utf8, 3)),
        [0xFE, 0xFF, ..] => Some((Encoding::Utf16Be, 2)),
        [0xFF, 0xFE, ..] => Some((Encoding::Utf16Le, 2)),
        _ => None,
    }
}

/// Valid UTF-8, allowing an incomplete sequence at the very end.
fn is_utf8(bytes: &[u8]) -> bool {
    match core::str::from_utf8(bytes) {
        Ok(_) => true,
        Err(e) => e.error_len().is_none(),
    }
}

struct Utf8Scan {
    valid_multibyte: usize,
    invalid_sequences: usize,
}

fn scan_utf8(bytes: &[u8]) -> Utf8Scan {
    let mut valid_multibyte = 0;
    let mut invalid_sequences = 0;
    let mut remaining = bytes;

    while !remaining.is_empty() {
        match core::str::from_utf8(remaining) {
            Ok(valid) => {
                for &b in valid.as_bytes() {
                    if b >= 0xC0 {
                        valid_multibyte += 1;
                    }
                }
                break;
            }
            Err(e) => {
                let valid_len = e.valid_up_to();
                for &b in &remaining[..valid_len] {
                    if b >= 0xC0 {
                        valid_multibyte += 1;
                    }
                }
                invalid_sequences += 1;
                match e.error_len() {
                    Some(len) => {
                        remaining = &remaining[valid_len + len..];
                    }
                    None => {
                        break;
                    }
                }
            }
        }
    }

    Utf8Scan {
        valid_multibyte,
        invalid_sequences,
    }
}

fn decode_as(bytes: &[u8], encoding: Encoding) -> String {
    match encoding {
        Encoding::Utf8 => match core::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(bytes).into_owned(),
        },
        Encoding::Utf16Le => decode_utf16(bytes, u16::from_le_bytes),
        Encoding::Utf16Be => decode_utf16(bytes, u16::from_be_bytes),
        Encoding::Windows1252 => bytes.iter().map(|&b| windows_1252(b)).collect(),
    }
}

/// UTF-16 with unpaired surrogates and a dangling odd byte as U+FFFD.
fn decode_utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let (pairs, odd) = bytes.as_chunks::<2>();
    let units = pairs.iter().map(|&pair| unit(pair));
    let mut out: String = char::decode_utf16(units)
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect();
    if !odd.is_empty() {
        out.push(char::REPLACEMENT_CHARACTER);
    }
    out
}

/// Windows-1252 per the WHATWG index: Latin-1, except 0x80–0x9F, where the
/// five unassigned bytes map to the C1 controls of the same value.
fn windows_1252(b: u8) -> char {
    const C1: [char; 32] = [
        '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}',
        '\u{017D}', '\u{008F}', '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}',
        '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
    ];
    match b {
        0x80..=0x9F => C1.get(usize::from(b - 0x80)).copied().unwrap_or('\u{FFFD}'),
        _ => char::from(b),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Prescan {
    Found(Encoding),
    /// A `<meta>` named an encoding this module does not support.
    Unsupported(String),
    None,
}

/// The WHATWG "prescan a byte stream to determine its encoding" algorithm,
/// simplified: comments, `<meta>` and other tags (whose attributes are
/// skipped properly, so a `>` inside a quoted value does not end them).
fn prescan(head: &[u8]) -> Prescan {
    let mut unsupported = None;
    let mut i = 0;
    while i < head.len() {
        let rest = head.get(i..).unwrap_or_default();
        if rest.starts_with(b"<!--") {
            // `<!-->` and `<!--->` are complete comments, as in the tokenizer.
            match find(head, i + 2, b"-->") {
                Some(end) => i = end + 3,
                None => break,
            }
            continue;
        }
        if starts_with_ignore_case(rest, b"<meta")
            && rest.get(5).is_some_and(|&b| is_space(b) || b == b'/')
        {
            i += 6;
            match meta_charset(head, &mut i) {
                Some(Ok(encoding)) => return Prescan::Found(encoding),
                Some(Err(label)) => {
                    unsupported.get_or_insert(label);
                }
                None => {}
            }
            continue;
        }
        let tag_start = match rest {
            [b'<', b'/', c, ..] if c.is_ascii_alphabetic() => Some(2),
            [b'<', c, ..] if c.is_ascii_alphabetic() => Some(1),
            _ => None,
        };
        if let Some(skip) = tag_start {
            i += skip;
            while head.get(i).is_some_and(|&b| !is_space(b) && b != b'>') {
                i += 1;
            }
            while attribute(head, &mut i).is_some() {}
            continue;
        }
        if rest.starts_with(b"<!") || rest.starts_with(b"</") || rest.starts_with(b"<?") {
            match find(head, i, b">") {
                Some(end) => i = end + 1,
                None => break,
            }
            continue;
        }
        i += 1;
    }
    unsupported.map_or(Prescan::None, Prescan::Unsupported)
}

/// The attributes of one `<meta>` tag (from just after `<meta`): the
/// encoding it declares, `Err(label)` for an unsupported one, or `None`.
fn meta_charset(head: &[u8], i: &mut usize) -> Option<Result<Encoding, String>> {
    let mut seen: Vec<Vec<u8>> = Vec::new();
    let (mut got_pragma, mut need_pragma) = (false, None);
    let mut charset: Option<String> = None;
    while let Some((name, value)) = attribute(head, i) {
        if seen.contains(&name) {
            continue;
        }
        match name.as_slice() {
            b"http-equiv" => got_pragma |= value.eq_ignore_ascii_case(b"content-type"),
            b"content" if charset.is_none() => {
                if let Some(label) = charset_from_content(&value) {
                    charset = Some(String::from_utf8_lossy(label).into_owned());
                    need_pragma = Some(true);
                }
            }
            b"charset" => {
                charset = Some(String::from_utf8_lossy(&value).into_owned());
                need_pragma = Some(false);
            }
            _ => {}
        }
        seen.push(name);
    }
    match need_pragma {
        None => return None,
        Some(true) if !got_pragma => return None,
        _ => {}
    }
    let label = charset?;
    Some(match lookup_encoding(&label) {
        // A document whose `<meta>` can be read as ASCII is not UTF-16.
        Some(Encoding::Utf16Le | Encoding::Utf16Be) => Ok(Encoding::Utf8),
        Some(encoding) => Ok(encoding),
        None if label.trim().eq_ignore_ascii_case("x-user-defined") => Ok(Encoding::Windows1252),
        None => Err(label),
    })
}

/// The WHATWG "get an attribute" step: the next `(name, value)` of a tag,
/// names and unquoted values lower-cased; `None` at the end of the tag (or
/// of the input). Always advances `i` past what it read.
fn attribute(head: &[u8], i: &mut usize) -> Option<(Vec<u8>, Vec<u8>)> {
    let at = |i: usize| head.get(i).copied();
    while at(*i).is_some_and(|b| is_space(b) || b == b'/') {
        *i += 1;
    }
    let first = at(*i)?;
    if first == b'>' {
        return None;
    }
    let (mut name, mut value) = (Vec::new(), Vec::new());
    loop {
        let b = at(*i)?;
        match b {
            b'=' if !name.is_empty() => {
                *i += 1;
                break;
            }
            b if is_space(b) => {
                while at(*i).is_some_and(is_space) {
                    *i += 1;
                }
                if at(*i) != Some(b'=') {
                    return Some((name, value));
                }
                *i += 1;
                break;
            }
            b'/' | b'>' => return Some((name, value)),
            b => name.push(b.to_ascii_lowercase()),
        }
        *i += 1;
    }
    while at(*i).is_some_and(is_space) {
        *i += 1;
    }
    match at(*i) {
        Some(quote @ (b'"' | b'\'')) => {
            *i += 1;
            loop {
                let b = at(*i)?;
                *i += 1;
                if b == quote {
                    return Some((name, value));
                }
                value.push(b.to_ascii_lowercase());
            }
        }
        Some(b'>') | None => return Some((name, value)),
        Some(_) => {}
    }
    while let Some(b) = at(*i) {
        if is_space(b) || b == b'>' {
            break;
        }
        value.push(b.to_ascii_lowercase());
        *i += 1;
    }
    Some((name, value))
}

/// The WHATWG "extract a character encoding from a meta element" algorithm:
/// the label after `charset=` in a `content` value.
fn charset_from_content(content: &[u8]) -> Option<&[u8]> {
    let mut from = 0;
    loop {
        let at = find_ignore_case(content, from, b"charset")?;
        let mut i = at + b"charset".len();
        while content.get(i).copied().is_some_and(is_space) {
            i += 1;
        }
        if content.get(i) != Some(&b'=') {
            from = i;
            continue;
        }
        i += 1;
        while content.get(i).copied().is_some_and(is_space) {
            i += 1;
        }
        return match content.get(i).copied()? {
            quote @ (b'"' | b'\'') => {
                let end = find(content, i + 1, &[quote])?;
                content.get(i + 1..end)
            }
            _ => {
                let rest = content.get(i..)?;
                let end = rest
                    .iter()
                    .position(|&b| is_space(b) || b == b';')
                    .unwrap_or(rest.len());
                rest.get(..end)
            }
        };
    }
}

fn is_space(b: u8) -> bool {
    matches!(b, b'\t' | b'\n' | b'\x0C' | b'\r' | b' ')
}

fn starts_with_ignore_case(haystack: &[u8], prefix: &[u8]) -> bool {
    haystack
        .get(..prefix.len())
        .is_some_and(|h| h.eq_ignore_ascii_case(prefix))
}

/// The first index `>= from` where `needle` occurs.
fn find(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    let rest = haystack.get(from..)?;
    rest.windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn find_ignore_case(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    let rest = haystack.get(from..)?;
    rest.windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle))
        .map(|p| p + from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn detect(bytes: &[u8]) -> (String, Encoding) {
        let d = decode(bytes);
        (d.text, d.encoding)
    }

    fn utf16(s: &str, big_endian: bool) -> Vec<u8> {
        s.encode_utf16()
            .flat_map(|u| {
                if big_endian {
                    u.to_be_bytes()
                } else {
                    u.to_le_bytes()
                }
            })
            .collect()
    }

    #[test]
    fn utf8_bom_is_stripped() {
        assert_eq!(
            detect(b"\xEF\xBB\xBF<p>caf\xC3\xA9"),
            ("<p>café".into(), Encoding::Utf8)
        );
    }

    #[test]
    fn utf16_boms() {
        let mut le = vec![0xFF, 0xFE];
        le.extend(utf16("<p>naïve 😀</p>", false));
        assert_eq!(detect(&le), ("<p>naïve 😀</p>".into(), Encoding::Utf16Le));
        let mut be = vec![0xFE, 0xFF];
        be.extend(utf16("<p>naïve 😀</p>", true));
        assert_eq!(detect(&be), ("<p>naïve 😀</p>".into(), Encoding::Utf16Be));
    }

    #[test]
    fn invalid_utf16_becomes_replacement_characters() {
        // A lone high surrogate, then a lone low surrogate.
        let mut bytes = vec![0xFF, 0xFE, b'a', 0];
        bytes.extend([0x3D, 0xD8, b'b', 0, 0x00, 0xDC]);
        assert_eq!(detect(&bytes).0, "a\u{FFFD}b\u{FFFD}");
        // An odd trailing byte (truncated input).
        assert_eq!(detect(&[0xFE, 0xFF, 0, b'a', 0]).0, "a\u{FFFD}");
        // A surrogate pair cut in half.
        assert_eq!(detect(&[0xFF, 0xFE, 0x3D, 0xD8]).0, "\u{FFFD}");
        // Just a BOM.
        assert_eq!(detect(&[0xFF, 0xFE]).0, "");
    }

    #[test]
    fn meta_charset() {
        let html = b"<!doctype html><html><head><meta charset=\"windows-1252\">caf\xC3\xA9";
        assert_eq!(
            detect(html),
            (
                "<!doctype html><html><head><meta charset=\"windows-1252\">caf\u{C3}\u{A9}".into(),
                Encoding::Windows1252
            )
        );
        assert_eq!(
            detect(b"<META CharSet='Latin1'>\x80").1,
            Encoding::Windows1252
        );
        assert_eq!(
            detect(b"<meta/charset=utf-8>\xE9").0,
            "<meta/charset=utf-8>\u{FFFD}"
        );
    }

    #[test]
    fn meta_http_equiv() {
        let html =
            b"<meta http-equiv=\"Content-Type\" content=\"text/html; charset=ISO-8859-1\">\xE9";
        assert_eq!(detect(html).1, Encoding::Windows1252);
        // `content` without the pragma does not count.
        let html = b"<meta content=\"text/html; charset=ISO-8859-1\">caf\xC3\xA9";
        assert_eq!(
            detect(html),
            (
                "<meta content=\"text/html; charset=ISO-8859-1\">café".into(),
                Encoding::Utf8
            )
        );
        assert_eq!(
            charset_from_content(b"text/html;charset = \"utf-8\" x"),
            Some(&b"utf-8"[..])
        );
        assert_eq!(
            charset_from_content(b"charsetx; charset=koi8-r;"),
            Some(&b"koi8-r"[..])
        );
        assert_eq!(charset_from_content(b"charset='unterminated"), None);
        assert_eq!(charset_from_content(b"charset="), None);
    }

    #[test]
    fn prescan_skips_comments_and_attribute_values() {
        let html = b"<!-- <meta charset=latin1> --><p>caf\xC3\xA9";
        assert_eq!(detect(html).1, Encoding::Utf8);
        let html = b"<div title='<meta charset=latin1>'>caf\xC3\xA9";
        assert_eq!(detect(html).1, Encoding::Utf8);
        let html = b"<!--><meta charset=latin1>\xC3\xA9";
        assert_eq!(detect(html).1, Encoding::Windows1252);
        // Unterminated constructs end the prescan without looping forever.
        assert_eq!(prescan(b"<!-- <meta charset=latin1>"), Prescan::None);
        assert_eq!(prescan(b"<meta charset=\"latin1"), Prescan::None);
        assert_eq!(prescan(b"<div a='>"), Prescan::None);
        assert_eq!(prescan(b"<?xml "), Prescan::None);
        assert_eq!(prescan(b"<meta"), Prescan::None);
    }

    #[test]
    fn prescan_only_reads_the_first_1024_bytes() {
        let mut html = vec![b' '; 1024];
        html.extend(b"<meta charset=latin1>caf\xC3\xA9");
        assert_eq!(detect(&html).1, Encoding::Utf8);
    }

    #[test]
    fn meta_utf16_means_utf8() {
        assert_eq!(
            detect(b"<meta charset=utf-16>\xC3\xA9").0,
            "<meta charset=utf-16>é"
        );
        assert_eq!(
            detect(b"<meta charset=x-user-defined>\xE9").1,
            Encoding::Windows1252
        );
    }

    #[test]
    fn valid_utf8_and_windows_1252_fallback() {
        assert_eq!(
            detect(b"<p>caf\xC3\xA9"),
            ("<p>café".into(), Encoding::Utf8)
        );
        assert_eq!(detect(b""), (String::new(), Encoding::Utf8));
        assert_eq!(
            detect(b"<p>caf\xE9 \x80\x81\x9F\xFF"),
            ("<p>café €\u{81}Ÿÿ".into(), Encoding::Windows1252)
        );
    }

    #[test]
    fn truncated_utf8_is_still_utf8() {
        // "é" cut after its first byte, as `head -c` would.
        assert_eq!(
            detect(b"<p>caf\xC3\xA9 caf\xC3"),
            ("<p>café caf\u{FFFD}".into(), Encoding::Utf8)
        );
        // An invalid byte that is *not* at the end is not truncation.
        assert_eq!(detect(b"caf\xC3 x").1, Encoding::Windows1252);
    }

    #[test]
    fn label_override() {
        let d = decode_with(&utf16("<p>hi", false), Some("UTF-16LE"));
        assert_eq!((d.text.as_str(), d.encoding), ("<p>hi", Encoding::Utf16Le));
        let d = decode_with(&utf16("<p>hi", true), Some("utf-16be"));
        assert_eq!(d.text, "<p>hi");
        // Overrides `<meta>` and UTF-8 validity.
        let d = decode_with(b"<meta charset=latin1>\xC3\xA9", Some("utf8"));
        assert_eq!(d.text, "<meta charset=latin1>é");
        for alias in [" Latin1 ", "iso-8859-1", "ascii", "us-ascii", "cp1252"] {
            let d = decode_with(b"caf\xC3\xA9", Some(alias));
            assert_eq!(d.text, "caf\u{C3}\u{A9}", "{alias}");
        }
        // A BOM still wins.
        let d = decode_with(b"\xEF\xBB\xBFcaf\xC3\xA9", Some("windows-1252"));
        assert_eq!((d.text.as_str(), d.encoding), ("café", Encoding::Utf8));
    }

    #[test]
    fn unknown_labels_warn_and_decode_lossy_utf8() {
        let d = decode_with(b"caf\xE9", Some("shift_jis"));
        assert_eq!(
            (d.text.as_str(), d.encoding),
            ("caf\u{FFFD}", Encoding::Utf8)
        );
        assert!(d.warning.is_some_and(|w| w.contains("shift_jis")));
    }

    #[test]
    fn unsupported_meta_charset_falls_through() {
        // Valid UTF-8: decoded as such, quietly.
        assert_eq!(
            detect(b"<meta charset=koi8-r>caf\xC3\xA9").1,
            Encoding::Utf8
        );
        // Otherwise Windows-1252, with a warning; a later supported `<meta>` wins.
        let d = decode_with(b"<meta charset=koi8-r>\xE9!", None);
        assert_eq!(d.encoding, Encoding::Windows1252);
        assert!(d.warning.is_some_and(|w| w.contains("koi8-r")));
        let d = decode_with(b"<meta charset=koi8-r><meta charset=utf-8>\xE9", None);
        assert_eq!((d.encoding, d.warning), (Encoding::Utf8, None));
    }

    #[test]
    fn never_panics_on_arbitrary_prefixes() {
        let html =
            b"<!--x--><a b='c'd=e f><meta http-equiv=content-type content='a;charset=\"x'>\xFF\xFE";
        for end in 0..=html.len() {
            let _ = decode(&html[..end]);
            let _ = prescan(&html[..end]);
        }
    }

    #[test]
    fn stray_invalid_byte_recovers_as_utf8_with_warning() {
        // Issue #143 repro: 50 UTF-8 paragraphs with 1 stray \xa0 byte.
        let mut input = Vec::new();
        for _ in 0..50 {
            input.extend_from_slice("<p>café naïve — “quotes” 日本語</p>".as_bytes());
        }
        input.extend_from_slice(b"<p>stray \xa0 byte</p>");
        let d = decode(&input);
        assert_eq!(d.encoding, Encoding::Utf8);
        assert!(d.text.contains("café naïve — “quotes” 日本語"));
        assert!(d.text.contains("stray \u{FFFD} byte"));
        assert_eq!(
            d.warning.as_deref(),
            Some("1 invalid UTF-8 sequence replaced with U+FFFD")
        );
    }

    #[test]
    fn pure_latin1_falls_back_to_windows_1252_with_warning() {
        let d = decode(b"<p>caf\xE9</p>");
        assert_eq!(d.encoding, Encoding::Windows1252);
        assert_eq!(d.text, "<p>café</p>");
        assert_eq!(
            d.warning.as_deref(),
            Some(
                "no charset declared and input is not valid UTF-8; decoding as windows-1252 (use --encoding to override)"
            )
        );
    }

    #[test]
    fn multiple_stray_invalid_bytes_plural_warning() {
        let d = decode(b"<p>caf\xC3\xA9</p> \xA0 \xA1");
        assert_eq!(d.encoding, Encoding::Utf8);
        assert_eq!(
            d.warning.as_deref(),
            Some("2 invalid UTF-8 sequences replaced with U+FFFD")
        );
    }

    #[test]
    fn parse_bytes_api() {
        let dom = crate::parse_bytes(b"<article><h1>News</h1><p>Caf\xC3\xA9</p></article>");
        assert_eq!(dom.to_text(), "News\n\nCafé");

        let dom = crate::parse_bytes_with(b"<h1>News</h1><p>Caf\xe9</p>", Some("latin1"));
        assert_eq!(dom.to_text(), "News\n\nCafé");
    }
}
