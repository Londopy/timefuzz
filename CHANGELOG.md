# Changelog

## 0.2.0 — 2026-07-12

Delivers the v0.2–v0.4 roadmap items: richer anchored phrases,
business-calendar rules, confidence-model refinement, more `Ambiguous`
candidates, corpus expansion, benches, and cookbook growth.

### Richer anchored phrases
- Bare anchors: `on my birthday` resolves the anchor date itself.
- `2nd tuesday after my birthday` (ordinal weekday after an anchor).
- `the week/month after/before <anchor>` → `Range`.
- `the weekend after <anchor>` → `Range`.
- `N business days after/before <anchor>` (respects the holiday hook).

### Business calendar
- `first/2nd/nth business day of <period>`, with `ParseError` when the
  period has fewer business days than asked.
- `N business days ago`.
- Shorthand: `eow`, `eom`, `eoq`, `eoy`.
- Bare units name the current period: `mid month`, `end of week`.

### Weekends
- New period type (Sat–Sun): `this/last/next weekend`,
  `sometime next weekend`, `the weekend after <anchor>`.

### Weekday-of-week
- `friday next week`, `monday last week`, `tuesday this week` — the
  unambiguous alternative to `next friday`.

### Confidence refinement
- `DOUBLE_VAGUE` (0.70) tier for stacked hedges (`sometime early next month`),
  tagged "(doubly hedged)".
- Horizon penalty: relative offsets beyond ~10 years lose 0.05 and are tagged
  "(distant horizon)".
- `<weekday> this week` in the past takes the same passed-date penalty as
  `this <weekday>`.
- f32 representation noise hidden at the Python boundary (scores round-trip
  as clean decimals).

### More `Ambiguous` candidates
- `next weekend` said midweek: the coming weekend vs. the one after.
- A bare month named during that month (`july` in July): current month vs.
  next year.
- (Existing: bare weekday on that weekday; unknown anchors.)

### Corpus, benches, docs
- Corpus grown to 90+ cases (`tests/corpus.jsonl` + `tests/corpus_v02.jsonl`);
  192 tests total.
- Benches split: full-corpus, single-phrase, tokenizer-only.
- Grammar reference §8 (weekends), expanded §5/§7; confidence doc covers the
  new tiers and built-in ambiguity table; six new cookbook recipes.

## 0.1.0 — 2026-07-12

Initial release: tokenizer + rule-based grammar (relative offsets, weekday
navigation, ordinal-in-period, anchored phrases, vague spans, business
calendar), deterministic confidence model, `Instant`/`Range`/`Ambiguous`
result types, corpus-driven test suite, Rust core with PyO3 bindings shipped
as abi3 wheels.
