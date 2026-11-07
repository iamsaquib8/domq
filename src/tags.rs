//! Static, allocation-free tag classification tables.
//!
//! Everything here is a `const`/`match` over ASCII-lowercased tag names. These
//! tables drive the tokenizer's raw-text handling and the tree builder's
//! void-element and implicit-close behaviour. Names are compared
//! case-insensitively by the callers, which lower-case before lookup.

/// Elements that never have children and never take an end tag.
/// (The HTML "void elements".)
pub(crate) fn is_void(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Elements whose content is raw text: `<` does not start a tag inside them,
/// and entities are *not* decoded. Scanning runs until the matching end tag
/// (for `<plaintext>`, to the end of the input).
pub(crate) fn is_raw_text(name: &str) -> bool {
    matches!(
        name,
        "script" | "style" | "xmp" | "iframe" | "noembed" | "noframes" | "plaintext"
    )
}

/// Elements the tokenizer reads as text up to their end tag: the raw-text
/// ones plus `title` and `textarea` (whose text is still entity-decoded).
/// Whatever is serialized inside one must not contain that end tag.
pub(crate) fn ends_only_at_end_tag(name: &str) -> bool {
    is_raw_text(name) || matches!(name, "title" | "textarea")
}

/// Whether `name` can be written out as an attribute name as it is: not
/// empty, and none of ASCII whitespace, `"`, `'`, `<`, `>`, `/`, `=` or NUL,
/// which would end the name early or start new markup.
pub(crate) fn is_valid_attr_name(name: &str) -> bool {
    !name.is_empty()
        && !name.bytes().any(|b| {
            b.is_ascii_whitespace() || matches!(b, b'"' | b'\'' | b'<' | b'>' | b'/' | b'=' | 0)
        })
}

/// Whether `name` can be written out as a tag name as it is: a valid
/// attribute name that starts with an ASCII letter. (After `<` anything else
/// is text, and the element's attribute values would be read as markup.)
pub(crate) fn is_valid_tag_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic) && is_valid_attr_name(name)
}

/// Start tags that implicitly close an open `<p>`: every element the WHATWG
/// "optional tags" rules let a `</p>` be omitted before, plus the other start
/// tags tree construction closes a `<p>` for (`center`, `dir`, `li`, `dd`,
/// `dt`, `summary`, `listing`, `plaintext`, `xmp`).
fn closes_p(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "center"
            | "dd"
            | "details"
            | "dialog"
            | "dir"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hgroup"
            | "hr"
            | "li"
            | "listing"
            | "main"
            | "menu"
            | "nav"
            | "ol"
            | "p"
            | "plaintext"
            | "pre"
            | "search"
            | "section"
            | "summary"
            | "table"
            | "ul"
            | "xmp"
    )
}

