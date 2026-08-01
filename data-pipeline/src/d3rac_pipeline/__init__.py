"""
D3R·AC data pipeline.

Implements docs/data-pipeline-srs.md: turns real hazard signals (satellite
and sensor sources, Africa-prioritized) plus exposure/vulnerability data
into the H(t)/E(c)/V(c) inputs RiskRegistry.sol and D3RACHub.sol already
expect, and pushes them on-chain via Hub.updateRisk.
"""

__version__ = "0.1.0"
