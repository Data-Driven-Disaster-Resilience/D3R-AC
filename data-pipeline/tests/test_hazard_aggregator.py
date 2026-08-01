from datetime import datetime, timezone

import pytest

from d3rac_pipeline.adapters.base import HazardReading
from d3rac_pipeline.hazard_aggregator import combine


def reading(value, source="src", detail="d"):
    return HazardReading(value=value, observed_at=datetime.now(timezone.utc), source=source, detail=detail)


def test_max_strategy_picks_highest():
    readings = [reading(0.2, "a"), reading(0.9, "b"), reading(0.5, "c")]
    result = combine(readings, strategy="max")
    assert result.value == 0.9
    assert result.driving_source == "b"


def test_weighted_mean_strategy():
    readings = [reading(0.2, "a"), reading(0.8, "b")]
    result = combine(readings, strategy="weighted_mean", weights={"a": 1.0, "b": 1.0})
    assert result.value == pytest.approx(0.5)


def test_weighted_mean_respects_weights():
    readings = [reading(0.0, "a"), reading(1.0, "b")]
    result = combine(readings, strategy="weighted_mean", weights={"a": 3.0, "b": 1.0})
    # (0*3 + 1*1) / 4 = 0.25
    assert result.value == pytest.approx(0.25)


def test_empty_readings_raises():
    with pytest.raises(ValueError):
        combine([], strategy="max")


def test_unknown_strategy_raises():
    with pytest.raises(ValueError):
        combine([reading(0.5)], strategy="not_a_real_strategy")
