"""
Multi-hazard event catalog via NASA EONET (Earth Observatory Natural
Event Tracker) — floods, severe storms, volcanoes, and other categories,
each event tied to a satellite/agency-verified source.

Docs: https://eonet.gsfc.nasa.gov/docs/v3
No API key required.

EONET events are points/polygons with a category and a list of dated
geometries (a storm's track over time, for example). We take each
community's most recent in-bbox geometry per open event and score by
category severity, since EONET doesn't provide a normalized intensity
figure the way FIRMS (FRP) or USGS (magnitude) do.
"""

from __future__ import annotations

import logging

import requests

from ..config import Community
from .base import HazardAdapter, HazardReading, utcnow

logger = logging.getLogger(__name__)

EONET_URL = "https://eonet.gsfc.nasa.gov/api/v3/events"
DAYS = 10  # only consider events with activity in the last N days
STATUS = "open"

# Category severity weights — deliberately conservative defaults; a
# disaster-response domain expert should recalibrate these (open
# decision, docs/data-pipeline-srs.md §8). Categories not listed default
# to 0.5.
CATEGORY_SEVERITY = {
    "severeStorms": 0.85,
    "floods": 0.85,
    "volcanoes": 0.8,
    "wildfires": 0.75,
    "drought": 0.6,
    "landslides": 0.7,
    "dustHaze": 0.3,
    "seaLakeIce": 0.2,
    "snow": 0.3,
    "tempExtremes": 0.55,
    "waterColor": 0.15,
}


class EonetEventsAdapter(HazardAdapter):
    name = "eonet_events"

    def __init__(self, session: requests.Session | None = None):
        self.session = session or requests.Session()

    def fetch(self, community: Community) -> HazardReading:
        params = {"status": STATUS, "days": DAYS, "limit": 200}
        resp = self.session.get(EONET_URL, params=params, timeout=20)
        resp.raise_for_status()
        data = resp.json()
        events = data.get("events", [])

        min_lon, min_lat, max_lon, max_lat = community.bbox

        matches = []
        for event in events:
            categories = [c.get("id") for c in event.get("categories", [])]
            for geom in event.get("geometry", []):
                coords = geom.get("coordinates")
                point = _extract_point(geom.get("type"), coords)
                if point is None:
                    continue
                lon, lat = point
                if min_lon <= lon <= max_lon and min_lat <= lat <= max_lat:
                    matches.append((event, categories, geom.get("date")))
                    break  # one match per event is enough

        if not matches:
            return HazardReading(
                value=0.0,
                observed_at=utcnow(),
                source=self.name,
                detail=f"No open EONET events with activity in bbox in the last {DAYS} days.",
            )

        best_severity = 0.0
        best_event_title = ""
        latest_date = None
        for event, categories, date_str in matches:
            severity = max((CATEGORY_SEVERITY.get(c, 0.5) for c in categories), default=0.5)
            if severity > best_severity:
                best_severity = severity
                best_event_title = event.get("title", "unnamed event")
            ts = _parse_iso(date_str)
            if ts and (latest_date is None or ts > latest_date):
                latest_date = ts

        return HazardReading(
            value=min(1.0, best_severity),
            observed_at=latest_date or utcnow(),
            source=self.name,
            detail=f"{len(matches)} open EONET event(s) in bbox; most severe: {best_event_title}.",
        )


def _extract_point(geom_type: str | None, coords):
    if coords is None:
        return None
    if geom_type == "Point":
        return coords[0], coords[1]
    if geom_type == "Polygon":
        # first coordinate of the outer ring is a reasonable representative point
        try:
            return coords[0][0][0], coords[0][0][1]
        except (IndexError, TypeError):
            return None
    return None


def _parse_iso(date_str):
    if not date_str:
        return None
    from datetime import datetime

    try:
        return datetime.fromisoformat(date_str.replace("Z", "+00:00"))
    except ValueError:
        return None
