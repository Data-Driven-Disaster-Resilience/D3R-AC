"""
hazard-ingestor: publishes H(t), the hazard-probability term of R(c,t) = H(t)*E(c)*V(c).

Replace `fetch_hazard_signals` with real feeds (weather APIs, seismic networks, flood
gauges, satellite change-detection, etc). It's intentionally isolated from the rest of
the pipeline so swapping data sources never touches risk_model.py or the bus contract.
"""
from __future__ import annotations

from typing import TypedDict

from .bus import get_bus


class HazardSignal(TypedDict):
    community_id: str
    hazard_type: str
    probability: float  # 0..1
    source: str


def fetch_hazard_signals() -> list[HazardSignal]:
    """Stub: replace with real hazard-feed ingestion (HTTP/webhook/queue)."""
    return [
        {"community_id": "abuja-mararaba", "hazard_type": "flood", "probability": 0.72, "source": "sample-feed"},
        {"community_id": "abuja-nyanya", "hazard_type": "flood", "probability": 0.31, "source": "sample-feed"},
    ]


class HazardAgent:
    def __init__(self):
        self.bus = get_bus()

    def run_once(self) -> list[HazardSignal]:
        signals = fetch_hazard_signals()
        for s in signals:
            self.bus.publish("hazard.signal", s)
        return signals


if __name__ == "__main__":
    published = HazardAgent().run_once()
    print(f"published {len(published)} hazard.signal events")
