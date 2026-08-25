//! Event definitions via the Casper Event Standard. The first seven
//! match the CEP-18 standard's own `CEP18Event` enum
//! (ceps/text/0018-token-standard.md) field-for-field -- load-bearing
//! for interop, not internal convention. The last three are this
//! contract's own extensions (D3RACToken.sol's OwnershipTransferred/
//! OwnershipTransferProposed/MinterUpdated), same as every other
//! two-step-transfer contract in this suite.

use casper_event_standard::Event;
use casper_types::{Key, U256};

#[derive(Event, Debug, PartialEq, Eq)]
pub struct Mint {
    pub recipient: Key,
    pub amount: U256,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct Burn {
    pub owner: Key,
    pub amount: U256,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct SetAllowance {
    pub owner: Key,
    pub spender: Key,
    pub allowance: U256,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct IncreaseAllowance {
    pub owner: Key,
    pub spender: Key,
    pub allowance: U256,
    pub inc_by: U256,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct DecreaseAllowance {
    pub owner: Key,
    pub spender: Key,
    pub allowance: U256,
    pub decr_by: U256,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub sender: Key,
    pub recipient: Key,
    pub amount: U256,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct TransferFrom {
    pub spender: Key,
    pub owner: Key,
    pub recipient: Key,
    pub amount: U256,
}

// -- Extensions beyond the CEP-18 standard --

#[derive(Event, Debug, PartialEq, Eq)]
pub struct OwnershipTransferred {
    pub previous_owner: Key,
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
    pub is_minter: bool,
}
