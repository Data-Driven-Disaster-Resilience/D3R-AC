//! Error variants, one per `require(...)` revert reason in
//! `DisbursementController.sol`, plus the same Casper-specific
//! storage-layer failures risk-registry's error.rs documents (see
//! that file's module comment for the full rationale -- identical
//! reasoning applies here).

use casper_types::ApiError;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisbursementError {
    /// `DisbursementController.sol`'s `onlyAdmin` modifier.
    CallerIsNotAdmin = 1,
    /// `DisbursementController.sol`'s `onlyAttester` modifier.
    CallerIsNotAttester = 2,
    /// `createCommitment`'s "recipient not verified" guard -- this is
    /// the real cross-contract call: a live `is_verified` query
    /// against `registry_package_hash`, not a locally cached flag.
    RecipientNotVerified = 3,
    /// `createCommitment`'s "at least one milestone required" guard.
    NoMilestones = 4,
    /// `createCommitment`'s "length mismatch" guard
    /// (`descriptions.length == amounts.length`).
    LengthMismatch = 5,
    /// `createCommitment`'s "milestone amount must be > 0" guard.
    ZeroMilestoneAmount = 6,
    /// `_requireCommitment`'s "commitment does not exist" guard.
    CommitmentDoesNotExist = 7,
    /// `_getActiveCommitment`'s "commitment not active" guard.
    CommitmentNotActive = 8,
    /// `_getMilestone`'s "milestone does not exist" guard.
    MilestoneDoesNotExist = 9,
    /// `attestMilestone`'s "milestone already attested" guard.
    MilestoneAlreadyAttested = 10,
    /// `releaseMilestone`'s "milestone not attested" guard.
    MilestoneNotAttested = 11,
    /// `releaseMilestone`'s "milestone already released" guard.
    MilestoneAlreadyReleased = 12,
    /// `releaseMilestone`'s "insufficient contract balance for
    /// milestone" guard -- a live `balance_of` query against
    /// `token_package_hash`, same reasoning as
    /// `RecipientNotVerified` above.
    InsufficientContractBalance = 13,
    /// `proposeNewAdmin`/`createCommitment`'s zero-address guards --
    /// Casper has no zero-address concept the way EVM/TVM does (see
    /// identity-registry's identical reasoning on `NewAdminInvalid`),
    /// kept for API-shape symmetry rather than a case reachable
    /// today, same as that contract's own precedent.
    InvalidAddress = 14,
    /// `acceptAdmin`'s "caller is not the pending admin" guard.
    CallerIsNotPendingAdmin = 15,
    /// Same as risk-registry's `MissingKey`.
    MissingKey = 16,
    /// Same as risk-registry's `UnexpectedKeyType`.
    UnexpectedKeyType = 17,
    /// Same as risk-registry's `DictionaryReadFailed`.
    DictionaryReadFailed = 18,
}

impl From<DisbursementError> for ApiError {
    fn from(error: DisbursementError) -> Self {
        ApiError::User(error as u16)
    }
}
