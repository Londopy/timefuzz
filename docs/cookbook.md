# Cookbook

Practical recipes for schedulers, reminder apps, and chat bots.

## Confirm low-confidence parses (the whole point)

```python
import timefuzz as tf

CONFIRM_BELOW = 0.8

def resolve_or_ask(text: str, **kw):
    try:
        r = tf.parse(text, **kw)
    except tf.ParseError as e:
        return ask_user(f"I couldn't read that as a time ({e}). Try 'next friday'?")

    match r:
        case tf.Ambiguous(candidates=[], reason=why):
            return ask_user(f"I need more info: {why}")
        case tf.Ambiguous(candidates=cands):
            return ask_user_to_pick(cands)  # each has .interpretation
        case tf.Instant(confidence=c) | tf.Range(confidence=c) if c < CONFIRM_BELOW:
            return ask_user(f"Did you mean {r.interpretation}?")
        case _:
            return r
```

## Presenting `Ambiguous` candidates as buttons

Candidates come ordered most-likely-first with per-candidate confidence, so a
two-button confirm is one loop:

```python
r = tf.parse("next weekend")   # said on a Wednesday -> Ambiguous
if isinstance(r, tf.Ambiguous):
    for i, c in enumerate(r.candidates):
        label = f"{c.start:%a %b %d}" if isinstance(c, tf.Range) else f"{c.when:%a %b %d}"
        add_button(text=f"{label} — {c.interpretation}", default=(i == 0))
```

The built-in ambiguous phrases are: a bare weekday said on that weekday
("sunday" on a Sunday), "next weekend" said midweek, a bare month said during
that month ("july" in July), and unknown anchors (empty candidate list).

## Discord/chat bot: remind me

```python
from datetime import datetime
import timefuzz as tf

async def on_remind(msg, when_text: str):
    r = tf.parse(when_text, now=datetime.now())
    if isinstance(r, tf.Range):
        # "sometime next week" -> pick the start, tell the user the span
        await msg.reply(f"Okay — {r.interpretation}. I'll ping you {r.start:%a %b %d}.")
        schedule(msg, r.start)
    elif isinstance(r, tf.Instant):
        schedule(msg, r.when)
```

## Per-user anchors

```python
anchors = {
    "my birthday": user.birthday,
    "payday": user.next_payday,
    "the wedding": events["wedding"].date,
}
tf.parse("the friday before the wedding", anchors=anchors)
tf.parse("the week after the wedding", anchors=anchors)     # -> Range
tf.parse("2 business days before payday", anchors=anchors)  # -> Instant
tf.parse("on payday", anchors=anchors)                      # bare anchor works too
```

Unknown anchors come back as `Ambiguous(candidates=[], reason="unknown anchor
'…'")` — surface the reason; it tells the user exactly what name to register.

## Invoice due dates & SLAs

Business-day arithmetic composes with anchors and your holiday calendar:

```python
import holidays  # pip install holidays
us = holidays.US()
cfg = tf.Config(holidays=lambda d: d in us)

anchors = {"the invoice date": invoice.issued_on}
due = tf.parse("10 business days after the invoice date", anchors=anchors, config=cfg)

followup = tf.parse("first business day of next month", config=cfg)
standup_cutoff = tf.parse("eod" if False else "eow", config=cfg)  # eow/eom/eoq/eoy shorthand
```

Any `date -> bool` callable works — a set lookup, a database query, whatever.

## Weekend planning

```python
r = tf.parse("next weekend")
if isinstance(r, tf.Ambiguous):
    # said midweek: offer both weekends
    show_choices(r.candidates)
else:
    book_campsite(r.start, r.end)   # Sat 00:00 .. Sun 23:59:59
```

## Exact meeting times from fuzzy phrases

Trailing clock times ride along with any date rule, so one parse handles
"put it on the calendar":

```python
r = tf.parse("first business day of next month at 8am")
r.when                       # e.g. datetime(2026, 8, 3, 8, 0)

r = tf.parse("3pm")          # bare time -> next occurrence
r.interpretation             # "15:00 tomorrow (that time has already passed today)"
```

`Ambiguous` candidates carry the time too — "sunday at 3pm" said on a Sunday
gives you both Sundays at 15:00, ready for a two-button confirm.

## Timezone-aware scheduling

```python
from zoneinfo import ZoneInfo

cfg = tf.Config(tz=ZoneInfo("America/New_York"))
r = tf.parse("tomorrow", config=cfg)
r.when          # tz-aware datetime, 09:00 New York wall time
```

Math is wall-clock: "tomorrow 09:00" stays 09:00 even across a DST jump.
Leave `tz` unset for naive datetimes (the default).

## Sunday-start weeks + a different default hour

```python
from datetime import time

cfg = tf.Config(week_start=tf.Weekday.SUN, default_time=time(8, 0))
tf.parse("this friday", config=cfg)   # Friday of the Sun-Sat week, 08:00
```

## Trusting far-future input less

Relative offsets beyond ~10 years take a small confidence penalty and are
tagged "(distant horizon)" — useful for catching typos like "in 300 days"
vs. "in 300 years":

```python
r = tf.parse("in 30 years")
if "distant horizon" in r.interpretation:
    confirm_with_user(r)
```

## Feeding a slot picker from a vague answer

```python
r = tf.parse("sometime next week")
if isinstance(r, tf.Range):
    slots = generate_slots(r.start, r.end, granularity="1h")
    show_picker(slots, hint=r.interpretation)
```

Doubly-hedged phrases ("sometime early next month") resolve to the same kind
of range with a lower score (0.7) — treat them as slot-picker input, not as
commitments.

## Deterministic tests in your own app

Pin `now` (and anchors) exactly like timefuzz's own corpus suite does:

```python
FIXED_NOW = datetime(2026, 7, 12, 15, 30)

def test_my_bot_understands_fuzzy_times():
    r = tf.parse("early next month", now=FIXED_NOW)
    assert (r.start.month, r.start.day) == (8, 1)
```
