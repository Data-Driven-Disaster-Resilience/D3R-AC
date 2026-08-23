//! Event definitions via the Casper Event Standard (CES) -- see
//! risk-registry/src/events.rs's module comment for why CES over a
//! hand-rolled mechanism (same reasoning applies here).
//!
//! One event per `IdentityRegistry.sol` event, same fields, same
//! semantics.

use alloc::string::String;

use casper_event_standard::Event;
use casper_types::Key;

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
pub struct VerifierUpdated {
    pub account: Key,
    pub is_verifier: bool,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct RecipientVerified {
    pub recipient: Key,
    pub community: String,
    pub verified_by: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct RecipientRevoked {
    pub recipient: Key,
    pub revoked_by: Key,
}
