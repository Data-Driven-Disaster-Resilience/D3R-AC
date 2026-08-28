//! Constants for named keys, entry-point names, and runtime argument
//! names -- see identity-registry/src/constants.rs for why these are
//! centralized. Simple-value named keys (name/symbol/decimals/
//! total_supply) and entry-point/arg names for the standard surface
//! match the CEP-18 spec's own naming exactly
//! (ceps/text/0018-token-standard.md) -- these are load-bearing for
//! interop with anything that expects a real CEP-18 token, not just
//! internal convention.

// Named keys (contract storage) -- "name", "symbol", "decimals",
// "total_supply", "balances", "allowances" are the CEP-18 standard's
// own storage-interface key names verbatim. See main.rs's module
// comment for the one deliberate point of non-compliance (dictionary
// key derivation, not key *names*).
pub const KEY_NAME: &str = "name";
pub const KEY_SYMBOL: &str = "symbol";
pub const KEY_DECIMALS: &str = "decimals";
pub const KEY_TOTAL_SUPPLY: &str = "total_supply";
pub const KEY_BALANCES_DICT: &str = "balances";
pub const KEY_ALLOWANCES_DICT: &str = "allowances";

// Extension storage (D3RACToken.sol's own additions beyond the CEP-18
// standard: ownership + minter role).
pub const KEY_OWNER: &str = "owner";
pub const KEY_PENDING_OWNER: &str = "pending_owner";
pub const KEY_MINTERS_DICT: &str = "minters";

pub const CONTRACT_HASH_KEY_NAME: &str = "d3rac_token_contract_hash";
pub const PACKAGE_HASH_KEY_NAME: &str = "d3rac_token_package_hash";
pub const ACCESS_UREF_KEY_NAME: &str = "d3rac_token_access_uref";

// Entry point names -- the 11 standard ones (CEP18Interface trait, see
// module comment) match the spec's own fn names verbatim.
pub const ENTRY_POINT_NAME: &str = "name";
pub const ENTRY_POINT_SYMBOL: &str = "symbol";
pub const ENTRY_POINT_DECIMALS: &str = "decimals";
pub const ENTRY_POINT_TOTAL_SUPPLY: &str = "total_supply";
pub const ENTRY_POINT_BALANCE_OF: &str = "balance_of";
pub const ENTRY_POINT_ALLOWANCE: &str = "allowance";
pub const ENTRY_POINT_TRANSFER: &str = "transfer";
pub const ENTRY_POINT_TRANSFER_FROM: &str = "transfer_from";
pub const ENTRY_POINT_APPROVE: &str = "approve";
pub const ENTRY_POINT_DECREASE_ALLOWANCE: &str = "decrease_allowance";
pub const ENTRY_POINT_INCREASE_ALLOWANCE: &str = "increase_allowance";

// Extension entry points (D3RACToken.sol's own additions).
pub const ENTRY_POINT_MINT: &str = "mint";
pub const ENTRY_POINT_BURN: &str = "burn";
pub const ENTRY_POINT_SET_MINTER: &str = "set_minter";
pub const ENTRY_POINT_IS_MINTER: &str = "is_minter";
pub const ENTRY_POINT_PROPOSE_NEW_OWNER: &str = "propose_new_owner";
pub const ENTRY_POINT_ACCEPT_OWNERSHIP: &str = "accept_ownership";
pub const ENTRY_POINT_INIT: &str = "init";

// Runtime argument names -- "recipient"/"amount"/"owner"/"spender"/
// "account"/"inc_by"/"decr_by" match the CEP-18 spec's own fn
// signatures verbatim (see module comment).
pub const ARG_ACCOUNT: &str = "account";
pub const ARG_OWNER: &str = "owner";
pub const ARG_SPENDER: &str = "spender";
pub const ARG_RECIPIENT: &str = "recipient";
pub const ARG_AMOUNT: &str = "amount";
pub const ARG_INC_BY: &str = "inc_by";
pub const ARG_DECR_BY: &str = "decr_by";
pub const ARG_IS_MINTER: &str = "is_minter";
pub const ARG_NEW_OWNER: &str = "new_owner";
pub const ARG_INITIAL_SUPPLY: &str = "initial_supply";
pub const ARG_OWNER_ARG: &str = "owner_"; // installer's own owner_ constructor arg
