# Changelog

## 0.4.0 — 2026-08-01

Bare durations: the `in` in `in 30 minutes` is now optional.

### New grammar rule — `<qty> <unit>` (`src/grammar/duration.rs`)
- `30 minutes`, `2 hrs`, `3 days`, `a week`, `two weeks`, `6 months` resolve
  exactly like their `in …` forms, at the same confidence. Writing a short
  interval without the marker is the most common way people type an ETA and
  previously raised `ParseError`.
- `half an hour`, `half hour`, `half a day`, `half a week`, `half a minute`
  and `half a year` are supported. A unit is halved only where the halving is
  exact under the arithmetic this crate already uses for it, so `half a month`
  and `half a quarter` still don't parse rather than silently rounding.
- Approximate quantities — `a couple hours`, `a few days`, `several weeks` —
  resolve to 2/3/3 units at `BARE` confidence with "approximately …" in the
  interpretation, so callers confirm instead of scheduling silently. The
  article is optional (`couple hours`) and `of` is tolerated
  (`a couple of hours`).
- `3 business days` works bare too, skipping weekends and `holidays`.
- Unrecognized **leading** words are read past — `back in a couple hours`,
  `gimme 5 mins`, `ttyl in 2 hrs` — at a 0.05 confidence cost, with the
  ignored words named in the interpretation. This tolerance is scoped to this
  one rule; every other rule still matches the whole token stream strictly, so
  `blah next friday` remains a `ParseError`.

### Deliberately still unparsed
`week` (a bare unit is a period, not a duration), `5 apples`, `3 whole days`
(unknown words are only skipped at the *start*), `5 mins ok`, `2 weekends`,
`an hour and a half`, `90 seconds`.

### Locale
- Two new `Locale` tables — `fuzzy_words` and `half_words` — so the duration
  vocabulary crosses the i18n seam like every other word list; the tokenizer
  and grammar stay language-neutral. New `Tok::Fuzzy(i64)` and `Tok::Half`
  token types, with tokenizer merges for `a couple` → `couple` and
  `half an hour` → `half hour`.
- The miniature Spanish locale now proves bare durations tokenize in another
  language with zero grammar changes.

### Compatibility
No phrase that parsed before 0.4.0 changes its result; verified by a
differential run of every corpus and documented phrase (227) against 0.3.1 —
198 identical, 29 previously `ParseError`, 0 changed results. Adding
`Tok::Fuzzy`/`Tok::Half` is source-breaking only for downstream Rust code
matching exhaustively on `Tok`; the Python API is unchanged.

### Tests
- `tests/corpus_v04.jsonl` (46 cases) and `tests/test_duration.py` (59 cases);
  suite is 329 green, up from 224.
- `parse_durations_6` added to the benches, alongside the untouched
  `parse_corpus_18` so earlier numbers stay comparable.

## 0.3.1 — 2026-07-12

- Release CI: the retired `macos-13` runner label left the macOS x86-64 wheel
  job queued forever; moved to `macos-15-intel` (GitHub's last Intel image,
  supported to Aug 2027).
- CI (from 0.3.0's first runs): install `tzdata` on Windows runners so the
  zoneinfo tests pass.

## 0.3.0 — 2026-07-12

Pre-1.0 groundwork: the i18n seam, clock-time support, and a written API
stability policy.

### i18n-ready grammar structure
- All language-specific word tables (weekdays, months, units, keywords,
  numbers, ordinals, fillers, shorthand, am/pm, noon/midnight) extracted into
  a `Locale` struct (`src/locale.rs`); the tokenizer takes a locale and the
  grammar rules are fully language-neutral.
- English (`EN`) is the built-in default via `Cfg.locale`.
- Proven by unit tests: a miniature Spanish locale tokenizes and parses
  end-to-end with zero grammar changes. `cargo test --lib` added to CI.
- Python API unchanged; locale selection will be exposed post-1.0.

### Clock-time support
- Trailing times override the default time of day: `next friday at 3pm`,
  `tomorrow at 7:30`, `end of month at 5pm`, `the tuesday after my birthday
  at 5pm` — works with every date-producing rule, including `Ambiguous`
  candidates.
- Bare times: `3pm`, `15:45`, `noon`, `midnight` resolve to the next
  occurrence (today, or tomorrow if passed).
- Formats: `3pm`, `3 pm`, `3:30pm`, `7:30`, `15:45`, `noon`, `midnight`;
  `12pm`=noon, `12am`=midnight; invalid times (`13pm`, `25:00`) don't parse.
- `in 3 days at 5pm` applies the time to day-granular arithmetic;
  minute/hour arithmetic keeps its own clock.
- Explicit times never change confidence; ranges ignore trailing times.

### Stable-API groundwork
- docs/stability.md: semver commitment, the contract/tunable split
  (signatures, range inclusivity, and conventions are contracts; exact
  confidence values and interpretation wording are tunables), deprecation
  process, and the pre-1.0 checklist.

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
