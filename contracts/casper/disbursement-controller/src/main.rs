//! DisbursementController — Casper port of
//! `contracts/tron/tronbox/contracts/DisbursementController.sol`.
//!
//! Behavioral parity target: conditional, milestone-based fund release
//! against a verified recipient (checked via cross-contract call into
//! `identity-registry`'s `is_verified`), gated by an attester role,
//! with the same two-step admin transfer as every other contract in
//! this suite (see `docs/casper-contracts-srs.md` FR-3).
//!
//! Same boilerplate/design decisions as risk-registry/src/main.rs and
//! identity-registry/src/main.rs -- not re-derived here. Two things
//! this file adds beyond either of those:
//!
//! 1. **Cross-contract calls**, both to `identity-registry` (checking
//!    `is_verified` in `create_commitment`) and to whatever CEP-18
//!    token a commitment names (`transfer` in `release_milestone`).
//!    Uses `runtime::call_contract::<T>(ContractHash, entry_point,
//!    RuntimeArgs) -> T` -- confirmed against Casper's own
//!    cross-contract-communication docs and reference tutorials, not
//!    guessed.
//! 2. **No SafeERC20-style tolerant-decode wrapper** the way
//!    `DisbursementController.sol`'s `_safeTransfer` (M-3 fix) needs.
//!    That fix exists because some real ERC-20/TRC-20 tokens don't
//!    return a `bool` from `transfer` at all, which reverts a strict
//!    ABI-decode even though the transfer succeeded. CEP-18 doesn't
//!    have that ambiguity: the standard's `transfer` entry point
//!    returns nothing and simply reverts on failure (see
//!    `ceps/text/0018-token-standard.md`) -- calling it and letting a
//!    failure propagate as this call's own revert is already the
//!    correct, complete handling, not a shortcut.
//!
//! NOT yet independently confirmed compiling to wasm32-unknown-unknown
//! in CI. Written the same way identity-registry was: as carefully as
//! possible against confirmed-real APIs, but this is a first real
//! compiler pass, not a confirmed-working file yet. See
//! contracts/casper/README.md for current, itemized status.

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
    runtime_args, CLType, CLValue, ContractHash, EntryPointAccess, EntryPointType, EntryPoints,
    Key, Parameter, URef, U256,
};

mod constants;
mod error;
mod events;
mod model;

use constants::*;
use error::DisbursementControllerError;
use events::{
    AdminTransferProposed, AdminTransferred, AttesterUpdated, CommitmentCancelled,
    CommitmentCreated, MilestoneAttested, MilestoneReleased,
};
use model::{Commitment, CommitmentView, Milestone};

// ---------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn set_attester() {
    only_admin();
    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    let is_attester: bool = runtime::get_named_arg(ARG_IS_ATTESTER);

    storage::dictionary_put(attesters_dict(), &key_to_dict_key(&account), is_attester);
    casper_event_standard::emit(AttesterUpdated {
        account,
        is_attester,
    });
}

#[no_mangle]
pub extern "C" fn propose_new_admin() {
    only_admin();
    let new_admin: Key = runtime::get_named_arg(ARG_NEW_ADMIN);
    let current_admin = get_admin();
    set_pending_admin(Some(new_admin));
    casper_event_standard::emit(AdminTransferProposed {
        current_admin,
        proposed_admin: new_admin,
    });
}

#[no_mangle]
pub extern "C" fn accept_admin() {
    let caller = Key::from(runtime::get_caller());
    match get_pending_admin() {
        Some(pending_admin) if pending_admin == caller => {
            let previous_admin = get_admin();
            set_admin(pending_admin);
            set_pending_admin(None);
            casper_event_standard::emit(AdminTransferred {
                previous_admin,
                new_admin: pending_admin,
            });
        }
        _ => runtime::revert(DisbursementControllerError::CallerIsNotPendingAdmin),
    }
}

