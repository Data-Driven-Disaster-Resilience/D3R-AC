//! Error variants, one per `require(...)` revert reason in
//! `D3RACToken.sol`, plus the same Casper-specific storage-layer
//! failures risk-registry's error.rs documents (see that file's module
//! comment for the full rationale -- identical reasoning applies here).

use casper_types::ApiError;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3racTokenError {
    /// `D3RACToken.sol`'s `onlyOwner` modifier.
    CallerIsNotOwner = 1,
    /// `D3RACToken.sol`'s `onlyMinter` modifier.
    CallerIsNotMinter = 2,
    /// `D3RACToken.sol::transfer`/`_transfer`'s "transfer exceeds
    /// balance" guard.
    InsufficientBalance = 3,
    /// `D3RACToken.sol::transferFrom`'s "transfer exceeds allowance"
    /// guard.
    InsufficientAllowance = 4,
    /// `D3RACToken.sol::burn`/`_burn`'s "burn exceeds balance" guard.
    BurnExceedsBalance = 5,
    /// `D3RACToken.sol::proposeNewOwner`'s "new owner is zero address"
    /// guard -- Casper has no zero-address concept the way EVM/TVM
    /// does (see identity-registry's identical reasoning on
    /// `NewAdminInvalid`), so this instead guards against a
    /// `new_owner`/`to`/`spender`/`from` argument that isn't a usable
    /// `Key`. As with identity-registry's equivalent, deserialization
    /// already enforces a well-formed `Key`, so this exists for API-
    /// shape symmetry with the TRON contract's guard rather than a
    /// case reachable today.
    InvalidAddress = 6,
    /// `D3RACToken.sol::acceptOwnership`'s "caller is not the pending
    /// owner" guard -- also covers the case where no ownership
    /// transfer has been proposed at all (`pending_owner` is `None`).
    CallerIsNotPendingOwner = 7,
    /// `call()`'s initial-supply scaling (`initial_supply *
    /// 10^decimals`) overflowed `U256` -- has no direct
    /// `D3RACToken.sol` equivalent since Solidity 0.8's built-in
    /// overflow checks handle this implicitly; Casper's `U256`
    /// doesn't panic on overflow the way Solidity 0.8 does, so this
    /// needs an explicit `checked_mul` + revert instead.
    SupplyOverflow = 11,
    /// Same as risk-registry's `MissingKey`.
    MissingKey = 12,
    /// Same as risk-registry's `UnexpectedKeyType`.
    UnexpectedKeyType = 13,
    /// Same as risk-registry's `DictionaryReadFailed`.
    DictionaryReadFailed = 14,
}

impl From<D3racTokenError> for ApiError {
    fn from(error: D3racTokenError) -> Self {
        ApiError::User(error as u16)
    }
}
