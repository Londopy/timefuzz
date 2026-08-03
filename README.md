# timefuzz

[![PyPI](https://img.shields.io/pypi/v/timefuzz)](https://pypi.org/project/timefuzz/)
[![Python versions](https://img.shields.io/pypi/pyversions/timefuzz)](https://pypi.org/project/timefuzz/)
[![CI](https://github.com/Londopy/timefuzz/actions/workflows/ci.yml/badge.svg)](https://github.com/Londopy/timefuzz/actions/workflows/ci.yml)
[![Release](https://github.com/Londopy/timefuzz/actions/workflows/release.yml/badge.svg)](https://github.com/Londopy/timefuzz/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/Londopy/timefuzz/blob/main/LICENSE)
[![Rust core](https://img.shields.io/badge/core-Rust-orange)](https://github.com/Londopy/timefuzz/tree/main/src)
[![parsed by timefuzz](https://img.shields.io/badge/parsed%20by-timefuzz-007ec6)](#using-timefuzz-add-a-badge)

> Fuzzy, natural-language time parsing that goes past `dateparser`.

`timefuzz` resolves human phrases like *"sometime next week"*, *"the Tuesday
after my birthday"*, or *"end of Q3"* into concrete datetimes or ranges —
**with a confidence score**, so your app knows when to ask the user to confirm.

Rust core, thin Python API. Ships as compiled wheels: `pip install timefuzz`,
no Rust toolchain required.

```python
import timefuzz as tf
from datetime import date, datetime

now = datetime(2026, 7, 12, 15, 30)

tf.parse("next friday", now=now)
# Instant(when=datetime(2026, 7, 17, 9, 0), confidence=0.95,
#         interpretation='next Friday, default 09:00')

tf.parse("sometime next week", now=now)
# Range(start=datetime(2026, 7, 13, 0, 0), end=datetime(2026, 7, 19, 23, 59, 59),
#       confidence=0.8, interpretation='the calendar week after this one')

tf.parse("the tuesday after my birthday", now=now,
         anchors={"my birthday": date(2026, 8, 3)})
# Instant(when=datetime(2026, 8, 4, 9, 0), confidence=0.9,
#         interpretation='first Tuesday strictly after 2026-08-03')

tf.parse("end of q3", now=now)
# Range(start=datetime(2026, 9, 1, 0, 0), end=datetime(2026, 9, 30, 23, 59, 59),
#       confidence=0.85, interpretation='last month of Q3')
```

## Why another date parser?

Existing parsers (`dateparser`, `parsedatetime`) handle *"in 3 days"* and
*"next Friday"* but fall over on:

- **Ranges & vagueness** — *"sometime next week"* is a *span*, not an instant.
- **Anchored relatives** — *"the Tuesday after my birthday"* needs a
  user-supplied anchor date.
- **Business calendars** — *"end of Q3"*, *"next business day"*,
  *"the 2nd Monday of March"*, *"10 business days after the invoice date"*.
- **Honest ambiguity** — *"next weekend"* said on a Wednesday has two
  defensible readings; timefuzz returns both instead of silently guessing.
- **Confidence** — callers want to know *how* fuzzy a result is, so they can
  confirm low-confidence parses with the user instead of silently guessing.

Every parse returns the resolved value **plus** a confidence score **plus**
the interpretation the parser chose, so schedulers, reminder apps, and chat
bots can decide when to double-check.

## Install

```
pip install timefuzz
```

Wheels are published for CPython 3.10–3.13 on Linux (manylinux), macOS
(x86-64 + arm64), and Windows (x86-64). An sdist falls back to building from
source (requires a Rust toolchain) if no wheel matches.

## What it returns

Every parse yields one of three shapes:

| Shape       | Fields | Example input |
|-------------|--------|----------------|
| `Instant`   | `when`, `confidence`, `interpretation` | `"next friday"` |
| `Range`     | `start`, `end` (inclusive), `confidence`, `interpretation` | `"sometime next week"` |
| `Ambiguous` | `candidates: list[Instant \| Range]`, `reason` | `"next weekend"` said midweek |

If nothing matches at all, `ParseError` is raised with the reason.
An unknown anchor (*"after my graduation"* with no `"my graduation"` anchor)
returns `Ambiguous` with an empty candidate list and an explanatory reason.
Candidates are ordered most-likely-first.

```python
match tf.parse(user_text):
    case tf.Instant(when=when, confidence=c) if c >= 0.8:
        schedule(when)
    case tf.Instant(when=when):
        confirm_with_user(when)          # low confidence -> ask first
    case tf.Range(start=s, end=e):
        offer_slot_picker(s, e)
    case tf.Ambiguous(candidates=cands, reason=why):
        disambiguate(cands, why)
```

## API

```python
def parse(
    text: str,
    now: datetime | None = None,        # reference moment (default: datetime.now())
    anchors: dict[str, date] | None = None,
    config: Config | None = None,
) -> Instant | Range | Ambiguous: ...

@dataclass(frozen=True)
class Config:
    default_time: time = time(9, 0)     # time attached to date-only results
    week_start: Weekday = Weekday.MON   # or Weekday.SUN
    next_skips_today: bool = True       # "next friday" said on a Friday
    tz: tzinfo | None = None            # naive by default; tz-aware opt-in
    holidays: Callable[[date], bool] | None = None   # business-day hook
```

## What the grammar understands (v0.4)

- **Relative offsets:** `in 3 days`, `2 weeks ago`, `3 days from now`,
  `in a week`, `tomorrow`, `yesterday`, `today`, `now`
- **Bare durations** (the `in` is optional): `30 minutes`, `2 hrs`, `3 days`,
  `half an hour`, `half a day`, `a couple hours`, `a few days`,
  `back in a couple hours`
- **Period spans:** `next week`, `this month`, `last quarter`, `next year`,
  bare month names (`august`)
- **Weekday navigation:** `next friday`, `this tuesday`, `last monday`,
  bare `friday`, `friday next week`, `monday last week`
- **Weekends:** `this weekend`, `next weekend`, `last weekend`,
  `sometime next weekend`, `the weekend after the wedding`
- **Ordinal-in-period:** `2nd monday of march`, `2nd monday of march 2027`,
  `last friday of october`, `first monday of next month`
- **Anchored:** `the tuesday after my birthday`, `2nd tuesday after my
  birthday`, `the day before the wedding`, `2 weeks after my birthday`,
  `the week after my birthday`, `3 business days after the invoice date`,
  bare `on my birthday` (anchors supplied by you)
- **Clock times:** `next friday at 3pm`, `tomorrow at 7:30`, `friday at
  midnight`, bare `3pm` / `15:45` / `noon` (next occurrence)
- **Vague spans:** `sometime next week`, `sometime in august`, `early march`,
  `mid q3`, `mid month`, `late next month`, `sometime early next month`
- **Business calendar:** `next business day`, `in 3 business days`,
  `5 business days ago`, `end of q3`, `start of q4`, `end of month`,
  `end of next month`, `first business day of next month`,
  `last business day of the month`, `eow`/`eom`/`eoq`/`eoy`

See [docs/grammar_reference.md](https://github.com/Londopy/timefuzz/blob/main/docs/grammar_reference.md) for the complete
rule catalogue and [docs/cookbook.md](https://github.com/Londopy/timefuzz/blob/main/docs/cookbook.md) for recipes.

## Conventions (the fine print)

These are deliberate, documented choices — see the config knobs above:

- **"next Friday" when today is Friday** is culture-dependent. Default:
  skip today (`next_skips_today=True`), i.e. you get the Friday seven days out.
- **Date-only results get `default_time`** (09:00 by default); an explicit
  trailing time (`… at 3pm`) overrides it.
- **Arithmetic offsets keep the clock:** `in 3 days` = now + 72h;
  `tomorrow` = tomorrow at `default_time`.
- **The `in` is optional.** `30 minutes` resolves exactly like
  `in 30 minutes`. Approximate amounts (`a couple hours`) resolve too, but at
  0.7 so you confirm rather than schedule silently, and `half a month` stays
  a `ParseError` because there is no exact half of a calendar month.
- **Ranges are inclusive**, `00:00:00` through `23:59:59` of the last day.
- **Weekends are Sat–Sun**, whatever `week_start` says.
- **Naive by default.** Set `Config(tz=...)` to get tz-aware results;
  math is wall-clock, so "tomorrow 09:00" is 09:00 across a DST jump.
- **Month names and quarters assume the current cycle** and roll forward if
  already past (the interpretation string says so).
- **Genuinely contested phrases return `Ambiguous`** rather than a guess:
  `next weekend` midweek, `sunday` said on a Sunday, `july` said during July.
- **Confidence is deterministic**, not learned — the same phrase always gets
  the same score, offsets past ~10 years are trusted slightly less, and
  stacked hedges (`sometime early …`) cap lower.
  See [docs/confidence.md](https://github.com/Londopy/timefuzz/blob/main/docs/confidence.md).

## Development

```
# build + test
pip install maturin
maturin develop --release
pytest

# rust checks
cargo clippy --all-targets
cargo fmt --check
cargo bench          # informational: full-corpus, single-phrase, tokenizer-only
```

The test suite is **corpus-driven**: `tests/corpus*.jsonl` map phrases (plus a
fixed reference `now`) to expected outputs. Adding a phrase = adding a line.

## Roadmap

- **v0.2** — ✅ richer anchored phrases + business-calendar rules
- **v0.3** — ✅ confidence-model refinement, more `Ambiguous` candidates
- **v0.4** — ✅ corpus expansion, benches, cookbook growth,
  ✅ bare durations (`30 minutes`, `half an hour`, `a couple hours`)
- **v1.0** — ✅ i18n-ready grammar structure (`Locale` seam, English-only),
  ✅ clock-time support, ✅ written [stability policy](https://github.com/Londopy/timefuzz/blob/main/docs/stability.md);
  remaining: a soak period on 0.4.x, then the freeze

See [CHANGELOG.md](https://github.com/Londopy/timefuzz/blob/main/CHANGELOG.md) for details.

## Using timefuzz? Add a badge

[![parsed by timefuzz](https://img.shields.io/badge/parsed%20by-timefuzz-007ec6)](https://github.com/Londopy/timefuzz)

```markdown
[![parsed by timefuzz](https://img.shields.io/badge/parsed%20by-timefuzz-007ec6)](https://github.com/Londopy/timefuzz)
```

Or as HTML:

```html
<a href="https://github.com/Londopy/timefuzz"><img alt="parsed by timefuzz"
   src="https://img.shields.io/badge/parsed%20by-timefuzz-007ec6"></a>
```

If "capability" reads better than "credit" in your README, this wording says
what your project does rather than crediting this one:

[![time parsing: timefuzz](https://img.shields.io/badge/time%20parsing-timefuzz-007ec6)](https://github.com/Londopy/timefuzz)

```markdown
[![time parsing: timefuzz](https://img.shields.io/badge/time%20parsing-timefuzz-007ec6)](https://github.com/Londopy/timefuzz)
```

Rendered by [shields.io](https://shields.io) — nothing to install, and no call
back to this project. Append `?style=flat-square` or `?style=for-the-badge` if
that suits your README better. If you reword it, mind shields' escaping: spaces
are `%20` (or `_`), and a literal hyphen has to be doubled (`--`).

## License

MIT
