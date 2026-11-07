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

// --- Tag classification ---------------------------------------------------
//
// The tree builder asks many questions about each tag name (void? closes a
// `<p>`? a scope boundary? counted?). They are answered from a [`Tag`]: the
// name is looked up once, in the tokenizer, and everything after that is an
// integer test on the tag's id or its flag bits. Unknown names (`<my-tag>`)
// are [`Tag::OTHER`], which has no flags.

const VOID: u16 = 1 << 0;
const RAW_TEXT: u16 = 1 << 1;
/// The tokenizer reads the content as text up to the end tag: `RAW_TEXT`
/// plus `title` and `textarea`.
const RAWISH: u16 = 1 << 2;
const CLOSES_P: u16 = 1 << 3;
const SPECIAL: u16 = 1 << 4;
const SCOPE: u16 = 1 << 5;
const HEADING: u16 = 1 << 6;
const IN_HEAD: u16 = 1 << 7;
const TABLE_STRUCT: u16 = 1 << 8;
const P_STOP: u16 = 1 << 9;
const END_OPTIONAL: u16 = 1 << 10;
/// List items, cells, rows, table sections and captions: what [`search`]
/// closes (counted in the builder's `open_items`).
const ITEM: u16 = 1 << 11;
/// `a`, `nobr`, `button` and `select`: closed by a start tag of the same name.
const NESTED: u16 = 1 << 12;
const DROPS_LF: u16 = 1 << 13;
const TEXT_CLOSES: u16 = 1 << 14;

/// A tag name classified once: an id for the names the builder has rules for
/// (`0` for any other name) that indexes the flag table.
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(Debug))]
pub(crate) struct Tag(u8);

macro_rules! tags {
    ($($id:ident $name:literal $flags:expr,)*) => {
        /// Numbers the ids (`Tag::A` is `Index::A + 1`).
        #[allow(dead_code, clippy::upper_case_acronyms)]
        enum Index { $($id,)* }
        #[allow(dead_code)]
        impl Tag {
            $(pub(crate) const $id: Tag = Tag(Index::$id as u8 + 1);)*
        }
        /// The lower-case names, in id order (id `n` is `NAMES[n - 1]`).
        const NAMES: &[&str] = &[$($name),*];
        /// The flags of each id (`0`, no flags, for [`Tag::OTHER`]).
        const FLAGS: &[u16] = &[0, $($flags),*];
    };
}

