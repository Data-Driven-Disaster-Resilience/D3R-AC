"""
Seismic hazard via the USGS Earthquake Catalog (FDSN event web service).

Docs: https://earthquake.usgs.gov/fdsnws/event/1/
No API key required. Global coverage, so this source works for both
Africa-priority and global communities without modification.
"""

from __future__ import annotations

import logging
from datetime import datetime, timedelta, timezone

import requests

from ..config import Community
from .base import HazardAdapter, HazardReading, utcnow

logger = logging.getLogger(__name__)

USGS_URL = "https://earthquake.usgs.gov/fdsnws/event/1/query"
LOOKBACK = timedelta(days=7)

# Magnitude 7.0 is treated as the top of the normalized scale — the
# Richter-equivalent scale is logarithmic and effectively unbounded, but a
# hazard *input* to a [0,1] risk model needs a ceiling; a domain expert
# should revisit this if the pipeline is deployed somewhere with a very
# different seismic baseline than Nigeria's monitored communities.
MAGNITUDE_SATURATION = 7.0


class SeismicUSGSAdapter(HazardAdapter):
    name = "seismic_usgs"

    def __init__(self, session: requests.Session | None = None):
        self.session = session or requests.Session()

    def fetch(self, community: Community) -> HazardReading:
        min_lon, min_lat, max_lon, max_lat = community.bbox
        params = {
            "format": "geojson",
            "starttime": (utcnow() - LOOKBACK).strftime("%Y-%m-%dT%H:%M:%S"),
            "minlatitude": min_lat,
            "maxlatitude": max_lat,
            "minlongitude": min_lon,
            "maxlongitude": max_lon,
            "orderby": "time",
        }
        resp = self.session.get(USGS_URL, params=params, timeout=20)
        resp.raise_for_status()
        data = resp.json()
        features = data.get("features", [])

        if not features:
            return HazardReading(
                value=0.0,
                observed_at=utcnow(),
                source=self.name,
                detail=f"No recorded seismic events in bbox in the last {LOOKBACK.days} days.",
            )

        max_mag = 0.0
        latest_ts = None
        for feat in features:
            props = feat.get("properties", {})
            mag = props.get("mag") or 0.0
            max_mag = max(max_mag, mag)
            ts_ms = props.get("time")
            if ts_ms:
                ts = datetime.fromtimestamp(ts_ms / 1000, tz=timezone.utc)
                if latest_ts is None or ts > latest_ts:
                    latest_ts = ts

        value = min(1.0, max(0.0, max_mag) / MAGNITUDE_SATURATION)

        return HazardReading(
            value=value,
            observed_at=latest_ts or utcnow(),
            source=self.name,
            detail=f"{len(features)} seismic event(s), max magnitude {max_mag:.1f}.",
        )
