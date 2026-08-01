"""
NFR-3 — Auditability: log, off-chain, which source data produced each
submitted value, so a questioned figure can be traced back to its input.

NFR-4 — Observability: a per-cycle summary an operator can actually see.

Deliberately simple (append-only JSON-lines file) rather than a logging
framework integration, so it works the same whether this pipeline runs as
a cron job, a container, or a Lambda-equivalent. Point a log shipper at
the file in production if you want it in a central log system.
"""

from __future__ import annotations

import json
import logging
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

logger = logging.getLogger(__name__)


@dataclass
class SubmissionAuditRecord:
    cycle_id: str
    community_id: str
    hazard_value: float
    hazard_source: str
    hazard_detail: str
    exposure_value: float
    vulnerability_value: float
    hazard_fixed_point: int
    exposure_fixed_point: int
    vulnerability_fixed_point: int
    tx_id: Optional[str]
    timestamp: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())


@dataclass
class CycleSummary:
    cycle_id: str
    started_at: str
    finished_at: str
    total_communities: int
    succeeded: int
    failed: int
    skipped_unchanged: int
    skipped_stale: int
    failures: list  # list of {community_id, stage, detail}


class AuditLog:
    def __init__(self, path: str):
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)

    def record_submission(self, record: SubmissionAuditRecord) -> None:
        self._append({"type": "submission", **asdict(record)})

    def record_cycle_summary(self, summary: CycleSummary) -> None:
        self._append({"type": "cycle_summary", **asdict(summary)})
        logger.info(
            "cycle %s complete: %d/%d succeeded, %d failed, %d skipped (unchanged), "
            "%d skipped (stale)",
            summary.cycle_id,
            summary.succeeded,
            summary.total_communities,
            summary.failed,
            summary.skipped_unchanged,
            summary.skipped_stale,
        )

    def _append(self, obj: dict) -> None:
        with open(self.path, "a", encoding="utf-8") as f:
            f.write(json.dumps(obj, default=str) + "\n")
