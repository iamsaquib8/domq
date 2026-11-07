Parsing is faster on real pages: the tokenizer classifies each tag name once
(the tree builder's rules are then integer tests instead of string matches),
notes whether text and attribute values hold a `&` so values without one skip
the entity search, and an `<image>` outside `<svg>` and `<math>` no longer
scans the open elements for them (200,000 `<image>` under 300 `<div>`s: about
1 s to about 10 ms). The tree and the diagnostics are unchanged
([#73](https://github.com/iamsaquib8/domq/issues/73)).
