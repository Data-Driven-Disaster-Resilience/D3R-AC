//! DisbursementController — Casper port of
//! `contracts/tron/tronbox/contracts/DisbursementController.sol`.
//!
//! Behavioral parity target: milestone-based commitments for a
//! verified recipient (checked live against `identity-registry` via
//! cross-contract call), attester-gated milestone attestation, and
//! permissionless release once attested -- matching
//! `docs/casper-contracts-srs.md` FR-3, including the TRON contract's
//! explicit design choice that attestation is the real gate, not who
//! submits the release transaction.
//!
//! Two real simplifications versus the TRON contract, not just
//! translation details:
//!
//! 1. No `ITRC20` local-interface / `_safeTransfer` workaround.
//!    `DisbursementController.sol` needs `_safeTransfer` because some
//!    real ERC-20/TRC-20 tokens (Ethereum USDT among them) don't
//!    return the `bool` the standard nominally requires, which
//!    `require(token.transfer(...))` chokes decoding even on a
//!    successful transfer. This suite's own `d3rac-token` (and CEP-18
//!    generally) doesn't have that ambiguity -- see `d3rac-token/src/
//!    main.rs`'s module comment: CEP-18-standard entry points revert
//!    on failure and return `Unit`, full stop. So `transfer` here is
//!    a direct `call_versioned_contract::<()>`, no return-value
//!    tolerance logic needed.
//! 2. This contract's own "balance" is tracked by *its own package
//!    hash*, used as the `Key` identity passed to the token's
//!    `balance_of`/as the implicit sender of `transfer` -- the Casper
//!    analog of Solidity's `address(this)`. **This is the
//!    least-verified assumption in this file**: it depends on the
//!    callee (`d3rac-token`)'s `runtime::get_caller()` returning
//!    *this* contract's own identity during the nested call, not the
//!    original externally-owned account that initiated the top-level
//!    deploy. Every other cross-contract-call question this suite has
//!    hit so far (`multisig-admin`'s `call_versioned_contract` usage)
//!    was about getting the call to compile and execute at all, not
//!    about caller-identity semantics inside the callee -- this is a
//!    new, untested question. If it resolves the other way, the token
//!    debit in `release_milestone` would target the wrong account.
//!    Needs a real local-network integration test against a deployed
//!    `d3rac-token`, not just a compile pass, before this is trusted
//!    for a real disbursement.
//!
//! Same boilerplate, same design decisions, same hard-won lessons as
//! `risk-registry/src/main.rs` -- not re-derived here. Written against
//! the casper-types 6.1.0 API surface `multisig-admin`'s PR #20
//! needed two real CI-caught fix rounds to discover (`contracts::`
//! module paths, `Key::into_hash_addr` not `into_hash`), applied from
//! the start.
//!
//! NOT yet confirmed compiling -- unlike `d3rac-token` (which compiled
//! clean on its first CI pass after those fixes were already known),
//! this file has meaningfully more surface area (nested struct
//! storage, two distinct cross-contract call shapes) that hasn't been
//! through CI at all yet. See `contracts/casper/README.md` for
//! current, itemized status.

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
    contracts::{ContractPackageHash, EntryPoint, NamedKeys},
    runtime_args, CLType, CLValue, EntryPointAccess, EntryPointType, EntryPoints, Key, Parameter,
    URef, U256,
};

mod constants;
mod error;
mod events;
mod model;

use constants::*;
use error::DisbursementError;
use events::{
    AdminTransferProposed, AdminTransferred, AttesterUpdated, CommitmentCancelled,
    CommitmentCreated, MilestoneAttested, MilestoneReleased,
};
use model::{Commitment, Milestone};

// ---------------------------------------------------------------
// Entry points -- admin / attester management
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
pub extern "C" fn is_attester() {
    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    runtime::ret(CLValue::from_t(is_attester_internal(account)).unwrap_or_revert());
}

/// Step 1 of admin transfer. Same semantics as
/// `DisbursementController.sol::proposeNewAdmin`.
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

/// Step 2: the proposed admin claims the role themselves. Same
/// semantics as `DisbursementController.sol::acceptAdmin`.
#[no_mangle]
pub extern "C" fn accept_admin() {
    let caller = Key::from(runtime::get_caller());
    let pending = get_pending_admin();

    match pending {
        Some(pending_admin) if pending_admin == caller => {
            let previous_admin = get_admin();
            set_admin(pending_admin);
            set_pending_admin(None);
            casper_event_standard::emit(AdminTransferred {
                previous_admin: Some(previous_admin),
                new_admin: pending_admin,
            });
        }
        _ => runtime::revert(DisbursementError::CallerIsNotPendingAdmin),
    }
}

// ---------------------------------------------------------------
// Entry points -- commitment lifecycle
// ---------------------------------------------------------------

