//! Event definitions via the Casper Event Standard (CES) -- see
//! risk-registry/src/events.rs's module comment for why CES over a
//! hand-rolled mechanism (same reasoning applies here).
//!
//! One event per `D3RACToken.sol` event, same fields, same semantics,
//! adapted to Casper's `Key` addressing.

use casper_event_standard::Event;
use casper_types::{Key, U256};

#[derive(Event, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub from: Key,
    pub to: Key,
    pub value: U256,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct Approval {
    pub owner: Key,
    pub spender: Key,
    pub value: U256,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct OwnershipTransferred {
    pub previous_owner: Option<Key>,
    pub new_owner: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct OwnershipTransferProposed {
    pub current_owner: Key,
    pub proposed_owner: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct MinterUpdated {
    pub account: Key,
    pub can_mint: bool,
}
