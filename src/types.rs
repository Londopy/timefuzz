//! Result types: Instant / Range / Ambiguous.

use chrono::NaiveDateTime;
use serde::Serialize;

/// A resolved value: either a single point in time or an inclusive span.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Instant {
        when: NaiveDateTime,
    },
    Range {
        start: NaiveDateTime,
        end: NaiveDateTime,
    },
}

/// A single interpretation of the input phrase.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolution {
    pub value: Value,
    pub confidence: f32,
    pub interpretation: String,
}

/// Overall outcome of a parse.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Resolved(Resolution),
    Ambiguous {
        candidates: Vec<Resolution>,
        reason: String,
    },
    NoParse {
        reason: String,
    },
}

// ---- JSON wire format (consumed by the Python layer) ----

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireResolution {
    Instant {
        when: String,
        confidence: f32,
        interpretation: String,
    },
    Range {
        start: String,
        end: String,
        confidence: f32,
        interpretation: String,
    },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireOutcome {
    Instant {
        when: String,
        confidence: f32,
        interpretation: String,
    },
    Range {
        start: String,
        end: String,
        confidence: f32,
        interpretation: String,
    },
    Ambiguous {
        candidates: Vec<WireResolution>,
        reason: String,
    },
    NoParse {
        reason: String,
    },
}

const ISO: &str = "%Y-%m-%dT%H:%M:%S";

fn wire_res(r: &Resolution) -> WireResolution {
    match &r.value {
        Value::Instant { when } => WireResolution::Instant {
            when: when.format(ISO).to_string(),
            confidence: r.confidence,
            interpretation: r.interpretation.clone(),
        },
        Value::Range { start, end } => WireResolution::Range {
            start: start.format(ISO).to_string(),
            end: end.format(ISO).to_string(),
            confidence: r.confidence,
            interpretation: r.interpretation.clone(),
        },
    }
}

impl Outcome {
    pub fn to_json(&self) -> String {
        let wire = match self {
            Outcome::Resolved(r) => match wire_res(r) {
                WireResolution::Instant {
                    when,
                    confidence,
                    interpretation,
                } => WireOutcome::Instant {
                    when,
                    confidence,
                    interpretation,
                },
                WireResolution::Range {
                    start,
                    end,
                    confidence,
                    interpretation,
                } => WireOutcome::Range {
                    start,
                    end,
                    confidence,
                    interpretation,
                },
            },
            Outcome::Ambiguous { candidates, reason } => WireOutcome::Ambiguous {
                candidates: candidates.iter().map(wire_res).collect(),
                reason: reason.clone(),
            },
            Outcome::NoParse { reason } => WireOutcome::NoParse {
                reason: reason.clone(),
            },
        };
        serde_json::to_string(&wire).expect("outcome serialization cannot fail")
    }
}
