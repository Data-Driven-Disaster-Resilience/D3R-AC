//! Constants for named keys, entry-point names, and runtime argument
//! names. Centralized so the installer (`call`) and every entry point
//! reference the exact same strings -- a typo'd key name here is a
//! silent runtime failure on Casper, not a compile error, since these
//! are all plain string lookups.

/// Fixed-point scale, matching `RiskRegistry.sol::SCALE` (1e18) and
/// `docs/risk-model.md` exactly. H, E, V, and R are all represented in
/// `[0, SCALE]` for `[0.0, 1.0]`.
pub const SCALE: u64 = 1_000_000_000_000_000_000;

// Named keys (contract storage)
pub const KEY_OWNER: &str = "owner";
pub const KEY_RISK_THRESHOLD: &str = "risk_threshold";
pub const KEY_COMMUNITIES_DICT: &str = "communities";
pub const KEY_DATA_FEEDERS_DICT: &str = "data_feeders";
pub const KEY_COMMUNITY_IDS: &str = "community_ids";

/// Named key under the *deploying account* (not the contract itself)
/// pointing at this contract's hash -- the installer's own record of
/// what it just deployed, same idea as a deploy script logging the
/// resulting address on TRON.
pub const CONTRACT_HASH_KEY_NAME: &str = "risk_registry_contract_hash";
pub const CONTRACT_PACKAGE_HASH_KEY_NAME: &str = "risk_registry_contract_package_hash";

/// Named keys *within* the contract's own package, referenced by
/// `storage::new_locked_contract`'s optional name arguments.
pub const PACKAGE_HASH_KEY_NAME: &str = "risk_registry_package_hash";
pub const ACCESS_UREF_KEY_NAME: &str = "risk_registry_access_uref";

// Entry point names
pub const ENTRY_POINT_REGISTER_COMMUNITY: &str = "register_community";
pub const ENTRY_POINT_UPDATE_RISK: &str = "update_risk";
pub const ENTRY_POINT_RISK_SCORE: &str = "risk_score";
pub const ENTRY_POINT_IS_ABOVE_THRESHOLD: &str = "is_above_threshold";
pub const ENTRY_POINT_GET_COMMUNITY: &str = "get_community";
pub const ENTRY_POINT_COMMUNITY_COUNT: &str = "community_count";
pub const ENTRY_POINT_TRANSFER_OWNERSHIP: &str = "transfer_ownership";
pub const ENTRY_POINT_ADD_DATA_FEEDER: &str = "add_data_feeder";
pub const ENTRY_POINT_REMOVE_DATA_FEEDER: &str = "remove_data_feeder";
pub const ENTRY_POINT_SET_RISK_THRESHOLD: &str = "set_risk_threshold";

// Runtime argument names
pub const ARG_INITIAL_THRESHOLD: &str = "initial_threshold";
pub const ARG_INITIAL_DATA_FEEDER: &str = "initial_data_feeder";
pub const ARG_COMMUNITY_ID: &str = "community_id";
pub const ARG_NAME: &str = "name";
pub const ARG_REGION: &str = "region";
pub const ARG_HAZARD: &str = "hazard";
pub const ARG_EXPOSURE: &str = "exposure";
pub const ARG_VULNERABILITY: &str = "vulnerability";
pub const ARG_NEW_OWNER: &str = "new_owner";
pub const ARG_FEEDER: &str = "feeder";
pub const ARG_NEW_THRESHOLD: &str = "new_threshold";
