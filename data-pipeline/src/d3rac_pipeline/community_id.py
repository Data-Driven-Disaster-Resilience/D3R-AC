"""
Maps this pipeline's stable off-chain community `id` (a short slug in
config/communities.yaml, e.g. "kebbi-river-basin") to the bytes32
`communityId` RiskRegistry.sol and D3RACHub.sol use.

We derive it deterministically as keccak256(off_chain_id_utf8), the same
way Solidity's `keccak256(abi.encodePacked(string))` would for a literal.
This means the mapping never needs to be stored anywhere — it's a pure
function of the slug in the config file. IMPORTANT: never rename an `id`
already registered on-chain; that produces a *different* communityId and
orphans the on-chain record (add a new community instead — see the
comment at the top of config/communities.yaml).
"""

from eth_hash.auto import keccak


def to_community_id(off_chain_id: str) -> bytes:
    """Return the 32-byte communityId for a given off-chain slug."""
    return keccak(off_chain_id.encode("utf-8"))


def to_community_id_hex(off_chain_id: str) -> str:
    """Same as to_community_id, but as a 0x-prefixed hex string — the form
    tronpy / most chain tooling expects for a bytes32 argument."""
    return "0x" + to_community_id(off_chain_id).hex()