fn is_heading(name: &str) -> bool {
    matches!(name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

/// Metadata start tags that stay inside an open `<head>`; any other start tag
/// (`<body>`, `<div>`, …) means the `</head>` was omitted.
fn in_head(name: &str) -> bool {
    matches!(
        name,
        "base"
            | "basefont"
            | "bgsound"
            | "link"
            | "meta"
            | "noframes"
            | "noscript"
            | "script"
            | "style"
            | "template"
            | "title"
    )
}

/// Table-structure start tags that end an open cell, row, row group or
/// caption (the WHATWG "in cell" / "in row" / "in table body" / "in caption"
/// rules, minus the scope checks).
fn table_structure(name: &str) -> bool {
    matches!(
        name,
        "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead"
    )
}

/// Given a start tag `opening` and the tag name `open` currently on top of the
/// open-element stack, decide whether `open` should be implicitly closed first.
///
/// These are the WHATWG "optional tags" rules (13.1.2.4) — an end tag may be
/// omitted when the element is immediately followed by one of these start
/// tags — together with the tree-construction cases that act on the current
/// node: `<li>`/`<dt>`/`<dd>`/headings closing `<p>`, a heading closing an
/// open heading, `<colgroup>` ending at the first non-`<col>` content, and
/// table sections, rows, cells and captions closing each other. They only ever
/// look at the *top* of the stack, so the builder needs no insertion modes,
/// scopes or active-formatting list.
pub(crate) fn implicitly_closes(opening: &str, open: &str) -> bool {
    match open {
        "p" => closes_p(opening),
        "li" => opening == "li",
        "dt" | "dd" => matches!(opening, "dt" | "dd"),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => is_heading(opening),
        "head" => !in_head(opening),
        "option" => matches!(opening, "option" | "optgroup" | "hr"),
        "optgroup" => matches!(opening, "optgroup" | "hr"),
        "rb" | "rt" | "rp" => matches!(opening, "rb" | "rt" | "rtc" | "rp"),
        "rtc" => matches!(opening, "rb" | "rtc"),
        "td" | "th" => matches!(opening, "td" | "th" | "tr") || table_structure(opening),
        "tr" => opening == "tr" || table_structure(opening),
        "thead" | "tbody" | "tfoot" => table_structure(opening),
        "caption" => matches!(opening, "td" | "th" | "tr") || table_structure(opening),
        "colgroup" => !matches!(opening, "col" | "template"),
        _ => false,
    }
}

/// [`search`] or [`p_search`].
pub(crate) type SearchFn = fn(&str, &str) -> Option<Search>;

/// What a start tag finds at one open element while looking past the
/// current node for an element it closes (see [`search`]).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Search {
    /// The start tag closes this element (and everything above it).
    Closes,
    /// The search ends here: this element is a scope boundary.
    Stops,
    /// Look further down.
    Continue,
}

/// For the start tags WHATWG lets close an element *below* the current node,
/// what `opening` finds at the open element `open`; `None` for every other
/// start tag ([`implicitly_closes`] only looks at the current node).
///
/// * `<li>`, `<dd>`, `<dt>` close the open list item of their kind, looking
///   past anything but a "special" element (other than `address`, `div` and
///   `p`): `<li><a>One<li>` gives two items.
/// * `<a>`, `<nobr>`, `<button>` and `<select>` close an open one of their own
///   name, looking past anything but a scope boundary
///   ([`is_scope_boundary`]): browsers never nest them (`<a>1<a>2` gives two
///   links).
/// * Table rows, cells and sections close the open cell, row, section or
///   caption they end, looking past anything but `table`, `template` or
///   `html` (table scope): `<td><b>x<td>` gives two cells.
///
/// After that, a start tag that closes a `<p>` closes one in "button scope"
/// ([`p_search`]). The elements above the one closed are closed with it;
/// browsers would reopen formatting elements among them (`<b>`, `<i>`, …)
/// inside the new element, which domq does not.
pub(crate) fn search(opening: &str, open: &str) -> Option<Search> {
    let found = |closes: bool, stops: bool| {
        Some(if closes {
            Search::Closes
        } else if stops {
            Search::Stops
        } else {
            Search::Continue
        })
    };
    match opening {
        "li" => found(
            open == "li",
            is_special(open) && !matches!(open, "address" | "div" | "p"),
        ),
        "dd" | "dt" => found(
            matches!(open, "dd" | "dt"),
            is_special(open) && !matches!(open, "address" | "div" | "p"),
        ),
        "a" | "nobr" | "button" | "select" => found(open == opening, is_scope_boundary(open)),
        "td" | "th" | "tr" => found(
            implicitly_closes(opening, open),
            matches!(open, "table" | "template" | "html"),
        ),
        o if table_structure(o) => found(
            implicitly_closes(opening, open),
            matches!(open, "table" | "template" | "html"),
        ),
        _ => None,
    }
}

/// The elements that end a search "in scope" (WHATWG's default scope, without
/// the SVG and MathML integration points): a search for an element stops at
/// one of these.
pub(crate) fn is_scope_boundary(name: &str) -> bool {
    matches!(
        name,
        "applet" | "caption" | "html" | "marquee" | "object" | "table" | "td" | "template" | "th"
    )
}

/// How far the end tag `closing` looks for the element it closes (see
/// [`EndScope::search`]); decided once per end tag, not per open element.
#[derive(Clone, Copy)]
pub(crate) enum EndScope {
    /// `</body>`, `</html>`, `</template>`: through everything.
    Anywhere,
    /// `</td>`, `</tr>`, `</table>`, …: to the nearest `table`, `template` or
    /// `html`.
    Table,
    /// Every other end tag: to the nearest [scope boundary](is_scope_boundary).
    Default,
}

impl EndScope {
    pub(crate) fn of(closing: &str) -> Self {
        match closing {
            "body" | "html" | "template" => EndScope::Anywhere,
            "table" | "tbody" | "thead" | "tfoot" | "tr" | "td" | "th" | "caption" | "colgroup" => {
                EndScope::Table
            }
            _ => EndScope::Default,
        }
    }

    /// What the end tag `closing` finds at the open element `open` while
    /// looking for the element it closes: [`Search::Closes`] at the element it
    /// names (any open heading, for a heading end tag: `<h2>x</h3>` closes the
    /// `<h2>`), [`Search::Stops`] at a scope boundary, else
    /// [`Search::Continue`].
    ///
    /// A stray `</div>` inside a `<td>` whose `<div>` opened outside the
    /// table is thus ignored instead of closing the cell, the row and the
    /// table.
    #[inline]
    pub(crate) fn search(self, closing: &str, open: &str) -> Search {
        if open == closing || (is_heading(closing) && is_heading(open)) {
            return Search::Closes;
        }
        let stops = match self {
            EndScope::Anywhere => false,
            EndScope::Table => matches!(open, "table" | "template" | "html"),
            EndScope::Default => is_scope_boundary(open),
        };
        if stops {
            Search::Stops
        } else {
            Search::Continue
        }
    }
}

/// [`search`] for the `<p>` that `opening` closes, if it closes one: in
/// "button scope", past inline elements but not past a table, cell, caption,
/// button, object, applet, marquee, template, SVG or MathML.
pub(crate) fn p_search(opening: &str, open: &str) -> Option<Search> {
    if !closes_p(opening) {
        return None;
    }
    Some(if open == "p" {
        Search::Closes
    } else if matches!(
        open,
        "applet"
            | "button"
            | "caption"
            | "html"
            | "marquee"
            | "math"
            | "object"
            | "svg"
            | "table"
            | "td"
            | "template"
            | "th"
    ) {
        Search::Stops
    } else {
        Search::Continue
    })
}

/// WHATWG's "special" elements (13.2.4.2), which end the search of
/// [`search`] for list items.
fn is_special(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "applet"
            | "area"
            | "article"
            | "aside"
            | "base"
            | "basefont"
            | "bgsound"
            | "blockquote"
            | "body"
            | "br"
            | "button"
            | "caption"
            | "center"
            | "col"
            | "colgroup"
            | "dd"
            | "details"
            | "dialog"
            | "dir"
            | "div"
            | "dl"
            | "dt"
            | "embed"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "frame"
            | "frameset"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "head"
            | "header"
            | "hgroup"
            | "hr"
            | "html"
            | "iframe"
            | "img"
            | "input"
            | "keygen"
            | "li"
            | "link"
            | "listing"
            | "main"
            | "marquee"
            | "menu"
            | "meta"
            | "nav"
            | "noembed"
            | "noframes"
            | "noscript"
            | "object"
            | "ol"
            | "p"
            | "param"
            | "plaintext"
            | "pre"
            | "script"
            | "search"
            | "section"
            | "select"
            | "source"
            | "style"
            | "summary"
            | "table"
            | "tbody"
            | "td"
            | "template"
            | "textarea"
            | "tfoot"
            | "th"
            | "thead"
            | "title"
            | "tr"
            | "track"
            | "ul"
            | "wbr"
            | "xmp"
            | "svg"
            | "math"
    )
}