/// Admin-only. Checks `recipient` against `identity-registry` via
/// cross-contract call before recording the schedule. Same "doesn't
/// move any tokens, just records the schedule" behavior as
/// `DisbursementController.sol::createCommitment`.
#[no_mangle]
pub extern "C" fn create_commitment() {
    only_admin();

    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let token: Key = runtime::get_named_arg(ARG_TOKEN);
    let community: String = runtime::get_named_arg(ARG_COMMUNITY);
    let descriptions: Vec<String> = runtime::get_named_arg(ARG_DESCRIPTIONS);
    let amounts: Vec<U256> = runtime::get_named_arg(ARG_AMOUNTS);

    let is_verified: bool = runtime::call_contract(
        get_registry_hash(),
        IDENTITY_REGISTRY_ENTRY_POINT_IS_VERIFIED,
        runtime_args! { ARG_RECIPIENT => recipient },
    );
    if !is_verified {
        runtime::revert(DisbursementControllerError::RecipientNotVerified);
    }

    if descriptions.is_empty() {
        runtime::revert(DisbursementControllerError::NoMilestones);
    }
    if descriptions.len() != amounts.len() {
        runtime::revert(DisbursementControllerError::LengthMismatch);
    }

    let mut milestones = Vec::with_capacity(descriptions.len());
    let mut total_amount = U256::zero();
    for (description, amount) in descriptions.into_iter().zip(amounts.into_iter()) {
        if amount.is_zero() {
            runtime::revert(DisbursementControllerError::MilestoneAmountIsZero);
        }
        milestones.push(Milestone {
            description,
            amount,
            attested: false,
            released: false,
            attested_by: None,
            attested_at: 0,
            released_at: 0,
        });
        total_amount += amount;
    }
    let milestone_count = milestones.len() as u64;

    let commitment_id = get_commitment_count();
    let commitment = Commitment {
        recipient,
        token,
        community: community.clone(),
        active: true,
        cancelled: false,
        created_at: runtime::get_blocktime().into(),
        total_amount,
        released_amount: U256::zero(),
        milestones,
    };
    storage::dictionary_put(
        commitments_dict(),
        &commitment_id.to_string(),
        commitment,
    );
    set_commitment_count(commitment_id + 1);

    casper_event_standard::emit(CommitmentCreated {
        commitment_id,
        recipient,
        token,
        community,
        total_amount,
        milestone_count,
    });
}

/// Attester-role-gated. Same semantics as
/// `DisbursementController.sol::attestMilestone`.
#[no_mangle]
pub extern "C" fn attest_milestone() {
    only_attester();

    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);
    let milestone_index: u64 = runtime::get_named_arg(ARG_MILESTONE_INDEX);

    let mut commitment = get_active_commitment(commitment_id);
    let milestone = get_milestone_mut(&mut commitment, milestone_index);
    if milestone.attested {
        runtime::revert(DisbursementControllerError::MilestoneAlreadyAttested);
    }

    let caller = Key::from(runtime::get_caller());
    milestone.attested = true;
    milestone.attested_by = Some(caller);
    milestone.attested_at = runtime::get_blocktime().into();

    storage::dictionary_put(commitments_dict(), &commitment_id.to_string(), commitment);

    casper_event_standard::emit(MilestoneAttested {
        commitment_id,
        milestone_index,
        attested_by: caller,
    });
}

