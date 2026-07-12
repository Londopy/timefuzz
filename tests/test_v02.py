"""v0.2 feature tests: richer anchored phrases, business-calendar rules,
weekends, doubly-hedged vagueness, horizon penalty, new Ambiguous cases."""

from datetime import date, datetime

import pytest

import timefuzz as tf
from timefuzz import Ambiguous, Config, Instant, Range

BIRTHDAY = {"my birthday": date(2026, 8, 3)}  # a Monday


class TestRicherAnchors:
    def test_bare_anchor(self, now):
        r = tf.parse("my birthday", now=now, anchors=BIRTHDAY)
        assert isinstance(r, Instant)
        assert r.when == datetime(2026, 8, 3, 9, 0)
        assert r.confidence == 0.95

    def test_nth_weekday_after_anchor(self, now):
        r = tf.parse("2nd tuesday after my birthday", now=now, anchors=BIRTHDAY)
        assert r.when == datetime(2026, 8, 11, 9, 0)
        assert "2nd Tuesday" in r.interpretation

    def test_week_after_anchor_is_range(self, now):
        r = tf.parse("the week after my birthday", now=now, anchors=BIRTHDAY)
        assert isinstance(r, Range)
        assert r.start == datetime(2026, 8, 10)  # birthday week is Aug 3-9
        assert r.end == datetime(2026, 8, 16, 23, 59, 59)

    def test_month_before_anchor(self, now):
        r = tf.parse("the month before my birthday", now=now, anchors=BIRTHDAY)
        assert r.start == datetime(2026, 7, 1)
        assert r.end == datetime(2026, 7, 31, 23, 59, 59)

    def test_weekend_after_anchor(self, now):
        r = tf.parse("the weekend after my birthday", now=now, anchors=BIRTHDAY)
        assert r.start == datetime(2026, 8, 8)
        assert r.end == datetime(2026, 8, 9, 23, 59, 59)

    def test_business_days_around_anchor(self, now):
        after = tf.parse("3 business days after my birthday", now=now, anchors=BIRTHDAY)
        assert after.when == datetime(2026, 8, 6, 9, 0)  # Mon -> Thu
        before = tf.parse("2 business days before my birthday", now=now, anchors=BIRTHDAY)
        assert before.when == datetime(2026, 7, 30, 9, 0)  # Mon -> Thu prev week

    def test_business_days_after_anchor_respect_holidays(self, now):
        cfg = Config(holidays=lambda d: d == date(2026, 8, 4))
        r = tf.parse("1 business day after my birthday", now=now, anchors=BIRTHDAY, config=cfg)
        assert r.when == datetime(2026, 8, 5, 9, 0)


class TestBusinessCalendar:
    def test_first_business_day_of_next_month(self, now):
        r = tf.parse("first business day of next month", now=now)
        assert r.when == datetime(2026, 8, 3, 9, 0)  # Aug 1-2 is a weekend

    def test_nth_business_day(self, now):
        r = tf.parse("10th business day of next month", now=now)
        assert r.when == datetime(2026, 8, 14, 9, 0)

    def test_nth_business_day_overflow_raises(self, now):
        with pytest.raises(tf.ParseError, match="fewer than"):
            tf.parse("25th business day of next month", now=now)

    def test_first_business_day_respects_holidays(self, now):
        cfg = Config(holidays=lambda d: d == date(2026, 8, 3))
        r = tf.parse("first business day of next month", now=now, config=cfg)
        assert r.when == datetime(2026, 8, 4, 9, 0)

    def test_business_days_ago(self, now):
        r = tf.parse("5 business days ago", now=now)  # Sun back to prev Monday
        assert r.when == datetime(2026, 7, 6, 9, 0)

    @pytest.mark.parametrize(
        "text,expected",
        [
            ("eom", datetime(2026, 7, 31, 9, 0)),
            ("eoq", datetime(2026, 9, 30, 9, 0)),
            ("eoy", datetime(2026, 12, 31, 9, 0)),
            ("eow", datetime(2026, 7, 12, 9, 0)),
        ],
    )
    def test_shorthand(self, now, text, expected):
        r = tf.parse(text, now=now)
        assert r.when == expected


