//! D3RACHub — Casper port of
//! `contracts/tron/tronbox/contracts/D3RACHub.sol`.
//!
//! Behavioral parity target: the central coordinator sitting in front
//! of `d3rac-token`, `identity-registry`, `disbursement-controller`,
//! `risk-registry`, and `funding-request-registry` -- one admin
//! surface, one emergency pause, one aggregate status read. Same
//! "additive vs exclusive" wiring model as the TRON original (see
//! `D3RACHub.sol`'s own header comment, not re-derived here) --
//! `contracts/casper/README.md` needs its own "Wiring the Hub" section
//! once this is confirmed compiling, mirroring
//! `contracts/tron/README.md`'s.
//!
//! **Load-bearing dependency, not yet merged**: this contract's own
//! `only_admin()` check, and every cross-contract call it makes into
//! the other five contracts, depend on the callee correctly resolving
//! *this contract* as the caller (not the original transaction-
//! signing account) via `runtime::get_immediate_caller()`. As of this
//! writing, that fix exists only on branch
//! `fix/get-caller-systemic-immediate-caller` (found while writing this
//! file, not by this file) -- `risk-registry`, `identity-registry`,
//! `disbursement-controller`, and `multisig-admin` on `main` still use
//! the older, buggy `Key::from(runtime::get_caller())`, which resolves
//! to the original signing account even when called through an
//! intermediate contract. Until that branch merges, every Hub-mediated
//! `only_owner`/`only_admin`/role check on those four contracts will
//! misattribute the caller and revert. This file's OWN `only_admin()`
//! already uses the correct pattern (`immediate_caller_key`, copied
//! from that branch's `risk-registry`/`identity-registry`/
//! `multisig-admin` fixes) so it isn't adding a fifth instance of the
//! same bug -- but it cannot be *end-to-end* correct until that branch
//! lands, and this file's own cross-contract calls are equally
//! unverified against it besides.
//!
//! Two known, out-of-scope-to-fix-here gaps this file works around
//! rather than silently ports past:
//!
//! 1. `risk-registry`'s ownership transfer is single-step
//!    (`transfer_ownership`), not the two-step `proposeNewOwner`/
//!    `acceptOwnership` pair `D3RACHub.sol` was written against
//!    (`RiskRegistry.sol` itself IS two-step -- a gap in the existing
//!    Casper port, not a deliberate TRON<->Casper difference). This
//!    Hub's `transfer_risk_registry_ownership` entry point is
//!    therefore single-step too, matching what `risk-registry` actually
//!    exposes today, not a `propose_risk_registry_ownership`/
//!    `accept_risk_registry_ownership` pair.
//! 2. `disbursement-controller::create_commitment` declares a `U64`
//!    return type but never calls `runtime::ret` for it (confirmed by
//!    reading its source, not assumed) -- so this Hub's own
//!    `create_commitment` wrapper cannot reliably forward a
//!    `commitment_id` return value either, and doesn't try to. Callers
//!    should read the `CommitmentCreated` event `disbursement-
//!    controller` emits instead, same as the TRON Hub's off-chain
//!    integrators already do via its own `CommitmentCreated` log.
//!    `open_funding_request` does NOT have this problem -- this Hub's
//!    sibling `funding-request-registry::open_request` was written
//!    (by this same pass) to actually call `runtime::ret`, so its
//!    return value forwards correctly.
//!
//! No zero-address-style guard on `propose_new_admin`/`set_token`/
//! `set_identity_registry`/`set_disbursement_controller` (Solidity's
//! `require(x != address(0), ...)`): every required `Key` argument in
//! Casper's entry-point ABI must be a real, well-formed `Key` value to
//! deserialize at all -- there's no Casper-level equivalent of
//! Solidity's "syntactically valid but semantically zero" address to
//! guard against, unlike the optional `risk_registry`/
//! `funding_request_registry` modules, which use `Option<Key>`
//! specifically to express "may be absent."
//!
//! NOT yet independently confirmed compiling to wasm32-unknown-unknown
//! in CI -- see this same note on every other first-pass contract in
//! this suite, and the load-bearing-dependency note above for why this
//! one specifically also can't be end-to-end verified even once it
//! does compile, until the caller-resolution fix lands.

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
    contracts::{ContractHash, ContractPackageHash, EntryPoint, NamedKeys},
    runtime_args, AddressableEntityHash, CLType, CLValue, EntryPointAccess, EntryPointType,
    EntryPoints, Key, Parameter, URef, U256,
};

mod constants;
mod error;
mod events;
mod model;

use constants::*;
use error::D3racHubError;
use events::{AdminTransferProposed, AdminTransferred, ModuleUpdated, Paused, Unpaused};
use model::SystemStatusView;

// ---------------------------------------------------------------
// Admin / module management (always callable, even while paused)
// ---------------------------------------------------------------

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
    let caller = immediate_caller_key();
    let pending = get_pending_admin();
    match pending {
        Some(pending_admin) if pending_admin == caller => {
            let previous_admin = get_admin();
            set_admin(pending_admin);
            set_pending_admin(None);
            casper_event_standard::emit(AdminTransferred {
                previous_admin,
                new_admin: pending_admin,
            });
        }
        _ => runtime::revert(D3racHubError::CallerIsNotPendingAdmin),
    }
}