/// Owner-only. Same guards/semantics as
/// `DisbursementController.sol::createCommitment`, including the live
/// `is_verified` cross-contract check against `identity-registry`
/// (not a locally cached verification flag).
#[no_mangle]
pub extern "C" fn create_commitment() {
    only_admin();

    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let token_package_hash: Key = runtime::get_named_arg(ARG_TOKEN_PACKAGE_HASH);
    let community: String = runtime::get_named_arg(ARG_COMMUNITY);
    let descriptions: Vec<String> = runtime::get_named_arg(ARG_DESCRIPTIONS);
    let amounts: Vec<U256> = runtime::get_named_arg(ARG_AMOUNTS);

    if descriptions.is_empty() {
        runtime::revert(DisbursementError::NoMilestones);
    }
    if descriptions.len() != amounts.len() {
        runtime::revert(DisbursementError::LengthMismatch);
    }

    let registry_package_hash = get_registry_package_hash();
    let registry_hash = key_to_package_hash(&registry_package_hash)
        .unwrap_or_revert_with(DisbursementError::UnexpectedKeyType);
    let is_verified: bool = runtime::call_versioned_contract(
        registry_hash,
        None,
        REMOTE_ENTRY_POINT_IS_VERIFIED,
        runtime_args! {
            REMOTE_ARG_RECIPIENT => recipient,
        },
    );
    if !is_verified {
        runtime::revert(DisbursementError::RecipientNotVerified);
    }

    let mut milestones = Vec::with_capacity(descriptions.len());
    let mut total_amount = U256::zero();
    let created_at = runtime::get_blocktime().into();

    for (description, amount) in descriptions.into_iter().zip(amounts.into_iter()) {
        if amount.is_zero() {
            runtime::revert(DisbursementError::ZeroMilestoneAmount);
        }
        total_amount += amount;
        milestones.push(Milestone {
            description,
            amount,
            attested: false,
            released: false,
            attested_by: None,
            attested_at: 0,
            released_at: 0,
        });
    }
    let milestone_count = milestones.len() as u64;

    let commitment_id = get_commitment_count();
    let commitment = Commitment {
        recipient,
        token_package_hash,
        community: community.clone(),
        active: true,
        cancelled: false,
        created_at,
        total_amount,
        released_amount: U256::zero(),
        milestones,
    };
    storage::dictionary_put(commitments_dict(), &commitment_id.to_string(), commitment);
    set_commitment_count(commitment_id + 1);

    casper_event_standard::emit(CommitmentCreated {
        commitment_id,
        recipient,
        token_package_hash,
        community,
        total_amount,
        milestone_count,
    });

    runtime::ret(CLValue::from_t(commitment_id).unwrap_or_revert());
}

/// Attester-only. Same guards/semantics as
/// `DisbursementController.sol::attestMilestone`.
#[no_mangle]
pub extern "C" fn attest_milestone() {
    only_attester();

    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);
    let milestone_index: u64 = runtime::get_named_arg(ARG_MILESTONE_INDEX);
    let caller = Key::from(runtime::get_caller());

    let mut commitment = get_active_commitment(commitment_id);
    let milestone_idx = milestone_index as usize;
    if milestone_idx >= commitment.milestones.len() {
        runtime::revert(DisbursementError::MilestoneDoesNotExist);
    }
    if commitment.milestones[milestone_idx].attested {
        runtime::revert(DisbursementError::MilestoneAlreadyAttested);
    }

    commitment.milestones[milestone_idx].attested = true;
    commitment.milestones[milestone_idx].attested_by = Some(caller);
    commitment.milestones[milestone_idx].attested_at = runtime::get_blocktime().into();
    storage::dictionary_put(commitments_dict(), &commitment_id.to_string(), commitment);

    casper_event_standard::emit(MilestoneAttested {
        commitment_id,
        milestone_index,
        attested_by: caller,
    });
}

