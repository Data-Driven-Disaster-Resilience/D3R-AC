import json

from d3rac_pipeline.config import Community
from d3rac_pipeline.fixed_point import to_fixed_point
from d3rac_pipeline.frontend_feed import build_frontend_feed, write_frontend_feed
from d3rac_pipeline.state_store import StateStore


def make_community(id_="test-community"):
    return Community(
        id=id_, name="Test Community", region="Test, XX",
        priority_region="africa", bbox=[0, 0, 1, 1], exposure=0.5, vulnerability=0.5,
    )


def test_feed_for_never_submitted_community_uses_config_fallback(tmp_path):
    community = make_community()
    store = StateStore(str(tmp_path / "state.sqlite3"))
    feed = build_frontend_feed([community], store, stale_after_hours=12)

    row = feed.communities[0]
    assert row.id == "test-community"
    assert row.hazard == 0.0
    assert row.exposure == 0.5
    assert row.vulnerability == 0.5
    assert row.stale is True


def test_feed_reflects_submitted_values(tmp_path):
    community = make_community()
    store = StateStore(str(tmp_path / "state.sqlite3"))
    store.record_submission(
        community.id,
        to_fixed_point(0.8),
        to_fixed_point(0.6),
        to_fixed_point(0.4),
        cycle_id="cycle-1",
    )
    feed = build_frontend_feed([community], store, stale_after_hours=12,
                                last_hazard_sources={"test-community": "satellite_fire"})
    row = feed.communities[0]
    assert row.hazard == 0.8
    assert row.exposure == 0.6
    assert row.vulnerability == 0.4
    assert row.hazardSource == "satellite_fire"
    assert row.stale is False


def test_write_frontend_feed_writes_valid_json_to_multiple_paths(tmp_path):
    community = make_community()
    store = StateStore(str(tmp_path / "state.sqlite3"))
    feed = build_frontend_feed([community], store, stale_after_hours=12)

    path_a = tmp_path / "a" / "communities.json"
    path_b = tmp_path / "b" / "communities.json"
    write_frontend_feed(feed, str(path_a), str(path_b))

    for path in (path_a, path_b):
        with open(path) as f:
            data = json.load(f)
        assert "generatedAt" in data
        assert data["communities"][0]["id"] == "test-community"
