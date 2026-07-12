//! Vague spans: "sometime next week", "early march", "late next month",
//! "mid q3".

use super::{day_range, period_of, RuleResult};
use crate::confidence;
use crate::config::Ctx;
use crate::tokenize::{Kw, Tok};
use chrono::{Duration, NaiveDate};

/// Split an inclusive date span into thirds and return the requested one.
fn third_of(start: NaiveDate, end: NaiveDate, which: Kw) -> (NaiveDate, NaiveDate, &'static str) {
    let len = (end - start).num_days() + 1;
    let a = len / 3;
    let b = 2 * len / 3;
    match which {
        Kw::Early => (start, start + Duration::days(a.max(1) - 1), "early part of"),
        Kw::Mid => (
            start + Duration::days(a),
            start + Duration::days(b.max(a + 1) - 1),
            "middle part of",
        ),
        _ => (start + Duration::days(b), end, "late part of"),
    }
}

pub fn try_match(tokens: &[Tok], ctx: &Ctx) -> RuleResult {
    match tokens {
        // "sometime <period>" / "sometime in <period>"
        [Tok::Kw(Kw::Sometime), rest @ ..] => {
            let rest = match rest {
                [Tok::Kw(Kw::In), r @ ..] => r,
                r => r,
            };
            match period_of(rest, ctx) {
                Some(p) => {
                    let conf = confidence::VAGUE.min(p.base_confidence);
                    RuleResult::One(day_range(ctx, p.start, p.end, conf, p.description))
                }
                None => RuleResult::None,
            }
        }
        // "early/mid/late <period>" (optional "in": "late in the year")
        [Tok::Kw(k @ (Kw::Early | Kw::Mid | Kw::Late)), rest @ ..] => {
            let rest = match rest {
                [Tok::Kw(Kw::In), r @ ..] => r,
                r => r,
            };
            match period_of(rest, ctx) {
                Some(p) => {
                    let (s, e, label) = third_of(p.start, p.end, *k);
                    RuleResult::One(day_range(
                        ctx,
                        s,
                        e,
                        confidence::PART.min(p.base_confidence),
                        format!("the {label} {}", p.description),
                    ))
                }
                None => RuleResult::None,
            }
        }
        _ => RuleResult::None,
    }
}
