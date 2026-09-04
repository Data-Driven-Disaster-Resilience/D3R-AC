//! Error variants, one per `require(...)` revert reason in
//! `FundingRequestRegistry.sol`, plus the same Casper-specific
//! storage-layer failure variants every contract in this suite has.
//! See risk-registry/src/error.rs's own header comment for why these
//! exist as a stable numeric code rather than a string reason.

use casper_types::ApiError;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FundingRequestRegistryError {
    /// `onlyOwner` modifier.
    CallerIsNotOwner = 1,
    /// `onlyProposer` modifier (`_checkRole(PROPOSER_ROLE, ...)`).
    CallerIsNotProposer = 2,
    /// `proposeNewOwner`'s zero-address guard.
    ZeroAddress = 3,
    /// `acceptOwnership`'s "caller is not the pending owner" guard --
    /// including, same as identity-registry's `accept_admin`, the case
    /// where no transfer has been proposed at all (`pending_owner` is
    /// `None`).
    CallerIsNotPendingOwner = 4,
    /// `openRequest`'s "amount must be > 0" guard.
    AmountMustBePositive = 5,
    /// `_getRequest`'s "invalid request id" guard (`requestId <
    /// _requests.length`) -- reused by every entry point that takes a
    /// `request_id` argument (`recordPledge`, `linkToCommitment`,
    /// `closeRequest`, `getRequest`), same as the Solidity original.
    InvalidRequestId = 6,
    /// `recordPledge`/`linkToCommitment`/`closeRequest`'s "not
    /// authorized" guard (`msg.sender == r.requester || msg.sender ==
    /// owner`).
    NotAuthorizedForRequest = 7,
    /// `recordPledge`'s "request not open" guard (`status == Open ||
    /// status == PartiallyFunded`).
    RequestNotOpen = 8,
    /// `recordPledge`'s "pledge amount must be > 0" guard.
    PledgeAmountMustBePositive = 9,
    /// A named key this contract expects to have set up at install
    /// time is missing.
    MissingKey = 10,
    /// A named key resolved to an unexpected `Key` variant.
    UnexpectedKeyType = 11,
    /// `storage::dictionary_get`'s outer `Result` was `Err` -- see
    /// risk-registry/src/error.rs's `DictionaryReadFailed` for the
    /// real CI failure this is guarding against.
    DictionaryReadFailed = 12,
}

impl From<FundingRequestRegistryError> for ApiError {
    fn from(error: FundingRequestRegistryError) -> Self {
        ApiError::User(error as u16)
    }
}
