//! Error variants, one per `require(...)` revert reason in
//! `RiskRegistry.sol`, plus a few Casper-specific storage-layer
//! failures that have no Solidity equivalent (Solidity's storage reads
//! never "fail to find a key" the way Casper's named-key/uref lookups
//! can). Mapped into Casper's `ApiError::User(u16)` space so a caller
//! (or a block explorer) can distinguish exactly which precondition
//! failed, the same diagnostic value `require(cond, "reason string")`
//! gives on TRON -- Casper's WASM execution has no string-revert-reason
//! equivalent, so a stable, documented error code is the closest
//! analog.

use casper_types::ApiError;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskRegistryError {
    /// `RiskRegistry.sol::registerCommunity`'s
    /// "community already registered" guard.
    CommunityAlreadyRegistered = 1,
    /// `RiskRegistry.sol::updateRisk`/`getCommunity`'s
    /// "community not registered" guard.
    CommunityNotRegistered = 2,
    /// `RiskRegistry.sol::updateRisk`'s
    /// "value out of [0,1] range" guard.
    ValueOutOfRange = 3,
    /// `RiskRegistry.sol`'s `onlyOwner` modifier.
    CallerIsNotOwner = 4,
    /// `RiskRegistry.sol`'s `onlyDataFeeder` modifier.
    CallerIsNotDataFeeder = 5,
    /// A named key this contract expects to have set up at install
    /// time (`call()`) is missing -- should be unreachable outside a
    /// corrupted/tampered deploy, but Casper's key lookups are
    /// `Option`-returning, so this must be handled explicitly rather
    /// than assumed.
    MissingKey = 6,
    /// A named key resolved to a `Key` variant other than the one
    /// expected (e.g. a `URef` lookup landing on a `Key::Hash`) --
    /// same "should be unreachable, but Casper's types don't guarantee
    /// it statically" reasoning as `MissingKey`.
    UnexpectedKeyType = 7,
    /// `storage::dictionary_get`'s outer `Result` was `Err` (as
    /// opposed to `Ok(None)` for a simply-missing key) -- added after
    /// a real CI test failure surfaced as a generic, undiagnosable
    /// `ApiError::None` from a bare `.unwrap_or_revert()` on exactly
    /// this call; every `dictionary_get` call site in this contract
    /// now uses this instead, so any future occurrence is immediately
    /// attributable to a specific line rather than a mystery revert.
    DictionaryReadFailed = 8,
}

impl From<RiskRegistryError> for ApiError {
    fn from(error: RiskRegistryError) -> Self {
        ApiError::User(error as u16)
    }
}