#[no_mangle]
pub extern "C" fn set_token() {
    only_admin();
    let new_token: Key = runtime::get_named_arg(ARG_NEW_TOKEN);
    let previous = get_token();
    set_token_internal(new_token);
    casper_event_standard::emit(ModuleUpdated {
        module: "token".to_string(),
        previous_address: Some(previous),
        new_address: Some(new_token),
    });
}

#[no_mangle]
pub extern "C" fn set_identity_registry() {
    only_admin();
    let new_registry: Key = runtime::get_named_arg(ARG_NEW_REGISTRY);
    let previous = get_identity_registry();
    set_identity_registry_internal(new_registry);
    casper_event_standard::emit(ModuleUpdated {
        module: "identity_registry".to_string(),
        previous_address: Some(previous),
        new_address: Some(new_registry),
    });
}

#[no_mangle]
pub extern "C" fn set_disbursement_controller() {
    only_admin();
    let new_controller: Key = runtime::get_named_arg(ARG_NEW_CONTROLLER);
    let previous = get_disbursement_controller();
    set_disbursement_controller_internal(new_controller);
    casper_event_standard::emit(ModuleUpdated {
        module: "disbursement_controller".to_string(),
        previous_address: Some(previous),
        new_address: Some(new_controller),
    });
}

/// Set or clear the RiskRegistry module -- `None` accepted, same
/// optionality as `D3RACHub.sol::setRiskRegistry`.
#[no_mangle]
pub extern "C" fn set_risk_registry() {
    only_admin();
    let new_risk_registry: Option<Key> = runtime::get_named_arg(ARG_NEW_RISK_REGISTRY);
    let previous = get_risk_registry();
    set_risk_registry_internal(new_risk_registry);
    casper_event_standard::emit(ModuleUpdated {
        module: "risk_registry".to_string(),
        previous_address: previous,
        new_address: new_risk_registry,
    });
}

#[no_mangle]
pub extern "C" fn set_funding_request_registry() {
    only_admin();
    let new_frr: Option<Key> = runtime::get_named_arg(ARG_NEW_FUNDING_REQUEST_REGISTRY);
    let previous = get_funding_request_registry();
    set_funding_request_registry_internal(new_frr);
    casper_event_standard::emit(ModuleUpdated {
        module: "funding_request_registry".to_string(),
        previous_address: previous,
        new_address: new_frr,
    });
}

// ---------------------------------------------------------------
// Emergency pause
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn pause() {
    only_admin();
    if get_paused() {
        runtime::revert(D3racHubError::AlreadyPaused);
    }
    set_paused(true);
    casper_event_standard::emit(Paused {
        by: immediate_caller_key(),
    });
}

#[no_mangle]
pub extern "C" fn unpause() {
    only_admin();
    if !get_paused() {
        runtime::revert(D3racHubError::NotPaused);
    }
    set_paused(false);
    casper_event_standard::emit(Unpaused {
        by: immediate_caller_key(),
    });
}

// ---------------------------------------------------------------
// Orchestration (admin + not-paused gated, except cancel/close --
// see D3RACHub.sol's own note on why those two stay callable while
// paused)
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn verify_recipient() {
    only_admin();
    when_not_paused();
    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let community: String = runtime::get_named_arg(ARG_COMMUNITY);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_identity_registry()),
        IDENTITY_REGISTRY_ENTRY_POINT_VERIFY_RECIPIENT,
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_COMMUNITY => community,
        },
    );
}

/// Does not return `commitment_id` -- see this file's header, gap #2.
#[no_mangle]
pub extern "C" fn create_commitment() {
    only_admin();
    when_not_paused();
    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let token: Key = runtime::get_named_arg(ARG_COMMITMENT_TOKEN);
    let community: String = runtime::get_named_arg(ARG_COMMUNITY);
    let descriptions: Vec<String> = runtime::get_named_arg(ARG_DESCRIPTIONS);
    let amounts: Vec<U256> = runtime::get_named_arg(ARG_AMOUNTS);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_disbursement_controller()),
        DISBURSEMENT_CONTROLLER_ENTRY_POINT_CREATE_COMMITMENT,
        runtime_args! {
            "recipient" => recipient,
            "token" => token,
            ARG_COMMUNITY => community,
            "descriptions" => descriptions,
            "amounts" => amounts,
        },
    );
}

#[no_mangle]
pub extern "C" fn attest_milestone() {
    only_admin();
    when_not_paused();
    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);
    let milestone_index: u64 = runtime::get_named_arg(ARG_MILESTONE_INDEX);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_disbursement_controller()),
        DISBURSEMENT_CONTROLLER_ENTRY_POINT_ATTEST_MILESTONE,
        runtime_args! {
            "commitment_id" => commitment_id,
            "milestone_index" => milestone_index,
        },
    );
}

/// Deliberately NOT `when_not_paused`-gated -- matches
/// `D3RACHub.sol::cancelCommitment`'s own note.
#[no_mangle]
pub extern "C" fn cancel_commitment() {
    only_admin();
    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_disbursement_controller()),
        DISBURSEMENT_CONTROLLER_ENTRY_POINT_CANCEL_COMMITMENT,
        runtime_args! { "commitment_id" => commitment_id },
    );
}

