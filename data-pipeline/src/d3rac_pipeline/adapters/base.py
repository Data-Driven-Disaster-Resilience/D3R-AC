"""
FR-8 — Source-agnostic ingestion interface.

Every hazard source (satellite fire detection, seismic feeds, storm/flood
event catalogs, etc.) implements HazardAdapter. The rest of the pipeline
(pipeline.py) only ever talks to this interface, never to a specific
source's API shape — so adding, removing, or swapping a source never
touches FR-1 through FR-7's logic.

Exposure and vulnerability adapters use the same shape (a `fetch(community)
-> float in [0,1] or NoFreshData` contract) even though today's
implementations are static config reads (see static_exposure.py /
static_vulnerability.py) — that keeps the door open for a future adapter
that computes E(c)/V(c) from real geospatial/socioeconomic data (the open
decision in docs/data-pipeline-srs.md §8) without changing pipeline.py.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Optional

from ..config import Community


class NoFreshData(Exception):
    """Raised by an adapter when it has no data within the community's
    bbox at all (as opposed to stale data — see FR-7, handled by
    pipeline.py comparing HazardReading.observed_at to the staleness
    window, not by the adapter itself)."""


@dataclass
class HazardReading:
    value: float  # [0,1], already normalized by the adapter
    observed_at: datetime  # UTC timestamp of the underlying satellite/sensor pass or event
    source: str  # short adapter name, for audit logging (NFR-3)
    detail: str  # human-readable one-liner: what specifically was observed


class HazardAdapter(ABC):
    """One adapter per hazard category/source. `name` must be stable —
    it's used as a key in settings.yaml's hazard_weights and in audit logs."""

    name: str = "unnamed-hazard-adapter"

    @abstractmethod
    def fetch(self, community: Community) -> HazardReading:
        """Return the current best hazard reading for this community's
        bbox. Raise NoFreshData if the source has nothing for this area
        at all. Must NOT raise for "nothing bad is happening right now" —
        that's a HazardReading(value=0.0, ...), not an exception."""
        raise NotImplementedError


class ExposureAdapter(ABC):
    name: str = "unnamed-exposure-adapter"

    @abstractmethod
    def fetch(self, community: Community) -> float:
        raise NotImplementedError


class VulnerabilityAdapter(ABC):
    name: str = "unnamed-vulnerability-adapter"

    @abstractmethod
    def fetch(self, community: Community) -> float:
        raise NotImplementedError


def utcnow() -> datetime:
    return datetime.now(timezone.utc)
