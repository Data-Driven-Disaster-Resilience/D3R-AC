//! D3RACToken — Casper CEP-18 port of
//! `contracts/tron/tronbox/contracts/D3RACToken.sol`.
//!
//! Behavioral parity target: `docs/casper-contracts-srs.md` FR-1 --
//! full CEP-18 standard surface (`ceps/text/0018-token-standard.md`),
//! plus D3RACToken.sol's own extensions (minter-role-gated `mint`,
//! public `burn`, two-step ownership transfer). This is the token
//! `disbursement-controller` already calls `transfer`/expects to exist
//! at a `Key` it's configured with.
//!
//! **One deliberate point of standard non-compliance, for `balances`
//! only**: the CEP-18 spec's storage-interface section mandates a
//! specific dictionary-key derivation for `balances` (base64-encoded
//! CLType bytes of the account `Key`) so external tooling can read
//! balances directly from raw contract storage without going through
//! an entry point. `balances`' named key and entry-point signatures
//! match the standard exactly, but its dictionary VALUES are keyed
//! using this suite's own established `key_to_dict_key`
//! (`Key::to_string()`) pattern instead -- full entry-point-level
//! composability is preserved (`balance_of`/`transfer` work exactly as
//! specified), but a generic CEP-18 block explorer expecting the
//! standard's exact storage layout wouldn't be able to read balances
//! by direct storage query. `balances` only ever stores one `Key`'s
//! worth of bytes per entry, which stays well under Casper's
//! dictionary-item-key length limit either way, so this one is a real
//! choice, not a workaround.
//!
//! `allowances`, by contrast, DOES use the standard's exact blake2b-hash
//! derivation (see `allowance_dict_key`) -- concatenating two Key
//! `Display` strings (this file's first draft) turned out to exceed
//! that length limit in practice, confirmed by a real
//! `ApiError::DictionaryItemKeyTooLarge` from this contract's own first
//! CI run. The standard's fixed-length hash output exists specifically
//! to solve that, not just for tooling compatibility -- so for
//! `allowances` there was no real choice to make once that surfaced.
//!
//! Same boilerplate/design decisions as the rest of this suite --
//! see risk-registry/src/main.rs's module comment.
//!
//! NOT yet independently confirmed compiling to wasm32-unknown-unknown
//! in CI. See contracts/casper/README.md for current, itemized status.

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
    account::AccountHash,
    bytesrepr::ToBytes,
    contracts::{ContractPackageHash, EntryPoint, NamedKeys},
    runtime_args, CLType, CLValue, EntryPointAccess, EntryPointType, EntryPoints, Key, Parameter,
    URef, U256,
};

mod constants;
mod error;
mod events;

use constants::*;
use error::D3racTokenError;
use events::{
    Burn, DecreaseAllowance, IncreaseAllowance, Mint, MinterUpdated, OwnershipTransferProposed,
    OwnershipTransferred, SetAllowance, Transfer, TransferFrom,
};

const TOKEN_NAME: &str = "D3R-AC Relief Token";
const TOKEN_SYMBOL: &str = "D3RAC";
const TOKEN_DECIMALS: u8 = 18;