class TestWeekdayOfWeek:
    def test_friday_next_week(self, now):
        r = tf.parse("friday next week", now=now)
        assert r.when == datetime(2026, 7, 17, 9, 0)
        assert r.confidence == 0.95

    def test_monday_last_week(self, now):
        r = tf.parse("monday last week", now=now)
        assert r.when == datetime(2026, 6, 29, 9, 0)

    def test_tuesday_this_week_already_passed(self, now):
        r = tf.parse("tuesday this week", now=now)
        assert r.when == datetime(2026, 7, 7, 9, 0)
        assert r.confidence == 0.75
        assert "passed" in r.interpretation


class TestWeekends:
    def test_this_weekend(self, now):
        r = tf.parse("this weekend", now=now)
        assert r.start == datetime(2026, 7, 11)
        assert r.end == datetime(2026, 7, 12, 23, 59, 59)

    def test_last_weekend(self, now):
        r = tf.parse("last weekend", now=now)
        assert r.start == datetime(2026, 7, 4)

    def test_next_weekend_on_a_sunday_is_unambiguous(self, now):
        r = tf.parse("next weekend", now=now)  # now is Sunday
        assert isinstance(r, Range)
        assert r.start == datetime(2026, 7, 18)

    def test_next_weekend_midweek_is_ambiguous(self):
        wednesday = datetime(2026, 7, 8, 12, 0)
        r = tf.parse("next weekend", now=wednesday)
        assert isinstance(r, Ambiguous)
        assert len(r.candidates) == 2
        assert r.candidates[0].start == datetime(2026, 7, 11)
        assert r.candidates[1].start == datetime(2026, 7, 18)
        assert r.candidates[0].confidence > r.candidates[1].confidence
        assert "midweek" in r.reason

    def test_sometime_next_weekend(self, now):
        r = tf.parse("sometime next weekend", now=now)
        assert r.start == datetime(2026, 7, 18)
        assert r.confidence <= 0.8


class TestBareMonths:
    def test_upcoming_month(self, now):
        r = tf.parse("august", now=now)
        assert isinstance(r, Range)
        assert r.start == datetime(2026, 8, 1)
        assert r.confidence == 0.7

    def test_passed_month_rolls_forward(self, now):
        r = tf.parse("march", now=now)
        assert r.start == datetime(2027, 3, 1)
        assert "passed" in r.interpretation

    def test_current_month_is_ambiguous(self, now):
        r = tf.parse("july", now=now)  # said during July
        assert isinstance(r, Ambiguous)
        assert len(r.candidates) == 2
        assert r.candidates[0].start == datetime(2026, 7, 1)
        assert r.candidates[1].start == datetime(2027, 7, 1)
        assert "current month" in r.reason


class TestConfidenceRefinement:
    def test_horizon_penalty_far_future(self, now):
        r = tf.parse("in 15 years", now=now)
        assert r.when == datetime(2041, 7, 12, 15, 30)
        assert r.confidence == pytest.approx(0.9)
        assert "distant horizon" in r.interpretation

    def test_no_penalty_within_horizon(self, now):
        r = tf.parse("in 5 years", now=now)
        assert r.confidence == pytest.approx(0.95)

    def test_horizon_penalty_far_past(self, now):
        r = tf.parse("20 years ago", now=now)
        assert r.confidence == pytest.approx(0.9)

    def test_double_hedge_scores_below_single(self, now):
        single = tf.parse("early next month", now=now)
        double = tf.parse("sometime early next month", now=now)
        assert double.start == single.start
        assert double.end == single.end
        assert double.confidence < single.confidence
        assert double.confidence == pytest.approx(0.7)
        assert "doubly hedged" in double.interpretation

    def test_mid_month_bare_unit(self, now):
        r = tf.parse("mid month", now=now)
        assert r.start == datetime(2026, 7, 11)
        assert r.end == datetime(2026, 7, 20, 23, 59, 59)
