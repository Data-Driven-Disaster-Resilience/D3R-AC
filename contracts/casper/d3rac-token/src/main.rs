//! D3RACToken — Casper port of
//! `contracts/tron/tronbox/contracts/D3RACToken.sol`.
//!
//! Behavioral parity target: CEP-18-standard entry points
//! (`transfer`/`approve`/`transfer_from`/`balance_of`/`allowance`/
//! `total_supply`, plus `name`/`symbol`/`decimals`) matching
//! `docs/casper-contracts-srs.md` FR-1, with the same non-standard
//! owner-gated `mint`/`set_minter` pair and two-step ownership
//! (`propose_new_owner`/`accept_ownership`) `D3RACToken.sol` adds on
//! top of the base standard. Uses `casper_types::U256` for amounts,
//! matching Solidity's `uint256` -- unlike the network's own native
//! CSPR token (which Casper contracts don't need special handling for
//! here, since this is a fungible-token *contract*, not a purse-based
//! transfer).
//!
//! Standard CEP-18 semantics (matching the reference implementation,
//! not `D3RACToken.sol`'s ERC-20-style `bool` returns): `transfer`/
//! `approve`/`transfer_from`/`mint`/`burn` all revert on failure and
//! return `Unit`, not `bool` -- there is no "returns false" outcome to
//! report on a successful call the way ERC-20 nominally allows,
//! consistent with how every other guard in this suite already works
//! (a failed `require` reverts the whole call, full stop).
//!
//! Same boilerplate, same design decisions, same hard-won lessons as
//! `risk-registry/src/main.rs` -- see that file's own module comment
//! for the detailed rationale behind the global allocator/panic
//! handler, `is_locked = true`, AccountHash-normalized addressing, the
//! `init()` self-initialization pattern, and the exact
//! `new_locked_contract` 5-argument signature. Not re-derived here;
//! this file follows the same template, adapted for this contract's
//! own entry points and storage shape.
//!
//! NOT yet independently confirmed compiling to wasm32-unknown-unknown
//! in CI -- unlike `multisig-admin` (PR #20), which went through two
//! real CI-caught-and-fixed rounds before merging, this file hasn't
//! had a CI round at all yet. Written against the same now-twice-
//! confirmed casper-types 6.1.0 API surface `multisig-admin` needed
//! fixes for (`contracts::` module paths, `bytesrepr::FromBytes` needing
//! to be in scope, `Key::into_hash_addr` not `into_hash`), but that's
//! not a substitute for this file's own CI round. See
//! `contracts/casper/README.md` for current, itemized status.

#![no_std]
#![no_main]

extern crate alloc;

#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    #[cfg(target_arch = "wasm32")]
    core::arch::wasm32::unreachable();
    #[cfg(not(target_arch = "wasm32"))]
    loop {}
}

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use casper_contract::contract_api::{runtime, storage};
use casper_contract::unwrap_or_revert::UnwrapOrRevert;
use casper_event_standard::Schemas;
use casper_types::{
    contracts::{EntryPoint, NamedKeys},
    runtime_args, CLType, CLValue, EntryPointAccess, EntryPointType, EntryPoints, Key, Parameter,
    URef, U256,
};

mod constants;
mod error;
mod events;

use constants::*;
use error::D3racTokenError;
use events::{
    Approval, MinterUpdated, OwnershipTransferProposed, OwnershipTransferred, Transfer,
};

/// Matches `D3RACToken.sol`'s `name = "D3R-AC Relief Token"` -- a
/// compile-time constant, not stored state, since it's never mutated
/// after deployment (same reasoning applies to `SYMBOL`/`DECIMALS`
/// below -- storing an immutable value in a `URef` just to read it
/// back unchanged would be pure overhead).
const TOKEN_NAME: &str = "D3R-AC Relief Token";
const TOKEN_SYMBOL: &str = "D3RAC";
const TOKEN_DECIMALS: u8 = 18;

