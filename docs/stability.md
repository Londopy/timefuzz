# API stability policy

This document says exactly what timefuzz promises at **v1.0** — what is
frozen, what may still move, and how changes are communicated. Until 1.0,
0.x minor releases may adjust anything below (with changelog notes).

## Semver commitment (from 1.0)

- **Major** (2.0): any break to a *contract* below.
- **Minor** (1.x): new grammar rules, new `Config` fields with defaults, new
  locales, tunable adjustments.
- **Patch**: bug fixes where behavior contradicted the documented contract.

## Contracts — frozen at 1.0

**Signatures and types.**

- `parse(text, now=None, anchors=None, config=None) -> Instant | Range | Ambiguous`
- `Instant(when, confidence, interpretation)`,
  `Range(start, end, confidence, interpretation)`,
  `Ambiguous(candidates, reason)` — frozen dataclasses; field names, types,
  and immutability are contracts.
- `ParseError` (subclass of `TimefuzzError`) raised only when no rule matches
  or the phrase names something impossible.
- `Config` field names, types, and defaults:
  `default_time=09:00`, `week_start=MON`, `next_skips_today=True`,
  `tz=None`, `holidays=None`.

**Semantics.**

- Ranges are **inclusive**: first day 00:00:00 through last day 23:59:59,
  and `start <= end` always.
- **Naive by default**; `Config(tz=...)` opts into tz-aware results with
  wall-clock math across DST.
- `next <weekday>` = first occurrence strictly after today;
  `next_skips_today` governs the same-day case.
- Date-only results carry `default_time`; an explicit trailing clock time
  ("… at 3pm") overrides it.
- Arithmetic offsets keep the clock (`in 3 days` = now + 72h).
- Weekends are Sat–Sun regardless of `week_start`.
- Confidence is deterministic: same input + same config = same score, always.
- `Ambiguous.candidates` are ordered most-likely-first; unknown anchors give
  an empty candidate list with a reason naming the anchor.

## Tunables — may change in any minor release

- **Exact confidence values.** The *ordering* of the tiers
  (CERTAIN > EXACT > STRONG > CALENDAR > VAGUE > PART ≥ BARE/DOUBLE_VAGUE)
  is a contract; the numbers (0.95, 0.9, …) are not. Compare thresholds you
  choose, don't pin exact values.
- **Interpretation wording.** `interpretation` is for humans. Substring
  checks in tests may break on minor releases; don't parse it in production.
- **Grammar coverage.** New phrases may start parsing (never `ParseError` →
  different result for a previously-parsing phrase without a major bump).
- **Corpus contents, benches, docs.**

## The i18n seam

Language-specific word tables live in the Rust `Locale` struct
(`src/locale.rs`); the tokenizer and every grammar rule are language-neutral,
which the test suite proves with a miniature Spanish locale. v1.0 ships
English only. Exposing locale selection in the Python API
(`Config(locale="…")`) is planned as a **minor** post-1.0 release;
`interpretation` strings remain English until output localization is
designed separately.

## Deprecation process

Nothing is removed without: a deprecation note in the changelog, a
`DeprecationWarning` for at least one minor release, and removal only at the
next major.

## Pre-1.0 checklist

- [x] Grammar feature-complete for v1 scope (incl. clock times)
- [x] i18n seam in place and tested
- [x] Deterministic corpus suite (220+ cases) green on 3.10–3.13 × 3 OSes
- [x] Release pipeline exercised (wheels + sdist on tag)
- [ ] A soak period on 0.3.x — real users, no contract-breaking issue open
- [ ] Freeze: re-tag docs, bump to 1.0.0
