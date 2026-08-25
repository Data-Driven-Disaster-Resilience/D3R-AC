//! Constants for named keys, entry-point names, and runtime argument
//! names. See risk-registry/src/constants.rs's module comment for why
//! these are centralized -- same reasoning applies here.

// Named keys (contract storage)
pub const KEY_REGISTRY_PACKAGE_HASH: &str = "registry_package_hash";
pub const KEY_ADMIN: &str = "admin";
pub const KEY_PENDING_ADMIN: &str = "pending_admin";
pub const KEY_ATTESTERS_DICT: &str = "attesters";
pub const KEY_COMMITMENTS_DICT: &str = "commitments";
pub const KEY_COMMITMENT_COUNT: &str = "commitment_count";

/// Named key under the *deploying account* pointing at this contract's
/// hash -- see risk-registry/src/constants.rs's identical comment.
pub const CONTRACT_HASH_KEY_NAME: &str = "disbursement_controller_contract_hash";

/// Named keys *within* the contract's own package.
pub const PACKAGE_HASH_KEY_NAME: &str = "disbursement_controller_package_hash";
pub const ACCESS_UREF_KEY_NAME: &str = "disbursement_controller_access_uref";

// Entry point names on *this* contract.
pub const ENTRY_POINT_SET_ATTESTER: &str = "set_attester";
pub const ENTRY_POINT_PROPOSE_NEW_ADMIN: &str = "propose_new_admin";
pub const ENTRY_POINT_ACCEPT_ADMIN: &str = "accept_admin";
pub const ENTRY_POINT_CREATE_COMMITMENT: &str = "create_commitment";
pub const ENTRY_POINT_ATTEST_MILESTONE: &str = "attest_milestone";
pub const ENTRY_POINT_RELEASE_MILESTONE: &str = "release_milestone";
pub const ENTRY_POINT_CANCEL_COMMITMENT: &str = "cancel_commitment";
pub const ENTRY_POINT_IS_ATTESTER: &str = "is_attester";
pub const ENTRY_POINT_COMMITMENT_COUNT: &str = "commitment_count";
pub const ENTRY_POINT_GET_COMMITMENT: &str = "get_commitment";
pub const ENTRY_POINT_GET_MILESTONE: &str = "get_milestone";
/// Self-initializing entry point -- see risk-registry/src/main.rs's
/// `init()` doc comment for the full explanation.
pub const ENTRY_POINT_INIT: &str = "init";

// Entry point names on *other* contracts this one calls into --
// identity-registry's and d3rac-token's own, must match those crates'
// constants.rs exactly (there's no shared crate between them to
// enforce this at compile time, same as the TRON contract's own
// `ITRC20` local-interface pattern doesn't get compile-time checking
// against the real `D3RACToken.sol` either).
pub const REMOTE_ENTRY_POINT_IS_VERIFIED: &str = "is_verified";
pub const REMOTE_ARG_RECIPIENT: &str = "recipient";
pub const REMOTE_ENTRY_POINT_BALANCE_OF: &str = "balance_of";
pub const REMOTE_ARG_ACCOUNT: &str = "account";
pub const REMOTE_ENTRY_POINT_TRANSFER: &str = "transfer";
pub const REMOTE_ARG_AMOUNT: &str = "amount";

// Runtime argument names on *this* contract.
pub const ARG_REGISTRY_PACKAGE_HASH: &str = "registry_package_hash";
pub const ARG_ADMIN: &str = "admin";
pub const ARG_NEW_ADMIN: &str = "new_admin";
pub const ARG_ACCOUNT: &str = "account";
pub const ARG_IS_ATTESTER: &str = "is_attester";
pub const ARG_RECIPIENT: &str = "recipient";
pub const ARG_TOKEN_PACKAGE_HASH: &str = "token_package_hash";
pub const ARG_COMMUNITY: &str = "community";
pub const ARG_DESCRIPTIONS: &str = "descriptions";
pub const ARG_AMOUNTS: &str = "amounts";
pub const ARG_COMMITMENT_ID: &str = "commitment_id";
pub const ARG_MILESTONE_INDEX: &str = "milestone_index";
