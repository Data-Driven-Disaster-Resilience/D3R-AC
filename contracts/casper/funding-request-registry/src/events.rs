//! Casper Event Standard (CES) events -- one per
//! `FundingRequestRegistry.sol` event, same fields/semantics. See
//! risk-registry/src/events.rs's header for why CES over a hand-rolled
//! mechanism.

use alloc::string::String;

use casper_event_standard::Event;
use casper_types::{Key, U256};

use crate::model::RequestStatus;

#[derive(Event, Debug, PartialEq, Eq)]
pub struct RequestOpened {
    pub request_id: u64,
    pub community_id: String,
    pub requester: Key,
    pub amount_requested: U256,
    pub data_source_uri: String,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct PledgeRecorded {
    pub request_id: u64,
    pub amount: U256,
    pub pledge_source_uri: String,
    pub recorded_by: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct RequestLinkedToCommitment {
    pub request_id: u64,
    pub commitment_id: u64,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct RequestStatusChanged {
    pub request_id: u64,
    pub previous_status: RequestStatus,
    pub new_status: RequestStatus,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct ProposerAdded {
    pub proposer: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct ProposerRemoved {
    pub proposer: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct OwnershipTransferProposed {
    pub current_owner: Key,
    pub proposed_owner: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct OwnershipTransferred {
    pub previous_owner: Key,
    pub new_owner: Key,
}
