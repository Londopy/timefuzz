"""Shared fixtures. All tests run against a fixed reference `now` for
determinism: Sunday 2026-07-12 15:30."""

from datetime import datetime

import pytest

NOW = datetime(2026, 7, 12, 15, 30)  # Sunday


@pytest.fixture
def now() -> datetime:
    return NOW
