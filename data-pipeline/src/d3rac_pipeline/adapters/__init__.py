from .base import HazardAdapter, HazardReading, NoFreshData
from .satellite_fire import SatelliteFireAdapter
from .seismic_usgs import SeismicUSGSAdapter
from .eonet_events import EonetEventsAdapter
from .gdacs_alerts import GdacsAlertsAdapter
from .static_exposure import StaticExposureAdapter
from .static_vulnerability import StaticVulnerabilityAdapter

__all__ = [
    "HazardAdapter",
    "HazardReading",
    "NoFreshData",
    "SatelliteFireAdapter",
    "SeismicUSGSAdapter",
    "EonetEventsAdapter",
    "GdacsAlertsAdapter",
    "StaticExposureAdapter",
    "StaticVulnerabilityAdapter",
]
