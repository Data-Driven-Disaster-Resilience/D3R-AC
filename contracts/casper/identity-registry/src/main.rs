//! IdentityRegistry — Casper port of
//! `contracts/tron/tronbox/contracts/IdentityRegistry.sol`.
//!
//! Behavioral parity target: an admin-designated `verifiers` role can
//! verify a recipient account against a community label and revoke that
//! verification, with two-step admin-transfer semantics
//! (`propose_new_admin`/`accept_admin`) matching the TRON contract's
//! own `proposeNewAdmin`/`acceptAdmin` exactly (see
//! `docs/casper-contracts-srs.md` FR-2). Dependency-free by design,
//! matching `IdentityRegistry.sol`'s own "Dependency-free by design"
//! note -- this contract doesn't call into any other contract in the
//! suite.
//!
//! Same boilerplate, same design decisions, same hard-won lessons as
//! `risk-registry/src/main.rs` -- see that file's own module comment
//! for the detailed rationale behind the global allocator/panic
//! handler, `is_locked = true`, AccountHash-normalized addressing, the
//! `init()` self-initialization pattern, and the exact
//! `new_locked_contract` 5-argument signature. Not re-derived here;
//! this file follows the same template, adapted for this contract's own
//! entry points and storage shape.
//!
//! NOT yet independently confirmed compiling to wasm32-unknown-unknown
//! in CI (unlike risk-registry, which has a green CI run behind it) --
//! written by carefully following that confirmed-working file's exact
//! patterns, but this is genuinely the first real compiler pass this
//! specific code will get. See contracts/casper/README.md for current,
//! itemized status; don't infer a green build from this comment alone.

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
    URef,
};

mod constants;
mod error;
mod events;
mod model;

use constants::*;
use error::IdentityRegistryError;
use events::{
    AdminTransferProposed, AdminTransferred, RecipientRevoked, RecipientVerified, VerifierUpdated,
};
use model::Recipient;

// ---------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------

/// Admin-only. Same guard/semantics as
/// `IdentityRegistry.sol::setVerifier`.
#[no_mangle]
pub extern "C" fn set_verifier() {
    only_admin();

    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    let is_verifier: bool = runtime::get_named_arg(ARG_IS_VERIFIER);

    let dict_uref = verifiers_dict();
    storage::dictionary_put(dict_uref, &key_to_dict_key(&account), is_verifier);

    casper_event_standard::emit(VerifierUpdated {
        account,
        is_verifier,
    });
}

/// Step 1 of admin transfer: propose a new admin. Admin-only. Same
/// semantics as `IdentityRegistry.sol::proposeNewAdmin` -- takes effect
/// only once `new_admin` itself calls `accept_admin()`.
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
/// semantics as `IdentityRegistry.sol::acceptAdmin` -- reverts unless
/// the caller IS the pending admin, including when no transfer has
/// been proposed at all (`pending_admin` is `None`).
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
                previous_admin,
                new_admin: pending_admin,
            });
        }
        _ => runtime::revert(IdentityRegistryError::CallerIsNotPendingAdmin),
    }
}

/// Verifier-role-gated. Same guard/semantics as
/// `IdentityRegistry.sol::verifyRecipient`.
#[no_mangle]
pub extern "C" fn verify_recipient() {
    only_verifier();

    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let community: String = runtime::get_named_arg(ARG_COMMUNITY);

    if community.is_empty() {
        runtime::revert(IdentityRegistryError::CommunityLabelRequired);
    }

    let caller = Key::from(runtime::get_caller());
    let record = Recipient {
        verified: true,
        community: community.clone(),
        verified_by: caller,
        verified_at: runtime::get_blocktime().into(),
        revoked_at: 0,
    };

    let dict_uref = recipients_dict();
    storage::dictionary_put(dict_uref, &key_to_dict_key(&recipient), record);

    casper_event_standard::emit(RecipientVerified {
        recipient,
        community,
        verified_by: caller,
    });
}

/// Verifier-role-gated. Same guard/semantics as
/// `IdentityRegistry.sol::revokeRecipient` -- does not erase history,
/// same as the TRON contract.
#[no_mangle]
pub extern "C" fn revoke_recipient() {
    only_verifier();

    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let dict_uref = recipients_dict();

    let mut record: Recipient = storage::dictionary_get(dict_uref, &key_to_dict_key(&recipient))
        .unwrap_or_revert_with(IdentityRegistryError::DictionaryReadFailed)
        .unwrap_or_default();

    if !record.verified {
        runtime::revert(IdentityRegistryError::RecipientNotVerified);
    }

    record.verified = false;
    record.revoked_at = runtime::get_blocktime().into();
    storage::dictionary_put(dict_uref, &key_to_dict_key(&recipient), record);

    let caller = Key::from(runtime::get_caller());
    casper_event_standard::emit(RecipientRevoked {
        recipient,
        revoked_by: caller,
    });
}

