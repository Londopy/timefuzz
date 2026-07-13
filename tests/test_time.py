"""Clock-time support (v0.3): trailing "at <time>" overrides the default
time of day; bare clock times resolve to the next occurrence."""

from datetime import date, datetime

import pytest

import timefuzz as tf
from timefuzz import Ambiguous, Instant, Range


class TestTrailingTime:
    def test_next_friday_at_3pm(self, now):
        r = tf.parse("next friday at 3pm", now=now)
        assert r.when == datetime(2026, 7, 17, 15, 0)
        assert r.confidence == 0.95
        assert "at 15:00" in r.interpretation

    def test_24h_and_named_times(self, now):
        assert tf.parse("tomorrow at 7:30", now=now).when == datetime(2026, 7, 13, 7, 30)
        assert tf.parse("tomorrow at noon", now=now).when == datetime(2026, 7, 13, 12, 0)
        assert tf.parse("friday at midnight", now=now).when == datetime(2026, 7, 17, 0, 0)

    def test_time_does_not_change_confidence(self, now):
        plain = tf.parse("next friday", now=now)
        timed = tf.parse("next friday at 3pm", now=now)
        assert timed.confidence == plain.confidence

    def test_arithmetic_days_take_explicit_time(self, now):
        r = tf.parse("in 3 days at 5pm", now=now)
        assert r.when == datetime(2026, 7, 15, 17, 0)

    def test_arithmetic_hours_keep_their_clock(self, now):
        # hour arithmetic is already time-precise; trailing time is ignored
        r = tf.parse("in 3 hours at 5pm", now=now)
        assert r.when == datetime(2026, 7, 12, 18, 30)

    def test_ranges_ignore_trailing_time(self, now):
        r = tf.parse("next weekend at 3pm", now=now)
        assert isinstance(r, Range)
        assert r.start == datetime(2026, 7, 18)  # whole-day span unchanged

    def test_ambiguous_candidates_carry_the_time(self, now):
        r = tf.parse("sunday at 3pm", now=now)  # today is Sunday
        assert isinstance(r, Ambiguous)
        assert {c.when for c in r.candidates} == {
            datetime(2026, 7, 12, 15, 0),
            datetime(2026, 7, 19, 15, 0),
        }

    def test_anchored_with_time(self, now):
        r = tf.parse(
            "the tuesday after my birthday at 5pm",
            now=now,
            anchors={"my birthday": date(2026, 8, 3)},
        )
        assert r.when == datetime(2026, 8, 4, 17, 0)


class TestBareTimes:
    def test_future_time_today(self, now):  # now is 15:30
        r = tf.parse("15:45", now=now)
        assert isinstance(r, Instant)
        assert r.when == datetime(2026, 7, 12, 15, 45)
        assert "today" in r.interpretation

    def test_passed_time_rolls_to_tomorrow(self, now):
        r = tf.parse("3pm", now=now)
        assert r.when == datetime(2026, 7, 13, 15, 0)
        assert "passed" in r.interpretation
        assert r.confidence == 0.9

    def test_meridiem_edge_cases(self, now):
        assert tf.parse("12pm", now=now).when.hour == 12  # noon
        assert tf.parse("12am", now=now).when.hour == 0  # midnight
        assert tf.parse("3 pm", now=now).when.hour == 15  # spaced meridiem

    def test_invalid_times_do_not_parse(self, now):
        for text in ("25:00", "13pm", "7:99"):
            with pytest.raises(tf.ParseError):
                tf.parse(text, now=now)
