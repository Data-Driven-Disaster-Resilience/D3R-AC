//! FundingRequestRegistry — Casper port of
//! `contracts/tron/tronbox/contracts/FundingRequestRegistry.sol`.
//!
//! Behavioral parity target (see `docs/casper-contracts-srs.md`): a
//! public, permissionless-to-read funding-request board. Proposer-role-
//! gated writes for opening requests; owner-or-requester-gated writes
//! for the rest of a request's lifecycle (recording pledges, linking to
//! a `disbursement-controller` commitment, closing). Deliberately
//! standalone -- no cross-contract calls -- matching the TRON original,
//! which only *references* `RiskRegistry`/`DisbursementController` ids
//! rather than calling into them.
//!
//! Two-step owner transfer (`propose_new_owner`/`accept_ownership`),
//! matching `FundingRequestRegistry.sol`'s own `proposeNewOwner`/
//! `acceptOwnership` (already a two-step pattern on the TRON side, not
//! something this port is adding -- unlike `d3rac-token`'s M-2 fix,
//! which *did* change TRON's own contract from single- to two-step).
//! Implementation copied from identity-registry's already-CI-confirmed
//! `propose_new_admin`/`accept_admin`, renamed to match this contract's
//! own owner/pendingOwner naming.
//!
//! Casper-specific design decisions:
//!
//! - Requests are stored in a dictionary keyed by `request_id.to_string()`
//!   (same convention `disbursement-controller`'s `commitments` dictionary
//!   uses for `commitment_id`), with a separate `u64` counter
//!   (`KEY_REQUEST_COUNT`) standing in for `FundingRequest[].length` --
//!   Casper dictionaries don't support length/iteration the way a
//!   Solidity storage array does. See model.rs's header for more.
//! - No zero-address guard on `propose_new_owner` (Solidity's
//!   `require(newOwner != address(0), ...)`): neither `risk-registry`'s
//!   `transfer_ownership` nor `identity-registry`'s
//!   `propose_new_admin` implement an equivalent check, and Casper's
//!   `Key` (Account/Hash/URef/...) has no single canonical "zero"
//!   sentinel the way an EVM `address` does for this to faithfully
//!   port to -- following the established precedent already set by
//!   this suite's other two-step-transfer contracts, not inventing a
//!   new one here.
//!
//! NOT yet independently confirmed compiling to wasm32-unknown-unknown
//! in CI. Written the same way every other contract in this suite was
//! first written: as carefully as possible against confirmed-real APIs
//! (cross-checked against risk-registry's and identity-registry's own
//! already-green source rather than re-guessed from scratch), but this
//! is a first pass, not a confirmed-working file. This one in
//! particular was written from a sandbox with no working Rust
//! toolchain for this target at all (not even a stale-toolchain partial
//! check the way risk-registry originally got, per that file's own
//! "Sandbox limitations" note) -- see contracts/casper/README.md.

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
mod model;

use constants::*;
use error::FundingRequestRegistryError;
use events::{
    OwnershipTransferProposed, OwnershipTransferred, PledgeRecorded, ProposerAdded,
    ProposerRemoved, RequestLinkedToCommitment, RequestOpened, RequestStatusChanged,
};
use model::{FundingRequest, RequestStatus};

// ---------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------

/// Open a public funding request for a community. Proposer-role-gated,
/// same guard as `FundingRequestRegistry.sol::openRequest`.
#[no_mangle]
pub extern "C" fn open_request() {
    only_proposer();

    let community_id: String = runtime::get_named_arg(ARG_COMMUNITY_ID);
    let amount_requested: U256 = runtime::get_named_arg(ARG_AMOUNT_REQUESTED);
    let description: String = runtime::get_named_arg(ARG_DESCRIPTION);
    let data_source_uri: String = runtime::get_named_arg(ARG_DATA_SOURCE_URI);

    if amount_requested.is_zero() {
        runtime::revert(FundingRequestRegistryError::AmountMustBePositive);
    }

    let request_id = get_request_count();
    let requester = Key::from(runtime::get_caller());

    let record = FundingRequest {
        community_id: community_id.clone(),
        requester,
        amount_requested,
        amount_pledged: U256::zero(),
        description,
        data_source_uri: data_source_uri.clone(),
        linked_commitment_id: None,
        status: RequestStatus::Open,
        created_at: runtime::get_blocktime().into(),
        closed_at: 0,
    };
    storage::dictionary_put(requests_dict(), &request_id.to_string(), record);
    set_request_count(request_id + 1);

    casper_event_standard::emit(RequestOpened {
        request_id,
        community_id,
        requester,
        amount_requested,
        data_source_uri,
    });

    // Unlike disbursement-controller's create_commitment (which
    // declares a U64 return type but never actually calls
    // runtime::ret for it -- a pre-existing gap in already-merged
    // code, not touched here), this does return the new id, reusing
    // risk-registry's own proven runtime::ret(CLValue::from_t(...))
    // pattern -- nothing about that idiom is specific to read-only
    // entry points.
    runtime::ret(CLValue::from_t(request_id).unwrap_or_revert());
}

