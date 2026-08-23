//! Constants for named keys, entry-point names, and runtime argument
//! names. See risk-registry/src/constants.rs's module comment for why
//! these are centralized -- same reasoning applies here.

// Named keys (contract storage)
pub const KEY_ADMIN: &str = "admin";
pub const KEY_PENDING_ADMIN: &str = "pending_admin";
pub const KEY_VERIFIERS_DICT: &str = "verifiers";
pub const KEY_RECIPIENTS_DICT: &str = "recipients";

/// Named key under the *deploying account* pointing at this contract's
/// hash -- see risk-registry/src/constants.rs's identical comment.
pub const CONTRACT_HASH_KEY_NAME: &str = "identity_registry_contract_hash";

/// Named keys *within* the contract's own package.
pub const PACKAGE_HASH_KEY_NAME: &str = "identity_registry_package_hash";
pub const ACCESS_UREF_KEY_NAME: &str = "identity_registry_access_uref";

// Entry point names
pub const ENTRY_POINT_SET_VERIFIER: &str = "set_verifier";
pub const ENTRY_POINT_PROPOSE_NEW_ADMIN: &str = "propose_new_admin";
pub const ENTRY_POINT_ACCEPT_ADMIN: &str = "accept_admin";
pub const ENTRY_POINT_VERIFY_RECIPIENT: &str = "verify_recipient";
pub const ENTRY_POINT_REVOKE_RECIPIENT: &str = "revoke_recipient";
pub const ENTRY_POINT_IS_VERIFIED: &str = "is_verified";
pub const ENTRY_POINT_GET_RECIPIENT: &str = "get_recipient";
pub const ENTRY_POINT_IS_VERIFIER: &str = "is_verifier";
/// Self-initializing entry point -- see risk-registry/src/main.rs's
/// `init()` doc comment for the full explanation (CES bookkeeping keys
/// must be written in the contract's own execution context, not the
/// installing account's).
pub const ENTRY_POINT_INIT: &str = "init";

// Runtime argument names
pub const ARG_ACCOUNT: &str = "account";
pub const ARG_IS_VERIFIER: &str = "is_verifier";
pub const ARG_NEW_ADMIN: &str = "new_admin";
pub const ARG_RECIPIENT: &str = "recipient";
pub const ARG_COMMUNITY: &str = "community";
pub const ARG_INITIAL_VERIFIER: &str = "initial_verifier";