#[no_mangle]
pub extern "C" fn mint_tokens() {
    only_admin();
    when_not_paused();
    let to: Key = runtime::get_named_arg(ARG_TO);
    let value: U256 = runtime::get_named_arg(ARG_VALUE);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_token()),
        TOKEN_ENTRY_POINT_MINT,
        runtime_args! {
            "recipient" => to,
            "amount" => value,
        },
    );
}

#[no_mangle]
pub extern "C" fn register_community() {
    only_admin();
    when_not_paused();
    only_risk_registry_set();
    let community_id: String = runtime::get_named_arg(ARG_COMMUNITY_ID);
    let name: String = runtime::get_named_arg(ARG_NAME);
    let region: String = runtime::get_named_arg(ARG_REGION);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_risk_registry().unwrap_or_revert()),
        RISK_REGISTRY_ENTRY_POINT_REGISTER_COMMUNITY,
        runtime_args! {
            "community_id" => community_id,
            "name" => name,
            "region" => region,
        },
    );
}

#[no_mangle]
pub extern "C" fn update_risk() {
    only_admin();
    when_not_paused();
    only_risk_registry_set();
    let community_id: String = runtime::get_named_arg(ARG_COMMUNITY_ID);
    let hazard: u64 = runtime::get_named_arg(ARG_HAZARD);
    let exposure: u64 = runtime::get_named_arg(ARG_EXPOSURE);
    let vulnerability: u64 = runtime::get_named_arg(ARG_VULNERABILITY);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_risk_registry().unwrap_or_revert()),
        RISK_REGISTRY_ENTRY_POINT_UPDATE_RISK,
        runtime_args! {
            "community_id" => community_id,
            "hazard" => hazard,
            "exposure" => exposure,
            "vulnerability" => vulnerability,
        },
    );
}

/// Return value forwards correctly -- see this file's header, gap #2's
/// contrast with `create_commitment`.
#[no_mangle]
pub extern "C" fn open_funding_request() {
    only_admin();
    when_not_paused();
    only_funding_request_registry_set();
    let community_id: String = runtime::get_named_arg(ARG_COMMUNITY_ID);
    let amount_requested: U256 = runtime::get_named_arg(ARG_AMOUNT_REQUESTED);
    let description: String = runtime::get_named_arg(ARG_DESCRIPTION);
    let data_source_uri: String = runtime::get_named_arg(ARG_DATA_SOURCE_URI);
    let request_id: u64 = runtime::call_contract(
        key_to_contract_hash(get_funding_request_registry().unwrap_or_revert()),
        FRR_ENTRY_POINT_OPEN_REQUEST,
        runtime_args! {
            "community_id" => community_id,
            "amount_requested" => amount_requested,
            "description" => description,
            "data_source_uri" => data_source_uri,
        },
    );
    runtime::ret(CLValue::from_t(request_id).unwrap_or_revert());
}

/// Deliberately NOT `when_not_paused`-gated -- matches
/// `D3RACHub.sol::closeFundingRequest`'s own note.
#[no_mangle]
pub extern "C" fn close_funding_request() {
    only_admin();
    only_funding_request_registry_set();
    let request_id: u64 = runtime::get_named_arg(ARG_REQUEST_ID);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_funding_request_registry().unwrap_or_revert()),
        FRR_ENTRY_POINT_CLOSE_REQUEST,
        runtime_args! { "request_id" => request_id },
    );
}

// ---------------------------------------------------------------
// Role & ownership management on the underlying contracts (always
// callable, even while paused -- same reasoning as module management)
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn set_identity_verifier() {
    only_admin();
    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    let is_verifier: bool = runtime::get_named_arg(ARG_IS_VERIFIER);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_identity_registry()),
        IDENTITY_REGISTRY_ENTRY_POINT_SET_VERIFIER,
        runtime_args! { "account" => account, "is_verifier" => is_verifier },
    );
}

#[no_mangle]
pub extern "C" fn propose_identity_registry_admin() {
    only_admin();
    let new_admin: Key = runtime::get_named_arg(ARG_NEW_ADMIN);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_identity_registry()),
        IDENTITY_REGISTRY_ENTRY_POINT_PROPOSE_NEW_ADMIN,
        runtime_args! { "new_admin" => new_admin },
    );
}

#[no_mangle]
pub extern "C" fn accept_identity_registry_admin() {
    only_admin();
    runtime::call_contract::<()>(
        key_to_contract_hash(get_identity_registry()),
        IDENTITY_REGISTRY_ENTRY_POINT_ACCEPT_ADMIN,
        runtime_args! {},
    );
}

#[no_mangle]
pub extern "C" fn revoke_recipient() {
    only_admin();
    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_identity_registry()),
        IDENTITY_REGISTRY_ENTRY_POINT_REVOKE_RECIPIENT,
        runtime_args! { "recipient" => recipient },
    );
}

#[no_mangle]
pub extern "C" fn set_disbursement_attester() {
    only_admin();
    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    let is_attester: bool = runtime::get_named_arg(ARG_IS_ATTESTER);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_disbursement_controller()),
        DISBURSEMENT_CONTROLLER_ENTRY_POINT_SET_ATTESTER,
        runtime_args! { "account" => account, "is_attester" => is_attester },
    );
}

