`:first-of-type`, `:last-of-type`, `:nth-of-type()`, `:nth-last-of-type()` and
`:only-of-type` are linear on sibling lists with many distinct tag names (they were
quadratic: 20,000 distinct siblings took 1.4 to 4 s and now take about 5 ms)
([#72](https://github.com/iamsaquib8/domq/issues/72)).