/// Permissionless, matching `DisbursementController.sol::
/// releaseMilestone`'s explicit design choice: attestation is the
/// real gate, not who submits this call. See the module comment for
/// the real, not-yet-verified assumption this entry point rests on
/// (`d3rac-token`'s `get_caller()` seeing this contract's own
/// identity during the nested `transfer` call).
#[no_mangle]
pub extern "C" fn release_milestone() {
    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);
    let milestone_index: u64 = runtime::get_named_arg(ARG_MILESTONE_INDEX);

    let mut commitment = get_active_commitment(commitment_id);
    let milestone_idx = milestone_index as usize;
    if milestone_idx >= commitment.milestones.len() {
        runtime::revert(DisbursementError::MilestoneDoesNotExist);
    }
    let milestone = commitment.milestones[milestone_idx].clone();
    if !milestone.attested {
        runtime::revert(DisbursementError::MilestoneNotAttested);
    }
    if milestone.released {
        runtime::revert(DisbursementError::MilestoneAlreadyReleased);
    }

    let token_hash = key_to_package_hash(&commitment.token_package_hash)
        .unwrap_or_revert_with(DisbursementError::UnexpectedKeyType);
    let own_key = own_package_key();

    let own_balance: U256 = runtime::call_versioned_contract(
        token_hash,
        None,
        REMOTE_ENTRY_POINT_BALANCE_OF,
        runtime_args! {
            REMOTE_ARG_ACCOUNT => own_key,
        },
    );
    if own_balance < milestone.amount {
        runtime::revert(DisbursementError::InsufficientContractBalance);
    }

    // Effects before interaction -- same CEI ordering
    // `DisbursementController.sol::releaseMilestone` uses (marks
    // released, updates `releasedAmount`, and emits before the
    // external token-transfer call).
    commitment.milestones[milestone_idx].released = true;
    commitment.milestones[milestone_idx].released_at = runtime::get_blocktime().into();
    commitment.released_amount += milestone.amount;
    let recipient = commitment.recipient;
    storage::dictionary_put(commitments_dict(), &commitment_id.to_string(), commitment);

    casper_event_standard::emit(MilestoneReleased {
        commitment_id,
        milestone_index,
        recipient,
        amount: milestone.amount,
    });

    runtime::call_versioned_contract::<()>(
        token_hash,
        None,
        REMOTE_ENTRY_POINT_TRANSFER,
        runtime_args! {
            ARG_RECIPIENT => recipient,
            REMOTE_ARG_AMOUNT => milestone.amount,
        },
    );
}

/// Owner-only. Same guards/semantics as
/// `DisbursementController.sol::cancelCommitment`.
#[no_mangle]
pub extern "C" fn cancel_commitment() {
    only_admin();

    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);
    let caller = Key::from(runtime::get_caller());

    let mut commitment = get_active_commitment(commitment_id);
    commitment.active = false;
    commitment.cancelled = true;
    let unreleased = commitment.total_amount - commitment.released_amount;
    storage::dictionary_put(commitments_dict(), &commitment_id.to_string(), commitment);

    casper_event_standard::emit(CommitmentCancelled {
        commitment_id,
        cancelled_by: caller,
        unreleased_amount: unreleased,
    });
}

// ---------------------------------------------------------------
// Entry points -- views
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn commitment_count() {
    runtime::ret(CLValue::from_t(get_commitment_count()).unwrap_or_revert());
}

/// Returns the full `Commitment`, milestones included -- simpler than
/// `DisbursementController.sol::getCommitment`'s tuple-without-
/// milestones-array (a Solidity return-type limitation Casper's
/// `CLType::Any` doesn't share), with `get_milestone` still provided
/// separately below to match FR-3's adapter-parity expectation.
#[no_mangle]
pub extern "C" fn get_commitment() {
    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);
    let commitment = require_commitment(commitment_id);
    runtime::ret(CLValue::from_t(commitment).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn get_milestone() {
    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);
    let milestone_index: u64 = runtime::get_named_arg(ARG_MILESTONE_INDEX);
    let commitment = require_commitment(commitment_id);
    let milestone_idx = milestone_index as usize;
    if milestone_idx >= commitment.milestones.len() {
        runtime::revert(DisbursementError::MilestoneDoesNotExist);
    }
    runtime::ret(CLValue::from_t(commitment.milestones[milestone_idx].clone()).unwrap_or_revert());
}

// ---------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------

fn only_admin() {
    let caller = Key::from(runtime::get_caller());
    if caller != get_admin() {
        runtime::revert(DisbursementError::CallerIsNotAdmin);
    }
}

fn only_attester() {
    let caller = Key::from(runtime::get_caller());
    if !is_attester_internal(caller) {
        runtime::revert(DisbursementError::CallerIsNotAttester);
    }
}

fn is_attester_internal(account: Key) -> bool {
    storage::dictionary_get(attesters_dict(), &key_to_dict_key(&account))
        .unwrap_or_revert_with(DisbursementError::DictionaryReadFailed)
        .unwrap_or(false)
}

fn require_commitment(commitment_id: u64) -> Commitment {
    storage::dictionary_get(commitments_dict(), &commitment_id.to_string())
        .unwrap_or_revert_with(DisbursementError::DictionaryReadFailed)
        .unwrap_or_revert_with(DisbursementError::CommitmentDoesNotExist)
}

fn get_active_commitment(commitment_id: u64) -> Commitment {
    let commitment = require_commitment(commitment_id);
    if !commitment.active {
        runtime::revert(DisbursementError::CommitmentNotActive);
    }
    commitment
}

