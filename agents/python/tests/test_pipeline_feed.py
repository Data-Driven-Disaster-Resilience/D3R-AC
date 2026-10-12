import json
from datetime import datetime, timedelta, timezone

import pytest

from d3rac_agents import pipeline_feed
from d3rac_agents.bus import FileBus
from d3rac_agents.exposure_agent import ExposureAgent
from d3rac_agents.hazard_agent import HazardAgent, fetch_hazard_signals
from d3rac_agents.risk_model import RiskModelAgent
from d3rac_agents.vulnerability_agent import VulnerabilityAgent

NOW = datetime(2026, 10, 9, 12, 0, tzinfo=timezone.utc)


def row(cid, h, e, v, stale=False, source="firms"):
    return {"id": cid, "name": cid, "region": "NG", "hazard": h, "exposure": e, "vulnerability": v,
            "lastUpdated": NOW.isoformat(), "hazardSource": source, "stale": stale}


def write_feed(tmp_path, rows, generated=NOW):
    p = tmp_path / "communities.json"
    p.write_text(json.dumps({"generatedAt": generated.isoformat(), "communities": rows}))
    return p


@pytest.fixture(autouse=True)
def _env(monkeypatch):
    monkeypatch.delenv(pipeline_feed.FEED_ENV, raising=False)
    monkeypatch.delenv(pipeline_feed.MAX_AGE_ENV, raising=False)


def test_unset_env_keeps_sample_data():
    assert {s["source"] for s in fetch_hazard_signals()} == {"sample-feed"}


def test_fresh_rows_become_signals(tmp_path):
    p = write_feed(tmp_path, [row("a", 0.8, 0.7, 0.6)])
    rows = pipeline_feed.load_rows(p, now=NOW + timedelta(hours=1))
    assert rows[0]["fresh"] is True and rows[0]["source"] == "pipeline:firms"


def test_stale_row_is_not_fresh(tmp_path):
    p = write_feed(tmp_path, [row("a", 0.0, 0.7, 0.6, stale=True)])
    assert pipeline_feed.load_rows(p, now=NOW)[0]["fresh"] is False


def test_old_feed_suppresses_all_hazard_signals(tmp_path):
    p = write_feed(tmp_path, [row("a", 0.9, 0.7, 0.6)])
    rows = pipeline_feed.load_rows(p, now=NOW + timedelta(hours=7))
    assert rows and not any(r["fresh"] for r in rows)


@pytest.mark.parametrize("bad", [float("nan"), float("inf"), -0.1, 1.5, "0.5", True, None])
def test_invalid_values_skip_the_row(tmp_path, bad):
    p = write_feed(tmp_path, [row("a", bad, 0.7, 0.6), row("b", 0.5, 0.5, 0.5)])
    ids = [r["community_id"] for r in pipeline_feed.load_rows(p, now=NOW)]
    assert ids == ["b"]


@pytest.mark.parametrize("content", ["not json", "{}", '{"communities": 5}', ""])
def test_unusable_feed_fails_closed(tmp_path, monkeypatch, content):
    p = tmp_path / "communities.json"
    p.write_text(content)
    monkeypatch.setenv(pipeline_feed.FEED_ENV, str(p))
    assert pipeline_feed.load_rows() == []
    assert fetch_hazard_signals() == []  # never falls back to sample data


def test_missing_feed_file_fails_closed(tmp_path, monkeypatch):
    monkeypatch.setenv(pipeline_feed.FEED_ENV, str(tmp_path / "nope.json"))
    assert fetch_hazard_signals() == []


def test_end_to_end_feed_drives_risk_scores(tmp_path, monkeypatch):
    now = datetime.now(timezone.utc)
    p = write_feed(
        tmp_path,
        [row("hot", 0.9, 0.8, 0.7), row("quiet", 0.1, 0.5, 0.5), row("stale-one", 0.0, 0.9, 0.9, stale=True)],
        generated=now,
    )
    monkeypatch.setenv(pipeline_feed.FEED_ENV, str(p))
    bus_dir = tmp_path / "bus"
    for agent in (HazardAgent, ExposureAgent, VulnerabilityAgent):
        a = agent()
        a.bus = FileBus(str(bus_dir))
        a.run_once()
    risk = RiskModelAgent()
    risk.bus = FileBus(str(bus_dir))
    scores = {s["community_id"]: s for s in risk.run_once()}
    assert set(scores) == {"hot", "quiet"}           # stale community produces no risk event
    assert scores["hot"]["risk_score"] == round(0.9 * 0.8 * 0.7, 4)
    assert scores["hot"]["triggered"] is (scores["hot"]["risk_score"] >= risk.theta)
