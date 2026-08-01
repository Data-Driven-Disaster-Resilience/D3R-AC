"""
FR-3 — Fixed-point conversion matching RiskRegistry.sol exactly.

RiskRegistry represents H/E/V/R as uint256 in [0, 1e18] (see `SCALE` in
RiskRegistry.sol). This module is the ONLY place that conversion happens,
so every value the pipeline ever submits goes through the same rounding
rule: round-half-up, then clamp to [0, SCALE].

Round-half-up (not banker's rounding) is chosen because it's simple to
reason about and to re-derive by hand when auditing a submitted value
against its source float (NFR-3).
"""

from decimal import Decimal, ROUND_HALF_UP

SCALE = 10**18  # matches RiskRegistry.SCALE exactly


def to_fixed_point(value: float) -> int:
    """Convert a [0,1] float to the [0, 1e18] uint256 range the contract
    expects, using round-half-up. Values outside [0,1] are clamped rather
    than raising, since a slightly-out-of-range float (e.g. 1.0000000002
    from floating point noise) is a rounding artifact, not a data error —
    the contract itself is the final gate (`RiskRegistry: value out of
    [0,1] range`), and clamping here means the pipeline never sends a
    value that predictably reverts just from floating point drift.
    """
    clamped = max(0.0, min(1.0, float(value)))
    scaled = Decimal(str(clamped)) * Decimal(SCALE)
    return int(scaled.to_integral_value(rounding=ROUND_HALF_UP))


def from_fixed_point(value: int) -> float:
    """Inverse of to_fixed_point, for reading values back (e.g. when
    verifying against RiskRegistry.getCommunity in acceptance testing)."""
    return int(value) / SCALE
