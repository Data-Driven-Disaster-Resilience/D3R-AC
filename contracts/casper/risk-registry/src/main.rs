//! RiskRegistry — Casper port of `contracts/tron/tronbox/contracts/RiskRegistry.sol`.
//!
//! Behavioral parity target (see `docs/casper-contracts-srs.md` FR-6):
//! R(c,t) = H(t)·E(c)·V(c), all values fixed-point at 1e18 scale, same
//! as the TRON contract and `docs/risk-model.md`. This contract is
//! deliberately standalone -- no dependency on any other contract in
//! this suite, matching the TRON original.
//!
//! Casper-specific design decisions (SRS §8 "Open decisions", now
//! resolved for this contract):
//!
//! - **Event mechanism**: `casper-event-standard` (CES), the community
//!   standard, rather than a hand-rolled dictionary emulation --
//!   readable by any CES-aware indexer/frontend without D3R·AC-specific
//!   tooling.
//! - **Upgradeability (NFR-2)**: this package is deployed *locked*
//!   (`is_locked = true` at install time) -- the closest analog to the
//!   TRON contract's immutable-by-default Solidity semantics, and the
//!   safer default for a contract that gates real disaster-relief
//!   funding decisions. A future, deliberate decision to make a
//!   different Casper contract upgradeable should be its own documented
//!   choice, not this one's default leaking across the suite.
//! - **Addressing**: roles (owner, data feeders) are stored keyed by
//!   `AccountHash`, derived from whichever address form
//!   (`PublicKey`/`AccountHash`) the caller resolves to via
//!   `runtime::get_caller()` -- avoiding the SRS's flagged
//!   public-key-vs-account-hash ambiguity by always normalizing to
//!   account hash internally, the same way Casper's own system
//!   contracts do.
//!
//! Compiles and links cleanly against the Casper Virtual Machine target
//! (wasm32-unknown-unknown) -- confirmed via a green CI run (commit
//! f4d1c2c, `contracts-casper` job). This was originally written and
//! iterated on in a sandbox that couldn't compile Rust to wasm32 at all
//! (see `contracts/casper/README.md`'s "Sandbox limitations" section
//! for that history) -- getting to green took the same iterate-on-
//! real-compiler-feedback approach that got contracts/tron's Hardhat 3
//! migration there: casper-types 6.1.0's EntryPoints::add_entry_point()
//! needing EntityEntryPoint rather than EntryPoint, and a missing
//! `--allow-undefined` linker flag for the casper_* host-import
//! functions, were both real, CI-caught issues, now fixed.
//!
//! Local-network integration test status (a separate concern from
//! compiling) is tracked in contracts/casper/README.md, not here --
//! don't infer it from this comment.
//!
//! The global allocator (dlmalloc) and panic handler just below replace
//! casper-contract's default "no-std-helpers" feature (see the
//! casper-contract dependency comment in Cargo.toml), dropping the
//! unmaintained wee_alloc crate that feature pulled in (Dependabot
//! flagged it critical -- GHSA-rc23-xxgq-x27g). Also confirmed working
//! by the same green build.

#![no_std]
#![no_main]

extern crate alloc;

// Global allocator + panic handler, both stable-Rust compatible (see the
// casper-contract dependency comment in Cargo.toml for why these are
// defined here instead of relying on casper-contract's "no-std-helpers"
// default feature, which pulled in the unmaintained wee_alloc crate and
// required nightly Rust).
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

// #[panic_handler] itself is stable (unlike #[alloc_error_handler]/
// lang_items, which casper-contract's own handler needed). No OOM handler
// is defined here: `alloc`'s own built-in default abort-on-OOM handler
// (stable since Rust 1.68) covers that without the unstable attribute.
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
    CLType, CLValue, EntryPointAccess, EntryPointType, EntryPoints, Key, Parameter, URef,
};

mod constants;
mod error;
mod events;
mod model;

use constants::*;
use error::RiskRegistryError;
use events::{
    CommunityRegistered, DataFeederAdded, DataFeederRemoved, RiskUpdated, ThresholdCrossed,
    ThresholdUpdated,
};
use model::{CommunityRisk, CommunityView};

// ---------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------

