//! Constants for named keys, entry-point names, and runtime argument
//! names. See risk-registry/src/constants.rs's module comment for why
//! these are centralized -- same reasoning applies here.

// Named keys (contract storage)
pub const KEY_OWNERS: &str = "owners";
pub const KEY_THRESHOLD: &str = "threshold";
pub const KEY_OWNERS_DICT: &str = "is_owner";
pub const KEY_TRANSACTIONS_DICT: &str = "transactions";
pub const KEY_TX_COUNT: &str = "tx_count";
pub const KEY_CONFIRMATIONS_DICT: &str = "confirmations";

/// Named key under the *deploying account* pointing at this contract's
/// hash -- see risk-registry/src/constants.rs's identical comment.
pub const CONTRACT_HASH_KEY_NAME: &str = "multisig_admin_contract_hash";

/// Named keys *within* the contract's own package.
pub const PACKAGE_HASH_KEY_NAME: &str = "multisig_admin_package_hash";
pub const ACCESS_UREF_KEY_NAME: &str = "multisig_admin_access_uref";

// Entry point names
pub const ENTRY_POINT_SUBMIT_TRANSACTION: &str = "submit_transaction";
pub const ENTRY_POINT_CONFIRM_TRANSACTION: &str = "confirm_transaction";
pub const ENTRY_POINT_REVOKE_CONFIRMATION: &str = "revoke_confirmation";
pub const ENTRY_POINT_EXECUTE_TRANSACTION: &str = "execute_transaction";
pub const ENTRY_POINT_IS_OWNER: &str = "is_owner";
pub const ENTRY_POINT_OWNER_COUNT: &str = "owner_count";
pub const ENTRY_POINT_TRANSACTION_COUNT: &str = "transaction_count";
pub const ENTRY_POINT_IS_CONFIRMED: &str = "is_confirmed";
pub const ENTRY_POINT_GET_TRANSACTION: &str = "get_transaction";
/// Self-initializing entry point -- see risk-registry/src/main.rs's
/// `init()` doc comment for the full explanation (CES bookkeeping keys
/// must be written in the contract's own execution context, not the
/// installing account's).
pub const ENTRY_POINT_INIT: &str = "init";

// Runtime argument names
pub const ARG_OWNERS: &str = "owners";
pub const ARG_THRESHOLD: &str = "threshold";
pub const ARG_TARGET_PACKAGE_HASH: &str = "target_package_hash";
pub const ARG_TARGET_ENTRY_POINT: &str = "target_entry_point";
/// Bytesrepr-serialized `RuntimeArgs` for the target call -- see
/// main.rs's `execute_transaction` doc comment for why this is passed
/// as opaque, pre-serialized bytes rather than a fixed argument list.
pub const ARG_TARGET_ARGS_BYTES: &str = "target_args_bytes";
pub const ARG_TX_ID: &str = "tx_id";
pub const ARG_OWNER: &str = "owner";
