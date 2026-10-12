"""
Bridge from the data pipeline (data-pipeline/) to the agent fleet.

The pipeline (NASA FIRMS / USGS / EONET / GDACS adapters -> hazard aggregation) writes a
`communities.json` feed (see data-pipeline/src/d3rac_pipeline/frontend_feed.py). Setting

    D3RAC_PIPELINE_FEED=/path/to/communities.json

makes hazard-ingestor, exposure-scorer and vulnerability-scorer read their H/E/V terms from
that file instead of the built-in sample data -- so the same risk numbers the pipeline submits
on-chain also drive the agents' response planning.

Fail-closed by design, because a triggered community can lead to a fund-release request:
  * feed set but missing/unreadable/malformed  -> no signals (never silently falls back to samples)
  * feed older than D3RAC_PIPELINE_FEED_MAX_AGE_HOURS (default 6) -> no hazard signals
  * rows flagged stale by the pipeline          -> no hazard signal for that community
  * non-finite or out-of-range values           -> that row is skipped
With the variable unset nothing changes: the agents keep using their sample data.
"""
from __future__ import annotations

import json
import logging
import math
import os
from datetime import datetime, timezone
from pathlib import Path

logger = logging.getLogger(__name__)

FEED_ENV = "D3RAC_PIPELINE_FEED"
MAX_AGE_ENV = "D3RAC_PIPELINE_FEED_MAX_AGE_HOURS"
DEFAULT_MAX_AGE_HOURS = 6.0


def feed_path() -> Path | None:
    raw = os.environ.get(FEED_ENV, "").strip()
    return Path(raw) if raw else None


def _unit(value: object) -> float | None:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    v = float(value)
    return v if math.isfinite(v) and 0.0 <= v <= 1.0 else None


def _feed_is_fresh(payload: dict, now: datetime) -> bool:
    try:
        generated = datetime.fromisoformat(payload["generatedAt"])
    except (KeyError, TypeError, ValueError):
        return False
    if generated.tzinfo is None:
        generated = generated.replace(tzinfo=timezone.utc)
    max_age = float(os.environ.get(MAX_AGE_ENV, DEFAULT_MAX_AGE_HOURS))
    return 0.0 <= (now - generated).total_seconds() <= max_age * 3600.0


def load_rows(path: Path | None = None, *, now: datetime | None = None) -> list[dict]:
    """Valid rows from the pipeline feed, each with id/hazard/exposure/vulnerability in [0,1],
    plus `fresh` (bool: the row may drive a hazard signal) and `source`."""
    path = path or feed_path()
    if path is None:
        return []
    now = now or datetime.now(timezone.utc)
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
        communities = payload["communities"]
        if not isinstance(communities, list):
            raise TypeError("'communities' is not a list")
    except (OSError, ValueError, KeyError, TypeError) as exc:
        logger.error("pipeline feed %s unusable (%s); publishing nothing", path, exc)
        return []

    feed_fresh = _feed_is_fresh(payload, now)
    if not feed_fresh:
        logger.warning("pipeline feed %s is missing or stale generatedAt; hazard signals suppressed", path)

    rows: list[dict] = []
    for raw in communities:
        if not isinstance(raw, dict) or not isinstance(raw.get("id"), str):
            continue
        h, e, v = _unit(raw.get("hazard")), _unit(raw.get("exposure")), _unit(raw.get("vulnerability"))
        if h is None or e is None or v is None:
            logger.warning("skipping feed row %r: non-finite or out-of-range value", raw.get("id"))
            continue
        rows.append(
            {
                "community_id": raw["id"],
                "hazard": h,
                "exposure": e,
                "vulnerability": v,
                "fresh": feed_fresh and raw.get("stale") is False,
                "source": f"pipeline:{raw.get('hazardSource', 'unknown')}",
            }
        )
    return rows
