"""
Satellite active-fire detection via NASA FIRMS (Fire Information for
Resource Management System) — VIIRS/MODIS instruments.

Docs: https://firms.modaps.eosdis.nasa.gov/api/
Requires a free MAP_KEY: https://firms.modaps.eosdis.nasa.gov/api/map_key/

This is the pipeline's most direct "satellite hunter" source: each row
returned is an actual satellite pixel flagged as an active fire, with a
confidence level and FRP (Fire Radiative Power, a proxy for fire
intensity). We normalize per-community hazard as a function of both
detection count and intensity within the community's bbox, not just a
raw count, so one large/intense fire scores similarly to several small
confirmed ones rather than being drowned out by a single low-confidence
pixel.
"""

from __future__ import annotations

import csv
import io
import logging
import os
from datetime import datetime, timezone

import requests

from ..config import Community
from .base import HazardAdapter, HazardReading, NoFreshData, utcnow

logger = logging.getLogger(__name__)

FIRMS_BASE_URL = "https://firms.modaps.eosdis.nasa.gov/api/area/csv"
# VIIRS_SNPP_NRT / VIIRS_NOAA20_NRT / VIIRS_NOAA21_NRT / MODIS_NRT are the
# common near-real-time products; VIIRS has finer spatial resolution
# (375m) than MODIS (1km), which matters for smaller community bboxes.
DEFAULT_SOURCE = "VIIRS_SNPP_NRT"
DAY_RANGE = 1  # most recent 24h of detections

# FRP values above this are treated as "as severe as it gets" for
# normalization purposes (large, intense active fires commonly exceed
# 50 MW; this is a deliberately conservative ceiling, not a scientific
# threshold — tune per deployment if a domain expert wants scaling
# calibrated against a specific FRP distribution).
FRP_SATURATION_MW = 80.0
MAX_CONSIDERED_DETECTIONS = 20  # detections beyond this no longer add hazard


class SatelliteFireAdapter(HazardAdapter):
    name = "satellite_fire"

    def __init__(self, map_key: str | None = None, session: requests.Session | None = None):
        self.map_key = map_key or os.environ.get("NASA_FIRMS_MAP_KEY", "")
        self.session = session or requests.Session()

    def fetch(self, community: Community) -> HazardReading:
        if not self.map_key:
            raise NoFreshData(
                f"[{self.name}] NASA_FIRMS_MAP_KEY not configured; skipping satellite fire "
                f"check for {community.id}"
            )

        min_lon, min_lat, max_lon, max_lat = community.bbox
        area = f"{min_lon},{min_lat},{max_lon},{max_lat}"
        url = f"{FIRMS_BASE_URL}/{self.map_key}/{DEFAULT_SOURCE}/{area}/{DAY_RANGE}"

        resp = self.session.get(url, timeout=20)
        resp.raise_for_status()

        rows = list(csv.DictReader(io.StringIO(resp.text)))
        if not rows:
            return HazardReading(
                value=0.0,
                observed_at=utcnow(),
                source=self.name,
                detail="No active-fire detections in bbox in the last 24h.",
            )

        max_frp = 0.0
        latest_ts = None
        for row in rows:
            try:
                frp = float(row.get("frp", 0.0))
            except (TypeError, ValueError):
                frp = 0.0
            max_frp = max(max_frp, frp)

            ts = _parse_acq_datetime(row.get("acq_date"), row.get("acq_time"))
            if ts and (latest_ts is None or ts > latest_ts):
                latest_ts = ts

        count_component = min(len(rows), MAX_CONSIDERED_DETECTIONS) / MAX_CONSIDERED_DETECTIONS
        intensity_component = min(max_frp, FRP_SATURATION_MW) / FRP_SATURATION_MW
        # Weighted toward intensity: a single very hot detection matters
        # more for response prioritization than many low-confidence ones.
        value = 0.4 * count_component + 0.6 * intensity_component

        return HazardReading(
            value=min(1.0, value),
            observed_at=latest_ts or utcnow(),
            source=self.name,
            detail=f"{len(rows)} active-fire detection(s), max FRP {max_frp:.1f} MW.",
        )


def _parse_acq_datetime(acq_date: str | None, acq_time: str | None):
    if not acq_date:
        return None
    try:
        time_str = (acq_time or "0000").zfill(4)
        return datetime.strptime(f"{acq_date} {time_str}", "%Y-%m-%d %H%M").replace(
            tzinfo=timezone.utc
        )
    except ValueError:
        return None
