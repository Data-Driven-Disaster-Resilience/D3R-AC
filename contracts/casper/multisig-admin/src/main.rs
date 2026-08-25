//! MultiSigAdmin — Casper port of
//! `contracts/tron/tronbox/contracts/MultiSigAdmin.sol`.
//!
//! Behavioral parity target: a fixed N-of-M owner set proposes
//! transactions (`submit_transaction`, auto-confirming from the
//! submitter), confirms/revokes confirmations, and executes once a
//! transaction has >= `threshold` confirmations -- matching the TRON
//! contract's own `submitTransaction`/`confirmTransaction`/
//! `revokeConfirmation`/`executeTransaction` flow exactly (see
//! `docs/casper-contracts-srs.md` FR-4). Owners are fixed at
//! deployment here too; rotate by deploying a new `multisig-admin` and
//! re-pointing the other contracts' admin role to it, same as the
//! TRON contract's own doc comment describes.
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
//! One real behavioral difference from the TRON contract, not just a
//! translation detail -- see `execute_transaction`'s own doc comment
//! for the full explanation of why there's no `value`/native-token
//! argument and no `bytes data` calldata blob the way
//! `MultiSigAdmin.sol` has them.
//!
//! NOT yet independently confirmed compiling to wasm32-unknown-unknown
//! in CI (unlike risk-registry, which has a green CI run behind it) --
//! written by carefully following that confirmed-working file's exact
//! patterns, but this is genuinely the first real compiler pass this
//! specific code will get, and `execute_transaction`'s
//! `call_versioned_contract` usage in particular is the least-verified
//! part of it (no test in this suite has exercised a real
//! cross-contract call yet). See contracts/casper/README.md for
//! current, itemized status; don't infer a green build from this
//! comment alone.

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
    bytesrepr::FromBytes,
    contracts::{ContractPackageHash, EntryPoint, NamedKeys},
    runtime_args, CLType, CLValue, EntryPointAccess, EntryPointType, EntryPoints, Key, Parameter,
    RuntimeArgs, URef,
};

mod constants;
mod error;
mod events;
mod model;

use constants::*;
use error::MultisigAdminError;
use events::{
    ConfirmationRevoked, OwnerAdded, TransactionConfirmed, TransactionExecuted,
    TransactionSubmitted,
};
use model::Transaction;

// ---------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------

/// Owner-only. Proposes a call to `target_package_hash`'s
/// `target_entry_point`, with `target_args_bytes` as the arguments.
/// Auto-confirms from the submitter, matching
/// `MultiSigAdmin.sol::submitTransaction`'s `_confirm(txId)` call at
/// the end.
#[no_mangle]
pub extern "C" fn submit_transaction() {
    only_owner();

    let target_package_hash: Key = runtime::get_named_arg(ARG_TARGET_PACKAGE_HASH);
    let target_entry_point: String = runtime::get_named_arg(ARG_TARGET_ENTRY_POINT);
    let target_args_bytes: Vec<u8> = runtime::get_named_arg(ARG_TARGET_ARGS_BYTES);

    // Casper has no zero-address concept the way EVM/TVM does (see
    // identity-registry's identical reasoning on `NewAdminInvalid`),
    // so this instead confirms `target_package_hash` actually parses
    // as a contract package hash before it's stored -- catches a
    // malformed target at submission time rather than only at
    // execution time.
    if key_to_package_hash(&target_package_hash).is_none() {
        runtime::revert(MultisigAdminError::InvalidTarget);
    }

    let tx_id = get_tx_count();
    let submitter = Key::from(runtime::get_caller());

    let tx = Transaction {
        target_package_hash,
        target_entry_point,
        target_args_bytes,
        executed: false,
        confirmation_count: 0,
    };
    storage::dictionary_put(transactions_dict(), &tx_id.to_string(), tx);
    set_tx_count(tx_id + 1);

    casper_event_standard::emit(TransactionSubmitted {
        tx_id,
        submitter,
        target_package_hash,
    });

    confirm(tx_id, submitter);

    runtime::ret(CLValue::from_t(tx_id).unwrap_or_revert());
}

/// Owner-only. Same guards/semantics as
/// `MultiSigAdmin.sol::confirmTransaction`.
#[no_mangle]
pub extern "C" fn confirm_transaction() {
    only_owner();

    let tx_id: u64 = runtime::get_named_arg(ARG_TX_ID);
    let caller = Key::from(runtime::get_caller());

    let tx = require_tx_exists_and_not_executed(tx_id);
    let _ = tx;

    if is_confirmed_internal(tx_id, caller) {
        runtime::revert(MultisigAdminError::AlreadyConfirmed);
    }

    confirm(tx_id, caller);
}