/// Whether non-whitespace text implicitly closes the open element `open`:
/// text never belongs in a `<head>` or a `<colgroup>`.
/// Elements an ancestor's end tag may close without a parse error, because
/// their own end tag is optional (WHATWG "generate implied end tags
/// thoroughly", plus `html`/`head`/`body`): `</ul>` closing an open `<li>` is
/// valid HTML.
pub(crate) fn end_tag_optional(name: &str) -> bool {
    matches!(
        name,
        "body"
            | "caption"
            | "colgroup"
            | "dd"
            | "dt"
            | "head"
            | "html"
            | "li"
            | "optgroup"
            | "option"
            | "p"
            | "rb"
            | "rp"
            | "rt"
            | "rtc"
            | "tbody"
            | "td"
            | "tfoot"
            | "th"
            | "thead"
            | "tr"
    )
}

pub(crate) fn text_closes(open: &str) -> bool {
    matches!(open, "head" | "colgroup")
}

/// How the readable-output serializers (`to_text`, `to_markdown`) lay an
/// element out: a rough stand-in for the CSS `display` of the browser's default
/// stylesheet. Unknown and custom elements are inline.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Display {
    /// Never rendered: metadata, scripts, templates, embedded objects.
    Hidden,
    /// Flows with the surrounding text.
    Inline,
    /// Starts on a new line.
    Block,
    /// Separated from its neighbours by a blank line.
    Paragraph,
}

/// The [`Display`] of an element, by lower-cased tag name.
pub(crate) fn display(name: &str) -> Display {
    match name {
        "head" | "title" | "script" | "style" | "template" | "noscript" | "noembed" | "iframe"
        | "object" | "svg" | "datalist" => Display::Hidden,
        "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote" | "pre" | "ul" | "ol"
        | "menu" | "dl" | "table" | "hr" => Display::Paragraph,
        "address" | "article" | "aside" | "body" | "caption" | "center" | "dd" | "details"
        | "dialog" | "div" | "dt" | "fieldset" | "figcaption" | "figure" | "footer" | "form"
        | "header" | "hgroup" | "html" | "legend" | "li" | "main" | "nav" | "optgroup"
        | "option" | "search" | "section" | "summary" | "tbody" | "textarea" | "tfoot"
        | "thead" | "tr" => Display::Block,
        _ => Display::Inline,
    }
}