#[no_mangle]
pub extern "C" fn propose_disbursement_controller_admin() {
    only_admin();
    let new_admin: Key = runtime::get_named_arg(ARG_NEW_ADMIN);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_disbursement_controller()),
        DISBURSEMENT_CONTROLLER_ENTRY_POINT_PROPOSE_NEW_ADMIN,
        runtime_args! { "new_admin" => new_admin },
    );
}

#[no_mangle]
pub extern "C" fn accept_disbursement_controller_admin() {
    only_admin();
    runtime::call_contract::<()>(
        key_to_contract_hash(get_disbursement_controller()),
        DISBURSEMENT_CONTROLLER_ENTRY_POINT_ACCEPT_ADMIN,
        runtime_args! {},
    );
}

#[no_mangle]
pub extern "C" fn set_token_minter() {
    only_admin();
    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    let can_mint: bool = runtime::get_named_arg(ARG_CAN_MINT);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_token()),
        TOKEN_ENTRY_POINT_SET_MINTER,
        runtime_args! { "account" => account, "is_minter" => can_mint },
    );
}

#[no_mangle]
pub extern "C" fn propose_token_ownership() {
    only_admin();
    let new_owner: Key = runtime::get_named_arg(ARG_NEW_OWNER);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_token()),
        TOKEN_ENTRY_POINT_PROPOSE_NEW_OWNER,
        runtime_args! { "new_owner" => new_owner },
    );
}

#[no_mangle]
pub extern "C" fn accept_token_ownership() {
    only_admin();
    runtime::call_contract::<()>(
        key_to_contract_hash(get_token()),
        TOKEN_ENTRY_POINT_ACCEPT_OWNERSHIP,
        runtime_args! {},
    );
}

#[no_mangle]
pub extern "C" fn set_risk_data_feeder() {
    only_admin();
    only_risk_registry_set();
    let feeder: Key = runtime::get_named_arg(ARG_ACCOUNT);
    let is_feeder: bool = runtime::get_named_arg(ARG_IS_FEEDER);
    let entry_point = if is_feeder {
        RISK_REGISTRY_ENTRY_POINT_ADD_DATA_FEEDER
    } else {
        RISK_REGISTRY_ENTRY_POINT_REMOVE_DATA_FEEDER
    };
    runtime::call_contract::<()>(
        key_to_contract_hash(get_risk_registry().unwrap_or_revert()),
        entry_point,
        runtime_args! { "feeder" => feeder },
    );
}

#[no_mangle]
pub extern "C" fn set_risk_threshold() {
    only_admin();
    only_risk_registry_set();
    let new_threshold: u64 = runtime::get_named_arg(ARG_NEW_THRESHOLD);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_risk_registry().unwrap_or_revert()),
        RISK_REGISTRY_ENTRY_POINT_SET_RISK_THRESHOLD,
        runtime_args! { "new_threshold" => new_threshold },
    );
}

/// Single-step, NOT a propose/accept pair -- see this file's header,
/// gap #1.
#[no_mangle]
pub extern "C" fn transfer_risk_registry_ownership() {
    only_admin();
    only_risk_registry_set();
    let new_owner: Key = runtime::get_named_arg(ARG_NEW_OWNER);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_risk_registry().unwrap_or_revert()),
        RISK_REGISTRY_ENTRY_POINT_TRANSFER_OWNERSHIP,
        runtime_args! { "new_owner" => new_owner },
    );
}

#[no_mangle]
pub extern "C" fn set_funding_proposer() {
    only_admin();
    only_funding_request_registry_set();
    let proposer: Key = runtime::get_named_arg(ARG_ACCOUNT);
    let is_proposer: bool = runtime::get_named_arg(ARG_IS_PROPOSER);
    let entry_point = if is_proposer {
        FRR_ENTRY_POINT_ADD_PROPOSER
    } else {
        FRR_ENTRY_POINT_REMOVE_PROPOSER
    };
    runtime::call_contract::<()>(
        key_to_contract_hash(get_funding_request_registry().unwrap_or_revert()),
        entry_point,
        runtime_args! { "proposer" => proposer },
    );
}

#[no_mangle]
pub extern "C" fn record_funding_pledge() {
    only_admin();
    only_funding_request_registry_set();
    let request_id: u64 = runtime::get_named_arg(ARG_REQUEST_ID);
    let amount: U256 = runtime::get_named_arg(ARG_AMOUNT);
    let pledge_source_uri: String = runtime::get_named_arg(ARG_PLEDGE_SOURCE_URI);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_funding_request_registry().unwrap_or_revert()),
        FRR_ENTRY_POINT_RECORD_PLEDGE,
        runtime_args! {
            "request_id" => request_id,
            "amount" => amount,
            "pledge_source_uri" => pledge_source_uri,
        },
    );
}

#[no_mangle]
pub extern "C" fn link_funding_request_to_commitment() {
    only_admin();
    only_funding_request_registry_set();
    let request_id: u64 = runtime::get_named_arg(ARG_REQUEST_ID);
    let commitment_id: u64 = runtime::get_named_arg(ARG_COMMITMENT_ID);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_funding_request_registry().unwrap_or_revert()),
        FRR_ENTRY_POINT_LINK_TO_COMMITMENT,
        runtime_args! { "request_id" => request_id, "commitment_id" => commitment_id },
    );
}

