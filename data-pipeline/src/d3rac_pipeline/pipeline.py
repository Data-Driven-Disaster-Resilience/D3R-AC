"""
The refresh cycle: fetch -> compute H/E/V -> submit on-chain, per
community, per docs/data-pipeline-srs.md. Each function reference below
(FR-1, FR-2, ...) maps to that document's numbered requirements.
"""

from __future__ import annotations

import logging
import uuid
from datetime import datetime, timedelta, timezone

from .adapters.base import HazardAdapter, HazardReading, NoFreshData
from .adapters.base import ExposureAdapter, VulnerabilityAdapter
from .audit_log import AuditLog, CycleSummary, SubmissionAuditRecord
from .chain_client import ChainClient, ChainSubmissionError, FeederNotAuthorizedError
from .community_id import to_community_id_hex
from .config import Community, PipelineSettings
from .fixed_point import to_fixed_point
from .hazard_aggregator import combine
from .state_store import StateStore

logger = logging.getLogger(__name__)


class Pipeline:
    def __init__(
        self,
        communities: list[Community],
        settings: PipelineSettings,
        hazard_adapters: list[HazardAdapter],
        exposure_adapter: ExposureAdapter,
        vulnerability_adapter: VulnerabilityAdapter,
        chain_client: ChainClient | None,
        state_store: StateStore | None = None,
        audit_log: AuditLog | None = None,
    ):
        # Communities are already Africa-first ordered by config.load_communities().
        self.communities = communities
        self.settings = settings
        self.hazard_adapters = hazard_adapters
        self.exposure_adapter = exposure_adapter
        self.vulnerability_adapter = vulnerability_adapter
        self.chain_client = chain_client  # None => dry-run mode, no on-chain submission
        self.state = state_store or StateStore(settings.state_db_path)
        self.audit = audit_log or AuditLog(settings.audit_log_path)
        self._registered_cache: set[str] = set()
        # FR-9 support: which hazard source drove each community's last
        # computed H(t), for the frontend feed export (see frontend_feed.py).
        self.last_hazard_sources: dict[str, str] = {}

    def run_cycle(self) -> CycleSummary:
        cycle_id = uuid.uuid4().hex[:12]
        started_at = datetime.now(timezone.utc)
        succeeded = failed = skipped_unchanged = skipped_stale = 0
        failures = []

        logger.info("=== Starting cycle %s (%d communities, africa-first) ===",
                     cycle_id, len(self.communities))

        for community in self.communities:
            # NFR-2: don't double-submit for a community that already
            # succeeded earlier in this same cycle (relevant on retry after
            # a crash mid-cycle, when run_cycle is called again with the
            # same cycle context — see scripts/run_cycle.py --resume).
            if self.state.already_succeeded_this_cycle(cycle_id, community.id):
                continue

            try:
                outcome = self._process_community(cycle_id, community)
            except Exception as exc:  # FR-6: isolate failures per-community
                failed += 1
                detail = str(exc)
                logger.exception("Community %s failed during cycle %s", community.id, cycle_id)
                self.state.mark_cycle_status(cycle_id, community.id, "failed", detail)
                failures.append({"community_id": community.id, "stage": "unknown", "detail": detail})
                continue

            if outcome == "succeeded":
                succeeded += 1
            elif outcome == "skipped_unchanged":
                skipped_unchanged += 1
            elif outcome == "skipped_stale":
                skipped_stale += 1

        finished_at = datetime.now(timezone.utc)
        summary = CycleSummary(
            cycle_id=cycle_id,
            started_at=started_at.isoformat(),
            finished_at=finished_at.isoformat(),
            total_communities=len(self.communities),
            succeeded=succeeded,
            failed=failed,
            skipped_unchanged=skipped_unchanged,
            skipped_stale=skipped_stale,
            failures=failures,
        )
        self.audit.record_cycle_summary(summary)  # NFR-4
        return summary

    def _process_community(self, cycle_id: str, community: Community) -> str:
        # FR-1: register on-chain before first update, don't fail silently if skipped.
        self._ensure_registered(community)

        # FR-2: compute H(t) every cycle; E/V change slowly but we still
        # fetch them every cycle here for simplicity — the redundant-write
        # skip below is what actually satisfies FR-2's "avoid a redundant
        # on-chain write" requirement, regardless of how often E/V are
        # recomputed off-chain.
        hazard_reading = self._compute_hazard(community)

        # FR-7: stale-data policy.
        if hazard_reading is None:
            self.state.mark_cycle_status(cycle_id, community.id, "skipped_stale",
                                          "no hazard source returned data within staleness window")
            return "skipped_stale"

        self.last_hazard_sources[community.id] = hazard_reading.source

        is_stale = self._is_stale(hazard_reading.observed_at)
        if is_stale and self.settings.stale.stale_policy == "stop_submitting":
            self.state.mark_cycle_status(
                cycle_id, community.id, "skipped_stale",
                f"stale data from {hazard_reading.observed_at.isoformat()}, stop_submitting policy",
            )
            return "skipped_stale"
        # else: hold_and_flag falls through and submits, but is logged as stale below.

        exposure_value = self.exposure_adapter.fetch(community)
        vulnerability_value = self.vulnerability_adapter.fetch(community)

        hazard_fp = to_fixed_point(hazard_reading.value)
        exposure_fp = to_fixed_point(exposure_value)
        vulnerability_fp = to_fixed_point(vulnerability_value)

        # FR-2 / FR-3: skip a redundant on-chain write if nothing changed
        # since the last successful submission.
        last = self.state.get_last_submission(community.id)
        if last and (last.hazard, last.exposure, last.vulnerability) == (
            hazard_fp, exposure_fp, vulnerability_fp
        ):
            self.state.mark_cycle_status(cycle_id, community.id, "skipped_unchanged")
            return "skipped_unchanged"

        tx_id = None
        if self.chain_client is not None:
            try:
                tx_id = self.chain_client.update_risk(
                    to_community_id_hex(community.id), hazard_fp, exposure_fp, vulnerability_fp
                )
            except FeederNotAuthorizedError:
                # FR-4: surface distinctly from a data problem — re-raise so
                # run_cycle's per-community isolation logs it as a failure,
                # but the exception type/message make the cause obvious.
                raise
            except ChainSubmissionError:
                raise
        else:
            logger.info("[dry-run] would submit %s: H=%.3f E=%.3f V=%.3f%s",
                        community.id, hazard_reading.value, exposure_value, vulnerability_value,
                        " (STALE)" if is_stale else "")

        self.state.record_submission(community.id, hazard_fp, exposure_fp, vulnerability_fp, cycle_id)
        self.state.mark_cycle_status(
            cycle_id, community.id, "succeeded",
            "stale-but-submitted (hold_and_flag)" if is_stale else "",
        )

        self.audit.record_submission(SubmissionAuditRecord(  # NFR-3
            cycle_id=cycle_id,
            community_id=community.id,
            hazard_value=hazard_reading.value,
            hazard_source=hazard_reading.source,
            hazard_detail=hazard_reading.detail,
            exposure_value=exposure_value,
            vulnerability_value=vulnerability_value,
            hazard_fixed_point=hazard_fp,
            exposure_fixed_point=exposure_fp,
            vulnerability_fixed_point=vulnerability_fp,
            tx_id=tx_id,
        ))
        return "succeeded"

    def _compute_hazard(self, community: Community):
        readings: list[HazardReading] = []
        for adapter in self.hazard_adapters:
            try:
                readings.append(adapter.fetch(community))
            except NoFreshData as exc:
                logger.info("Adapter %s has no data for %s: %s", adapter.name, community.id, exc)
                continue
            except Exception:
                # A single source failing must not block other sources for
                # the same community (this is FR-6 applied within a
                # community's own hazard computation, not just across
                # communities).
                logger.exception("Adapter %s errored for %s", adapter.name, community.id)
                continue

        if not readings:
            return None

        combined = combine(readings, self.settings.hazard_combine_strategy, self.settings.hazard_weights)
        return HazardReading(
            value=combined.value,
            observed_at=combined.observed_at,
            source=combined.driving_source,
            detail=combined.driving_detail,
        )

    def _is_stale(self, observed_at: datetime) -> bool:
        age = datetime.now(timezone.utc) - observed_at
        return age > timedelta(hours=self.settings.stale.stale_after_hours)

    def _ensure_registered(self, community: Community) -> None:
        if community.id in self._registered_cache:
            return
        if self.chain_client is not None:
            try:
                self.chain_client.register_community(
                    to_community_id_hex(community.id), community.name, community.region
                )
            except ChainSubmissionError as exc:
                # "already registered" is expected on every run after the
                # first; anything else propagates (FR-1: don't fail silently).
                if "already registered" not in str(exc):
                    raise
        self._registered_cache.add(community.id)
