//! The outline must be what the quadratic algorithm it replaced made
//! (`super::oracle`), on every page and from every node.

use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use proptest::prelude::*;

use super::oracle;
use crate::{Dom, parse};

fn assert_same(dom: &Dom, what: &str) {
    assert_eq!(dom.outline(), oracle::outline(dom), "{what}");
    // From any element: its ancestors still count.
    for node in dom.nodes().filter(|n| n.is_element()).take(12) {
        assert_eq!(
            node.outline(),
            oracle::node_outline(node),
            "{what}: from <{}>",
            node.tag_name().unwrap_or("")
        );
    }
}

fn check(html: &str) {
    assert_same(&parse(html), html);
}

#[test]
fn names_are_cut_at_the_limit_whatever_the_text_is_made_of() {
    // Around the 100-character limit, with whitespace that `to_text` trims
    // but `collapse` keeps (an em space), in nested and in separate nodes.
    for len in [0, 1, 99, 100, 101, 102, 150] {
        let x = "x".repeat(len);
        for html in [
            format!("<a href=/>{x}</a>"),
            format!("<a href=/>{x}\u{2003}</a>"),
            format!("<a href=/>{x}\u{2003}<b>y</b></a>"),
            format!("<a href=/>{x}\u{2003}\u{2003}<b>\u{2003}</b><i>z</i></a>"),
            format!(
                "<a href=/>{}<b>{}</b>{}</a>",
                &x[..len / 2],
                &x[len / 2..],
                "w"
            ),
            format!("<a href=/>{}  \n {}</a>", &x[..len / 2], &x[len / 2..]),
            format!("<a href=/>\u{2003}{x}</a>"),
            format!("<button><pre>{x}\n\n</pre>after</button>"),
            format!("<h1>{x}<br><br>tail</h1>"),
            format!("<p id=n>{x} tail</p><button aria-labelledby='n n'>b</button>"),
            format!("<label for=q>{x}</label><label for=q>second</label><input id=q>"),
            format!("<label>{x}\u{2003}<input></label><label><input></label>"),
        ] {
            check(&html);
        }
    }
}

#[test]
fn names_come_from_the_right_label() {
    for html in [
        // The first label with text wins; the empty one is skipped.
        "<label for=a></label><label for=a>Second</label><label for=a>Third</label><input id=a>",
        // A label for an id used twice still names both.
        "<label for=a>L</label><input id=a><input id=a>",
        // `for` is matched exactly; the tag name is not case sensitive.
        "<LABEL FOR=a>Upper</LABEL><label for=A>Lower</label><input id=a><input id=A>",
        // An ancestor label with no text falls through to the placeholder.
        "<label><input placeholder=ph></label><label>T<span><input></span></label>",
        // Nested labels: the nearest one is used.
        "<label>Outer<label>Inner<input></label><input></label>",
        // aria-labelledby: the first element with the id, many ids, a miss.
        "<i id=a>One</i><i id=a>Two</i><i id=b></i><button aria-labelledby='b a x'>z</button>",
        // Labels outside the node `outline` is called on.
        "<label for=a>Out</label><section><input id=a></section>",
    ] {
        check(html);
    }
}

#[test]
fn selectors_are_the_same_with_repeats_everywhere() {
    for html in [
        "<ul><li><a href=/1>1</a></li><li><a href=/2>2</a></li><li class=x><a href=/3 class='x x'>3</a></li></ul>",
        "<div id=a><p>1</p><span>2</span><p>3</p></div><div id=a><button>b</button></div>",
        "<main><header><nav><a href=/>h</a></nav></header><footer><a href=/>f</a></footer></main>",
        "<header><a href=/>h</a></header><footer><a href=/>f</a></footer><article><header>x</header></article>",
        "<form><input name=q><input name=q type=search><button type=submit>go</button><button type=submit>go</button></form>",
        "<div data-testid=a><button data-test-id=a>x</button><button data-testid=a>y</button></div>",
        "<button class='a b'>1</button><button class='b c'>2</button><a class='a' href=/>3</a>",
        "<div id=' x '><button>1</button></div><div id='a b'><button>2</button></div><div id=9><button>3</button></div>",
        "<table><tr><td><a href=/>1</a></td><td><a href=/>2</a></td></tr><tr><td><a href=/>3</a></td></tr></table>",
    ] {
        check(html);
    }
}

fn soup() -> impl Strategy<Value = String> {
    let long = "word ".repeat(30);
    let pieces: Vec<String> = [
        "<header>",
        "</header>",
        "<footer>",
        "</footer>",
        "<nav>",
        "</nav>",
        "<main>",
        "</main>",
        "<section>",
        "</section>",
        "<article>",
        "</article>",
        "<aside>",
        "<form id=f1>",
        "<form>",
        "</form>",
        "<div>",
        "</div>",
        "<div role=button>",
        "<div role=heading aria-level=3>",
        "<div role=tab id=t1>",
        "<div role='link' class='a b a'>",
        "<div role=none>",
        "<span>",
        "</span>",
        "<a href=/x>",
        "<a href=/y class=c1>",
        "<a href=/z id=i1>",
        "<a>",
        "</a>",
        "<a href=/x aria-label='AL'>",
        "<button>",
        "<button class=btn>",
        "<button type=submit>",
        "<button type=reset name=n>",
        "</button>",
        "<input id=i1>",
        "<input id=i2 name=q>",
        "<input name=q type=search>",
        "<input type=hidden>",
        "<input type=checkbox id=i3 placeholder=ph>",
        "<input value=v>",
        "<input title=T>",
        "<input type=submit>",
        "<textarea id=i1>",
        "</textarea>",
        "<select name=s>",
        "</select>",
        "<label for=i1>",
        "<label for=i2>",
        "<label for=i1></label>",
        "<label>",
        "</label>",
        "<h1>",
        "</h1>",
        "<h2 class=c1>",
        "</h2>",
        "<h3 id=dup>",
        "<h4 id=dup>",
        "<h3 id='1bad'>",
        "<div id=' x '>",
        "<div id='a b'>",
        "<ul>",
        "</ul>",
        "<li>",
        "<table><tr><td>",
        "</table>",
        "<pre>",
        "</pre>",
        "<img alt=pic>",
        "<img alt='alt text' src=x>",
        "<p>",
        "text word ",
        "  ",
        "\u{2003}",
        "\u{a0}",
        "<br>",
        "<span hidden>",
        "<div aria-hidden=true>",
        "<div style='display:none'>",
        "<script>x</script>",
        "<span id=lab1>Lab one</span>",
        "<button aria-labelledby='lab1 i2 nope'>",
        "<div aria-labelledby='t1 lab1'>",
        "<div data-testid=t1>",
        "<div data-testid=t2 data-test-id=tt>",
        "<i class='c1'>",
        "<i class='c1 c2'>",
        "<details><summary>S",
        "</details>",
        "<template>",
        "<noscript>",
        "<b>\u{2003}</b>",
        long.as_str(),
        "<em>",
        "</em>",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    prop::collection::vec(prop::sample::select(pieces), 0..120).prop_map(|v| v.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn outline_matches_the_quadratic_algorithm(html in soup()) {
        check(&html);
    }
}