/// Callable by anyone once attested -- same reasoning as
/// `DisbursementController.sol::releaseMilestone`'s doc comment: the
/// attestation is the real gate, not who submits the release call.
/// Reentrancy-guarded around the one external call this contract makes
/// (the CEP-18 `transfer`), same purpose as
/// `DisbursementController.sol`'s `nonReentrant`.
#[no_mangle]
pub extern "C" fn release_milestone() {
    if get_reentrancy_guard() {
        runtime::revert(DisbursementControllerError::ReentrantCall);
    }
    set_reentrancy_guard(true);

    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);
    let milestone_index: u64 = runtime::get_named_arg(ARG_MILESTONE_INDEX);

    let mut commitment = get_active_commitment(commitment_id);
    let token = commitment.token;
    let recipient = commitment.recipient;

    let milestone = get_milestone_mut(&mut commitment, milestone_index);
    if !milestone.attested {
        runtime::revert(DisbursementControllerError::MilestoneNotAttested);
    }
    if milestone.released {
        runtime::revert(DisbursementControllerError::MilestoneAlreadyReleased);
    }
    let amount = milestone.amount;

    let token_hash = key_to_contract_hash(token);
    let this_contract_key = Key::from(get_this_contract_hash());
    let contract_balance: U256 = runtime::call_contract(
        token_hash,
        CEP18_ENTRY_POINT_BALANCE_OF,
        runtime_args! { CEP18_ARG_ACCOUNT => this_contract_key },
    );
    if contract_balance < amount {
        runtime::revert(DisbursementControllerError::InsufficientBalance);
    }

    // Effects before interaction.
    milestone.released = true;
    milestone.released_at = runtime::get_blocktime().into();
    commitment.released_amount += amount;
    storage::dictionary_put(commitments_dict(), &commitment_id.to_string(), commitment);

    casper_event_standard::emit(MilestoneReleased {
        commitment_id,
        milestone_index,
        recipient,
        amount,
    });

    // No tolerant-decode wrapper needed here -- see this file's module
    // comment for why CEP-18's transfer doesn't have the "some tokens
    // don't return a bool" problem SafeERC20/M-3 exists for on the TRON
    // side. A failed transfer reverts this whole call, same net effect.
    let _: () = runtime::call_contract(
        token_hash,
        CEP18_ENTRY_POINT_TRANSFER,
        runtime_args! {
            CEP18_ARG_RECIPIENT => recipient,
            CEP18_ARG_AMOUNT => amount,
        },
    );

    set_reentrancy_guard(false);
}

/// Admin-only. Same "doesn't sweep funds" behavior as
/// `DisbursementController.sol::cancelCommitment`.
#[no_mangle]
pub extern "C" fn cancel_commitment() {
    only_admin();
    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);

    let mut commitment = get_active_commitment(commitment_id);
    commitment.active = false;
    commitment.cancelled = true;
    let unreleased_amount = commitment.total_amount - commitment.released_amount;

    storage::dictionary_put(commitments_dict(), &commitment_id.to_string(), commitment);

    let caller = Key::from(runtime::get_caller());
    casper_event_standard::emit(CommitmentCancelled {
        commitment_id,
        cancelled_by: caller,
        unreleased_amount,
    });
}

