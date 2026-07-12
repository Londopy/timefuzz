//! Rule-based grammar over the token stream (not ML).
//!
//! Each rule family inspects the token stream and produces a `Resolution`,
//! signals ambiguity, flags an invalid phrase, or passes. The dispatcher
//! collects all matches; several surviving interpretations => `Ambiguous`.

pub mod anchored;
pub mod business;
pub mod ext;
pub mod ordinal;
pub mod relative;
pub mod vague;
#[path = "weekday_impl.rs"]
pub mod weekday;

use crate::confidence;
use crate::config::Ctx;
use crate::tokenize::{Kw, Tok, Unit};
use crate::types::{Outcome, Resolution, Value};
use chrono::{Datelike, NaiveDate};

pub enum RuleResult {
    /// Rule does not apply.
    None,
    /// One interpretation.
    One(Resolution),
    /// The rule applies but the phrase is inherently ambiguous.
    Ambiguous(Vec<Resolution>, String),
    /// The phrase names something that doesn't exist ("5th monday of march").
    Invalid(String),
}

type Rule = fn(&[Tok], &Ctx) -> RuleResult;

const RULES: [Rule; 7] = [
    relative::try_match,
    weekday::try_match,
    ordinal::try_match,
    anchored::try_match,
    vague::try_match,
    business::try_match,
    ext::try_match,
];

/// Confidence gap for the top candidate to win outright.
const DOMINANCE_GAP: f32 = 0.15;

pub fn resolve(text: &str, tokens: &[Tok], ctx: &Ctx) -> Outcome {
    let mut candidates: Vec<Resolution> = Vec::new();
    let mut ambiguities: Vec<(Vec<Resolution>, String)> = Vec::new();
    let mut invalid: Option<String> = None;

    for rule in RULES {
        match rule(tokens, ctx) {
            RuleResult::None => {}
            RuleResult::One(r) => {
                if !candidates.iter().any(|c| c.value == r.value) {
                    candidates.push(r);
                }
            }
            RuleResult::Ambiguous(c, reason) => ambiguities.push((c, reason)),
            RuleResult::Invalid(reason) => invalid = Some(reason),
        }
    }

    match candidates.len() {
        0 => {
            if let Some((c, reason)) = ambiguities.into_iter().next() {
                return Outcome::Ambiguous {
                    candidates: c,
                    reason,
                };
            }
            if let Some(reason) = invalid {
                return Outcome::NoParse { reason };
            }
            Outcome::NoParse {
                reason: format!("no grammar rule matched {text:?}"),
            }
        }
        1 => Outcome::Resolved(candidates.into_iter().next().unwrap()),
        n => {
            candidates.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
            if candidates[0].confidence - candidates[1].confidence >= DOMINANCE_GAP {
                Outcome::Resolved(candidates.into_iter().next().unwrap())
            } else {
                let penalized: Vec<Resolution> = candidates
                    .iter()
                    .map(|r| Resolution {
                        value: r.value.clone(),
                        confidence: confidence::ambiguity_penalty(r.confidence, n),
                        interpretation: r.interpretation.clone(),
                    })
                    .collect();
                Outcome::Ambiguous {
                    candidates: penalized,
                    reason: format!("{n} grammar rules matched with comparable confidence"),
                }
            }
        }
    }
}

// ---- Shared helpers ----

pub struct Period {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub description: String,
    /// Confidence appropriate for naming this period exactly.
    pub base_confidence: f32,
}

/// Recognize a period expression: `next/this/last week|month|quarter|year`,
/// a bare month name ("march"), or a bare quarter ("q3").
pub fn period_of(tokens: &[Tok], ctx: &Ctx) -> Option<Period> {
    use crate::dates::*;
    let today = ctx.today();

    let (offset, unit): (i32, &Unit) = match tokens {
        [Tok::Kw(Kw::Next), Tok::Unit(u)] => (1, u),
        [Tok::Kw(Kw::This), Tok::Unit(u)] => (0, u),
        [Tok::Kw(Kw::Last), Tok::Unit(u)] => (-1, u),
        // a bare unit means the current period ("mid month", "eom")
        [Tok::Unit(u)] => (0, u),
        [Tok::Month(m)] => {
            let (mut start, mut end) = month_range(today.year(), *m);
            let mut desc = format!("{} of the current year", month_name(*m));
            if end < today {
                let (s2, e2) = month_range(today.year() + 1, *m);
                start = s2;
                end = e2;
                desc = format!(
                    "{} {} (this year's {} has passed)",
                    month_name(*m),
                    today.year() + 1,
                    month_name(*m)
                );
            }
            return Some(Period {
                start,
                end,
                description: desc,
                base_confidence: confidence::PART,
            });
        }
        [Tok::Quarter(q)] => {
            let (mut start, mut end) = quarter_range(today.year(), *q);
            let mut desc = format!("Q{q} of the current year");
            if end < today {
                let (s2, e2) = quarter_range(today.year() + 1, *q);
                start = s2;
                end = e2;
                desc = format!(
                    "Q{} {} (this year's Q{} has passed)",
                    q,
                    today.year() + 1,
                    q
                );
            }
            return Some(Period {
                start,
                end,
                description: desc,
                base_confidence: confidence::CALENDAR,
            });
        }
        _ => return None,
    };

    let (start, end, what) = match unit {
        Unit::Week => {
            let (s, _) = week_range(today, ctx.cfg.week_start);
            let s = s + chrono::Duration::days(7 * offset as i64);
            (s, s + chrono::Duration::days(6), "calendar week")
        }
        Unit::Month => {
            let anchor = add_months(today.with_day(1).unwrap(), offset);
            let (s, e) = month_range(anchor.year(), anchor.month());
            (s, e, "calendar month")
        }
        Unit::Quarter => {
            let anchor = add_months(today.with_day(1).unwrap(), 3 * offset);
            let q = quarter_of_month(anchor.month());
            let (s, e) = quarter_range(anchor.year(), q);
            (s, e, "calendar quarter")
        }
        Unit::Year => {
            let (s, e) = year_range(today.year() + offset);
            (s, e, "calendar year")
        }
        Unit::Weekend => {
            let (ws, _) = week_range(today, ctx.cfg.week_start);
            let sat = next_weekday(
                ws + chrono::Duration::days(7 * offset as i64),
                chrono::Weekday::Sat,
                false,
            );
            (sat, sat + chrono::Duration::days(1), "weekend (Sat-Sun)")
        }
        _ => return None,
    };

    let description = match offset {
        1 => format!("the {what} after this one"),
        -1 => format!("the {what} before this one"),
        _ => format!("the current {what}"),
    };

    Some(Period {
        start,
        end,
        description,
        base_confidence: confidence::STRONG,
    })
}

/// Build a Range resolution over whole days.
pub fn day_range(
    ctx: &Ctx,
    start: NaiveDate,
    end: NaiveDate,
    confidence: f32,
    interpretation: String,
) -> Resolution {
    let (s, e) = ctx.day_span(start, end);
    Resolution {
        value: Value::Range { start: s, end: e },
        confidence,
        interpretation,
    }
}

/// Build an Instant resolution at the configured default time.
pub fn day_instant(ctx: &Ctx, d: NaiveDate, confidence: f32, interpretation: String) -> Resolution {
    Resolution {
        value: Value::Instant {
            when: ctx.at_default(d),
        },
        confidence,
        interpretation,
    }
}