#[no_mangle]
pub extern "C" fn propose_funding_request_registry_ownership() {
    only_admin();
    only_funding_request_registry_set();
    let new_owner: Key = runtime::get_named_arg(ARG_NEW_OWNER);
    runtime::call_contract::<()>(
        key_to_contract_hash(get_funding_request_registry().unwrap_or_revert()),
        FRR_ENTRY_POINT_PROPOSE_NEW_OWNER,
        runtime_args! { "new_owner" => new_owner },
    );
}

#[no_mangle]
pub extern "C" fn accept_funding_request_registry_ownership() {
    only_admin();
    only_funding_request_registry_set();
    runtime::call_contract::<()>(
        key_to_contract_hash(get_funding_request_registry().unwrap_or_revert()),
        FRR_ENTRY_POINT_ACCEPT_OWNERSHIP,
        runtime_args! {},
    );
}

// ---------------------------------------------------------------
// Aggregate status
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn system_status() {
    let token = get_token();
    let identity_registry = get_identity_registry();
    let disbursement_controller = get_disbursement_controller();
    let risk_registry = get_risk_registry();
    let funding_request_registry = get_funding_request_registry();

    let token_total_supply: U256 = runtime::call_contract(
        key_to_contract_hash(token),
        TOKEN_ENTRY_POINT_TOTAL_SUPPLY,
        runtime_args! {},
    );
    let total_commitments: u64 = runtime::call_contract(
        key_to_contract_hash(disbursement_controller),
        "commitment_count",
        runtime_args! {},
    );
    let total_communities: u64 = match risk_registry {
        Some(rr) => runtime::call_contract(
            key_to_contract_hash(rr),
            RISK_REGISTRY_ENTRY_POINT_COMMUNITY_COUNT,
            runtime_args! {},
        ),
        None => 0,
    };
    let total_funding_requests: u64 = match funding_request_registry {
        Some(frr) => runtime::call_contract(
            key_to_contract_hash(frr),
            FRR_ENTRY_POINT_REQUEST_COUNT,
            runtime_args! {},
        ),
        None => 0,
    };

    let view = SystemStatusView {
        token_address: token,
        identity_registry_address: identity_registry,
        disbursement_controller_address: disbursement_controller,
        risk_registry_address: risk_registry,
        funding_request_registry_address: funding_request_registry,
        is_paused: get_paused(),
        token_total_supply,
        total_commitments,
        total_communities,
        total_funding_requests,
    };
    runtime::ret(CLValue::from_t(view).unwrap_or_revert());
}

// ---------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------

/// Resolves the actual calling entity, NOT the original transaction
/// signer -- copied verbatim (module path aside) from
/// `identity-registry`/`risk-registry`/`multisig-admin`'s own
/// `immediate_caller_key` on `fix/get-caller-systemic-immediate-
/// caller` (see that branch, and this file's own header, for the full
/// story). Applied to every caller-identity check in this file --
/// `only_admin` above all, since the Hub's whole purpose is *being* an
/// intermediate caller for the other five contracts, and would
/// misidentify its own caller as the original EOA instead of e.g.
/// `multisig-admin` without this.
fn immediate_caller_key() -> Key {
    let caller_info = runtime::get_immediate_caller().unwrap_or_revert();
    match caller_info.kind() {
        0 => {
            let account_hash: Option<AccountHash> = caller_info
                .get_field_by_index(0)
                .unwrap_or_revert()
                .clone()
                .into_t()
                .unwrap_or_revert();
            Key::from(account_hash.unwrap_or_revert())
        }
        4 => {
            let contract_package_hash: Option<ContractPackageHash> = caller_info
                .get_field_by_index(2)
                .unwrap_or_revert()
                .clone()
                .into_t()
                .unwrap_or_revert();
            Key::from(contract_package_hash.unwrap_or_revert())
        }
        _ => runtime::revert(D3racHubError::UnrecognizedCallerKind),
    }
}

fn only_admin() {
    if immediate_caller_key() != get_admin() {
        runtime::revert(D3racHubError::CallerIsNotAdmin);
    }
}

fn when_not_paused() {
    if get_paused() {
        runtime::revert(D3racHubError::ContractIsPaused);
    }
}

fn only_risk_registry_set() {
    if get_risk_registry().is_none() {
        runtime::revert(D3racHubError::RiskRegistryNotSet);
    }
}

fn only_funding_request_registry_set() {
    if get_funding_request_registry().is_none() {
        runtime::revert(D3racHubError::FundingRequestRegistryNotSet);
    }
}

/// Same `Key` -> `AddressableEntityHash` -> `ContractHash` conversion
/// `disbursement-controller`'s own `get_registry_hash()` +
/// its call site's trailing `.into()` do together (`runtime::
/// call_contract` needs a `ContractHash` specifically, not the
/// `AddressableEntityHash` this contract's `Key` resolves to
/// directly) -- done here inside the helper itself, once, rather than
/// requiring every one of this file's 30+ call sites to remember an
/// `.into()` the way disbursement-controller's single call site does.
fn key_to_contract_hash(key: Key) -> ContractHash {
    let entity_hash: AddressableEntityHash = key
        .into_hash_addr()
        .map(AddressableEntityHash::new)
        .unwrap_or_revert_with(D3racHubError::UnexpectedKeyType);
    entity_hash.into()
}