tags! {
    A "a" NESTED,
    ADDRESS "address" CLOSES_P|SPECIAL,
    APPLET "applet" SPECIAL|SCOPE|P_STOP,
    AREA "area" VOID|SPECIAL,
    ARTICLE "article" CLOSES_P|SPECIAL,
    ASIDE "aside" CLOSES_P|SPECIAL,
    BASE "base" VOID|SPECIAL|IN_HEAD,
    BASEFONT "basefont" SPECIAL|IN_HEAD,
    BGSOUND "bgsound" SPECIAL|IN_HEAD,
    BLOCKQUOTE "blockquote" CLOSES_P|SPECIAL,
    BODY "body" SPECIAL|END_OPTIONAL,
    BR "br" VOID|SPECIAL,
    BUTTON "button" SPECIAL|P_STOP|NESTED,
    CAPTION "caption" SPECIAL|SCOPE|TABLE_STRUCT|P_STOP|END_OPTIONAL|ITEM,
    CENTER "center" CLOSES_P|SPECIAL,
    COL "col" VOID|SPECIAL|TABLE_STRUCT,
    COLGROUP "colgroup" SPECIAL|TABLE_STRUCT|END_OPTIONAL|TEXT_CLOSES,
    DD "dd" CLOSES_P|SPECIAL|END_OPTIONAL|ITEM,
    DETAILS "details" CLOSES_P|SPECIAL,
    DIALOG "dialog" CLOSES_P|SPECIAL,
    DIR "dir" CLOSES_P|SPECIAL,
    DIV "div" CLOSES_P|SPECIAL,
    DL "dl" CLOSES_P|SPECIAL,
    DT "dt" CLOSES_P|SPECIAL|END_OPTIONAL|ITEM,
    EMBED "embed" VOID|SPECIAL,
    FIELDSET "fieldset" CLOSES_P|SPECIAL,
    FIGCAPTION "figcaption" CLOSES_P|SPECIAL,
    FIGURE "figure" CLOSES_P|SPECIAL,
    FOOTER "footer" CLOSES_P|SPECIAL,
    FORM "form" CLOSES_P|SPECIAL,
    FRAME "frame" SPECIAL,
    FRAMESET "frameset" SPECIAL,
    H1 "h1" CLOSES_P|SPECIAL|HEADING,
    H2 "h2" CLOSES_P|SPECIAL|HEADING,
    H3 "h3" CLOSES_P|SPECIAL|HEADING,
    H4 "h4" CLOSES_P|SPECIAL|HEADING,
    H5 "h5" CLOSES_P|SPECIAL|HEADING,
    H6 "h6" CLOSES_P|SPECIAL|HEADING,
    HEAD "head" SPECIAL|END_OPTIONAL|TEXT_CLOSES,
    HEADER "header" CLOSES_P|SPECIAL,
    HGROUP "hgroup" CLOSES_P|SPECIAL,
    HR "hr" VOID|CLOSES_P|SPECIAL,
    HTML "html" SPECIAL|SCOPE|P_STOP|END_OPTIONAL,
    IFRAME "iframe" RAW_TEXT|RAWISH|SPECIAL,
    IMAGE "image" 0,
    IMG "img" VOID|SPECIAL,
    INPUT "input" VOID|SPECIAL,
    KEYGEN "keygen" SPECIAL,
    LI "li" CLOSES_P|SPECIAL|END_OPTIONAL|ITEM,
    LINK "link" VOID|SPECIAL|IN_HEAD,
    LISTING "listing" CLOSES_P|SPECIAL|DROPS_LF,
    MAIN "main" CLOSES_P|SPECIAL,
    MARQUEE "marquee" SPECIAL|SCOPE|P_STOP,
    MATH "math" SPECIAL|P_STOP,
    MENU "menu" CLOSES_P|SPECIAL,
    META "meta" VOID|SPECIAL|IN_HEAD,
    NAV "nav" CLOSES_P|SPECIAL,
    NOBR "nobr" NESTED,
    NOEMBED "noembed" RAW_TEXT|RAWISH|SPECIAL,
    NOFRAMES "noframes" RAW_TEXT|RAWISH|SPECIAL|IN_HEAD,
    NOSCRIPT "noscript" SPECIAL|IN_HEAD,
    OBJECT "object" SPECIAL|SCOPE|P_STOP,
    OL "ol" CLOSES_P|SPECIAL,
    OPTGROUP "optgroup" END_OPTIONAL,
    OPTION "option" END_OPTIONAL,
    P "p" CLOSES_P|SPECIAL|END_OPTIONAL,
    PARAM "param" VOID|SPECIAL,
    PLAINTEXT "plaintext" RAW_TEXT|RAWISH|CLOSES_P|SPECIAL,
    PRE "pre" CLOSES_P|SPECIAL|DROPS_LF,
    RB "rb" END_OPTIONAL,
    RP "rp" END_OPTIONAL,
    RT "rt" END_OPTIONAL,
    RTC "rtc" END_OPTIONAL,
    SCRIPT "script" RAW_TEXT|RAWISH|SPECIAL|IN_HEAD,
    SEARCH "search" CLOSES_P|SPECIAL,
    SECTION "section" CLOSES_P|SPECIAL,
    SELECT "select" SPECIAL|NESTED,
    SOURCE "source" VOID|SPECIAL,
    STYLE "style" RAW_TEXT|RAWISH|SPECIAL|IN_HEAD,
    SUMMARY "summary" CLOSES_P|SPECIAL,
    SVG "svg" SPECIAL|P_STOP,
    TABLE "table" CLOSES_P|SPECIAL|SCOPE|P_STOP,
    TBODY "tbody" SPECIAL|TABLE_STRUCT|END_OPTIONAL|ITEM,
    TD "td" SPECIAL|SCOPE|P_STOP|END_OPTIONAL|ITEM,
    TEMPLATE "template" SPECIAL|SCOPE|IN_HEAD|P_STOP,
    TEXTAREA "textarea" RAWISH|SPECIAL|DROPS_LF,
    TFOOT "tfoot" SPECIAL|TABLE_STRUCT|END_OPTIONAL|ITEM,
    TH "th" SPECIAL|SCOPE|P_STOP|END_OPTIONAL|ITEM,
    THEAD "thead" SPECIAL|TABLE_STRUCT|END_OPTIONAL|ITEM,
    TITLE "title" RAWISH|SPECIAL|IN_HEAD,
    TR "tr" SPECIAL|END_OPTIONAL|ITEM,
    TRACK "track" VOID|SPECIAL,
    UL "ul" CLOSES_P|SPECIAL,
    WBR "wbr" VOID|SPECIAL,
    XMP "xmp" RAW_TEXT|RAWISH|CLOSES_P|SPECIAL,
}

