//! Error variants. The first three preserve the CEP-18 standard's own
//! exact numeric codes (ceps/text/0018-token-standard.md's `CEP18Error`
//! enum: InsufficientBalance = 60001, InsufficientAllowance = 60002,
//! CannotTargetSelfUser = 60003) -- these are part of the standard, not
//! an internal convention, so callers built against CEP-18 generally
//! (not specifically against this contract) can recognize them. This
//! contract's own extensions (ownership/minter role) use a low, clearly
//! separate number range so they can never collide with the standard's
//! reserved 60001-60003.

use casper_types::ApiError;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3racTokenError {
    /// CEP-18 standard, exact code.
    InsufficientBalance = 60001,
    /// CEP-18 standard, exact code.
    InsufficientAllowance = 60002,
    /// CEP-18 standard, exact code.
    CannotTargetSelfUser = 60003,

    /// D3RACToken.sol's onlyOwner modifier.
    CallerIsNotOwner = 1,
    /// D3RACToken.sol's onlyMinter modifier
    /// (_checkRole(MINTER_ROLE, ...)).
    CallerIsNotMinter = 2,
    /// D3RACToken.sol::proposeNewOwner's zero-owner guard (see
    /// IdentityRegistryError::NewAdminInvalid's identical Casper-vs-EVM
    /// caveat).
    NewOwnerInvalid = 3,
    /// D3RACToken.sol::acceptOwnership's "caller is not the pending
    /// owner" guard.
    CallerIsNotPendingOwner = 4,
    /// D3RACToken.sol::mint's "mint to zero address" guard -- see
    /// NewOwnerInvalid's caveat; kept for API-shape symmetry.
    MintToInvalidAddress = 5,

    MissingKey = 6,
    UnexpectedKeyType = 7,
    DictionaryReadFailed = 8,
}

impl From<D3racTokenError> for ApiError {
    fn from(error: D3racTokenError) -> Self {
        ApiError::User(error as u16)
    }
}