fn get_uref(name: &str) -> URef {
    runtime::get_key(name)
        .unwrap_or_revert_with(D3racHubError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(D3racHubError::UnexpectedKeyType)
}

fn read_uref_value<T: casper_types::CLTyped + casper_types::bytesrepr::FromBytes>(name: &str) -> T {
    let uref = get_uref(name);
    storage::read(uref)
        .unwrap_or_revert_with(D3racHubError::MissingKey)
        .unwrap_or_revert_with(D3racHubError::MissingKey)
}

fn write_uref_value<T: casper_types::CLTyped + casper_types::bytesrepr::ToBytes>(
    name: &str,
    value: T,
) {
    let uref = get_uref(name);
    storage::write(uref, value);
}

fn get_admin() -> Key {
    read_uref_value(KEY_ADMIN)
}
fn set_admin(admin: Key) {
    write_uref_value(KEY_ADMIN, admin);
}
fn get_pending_admin() -> Option<Key> {
    read_uref_value(KEY_PENDING_ADMIN)
}
fn set_pending_admin(pending: Option<Key>) {
    write_uref_value(KEY_PENDING_ADMIN, pending);
}
fn get_paused() -> bool {
    read_uref_value(KEY_PAUSED)
}
fn set_paused(paused: bool) {
    write_uref_value(KEY_PAUSED, paused);
}
fn get_token() -> Key {
    read_uref_value(KEY_TOKEN)
}
fn set_token_internal(token: Key) {
    write_uref_value(KEY_TOKEN, token);
}
fn get_identity_registry() -> Key {
    read_uref_value(KEY_IDENTITY_REGISTRY)
}
fn set_identity_registry_internal(registry: Key) {
    write_uref_value(KEY_IDENTITY_REGISTRY, registry);
}
fn get_disbursement_controller() -> Key {
    read_uref_value(KEY_DISBURSEMENT_CONTROLLER)
}
fn set_disbursement_controller_internal(controller: Key) {
    write_uref_value(KEY_DISBURSEMENT_CONTROLLER, controller);
}
fn get_risk_registry() -> Option<Key> {
    read_uref_value(KEY_RISK_REGISTRY)
}
fn set_risk_registry_internal(registry: Option<Key>) {
    write_uref_value(KEY_RISK_REGISTRY, registry);
}
fn get_funding_request_registry() -> Option<Key> {
    read_uref_value(KEY_FUNDING_REQUEST_REGISTRY)
}
fn set_funding_request_registry_internal(registry: Option<Key>) {
    write_uref_value(KEY_FUNDING_REQUEST_REGISTRY, registry);
}

// ---------------------------------------------------------------
// Installer (`call`)
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn call() {
    let admin_arg: Key = runtime::get_named_arg(ARG_ADMIN);
    let token_arg: Key = runtime::get_named_arg(ARG_TOKEN);
    let identity_registry_arg: Key = runtime::get_named_arg(ARG_IDENTITY_REGISTRY);
    let disbursement_controller_arg: Key = runtime::get_named_arg(ARG_DISBURSEMENT_CONTROLLER);
    let risk_registry_arg: Option<Key> = runtime::get_named_arg(ARG_RISK_REGISTRY);
    let funding_request_registry_arg: Option<Key> =
        runtime::get_named_arg(ARG_FUNDING_REQUEST_REGISTRY);

    let mut named_keys = NamedKeys::new();
    named_keys.insert(KEY_ADMIN.to_string(), storage::new_uref(admin_arg).into());
    named_keys.insert(
        KEY_PENDING_ADMIN.to_string(),
        storage::new_uref(Option::<Key>::None).into(),
    );
    named_keys.insert(KEY_PAUSED.to_string(), storage::new_uref(false).into());
    named_keys.insert(KEY_TOKEN.to_string(), storage::new_uref(token_arg).into());
    named_keys.insert(
        KEY_IDENTITY_REGISTRY.to_string(),
        storage::new_uref(identity_registry_arg).into(),
    );
    named_keys.insert(
        KEY_DISBURSEMENT_CONTROLLER.to_string(),
        storage::new_uref(disbursement_controller_arg).into(),
    );
    named_keys.insert(
        KEY_RISK_REGISTRY.to_string(),
        storage::new_uref(risk_registry_arg).into(),
    );
    named_keys.insert(
        KEY_FUNDING_REQUEST_REGISTRY.to_string(),
        storage::new_uref(funding_request_registry_arg).into(),
    );

    let entry_points = build_entry_points();

    // Same 5-arg new_locked_contract signature, same fixed/locked-at-
    // install reasoning, as every other contract in this suite.
    let (contract_hash, _contract_version) = storage::new_locked_contract(
        entry_points,
        Some(named_keys),
        Some(PACKAGE_HASH_KEY_NAME.to_string()),
        Some(ACCESS_UREF_KEY_NAME.to_string()),
        None,
    );

    // Same context-boundary reasoning as every other contract's own
    // `call()` for why `init` runs via `runtime::call_contract`.
    runtime::call_contract::<()>(contract_hash, ENTRY_POINT_INIT, runtime_args! {});

    runtime::put_key(CONTRACT_HASH_KEY_NAME, contract_hash.into());
}

#[no_mangle]
pub extern "C" fn init() {
    let schemas = Schemas::new()
        .with::<AdminTransferred>()
        .with::<AdminTransferProposed>()
        .with::<ModuleUpdated>()
        .with::<Paused>()
        .with::<Unpaused>();
    casper_event_standard::init(schemas);

    // Same install-time event emission as D3RACHub.sol's constructor --
    // this reads the same named keys `call()` just wrote, but must run
    // from *this contract's* own execution context (see every other
    // contract's own `init` for the full context-boundary reasoning),
    // so re-reads rather than being passed the values again as args.
    let admin = get_admin();
    casper_event_standard::emit(AdminTransferred {
        previous_admin: admin, // no real "previous" at install time; TRON emits address(0)->admin_, closest Casper analog documented as this being the initial value, not a real transfer
        new_admin: admin,
    });
    casper_event_standard::emit(ModuleUpdated {
        module: "token".to_string(),
        previous_address: None,
        new_address: Some(get_token()),
    });
    casper_event_standard::emit(ModuleUpdated {
        module: "identity_registry".to_string(),
        previous_address: None,
        new_address: Some(get_identity_registry()),
    });
    casper_event_standard::emit(ModuleUpdated {
        module: "disbursement_controller".to_string(),
        previous_address: None,
        new_address: Some(get_disbursement_controller()),
    });
    casper_event_standard::emit(ModuleUpdated {
        module: "risk_registry".to_string(),
        previous_address: None,
        new_address: get_risk_registry(),
    });
    casper_event_standard::emit(ModuleUpdated {
        module: "funding_request_registry".to_string(),
        previous_address: None,
        new_address: get_funding_request_registry(),
    });
}

fn build_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();

    macro_rules! ep {
        ($name:expr, $params:expr, $ret:expr) => {
            entry_points.add_entry_point(
                EntryPoint::new($name, $params, $ret, EntryPointAccess::Public, EntryPointType::Called)
                    .into(),
            );
        };
    }

    ep!(ENTRY_POINT_PROPOSE_NEW_ADMIN, vec![Parameter::new(ARG_NEW_ADMIN, CLType::Key)], CLType::Unit);
    ep!(ENTRY_POINT_ACCEPT_ADMIN, Vec::new(), CLType::Unit);
    ep!(ENTRY_POINT_SET_TOKEN, vec![Parameter::new(ARG_NEW_TOKEN, CLType::Key)], CLType::Unit);
    ep!(ENTRY_POINT_SET_IDENTITY_REGISTRY, vec![Parameter::new(ARG_NEW_REGISTRY, CLType::Key)], CLType::Unit);
    ep!(ENTRY_POINT_SET_DISBURSEMENT_CONTROLLER, vec![Parameter::new(ARG_NEW_CONTROLLER, CLType::Key)], CLType::Unit);
    ep!(ENTRY_POINT_SET_RISK_REGISTRY, vec![Parameter::new(ARG_NEW_RISK_REGISTRY, CLType::Option(alloc::boxed::Box::new(CLType::Key)))], CLType::Unit);
    ep!(ENTRY_POINT_SET_FUNDING_REQUEST_REGISTRY, vec![Parameter::new(ARG_NEW_FUNDING_REQUEST_REGISTRY, CLType::Option(alloc::boxed::Box::new(CLType::Key)))], CLType::Unit);
    ep!(ENTRY_POINT_PAUSE, Vec::new(), CLType::Unit);
    ep!(ENTRY_POINT_UNPAUSE, Vec::new(), CLType::Unit);

    ep!(ENTRY_POINT_VERIFY_RECIPIENT, vec![Parameter::new(ARG_RECIPIENT, CLType::Key), Parameter::new(ARG_COMMUNITY, CLType::String)], CLType::Unit);
    ep!(ENTRY_POINT_CREATE_COMMITMENT, vec![
        Parameter::new(ARG_RECIPIENT, CLType::Key),
        Parameter::new(ARG_COMMITMENT_TOKEN, CLType::Key),
        Parameter::new(ARG_COMMUNITY, CLType::String),
        Parameter::new(ARG_DESCRIPTIONS, CLType::List(alloc::boxed::Box::new(CLType::String))),
        Parameter::new(ARG_AMOUNTS, CLType::List(alloc::boxed::Box::new(CLType::U256))),
    ], CLType::Unit);
    ep!(ENTRY_POINT_ATTEST_MILESTONE, vec![Parameter::new(ARG_COMMITMENT_ID, CLType::U64), Parameter::new(ARG_MILESTONE_INDEX, CLType::U64)], CLType::Unit);
    ep!(ENTRY_POINT_CANCEL_COMMITMENT, vec![Parameter::new(ARG_COMMITMENT_ID, CLType::U64)], CLType::Unit);
    ep!(ENTRY_POINT_MINT_TOKENS, vec![Parameter::new(ARG_TO, CLType::Key), Parameter::new(ARG_VALUE, CLType::U256)], CLType::Unit);
    ep!(ENTRY_POINT_REGISTER_COMMUNITY, vec![Parameter::new(ARG_COMMUNITY_ID, CLType::String), Parameter::new(ARG_NAME, CLType::String), Parameter::new(ARG_REGION, CLType::String)], CLType::Unit);
    ep!(ENTRY_POINT_UPDATE_RISK, vec![
        Parameter::new(ARG_COMMUNITY_ID, CLType::String),
        Parameter::new(ARG_HAZARD, CLType::U64),
        Parameter::new(ARG_EXPOSURE, CLType::U64),
        Parameter::new(ARG_VULNERABILITY, CLType::U64),
    ], CLType::Unit);
    ep!(ENTRY_POINT_OPEN_FUNDING_REQUEST, vec![
        Parameter::new(ARG_COMMUNITY_ID, CLType::String),
        Parameter::new(ARG_AMOUNT_REQUESTED, CLType::U256),
        Parameter::new(ARG_DESCRIPTION, CLType::String),
        Parameter::new(ARG_DATA_SOURCE_URI, CLType::String),
    ], CLType::U64);
    ep!(ENTRY_POINT_CLOSE_FUNDING_REQUEST, vec![Parameter::new(ARG_REQUEST_ID, CLType::U64)], CLType::Unit);

    ep!(ENTRY_POINT_SET_IDENTITY_VERIFIER, vec![Parameter::new(ARG_ACCOUNT, CLType::Key), Parameter::new(ARG_IS_VERIFIER, CLType::Bool)], CLType::Unit);
    ep!(ENTRY_POINT_PROPOSE_IDENTITY_REGISTRY_ADMIN, vec![Parameter::new(ARG_NEW_ADMIN, CLType::Key)], CLType::Unit);
    ep!(ENTRY_POINT_ACCEPT_IDENTITY_REGISTRY_ADMIN, Vec::new(), CLType::Unit);
    ep!(ENTRY_POINT_REVOKE_RECIPIENT, vec![Parameter::new(ARG_RECIPIENT, CLType::Key)], CLType::Unit);
    ep!(ENTRY_POINT_SET_DISBURSEMENT_ATTESTER, vec![Parameter::new(ARG_ACCOUNT, CLType::Key), Parameter::new(ARG_IS_ATTESTER, CLType::Bool)], CLType::Unit);
    ep!(ENTRY_POINT_PROPOSE_DISBURSEMENT_CONTROLLER_ADMIN, vec![Parameter::new(ARG_NEW_ADMIN, CLType::Key)], CLType::Unit);
    ep!(ENTRY_POINT_ACCEPT_DISBURSEMENT_CONTROLLER_ADMIN, Vec::new(), CLType::Unit);
    ep!(ENTRY_POINT_SET_TOKEN_MINTER, vec![Parameter::new(ARG_ACCOUNT, CLType::Key), Parameter::new(ARG_CAN_MINT, CLType::Bool)], CLType::Unit);
    ep!(ENTRY_POINT_PROPOSE_TOKEN_OWNERSHIP, vec![Parameter::new(ARG_NEW_OWNER, CLType::Key)], CLType::Unit);
    ep!(ENTRY_POINT_ACCEPT_TOKEN_OWNERSHIP, Vec::new(), CLType::Unit);
    ep!(ENTRY_POINT_SET_RISK_DATA_FEEDER, vec![Parameter::new(ARG_ACCOUNT, CLType::Key), Parameter::new(ARG_IS_FEEDER, CLType::Bool)], CLType::Unit);
    ep!(ENTRY_POINT_SET_RISK_THRESHOLD, vec![Parameter::new(ARG_NEW_THRESHOLD, CLType::U64)], CLType::Unit);
    ep!(ENTRY_POINT_TRANSFER_RISK_REGISTRY_OWNERSHIP, vec![Parameter::new(ARG_NEW_OWNER, CLType::Key)], CLType::Unit);
    ep!(ENTRY_POINT_SET_FUNDING_PROPOSER, vec![Parameter::new(ARG_ACCOUNT, CLType::Key), Parameter::new(ARG_IS_PROPOSER, CLType::Bool)], CLType::Unit);
    ep!(ENTRY_POINT_RECORD_FUNDING_PLEDGE, vec![
        Parameter::new(ARG_REQUEST_ID, CLType::U64),
        Parameter::new(ARG_AMOUNT, CLType::U256),
        Parameter::new(ARG_PLEDGE_SOURCE_URI, CLType::String),
    ], CLType::Unit);
    ep!(ENTRY_POINT_LINK_FUNDING_REQUEST_TO_COMMITMENT, vec![Parameter::new(ARG_REQUEST_ID, CLType::U64), Parameter::new(ARG_COMMITMENT_ID, CLType::U64)], CLType::Unit);
    ep!(ENTRY_POINT_PROPOSE_FUNDING_REQUEST_REGISTRY_OWNERSHIP, vec![Parameter::new(ARG_NEW_OWNER, CLType::Key)], CLType::Unit);
    ep!(ENTRY_POINT_ACCEPT_FUNDING_REQUEST_REGISTRY_OWNERSHIP, Vec::new(), CLType::Unit);

    ep!(ENTRY_POINT_SYSTEM_STATUS, Vec::new(), CLType::Any);

    ep!(ENTRY_POINT_INIT, Vec::new(), CLType::Unit);

    entry_points
}
