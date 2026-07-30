"""
risk-model: the analysis agent at the center of D3R.AC.

Implements R(c, t) = H(t) * E(c) * V(c), per the main D3R-AC README's risk model.
Joins the three upstream signals by community_id, computes R, and publishes risk.score.
When R crosses theta (from agents.config.yaml -> agents.risk-model.config.theta), the
event is flagged so downstream Node coordination agents know to act.
"""
from __future__ import annotations

from typing import TypedDict

from . import _generated_manifest as manifest
from .bus import get_bus


class RiskScore(TypedDict):
    community_id: str
    hazard_probability: float
    exposure_factor: float
    vulnerability_index: float
    risk_score: float
    theta: float
    triggered: bool


def _theta() -> float:
    return float(manifest.AGENTS["risk-model"]["config"]["theta"])


def compute_risk(hazard_probability: float, exposure_factor: float, vulnerability_index: float) -> float:
    return round(hazard_probability * exposure_factor * vulnerability_index, 4)


class RiskModelAgent:
    def __init__(self):
        self.bus = get_bus()
        self.theta = _theta()

    def _latest_by_community(self, topic: str, value_key: str) -> dict[str, float]:
        latest: dict[str, float] = {}
        for event in self.bus.read_all(topic):
            payload = event["payload"]
            latest[payload["community_id"]] = payload[value_key]
        return latest

    def run_once(self) -> list[RiskScore]:
        hazards = self._latest_by_community("hazard.signal", "probability")
        exposures = self._latest_by_community("exposure.score", "exposure_factor")
        vulnerabilities = self._latest_by_community("vulnerability.index", "vulnerability_index")

        results: list[RiskScore] = []
        for community_id in hazards.keys() & exposures.keys() & vulnerabilities.keys():
            h, e, v = hazards[community_id], exposures[community_id], vulnerabilities[community_id]
            r = compute_risk(h, e, v)
            score: RiskScore = {
                "community_id": community_id,
                "hazard_probability": h,
                "exposure_factor": e,
                "vulnerability_index": v,
                "risk_score": r,
                "theta": self.theta,
                "triggered": r >= self.theta,
            }
            self.bus.publish("risk.score", score)
            results.append(score)
        return results


if __name__ == "__main__":
    published = RiskModelAgent().run_once()
    for s in published:
        flag = "TRIGGERED" if s["triggered"] else "below theta"
        print(f"{s['community_id']}: R={s['risk_score']} ({flag}, theta={s['theta']})")
