"""
GDACS (Global Disaster Alert and Coordination System) — a UN OCHA / EC
Joint Research Centre system that issues green/orange/red alert-level
events (earthquakes, tropical cyclones, floods, volcanoes, droughts)
combining satellite and model data, with population-exposure estimates
already baked into the alert level. Useful as a cross-check against the
pipeline's own per-source adapters, since GDACS' alert level already
reflects an independent severity/exposure judgement.

Docs: https://www.gdacs.org/Knowledge/gdacs_web_services.aspx
No API key required.
"""

from __future__ import annotations

import logging

import requests

from ..config import Community
from .base import HazardAdapter, HazardReading, utcnow

logger = logging.getLogger(__name__)

GDACS_EVENTS_URL = "https://www.gdacs.org/gdacsapi/api/events/geteventlist/SEARCH"

ALERT_LEVEL_SEVERITY = {
    "Green": 0.25,
    "Orange": 0.6,
    "Red": 0.95,
}


class GdacsAlertsAdapter(HazardAdapter):
    name = "gdacs_alerts"

    def __init__(self, session: requests.Session | None = None):
        self.session = session or requests.Session()

    def fetch(self, community: Community) -> HazardReading:
        min_lon, min_lat, max_lon, max_lat = community.bbox
        params = {
            "bbox": f"{min_lon},{min_lat},{max_lon},{max_lat}",
            "alertlevel": "green;orange;red",
        }
        resp = self.session.get(GDACS_EVENTS_URL, params=params, timeout=20)
        resp.raise_for_status()
        data = resp.json()

        features = data.get("features", []) if isinstance(data, dict) else (data or [])

        if not features:
            return HazardReading(
                value=0.0,
                observed_at=utcnow(),
                source=self.name,
                detail="No active GDACS alerts in bbox.",
            )

        best_severity = 0.0
        best_title = ""
        latest_ts = None
        for feat in features:
            props = feat.get("properties", feat)
            level = props.get("alertlevel", "Green")
            severity = ALERT_LEVEL_SEVERITY.get(level, 0.25)
            if severity > best_severity:
                best_severity = severity
                best_title = props.get("eventname") or props.get("name") or "unnamed GDACS event"
            ts = props.get("todate") or props.get("fromdate")
            parsed = _parse_gdacs_date(ts)
            if parsed and (latest_ts is None or parsed > latest_ts):
                latest_ts = parsed

        return HazardReading(
            value=min(1.0, best_severity),
            observed_at=latest_ts or utcnow(),
            source=self.name,
            detail=f"{len(features)} GDACS alert(s) in bbox; highest severity: {best_title}.",
        )


def _parse_gdacs_date(value):
    if not value:
        return None
    from datetime import datetime, timezone

    # GDACS timestamps are UTC but arrive without a timezone suffix.
    # Every other adapter (satellite_fire, eonet_events) returns
    # timezone-*aware* UTC datetimes; pipeline.py's _is_stale() does
    # `datetime.now(timezone.utc) - observed_at`, which raises
    # `TypeError: can't subtract offset-naive and offset-aware
    # datetimes` the moment a naive datetime reaches it. Since
    # hazard_aggregator.combine() always propagates the *driving*
    # reading's observed_at untouched (see combine(), both "max" and
    # "weighted_mean" strategies), this crashed real community
    # processing specifically whenever GDACS was the most severe/
    # driving source for that cycle -- i.e. exactly the "live disaster
    # in progress" case this pipeline exists to handle correctly.
    for fmt in ("%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S"):
        try:
            return datetime.strptime(value[:19], fmt).replace(tzinfo=timezone.utc)
        except ValueError:
            continue
    return None
