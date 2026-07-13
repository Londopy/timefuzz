//! Informational benchmarks of the grammar hot path (`cargo bench`).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use timefuzz::config::Cfg;
use timefuzz::locale::EN;
use timefuzz::{parse_str, tokenize};

const PHRASES: &[&str] = &[
    // v0.1
    "next friday",
    "in 3 days",
    "sometime next week",
    "end of q3",
    "2nd monday of march",
    "early next month",
    "next business day",
    "2 weeks ago",
    "the tuesday after my birthday",
    "last business day of the month",
    // v0.2
    "friday next week",
    "first business day of next month",
    "the week after my birthday",
    "next weekend",
    "sometime early next month",
    "eom",
    "5 business days ago",
    "august",
];

fn bench_parse(c: &mut Criterion) {
    let now = "2026-07-12T15:30:00".parse().unwrap();
    let cfg = Cfg::default();
    let mut anchors = HashMap::new();
    anchors.insert("my birthday".to_string(), "2026-08-03".parse().unwrap());
    let no_holidays = |_d: chrono::NaiveDate| false;
    let anchor_names: Vec<String> = anchors.keys().cloned().collect();

    c.bench_function("parse_corpus_18", |b| {
        b.iter(|| {
            for p in PHRASES {
                black_box(parse_str(black_box(p), now, &anchors, &cfg, &no_holidays));
            }
        })
    });

    c.bench_function("parse_single_simple", |b| {
        b.iter(|| {
            black_box(parse_str(
                black_box("next friday"),
                now,
                &anchors,
                &cfg,
                &no_holidays,
            ))
        })
    });

    c.bench_function("tokenize_only", |b| {
        b.iter(|| {
            for p in PHRASES {
                black_box(tokenize::tokenize(black_box(p), &anchor_names, &EN));
            }
        })
    });
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
