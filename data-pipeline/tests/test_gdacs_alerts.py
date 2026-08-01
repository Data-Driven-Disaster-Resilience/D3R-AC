from datetime import datetime, timezone

import pytest
import responses

from d3rac_pipeline.adapters.gdacs_alerts import (
    GDACS_EVENTS_URL,
    GdacsAlertsAdapter,
    _parse_gdacs_date,
)
from d3rac_pipeline.config import Community
from d3rac_pipeline.pipeline import Pipeline


def community():
    return Community(
        id="test-community",
        name="Test Community",
        region="Test Region",
        priority_region="africa",
        bbox=[10.0, 10.0, 12.0, 12.0],
        exposure=0.5,
        vulnerability=0.5,
    )


def test_parsed_gdacs_dates_are_utc_aware():
    # Regression test: _parse_gdacs_date previously returned naive
    # datetimes while every other adapter (satellite_fire, seismic_usgs,
    # eonet_events) returns UTC-aware ones. pipeline.py's _is_stale()
    # does `datetime.now(timezone.utc) - observed_at`, which raises
    # TypeError the instant a naive datetime reaches it -- this crashed
    # a community's processing for that cycle specifically whenever
    # GDACS was the driving/most-severe hazard source, i.e. exactly the
    # live-disaster-in-progress case this pipeline exists to handle.
    ts = _parse_gdacs_date("2026-07-28T14:30:00")
    assert ts is not None
    assert ts.tzinfo is not None
    # Must not raise -- this is the exact operation pipeline.py performs.
    datetime.now(timezone.utc) - ts


def test_parse_gdacs_date_alternate_format_also_aware():
    ts = _parse_gdacs_date("2026-07-28 14:30:00")
    assert ts is not None
    assert ts.tzinfo is not None


def test_parse_gdacs_date_handles_missing_value():
    assert _parse_gdacs_date(None) is None
    assert _parse_gdacs_date("") is None


@responses.activate
def test_fetch_returns_aware_datetime_and_correct_severity():
    responses.add(
        responses.GET,
        GDACS_EVENTS_URL,
        json={
            "features": [
                {
                    "properties": {
                        "alertlevel": "Red",
                        "eventname": "Test Cyclone",
                        "todate": "2026-07-28T14:30:00",
                    }
                }
            ]
        },
        status=200,
    )
    adapter = GdacsAlertsAdapter()
    reading = adapter.fetch(community())

    assert reading.value == pytest.approx(0.95)
    assert reading.observed_at.tzinfo is not None
    assert "Test Cyclone" in reading.detail
    # The real-world failure mode: staleness check must not raise.
    datetime.now(timezone.utc) - reading.observed_at


@responses.activate
def test_gdacs_driven_reading_survives_full_pipeline_staleness_check(tmp_path):
    """End-to-end regression: a GDACS-only, Red-alert reading must make
    it through Pipeline._is_stale() without raising, in dry-run mode,
    the same as any other source. This is the actual crash path that
    existed before the fix (TypeError inside run_cycle, per-community
    isolated but still a silent failure to submit for the affected
    community)."""
    from d3rac_pipeline.audit_log import AuditLog
    from d3rac_pipeline.config import ChainConfig, PipelineSettings, StaleConfig
    from d3rac_pipeline.state_store import StateStore

    responses.add(
        responses.GET,
        GDACS_EVENTS_URL,
        json={
            "features": [
                {
                    "properties": {
                        "alertlevel": "Red",
                        "eventname": "Test Flood",
                        "todate": "2026-07-28T14:30:00",
                    }
                }
            ]
        },
        status=200,
    )

    class ZeroAdapter:
        name = "zero"

        def fetch(self, community):
            from d3rac_pipeline.adapters.base import HazardReading, utcnow
            return HazardReading(value=0.0, observed_at=utcnow(), source=self.name, detail="none")

    class StaticAdapter:
        def fetch(self, community):
            return 0.5

    settings = PipelineSettings(
        state_db_path=str(tmp_path / "state.db"),
        audit_log_path=str(tmp_path / "audit.log"),
        hazard_combine_strategy="max",
        hazard_weights={},
        stale=StaleConfig(stale_after_hours=48, stale_policy="hold_and_flag"),
        chain=ChainConfig(network="dry-run", hub_address="", full_node=""),
        log_level="INFO",
    )

    pipeline = Pipeline(
        communities=[community()],
        settings=settings,
        hazard_adapters=[GdacsAlertsAdapter(), ZeroAdapter()],
        exposure_adapter=StaticAdapter(),
        vulnerability_adapter=StaticAdapter(),
        chain_client=None,  # dry-run
        state_store=StateStore(str(tmp_path / "state.db")),
        audit_log=AuditLog(str(tmp_path / "audit.log")),
    )

    summary = pipeline.run_cycle()
    assert summary.failed == 0, f"community processing failed: {summary.failures}"
    assert summary.succeeded == 1
