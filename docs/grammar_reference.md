# Grammar reference

The parser is a small **rule-based grammar** over a typed token stream — no ML.
The tokenizer classifies words (numbers, weekdays, months, units, keywords,
ordinals, quarters, weekends, anchors); each rule family then matches the whole
token stream against its patterns. Articles (`the`, `on`, `at`, `for`) are
dropped, punctuation is stripped, matching is case-insensitive, and common
abbreviations work (`fri`, `tues`, `sept`, `q3`, `wk`, `hrs`, …). Business
shorthand expands inline: `eow`, `eom`, `eoq`, `eoy` read as "end of
week/month/quarter/year".

If several rule families produce *different* results with comparable
confidence, you get `Ambiguous` with all candidates. If a phrase is understood
but names something impossible ("5th friday of february"), `ParseError`
explains why.

## 1. Relative offsets → `Instant`

| Pattern | Example | Result |
|---|---|---|
| `now` | `now` | the reference moment, confidence 1.0 |
| `today` / `tomorrow` / `yesterday` | `tomorrow` | that date at `default_time` |
| `in N <unit>` | `in 3 days`, `in a week`, `in 45 minutes` | now + N·unit, clock kept |
| `N <unit> from now` | `3 days from now` | now + N·unit |
| `N <unit> ago` | `2 weeks ago` | now − N·unit |

Units: `minutes`, `hours`, `days`, `weeks`, `months`, `quarters`, `years`
(and their abbreviations). Number words `one`–`twelve` and `a`/`an` work.
Month/quarter/year offsets use calendar arithmetic with end-clamping:
`in 1 month` from Jan 31 gives the last day of February. Offsets further than
~10 years out take a small confidence penalty and say "(distant horizon)".

## 2. Period spans → `Range`

| Pattern | Example | Result |
|---|---|---|
| `next <period>` | `next week` | the following calendar period |
| `this <period>` | `this month` | the current calendar period |
| `last <period>` | `last quarter` | the preceding calendar period |
| bare month name | `august` | that month, assuming the current cycle |

Periods: `week` (per `week_start`), `month`, `quarter`, `year`, `weekend`
(Sat–Sun; see §8). Ranges are inclusive: first day 00:00:00 → last day
23:59:59. A bare month name said *during* that month ("july" in July) returns
`Ambiguous`: current month vs. the same month next year.

## 3. Weekday navigation → `Instant`

| Pattern | Example | Convention |
|---|---|---|
| `next <weekday>` | `next friday` | first such weekday strictly after today; if today *is* that weekday, `next_skips_today` decides |
| `this <weekday>` | `this tuesday` | that weekday of the *current* week (may be in the past — confidence drops and the interpretation says so) |
| `last <weekday>` | `last monday` | most recent, strictly before today |
| bare `<weekday>` | `friday` | the upcoming one, confidence 0.7; said on that same weekday it returns `Ambiguous` (today vs. next week) |
| `<weekday> next/this/last week` | `friday next week` | that weekday of the named week — never ambiguous |

## 4. Ordinal-in-period → `Instant`

| Pattern | Example |
|---|---|
| `<nth> <weekday> of <month> [year]` | `2nd monday of march`, `2nd monday of march 2027` |
| `last <weekday> of <month> [year]` | `last friday of october` |
| `<nth> <weekday> of next/this month` | `first monday of next month` |

Without an explicit year, a date that already passed rolls to next year (the
interpretation notes the assumption). A nonexistent ordinal raises
`ParseError` ("February 2026 has no 5th Friday").

## 5. Anchored relatives

Anchors are named dates you pass in: `anchors={"my birthday": date(2026, 8, 3)}`.
Matching is case-insensitive substring matching on the anchor name; when
several anchor names overlap, the longest match wins.

| Pattern | Example | Result |
|---|---|---|
| bare `<anchor>` | `on my birthday` | `Instant`: the anchor date itself |
| `<weekday> after/before <anchor>` | `the tuesday after my birthday` | `Instant` (strictly after/before) |
| `<nth> <weekday> after <anchor>` | `2nd tuesday after my birthday` | `Instant` |
| `the day after/before <anchor>` | `the day before the wedding` | `Instant` |
| `N <unit> after/before <anchor>` | `2 weeks after my birthday` | `Instant` |
| `N business days after/before <anchor>` | `3 business days after the wedding` | `Instant`; skips weekends + holidays |
| `the week/month after/before <anchor>` | `the week after my birthday` | `Range`: the calendar week/month adjacent to the one containing the anchor |
| `the weekend after <anchor>` | `the weekend after the wedding` | `Range`: the first Sat–Sun strictly after |

