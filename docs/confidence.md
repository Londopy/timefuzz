# The confidence model

Deterministic and explainable — **not** a black box. The same phrase with the
same config always produces the same score. There are no learned weights.

## Base scores per rule family

| Score | Constant | Applies to | Examples |
|---|---|---|---|
| 1.00 | `CERTAIN` | fully specified, one reading | `now` |
| 0.95 | `EXACT` | exact grammatical matches | `in 3 days`, `next friday`, `today`, `next business day`, `the day after <anchor>`, `friday next week`, a bare registered anchor |
| 0.90 | `STRONG` | strong matches that rest on a documented convention | `next sunday` said on a Sunday, `<weekday> after <anchor>`, `next week`, `in 3 business days`, `this weekend`, `the week after <anchor>`, ordinals with an explicit year |
| 0.85 | `CALENDAR` | calendar-derived results that assume the current cycle | `2nd monday of march`, `end of q3`, `eom`, `first business day of next month` |
| 0.80 | `VAGUE` | vague-span words cap here | `sometime next week` |
| 0.75 | `PART` | sub-period fuzz; month names that assume a year | `early march`, `mid month`, `this friday` when already past |
| 0.70 | `BARE` | bare convention-heavy phrases | `friday`, `august` |
| 0.70 | `DOUBLE_VAGUE` | doubly-hedged phrases | `sometime early next month` |

## Modifiers

- **Vagueness caps.** A vague phrase can never exceed its cap even when the
  underlying period is exact: `sometime next week` = min(0.8, 0.9) = 0.8.
  Stacked hedges cap lower: `sometime early next month` = 0.7, and the
  interpretation says "(doubly hedged)".
- **Passed-date penalty.** `this friday` / `tuesday this week` when that day
  already passed this week drops to 0.75 and the interpretation says
  "(already passed this week)".
- **Horizon penalty.** Relative offsets further than ~10 years (3650 days)
  from `now` lose 0.05 and the interpretation appends "(distant horizon)":
  `in 15 years` scores 0.90 instead of 0.95.
- **Ambiguity penalty.** When several rule families survive with comparable
  confidence (within 0.15 of each other), the parse returns `Ambiguous` and
  every candidate is penalized by 0.1 per extra candidate.
- **Dominance.** If one interpretation beats all others by ≥ 0.15, it wins
  outright and no `Ambiguous` is returned.
- Scores are clamped to [0.05, 1.0]; the suite property-tests that every
  returned confidence is in [0, 1].

## Built-in `Ambiguous` cases

Some phrases are ambiguous *within* a single rule and return fixed candidate
scores rather than going through the dispatcher:

| Phrase | Candidates |
|---|---|
| bare weekday on that weekday (`sunday` said on a Sunday) | today 0.70 vs. a week out 0.60 |
| `next weekend` said midweek | the coming weekend 0.65 vs. the one after 0.55 |
| bare month during that month (`july` said in July) | current month 0.60 vs. next year 0.55 |
| unknown anchor (`after my graduation`) | no candidates; the reason names the missing anchor |

Candidates are always ordered most-likely-first, so
`result.candidates[0]` is the parser's best guess.

## Suggested thresholds for callers

| Confidence | Suggested handling |
|---|---|
| ≥ 0.9 | act on it |
| 0.75 – 0.9 | act, but surface the interpretation string |
| < 0.75 | confirm with the user |
| `Ambiguous` | always confirm — present the candidates |

These are conventions, not contracts: the exact numbers may be tuned in 0.x
releases, but the *ordering* of rule families is stable.