fn key_to_dict_key(key: &Key) -> String {
    key.to_string()
}

fn key_to_package_hash(key: &Key) -> Option<ContractPackageHash> {
    key.into_hash_addr().map(ContractPackageHash::new)
}

/// This contract's own package hash, as a `Key` -- the Casper analog
/// of Solidity's `address(this)`. See the module comment for why this
/// is the least-verified assumption in this file.
fn own_package_key() -> Key {
    *runtime::get_key(PACKAGE_HASH_KEY_NAME).unwrap_or_revert_with(DisbursementError::MissingKey)
}

fn get_registry_package_hash() -> Key {
    let uref: URef = runtime::get_key(KEY_REGISTRY_PACKAGE_HASH)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(DisbursementError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
}

fn get_admin() -> Key {
    let uref: URef = runtime::get_key(KEY_ADMIN)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(DisbursementError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
}

fn set_admin(new_admin: Key) {
    let uref: URef = runtime::get_key(KEY_ADMIN)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(DisbursementError::UnexpectedKeyType);
    storage::write(uref, new_admin);
}

fn get_pending_admin() -> Option<Key> {
    let uref: URef = runtime::get_key(KEY_PENDING_ADMIN)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(DisbursementError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
}

fn set_pending_admin(pending: Option<Key>) {
    let uref: URef = runtime::get_key(KEY_PENDING_ADMIN)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(DisbursementError::UnexpectedKeyType);
    storage::write(uref, pending);
}

fn get_commitment_count() -> u64 {
    let uref: URef = runtime::get_key(KEY_COMMITMENT_COUNT)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(DisbursementError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
}

fn set_commitment_count(count: u64) {
    let uref: URef = runtime::get_key(KEY_COMMITMENT_COUNT)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(DisbursementError::UnexpectedKeyType);
    storage::write(uref, count);
}

fn attesters_dict() -> URef {
    *runtime::get_key(KEY_ATTESTERS_DICT)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(DisbursementError::UnexpectedKeyType)
}

fn commitments_dict() -> URef {
    *runtime::get_key(KEY_COMMITMENTS_DICT)
        .unwrap_or_revert_with(DisbursementError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(DisbursementError::UnexpectedKeyType)
}

// ---------------------------------------------------------------
// Installer (`call`)
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn call() {
    let registry_package_hash: Key = runtime::get_named_arg(ARG_REGISTRY_PACKAGE_HASH);
    let admin: Key = runtime::get_named_arg(ARG_ADMIN);

    let mut named_keys = NamedKeys::new();

    named_keys.insert(
        KEY_REGISTRY_PACKAGE_HASH.to_string(),
        storage::new_uref(registry_package_hash).into(),
    );
    named_keys.insert(KEY_ADMIN.to_string(), storage::new_uref(admin).into());
    named_keys.insert(
        KEY_PENDING_ADMIN.to_string(),
        storage::new_uref(Option::<Key>::None).into(),
    );
    named_keys.insert(
        KEY_COMMITMENT_COUNT.to_string(),
        storage::new_uref(0u64).into(),
    );

    let attesters_dict_uref = storage::new_dictionary(KEY_ATTESTERS_DICT).unwrap_or_revert();
    named_keys.insert(KEY_ATTESTERS_DICT.to_string(), attesters_dict_uref.into());

    let commitments_dict_uref = storage::new_dictionary(KEY_COMMITMENTS_DICT).unwrap_or_revert();
    named_keys.insert(
        KEY_COMMITMENTS_DICT.to_string(),
        commitments_dict_uref.into(),
    );

    // Admin is always implicitly an attester at install, matching
    // `DisbursementController.sol`'s constructor
    // (`_grantRole(ATTESTER_ROLE, admin_)`) exactly.
    storage::dictionary_put(attesters_dict_uref, &key_to_dict_key(&admin), true);

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
            ARG_ADMIN => admin,
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
        .with::<CommitmentCreated>()
        .with::<MilestoneAttested>()
        .with::<MilestoneReleased>()
        .with::<CommitmentCancelled>()
        .with::<AdminTransferred>()
        .with::<AdminTransferProposed>()
        .with::<AttesterUpdated>();
    casper_event_standard::init(schemas);

    let admin: Key = runtime::get_named_arg(ARG_ADMIN);
    casper_event_standard::emit(AdminTransferred {
        previous_admin: None,
        new_admin: admin,
    });
    casper_event_standard::emit(AttesterUpdated {
        account: admin,
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
                Parameter::new(ARG_TOKEN_PACKAGE_HASH, CLType::Key),
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
            CLType::Any, // Commitment -- see model.rs's CLTyped impl.
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
            ENTRY_POINT_INIT,
            vec![Parameter::new(ARG_ADMIN, CLType::Key)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points
}
