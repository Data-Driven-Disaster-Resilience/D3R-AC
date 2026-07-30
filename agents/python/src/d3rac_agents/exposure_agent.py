"""
exposure-scorer: publishes E(c), the exposure-factor term of R(c,t) = H(t)*E(c)*V(c).

E(c) should reflect how much population/infrastructure/economic value sits in harm's
way for community c (population density, critical-facility count, asset value, etc).
Replace `community_registry` with a real feed from your community-registry data source.
"""
from __future__ import annotations

from typing import TypedDict

from .bus import get_bus


class ExposureScore(TypedDict):
    community_id: str
    exposure_factor: float  # 0..1
    population: int


def community_registry() -> list[dict]:
    """Stub: replace with real community registry (population, critical facilities, assets)."""
    return [
        {"community_id": "abuja-mararaba", "population": 210_000, "critical_facilities": 14},
        {"community_id": "abuja-nyanya", "population": 95_000, "critical_facilities": 6},
    ]


def compute_exposure(population: int, critical_facilities: int) -> float:
    # Simple normalized blend; swap in a calibrated model once real data is available.
    pop_term = min(population / 500_000, 1.0)
    facility_term = min(critical_facilities / 25, 1.0)
    return round(0.7 * pop_term + 0.3 * facility_term, 4)


class ExposureAgent:
    def __init__(self):
        self.bus = get_bus()

    def run_once(self) -> list[ExposureScore]:
        out: list[ExposureScore] = []
        for c in community_registry():
            score: ExposureScore = {
                "community_id": c["community_id"],
                "exposure_factor": compute_exposure(c["population"], c["critical_facilities"]),
                "population": c["population"],
            }
            self.bus.publish("exposure.score", score)
            out.append(score)
        return out


if __name__ == "__main__":
    published = ExposureAgent().run_once()
    print(f"published {len(published)} exposure.score events")
