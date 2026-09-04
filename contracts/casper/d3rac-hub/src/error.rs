//! Error variants, one per `require(...)` revert reason in
//! `D3RACHub.sol`, plus this suite's usual Casper-specific storage- and
//! caller-resolution-layer failures. See risk-registry/src/error.rs's
//! header for why these exist as stable numeric codes.

use casper_types::ApiError;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3racHubError {
    /// `onlyAdmin` modifier.
    CallerIsNotAdmin = 1,
    /// `whenNotPaused` modifier.
    ContractIsPaused = 2,
    /// Constructor's zero-address guards, for token/identityRegistry/
    /// disbursementController -- these three are unconditionally
    /// required, matching `D3RACHub.sol`'s constructor.
    RequiredModuleMissing = 3,
    /// `acceptAdmin`'s "caller is not the pending admin" guard,
    /// including when no transfer has been proposed at all.
    CallerIsNotPendingAdmin = 4,
    /// `registerCommunity`/`updateRisk`/`setRiskDataFeeder`/
    /// `setRiskThreshold`/`transferRiskRegistryOwnership`'s
    /// "riskRegistry not set" guard.
    RiskRegistryNotSet = 5,
    /// `openFundingRequest`/`closeFundingRequest`/
    /// `setFundingProposer`/`recordFundingPledge`/
    /// `linkFundingRequestToCommitment`/
    /// `proposeFundingRequestRegistryOwnership`/
    /// `acceptFundingRequestRegistryOwnership`'s "fundingRequestRegistry
    /// not set" guard.
    FundingRequestRegistryNotSet = 6,
    /// `pause`'s "already paused" guard.
    AlreadyPaused = 7,
    /// `unpause`'s "not paused" guard.
    NotPaused = 8,
    /// A named key this contract expects to have set up at install
    /// time is missing.
    MissingKey = 9,
    /// A named key resolved to an unexpected `Key` variant.
    UnexpectedKeyType = 10,
    /// `runtime::get_immediate_caller()`'s `CallerInfo` resolved to a
    /// kind this contract doesn't recognize (not account, not
    /// contract-package) -- see main.rs's `immediate_caller_key` for
    /// the two kinds it does handle. Same defensive-completeness
    /// reasoning as every other contract in this suite that was
    /// updated with this helper (`fix/get-caller-systemic-immediate-
    /// caller`).
    UnrecognizedCallerKind = 11,
}

impl From<D3racHubError> for ApiError {
    fn from(error: D3racHubError) -> Self {
        ApiError::User(error as u16)
    }
}