// ---------------------------------------------------------------
// Entry points -- CEP-18 standard surface
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn name() {
    runtime::ret(CLValue::from_t(TOKEN_NAME.to_string()).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn symbol() {
    runtime::ret(CLValue::from_t(TOKEN_SYMBOL.to_string()).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn decimals() {
    runtime::ret(CLValue::from_t(TOKEN_DECIMALS).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn total_supply() {
    runtime::ret(CLValue::from_t(get_total_supply()).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn balance_of() {
    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    runtime::ret(CLValue::from_t(get_balance(account)).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn allowance() {
    let owner: Key = runtime::get_named_arg(ARG_OWNER);
    let spender: Key = runtime::get_named_arg(ARG_SPENDER);
    runtime::ret(CLValue::from_t(get_allowance(owner, spender)).unwrap_or_revert());
}

/// Same guard/semantics as `D3RACToken.sol::transfer` ->
/// `_transfer(msg.sender, to, value)`. CEP-18-standard: reverts on
/// failure, returns `Unit` -- see module comment on why there's no
/// `bool` return the way the TRON contract has one.
#[no_mangle]
pub extern "C" fn transfer() {
    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let amount: U256 = runtime::get_named_arg(ARG_AMOUNT);
    let caller = Key::from(runtime::get_caller());
    do_transfer(caller, recipient, amount);
}

/// Same guard/semantics as `D3RACToken.sol::approve` ->
/// `_approve(msg.sender, spender, value)`.
#[no_mangle]
pub extern "C" fn approve() {
    let spender: Key = runtime::get_named_arg(ARG_SPENDER);
    let amount: U256 = runtime::get_named_arg(ARG_AMOUNT);
    let caller = Key::from(runtime::get_caller());
    set_allowance(caller, spender, amount);
    casper_event_standard::emit(Approval {
        owner: caller,
        spender,
        value: amount,
    });
}

/// Same guard/semantics as `D3RACToken.sol::transferFrom`.
#[no_mangle]
pub extern "C" fn transfer_from() {
    let owner: Key = runtime::get_named_arg(ARG_OWNER);
    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let amount: U256 = runtime::get_named_arg(ARG_AMOUNT);
    let caller = Key::from(runtime::get_caller());

    let current_allowance = get_allowance(owner, caller);
    if current_allowance < amount {
        runtime::revert(D3racTokenError::InsufficientAllowance);
    }
    // Matches `D3RACToken.sol::transferFrom`'s `unchecked { _approve(from,
    // msg.sender, currentAllowance - value); }` -- the subtraction can't
    // underflow given the guard just above, same reasoning the TRON
    // contract's own `unchecked` block relies on.
    set_allowance(owner, caller, current_allowance - amount);

    do_transfer(owner, recipient, amount);
}

// ---------------------------------------------------------------
// Entry points -- non-standard additions (mint/minter/ownership)
// ---------------------------------------------------------------

/// Minter-role-gated. Same guard/semantics as
/// `D3RACToken.sol::mint`.
#[no_mangle]
pub extern "C" fn mint() {
    only_minter();

    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let amount: U256 = runtime::get_named_arg(ARG_AMOUNT);

    set_total_supply(get_total_supply() + amount);
    set_balance(recipient, get_balance(recipient) + amount);

    casper_event_standard::emit(Transfer {
        from: zero_key(),
        to: recipient,
        value: amount,
    });
}

/// Same guard/semantics as `D3RACToken.sol::burn` ->
/// `_burn(msg.sender, value)`. Burns from the caller's own balance --
/// no `burn_from`/allowance-gated burn exists in the TRON contract
/// either, so none is added here.
#[no_mangle]
pub extern "C" fn burn() {
    let amount: U256 = runtime::get_named_arg(ARG_AMOUNT);
    let caller = Key::from(runtime::get_caller());

    let balance = get_balance(caller);
    if balance < amount {
        runtime::revert(D3racTokenError::BurnExceedsBalance);
    }
    set_balance(caller, balance - amount);
    set_total_supply(get_total_supply() - amount);

    casper_event_standard::emit(Transfer {
        from: caller,
        to: zero_key(),
        value: amount,
    });
}

/// Owner-only. Same guard/semantics as `D3RACToken.sol::setMinter`.
#[no_mangle]
pub extern "C" fn set_minter() {
    only_owner();

    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    let can_mint: bool = runtime::get_named_arg(ARG_CAN_MINT);

    storage::dictionary_put(minters_dict(), &key_to_dict_key(&account), can_mint);

    casper_event_standard::emit(MinterUpdated { account, can_mint });
}

#[no_mangle]
pub extern "C" fn is_minter() {
    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    runtime::ret(CLValue::from_t(is_minter_internal(account)).unwrap_or_revert());
}

/// Step 1 of ownership transfer. Owner-only. Same semantics as
/// `D3RACToken.sol::proposeNewOwner`.
#[no_mangle]
pub extern "C" fn propose_new_owner() {
    only_owner();

    let new_owner: Key = runtime::get_named_arg(ARG_NEW_OWNER);
    let current_owner = get_owner();

    set_pending_owner(Some(new_owner));

    casper_event_standard::emit(OwnershipTransferProposed {
        current_owner,
        proposed_owner: new_owner,
    });
}

/// Step 2: the proposed owner claims the role themselves. Same
/// semantics as `D3RACToken.sol::acceptOwnership`.
#[no_mangle]
pub extern "C" fn accept_ownership() {
    let caller = Key::from(runtime::get_caller());
    let pending = get_pending_owner();

    match pending {
        Some(pending_owner) if pending_owner == caller => {
            let previous_owner = get_owner();
            set_owner(pending_owner);
            set_pending_owner(None);
            casper_event_standard::emit(OwnershipTransferred {
                previous_owner: Some(previous_owner),
                new_owner: pending_owner,
            });
        }
        _ => runtime::revert(D3racTokenError::CallerIsNotPendingOwner),
    }
}

// ---------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------

fn only_owner() {
    let caller = Key::from(runtime::get_caller());
    if caller != get_owner() {
        runtime::revert(D3racTokenError::CallerIsNotOwner);
    }
}

fn only_minter() {
    let caller = Key::from(runtime::get_caller());
    if !is_minter_internal(caller) {
        runtime::revert(D3racTokenError::CallerIsNotMinter);
    }
}

fn is_minter_internal(account: Key) -> bool {
    storage::dictionary_get(minters_dict(), &key_to_dict_key(&account))
        .unwrap_or_revert_with(D3racTokenError::DictionaryReadFailed)
        .unwrap_or(false)
}

/// Matches `D3RACToken.sol::_transfer` exactly, including which guard
/// runs first (balance check, then the write).
fn do_transfer(from: Key, to: Key, amount: U256) {
    let from_balance = get_balance(from);
    if from_balance < amount {
        runtime::revert(D3racTokenError::InsufficientBalance);
    }
    set_balance(from, from_balance - amount);
    set_balance(to, get_balance(to) + amount);

    casper_event_standard::emit(Transfer {
        from,
        to,
        value: amount,
    });
}

fn get_balance(account: Key) -> U256 {
    storage::dictionary_get(balances_dict(), &key_to_dict_key(&account))
        .unwrap_or_revert_with(D3racTokenError::DictionaryReadFailed)
        .unwrap_or(U256::zero())
}

fn set_balance(account: Key, value: U256) {
    storage::dictionary_put(balances_dict(), &key_to_dict_key(&account), value);
}

fn get_allowance(owner: Key, spender: Key) -> U256 {
    storage::dictionary_get(allowances_dict(), &allowance_dict_key(&owner, &spender))
        .unwrap_or_revert_with(D3racTokenError::DictionaryReadFailed)
        .unwrap_or(U256::zero())
}

fn set_allowance(owner: Key, spender: Key, value: U256) {
    storage::dictionary_put(
        allowances_dict(),
        &allowance_dict_key(&owner, &spender),
        value,
    );
}

fn allowance_dict_key(owner: &Key, spender: &Key) -> String {
    // Composite key -- see multisig-admin/src/main.rs's
    // `confirmation_dict_key` for the identical reasoning (Casper
    // dictionaries are single-level, so `mapping(address => mapping(...))`
    // flattens into one dictionary with a composite string key).
    let mut s = owner.to_string();
    s.push(':');
    s.push_str(&spender.to_string());
    s
}

fn key_to_dict_key(key: &Key) -> String {
    key.to_string()
}

/// Casper has no zero-address concept the way EVM/TVM does -- see
/// identity-registry's identical reasoning on `NewAdminInvalid`. Used
/// here purely as the conventional "from"/"to" value on `Transfer`
/// events for mint/burn, matching `D3RACToken.sol`'s own
/// `address(0)` usage in `Transfer(address(0), to, value)` /
/// `Transfer(from, address(0), value)` -- an event-log convention,
/// not a real spendable account.
fn zero_key() -> Key {
    Key::from(casper_types::account::AccountHash::new([0u8; 32]))
}

fn get_total_supply() -> U256 {
    let uref: URef = runtime::get_key(KEY_TOTAL_SUPPLY)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(D3racTokenError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
}

fn set_total_supply(value: U256) {
    let uref: URef = runtime::get_key(KEY_TOTAL_SUPPLY)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(D3racTokenError::UnexpectedKeyType);
    storage::write(uref, value);
}

fn get_owner() -> Key {
    let uref: URef = runtime::get_key(KEY_OWNER)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(D3racTokenError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
}

fn set_owner(new_owner: Key) {
    let uref: URef = runtime::get_key(KEY_OWNER)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(D3racTokenError::UnexpectedKeyType);
    storage::write(uref, new_owner);
}

fn get_pending_owner() -> Option<Key> {
    let uref: URef = runtime::get_key(KEY_PENDING_OWNER)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(D3racTokenError::UnexpectedKeyType);
    // See identity-registry/src/main.rs's `get_pending_admin` comment
    // for why this needs exactly two `unwrap_or_revert_with` calls --
    // identical reasoning (T here is itself `Option<Key>`).
    storage::read(uref)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
}

fn set_pending_owner(pending: Option<Key>) {
    let uref: URef = runtime::get_key(KEY_PENDING_OWNER)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(D3racTokenError::UnexpectedKeyType);
    storage::write(uref, pending);
}

fn balances_dict() -> URef {
    *runtime::get_key(KEY_BALANCES_DICT)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(D3racTokenError::UnexpectedKeyType)
}

fn allowances_dict() -> URef {
    *runtime::get_key(KEY_ALLOWANCES_DICT)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(D3racTokenError::UnexpectedKeyType)
}

fn minters_dict() -> URef {
    *runtime::get_key(KEY_MINTERS_DICT)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(D3racTokenError::UnexpectedKeyType)
}

// ---------------------------------------------------------------
// Installer (`call`)
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn call() {
    let initial_supply: U256 = runtime::get_named_arg(ARG_INITIAL_SUPPLY);
    let owner: Key = runtime::get_named_arg(ARG_OWNER);

    // Matches `D3RACToken.sol`'s constructor: `initialSupply * (10 **
    // decimals)` -- scaled from whole tokens to the smallest unit,
    // same as the TRON contract's own comment describes.
    let scale = U256::from(10u64).pow(U256::from(TOKEN_DECIMALS));
    let scaled_supply = initial_supply
        .checked_mul(scale)
        .unwrap_or_revert_with(D3racTokenError::SupplyOverflow);

    let mut named_keys = NamedKeys::new();

    named_keys.insert(KEY_OWNER.to_string(), storage::new_uref(owner).into());
    named_keys.insert(
        KEY_PENDING_OWNER.to_string(),
        storage::new_uref(Option::<Key>::None).into(),
    );
    named_keys.insert(
        KEY_TOTAL_SUPPLY.to_string(),
        storage::new_uref(scaled_supply).into(),
    );

    let balances_dict_uref = storage::new_dictionary(KEY_BALANCES_DICT).unwrap_or_revert();
    named_keys.insert(KEY_BALANCES_DICT.to_string(), balances_dict_uref.into());

    let allowances_dict_uref = storage::new_dictionary(KEY_ALLOWANCES_DICT).unwrap_or_revert();
    named_keys.insert(
        KEY_ALLOWANCES_DICT.to_string(),
        allowances_dict_uref.into(),
    );

    let minters_dict_uref = storage::new_dictionary(KEY_MINTERS_DICT).unwrap_or_revert();
    named_keys.insert(KEY_MINTERS_DICT.to_string(), minters_dict_uref.into());

    // Owner is always implicitly a minter at install, matching
    // `D3RACToken.sol`'s constructor (`_grantRole(MINTER_ROLE,
    // owner_)`) exactly.
    storage::dictionary_put(minters_dict_uref, &key_to_dict_key(&owner), true);

    if scaled_supply > U256::zero() {
        storage::dictionary_put(balances_dict_uref, &key_to_dict_key(&owner), scaled_supply);
    }

    let entry_points = build_entry_points();

    let (contract_hash, _contract_version) = storage::new_locked_contract(
        entry_points,
        Some(named_keys),
        Some(PACKAGE_HASH_KEY_NAME.to_string()),
        Some(ACCESS_UREF_KEY_NAME.to_string()),
        None,
    );

    // See risk-registry/src/main.rs's `init()` doc comment for why CES
    // schema registration (and, there, the initial-role event) must
    // run via `runtime::call_contract` in the contract's own context
    // rather than directly from `call()`'s body.
    runtime::call_contract::<()>(
        contract_hash,
        ENTRY_POINT_INIT,
        runtime_args! {
            ARG_OWNER => owner,
            ARG_INITIAL_SUPPLY => scaled_supply,
        },
    );

    runtime::put_key(CONTRACT_HASH_KEY_NAME, contract_hash.into());
}

/// Self-initializing entry point -- see risk-registry/src/main.rs's
/// `init()` for the full explanation (same real CI-caught bug class
/// this pattern was already fixed against there).
#[no_mangle]
pub extern "C" fn init() {
    let schemas = Schemas::new()
        .with::<Transfer>()
        .with::<Approval>()
        .with::<OwnershipTransferred>()
        .with::<OwnershipTransferProposed>()
        .with::<MinterUpdated>();
    casper_event_standard::init(schemas);

    // Emit the install-time owner/minter grant and initial-supply
    // mint from inside the contract's own context, same reasoning as
    // identity-registry's initial-verifier event.
    let owner: Key = runtime::get_named_arg(ARG_OWNER);
    let initial_supply: U256 = runtime::get_named_arg(ARG_INITIAL_SUPPLY);

    casper_event_standard::emit(OwnershipTransferred {
        previous_owner: None,
        new_owner: owner,
    });
    casper_event_standard::emit(MinterUpdated {
        account: owner,
        can_mint: true,
    });
    if initial_supply > U256::zero() {
        casper_event_standard::emit(Transfer {
            from: zero_key(),
            to: owner,
            value: initial_supply,
        });
    }
}

fn build_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_NAME,
            Vec::new(),
            CLType::String,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_SYMBOL,
            Vec::new(),
            CLType::String,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_DECIMALS,
            Vec::new(),
            CLType::U8,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_TOTAL_SUPPLY,
            Vec::new(),
            CLType::U256,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_BALANCE_OF,
            vec![Parameter::new(ARG_ACCOUNT, CLType::Key)],
            CLType::U256,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_ALLOWANCE,
            vec![
                Parameter::new(ARG_OWNER, CLType::Key),
                Parameter::new(ARG_SPENDER, CLType::Key),
            ],
            CLType::U256,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_TRANSFER,
            vec![
                Parameter::new(ARG_RECIPIENT, CLType::Key),
                Parameter::new(ARG_AMOUNT, CLType::U256),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_APPROVE,
            vec![
                Parameter::new(ARG_SPENDER, CLType::Key),
                Parameter::new(ARG_AMOUNT, CLType::U256),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_TRANSFER_FROM,
            vec![
                Parameter::new(ARG_OWNER, CLType::Key),
                Parameter::new(ARG_RECIPIENT, CLType::Key),
                Parameter::new(ARG_AMOUNT, CLType::U256),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_MINT,
            vec![
                Parameter::new(ARG_RECIPIENT, CLType::Key),
                Parameter::new(ARG_AMOUNT, CLType::U256),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_BURN,
            vec![Parameter::new(ARG_AMOUNT, CLType::U256)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_SET_MINTER,
            vec![
                Parameter::new(ARG_ACCOUNT, CLType::Key),
                Parameter::new(ARG_CAN_MINT, CLType::Bool),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_IS_MINTER,
            vec![Parameter::new(ARG_ACCOUNT, CLType::Key)],
            CLType::Bool,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_PROPOSE_NEW_OWNER,
            vec![Parameter::new(ARG_NEW_OWNER, CLType::Key)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_ACCEPT_OWNERSHIP,
            Vec::new(),
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );
    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_INIT,
            vec![
                Parameter::new(ARG_OWNER, CLType::Key),
                Parameter::new(ARG_INITIAL_SUPPLY, CLType::U256),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points
}
