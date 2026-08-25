//! Event definitions via the Casper Event Standard (CES) -- see
//! risk-registry/src/events.rs's module comment for why CES over a
//! hand-rolled mechanism (same reasoning applies here).
//!
//! One event per `MultiSigAdmin.sol` event that has a Casper
//! equivalent, same fields, same semantics, adapted to Casper's `Key`
//! addressing. `MultiSigAdmin.sol`'s `TransactionExecutionFailed` has
//! deliberately been dropped -- see `main.rs`'s `execute_transaction`
//! doc comment: on Casper, a callee trap aborts the whole deploy, so
//! there's no "failed but recorded" state left to emit an event about
//! by the time execution would reach that point.

use casper_event_standard::Event;
use casper_types::Key;

#[derive(Event, Debug, PartialEq, Eq)]
pub struct OwnerAdded {
    pub owner: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct TransactionSubmitted {
    pub tx_id: u64,
    pub submitter: Key,
    pub target_package_hash: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct TransactionConfirmed {
    pub tx_id: u64,
    pub owner: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct ConfirmationRevoked {
    pub tx_id: u64,
    pub owner: Key,
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct TransactionExecuted {
    pub tx_id: u64,
    pub executor: Key,
}