// ---------------------------------------------------------------
// Entry points -- standard CEP-18 surface
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn name() {
    runtime::ret(CLValue::from_t(read_uref_value::<String>(KEY_NAME)).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn symbol() {
    runtime::ret(CLValue::from_t(read_uref_value::<String>(KEY_SYMBOL)).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn decimals() {
    runtime::ret(CLValue::from_t(read_uref_value::<u8>(KEY_DECIMALS)).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn total_supply() {
    runtime::ret(CLValue::from_t(read_uref_value::<U256>(KEY_TOTAL_SUPPLY)).unwrap_or_revert());
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

/// Per the standard: "The transfer should fail ... if the recipient is
/// the owner itself" (`CannotTargetSelfUser`).
#[no_mangle]
pub extern "C" fn transfer() {
    let caller = immediate_caller_key();
    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let amount: U256 = runtime::get_named_arg(ARG_AMOUNT);

    if recipient == caller {
        runtime::revert(D3racTokenError::CannotTargetSelfUser);
    }
    move_balance(caller, recipient, amount);

    casper_event_standard::emit(Transfer {
        sender: caller,
        recipient,
        amount,
    });
}

#[no_mangle]
pub extern "C" fn transfer_from() {
    let caller = immediate_caller_key();
    let owner: Key = runtime::get_named_arg(ARG_OWNER);
    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let amount: U256 = runtime::get_named_arg(ARG_AMOUNT);

    if recipient == owner {
        runtime::revert(D3racTokenError::CannotTargetSelfUser);
    }

    let current_allowance = get_allowance(owner, caller);
    if current_allowance < amount {
        runtime::revert(D3racTokenError::InsufficientAllowance);
    }
    set_allowance(owner, caller, current_allowance - amount);

    move_balance(owner, recipient, amount);

    casper_event_standard::emit(TransferFrom {
        spender: caller,
        owner,
        recipient,
        amount,
    });
}

#[no_mangle]
pub extern "C" fn approve() {
    let caller = immediate_caller_key();
    let spender: Key = runtime::get_named_arg(ARG_SPENDER);
    let amount: U256 = runtime::get_named_arg(ARG_AMOUNT);

    set_allowance(caller, spender, amount);
    casper_event_standard::emit(SetAllowance {
        owner: caller,
        spender,
        allowance: amount,
    });
}

#[no_mangle]
pub extern "C" fn increase_allowance() {
    let caller = immediate_caller_key();
    let spender: Key = runtime::get_named_arg(ARG_SPENDER);
    let inc_by: U256 = runtime::get_named_arg(ARG_INC_BY);

    let current = get_allowance(caller, spender);
    // "If the sum ... is greater than the maximum value of U256, the
    // allowance is set to the maximum value of U256" -- per spec,
    // saturate rather than revert/panic on overflow.
    let new_allowance = current.saturating_add(inc_by);
    set_allowance(caller, spender, new_allowance);

    casper_event_standard::emit(IncreaseAllowance {
        owner: caller,
        spender,
        allowance: new_allowance,
        inc_by,
    });
}

#[no_mangle]
pub extern "C" fn decrease_allowance() {
    let caller = immediate_caller_key();
    let spender: Key = runtime::get_named_arg(ARG_SPENDER);
    let decr_by: U256 = runtime::get_named_arg(ARG_DECR_BY);

    let current = get_allowance(caller, spender);
    // "If decr_by is greater than the current allowance, the allowance
    // is set to zero" -- per spec, saturate rather than revert.
    let new_allowance = current.saturating_sub(decr_by);
    set_allowance(caller, spender, new_allowance);

    casper_event_standard::emit(DecreaseAllowance {
        owner: caller,
        spender,
        allowance: new_allowance,
        decr_by,
    });
}

// ---------------------------------------------------------------
// Entry points -- extensions beyond the CEP-18 standard
// (D3RACToken.sol's own additions)
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn mint() {
    only_minter();
    let to: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let amount: U256 = runtime::get_named_arg(ARG_AMOUNT);

    let new_total = read_uref_value::<U256>(KEY_TOTAL_SUPPLY) + amount;
    write_uref_value(KEY_TOTAL_SUPPLY, new_total);
    let new_balance = get_balance(to) + amount;
    set_balance(to, new_balance);

    casper_event_standard::emit(Mint {
        recipient: to,
        amount,
    });
}

/// Public -- burns the caller's own tokens, same as
/// `D3RACToken.sol::burn(uint256 value)`. Uses `immediate_caller_key`
/// (not `get_caller`) for the same reason `transfer` does -- a
/// contract holding its own balance should be able to burn it.
#[no_mangle]
pub extern "C" fn burn() {
    let caller = immediate_caller_key();
    let amount: U256 = runtime::get_named_arg(ARG_AMOUNT);

    let balance = get_balance(caller);
    if balance < amount {
        runtime::revert(D3racTokenError::InsufficientBalance);
    }
    set_balance(caller, balance - amount);
    let new_total = read_uref_value::<U256>(KEY_TOTAL_SUPPLY) - amount;
    write_uref_value(KEY_TOTAL_SUPPLY, new_total);

    casper_event_standard::emit(Burn {
        owner: caller,
        amount,
    });
}

#[no_mangle]
pub extern "C" fn set_minter() {
    only_owner();
    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    let is_minter: bool = runtime::get_named_arg(ARG_IS_MINTER);

    storage::dictionary_put(minters_dict(), &key_to_dict_key(&account), is_minter);
    casper_event_standard::emit(MinterUpdated {
        account,
        is_minter,
    });
}

#[no_mangle]
pub extern "C" fn is_minter() {
    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    let result: bool = storage::dictionary_get(minters_dict(), &key_to_dict_key(&account))
        .unwrap_or_revert_with(D3racTokenError::DictionaryReadFailed)
        .unwrap_or(false);
    runtime::ret(CLValue::from_t(result).unwrap_or_revert());
}

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

#[no_mangle]
pub extern "C" fn accept_ownership() {
    let caller = immediate_caller_key();
    match get_pending_owner() {
        Some(pending_owner) if pending_owner == caller => {
            let previous_owner = get_owner();
            set_owner(pending_owner);
            set_pending_owner(None);
            casper_event_standard::emit(OwnershipTransferred {
                previous_owner,
                new_owner: pending_owner,
            });
        }
        _ => runtime::revert(D3racTokenError::CallerIsNotPendingOwner),
    }
}

// ---------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------

/// The real msg.sender-equivalent for CEP-18's "direct caller"
/// semantics -- NOT `runtime::get_caller()`, which returns the
/// `AccountHash` of whichever account originally signed the deploy,
/// all the way up the call stack (Casper's own docs describe it as
/// "the account that initiated the transaction" -- the tx.origin
/// equivalent, not msg.sender). When a contract like
/// `disbursement-controller` calls this token's `transfer` on its own
/// behalf, `get_caller()` still resolves to the original externally-
/// owned account, not the calling contract -- so a caller-gated debit
/// using `get_caller()` silently debits the wrong party's balance.
///
/// This was a real, CI-caught bug, not a hypothetical: the first
/// version of this file used `Key::from(runtime::get_caller())` here,
/// which compiled and passed every test that only ever called these
/// entry points directly from an account -- and only failed once
/// `disbursement-controller-tests` added a real end-to-end test
/// calling `transfer` from inside another contract
/// (`should_release_milestone_with_a_real_token_and_reject_when_unfunded`),
/// which reverted with `InsufficientBalance` despite the calling
/// contract genuinely holding the funds.
///
/// `runtime::get_immediate_caller()` is the correct primitive --
/// confirmed directly against `casper-contract` 5.1.1's and
/// `casper-types` 6.1.0's own source (downloaded and read, not
/// guessed): it returns a `CallerInfo`, whose `.kind()` /
/// `.get_field_by_index()` accessors are the only way to read it,
/// since the field-index constants (`ACCOUNT`/`PACKAGE`/
/// `CONTRACT_PACKAGE`/`ENTITY`/`CONTRACT` = 0/1/2/3/4) that `.kind()`
/// returns are private to `casper_types::system::caller` -- there is
/// no public `TryFrom<CallerInfo> for Caller` to convert back to the
/// ergonomic public enum. The raw values below (0, 4) are taken
/// directly from that source's `TryFrom<Caller> for CallerInfo` impl,
/// not inferred. Kind 3 (`Entity`) exists in that same source but
/// isn't handled below -- see `immediate_caller_key`'s own comment on
/// that match arm for why (a real, CI-caught `PackageHash` visibility
/// problem, not a decision to skip it).
fn immediate_caller_key() -> Key {
    let caller_info = runtime::get_immediate_caller().unwrap_or_revert();
    match caller_info.kind() {
        // ACCOUNT (0): Caller::Initiator -- an account called us
        // directly (no intervening contract).
        0 => {
            let account_hash: Option<AccountHash> = caller_info
                .get_field_by_index(0)
                .unwrap_or_revert()
                .clone()
                .into_t()
                .unwrap_or_revert();
            Key::from(account_hash.unwrap_or_revert())
        }
        // ENTITY (3): Caller::Entity. Not folded into a handled case --
        // this suite has never actually observed this variant (only
        // ACCOUNT and CONTRACT below), and the type needed to read its
        // field (PackageHash) turned out to not be part of
        // casper_types' public API in this version at any of the
        // import paths its own source suggested (casper_types::package
        // is private; casper_types::contracts re-imports it privately
        // too, per two real, separate CI-caught E0603 errors on this
        // exact line). Reverting here is honest about that rather than
        // guessing a third import path for a branch nothing in this
        // suite exercises -- revisit if a real call pattern ever
        // actually produces this kind.
        // CONTRACT (4): Caller::SmartContract -- field index 2 is the
        // ContractPackageHash. This is the variant this suite's own
        // contract-to-contract calls (disbursement-controller calling
        // this token) have actually been observed to produce.
        4 => {
            let contract_package_hash: Option<ContractPackageHash> = caller_info
                .get_field_by_index(2)
                .unwrap_or_revert()
                .clone()
                .into_t()
                .unwrap_or_revert();
            Key::from(contract_package_hash.unwrap_or_revert())
        }
        _ => runtime::revert(D3racTokenError::UnrecognizedCallerKind),
    }
}

fn only_owner() {
    if immediate_caller_key() != get_owner() {
        runtime::revert(D3racTokenError::CallerIsNotOwner);
    }
}

fn only_minter() {
    let caller = immediate_caller_key();
    let is_minter: bool = storage::dictionary_get(minters_dict(), &key_to_dict_key(&caller))
        .unwrap_or_revert_with(D3racTokenError::DictionaryReadFailed)
        .unwrap_or(false);
    if !is_minter {
        runtime::revert(D3racTokenError::CallerIsNotMinter);
    }
}

fn get_owner() -> Key {
    read_uref_value(KEY_OWNER)
}

fn set_owner(new_owner: Key) {
    write_uref_value(KEY_OWNER, new_owner);
}

fn get_pending_owner() -> Option<Key> {
    let uref = get_uref(KEY_PENDING_OWNER);
    storage::read(uref)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
}

fn set_pending_owner(pending: Option<Key>) {
    write_uref_value(KEY_PENDING_OWNER, pending);
}

fn get_balance(account: Key) -> U256 {
    storage::dictionary_get(balances_dict(), &key_to_dict_key(&account))
        .unwrap_or_revert_with(D3racTokenError::DictionaryReadFailed)
        .unwrap_or(U256::zero())
}

fn set_balance(account: Key, value: U256) {
    storage::dictionary_put(balances_dict(), &key_to_dict_key(&account), value);
}

fn move_balance(from: Key, to: Key, amount: U256) {
    let from_balance = get_balance(from);
    if from_balance < amount {
        runtime::revert(D3racTokenError::InsufficientBalance);
    }
    set_balance(from, from_balance - amount);
    let to_balance = get_balance(to);
    set_balance(to, to_balance + amount);
}

fn get_allowance(owner: Key, spender: Key) -> U256 {
    let key = allowance_dict_key(owner, spender);
    storage::dictionary_get(allowances_dict(), &key)
        .unwrap_or_revert_with(D3racTokenError::DictionaryReadFailed)
        .unwrap_or(U256::zero())
}

fn set_allowance(owner: Key, spender: Key, amount: U256) {
    let key = allowance_dict_key(owner, spender);
    storage::dictionary_put(allowances_dict(), &key, amount);
}

/// Matches the CEP-18 standard's own allowance-key derivation exactly
/// (ceps/text/0018-token-standard.md's example: blake2b hash of the
/// concatenated owner+spender bytes, hex-encoded) -- this turned out
/// to not be optional. The two Key Display strings this file's
/// original `Key::to_string()`-based approach used are long enough
/// that concatenating two of them exceeds Casper's dictionary item key
/// length limit (confirmed by a real
/// ApiError::DictionaryItemKeyTooLarge from CI on the first real test
/// run against this contract) -- the standard's fixed-length hash
/// output exists specifically to avoid that, not just for external
/// tooling compatibility as this file's module comment originally
/// assumed for the *balances* dictionary (which only ever holds ONE
/// key's worth of bytes, well under the limit, so that one is
/// unaffected).
fn allowance_dict_key(owner: Key, spender: Key) -> String {
    let mut preimage = owner
        .to_bytes()
        .unwrap_or_revert_with(D3racTokenError::UnexpectedKeyType);
    preimage.extend(
        spender
            .to_bytes()
            .unwrap_or_revert_with(D3racTokenError::UnexpectedKeyType),
    );
    let hash_bytes = runtime::blake2b(preimage);
    hex_encode(&hash_bytes)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX_CHARS[(b >> 4) as usize] as char);
        s.push(HEX_CHARS[(b & 0x0f) as usize] as char);
    }
    s
}

fn key_to_dict_key(key: &Key) -> String {
    key.to_string()
}

fn get_uref(name: &str) -> URef {
    runtime::get_key(name)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(D3racTokenError::UnexpectedKeyType)
}

fn read_uref_value<T: casper_types::CLTyped + casper_types::bytesrepr::FromBytes>(
    name: &str,
) -> T {
    let uref = get_uref(name);
    storage::read(uref)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
        .unwrap_or_revert_with(D3racTokenError::MissingKey)
}

fn write_uref_value<T: casper_types::CLTyped + casper_types::bytesrepr::ToBytes>(
    name: &str,
    value: T,
) {
    let uref = get_uref(name);
    storage::write(uref, value);
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
    // Matches D3RACToken.sol's constructor(uint256 initialSupply,
    // address owner_) exactly, including "0 means fund entirely via
    // mint later" semantics.
    let initial_supply: U256 = runtime::get_named_arg(ARG_INITIAL_SUPPLY);
    let owner_arg: Key = runtime::get_named_arg(ARG_OWNER_ARG);

    let mut named_keys = NamedKeys::new();

    named_keys.insert(
        KEY_NAME.to_string(),
        storage::new_uref(TOKEN_NAME.to_string()).into(),
    );
    named_keys.insert(
        KEY_SYMBOL.to_string(),
        storage::new_uref(TOKEN_SYMBOL.to_string()).into(),
    );
    named_keys.insert(
        KEY_DECIMALS.to_string(),
        storage::new_uref(TOKEN_DECIMALS).into(),
    );

    let scaled_supply = if initial_supply.is_zero() {
        U256::zero()
    } else {
        // TOKEN_DECIMALS is 18 -- 10^18 as a literal U256, rather than
        // relying on an unconfirmed U256::pow(exponent) signature in
        // this dependency version. If TOKEN_DECIMALS above is ever
        // changed, this literal must be updated to match.
        const TEN_POW_18: u64 = 1_000_000_000_000_000_000u64;
        initial_supply * U256::from(TEN_POW_18)
    };
    named_keys.insert(
        KEY_TOTAL_SUPPLY.to_string(),
        storage::new_uref(scaled_supply).into(),
    );

    named_keys.insert(KEY_OWNER.to_string(), storage::new_uref(owner_arg).into());
    named_keys.insert(
        KEY_PENDING_OWNER.to_string(),
        storage::new_uref(Option::<Key>::None).into(),
    );

    let balances_dict_uref = storage::new_dictionary(KEY_BALANCES_DICT).unwrap_or_revert();
    named_keys.insert(KEY_BALANCES_DICT.to_string(), balances_dict_uref.into());
    if !scaled_supply.is_zero() {
        storage::dictionary_put(balances_dict_uref, &key_to_dict_key(&owner_arg), scaled_supply);
    }

    let allowances_dict_uref = storage::new_dictionary(KEY_ALLOWANCES_DICT).unwrap_or_revert();
    named_keys.insert(KEY_ALLOWANCES_DICT.to_string(), allowances_dict_uref.into());

    let minters_dict_uref = storage::new_dictionary(KEY_MINTERS_DICT).unwrap_or_revert();
    named_keys.insert(KEY_MINTERS_DICT.to_string(), minters_dict_uref.into());
    // Owner is always implicitly a minter at install, matching
    // D3RACToken.sol's constructor (_grantRole(MINTER_ROLE, owner_))
    // exactly.
    storage::dictionary_put(minters_dict_uref, &key_to_dict_key(&owner_arg), true);

    let entry_points = build_entry_points();

    let (contract_hash, _contract_version) = storage::new_locked_contract(
        entry_points,
        Some(named_keys),
        Some(PACKAGE_HASH_KEY_NAME.to_string()),
        Some(ACCESS_UREF_KEY_NAME.to_string()),
        None,
    );

    runtime::call_contract::<()>(
        contract_hash,
        ENTRY_POINT_INIT,
        runtime_args! {
            ARG_OWNER_ARG => owner_arg,
            ARG_INITIAL_SUPPLY => scaled_supply,
        },
    );

    runtime::put_key(CONTRACT_HASH_KEY_NAME, contract_hash.into());
}

#[no_mangle]
pub extern "C" fn init() {
    let schemas = Schemas::new()
        .with::<Mint>()
        .with::<Burn>()
        .with::<SetAllowance>()
        .with::<IncreaseAllowance>()
        .with::<DecreaseAllowance>()
        .with::<Transfer>()
        .with::<TransferFrom>()
        .with::<OwnershipTransferred>()
        .with::<OwnershipTransferProposed>()
        .with::<MinterUpdated>();
    casper_event_standard::init(schemas);

    let owner_arg: Key = runtime::get_named_arg(ARG_OWNER_ARG);
    casper_event_standard::emit(MinterUpdated {
        account: owner_arg,
        is_minter: true,
    });

    let scaled_supply: U256 = runtime::get_named_arg(ARG_INITIAL_SUPPLY);
    if !scaled_supply.is_zero() {
        casper_event_standard::emit(Mint {
            recipient: owner_arg,
            amount: scaled_supply,
        });
    }
}

fn build_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();

    let view = |name: &'static str, ret: CLType| {
        EntryPoint::new(name, Vec::new(), ret, EntryPointAccess::Public, EntryPointType::Called)
    };

    entry_points.add_entry_point(view(ENTRY_POINT_NAME, CLType::String).into());
    entry_points.add_entry_point(view(ENTRY_POINT_SYMBOL, CLType::String).into());
    entry_points.add_entry_point(view(ENTRY_POINT_DECIMALS, CLType::U8).into());
    entry_points.add_entry_point(view(ENTRY_POINT_TOTAL_SUPPLY, CLType::U256).into());

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
            ENTRY_POINT_DECREASE_ALLOWANCE,
            vec![
                Parameter::new(ARG_SPENDER, CLType::Key),
                Parameter::new(ARG_DECR_BY, CLType::U256),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_INCREASE_ALLOWANCE,
            vec![
                Parameter::new(ARG_SPENDER, CLType::Key),
                Parameter::new(ARG_INC_BY, CLType::U256),
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
                Parameter::new(ARG_IS_MINTER, CLType::Bool),
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
                Parameter::new(ARG_OWNER_ARG, CLType::Key),
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