/// Record a pledge toward a request -- a ledger entry only, does not
/// move funds. Same guard/semantics as
/// `FundingRequestRegistry.sol::recordPledge`, including its automatic
/// `Open` -> `PartiallyFunded` -> `Funded` status transition.
#[no_mangle]
pub extern "C" fn record_pledge() {
    let request_id: u64 = runtime::get_named_arg(ARG_REQUEST_ID);
    let amount: U256 = runtime::get_named_arg(ARG_AMOUNT);
    let pledge_source_uri: String = runtime::get_named_arg(ARG_PLEDGE_SOURCE_URI);

    let mut record = get_request_or_revert(request_id);
    only_requester_or_owner(&record);

    if !matches!(record.status, RequestStatus::Open | RequestStatus::PartiallyFunded) {
        runtime::revert(FundingRequestRegistryError::RequestNotOpen);
    }
    if amount.is_zero() {
        runtime::revert(FundingRequestRegistryError::PledgeAmountMustBePositive);
    }

    // saturating_add, not checked_add -- matching d3rac-token's own
    // established, CI-confirmed U256 arithmetic convention (see e.g.
    // its increase_allowance) rather than an unverified guess at a
    // method this exact U256 type may or may not expose. Slightly
    // different failure mode than Solidity 0.8's default checked
    // arithmetic (which reverts on overflow; this caps at U256::MAX
    // instead of reverting) -- immaterial here in practice, since
    // reaching U256::MAX in cumulative pledged amount is not a
    // realistic scenario, but noted rather than silently changed.
    record.amount_pledged = record.amount_pledged.saturating_add(amount);

    let recorded_by = Key::from(runtime::get_caller());
    let previous = record.status;
    if record.amount_pledged >= record.amount_requested {
        record.status = RequestStatus::Funded;
    } else if !record.amount_pledged.is_zero() {
        record.status = RequestStatus::PartiallyFunded;
    }
    let new_status = record.status;

    storage::dictionary_put(requests_dict(), &request_id.to_string(), record);

    casper_event_standard::emit(PledgeRecorded {
        request_id,
        amount,
        pledge_source_uri,
        recorded_by,
    });

    if new_status != previous {
        casper_event_standard::emit(RequestStatusChanged {
            request_id,
            previous_status: previous,
            new_status,
        });
    }
}

/// Link this request to an actual `disbursement-controller` commitment
/// id. Same guard/semantics as
/// `FundingRequestRegistry.sol::linkToCommitment`.
#[no_mangle]
pub extern "C" fn link_to_commitment() {
    let request_id: u64 = runtime::get_named_arg(ARG_REQUEST_ID);
    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);

    let mut record = get_request_or_revert(request_id);
    only_requester_or_owner(&record);

    record.linked_commitment_id = Some(commitment_id);
    storage::dictionary_put(requests_dict(), &request_id.to_string(), record);

    casper_event_standard::emit(RequestLinkedToCommitment {
        request_id,
        commitment_id,
    });
}

/// Same guard/semantics as `FundingRequestRegistry.sol::closeRequest`.
#[no_mangle]
pub extern "C" fn close_request() {
    let request_id: u64 = runtime::get_named_arg(ARG_REQUEST_ID);

    let mut record = get_request_or_revert(request_id);
    only_requester_or_owner(&record);

    let previous = record.status;
    record.status = RequestStatus::Closed;
    record.closed_at = runtime::get_blocktime().into();
    storage::dictionary_put(requests_dict(), &request_id.to_string(), record);

    casper_event_standard::emit(RequestStatusChanged {
        request_id,
        previous_status: previous,
        new_status: RequestStatus::Closed,
    });
}

