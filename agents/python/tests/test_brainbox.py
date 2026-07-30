from d3rac_agents.brainbox import _fallback_directive


def test_fallback_directive_below_immediate_threshold():
    event = {"community_id": "test-c", "risk_score": 0.5, "theta": 0.15}
    directive = _fallback_directive(event)
    assert directive["recommended_action"] == "pre-position-and-monitor"
    assert directive["source"] == "fallback-deterministic"
    assert directive["release_pct_override"] is None


def test_fallback_directive_immediate_release_above_085():
    event = {"community_id": "test-c", "risk_score": 0.9, "theta": 0.15}
    directive = _fallback_directive(event)
    assert directive["recommended_action"] == "immediate-fund-release"
    assert directive["priority"] == 90