/// The longest name in [`NAMES`].
const MAX_NAME_LEN: usize = 10;
const TABLE_SIZE: usize = 256;

/// A hash of `name` that ignores ASCII case (`b | 0x20` folds a letter's case
/// and keeps the other bytes apart often enough: the lookup compares the
/// whole name anyway).
const fn hash(name: &[u8]) -> usize {
    let mut h: u32 = 0;
    let mut i = 0;
    while i < name.len() {
        h = h.wrapping_mul(31).wrapping_add((name[i] | 0x20) as u32);
        i += 1;
    }
    ((h ^ (h >> 7)) as usize) & (TABLE_SIZE - 1)
}

/// Open addressing over [`NAMES`]: slot → id (`0`: empty). Built at compile
/// time; with 95 names in 256 slots a probe rarely takes a second step.
const TABLE: [u8; TABLE_SIZE] = {
    let mut table = [0u8; TABLE_SIZE];
    let mut i = 0;
    while i < NAMES.len() {
        let mut slot = hash(NAMES[i].as_bytes());
        while table[slot] != 0 {
            slot = (slot + 1) & (TABLE_SIZE - 1);
        }
        table[slot] = (i + 1) as u8;
        i += 1;
    }
    table
};

impl Tag {
    /// Any name the builder has no rule for.
    pub(crate) const OTHER: Tag = Tag(0);

    /// Classify `name`, ignoring ASCII case.
    #[inline]
    pub(crate) fn of(name: &str) -> Tag {
        let bytes = name.as_bytes();
        if bytes.is_empty() || bytes.len() > MAX_NAME_LEN {
            return Tag::OTHER;
        }
        let mut slot = hash(bytes);
        // The table has empty slots, so a probe ends; the bound is only a
        // guard against a table that is somehow full.
        for _ in 0..TABLE_SIZE {
            let id = TABLE.get(slot).copied().unwrap_or(0);
            if id == 0 {
                break;
            }
            let tag = Tag(id);
            if tag.name().as_bytes().eq_ignore_ascii_case(bytes) {
                return tag;
            }
            slot = (slot + 1) & (TABLE_SIZE - 1);
        }
        Tag::OTHER
    }