#[no_mangle]
pub extern "C" fn request_count() {
    runtime::ret(CLValue::from_t(get_request_count()).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn get_request() {
    let request_id: u64 = runtime::get_named_arg(ARG_REQUEST_ID);
    let record = get_request_or_revert(request_id);
    runtime::ret(CLValue::from_t(record).unwrap_or_revert());
}

/// Owner-only. Same guard/semantics as
/// `FundingRequestRegistry.sol::addProposer`.
#[no_mangle]
pub extern "C" fn add_proposer() {
    only_owner();
    let proposer: Key = runtime::get_named_arg(ARG_PROPOSER);
    storage::dictionary_put(proposers_dict(), &key_to_dict_key(&proposer), true);
    casper_event_standard::emit(ProposerAdded { proposer });
}

#[no_mangle]
pub extern "C" fn remove_proposer() {
    only_owner();
    let proposer: Key = runtime::get_named_arg(ARG_PROPOSER);
    storage::dictionary_put(proposers_dict(), &key_to_dict_key(&proposer), false);
    casper_event_standard::emit(ProposerRemoved { proposer });
}

/// Compatibility view -- same purpose as `FundingRequestRegistry.sol`'s
/// `proposers(address)` external view.
#[no_mangle]
pub extern "C" fn is_proposer() {
    let account: Key = runtime::get_named_arg(ARG_PROPOSER);
    runtime::ret(CLValue::from_t(is_proposer_internal(&account)).unwrap_or_revert());
}

/// Step 1 of owner transfer: propose a new owner. Owner-only. Same
/// semantics as `FundingRequestRegistry.sol::proposeNewOwner`.
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
/// semantics as `FundingRequestRegistry.sol::acceptOwnership`.
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
                previous_owner,
                new_owner: pending_owner,
            });
        }
        _ => runtime::revert(FundingRequestRegistryError::CallerIsNotPendingOwner),
    }
}

// ---------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------

fn only_owner() {
    let caller = Key::from(runtime::get_caller());
    if caller != get_owner() {
        runtime::revert(FundingRequestRegistryError::CallerIsNotOwner);
    }
}

fn only_proposer() {
    let caller = Key::from(runtime::get_caller());
    if !is_proposer_internal(&caller) {
        runtime::revert(FundingRequestRegistryError::CallerIsNotProposer);
    }
}

/// Shared guard for `record_pledge`/`link_to_commitment`/
/// `close_request` -- same `msg.sender == r.requester || msg.sender ==
/// owner` check as all three of their Solidity originals.
fn only_requester_or_owner(record: &FundingRequest) {
    let caller = Key::from(runtime::get_caller());
    if caller != record.requester && caller != get_owner() {
        runtime::revert(FundingRequestRegistryError::NotAuthorizedForRequest);
    }
}

fn is_proposer_internal(account: &Key) -> bool {
    storage::dictionary_get(proposers_dict(), &key_to_dict_key(account))
        .unwrap_or_revert_with(FundingRequestRegistryError::DictionaryReadFailed)
        .unwrap_or(false)
}

fn get_request_or_revert(request_id: u64) -> FundingRequest {
    storage::dictionary_get(requests_dict(), &request_id.to_string())
        .unwrap_or_revert_with(FundingRequestRegistryError::DictionaryReadFailed)
        .unwrap_or_revert_with(FundingRequestRegistryError::InvalidRequestId)
}