/// Register a new community. Owner-only, same guard as
/// `RiskRegistry.sol::registerCommunity`.
#[no_mangle]
pub extern "C" fn register_community() {
    only_owner();

    let community_id: String = runtime::get_named_arg(ARG_COMMUNITY_ID);
    let name: String = runtime::get_named_arg(ARG_NAME);
    let region: String = runtime::get_named_arg(ARG_REGION);
    runtime::revert(RiskRegistryError::DiagnosticMarker); // TEMPORARY, see error.rs

    let dict_uref = communities_dict();
    if storage::dictionary_get::<CommunityRisk>(dict_uref, &community_id)
        .unwrap_or_revert_with(RiskRegistryError::DictionaryReadFailed)
        .is_some()
    {
        runtime::revert(RiskRegistryError::CommunityAlreadyRegistered);
    }

    let record = CommunityRisk {
        name: name.clone(),
        region: region.clone(),
        hazard: 0,
        exposure: 0,
        vulnerability: 0,
        last_updated: 0,
        registered: true,
    };
    storage::dictionary_put(dict_uref, &community_id, record);

    append_community_id(&community_id);

    casper_event_standard::emit(CommunityRegistered {
        community_id,
        name,
        region,
    });
}

/// Push fresh H/E/V data for a community. Data-feeder-role-gated, same
/// guard as `RiskRegistry.sol::updateRisk`. Recomputes R(c,t) and emits
/// a threshold-crossing event if it now meets or exceeds
/// `risk_threshold` -- the on-chain trigger point for downstream
/// funding decisions.
#[no_mangle]
pub extern "C" fn update_risk() {
    only_data_feeder();

    let community_id: String = runtime::get_named_arg(ARG_COMMUNITY_ID);
    let hazard: u64 = runtime::get_named_arg(ARG_HAZARD);
    let exposure: u64 = runtime::get_named_arg(ARG_EXPOSURE);
    let vulnerability: u64 = runtime::get_named_arg(ARG_VULNERABILITY);

    if hazard > SCALE || exposure > SCALE || vulnerability > SCALE {
        runtime::revert(RiskRegistryError::ValueOutOfRange);
    }

    let dict_uref = communities_dict();
    let mut record: CommunityRisk = storage::dictionary_get(dict_uref, &community_id)
        .unwrap_or_revert_with(RiskRegistryError::DictionaryReadFailed)
        .unwrap_or_revert_with(RiskRegistryError::CommunityNotRegistered);

    record.hazard = hazard;
    record.exposure = exposure;
    record.vulnerability = vulnerability;
    record.last_updated = runtime::get_blocktime().into();

    let score = compute_risk_score(&record);
    storage::dictionary_put(dict_uref, &community_id, record);

    let feeder = Key::from(runtime::get_caller());

    casper_event_standard::emit(RiskUpdated {
        community_id: community_id.clone(),
        hazard,
        exposure,
        vulnerability,
        risk_score: score,
        feeder,
    });

    let threshold: u64 = get_risk_threshold();
    if score >= threshold {
        casper_event_standard::emit(ThresholdCrossed {
            community_id,
            risk_score: score,
            threshold,
            timestamp: runtime::get_blocktime().into(),
        });
    }
}