/// Owner-only. Same guards/semantics as
/// `MultiSigAdmin.sol::revokeConfirmation`.
#[no_mangle]
pub extern "C" fn revoke_confirmation() {
    only_owner();

    let tx_id: u64 = runtime::get_named_arg(ARG_TX_ID);
    let caller = Key::from(runtime::get_caller());

    let mut tx = require_tx_exists_and_not_executed(tx_id);

    if !is_confirmed_internal(tx_id, caller) {
        runtime::revert(MultisigAdminError::NotConfirmed);
    }

    set_confirmed_internal(tx_id, caller, false);
    tx.confirmation_count -= 1;
    storage::dictionary_put(transactions_dict(), &tx_id.to_string(), tx);

    casper_event_standard::emit(ConfirmationRevoked {
        tx_id,
        owner: caller,
    });
}

/// Owner-only. Executes a transaction once it has >= `threshold`
/// confirmations, matching `MultiSigAdmin.sol::executeTransaction`.
///
/// Deliberately no `nonReentrant` guard here the way the TRON
/// contract has one: Casper's execution model doesn't have the
/// EVM-style single-call-stack reentrancy surface that guard defends
/// against (the callee can't call back into an *unfinished* execution
/// of this entry point the way an EVM `call` can), so it has no
/// Casper equivalent to add. This has NOT been independently verified
/// against a live cross-contract-reentrancy test in this suite --
/// flagged here rather than silently assumed.
///
/// The bigger difference: `MultiSigAdmin.sol` takes `to`/`value`/
/// `bytes data` and does a raw, dynamically-typed EVM `call`. Casper
/// contract calls are typed and entry-point-addressed --
/// `runtime::call_versioned_contract` needs a `ContractPackageHash`,
/// an entry-point *name*, and a `RuntimeArgs` map, not an opaque byte
/// string a callee interprets itself. `target_args_bytes` is the
/// bridge: submitted as bytesrepr-serialized `RuntimeArgs` (the same
/// encoding a deploy's own arguments use), deserialized back into
/// `RuntimeArgs` right here at execution time. This preserves the
/// "propose an arbitrary call now, execute it later once confirmed"
/// shape FR-4 asks for, without inventing a generic calldata-execution
/// primitive Casper doesn't have.
///
/// Also unlike the TRON version: there's no `t.executed = false` /
/// `TransactionExecutionFailed` fallback path on a failed callee call.
/// A trapping callee aborts the *entire* deploy on Casper -- there's
/// no catchable "call failed, but my own state changes so far still
/// commit" the way EVM's `call` returning `success = false` allows.
/// In practice this is actually equivalent to what
/// `MultiSigAdmin.sol` does today: it reverts unconditionally on a
/// failed call too (`revert("MultiSigAdmin: underlying call
/// reverted")` right after emitting the now-rolled-back
/// `TransactionExecutionFailed`), so no working code path in the TRON
/// contract actually relies on the "failed but not reverted" state
/// that event's name implies. See `events.rs` for where this was
/// dropped.
#[no_mangle]
pub extern "C" fn execute_transaction() {
    only_owner();

    let tx_id: u64 = runtime::get_named_arg(ARG_TX_ID);
    let executor = Key::from(runtime::get_caller());

    let mut tx = require_tx_exists_and_not_executed(tx_id);

    if tx.confirmation_count < get_threshold() {
        runtime::revert(MultisigAdminError::InsufficientConfirmations);
    }

    let package_hash = key_to_package_hash(&tx.target_package_hash)
        .unwrap_or_revert_with(MultisigAdminError::InvalidTarget);

    let target_args = RuntimeArgs::from_bytes(&tx.target_args_bytes)
        .map(|(args, _rem)| args)
        .unwrap_or_revert_with(MultisigAdminError::MalformedTargetArgs);

    // Mark executed *before* the call -- if the call traps, the whole
    // deploy (including this write) is rolled back by the runtime, so
    // there's no window where `executed = true` is visible on-chain
    // for a call that didn't actually succeed. Same CEI ordering
    // `MultiSigAdmin.sol::executeTransaction` uses (`t.executed =
    // true;` before the external call), same reasoning: see that
    // contract's own comment and this suite's
    // `docs/audit-pass-2026-08-24-triage.md` for why that ordering is
    // correct despite a naive reentrancy-pattern scan flagging it.
    tx.executed = true;
    storage::dictionary_put(transactions_dict(), &tx_id.to_string(), tx);

    runtime::call_versioned_contract::<()>(
        package_hash,
        None,
        &tx.target_entry_point,
        target_args,
    );

    casper_event_standard::emit(TransactionExecuted { tx_id, executor });
}

