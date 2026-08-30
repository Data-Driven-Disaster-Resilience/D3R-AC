//! Error variants, one per `require(...)` revert reason in
//! `MultiSigAdmin.sol`, plus the same Casper-specific storage-layer
//! failures risk-registry's error.rs documents (see that file's module
//! comment for the full rationale -- identical reasoning applies here).

use casper_types::ApiError;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultisigAdminError {
    /// `MultiSigAdmin.sol`'s `onlyOwner` modifier.
    CallerIsNotOwner = 1,
    /// `MultiSigAdmin.sol` constructor's "owners required" guard.
    OwnersRequired = 2,
    /// `MultiSigAdmin.sol` constructor's "invalid threshold" guard.
    InvalidThreshold = 3,
    /// `MultiSigAdmin.sol` constructor's "duplicate owner" guard.
    DuplicateOwner = 4,
    /// `MultiSigAdmin.sol::submitTransaction`'s "target is zero
    /// address" guard -- here, guards against a `target_package_hash`
    /// that doesn't parse as a package hash at all.
    InvalidTarget = 5,
    /// `MultiSigAdmin.sol`'s `txExists` modifier.
    TransactionDoesNotExist = 6,
    /// `MultiSigAdmin.sol`'s `notExecuted` modifier.
    TransactionAlreadyExecuted = 7,
    /// `MultiSigAdmin.sol::confirmTransaction`'s "already confirmed"
    /// guard.
    AlreadyConfirmed = 8,
    /// `MultiSigAdmin.sol::revokeConfirmation`'s "not confirmed" guard.
    NotConfirmed = 9,
    /// `MultiSigAdmin.sol::executeTransaction`'s "insufficient
    /// confirmations" guard.
    InsufficientConfirmations = 10,
    /// `target_args_bytes` did not deserialize as a valid `RuntimeArgs`
    /// -- has no TRON equivalent (TRON's low-level `call` accepts
    /// arbitrary calldata bytes with no comparable decode step; Casper
    /// requires well-formed, typed `RuntimeArgs` for a cross-contract
    /// call, so a malformed submission is rejected at execution time
    /// rather than left to the callee to interpret).
    MalformedTargetArgs = 11,
    /// Same as risk-registry's `MissingKey`.
    MissingKey = 12,
    /// Same as risk-registry's `UnexpectedKeyType`.
    UnexpectedKeyType = 13,
    /// Same as risk-registry's `DictionaryReadFailed`.
    DictionaryReadFailed = 14,
    /// `runtime::get_immediate_caller()` returned a `CallerInfo` kind
    /// `immediate_caller_key` doesn't recognize -- see that function's
    /// own comment for which kinds are handled and why.
    UnrecognizedCallerKind = 15,
}

impl From<MultisigAdminError> for ApiError {
    fn from(error: MultisigAdminError) -> Self {
        ApiError::User(error as u16)
    }
}