fn get_owner() -> Key {
    let uref: URef = runtime::get_key(KEY_OWNER)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(FundingRequestRegistryError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
}

fn set_owner(owner: Key) {
    let uref: URef = runtime::get_key(KEY_OWNER)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(FundingRequestRegistryError::UnexpectedKeyType);
    storage::write(uref, owner);
}

fn get_pending_owner() -> Option<Key> {
    let uref: URef = runtime::get_key(KEY_PENDING_OWNER)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(FundingRequestRegistryError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
}

fn set_pending_owner(pending: Option<Key>) {
    let uref: URef = runtime::get_key(KEY_PENDING_OWNER)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(FundingRequestRegistryError::UnexpectedKeyType);
    storage::write(uref, pending);
}

fn get_request_count() -> u64 {
    let uref: URef = runtime::get_key(KEY_REQUEST_COUNT)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(FundingRequestRegistryError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
}

fn set_request_count(count: u64) {
    let uref: URef = runtime::get_key(KEY_REQUEST_COUNT)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(FundingRequestRegistryError::UnexpectedKeyType);
    storage::write(uref, count);
}

fn requests_dict() -> URef {
    *runtime::get_key(KEY_REQUESTS_DICT)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(FundingRequestRegistryError::UnexpectedKeyType)
}

fn proposers_dict() -> URef {
    *runtime::get_key(KEY_PROPOSERS_DICT)
        .unwrap_or_revert_with(FundingRequestRegistryError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(FundingRequestRegistryError::UnexpectedKeyType)
}

/// Same "`Key` isn't itself a valid dictionary-item key type" reasoning
/// as risk-registry/src/main.rs's own `key_to_dict_key`.
fn key_to_dict_key(key: &Key) -> String {
    key.to_string()
}

// ---------------------------------------------------------------
// Installer (`call`)
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn call() {
    let initial_proposer: Option<Key> = runtime::get_named_arg(ARG_INITIAL_PROPOSER);

    let mut named_keys = NamedKeys::new();

    let owner_key = Key::from(runtime::get_caller());
    named_keys.insert(KEY_OWNER.to_string(), storage::new_uref(owner_key).into());
    named_keys.insert(
        KEY_PENDING_OWNER.to_string(),
        storage::new_uref(Option::<Key>::None).into(),
    );
    named_keys.insert(
        KEY_REQUEST_COUNT.to_string(),
        storage::new_uref(0u64).into(),
    );

    let requests_dict_uref = storage::new_dictionary(KEY_REQUESTS_DICT).unwrap_or_revert();
    named_keys.insert(KEY_REQUESTS_DICT.to_string(), requests_dict_uref.into());

    let proposers_dict_uref = storage::new_dictionary(KEY_PROPOSERS_DICT).unwrap_or_revert();
    named_keys.insert(KEY_PROPOSERS_DICT.to_string(), proposers_dict_uref.into());

    if let Some(proposer) = initial_proposer {
        storage::dictionary_put(proposers_dict_uref, &key_to_dict_key(&proposer), true);
    }

    let entry_points = build_entry_points();

    // Same 5-arg new_locked_contract signature, same reasoning for
    // fixed/locked-at-install over upgradeable, as risk-registry's own
    // `call()` -- not re-derived here.
    let (contract_hash, _contract_version) = storage::new_locked_contract(
        entry_points,
        Some(named_keys),
        Some(PACKAGE_HASH_KEY_NAME.to_string()),
        Some(ACCESS_UREF_KEY_NAME.to_string()),
        None,
    );

    // Same context-boundary reasoning as risk-registry's own `call()`
    // for why `init` runs via `runtime::call_contract` rather than a
    // plain Rust function call -- CES's own bookkeeping keys must be
    // written into *this contract's* named keys, not the installing
    // account's.
    runtime::call_contract::<()>(
        contract_hash,
        ENTRY_POINT_INIT,
        runtime_args! {
            ARG_INITIAL_PROPOSER => initial_proposer,
        },
    );

    runtime::put_key(CONTRACT_HASH_KEY_NAME, contract_hash.into());
}

#[no_mangle]
pub extern "C" fn init() {
    let schemas = Schemas::new()
        .with::<RequestOpened>()
        .with::<PledgeRecorded>()
        .with::<RequestLinkedToCommitment>()
        .with::<RequestStatusChanged>()
        .with::<ProposerAdded>()
        .with::<ProposerRemoved>()
        .with::<OwnershipTransferProposed>()
        .with::<OwnershipTransferred>();
    casper_event_standard::init(schemas);

    let initial_proposer: Option<Key> = runtime::get_named_arg(ARG_INITIAL_PROPOSER);
    if let Some(proposer) = initial_proposer {
        casper_event_standard::emit(ProposerAdded { proposer });
    }
}

fn build_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_OPEN_REQUEST,
            vec![
                Parameter::new(ARG_COMMUNITY_ID, CLType::String),
                Parameter::new(ARG_AMOUNT_REQUESTED, CLType::U256),
                Parameter::new(ARG_DESCRIPTION, CLType::String),
                Parameter::new(ARG_DATA_SOURCE_URI, CLType::String),
            ],
            CLType::U64,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_RECORD_PLEDGE,
            vec![
                Parameter::new(ARG_REQUEST_ID, CLType::U64),
                Parameter::new(ARG_AMOUNT, CLType::U256),
                Parameter::new(ARG_PLEDGE_SOURCE_URI, CLType::String),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_LINK_TO_COMMITMENT,
            vec![
                Parameter::new(ARG_REQUEST_ID, CLType::U64),
                Parameter::new(ARG_COMMITMENT_ID, CLType::U64),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_CLOSE_REQUEST,
            vec![Parameter::new(ARG_REQUEST_ID, CLType::U64)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_REQUEST_COUNT,
            Vec::new(),
            CLType::U64,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_GET_REQUEST,
            vec![Parameter::new(ARG_REQUEST_ID, CLType::U64)],
            CLType::Any,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_ADD_PROPOSER,
            vec![Parameter::new(ARG_PROPOSER, CLType::Key)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_REMOVE_PROPOSER,
            vec![Parameter::new(ARG_PROPOSER, CLType::Key)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_IS_PROPOSER,
            vec![Parameter::new(ARG_PROPOSER, CLType::Key)],
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
            vec![Parameter::new(
                ARG_INITIAL_PROPOSER,
                CLType::Option(alloc::boxed::Box::new(CLType::Key)),
            )],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points
}