#[no_mangle]
pub extern "C" fn is_owner() {
    let account: Key = runtime::get_named_arg(ARG_OWNER);
    let result = is_owner_internal(account);
    runtime::ret(CLValue::from_t(result).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn owner_count() {
    let owners: Vec<Key> = get_owners();
    runtime::ret(CLValue::from_t(owners.len() as u64).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn transaction_count() {
    runtime::ret(CLValue::from_t(get_tx_count()).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn is_confirmed() {
    let tx_id: u64 = runtime::get_named_arg(ARG_TX_ID);
    let owner: Key = runtime::get_named_arg(ARG_OWNER);
    let result = is_confirmed_internal(tx_id, owner);
    runtime::ret(CLValue::from_t(result).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn get_transaction() {
    let tx_id: u64 = runtime::get_named_arg(ARG_TX_ID);
    let tx: Transaction = storage::dictionary_get(transactions_dict(), &tx_id.to_string())
        .unwrap_or_revert_with(MultisigAdminError::DictionaryReadFailed)
        .unwrap_or_revert_with(MultisigAdminError::TransactionDoesNotExist);
    runtime::ret(CLValue::from_t(tx).unwrap_or_revert());
}

// ---------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------

fn only_owner() {
    let caller = Key::from(runtime::get_caller());
    if !is_owner_internal(caller) {
        runtime::revert(MultisigAdminError::CallerIsNotOwner);
    }
}

fn require_tx_exists_and_not_executed(tx_id: u64) -> Transaction {
    let tx: Transaction = storage::dictionary_get(transactions_dict(), &tx_id.to_string())
        .unwrap_or_revert_with(MultisigAdminError::DictionaryReadFailed)
        .unwrap_or_revert_with(MultisigAdminError::TransactionDoesNotExist);
    if tx.executed {
        runtime::revert(MultisigAdminError::TransactionAlreadyExecuted);
    }
    tx
}

fn confirm(tx_id: u64, owner: Key) {
    set_confirmed_internal(tx_id, owner, true);

    let mut tx: Transaction = storage::dictionary_get(transactions_dict(), &tx_id.to_string())
        .unwrap_or_revert_with(MultisigAdminError::DictionaryReadFailed)
        .unwrap_or_revert_with(MultisigAdminError::TransactionDoesNotExist);
    tx.confirmation_count += 1;
    storage::dictionary_put(transactions_dict(), &tx_id.to_string(), tx);

    casper_event_standard::emit(TransactionConfirmed { tx_id, owner });
}

fn is_owner_internal(account: Key) -> bool {
    storage::dictionary_get(owners_dict(), &key_to_dict_key(&account))
        .unwrap_or_revert_with(MultisigAdminError::DictionaryReadFailed)
        .unwrap_or(false)
}

fn is_confirmed_internal(tx_id: u64, owner: Key) -> bool {
    storage::dictionary_get(confirmations_dict(), &confirmation_dict_key(tx_id, &owner))
        .unwrap_or_revert_with(MultisigAdminError::DictionaryReadFailed)
        .unwrap_or(false)
}

fn set_confirmed_internal(tx_id: u64, owner: Key, value: bool) {
    storage::dictionary_put(
        confirmations_dict(),
        &confirmation_dict_key(tx_id, &owner),
        value,
    );
}

fn confirmation_dict_key(tx_id: u64, owner: &Key) -> String {
    // Composite key -- see risk-registry/src/main.rs's
    // `key_to_dict_key` for why `Key` is stringified for dictionary
    // lookups at all; `tx_id` prefixed the same way `_confirmations`'
    // outer `mapping(uint256 => ...)` key would be, just flattened
    // into one dictionary since Casper dictionaries are single-level.
    let mut s = tx_id.to_string();
    s.push(':');
    s.push_str(&owner.to_string());
    s
}

fn key_to_dict_key(key: &Key) -> String {
    key.to_string()
}

fn key_to_package_hash(key: &Key) -> Option<ContractPackageHash> {
    key.into_hash_addr().map(ContractPackageHash::new)
}

fn get_owners() -> Vec<Key> {
    let uref: URef = runtime::get_key(KEY_OWNERS)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(MultisigAdminError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
}

fn get_threshold() -> u64 {
    let uref: URef = runtime::get_key(KEY_THRESHOLD)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(MultisigAdminError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
}

fn get_tx_count() -> u64 {
    let uref: URef = runtime::get_key(KEY_TX_COUNT)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(MultisigAdminError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
}

fn set_tx_count(count: u64) {
    let uref: URef = runtime::get_key(KEY_TX_COUNT)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(MultisigAdminError::UnexpectedKeyType);
    storage::write(uref, count);
}

fn owners_dict() -> URef {
    *runtime::get_key(KEY_OWNERS_DICT)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(MultisigAdminError::UnexpectedKeyType)
}

fn transactions_dict() -> URef {
    *runtime::get_key(KEY_TRANSACTIONS_DICT)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(MultisigAdminError::UnexpectedKeyType)
}

fn confirmations_dict() -> URef {
    *runtime::get_key(KEY_CONFIRMATIONS_DICT)
        .unwrap_or_revert_with(MultisigAdminError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(MultisigAdminError::UnexpectedKeyType)
}

// ---------------------------------------------------------------
// Installer (`call`)
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn call() {
    let owners: Vec<Key> = runtime::get_named_arg(ARG_OWNERS);
    let threshold: u64 = runtime::get_named_arg(ARG_THRESHOLD);

    if owners.is_empty() {
        runtime::revert(MultisigAdminError::OwnersRequired);
    }
    if threshold == 0 || threshold > owners.len() as u64 {
        runtime::revert(MultisigAdminError::InvalidThreshold);
    }
    // Duplicate check -- O(n^2), same as `MultiSigAdmin.sol`'s own
    // `hasRole` check per iteration; owner sets are small (single
    // digits) in every real deployment this suite anticipates, so
    // this isn't the size that would need a dictionary-backed
    // check instead.
    for i in 0..owners.len() {
        for j in 0..i {
            if owners[i] == owners[j] {
                runtime::revert(MultisigAdminError::DuplicateOwner);
            }
        }
    }

    let mut named_keys = NamedKeys::new();

    named_keys.insert(
        KEY_OWNERS.to_string(),
        storage::new_uref(owners.clone()).into(),
    );
    named_keys.insert(
        KEY_THRESHOLD.to_string(),
        storage::new_uref(threshold).into(),
    );
    named_keys.insert(KEY_TX_COUNT.to_string(), storage::new_uref(0u64).into());

    let owners_dict_uref = storage::new_dictionary(KEY_OWNERS_DICT).unwrap_or_revert();
    named_keys.insert(KEY_OWNERS_DICT.to_string(), owners_dict_uref.into());

    let transactions_dict_uref = storage::new_dictionary(KEY_TRANSACTIONS_DICT).unwrap_or_revert();
    named_keys.insert(
        KEY_TRANSACTIONS_DICT.to_string(),
        transactions_dict_uref.into(),
    );

    let confirmations_dict_uref = storage::new_dictionary(KEY_CONFIRMATIONS_DICT).unwrap_or_revert();
    named_keys.insert(
        KEY_CONFIRMATIONS_DICT.to_string(),
        confirmations_dict_uref.into(),
    );

    for owner in owners.iter() {
        storage::dictionary_put(owners_dict_uref, &key_to_dict_key(owner), true);
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
            ARG_OWNERS => owners,
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
        .with::<OwnerAdded>()
        .with::<TransactionSubmitted>()
        .with::<TransactionConfirmed>()
        .with::<ConfirmationRevoked>()
        .with::<TransactionExecuted>();
    casper_event_standard::init(schemas);

    // Emit the install-time owner grants from inside the contract's
    // own context, same reasoning as identity-registry's initial-
    // verifier event.
    let owners: Vec<Key> = runtime::get_named_arg(ARG_OWNERS);
    for owner in owners {
        casper_event_standard::emit(OwnerAdded { owner });
    }
}

fn build_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_SUBMIT_TRANSACTION,
            vec![
                Parameter::new(ARG_TARGET_PACKAGE_HASH, CLType::Key),
                Parameter::new(ARG_TARGET_ENTRY_POINT, CLType::String),
                Parameter::new(
                    ARG_TARGET_ARGS_BYTES,
                    CLType::List(alloc::boxed::Box::new(CLType::U8)),
                ),
            ],
            CLType::U64,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_CONFIRM_TRANSACTION,
            vec![Parameter::new(ARG_TX_ID, CLType::U64)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_REVOKE_CONFIRMATION,
            vec![Parameter::new(ARG_TX_ID, CLType::U64)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_EXECUTE_TRANSACTION,
            vec![Parameter::new(ARG_TX_ID, CLType::U64)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_IS_OWNER,
            vec![Parameter::new(ARG_OWNER, CLType::Key)],
            CLType::Bool,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_OWNER_COUNT,
            Vec::new(),
            CLType::U64,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_TRANSACTION_COUNT,
            Vec::new(),
            CLType::U64,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_IS_CONFIRMED,
            vec![
                Parameter::new(ARG_TX_ID, CLType::U64),
                Parameter::new(ARG_OWNER, CLType::Key),
            ],
            CLType::Bool,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_GET_TRANSACTION,
            vec![Parameter::new(ARG_TX_ID, CLType::U64)],
            CLType::Any, // Transaction -- see model.rs's CLTyped impl.
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_INIT,
            vec![Parameter::new(
                ARG_OWNERS,
                CLType::List(alloc::boxed::Box::new(CLType::Key)),
            )],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points
}
