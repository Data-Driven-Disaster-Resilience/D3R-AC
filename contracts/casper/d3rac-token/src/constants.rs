//! Constants for named keys, entry-point names, and runtime argument
//! names. See risk-registry/src/constants.rs's module comment for why
//! these are centralized -- same reasoning applies here.
//!
//! Entry-point names follow the CEP-18 standard naming
//! (`transfer`/`approve`/`transfer_from`/`balance_of`/`allowance`/
//! `total_supply`) per `docs/casper-contracts-srs.md` FR-1, not this
//! suite's own `snake_case`-of-the-Solidity-name convention the other
//! contracts use -- CEP-18 parity is the explicit requirement here,
//! so entry points that exist on the standard use its names exactly;
//! the non-standard `mint`/`set_minter`/ownership pair (which have no
//! CEP-18 standard name to match) fall back to this suite's usual
//! convention.

// Named keys (contract storage)
pub const KEY_NAME: &str = "name";
pub const KEY_SYMBOL: &str = "symbol";
pub const KEY_DECIMALS: &str = "decimals";
pub const KEY_TOTAL_SUPPLY: &str = "total_supply";
pub const KEY_OWNER: &str = "owner";
pub const KEY_PENDING_OWNER: &str = "pending_owner";
pub const KEY_BALANCES_DICT: &str = "balances";
pub const KEY_ALLOWANCES_DICT: &str = "allowances";
pub const KEY_MINTERS_DICT: &str = "minters";

/// Named key under the *deploying account* pointing at this contract's
/// hash -- see risk-registry/src/constants.rs's identical comment.
pub const CONTRACT_HASH_KEY_NAME: &str = "d3rac_token_contract_hash";

/// Named keys *within* the contract's own package.
pub const PACKAGE_HASH_KEY_NAME: &str = "d3rac_token_package_hash";
pub const ACCESS_UREF_KEY_NAME: &str = "d3rac_token_access_uref";

// Entry point names -- CEP-18 standard surface first, then this
// suite's own additions (see module comment).
pub const ENTRY_POINT_NAME: &str = "name";
pub const ENTRY_POINT_SYMBOL: &str = "symbol";
pub const ENTRY_POINT_DECIMALS: &str = "decimals";
pub const ENTRY_POINT_TOTAL_SUPPLY: &str = "total_supply";
pub const ENTRY_POINT_BALANCE_OF: &str = "balance_of";
pub const ENTRY_POINT_ALLOWANCE: &str = "allowance";
pub const ENTRY_POINT_TRANSFER: &str = "transfer";
pub const ENTRY_POINT_APPROVE: &str = "approve";
pub const ENTRY_POINT_TRANSFER_FROM: &str = "transfer_from";
pub const ENTRY_POINT_MINT: &str = "mint";
pub const ENTRY_POINT_BURN: &str = "burn";
pub const ENTRY_POINT_SET_MINTER: &str = "set_minter";
pub const ENTRY_POINT_IS_MINTER: &str = "is_minter";
pub const ENTRY_POINT_PROPOSE_NEW_OWNER: &str = "propose_new_owner";
pub const ENTRY_POINT_ACCEPT_OWNERSHIP: &str = "accept_ownership";
/// Self-initializing entry point -- see risk-registry/src/main.rs's
/// `init()` doc comment for the full explanation (CES bookkeeping keys
/// must be written in the contract's own execution context, not the
/// installing account's).
pub const ENTRY_POINT_INIT: &str = "init";

// Runtime argument names
pub const ARG_OWNER: &str = "owner";
pub const ARG_INITIAL_SUPPLY: &str = "initial_supply";
pub const ARG_ACCOUNT: &str = "account";
pub const ARG_SPENDER: &str = "spender";
pub const ARG_RECIPIENT: &str = "recipient";
pub const ARG_AMOUNT: &str = "amount";
pub const ARG_CAN_MINT: &str = "can_mint";
pub const ARG_NEW_OWNER: &str = "new_owner";