A phrase with `after`/`before` and an *unregistered* anchor returns
`Ambiguous` with an empty candidate list and a reason telling you which
anchor to supply.

## 6. Vague spans → `Range`

| Pattern | Example | Result |
|---|---|---|
| `sometime [in] <period>` | `sometime next week`, `sometime in august` | the whole period, confidence ≤ 0.8 |
| `early <period>` | `early march` | first third of the period |
| `mid <period>` | `mid q3`, `mid month` | middle third |
| `late <period>` | `late next month` | final third |
| `sometime early/mid/late <period>` | `sometime early next month` | the third, **doubly hedged** — confidence capped at 0.7 |

`<period>` here also accepts bare month names, quarters (`march`, `q3`), bare
units meaning the current period (`mid month`), and weekends
(`sometime next weekend`). Thirds are computed by day count and tile the
period exactly.

## 7. Business calendar

| Pattern | Example | Result |
|---|---|---|
| `next business day` | — | `Instant`; skips Sat/Sun + `holidays` |
| `in N business days` / `N business days from now` | `in 3 business days` | `Instant` |
| `N business days ago` | `5 business days ago` | `Instant`; walks backwards |
| `end of q<N>` | `end of q3` | `Range`: the **last month** of the quarter |
| `start of q<N>` | `start of q4` | `Range`: the first month of the quarter |
| `end/start of [next/this/last] <period>` | `end of next month` | `Instant`: last/first day at `default_time` |
| `eow` / `eom` / `eoq` / `eoy` | `eom` | shorthand for `end of week/month/quarter/year` |
| `<nth> business day of <period>` | `first business day of next month` | `Instant`; `ParseError` if the period is too short |
| `last business day of <period>` | `last business day of the month` | `Instant` |

`working day(s)` is accepted as a synonym for `business day(s)`.

## 8. Clock times

A **trailing** clock time overrides the default time of day for any
date-producing rule (including `Ambiguous` candidates); ranges ignore it.

| Pattern | Example | Result |
|---|---|---|
| `<phrase> at <time>` | `next friday at 3pm`, `end of month at 5pm` | that rule's date, at the given time |
| bare `<time>` | `3pm`, `15:45`, `noon`, `midnight` | next occurrence: today, or tomorrow if already passed |

Formats: `3pm`, `3 pm`, `3:30pm`, `7:30`, `15:45`, `noon`, `midnight`.
`12pm` is noon, `12am` is midnight. Invalid times (`13pm`, `25:00`, `7:99`)
don't tokenize as times. `in 3 days at 5pm` applies the time to day-granular
arithmetic; `in 3 hours at 5pm` keeps hour arithmetic's own clock. Explicit
times never change a rule's confidence.

## 9. Weekends → `Range`

A weekend is Saturday 00:00:00 through Sunday 23:59:59 (the Sat–Sun pair
whose Saturday falls in the referenced week).

| Pattern | Example | Result |
|---|---|---|
| `this weekend` | — | the current week's Sat–Sun |
| `last weekend` | — | the previous week's Sat–Sun |
| `next weekend` | — | **said Sat/Sun:** next week's, unambiguous. **Said midweek:** `Ambiguous` — the coming weekend vs. the one after (a genuinely contested phrase in English) |
| `the weekend after <anchor>` | see §5 | first Sat–Sun strictly after the anchor |

## Extending

Grammar rules live in `src/grammar/` (one file per family; v0.2 extensions in
`ext.rs`) and are registered in the `RULES` array in `src/grammar/mod.rs`.

Since v0.3 the grammar is **i18n-ready**: every language-specific word table
(weekdays, months, units, keywords, number words, ordinals, fillers,
shorthand, am/pm markers, named times) lives in the `Locale` struct in
`src/locale.rs`, with English as the built-in default. Adding a language
means writing one more `Locale` table — the tokenizer and grammar rules stay
untouched, which the test suite proves with a miniature Spanish locale.
Result `interpretation` strings remain English for now; localizing output is
a separate, later concern.
