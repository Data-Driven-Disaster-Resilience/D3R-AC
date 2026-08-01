#!/usr/bin/env python3
"""
Runs one D3R·AC data-pipeline refresh cycle.

Usage:
    python scripts/run_cycle.py                 # dry-run (no chain submission, needs no secrets)
    python scripts/run_cycle.py --submit         # submits on-chain (needs .env configured)

Intended to be invoked on a schedule (cron, CI scheduled workflow, or any
job scheduler) — see docs/data-pipeline-srs.md's "Refresh cycle" definition.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

# Allow running directly from scripts/ without installing the package.
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from dotenv import load_dotenv

from d3rac_pipeline.adapters import (
    EonetEventsAdapter,
    GdacsAlertsAdapter,
    SatelliteFireAdapter,
    SeismicUSGSAdapter,
    StaticExposureAdapter,
    StaticVulnerabilityAdapter,
)
from d3rac_pipeline.chain_client import ChainClient
from d3rac_pipeline.config import load_communities, load_settings
from d3rac_pipeline.frontend_feed import build_frontend_feed, write_frontend_feed
from d3rac_pipeline.logging_setup import configure_logging
from d3rac_pipeline.pipeline import Pipeline


def main() -> int:
    load_dotenv()

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--submit", action="store_true",
                         help="Actually submit to chain. Without this flag, runs in dry-run mode.")
    args = parser.parse_args()

    settings = load_settings()
    configure_logging(settings.log_level)

    communities = load_communities()

    hazard_adapters = [
        SatelliteFireAdapter(),
        SeismicUSGSAdapter(),
        EonetEventsAdapter(),
        GdacsAlertsAdapter(),
    ]

    chain_client = None
    if args.submit:
        chain_client = ChainClient(
            full_node=settings.chain.full_node,
            hub_address=settings.chain.hub_address,
            feeder_private_key_hex=settings.chain.feeder_private_key,
        )

    pipeline = Pipeline(
        communities=communities,
        settings=settings,
        hazard_adapters=hazard_adapters,
        exposure_adapter=StaticExposureAdapter(),
        vulnerability_adapter=StaticVulnerabilityAdapter(),
        chain_client=chain_client,
    )

    summary = pipeline.run_cycle()

    # FR-9: keep the frontend's read-path up to date every cycle, dry-run
    # or --submit alike, so `npm run dev` always reflects the pipeline's
    # latest computed values, not just live-chain deployments.
    feed = build_frontend_feed(
        communities=communities,
        state_store=pipeline.state,
        stale_after_hours=settings.stale.stale_after_hours,
        last_hazard_sources=pipeline.last_hazard_sources,
    )
    repo_root = Path(__file__).resolve().parents[2]
    write_frontend_feed(
        feed,
        str(repo_root / "data-pipeline" / "output" / "communities.json"),
        str(repo_root / "frontend" / "public" / "data" / "communities.json"),
    )

    print(
        f"\nCycle {summary.cycle_id}: {summary.succeeded}/{summary.total_communities} succeeded, "
        f"{summary.failed} failed, {summary.skipped_unchanged} unchanged, "
        f"{summary.skipped_stale} stale-skipped."
    )
    if summary.failures:
        print("Failures:")
        for f in summary.failures:
            print(f"  - {f['community_id']}: {f['detail']}")

    return 1 if summary.failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
