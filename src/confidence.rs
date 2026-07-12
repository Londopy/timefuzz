//! Deterministic confidence model.
//!
//! Scores are fixed per rule family and adjusted by explainable modifiers —
//! no learned weights. See docs/confidence.md for the rationale.

/// Unambiguous, fully-specified phrases ("now").
pub const CERTAIN: f32 = 1.0;
/// Exact grammatical matches with one natural reading ("in 3 days", "next friday").
pub const EXACT: f32 = 0.95;
/// Strong matches with a documented convention choice ("2 weeks after <anchor>").
pub const STRONG: f32 = 0.9;
/// Calendar-derived results that assume the current cycle ("2nd monday of march").
pub const CALENDAR: f32 = 0.85;
/// Vague-span words cap here ("sometime next week").
pub const VAGUE: f32 = 0.8;
/// Sub-period fuzz ("early march") and year-assumed month names.
pub const PART: f32 = 0.75;
/// Bare, convention-heavy phrases ("friday").
pub const BARE: f32 = 0.7;
/// Doubly-hedged phrases ("sometime early next month").
pub const DOUBLE_VAGUE: f32 = 0.7;

/// Offsets further than ~10 years out are slightly less trustworthy.
pub fn horizon_penalty(c: f32, days_from_now: i64) -> f32 {
    if days_from_now.abs() > 3650 {
        clamp(c - 0.05)
    } else {
        c
    }
}

/// Penalty applied to each candidate when several interpretations survive.
pub fn ambiguity_penalty(c: f32, n_candidates: usize) -> f32 {
    clamp(c - 0.1 * (n_candidates.saturating_sub(1)) as f32)
}

pub fn clamp(c: f32) -> f32 {
    c.clamp(0.05, 1.0)
}
