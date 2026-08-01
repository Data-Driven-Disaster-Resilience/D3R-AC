"""
Combines multiple HazardAdapter readings for one community into a single
H(t) in [0,1], per settings.yaml's `hazard_combine_strategy`.

Kept separate from pipeline.py so the combination policy (max vs.
weighted mean today; a domain expert may want something more nuanced
later, e.g. co-occurring hazards compounding risk) is swappable on its
own, and independently unit-testable.
"""

from __future__ import annotations

from dataclasses import dataclass

from .adapters.base import HazardReading


@dataclass
class CombinedHazard:
    value: float
    driving_source: str
    driving_detail: str
    observed_at: object  # datetime; kept loose to avoid a circular import for typing only


def combine(
    readings: list[HazardReading],
    strategy: str = "max",
    weights: dict | None = None,
) -> CombinedHazard:
    if not readings:
        raise ValueError("combine() called with no hazard readings — at least one adapter, "
                          "even one reporting 0.0, must run per community per cycle (FR-2).")

    if strategy == "max":
        best = max(readings, key=lambda r: r.value)
        return CombinedHazard(
            value=best.value,
            driving_source=best.source,
            driving_detail=best.detail,
            observed_at=best.observed_at,
        )

    if strategy == "weighted_mean":
        weights = weights or {}
        total_weight = sum(weights.get(r.source, 1.0) for r in readings) or 1.0
        weighted_sum = sum(r.value * weights.get(r.source, 1.0) for r in readings)
        value = weighted_sum / total_weight
        best = max(readings, key=lambda r: r.value)  # still report the most severe as "driving"
        return CombinedHazard(
            value=value,
            driving_source=f"weighted_mean(driven by {best.source})",
            driving_detail=best.detail,
            observed_at=best.observed_at,
        )

    raise ValueError(f"Unknown hazard_combine_strategy: {strategy!r}")
