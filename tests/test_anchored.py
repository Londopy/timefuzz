from datetime import date, datetime

import timefuzz as tf
from timefuzz import Ambiguous, Instant

BIRTHDAY = {"my birthday": date(2026, 8, 3)}  # a Monday


def test_tuesday_after_birthday_matches_spec_example(now):
    r = tf.parse("the tuesday after my birthday", now=now, anchors=BIRTHDAY)
    assert isinstance(r, Instant)
    assert r.when == datetime(2026, 8, 4, 9, 0)
    assert r.confidence == 0.9
    assert "strictly after 2026-08-03" in r.interpretation


def test_weekday_before_anchor(now):
    r = tf.parse("the friday before my birthday", now=now, anchors=BIRTHDAY)
    assert r.when == datetime(2026, 7, 31, 9, 0)


def test_same_weekday_is_strict(now):
    # Anchor is a Monday; "monday after" must not return the anchor itself.
    r = tf.parse("the monday after my birthday", now=now, anchors=BIRTHDAY)
    assert r.when == datetime(2026, 8, 10, 9, 0)


def test_day_after_and_before(now):
    assert tf.parse(
        "the day after my birthday", now=now, anchors=BIRTHDAY
    ).when == datetime(2026, 8, 4, 9, 0)
    assert tf.parse(
        "the day before my birthday", now=now, anchors=BIRTHDAY
    ).when == datetime(2026, 8, 2, 9, 0)


def test_n_units_after_anchor(now):
    r = tf.parse("2 weeks after my birthday", now=now, anchors=BIRTHDAY)
    assert r.when == datetime(2026, 8, 17, 9, 0)
    r = tf.parse("3 days before my birthday", now=now, anchors=BIRTHDAY)
    assert r.when == datetime(2026, 7, 31, 9, 0)


def test_feb_29_birthday(now):
    anchors = {"my birthday": date(2028, 2, 29)}
    r = tf.parse("the day after my birthday", now=now, anchors=anchors)
    assert r.when == datetime(2028, 3, 1, 9, 0)
    r = tf.parse("1 year after my birthday", now=now, anchors=anchors)
    assert r.when == datetime(2029, 2, 28, 9, 0)  # clamped: 2029 has no Feb 29


def test_anchor_matching_is_case_insensitive(now):
    r = tf.parse(
        "the Tuesday after My Birthday",
        now=now,
        anchors={"My Birthday": date(2026, 8, 3)},
    )
    assert r.when == datetime(2026, 8, 4, 9, 0)


def test_longest_anchor_name_wins(now):
    anchors = {
        "the wedding": date(2026, 9, 5),
        "the wedding rehearsal": date(2026, 9, 4),
    }
    r = tf.parse("the day after the wedding rehearsal", now=now, anchors=anchors)
    assert r.when == datetime(2026, 9, 5, 9, 0)


def test_unknown_anchor_is_ambiguous_with_reason(now):
    r = tf.parse("the tuesday after my graduation", now=now)
    assert isinstance(r, Ambiguous)
    assert r.candidates == []
    assert "unknown anchor" in r.reason
    assert "my graduation" in r.reason
