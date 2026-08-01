from __future__ import annotations

import os
from dataclasses import dataclass, field
from pathlib import Path

import yaml

DEFAULT_CONFIG_DIR = Path(__file__).resolve().parents[2] / "config"


@dataclass
class Community:
    id: str
    name: str
    region: str
    priority_region: str  # "africa" | "global"
    bbox: list  # [min_lon, min_lat, max_lon, max_lat]
    exposure: float
    vulnerability: float


@dataclass
class StaleConfig:
    stale_after_hours: float
    stale_policy: str  # "hold_and_flag" | "stop_submitting"


@dataclass
class ChainConfig:
    network: str
    hub_address: str
    full_node: str
    feeder_private_key: str = field(repr=False, default="")  # NFR-1: never repr/log this


@dataclass
class PipelineSettings:
    stale: StaleConfig
    hazard_combine_strategy: str
    hazard_weights: dict
    state_db_path: str
    chain: ChainConfig
    log_level: str
    audit_log_path: str


def load_communities(path: Path | None = None) -> list[Community]:
    path = path or (DEFAULT_CONFIG_DIR / "communities.yaml")
    with open(path, "r", encoding="utf-8") as f:
        raw = yaml.safe_load(f)

    communities = [Community(**entry) for entry in raw["communities"]]

    # FR-1 note: this list *is* the pipeline's own record of monitored
    # communities per FR-1 — pipeline.py registers any not-yet-registered
    # community on-chain before its first updateRisk call.
    #
    # Africa-priority ordering: africa first, global after, stable within
    # each group (preserves config file order) so runs are deterministic.
    communities.sort(key=lambda c: 0 if c.priority_region == "africa" else 1)
    return communities


def load_settings(path: Path | None = None) -> PipelineSettings:
    path = path or (DEFAULT_CONFIG_DIR / "settings.yaml")
    with open(path, "r", encoding="utf-8") as f:
        raw = yaml.safe_load(f)

    chain_raw = raw["chain"]
    chain = ChainConfig(
        network=chain_raw["network"],
        hub_address=os.environ.get("D3RAC_HUB_ADDRESS", chain_raw.get("hub_address", "")),
        full_node=os.environ.get("D3RAC_TRON_FULL_NODE", chain_raw.get("full_node", "")),
        # NFR-1: secret comes from env/secrets-manager only, never from this file.
        feeder_private_key=os.environ.get("D3RAC_FEEDER_PRIVATE_KEY", ""),
    )

    return PipelineSettings(
        stale=StaleConfig(
            stale_after_hours=float(raw["stale_after_hours"]),
            stale_policy=raw["stale_policy"],
        ),
        hazard_combine_strategy=raw["hazard_combine_strategy"],
        hazard_weights=raw.get("hazard_weights", {}),
        state_db_path=raw["state_db_path"],
        chain=chain,
        log_level=raw.get("logging", {}).get("level", "INFO"),
        audit_log_path=raw.get("logging", {}).get("audit_log_path", "./data-pipeline-audit.log"),
    )
