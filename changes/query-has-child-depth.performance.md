`:has(> x)` (and any `:has()` whose argument uses only `>`, `+` and `~`) looks only
as many levels down as the argument has `>`s, not at the candidate's whole
subtree: `div:has(> span)` on 20,000 nested divs took 1.8 s to 8 s and now takes
under a millisecond ([#77](https://github.com/iamsaquib8/domq/issues/77)).