#[no_mangle]
pub extern "C" fn is_verified() {
    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let dict_uref = recipients_dict();
    let record: Recipient = storage::dictionary_get(dict_uref, &key_to_dict_key(&recipient))
        .unwrap_or_revert_with(IdentityRegistryError::DictionaryReadFailed)
        .unwrap_or_default();
    runtime::ret(CLValue::from_t(record.verified).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn get_recipient() {
    let recipient: Key = runtime::get_named_arg(ARG_RECIPIENT);
    let dict_uref = recipients_dict();
    let record: Recipient = storage::dictionary_get(dict_uref, &key_to_dict_key(&recipient))
        .unwrap_or_revert_with(IdentityRegistryError::DictionaryReadFailed)
        .unwrap_or_default();
    runtime::ret(CLValue::from_t(record).unwrap_or_revert());
}

/// Compatibility view -- same idea as `IdentityRegistry.sol`'s own
/// `verifiers(address)` compatibility view over the shared role
/// registry (see that contract's doc comment).
#[no_mangle]
pub extern "C" fn is_verifier() {
    let account: Key = runtime::get_named_arg(ARG_ACCOUNT);
    let dict_uref = verifiers_dict();
    let result: bool = storage::dictionary_get(dict_uref, &key_to_dict_key(&account))
        .unwrap_or_revert_with(IdentityRegistryError::DictionaryReadFailed)
        .unwrap_or(false);
    runtime::ret(CLValue::from_t(result).unwrap_or_revert());
}

// ---------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------

fn only_admin() {
    let caller = Key::from(runtime::get_caller());
    if caller != get_admin() {
        runtime::revert(IdentityRegistryError::CallerIsNotAdmin);
    }
}

fn only_verifier() {
    let caller = Key::from(runtime::get_caller());
    let dict_uref = verifiers_dict();
    let is_verifier: bool = storage::dictionary_get(dict_uref, &key_to_dict_key(&caller))
        .unwrap_or_revert_with(IdentityRegistryError::DictionaryReadFailed)
        .unwrap_or(false);
    if !is_verifier {
        runtime::revert(IdentityRegistryError::CallerIsNotVerifier);
    }
}

fn get_admin() -> Key {
    let uref: URef = runtime::get_key(KEY_ADMIN)
        .unwrap_or_revert_with(IdentityRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(IdentityRegistryError::UnexpectedKeyType);
    storage::read(uref)
        .unwrap_or_revert_with(IdentityRegistryError::MissingKey)
        .unwrap_or_revert_with(IdentityRegistryError::MissingKey)
}

fn set_admin(new_admin: Key) {
    let uref: URef = runtime::get_key(KEY_ADMIN)
        .unwrap_or_revert_with(IdentityRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(IdentityRegistryError::UnexpectedKeyType);
    storage::write(uref, new_admin);
}

fn get_pending_admin() -> Option<Key> {
    let uref: URef = runtime::get_key(KEY_PENDING_ADMIN)
        .unwrap_or_revert_with(IdentityRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(IdentityRegistryError::UnexpectedKeyType);
    // T here is itself `Option<Key>`, so storage::read returns
    // Result<Option<Option<Key>>, Error> -- the outer Option indicates
    // whether the URef holds any value at all (always true after
    // install, since call() writes it), the inner Option is the actual
    // pending-admin value this function returns. Two unwraps, same as
    // get_admin's Result<Option<Key>, Error> needing exactly one fewer
    // (that T is plain Key, not Option<Key>).
    storage::read(uref)
        .unwrap_or_revert_with(IdentityRegistryError::MissingKey)
        .unwrap_or_revert_with(IdentityRegistryError::MissingKey)
}

fn set_pending_admin(pending: Option<Key>) {
    let uref: URef = runtime::get_key(KEY_PENDING_ADMIN)
        .unwrap_or_revert_with(IdentityRegistryError::MissingKey)
        .into_uref()
        .unwrap_or_revert_with(IdentityRegistryError::UnexpectedKeyType);
    storage::write(uref, pending);
}

fn verifiers_dict() -> URef {
    *runtime::get_key(KEY_VERIFIERS_DICT)
        .unwrap_or_revert_with(IdentityRegistryError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(IdentityRegistryError::UnexpectedKeyType)
}

fn recipients_dict() -> URef {
    *runtime::get_key(KEY_RECIPIENTS_DICT)
        .unwrap_or_revert_with(IdentityRegistryError::MissingKey)
        .as_uref()
        .unwrap_or_revert_with(IdentityRegistryError::UnexpectedKeyType)
}

/// See risk-registry/src/main.rs's `key_to_dict_key` for why `Key` is
/// stringified for dictionary lookups -- same reasoning applies here.
fn key_to_dict_key(key: &Key) -> String {
    key.to_string()
}

// ---------------------------------------------------------------
// Installer (`call`)
// ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn call() {
    let initial_verifier: Option<Key> = runtime::get_named_arg(ARG_INITIAL_VERIFIER);

    let mut named_keys = NamedKeys::new();

    let admin_key = Key::from(runtime::get_caller());
    named_keys.insert(KEY_ADMIN.to_string(), storage::new_uref(admin_key).into());
    named_keys.insert(
        KEY_PENDING_ADMIN.to_string(),
        storage::new_uref(Option::<Key>::None).into(),
    );

    let verifiers_dict_uref = storage::new_dictionary(KEY_VERIFIERS_DICT).unwrap_or_revert();
    named_keys.insert(KEY_VERIFIERS_DICT.to_string(), verifiers_dict_uref.into());

    let recipients_dict_uref = storage::new_dictionary(KEY_RECIPIENTS_DICT).unwrap_or_revert();
    named_keys.insert(
        KEY_RECIPIENTS_DICT.to_string(),
        recipients_dict_uref.into(),
    );

    // Admin is always implicitly a verifier at install, matching
    // `IdentityRegistry.sol`'s constructor (`_grantRole(VERIFIER_ROLE,
    // admin_)`) exactly -- not conditional on `initial_verifier`, which
    // is a *separate*, optional additional verifier to grant at
    // install (mirroring risk-registry's own `initial_data_feeder`
    // pattern for a role beyond the installer/admin).
    storage::dictionary_put(verifiers_dict_uref, &key_to_dict_key(&admin_key), true);
    if let Some(verifier) = initial_verifier {
        storage::dictionary_put(verifiers_dict_uref, &key_to_dict_key(&verifier), true);
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
    // schema registration (and, there, the initial-role event) must run
    // via `runtime::call_contract` in the contract's own context rather
    // than directly from `call()`'s body.
    runtime::call_contract::<()>(
        contract_hash,
        ENTRY_POINT_INIT,
        runtime_args! {
            ARG_ACCOUNT => admin_key,
            ARG_INITIAL_VERIFIER => initial_verifier,
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
        .with::<AdminTransferred>()
        .with::<AdminTransferProposed>()
        .with::<VerifierUpdated>()
        .with::<RecipientVerified>()
        .with::<RecipientRevoked>();
    casper_event_standard::init(schemas);

    // Emit the install-time verifier grant(s) from inside the
    // contract's own context, same reasoning as risk-registry's
    // initial-data-feeder event.
    let admin_key: Key = runtime::get_named_arg(ARG_ACCOUNT);
    casper_event_standard::emit(VerifierUpdated {
        account: admin_key,
        is_verifier: true,
    });

    let initial_verifier: Option<Key> = runtime::get_named_arg(ARG_INITIAL_VERIFIER);
    if let Some(verifier) = initial_verifier {
        casper_event_standard::emit(VerifierUpdated {
            account: verifier,
            is_verifier: true,
        });
    }
}

fn build_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_SET_VERIFIER,
            vec![
                Parameter::new(ARG_ACCOUNT, CLType::Key),
                Parameter::new(ARG_IS_VERIFIER, CLType::Bool),
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
            ENTRY_POINT_VERIFY_RECIPIENT,
            vec![
                Parameter::new(ARG_RECIPIENT, CLType::Key),
                Parameter::new(ARG_COMMUNITY, CLType::String),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_REVOKE_RECIPIENT,
            vec![Parameter::new(ARG_RECIPIENT, CLType::Key)],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_IS_VERIFIED,
            vec![Parameter::new(ARG_RECIPIENT, CLType::Key)],
            CLType::Bool,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_GET_RECIPIENT,
            vec![Parameter::new(ARG_RECIPIENT, CLType::Key)],
            CLType::Any, // Recipient -- see model.rs's CLTyped impl.
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points.add_entry_point(
        EntryPoint::new(
            ENTRY_POINT_IS_VERIFIER,
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
            vec![
                Parameter::new(ARG_ACCOUNT, CLType::Key),
                Parameter::new(ARG_INITIAL_VERIFIER, CLType::Option(alloc::boxed::Box::new(CLType::Key))),
            ],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
        )
        .into(),
    );

    entry_points
}
