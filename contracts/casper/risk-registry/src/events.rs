//! Event definitions via the Casper Event Standard (CES) -- the
//! community-standard event mechanism selected in
//! `docs/casper-contracts-srs.md` §8 for this suite, over a hand-rolled
//! dictionary emulation, specifically so any CES-aware indexer or
//! future data-pipeline integration can read these without
//! D3R·AC-specific tooling (NFR-3: every state change must be readable
//! by an off-chain observer without needing the calling account's own
//! records).
//!
//! One event per `RiskRegistry.sol` event, same fields, same semantics.

use alloc::string::String;

use casper_event_standard::Event;
use casper_types::Key;

#[derive(Event, Debug, PartialEq, Eq)]
pub struct CommunityRegistered {
    pub community_id: String,
    pub name: String,
    pub region: String,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct RiskUpdated {
    pub community_id: String,
    pub hazard: u64,
    pub exposure: u64,
    pub vulnerability: u64,
    pub risk_score: u64,
    pub feeder: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct ThresholdCrossed {
    pub community_id: String,
    pub risk_score: u64,
    pub threshold: u64,
    pub timestamp: u64,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct DataFeederAdded {
    pub feeder: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct DataFeederRemoved {
    pub feeder: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct ThresholdUpdated {
    pub previous_threshold: u64,
    pub new_threshold: u64,
}
