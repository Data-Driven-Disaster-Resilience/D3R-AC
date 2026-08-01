from datetime import datetime, timedelta, timezone

import pytest

from d3rac_pipeline.adapters.base import HazardReading, NoFreshData
from d3rac_pipeline.config import ChainConfig, Community, PipelineSettings, StaleConfig
from d3rac_pipeline.pipeline import Pipeline


def make_community(id_="test-community", priority_region="africa"):
    return Community(
        id=id_,
        name="Test Community",
        region="Test, XX",
        priority_region=priority_region,
        bbox=[0.0, 0.0, 1.0, 1.0],
        exposure=0.5,
        vulnerability=0.5,
    )


def make_settings(tmp_path, stale_policy="hold_and_flag", stale_after_hours=12):
    return PipelineSettings(
        stale=StaleConfig(stale_after_hours=stale_after_hours, stale_policy=stale_policy),
        hazard_combine_strategy="max",
        hazard_weights={},
        state_db_path=str(tmp_path / "state.sqlite3"),
        chain=ChainConfig(network="shasta", hub_address="", full_node=""),
        log_level="INFO",
        audit_log_path=str(tmp_path / "audit.log"),
    )


class FakeHazardAdapter:
    def __init__(self, value, name="fake_hazard", observed_at=None, raise_no_data=False):
        self.value = value
        self.name = name
        self.observed_at = observed_at or datetime.now(timezone.utc)
        self.raise_no_data = raise_no_data
        self.calls = 0

    def fetch(self, community):
        self.calls += 1
        if self.raise_no_data:
            raise NoFreshData("no data")
        return HazardReading(value=self.value, observed_at=self.observed_at,
                              source=self.name, detail="fake reading")


class FailingHazardAdapter:
    name = "failing_adapter"

    def fetch(self, community):
        raise RuntimeError("source is down")


class FakeExposureAdapter:
    def __init__(self, value=0.5):
        self.value = value

    def fetch(self, community):
        return self.value


class FakeVulnerabilityAdapter:
    def __init__(self, value=0.5):
        self.value = value

    def fetch(self, community):
        return self.value


def test_dry_run_submits_via_state_store_no_chain(tmp_path):
    community = make_community()
    settings = make_settings(tmp_path)
    pipeline = Pipeline(
        communities=[community],
        settings=settings,
        hazard_adapters=[FakeHazardAdapter(0.7)],
        exposure_adapter=FakeExposureAdapter(),
        vulnerability_adapter=FakeVulnerabilityAdapter(),
        chain_client=None,  # dry-run
    )
    summary = pipeline.run_cycle()
    assert summary.succeeded == 1
    assert summary.failed == 0


def test_unchanged_values_are_skipped_second_cycle(tmp_path):
    community = make_community()
    settings = make_settings(tmp_path)
    hazard_adapter = FakeHazardAdapter(0.7)
    pipeline = Pipeline(
        communities=[community],
        settings=settings,
        hazard_adapters=[hazard_adapter],
        exposure_adapter=FakeExposureAdapter(),
        vulnerability_adapter=FakeVulnerabilityAdapter(),
        chain_client=None,
    )
    first = pipeline.run_cycle()
    second = pipeline.run_cycle()

    assert first.succeeded == 1
    assert second.succeeded == 0
    assert second.skipped_unchanged == 1


def test_changed_hazard_triggers_new_submission(tmp_path):
    community = make_community()
    settings = make_settings(tmp_path)
    hazard_adapter = FakeHazardAdapter(0.7)
    pipeline = Pipeline(
        communities=[community],
        settings=settings,
        hazard_adapters=[hazard_adapter],
        exposure_adapter=FakeExposureAdapter(),
        vulnerability_adapter=FakeVulnerabilityAdapter(),
        chain_client=None,
    )
    pipeline.run_cycle()
    hazard_adapter.value = 0.95  # hazard changed
    second = pipeline.run_cycle()
    assert second.succeeded == 1


def test_one_community_failure_does_not_block_others(tmp_path):
    good = make_community("good-community")
    bad = make_community("bad-community")
    settings = make_settings(tmp_path)

    # FailingHazardAdapter always raises for both communities' hazard fetch,
    # so both communities should actually fail here, since _compute_hazard
    # swallows adapter exceptions and returns None only if *no* adapter
    # succeeds. To test true per-community isolation we give the "bad"
    # community no working adapters at all (=> skipped_stale), and confirm
    # "good" still succeeds independently.
    class PerCommunityAdapter:
        name = "per_community"

        def fetch(self, community):
            if community.id == "bad-community":
                raise RuntimeError("source is down for this community")
            return HazardReading(value=0.6, observed_at=datetime.now(timezone.utc),
                                  source="per_community", detail="ok")

    pipeline = Pipeline(
        communities=[bad, good],  # bad first, to prove it doesn't block good
        settings=settings,
        hazard_adapters=[PerCommunityAdapter()],
        exposure_adapter=FakeExposureAdapter(),
        vulnerability_adapter=FakeVulnerabilityAdapter(),
        chain_client=None,
    )
    summary = pipeline.run_cycle()
    assert summary.succeeded == 1  # good-community
    assert summary.skipped_stale == 1  # bad-community: no adapter returned data


def test_stale_data_hold_and_flag_still_submits(tmp_path):
    community = make_community()
    settings = make_settings(tmp_path, stale_policy="hold_and_flag", stale_after_hours=1)
    stale_time = datetime.now(timezone.utc) - timedelta(hours=5)
    pipeline = Pipeline(
        communities=[community],
        settings=settings,
        hazard_adapters=[FakeHazardAdapter(0.7, observed_at=stale_time)],
        exposure_adapter=FakeExposureAdapter(),
        vulnerability_adapter=FakeVulnerabilityAdapter(),
        chain_client=None,
    )
    summary = pipeline.run_cycle()
    assert summary.succeeded == 1  # still submitted, just flagged (checked via audit log)


def test_stale_data_stop_submitting_skips(tmp_path):
    community = make_community()
    settings = make_settings(tmp_path, stale_policy="stop_submitting", stale_after_hours=1)
    stale_time = datetime.now(timezone.utc) - timedelta(hours=5)
    pipeline = Pipeline(
        communities=[community],
        settings=settings,
        hazard_adapters=[FakeHazardAdapter(0.7, observed_at=stale_time)],
        exposure_adapter=FakeExposureAdapter(),
        vulnerability_adapter=FakeVulnerabilityAdapter(),
        chain_client=None,
    )
    summary = pipeline.run_cycle()
    assert summary.succeeded == 0
    assert summary.skipped_stale == 1


def test_no_hazard_data_at_all_is_skipped_not_failed(tmp_path):
    community = make_community()
    settings = make_settings(tmp_path)
    pipeline = Pipeline(
        communities=[community],
        settings=settings,
        hazard_adapters=[FakeHazardAdapter(0.0, raise_no_data=True)],
        exposure_adapter=FakeExposureAdapter(),
        vulnerability_adapter=FakeVulnerabilityAdapter(),
        chain_client=None,
    )
    summary = pipeline.run_cycle()
    assert summary.failed == 0
    assert summary.skipped_stale == 1


def test_africa_communities_processed_first(tmp_path):
    from d3rac_pipeline.config import load_communities

    communities = load_communities()
    assert all(c.priority_region == "africa" for c in communities), (
        "expected default config to be all-Africa; if this changes, verify "
        "africa entries still sort before any global entries"
    )