    /// The lower-case name of a known tag (`""` for [`Tag::OTHER`]).
    #[inline]
    pub(crate) fn name(self) -> &'static str {
        NAMES
            .get(usize::from(self.0).wrapping_sub(1))
            .copied()
            .unwrap_or("")
    }

    #[inline]
    fn has(self, flag: u16) -> bool {
        FLAGS
            .get(usize::from(self.0))
            .is_some_and(|f| f & flag != 0)
    }

    /// Elements that never have children and never take an end tag.
    #[inline]
    pub(crate) fn is_void(self) -> bool {
        self.has(VOID)
    }

    /// Elements whose content is raw text (see [`is_raw_text`]).
    #[inline]
    pub(crate) fn is_raw_text(self) -> bool {
        self.has(RAW_TEXT)
    }

    /// Elements the tokenizer reads as text up to their end tag: the raw-text
    /// ones plus `title` and `textarea`.
    #[inline]
    pub(crate) fn is_rawish(self) -> bool {
        self.has(RAWISH)
    }

    /// `<pre>`, `<listing>`, `<textarea>`: a newline starting their content
    /// is dropped.
    #[inline]
    pub(crate) fn drops_lf(self) -> bool {
        self.has(DROPS_LF)
    }

    /// Whether non-whitespace text implicitly closes this open element: text
    /// never belongs in a `<head>` or a `<colgroup>`.
    #[inline]
    pub(crate) fn text_closes(self) -> bool {
        self.has(TEXT_CLOSES)
    }

    /// Elements an ancestor's end tag may close without a parse error,
    /// because their own end tag is optional (WHATWG "generate implied end
    /// tags thoroughly", plus `html`/`head`/`body`): `</ul>` closing an open
    /// `<li>` is valid HTML.
    #[inline]
    pub(crate) fn end_tag_optional(self) -> bool {
        self.has(END_OPTIONAL)
    }

    /// Whether the builder counts this element while it is open (see
    /// `OpenElements`): a `<p>`, an item (what [`search`] closes), or one of the
    /// elements closed by a start tag of their own name.
    #[inline]
    pub(crate) fn is_counted(self) -> bool {
        self.has(ITEM | NESTED) || self == Tag::P
    }

    #[inline]
    fn closes_p(self) -> bool {
        self.has(CLOSES_P)
    }

    #[inline]
    fn is_heading(self) -> bool {
        self.has(HEADING)
    }

    #[inline]
    fn in_head(self) -> bool {
        self.has(IN_HEAD)
    }

    #[inline]
    fn table_structure(self) -> bool {
        self.has(TABLE_STRUCT)
    }

    #[inline]
    fn is_special(self) -> bool {
        self.has(SPECIAL)
    }

    /// The elements that end a search "in scope" (WHATWG's default scope,
    /// without the SVG and MathML integration points): a search for an
    /// element stops at one of these.
    #[inline]
    pub(crate) fn is_scope_boundary(self) -> bool {
        self.has(SCOPE)
    }

    /// `table`, `template` or `html`: where a search in table scope ends.
    #[inline]
    fn ends_table_scope(self) -> bool {
        matches!(self, Tag::TABLE | Tag::TEMPLATE | Tag::HTML)
    }

    /// `a`, `nobr`, `button` and `select`.
    #[inline]
    pub(crate) fn is_nested(self) -> bool {
        self.has(NESTED)
    }
}

/// Given a start tag `opening` and the tag `open` currently on top of the
/// open-element stack, decide whether `open` should be implicitly closed
/// first.
///
/// These are the WHATWG "optional tags" rules (13.1.2.4) — an end tag may be
/// omitted when the element is immediately followed by one of these start
/// tags — together with the tree-construction cases that act on the current
/// node: `<li>`/`<dt>`/`<dd>`/headings closing `<p>`, a heading closing an
/// open heading, `<colgroup>` ending at the first non-`<col>` content, and
/// table sections, rows, cells and captions closing each other. They only ever
/// look at the *top* of the stack, so the builder needs no insertion modes,
/// scopes or active-formatting list.
///
/// `closes_p` covers every element the "optional tags" rules let a `</p>` be
/// omitted before, plus the other start tags tree construction closes a `<p>`
/// for (`center`, `dir`, `li`, `dd`, `dt`, `summary`, `listing`, `plaintext`,
/// `xmp`).
#[inline]
pub(crate) fn implicitly_closes(opening: Tag, open: Tag) -> bool {
    match open {
        Tag::P => opening.closes_p(),
        Tag::LI => opening == Tag::LI,
        Tag::DT | Tag::DD => matches!(opening, Tag::DT | Tag::DD),
        Tag::H1 | Tag::H2 | Tag::H3 | Tag::H4 | Tag::H5 | Tag::H6 => opening.is_heading(),
        Tag::HEAD => !opening.in_head(),
        Tag::OPTION => matches!(opening, Tag::OPTION | Tag::OPTGROUP | Tag::HR),
        Tag::OPTGROUP => matches!(opening, Tag::OPTGROUP | Tag::HR),
        Tag::RB | Tag::RT | Tag::RP => matches!(opening, Tag::RB | Tag::RT | Tag::RTC | Tag::RP),
        Tag::RTC => matches!(opening, Tag::RB | Tag::RTC),
        Tag::TD | Tag::TH => {
            matches!(opening, Tag::TD | Tag::TH | Tag::TR) || opening.table_structure()
        }
        Tag::TR => opening == Tag::TR || opening.table_structure(),
        Tag::THEAD | Tag::TBODY | Tag::TFOOT => opening.table_structure(),
        Tag::CAPTION => matches!(opening, Tag::TD | Tag::TH | Tag::TR) || opening.table_structure(),
        Tag::COLGROUP => !matches!(opening, Tag::COL | Tag::TEMPLATE),
        _ => false,
    }
}

