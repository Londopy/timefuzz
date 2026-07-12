"""Corpus-driven suite: every corpus.jsonl line is one test case.

Adding a phrase to the grammar = adding a line to the corpus.
All cases run against the fixed reference now (2026-07-12 15:30, a Sunday)
unless the line carries its own "now".
"""

import json
from datetime import date, datetime
from pathlib import Path

import pytest

import timefuzz as tf
from timefuzz import Ambiguous, Instant, Range

from conftest import NOW

CORPUS_FILES = sorted(Path(__file__).parent.glob("corpus*.jsonl"))


def _cases():
    lines: list[str] = []
    for f in CORPUS_FILES:
        lines += f.read_text(encoding="utf-8").splitlines()
    for i, line in enumerate(lines):
        line = line.strip()
        if line:
            case = json.loads(line)
            yield pytest.param(case, id=f"{i:03d}-{case['text'][:40]}")


@pytest.mark.parametrize("case", [*_cases()])
def test_corpus(case):
    now = datetime.fromisoformat(case["now"]) if "now" in case else NOW
    anchors = {
        k: date.fromisoformat(v) for k, v in case.get("anchors", {}).items()
    }
    expect = case["expect"]
    kind = expect["kind"]

    if kind == "no_parse":
        with pytest.raises(tf.ParseError) as exc:
            tf.parse(case["text"], now=now, anchors=anchors)
        if "reason_contains" in expect:
            assert expect["reason_contains"] in str(exc.value)
        return

    r = tf.parse(case["text"], now=now, anchors=anchors)

    if kind == "instant":
        assert isinstance(r, Instant), r
        assert r.when == datetime.fromisoformat(expect["when"])
    elif kind == "range":
        assert isinstance(r, Range), r
        assert r.start == datetime.fromisoformat(expect["start"])
        assert r.end == datetime.fromisoformat(expect["end"])
        assert r.start <= r.end
    elif kind == "ambiguous":
        assert isinstance(r, Ambiguous), r
        if "n_candidates" in expect:
            assert len(r.candidates) == expect["n_candidates"]
        if "reason_contains" in expect:
            assert expect["reason_contains"] in r.reason
        return
    else:
        pytest.fail(f"unknown expectation kind {kind!r}")

    if "confidence" in expect:
        assert r.confidence == pytest.approx(expect["confidence"], abs=1e-6)
    if "confidence_min" in expect:
        assert r.confidence >= expect["confidence_min"]
    if "confidence_max" in expect:
        assert r.confidence <= expect["confidence_max"]
    if "interpretation_contains" in expect:
        assert expect["interpretation_contains"] in r.interpretation
    assert 0.0 <= r.confidence <= 1.0
