"""
brainbox: the central AI-agent controller for D3R.AC.

Every other agent in this system is deterministic on purpose (the risk model is a fixed
formula, the response/contract agents follow fixed rules) -- predictable and auditable
matters a lot when the output can trigger a fund release. brainbox is the one piece that
is genuinely LLM-backed: it reads every signal on the bus for a cycle, asks Claude to
reason about the whole picture, and publishes a single authoritative `brainbox.directive`
event per triggered community. Downstream agents (coordinationAgent, alertAgent,
contractTriggerAgent in node/) read this directive first and only fall back to their own
built-in rules if brainbox didn't produce one for that community -- so the system degrades
gracefully rather than breaking if the API is unavailable.

Requires ANTHROPIC_API_KEY in the environment to actually call Claude. Without it, this
still runs (falls back to a deterministic directive equivalent to the theta-threshold
logic) so `make dev` and CI keep working without a live key.
"""
from __future__ import annotations

import json
import os
from typing import Any, TypedDict

from . import _generated_manifest as manifest
from .bus import get_bus

SYSTEM_PROMPT = """You are the D3R.AC brainbox: the reasoning layer for a blockchain-based
disaster resilience system covering communities in Nigeria. You receive hazard, exposure,
vulnerability, and computed risk-score signals for one or more communities. For each
community that has crossed its risk threshold, decide:

- priority: integer 0-100, relative urgency across all communities in this batch
- recommended_action: one of "monitor", "pre-position-and-monitor", "immediate-fund-release"
- release_pct_override: integer 0-100 or null (only set if you want to override the
  standard first-milestone 30% release given the severity/context)
- rationale: 1-2 plain-language sentences an NGO coordinator with zero blockchain
  literacy can act on
- alert_message: a single sentence to send directly to the community/NGO channel

Respond with ONLY a JSON array of objects with keys:
community_id, priority, recommended_action, release_pct_override, rationale, alert_message
No other text, no markdown fences."""


class Directive(TypedDict):
    community_id: str
    priority: int
    recommended_action: str
    release_pct_override: int | None
    rationale: str
    alert_message: str
    source: str  # "claude" or "fallback-deterministic"


def _fallback_directive(risk_event: dict) -> Directive:
    """Used when ANTHROPIC_API_KEY isn't set. Mirrors the theta-threshold logic so the
    pipeline still produces a usable directive offline/in CI."""
    r = risk_event["risk_score"]
    action = "immediate-fund-release" if r >= 0.85 else "pre-position-and-monitor"
    return {
        "community_id": risk_event["community_id"],
        "priority": min(int(r * 100), 100),
        "recommended_action": action,
        "release_pct_override": None,
        "rationale": (
            f"Risk score {r} crossed threshold {risk_event['theta']}; "
            f"following standard {action} procedure (no live AI reasoning -- "
            f"ANTHROPIC_API_KEY not set)."
        ),
        "alert_message": (
            f"{risk_event['community_id']}: risk threshold crossed (score {r}). "
            f"Support is being arranged."
        ),
        "source": "fallback-deterministic",
    }


def _call_claude(triggered_events: list[dict]) -> list[Directive]:
    import anthropic  # local import: only required if a real API call is being made

    client = anthropic.Anthropic()  # reads ANTHROPIC_API_KEY from env
    model = manifest.AGENTS.get("brainbox", {}).get("config", {}).get("model", "claude-sonnet-4-6")

    user_content = json.dumps({"triggered_communities": triggered_events}, indent=2)
    response = client.messages.create(
        model=model,
        max_tokens=2000,
        system=SYSTEM_PROMPT,
        messages=[{"role": "user", "content": user_content}],
    )
    text = "".join(block.text for block in response.content if block.type == "text")
    parsed: list[dict[str, Any]] = json.loads(text)
    directives: list[Directive] = []
    for item in parsed:
        item["source"] = "claude"
        directives.append(item)  # type: ignore[arg-type]
    return directives


class BrainboxAgent:
    def __init__(self):
        self.bus = get_bus()
        self.has_api_key = bool(os.environ.get("ANTHROPIC_API_KEY"))

    def run_once(self) -> list[Directive]:
        risk_events = [e["payload"] for e in self.bus.read_all("risk.score") if e["payload"]["triggered"]]
        if not risk_events:
            return []

        if self.has_api_key:
            try:
                directives = _call_claude(risk_events)
            except Exception as exc:  # noqa: BLE001 -- intentionally broad: any Claude/API failure must degrade, not crash the pipeline
                print(f"brainbox: Claude call failed ({exc}), falling back to deterministic directives")
                directives = [_fallback_directive(e) for e in risk_events]
        else:
            directives = [_fallback_directive(e) for e in risk_events]

        for directive in directives:
            self.bus.publish("brainbox.directive", directive)
        return directives


if __name__ == "__main__":
    published = BrainboxAgent().run_once()
    for d in published:
        print(f"[{d['source']}] {d['community_id']}: {d['recommended_action']} (priority {d['priority']}) - {d['rationale']}")
