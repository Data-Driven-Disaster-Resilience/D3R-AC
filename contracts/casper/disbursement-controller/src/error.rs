//! Error variants, one per `require(...)` revert reason in
//! `DisbursementController.sol`, plus Casper-specific storage-layer
//! failures. See identity-registry/src/error.rs's module comment for
//! the general approach.

use casper_types::ApiError;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisbursementControllerError {
    /// `onlyAdmin` modifier.
    CallerIsNotAdmin = 1,
    /// `onlyAttester` modifier.
    CallerIsNotAttester = 2,
    /// `proposeNewAdmin`'s zero-admin guard (see
    /// IdentityRegistryError::NewAdminInvalid's identical Casper-vs-EVM
    /// caveat).
    NewAdminInvalid = 3,
    /// `acceptAdmin`'s "caller is not the pending admin" guard.
    CallerIsNotPendingAdmin = 4,
    /// `createCommitment`'s "recipient not verified" guard.
    RecipientNotVerified = 5,
    /// `createCommitment`'s "at least one milestone required" guard.
    NoMilestones = 6,
    /// `createCommitment`'s "length mismatch" guard
    /// (descriptions/amounts).
    LengthMismatch = 7,
    /// `createCommitment`'s "milestone amount must be > 0" guard.
    MilestoneAmountIsZero = 8,
    /// `_requireCommitment`'s "commitment does not exist" guard.
    CommitmentDoesNotExist = 9,
    /// `_getActiveCommitment`'s "commitment not active" guard.
    CommitmentNotActive = 10,
    /// `_getMilestone`'s "milestone does not exist" guard.
    MilestoneDoesNotExist = 11,
    /// `attestMilestone`'s "milestone already attested" guard.
    MilestoneAlreadyAttested = 12,
    /// `releaseMilestone`'s "milestone not attested" guard.
    MilestoneNotAttested = 13,
    /// `releaseMilestone`'s "milestone already released" guard.
    MilestoneAlreadyReleased = 14,
    /// `releaseMilestone`'s "insufficient contract balance for
    /// milestone" guard.
    InsufficientBalance = 15,
    /// D3RACProperties.sol's nonReentrant guard, applied to
    /// release_milestone (the one entry point here that makes an
    /// external cross-contract call).
    ReentrantCall = 16,
    MissingKey = 17,
    UnexpectedKeyType = 18,
    DictionaryReadFailed = 19,
}

impl From<DisbursementControllerError> for ApiError {
    fn from(error: DisbursementControllerError) -> Self {
        ApiError::User(error as u16)
    }
}
