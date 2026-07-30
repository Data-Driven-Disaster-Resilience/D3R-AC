from d3rac_agents.risk_model import compute_risk
from d3rac_agents.exposure_agent import compute_exposure
from d3rac_agents.vulnerability_agent import compute_vulnerability


def test_compute_risk_matches_readme_formula():
    # R(c,t) = H(t) * E(c) * V(c)
    assert compute_risk(0.5, 0.5, 0.5) == 0.125
    assert compute_risk(1.0, 1.0, 1.0) == 1.0
    assert compute_risk(0.0, 0.9, 0.9) == 0.0


def test_compute_exposure_bounds():
    assert 0.0 <= compute_exposure(210_000, 14) <= 1.0
    assert compute_exposure(0, 0) == 0.0


def test_compute_vulnerability_bounds():
    assert 0.0 <= compute_vulnerability(0.44, 0.35) <= 1.0
    assert compute_vulnerability(0.0, 1.0) == 0.0
