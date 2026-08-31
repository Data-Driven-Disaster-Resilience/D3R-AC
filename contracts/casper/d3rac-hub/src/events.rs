//! CES events -- one per `D3RACHub.sol` event, same fields/semantics.
//! No per-orchestration-call events here beyond that: every
//! orchestration entry point (verify_recipient, create_commitment,
//! etc.) is a thin pass-through to the underlying contract, which
//! already emits its own event for that action -- duplicating that
//! into a second Hub-side event would be redundant, not
//! `D3RACHub.sol`'s own behavior either (it emits no events of its
//! own for pass-through calls, only for the admin/module/pause actions
//! below).

use alloc::string::String;

use casper_types::Key;

use casper_event_standard::Event;

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

/// `module` identifies which module pointer changed -- a short fixed
/// label (`"token"`, `"identity_registry"`, etc.), Casper-`String`
/// analog of `D3RACHub.sol`'s `bytes32 indexed module`.
#[derive(Event, Debug, PartialEq, Eq)]
pub struct ModuleUpdated {
    pub module: String,
    pub previous_address: Option<Key>,
    pub new_address: Option<Key>,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct Paused {
    pub by: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct Unpaused {
    pub by: Key,
}