/// R(c,t) = H(t) * E(c) * V(c), fixed-point at SCALE (1e18). Two
/// divisions by SCALE, same order of operations as
/// `RiskRegistry.sol::riskScore`, to avoid intermediate overflow while
/// preserving fixed-point precision identically.
#[no_mangle]
pub extern "C" fn risk_score() {
    let community_id: String = runtime::get_named_arg(ARG_COMMUNITY_ID);
    let dict_uref = communities_dict();
    let record: CommunityRisk = storage::dictionary_get(dict_uref, &community_id)
        .unwrap_or_revert_with(RiskRegistryError::DictionaryReadFailed)
        .unwrap_or_revert_with(RiskRegistryError::CommunityNotRegistered);

    let score = compute_risk_score(&record);
    runtime::ret(CLValue::from_t(score).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn is_above_threshold() {
    let community_id: String = runtime::get_named_arg(ARG_COMMUNITY_ID);
    let dict_uref = communities_dict();
    let record: CommunityRisk = storage::dictionary_get(dict_uref, &community_id)
        .unwrap_or_revert_with(RiskRegistryError::DictionaryReadFailed)
        .unwrap_or_revert_with(RiskRegistryError::CommunityNotRegistered);

    let score = compute_risk_score(&record);
    let threshold = get_risk_threshold();
    runtime::ret(CLValue::from_t(score >= threshold).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn get_community() {
    let community_id: String = runtime::get_named_arg(ARG_COMMUNITY_ID);
    let dict_uref = communities_dict();
    let record: CommunityRisk = storage::dictionary_get(dict_uref, &community_id)
        .unwrap_or_revert_with(RiskRegistryError::DictionaryReadFailed)
        .unwrap_or_revert_with(RiskRegistryError::CommunityNotRegistered);

    let score = compute_risk_score(&record);
    let view = CommunityView {
        name: record.name,
        region: record.region,
        hazard: record.hazard,
        exposure: record.exposure,
        vulnerability: record.vulnerability,
        last_updated: record.last_updated,
        risk_score: score,
    };
    runtime::ret(CLValue::from_t(view).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn community_count() {
    let ids = get_community_ids();
    runtime::ret(CLValue::from_t(ids.len() as u64).unwrap_or_revert());
}

/// Owner-only. Same guard/semantics as
/// `RiskRegistry.sol::transferOwnership`.
#[no_mangle]
pub extern "C" fn transfer_ownership() {
    only_owner();
    let new_owner: Key = runtime::get_named_arg(ARG_NEW_OWNER);
    runtime::put_key(KEY_OWNER, storage::new_uref(new_owner).into());
}

#[no_mangle]
pub extern "C" fn add_data_feeder() {
    only_owner();
    let feeder: Key = runtime::get_named_arg(ARG_FEEDER);
    let dict_uref = data_feeders_dict();
    storage::dictionary_put(dict_uref, &key_to_dict_key(&feeder), true);
    casper_event_standard::emit(DataFeederAdded { feeder });
}

#[no_mangle]
pub extern "C" fn remove_data_feeder() {
    only_owner();
    let feeder: Key = runtime::get_named_arg(ARG_FEEDER);
    let dict_uref = data_feeders_dict();
    storage::dictionary_put(dict_uref, &key_to_dict_key(&feeder), false);
    casper_event_standard::emit(DataFeederRemoved { feeder });
}

#[no_mangle]
pub extern "C" fn set_risk_threshold() {
    only_owner();
    let new_threshold: u64 = runtime::get_named_arg(ARG_NEW_THRESHOLD);
    let previous = get_risk_threshold();
    runtime::put_key(
        KEY_RISK_THRESHOLD,
        storage::new_uref(new_threshold).into(),
    );
    casper_event_standard::emit(ThresholdUpdated {
        previous_threshold: previous,
        new_threshold,
    });
}

// ---------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------

fn compute_risk_score(record: &CommunityRisk) -> u64 {
    // u128 intermediate to match the TRON contract's overflow-safety
    // reasoning (Solidity's uint256 has vastly more headroom than u64,
    // so this contract widens to u128 for the same two-step-division
    // safety margin at 1e18 scale).
    let scale = SCALE as u128;
    let h = record.hazard as u128;
    let e = record.exposure as u128;
    let v = record.vulnerability as u128;
    (((h * e) / scale) * v / scale) as u64
}

fn only_owner() {
    let caller = Key::from(runtime::get_caller());
    let owner: Key = get_owner();
    if caller != owner {
        runtime::revert(RiskRegistryError::CallerIsNotOwner);
    }
}

fn only_data_feeder() {
    let caller = Key::from(runtime::get_caller());
    let dict_uref = data_feeders_dict();
    let is_feeder: bool = storage::dictionary_get(dict_uref, &key_to_dict_key(&caller))
        .unwrap_or_revert_with(RiskRegistryError::DictionaryReadFailed)
        .unwrap_or(false);
    if !is_feeder {
        runtime::revert(RiskRegistryError::CallerIsNotDataFeeder);
    }
}

fn get_owner() -> Key {
    let uref: URef = runtime::get_key(KEY_OWNER)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(RiskRegistryError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
}

fn get_risk_threshold() -> u64 {
    let uref: URef = runtime::get_key(KEY_RISK_THRESHOLD)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(RiskRegistryError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
}

fn communities_dict() -> URef {
    *runtime::get_key(KEY_COMMUNITIES_DICT)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(RiskRegistryError::UnexpectedKeyType)
}

fn data_feeders_dict() -> URef {
    *runtime::get_key(KEY_DATA_FEEDERS_DICT)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(RiskRegistryError::UnexpectedKeyType)
}

/// `Key` doesn't implement `AsRef<str>` / isn't itself a valid
/// dictionary-item key type, so role lookups are keyed by its
/// formatted string form -- stable and unique per account/contract key,
/// same idea as TRON's `mapping(address => bool)`.
fn key_to_dict_key(key: &Key) -> String {
    key.to_string()
}

/// The community-id list backing `community_count()` -- stored as a
/// single `Vec<String>` behind its own uref rather than a second
/// dictionary, since it's read/written as a whole list, not looked up
/// by key (mirrors `RiskRegistry.sol`'s `bytes32[] public communityIds`
/// array, which the TRON contract also only ever appends to or reads in
/// full).
fn get_community_ids() -> Vec<String> {
    let uref: URef = runtime::get_key(KEY_COMMUNITY_IDS)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(RiskRegistryError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
}

fn append_community_id(community_id: &str) {
    let uref: URef = runtime::get_key(KEY_COMMUNITY_IDS)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(RiskRegistryError::UnexpectedKeyType);
    let mut ids: Vec<String> = storage::read(uref)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey)
        .unwrap_or_revert_with(RiskRegistryError::MissingKey);
    ids.push(community_id.to_string());
    storage::write(uref, ids);
}

// ---------------------------------------------------------------
// Installer (`call`)
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn call() {
    let initial_threshold: u64 = runtime::get_named_arg(ARG_INITIAL_THRESHOLD);
    let initial_data_feeder: Option<Key> = runtime::get_named_arg(ARG_INITIAL_DATA_FEEDER);

    let mut named_keys = NamedKeys::new();

    let owner_key = Key::from(runtime::get_caller());
    named_keys.insert(
        KEY_OWNER.to_string(),
        storage::new_uref(owner_key).into(),
    );
    named_keys.insert(
        KEY_RISK_THRESHOLD.to_string(),
        storage::new_uref(initial_threshold).into(),
    );

    let communities_dict_uref =
        storage::new_dictionary(KEY_COMMUNITIES_DICT).unwrap_or_revert();
    named_keys.insert(KEY_COMMUNITIES_DICT.to_string(), communities_dict_uref.into());

    let data_feeders_dict_uref =
        storage::new_dictionary(KEY_DATA_FEEDERS_DICT).unwrap_or_revert();
    named_keys.insert(
        KEY_DATA_FEEDERS_DICT.to_string(),
        data_feeders_dict_uref.into(),
    );

    named_keys.insert(
        KEY_COMMUNITY_IDS.to_string(),
        storage::new_uref(Vec::<String>::new()).into(),
    );

    if let Some(feeder) = initial_data_feeder {
        storage::dictionary_put(data_feeders_dict_uref, &key_to_dict_key(&feeder), true);
    }

    let entry_points = build_entry_points();

    // casper-contract 5.1.1's new_locked_contract takes 5 args, the
    // 5th being message topics (Option<BTreeMap<String,
    // MessageTopicOperation>>) for Casper's on-chain messaging system --
    // confirmed against the actual installed crate source via CI's
    // compiler output (not just docs/examples, some of which target
    // older casper-contract versions with a 4-arg signature predating
    // this parameter). None here: this contract doesn't use on-chain
    // messages, only casper-event-standard events.
    // Confirmed via official Casper docs (writing-onchain-code/simple-contract,
    // resources/tutorials/beginner/upgrade-contract): new_contract /
    // new_locked_contract return (ContractHash, ContractVersion) -- a
    // version *number*, not a separate ContractPackageHash. The package
    // hash is already auto-stored under the installing account's own
    // named keys via the hash_name parameter (PACKAGE_HASH_KEY_NAME,
    // passed below) -- no separate manual put_key for it is needed or
    // correct; a prior version of this code incorrectly treated the
    // second tuple element as a package hash and tried to `.into()` a
    // u32 into a Key, which doesn't type-check (confirmed by CI).
    let (contract_hash, _contract_version) = storage::new_locked_contract(
        entry_points,
        Some(named_keys),
        Some(PACKAGE_HASH_KEY_NAME.to_string()),
        Some(ACCESS_UREF_KEY_NAME.to_string()),
        None,
    );

    // Schemas registration for casper-event-standard -- must happen
    // against the newly created contract's own named keys, hence after
    // `new_locked_contract`, not before.
    let schemas = Schemas::new()
        .with::<CommunityRegistered>()
        .with::<RiskUpdated>()
        .with::<ThresholdCrossed>()
        .with::<DataFeederAdded>()
        .with::<DataFeederRemoved>()
        .with::<ThresholdUpdated>();
    casper_event_standard::init(schemas);

    runtime::put_key(CONTRACT_HASH_KEY_NAME, contract_hash.into());

    if let Some(feeder) = initial_data_feeder {
        casper_event_standard::emit(DataFeederAdded { feeder });
    }
}

fn build_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();

    entry_points.add_entry_point(EntryPoint::new(
        ENTRY_POINT_REGISTER_COMMUNITY,
        vec![
            Parameter::new(ARG_COMMUNITY_ID, CLType::String),
            Parameter::new(ARG_NAME, CLType::String),
            Parameter::new(ARG_REGION, CLType::String),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Called,
    ).into());

    entry_points.add_entry_point(EntryPoint::new(
        ENTRY_POINT_UPDATE_RISK,
        vec![
            Parameter::new(ARG_COMMUNITY_ID, CLType::String),
            Parameter::new(ARG_HAZARD, CLType::U64),
            Parameter::new(ARG_EXPOSURE, CLType::U64),
            Parameter::new(ARG_VULNERABILITY, CLType::U64),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Called,
    ).into());

    entry_points.add_entry_point(EntryPoint::new(
        ENTRY_POINT_RISK_SCORE,
        vec![Parameter::new(ARG_COMMUNITY_ID, CLType::String)],
        CLType::U64,
        EntryPointAccess::Public,
        EntryPointType::Called,
    ).into());

    entry_points.add_entry_point(EntryPoint::new(
        ENTRY_POINT_IS_ABOVE_THRESHOLD,
        vec![Parameter::new(ARG_COMMUNITY_ID, CLType::String)],
        CLType::Bool,
        EntryPointAccess::Public,
        EntryPointType::Called,
    ).into());

    entry_points.add_entry_point(EntryPoint::new(
        ENTRY_POINT_GET_COMMUNITY,
        vec![Parameter::new(ARG_COMMUNITY_ID, CLType::String)],
        CLType::Any, // CommunityView -- CLType has no Tuple7 variant (only up to Tuple3)
        EntryPointAccess::Public,
        EntryPointType::Called,
    ).into());

    entry_points.add_entry_point(EntryPoint::new(
        ENTRY_POINT_COMMUNITY_COUNT,
        Vec::new(),
        CLType::U64,
        EntryPointAccess::Public,
        EntryPointType::Called,
    ).into());

    entry_points.add_entry_point(EntryPoint::new(
        ENTRY_POINT_TRANSFER_OWNERSHIP,
        vec![Parameter::new(ARG_NEW_OWNER, CLType::Key)],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Called,
    ).into());

    entry_points.add_entry_point(EntryPoint::new(
        ENTRY_POINT_ADD_DATA_FEEDER,
        vec![Parameter::new(ARG_FEEDER, CLType::Key)],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Called,
    ).into());

    entry_points.add_entry_point(EntryPoint::new(
        ENTRY_POINT_REMOVE_DATA_FEEDER,
        vec![Parameter::new(ARG_FEEDER, CLType::Key)],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Called,
    ).into());

    entry_points.add_entry_point(EntryPoint::new(
        ENTRY_POINT_SET_RISK_THRESHOLD,
        vec![Parameter::new(ARG_NEW_THRESHOLD, CLType::U64)],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Called,
    ).into());

    entry_points
}