#[no_mangle]
pub extern "C" fn commitment_count() {
    runtime::ret(CLValue::from_t(get_commitment_count()).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn get_commitment() {
    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);
    let commitment = require_commitment(commitment_id);
    let view: CommitmentView = (&commitment).into();
    runtime::ret(CLValue::from_t(view).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn get_milestone() {
    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);
    let milestone_index: u64 = runtime::get_named_arg(ARG_MILESTONE_INDEX);
    let mut commitment = require_commitment(commitment_id);
    let milestone = get_milestone_mut(&mut commitment, milestone_index).clone();
    runtime::ret(CLValue::from_t(milestone).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn is_attester() {
    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    let result: bool = storage::dictionary_get(attesters_dict(), &key_to_dict_key(&account))
        .unwrap_or_revert_with(DisbursementControllerError::DictionaryReadFailed)
        .unwrap_or(false);
    runtime::ret(CLValue::from_t(result).unwrap_or_revert());
}

// ---------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------

fn only_admin() {
    if Key::from(runtime::get_caller()) != get_admin() {
        runtime::revert(DisbursementControllerError::CallerIsNotAdmin);
    }
}

fn only_attester() {
    let caller = Key::from(runtime::get_caller());
    let is_attester: bool = storage::dictionary_get(attesters_dict(), &key_to_dict_key(&caller))
        .unwrap_or_revert_with(DisbursementControllerError::DictionaryReadFailed)
        .unwrap_or(false);
    if !is_attester {
        runtime::revert(DisbursementControllerError::CallerIsNotAttester);
    }
}

fn require_commitment(commitment_id: u64) -> Commitment {
    if commitment_id >= get_commitment_count() {
        runtime::revert(DisbursementControllerError::CommitmentDoesNotExist);
    }
    storage::dictionary_get(commitments_dict(), &commitment_id.to_string())
        .unwrap_or_revert_with(DisbursementControllerError::DictionaryReadFailed)
        .unwrap_or_revert_with(DisbursementControllerError::CommitmentDoesNotExist)
}

fn get_active_commitment(commitment_id: u64) -> Commitment {
    let commitment = require_commitment(commitment_id);
    if !commitment.active {
        runtime::revert(DisbursementControllerError::CommitmentNotActive);
    }
    commitment
}

fn get_milestone_mut(commitment: &mut Commitment, milestone_index: u64) -> &mut Milestone {
    commitment
        .milestones
        .get_mut(milestone_index as usize)
        .unwrap_or_revert_with(DisbursementControllerError::MilestoneDoesNotExist)
}

fn key_to_contract_hash(key: Key) -> ContractHash {
    key.into_hash()
        .map(ContractHash::new)
        .unwrap_or_revert_with(DisbursementControllerError::UnexpectedKeyType)
}

fn get_this_contract_hash() -> ContractHash {
    let key = runtime::get_key(CONTRACT_HASH_KEY_NAME)
        .unwrap_or_revert_with(DisbursementControllerError::MissingKey);
    key_to_contract_hash(key)
}

fn get_admin() -> Key {
    read_uref_value(KEY_ADMIN)
}

fn set_admin(new_admin: Key) {
    write_uref_value(KEY_ADMIN, new_admin);
}

fn get_pending_admin() -> Option<Key> {
    let uref = get_uref(KEY_PENDING_ADMIN);
    storage::read(uref)
        .unwrap_or_revert_with(DisbursementControllerError::MissingKey)
        .unwrap_or_revert_with(DisbursementControllerError::MissingKey)
}

fn set_pending_admin(pending: Option<Key>) {
    write_uref_value(KEY_PENDING_ADMIN, pending);
}

fn get_registry_hash() -> ContractHash {
    let key: Key = read_uref_value(KEY_REGISTRY_HASH);
    key_to_contract_hash(key)
}

fn get_commitment_count() -> u64 {
    read_uref_value(KEY_COMMITMENT_COUNT)
}

fn set_commitment_count(count: u64) {
    write_uref_value(KEY_COMMITMENT_COUNT, count);
}

fn get_reentrancy_guard() -> bool {
    read_uref_value(KEY_REENTRANCY_GUARD)
}

fn set_reentrancy_guard(value: bool) {
    write_uref_value(KEY_REENTRANCY_GUARD, value);
}

fn get_uref(name: &str) -> URef {
    runtime::get_key(name)
        .unwrap_or_revert_with(DisbursementControllerError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(DisbursementControllerError::UnexpectedKeyType)
}

fn read_uref_value<T: casper_types::CLTyped + casper_types::bytesrepr::FromBytes>(
    name: &str,
) -> T {
    let uref = get_uref(name);
    storage::read(uref)
        .unwrap_or_revert_with(DisbursementControllerError::MissingKey)
        .unwrap_or_revert_with(DisbursementControllerError::MissingKey)
}

fn write_uref_value<T: casper_types::CLTyped + casper_types::bytesrepr::ToBytes>(
    name: &str,
    value: T,
) {
    let uref = get_uref(name);
    storage::write(uref, value);
}

fn attesters_dict() -> URef {
    *runtime::get_key(KEY_ATTESTERS_DICT)
        .unwrap_or_revert_with(DisbursementControllerError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(DisbursementControllerError::UnexpectedKeyType)
}

fn commitments_dict() -> URef {
    *runtime::get_key(KEY_COMMITMENTS_DICT)
        .unwrap_or_revert_with(DisbursementControllerError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(DisbursementControllerError::UnexpectedKeyType)
}

fn key_to_dict_key(key: &Key) -> String {
    key.to_string()
}

// ---------------------------------------------------------------
// Installer (`call`)
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn call() {
    let registry_hash_arg: Key = runtime::get_named_arg(ARG_REGISTRY_HASH);

    let mut named_keys = NamedKeys::new();

    let admin_key = Key::from(runtime::get_caller());
    named_keys.insert(KEY_ADMIN.to_string(), storage::new_uref(admin_key).into());
    named_keys.insert(
        KEY_PENDING_ADMIN.to_string(),
        storage::new_uref(Option::<Key>::None).into(),
    );
    named_keys.insert(
        KEY_REGISTRY_HASH.to_string(),
        storage::new_uref(registry_hash_arg).into(),
    );
    named_keys.insert(
        KEY_COMMITMENT_COUNT.to_string(),
        storage::new_uref(0u64).into(),
    );
    named_keys.insert(
        KEY_REENTRANCY_GUARD.to_string(),
        storage::new_uref(false).into(),
    );

    let attesters_dict_uref = storage::new_dictionary(KEY_ATTESTERS_DICT).unwrap_or_revert();
    named_keys.insert(KEY_ATTESTERS_DICT.to_string(), attesters_dict_uref.into());

    let commitments_dict_uref = storage::new_dictionary(KEY_COMMITMENTS_DICT).unwrap_or_revert();
    named_keys.insert(
        KEY_COMMITMENTS_DICT.to_string(),
        commitments_dict_uref.into(),
    );

    // Admin is always implicitly an attester at install, matching
    // DisbursementController.sol's constructor
    // (_grantRole(ATTESTER_ROLE, admin_)) exactly.
    storage::dictionary_put(attesters_dict_uref, &key_to_dict_key(&admin_key), true);

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
        runtime_args! { ARG_ACCOUNT => admin_key },
    );

    runtime::put_key(CONTRACT_HASH_KEY_NAME, contract_hash.into());
}

#[no_mangle]
pub extern "C" fn init() {
    let schemas = Schemas::new()
        .with::<AdminTransferred>()
        .with::<AdminTransferProposed>()
        .with::<AttesterUpdated>()
        .with::<CommitmentCreated>()
        .with::<MilestoneAttested>()
        .with::<MilestoneReleased>()
        .with::<CommitmentCancelled>();
    casper_event_standard::init(schemas);

    let admin_key: Key = runtime::get_named_arg(ARG_ACCOUNT);
    casper_event_standard::emit(AttesterUpdated {
        account: admin_key,
        is_attester: true,
    });
}

fn build_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_SET_ATTESTER,
            vec![
                Parameter::new(ARG_ACCOUNT, CLType::Key),
                Parameter::new(ARG_IS_ATTESTER, CLType::Bool),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_PROPOSE_NEW_ADMIN,
            vec![Parameter::new(ARG_NEW_ADMIN, CLType::Key)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_ACCEPT_ADMIN,
            Vec::new(),
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_CREATE_COMMITMENT,
            vec![
                Parameter::new(ARG_RECIPIENT, CLType::Key),
                Parameter::new(ARG_TOKEN, CLType::Key),
                Parameter::new(ARG_COMMUNITY, CLType::String),
                Parameter::new(
                    ARG_DESCRIPTIONS,
                    CLType::List(alloc::boxed::Box::new(CLType::String)),
                ),
                Parameter::new(
                    ARG_AMOUNTS,
                    CLType::List(alloc::boxed::Box::new(CLType::U256)),
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
            ENTRY_POINT_ATTEST_MILESTONE,
            vec![
                Parameter::new(ARG_COMMITMENT_ID, CLType::U64),
                Parameter::new(ARG_MILESTONE_INDEX, CLType::U64),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_RELEASE_MILESTONE,
            vec![
                Parameter::new(ARG_COMMITMENT_ID, CLType::U64),
                Parameter::new(ARG_MILESTONE_INDEX, CLType::U64),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_CANCEL_COMMITMENT,
            vec![Parameter::new(ARG_COMMITMENT_ID, CLType::U64)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_COMMITMENT_COUNT,
            Vec::new(),
            CLType::U64,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_GET_COMMITMENT,
            vec![Parameter::new(ARG_COMMITMENT_ID, CLType::U64)],
            CLType::Any, // CommitmentView -- see model.rs's CLTyped impl.
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_GET_MILESTONE,
            vec![
                Parameter::new(ARG_COMMITMENT_ID, CLType::U64),
                Parameter::new(ARG_MILESTONE_INDEX, CLType::U64),
            ],
            CLType::Any, // Milestone -- see model.rs's CLTyped impl.
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_IS_ATTESTER,
            vec![Parameter::new(ARG_ACCOUNT, CLType::Key)],
            CLType::Bool,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_INIT,
            vec![Parameter::new(ARG_ACCOUNT, CLType::Key)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points
}
