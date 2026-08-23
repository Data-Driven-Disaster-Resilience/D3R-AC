//! Error variants, one per `require(...)` revert reason in
//! `IdentityRegistry.sol`, plus the same Casper-specific storage-layer
//! failures risk-registry's error.rs documents (see that file's module
//! comment for the full rationale -- identical reasoning applies here).

use casper_types::ApiError;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityRegistryError {
    /// `IdentityRegistry.sol`'s `onlyAdmin` modifier.
    CallerIsNotAdmin = 1,
    /// `IdentityRegistry.sol`'s `onlyVerifier` modifier
    /// (`_checkRole(VERIFIER_ROLE, ...)`).
    CallerIsNotVerifier = 2,
    /// `IdentityRegistry.sol::proposeNewAdmin`'s "new admin is zero
    /// address" guard -- Casper has no zero-address concept the way
    /// EVM/TVM does, so this instead guards against `new_admin` not
    /// resolving to a usable `Key` at all (deserialization already
    /// enforces that a `Key` argument is well-formed, so in practice
    /// this variant exists for API-shape symmetry with the TRON
    /// contract's guard rather than a case this contract can actually
    /// hit today).
    NewAdminInvalid = 3,
    /// `IdentityRegistry.sol::acceptAdmin`'s "caller is not the pending
    /// admin" guard -- also covers the case where no admin transfer has
    /// been proposed at all (`pending_admin` is `None`).
    CallerIsNotPendingAdmin = 4,
    /// `IdentityRegistry.sol::verifyRecipient`'s "community label
    /// required" guard.
    CommunityLabelRequired = 5,
    /// `IdentityRegistry.sol::revokeRecipient`'s "recipient not
    /// verified" guard.
    RecipientNotVerified = 6,
    /// Same as risk-registry's `MissingKey`.
    MissingKey = 7,
    /// Same as risk-registry's `UnexpectedKeyType`.
    UnexpectedKeyType = 8,
    /// Same as risk-registry's `DictionaryReadFailed`.
    DictionaryReadFailed = 9,
}

impl From<IdentityRegistryError> for ApiError {
    fn from(error: IdentityRegistryError) -> Self {
        ApiError::User(error as u16)
    }
}