/// [`search`] or [`p_search`].
pub(crate) type SearchFn = fn(Tag, Tag) -> Option<Search>;

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
///   ([`Tag::is_scope_boundary`]): browsers never nest them (`<a>1<a>2` gives
///   two links).
/// * Table rows, cells and sections close the open cell, row, section or
///   caption they end, looking past anything but `table`, `template` or
///   `html` (table scope): `<td><b>x<td>` gives two cells.
///
/// After that, a start tag that closes a `<p>` closes one in "button scope"
/// ([`p_search`]). The elements above the one closed are closed with it;
/// browsers would reopen formatting elements among them (`<b>`, `<i>`, …)
/// inside the new element, which domq does not.
pub(crate) fn search(opening: Tag, open: Tag) -> Option<Search> {
    let found = |closes: bool, stops: bool| {
        Some(if closes {
            Search::Closes
        } else if stops {
            Search::Stops
        } else {
            Search::Continue
        })
    };
    // "special" elements other than `address`, `div` and `p` end a list
    // item's search.
    let item_stops = || open.is_special() && !matches!(open, Tag::ADDRESS | Tag::DIV | Tag::P);
    match opening {
        Tag::LI => found(open == Tag::LI, item_stops()),
        Tag::DD | Tag::DT => found(matches!(open, Tag::DD | Tag::DT), item_stops()),
        Tag::A | Tag::NOBR | Tag::BUTTON | Tag::SELECT => {
            found(open == opening, open.is_scope_boundary())
        }
        Tag::TD | Tag::TH | Tag::TR => {
            found(implicitly_closes(opening, open), open.ends_table_scope())
        }
        o if o.table_structure() => {
            found(implicitly_closes(opening, open), open.ends_table_scope())
        }
        _ => None,
    }
}

/// [`search`] for the `<p>` that `opening` closes, if it closes one: in
/// "button scope", past inline elements but not past a table, cell, caption,
/// button, object, applet, marquee, template, SVG or MathML.
pub(crate) fn p_search(opening: Tag, open: Tag) -> Option<Search> {
    if !opening.closes_p() {
        return None;
    }
    Some(if open == Tag::P {
        Search::Closes
    } else if open.has(P_STOP) {
        Search::Stops
    } else {
        Search::Continue
    })
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
    /// Every other end tag: to the nearest [scope boundary](Tag::is_scope_boundary).
    Default,
}

impl EndScope {
    pub(crate) fn of(closing: Tag) -> Self {
        match closing {
            Tag::BODY | Tag::HTML | Tag::TEMPLATE => EndScope::Anywhere,
            Tag::TABLE
            | Tag::TBODY
            | Tag::THEAD
            | Tag::TFOOT
            | Tag::TR
            | Tag::TD
            | Tag::TH
            | Tag::CAPTION
            | Tag::COLGROUP => EndScope::Table,
            _ => EndScope::Default,
        }
    }

    /// What the end tag `closing` finds at the open element `open` while
    /// looking for the element it closes: [`Search::Closes`] at the element it
    /// names (`named`; any open heading, for a heading end tag: `<h2>x</h3>`
    /// closes the `<h2>`), [`Search::Stops`] at a scope boundary, else
    /// [`Search::Continue`].
    ///
    /// A stray `</div>` inside a `<td>` whose `<div>` opened outside the
    /// table is thus ignored instead of closing the cell, the row and the
    /// table.
    #[inline]
    pub(crate) fn search(self, closing: Tag, open: Tag, named: bool) -> Search {
        if named || (closing.is_heading() && open.is_heading()) {
            return Search::Closes;
        }
        let stops = match self {
            EndScope::Anywhere => false,
            EndScope::Table => open.ends_table_scope(),
            EndScope::Default => open.is_scope_boundary(),
        };
        if stops {
            Search::Stops
        } else {
            Search::Continue
        }
    }
}

#[cfg(test)]
#[path = "tags_reference.rs"]
mod reference;

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
