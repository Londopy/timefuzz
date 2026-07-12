from datetime import datetime, time

import timefuzz as tf
from timefuzz import Ambiguous, Config, Instant


def test_next_friday_matches_spec_example(now):
    r = tf.parse("next friday", now=now)
    assert isinstance(r, Instant)
    assert r.when == datetime(2026, 7, 17, 9, 0)
    assert r.confidence == 0.95
    assert "Friday" in r.interpretation


def test_next_weekday_when_today_is_that_weekday(now):
    # now is a Sunday.
    r = tf.parse("next sunday", now=now)
    assert r.when == datetime(2026, 7, 19, 9, 0)  # skips today by default
    assert "next_skips_today" in r.interpretation

    cfg = Config(next_skips_today=False)
    r2 = tf.parse("next sunday", now=now, config=cfg)
    assert r2.when == datetime(2026, 7, 12, 9, 0)  # today counts


def test_this_weekday_already_passed(now):
    # Friday of the current week (Mon Jul 6 - Sun Jul 12) is Jul 10, in the past.
    r = tf.parse("this friday", now=now)
    assert r.when == datetime(2026, 7, 10, 9, 0)
    assert r.confidence < 0.8
    assert "passed" in r.interpretation


def test_last_weekday(now):
    r = tf.parse("last monday", now=now)
    assert r.when == datetime(2026, 7, 6, 9, 0)


def test_bare_weekday_is_upcoming(now):
    r = tf.parse("friday", now=now)
    assert r.when == datetime(2026, 7, 17, 9, 0)
    assert r.confidence == 0.7


def test_bare_weekday_on_same_day_is_ambiguous(now):
    r = tf.parse("sunday", now=now)  # today is Sunday
    assert isinstance(r, Ambiguous)
    assert len(r.candidates) == 2
    assert {c.when.day for c in r.candidates} == {12, 19}
    assert "today" in r.reason


def test_week_start_sunday_changes_this_week(now):
    # With Sunday weeks, "this friday" is Jul 17 (the week is Jul 12-18).
    cfg = Config(week_start=tf.Weekday.SUN)
    r = tf.parse("this friday", now=now, config=cfg)
    assert r.when == datetime(2026, 7, 17, 9, 0)


def test_custom_default_time(now):
    cfg = Config(default_time=time(14, 30))
    r = tf.parse("next friday", now=now, config=cfg)
    assert r.when == datetime(2026, 7, 17, 14, 30)


def test_abbreviated_weekday_names(now):
    assert tf.parse("next fri", now=now).when == datetime(2026, 7, 17, 9, 0)
    assert tf.parse("next tues", now=now).when == datetime(2026, 7, 14, 9, 0)
