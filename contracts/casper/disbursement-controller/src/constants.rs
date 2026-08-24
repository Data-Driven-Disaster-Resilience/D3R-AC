//! Constants for named keys, entry-point names, and runtime argument
//! names. See identity-registry/src/constants.rs / risk-registry's own
//! constants.rs for why these are centralized.

// Named keys (contract storage)
pub const KEY_ADMIN: &str = "admin";
pub const KEY_PENDING_ADMIN: &str = "pending_admin";
pub const KEY_ATTESTERS_DICT: &str = "attesters";
pub const KEY_REGISTRY_HASH: &str = "registry_hash";
pub const KEY_COMMITMENTS_DICT: &str = "commitments";
pub const KEY_COMMITMENT_COUNT: &str = "commitment_count";
/// Simple bool flag guard against reentrancy in release_milestone --
/// same idea as D3RACProperties.sol's nonReentrant status flag, applied
/// to the one entry point here that makes an external cross-contract
/// call (the CEP-18 transfer).
pub const KEY_REENTRANCY_GUARD: &str = "reentrancy_guard";

pub const CONTRACT_HASH_KEY_NAME: &str = "disbursement_controller_contract_hash";
pub const PACKAGE_HASH_KEY_NAME: &str = "disbursement_controller_package_hash";
pub const ACCESS_UREF_KEY_NAME: &str = "disbursement_controller_access_uref";

// Entry point names
pub const ENTRY_POINT_SET_ATTESTER: &str = "set_attester";
pub const ENTRY_POINT_PROPOSE_NEW_ADMIN: &str = "propose_new_admin";
pub const ENTRY_POINT_ACCEPT_ADMIN: &str = "accept_admin";
pub const ENTRY_POINT_CREATE_COMMITMENT: &str = "create_commitment";
pub const ENTRY_POINT_ATTEST_MILESTONE: &str = "attest_milestone";
pub const ENTRY_POINT_RELEASE_MILESTONE: &str = "release_milestone";
pub const ENTRY_POINT_CANCEL_COMMITMENT: &str = "cancel_commitment";
pub const ENTRY_POINT_COMMITMENT_COUNT: &str = "commitment_count";
pub const ENTRY_POINT_GET_COMMITMENT: &str = "get_commitment";
pub const ENTRY_POINT_GET_MILESTONE: &str = "get_milestone";
pub const ENTRY_POINT_IS_ATTESTER: &str = "is_attester";
pub const ENTRY_POINT_INIT: &str = "init";

// CEP-18 entry point this contract calls on the token it's configured
// with -- see docs/casper-contracts-srs.md and the CEP-18 standard
// (ceps/text/0018-token-standard.md): transfer(recipient: Key, amount:
// U256), no return value, reverts on failure -- including on
// insufficient balance, which is why this contract doesn't need its
// own separate balance_of pre-check (see release_milestone's comment).
pub const CEP18_ENTRY_POINT_TRANSFER: &str = "transfer";
pub const CEP18_ARG_RECIPIENT: &str = "recipient";
pub const CEP18_ARG_AMOUNT: &str = "amount";

// This contract's own runtime argument names
pub const ARG_ACCOUNT: &str = "account";
pub const ARG_IS_ATTESTER: &str = "is_attester";
pub const ARG_NEW_ADMIN: &str = "new_admin";
pub const ARG_REGISTRY_HASH: &str = "registry_hash";
pub const ARG_RECIPIENT: &str = "recipient";
pub const ARG_TOKEN: &str = "token";
pub const ARG_COMMUNITY: &str = "community";
pub const ARG_DESCRIPTIONS: &str = "descriptions";
pub const ARG_AMOUNTS: &str = "amounts";
pub const ARG_COMMITMENT_ID: &str = "commitment_id";
pub const ARG_MILESTONE_INDEX: &str = "milestone_index";

// IdentityRegistry entry point this contract calls to gate
// create_commitment -- matches identity-registry's own
// ENTRY_POINT_IS_VERIFIED / ARG_RECIPIENT constants exactly (kept as a
// literal here rather than a cross-crate dependency, same reasoning as
// DisbursementController.sol keeping its own local ITRC20 interface
// instead of importing D3RACToken.sol -- this contract shouldn't need
// to recompile if identity-registry's *internal* module layout changes,
// only if its entry point's public interface does).
pub const IDENTITY_REGISTRY_ENTRY_POINT_IS_VERIFIED: &str = "is_verified";
