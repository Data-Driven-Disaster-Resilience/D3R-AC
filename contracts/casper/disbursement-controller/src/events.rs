//! Event definitions via the Casper Event Standard -- one per
//! `DisbursementController.sol` event, same fields, same semantics.

use alloc::string::String;

use casper_event_standard::Event;
use casper_types::{Key, U256};

#[derive(Event, Debug, PartialEq, Eq)]
pub struct CommitmentCreated {
    pub commitment_id: u64,
    pub recipient: Key,
    pub token: Key,
    pub community: String,
    pub total_amount: U256,
    pub milestone_count: u64,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct MilestoneAttested {
    pub commitment_id: u64,
    pub milestone_index: u64,
    pub attested_by: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct MilestoneReleased {
    pub commitment_id: u64,
    pub milestone_index: u64,
    pub recipient: Key,
    pub amount: U256,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct CommitmentCancelled {
    pub commitment_id: u64,
    pub cancelled_by: Key,
    pub unreleased_amount: U256,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct AdminTransferred {
    pub previous_admin: Key,
    pub new_admin: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct AdminTransferProposed {
    pub current_admin: Key,
    pub proposed_admin: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct AttesterUpdated {
    pub account: Key,
    pub is_attester: bool,
}
