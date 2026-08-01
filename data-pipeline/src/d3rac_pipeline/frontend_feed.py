"""
FR-9 — Frontend read-path.

Writes the pipeline's computed risk data to a static JSON file the
frontend fetches instead of its hardcoded COMMUNITIES mock array (see
frontend/src/lib/dataFeed.ts). This is an interim bridge: the "real"
long-term read-path is the frontend reading RiskRegistry.getCommunity
directly once Hub/RiskRegistry are deployed (see docs/data-pipeline-srs.md
FR-9's own note that this isn't required yet). Until then, this file is
how "the frontend reads from the pipeline" without needing a deployed
contract or a live backend server.

Output shape matches frontend/src/lib/riskModel.ts's `Community` interface
minus `fundedMilestones`/`totalMilestones` (funding-milestone data comes
from DisbursementController, a different contract this pipeline has
nothing to do with) — the frontend merges those fields in locally. See
dataFeed.ts for the merge logic.
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path

from .config import Community
from .state_store import StateStore
from .fixed_point import from_fixed_point


@dataclass
class FrontendCommunityRow:
    id: str
    name: str
    region: str
    hazard: float
    exposure: float
    vulnerability: float
    lastUpdated: str  # ISO 8601 UTC
    hazardSource: str
    stale: bool


@dataclass
class FrontendFeed:
    generatedAt: str
    communities: list


def build_frontend_feed(
    communities: list[Community],
    state_store: StateStore,
    stale_after_hours: float,
    last_hazard_sources: dict | None = None,
) -> FrontendFeed:
    """Reads each community's last-submitted (or dry-run-computed) H/E/V
    from the state store and shapes it for the frontend. Communities with
    no submission yet fall back to their static config exposure/
    vulnerability and a hazard of 0.0, clearly marked stale=True, rather
    than being omitted — so the frontend always gets a complete, valid
    list even before the pipeline's first successful cycle.
    """
    last_hazard_sources = last_hazard_sources or {}
    rows = []
    now = datetime.now(timezone.utc)

    for community in communities:
        last = state_store.get_last_submission(community.id)
        if last is not None:
            hazard = from_fixed_point(last.hazard)
            exposure = from_fixed_point(last.exposure)
            vulnerability = from_fixed_point(last.vulnerability)
            last_updated = last.submitted_at
        else:
            hazard = 0.0
            exposure = community.exposure
            vulnerability = community.vulnerability
            last_updated = now

        age_hours = (now - last_updated).total_seconds() / 3600.0
        stale = last is None or age_hours > stale_after_hours

        rows.append(
            FrontendCommunityRow(
                id=community.id,
                name=community.name,
                region=community.region,
                hazard=round(hazard, 4),
                exposure=round(exposure, 4),
                vulnerability=round(vulnerability, 4),
                lastUpdated=last_updated.isoformat(),
                hazardSource=last_hazard_sources.get(community.id, "none"),
                stale=stale,
            )
        )

    return FrontendFeed(generatedAt=now.isoformat(), communities=rows)


def write_frontend_feed(feed: FrontendFeed, *output_paths: str) -> None:
    """Writes the same feed to one or more paths — typically both
    data-pipeline/output/communities.json (this repo's own record) and
    frontend/public/data/communities.json (what Vite actually serves).
    Writing to both keeps them from silently drifting apart."""
    payload = {
        "generatedAt": feed.generatedAt,
        "communities": [asdict(row) for row in feed.communities],
    }
    for path_str in output_paths:
        path = Path(path_str)
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            json.dump(payload, f, indent=2)
            f.write("\n")
