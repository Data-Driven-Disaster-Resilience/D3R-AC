"""
E(c) — exposure factor.

Per docs/data-pipeline-srs.md §8, how E(c) is actually derived (population
density from geospatial data, infrastructure proximity, a mix) is an open
decision requiring disaster-response domain expertise, not an engineering
default. This adapter is an honest placeholder: it reads the static value
already curated in config/communities.yaml (the same numbers the frontend
mock data uses today) rather than pretending to compute something it
doesn't have real data for.

Swap this for a real geospatial adapter (e.g. WorldPop population-density
tiles clipped to each community's bbox) without touching pipeline.py —
that's the point of the ExposureAdapter interface in base.py.
"""

from __future__ import annotations

from ..config import Community
from .base import ExposureAdapter


class StaticExposureAdapter(ExposureAdapter):
    name = "static_exposure_config"

    def fetch(self, community: Community) -> float:
        return float(community.exposure)
