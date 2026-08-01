"""
FR-4 — On-chain submission via the Hub (never RiskRegistry directly).
FR-5 — Threshold-crossing awareness for logging only, never as a gate on
        whether to submit.

Wraps tronpy so pipeline.py never has to know ABI/contract-call details.
Uses D3RACHub's minimal interface (see contracts/tron/contracts/D3RACHub.sol):

    registerCommunity(bytes32 communityId, string name_, string region)
    updateRisk(bytes32 communityId, uint256 hazard, uint256 exposure, uint256 vulnerability)

A submission reverting with "RiskRegistry: caller is not a data feeder"
is a deployment/wiring defect (the pipeline's address hasn't been granted
feeder status via Hub.setRiskDataFeeder) and is raised as
FeederNotAuthorizedError so callers can surface it distinctly from an
ordinary data-validation failure, per FR-4.
"""

from __future__ import annotations

import logging

from tronpy import Tron
from tronpy.keys import PrivateKey
from tronpy.providers import HTTPProvider

logger = logging.getLogger(__name__)

# Minimal ABI covering only the functions this pipeline calls.
HUB_ABI = [
    {
        "name": "registerCommunity",
        "inputs": [
            {"name": "communityId", "type": "bytes32"},
            {"name": "name_", "type": "string"},
            {"name": "region", "type": "string"},
        ],
        "outputs": [],
        "type": "function",
    },
    {
        "name": "updateRisk",
        "inputs": [
            {"name": "communityId", "type": "bytes32"},
            {"name": "hazard", "type": "uint256"},
            {"name": "exposure", "type": "uint256"},
            {"name": "vulnerability", "type": "uint256"},
        ],
        "outputs": [],
        "type": "function",
    },
]


class FeederNotAuthorizedError(Exception):
    """Raised when a submission reverts because this pipeline's on-chain
    identity hasn't been granted data-feeder status (FR-4) — a deployment
    defect, not a per-community data problem."""


class ChainSubmissionError(Exception):
    """Any other on-chain submission failure (e.g. community not yet
    registered, value out of range slipping past fixed_point.py, network
    error)."""


class ChainClient:
    def __init__(self, full_node: str, hub_address: str, feeder_private_key_hex: str):
        if not hub_address:
            raise ValueError("hub_address is not configured (see config/settings.yaml / D3RAC_HUB_ADDRESS)")
        if not feeder_private_key_hex:
            raise ValueError(
                "Feeder private key is not configured — set D3RAC_FEEDER_PRIVATE_KEY in the "
                "environment (NFR-1: never in a config file committed to source control)."
            )

        self.client = Tron(provider=HTTPProvider(full_node))
        self.priv_key = PrivateKey(bytes.fromhex(feeder_private_key_hex.removeprefix("0x")))
        self.feeder_address = self.priv_key.public_key.to_base58check_address()
        self.hub = self.client.get_contract(hub_address)
        self.hub.abi = HUB_ABI

    def register_community(self, community_id_hex: str, name: str, region: str) -> str:
        txn = (
            self.hub.functions.registerCommunity(community_id_hex, name, region)
            .with_owner(self.feeder_address)
            .build()
            .sign(self.priv_key)
        )
        result = txn.broadcast().wait()
        _raise_if_reverted(result)
        return result.get("id", "")

    def update_risk(self, community_id_hex: str, hazard: int, exposure: int, vulnerability: int) -> str:
        txn = (
            self.hub.functions.updateRisk(community_id_hex, hazard, exposure, vulnerability)
            .with_owner(self.feeder_address)
            .build()
            .sign(self.priv_key)
        )
        result = txn.broadcast().wait()
        _raise_if_reverted(result)
        return result.get("id", "")

    # FR-5: purely observational. Never call this to decide whether to submit.
    def is_above_threshold(self, risk_registry_contract, community_id_hex: str) -> bool:
        return bool(risk_registry_contract.functions.isAboveThreshold(community_id_hex))


def _raise_if_reverted(result: dict) -> None:
    receipt = result.get("receipt", {})
    if receipt.get("result") == "REVERT":
        message = str(result)
        if "not a data feeder" in message:
            raise FeederNotAuthorizedError(
                "Hub/RiskRegistry rejected this pipeline's address as a data feeder — "
                "check Hub.setRiskDataFeeder wiring (see contracts/tron/README.md, "
                "\"Wiring the Hub\")."
            )
        raise ChainSubmissionError(f"Transaction reverted: {message}")
