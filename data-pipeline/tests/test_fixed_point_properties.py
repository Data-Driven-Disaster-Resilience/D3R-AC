"""Property-based tests (Hypothesis) for the pipeline's fixed-point layer and a
Python model of RiskRegistry.riskScore. These are the first fuzz-style checks in
the repo (docs/audit-engagement-package.md section 3.5 lists their absence)."""
import math

import pytest
from hypothesis import given, strategies as st

from d3rac_pipeline.fixed_point import SCALE, from_fixed_point, to_fixed_point

unit = st.floats(min_value=0.0, max_value=1.0, allow_nan=False)
fixed = st.integers(min_value=0, max_value=SCALE)
UINT256_MAX = 2**256 - 1


def risk_score(h: int, e: int, v: int) -> int:
    """Mirror of RiskRegistry.riskScore: (h*e*v) / (SCALE*SCALE), integer division."""
    return (h * e * v) // (SCALE * SCALE)


@given(st.floats(allow_nan=False, allow_infinity=True))
def test_output_always_in_contract_range(x):
    if math.isinf(x):
        with pytest.raises(ValueError):
            to_fixed_point(x)
    else:
        assert 0 <= to_fixed_point(x) <= SCALE


@given(unit, unit)
def test_monotonic(a, b):
    lo, hi = sorted((a, b))
    assert to_fixed_point(lo) <= to_fixed_point(hi)


@given(unit)
def test_round_trip_error_is_tiny(x):
    assert abs(from_fixed_point(to_fixed_point(x)) - x) <= 1e-9


@pytest.mark.parametrize("bad", [float("nan"), float("inf"), float("-inf")])
def test_non_finite_is_rejected_not_clamped_to_max_risk(bad):
    with pytest.raises(ValueError):
        to_fixed_point(bad)


@given(fixed, fixed, fixed)
def test_risk_score_bounded_and_cannot_overflow_uint256(h, e, v):
    assert h * e * v <= UINT256_MAX  # RiskRegistry's documented 1e54 headroom
    assert 0 <= risk_score(h, e, v) <= SCALE


@given(fixed, fixed, fixed, fixed)
def test_risk_score_monotonic_in_hazard(h1, h2, e, v):
    lo, hi = sorted((h1, h2))
    assert risk_score(lo, e, v) <= risk_score(hi, e, v)


@given(fixed, fixed)
def test_zero_factor_means_zero_risk(a, b):
    assert risk_score(0, a, b) == risk_score(a, 0, b) == risk_score(a, b, 0) == 0


@given(fixed)
def test_full_exposure_and_vulnerability_returns_hazard(h):
    assert risk_score(h, SCALE, SCALE) == h
