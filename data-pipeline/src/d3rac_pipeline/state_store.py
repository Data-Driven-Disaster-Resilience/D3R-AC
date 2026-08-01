"""
Local state store — FR-2 (skip redundant on-chain writes when H/E/V are
all unchanged) and NFR-2 (idempotency: a re-run after a partial failure
must not double-submit for communities that already succeeded this
cycle).

SQLite, single file, no server required — appropriate for a pipeline that
runs as a scheduled job (cron/Lambda-equivalent), not a long-lived service.
"""

from __future__ import annotations

import sqlite3
from contextlib import closing
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path


@dataclass
class LastSubmission:
    hazard: int
    exposure: int
    vulnerability: int
    submitted_at: datetime
    cycle_id: str


class StateStore:
    def __init__(self, db_path: str):
        self.db_path = db_path
        Path(db_path).parent.mkdir(parents=True, exist_ok=True)
        self._init_schema()

    def _connect(self):
        return sqlite3.connect(self.db_path)

    def _init_schema(self):
        with closing(self._connect()) as conn:
            conn.execute(
                """
                CREATE TABLE IF NOT EXISTS last_submission (
                    community_id TEXT PRIMARY KEY,
                    hazard INTEGER NOT NULL,
                    exposure INTEGER NOT NULL,
                    vulnerability INTEGER NOT NULL,
                    submitted_at TEXT NOT NULL,
                    cycle_id TEXT NOT NULL
                )
                """
            )
            conn.execute(
                """
                CREATE TABLE IF NOT EXISTS cycle_progress (
                    cycle_id TEXT NOT NULL,
                    community_id TEXT NOT NULL,
                    status TEXT NOT NULL,   -- 'succeeded' | 'failed' | 'skipped_unchanged' | 'skipped_stale'
                    detail TEXT,
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY (cycle_id, community_id)
                )
                """
            )
            conn.commit()

    def get_last_submission(self, community_id: str) -> LastSubmission | None:
        with closing(self._connect()) as conn:
            row = conn.execute(
                "SELECT hazard, exposure, vulnerability, submitted_at, cycle_id "
                "FROM last_submission WHERE community_id = ?",
                (community_id,),
            ).fetchone()
        if row is None:
            return None
        return LastSubmission(
            hazard=row[0],
            exposure=row[1],
            vulnerability=row[2],
            submitted_at=datetime.fromisoformat(row[3]),
            cycle_id=row[4],
        )

    def record_submission(
        self, community_id: str, hazard: int, exposure: int, vulnerability: int, cycle_id: str
    ) -> None:
        now = datetime.now(timezone.utc).isoformat()
        with closing(self._connect()) as conn:
            conn.execute(
                """
                INSERT INTO last_submission (community_id, hazard, exposure, vulnerability, submitted_at, cycle_id)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(community_id) DO UPDATE SET
                    hazard=excluded.hazard,
                    exposure=excluded.exposure,
                    vulnerability=excluded.vulnerability,
                    submitted_at=excluded.submitted_at,
                    cycle_id=excluded.cycle_id
                """,
                (community_id, hazard, exposure, vulnerability, now, cycle_id),
            )
            conn.commit()

    # --- Idempotency across a crashed/resumed cycle (NFR-2) ---

    def mark_cycle_status(self, cycle_id: str, community_id: str, status: str, detail: str = "") -> None:
        now = datetime.now(timezone.utc).isoformat()
        with closing(self._connect()) as conn:
            conn.execute(
                """
                INSERT INTO cycle_progress (cycle_id, community_id, status, detail, updated_at)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(cycle_id, community_id) DO UPDATE SET
                    status=excluded.status, detail=excluded.detail, updated_at=excluded.updated_at
                """,
                (cycle_id, community_id, status, detail, now),
            )
            conn.commit()

    def already_succeeded_this_cycle(self, cycle_id: str, community_id: str) -> bool:
        with closing(self._connect()) as conn:
            row = conn.execute(
                "SELECT status FROM cycle_progress WHERE cycle_id = ? AND community_id = ?",
                (cycle_id, community_id),
            ).fetchone()
        return row is not None and row[0] == "succeeded"
