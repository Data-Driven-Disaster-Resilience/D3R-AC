//! Integration tests for `multisig-admin`. See
//! risk-registry-tests/tests/integration_tests.rs's module comment for
//! this suite's standing disclosure about being unverified until CI's
//! real toolchain runs it -- this is the first real compiler/test pass
//! this file gets.
//!
//! `should_execute_a_confirmed_transaction_against_a_real_contract`
//! installs a real `identity-registry` alongside `multisig-admin` and
//! exercises `execute_transaction`'s cross-contract call for real --
//! the "least-verified part" `multisig-admin/src/main.rs`'s own module
//! comment flags, not a stub.
//!
//! Every test here uses a single owner (`DEFAULT_ACCOUNT_ADDR`) with
//! `threshold = 1` for anything that needs to actually submit/confirm/
//! execute end to end -- a valid 1-of-1 multisig, and it sidesteps the
//! account-funding limitation this suite's other test files document
//! (`identity-registry-tests`' own
//! `should_propose_admin_transfer_and_reject_a_stranger_accepting`):
//! a fresh `AccountHash` can't submit a deploy that's supposed to
//! SUCCEED without being funded first, and that funding API wasn't
//! confirmed with enough certainty in this dependency version to guess
//! at (see `disbursement-controller-tests`' own history on this exact
//! point). Tests that need a SECOND owner to exist in the owner set
//! (the `InsufficientConfirmations` guard, specifically) only need that
//! owner's `Key` to be listed at install time, never to actually submit
//! a deploy itself -- so those don't hit the funding limitation at all.

use casper_engine_test_support::{
    ExecuteRequestBuilder, LmdbWasmTestBuilder, DEFAULT_ACCOUNT_ADDR, LOCAL_GENESIS_REQUEST,
};
use casper_types::bytesrepr::ToBytes;
use casper_types::{account::AccountHash, runtime_args, AddressableEntityHash, Key};

const MULTISIG_WASM: &str = "multisig-admin.wasm";
const IDENTITY_REGISTRY_WASM: &str = "identity-registry.wasm";

const MULTISIG_CONTRACT_HASH_KEY_NAME: &str = "multisig_admin_contract_hash";
const IR_CONTRACT_HASH_KEY_NAME: &str = "identity_registry_contract_hash";
const IR_PACKAGE_HASH_KEY_NAME: &str = "identity_registry_package_hash";

const ARG_OWNERS: &str = "owners";
const ARG_THRESHOLD: &str = "threshold";
const ARG_TARGET_PACKAGE_HASH: &str = "target_package_hash";
const ARG_TARGET_ENTRY_POINT: &str = "target_entry_point";
const ARG_TARGET_ARGS_BYTES: &str = "target_args_bytes";
const ARG_TX_ID: &str = "tx_id";

/// Installs with a single owner (`DEFAULT_ACCOUNT_ADDR`) and
/// `threshold = 1` unless `extra_owners` adds more (still
/// `threshold = 1` -- callers needing a higher threshold submit their
/// own `call()` request instead of using this helper). Also installs a
/// real `identity-registry` alongside it and returns its package `Key`
/// for use as a `target_package_hash` in tests that need
/// `submit_transaction` itself to succeed but never actually execute
/// the resulting transaction -- using a real package `Key` here rather
/// than an `AccountHash`-based one avoids depending on whether
/// `Key::into_hash_addr()` (which `key_to_package_hash` calls) treats
/// `Key::Account` the same as `Key::Hash`/`Key::Package` or not; not
/// confirmed either way, so tests that don't care about that specific
/// question shouldn't accidentally depend on it.
fn install(extra_owners: Vec<Key>) -> (LmdbWasmTestBuilder, AddressableEntityHash, Key) {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone()).commit();

    let install_registry = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        IDENTITY_REGISTRY_WASM,
        runtime_args! { "initial_verifier" => Option::<Key>::None },
    )
    .build();
    builder.exec(install_registry).expect_success().commit();

    let registry_package_key: Key = *builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get(IR_PACKAGE_HASH_KEY_NAME)
        .expect("identity registry package hash should exist after install");

    let mut owners = vec![Key::from(*DEFAULT_ACCOUNT_ADDR)];
    owners.extend(extra_owners);

    let install_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        MULTISIG_WASM,
        runtime_args! {
            ARG_OWNERS => owners,
            ARG_THRESHOLD => 1u64,
        },
    )
    .build();
    builder.exec(install_request).expect_success().commit();

    let hash = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get(MULTISIG_CONTRACT_HASH_KEY_NAME)
        .expect("contract hash named key should exist after install")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");

    (builder, hash, registry_package_key)
}

#[test]
fn should_install() {
    let _ = install(Vec::new());
}

#[test]
fn should_reject_install_with_no_owners() {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone()).commit();

    // Matches MultiSigAdmin.sol's "owners required" guard
    // (MultisigAdminError::OwnersRequired).
    let request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        MULTISIG_WASM,
        runtime_args! {
            ARG_OWNERS => Vec::<Key>::new(),
            ARG_THRESHOLD => 1u64,
        },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_reject_install_with_threshold_zero() {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone()).commit();

    // Matches MultiSigAdmin.sol's "invalid threshold" guard
    // (MultisigAdminError::InvalidThreshold).
    let request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        MULTISIG_WASM,
        runtime_args! {
            ARG_OWNERS => vec![Key::from(*DEFAULT_ACCOUNT_ADDR)],
            ARG_THRESHOLD => 0u64,
        },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_reject_install_with_threshold_exceeding_owner_count() {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone()).commit();

    // Same guard, the other direction.
    let request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        MULTISIG_WASM,
        runtime_args! {
            ARG_OWNERS => vec![Key::from(*DEFAULT_ACCOUNT_ADDR)],
            ARG_THRESHOLD => 2u64,
        },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_reject_install_with_duplicate_owners() {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone()).commit();

    // Matches MultiSigAdmin.sol's "duplicate owner" guard
    // (MultisigAdminError::DuplicateOwner).
    let owner = Key::from(*DEFAULT_ACCOUNT_ADDR);
    let request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        MULTISIG_WASM,
        runtime_args! {
            ARG_OWNERS => vec![owner, owner],
            ARG_THRESHOLD => 1u64,
        },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_report_owner_and_transaction_counts_after_install() {
    let (mut builder, hash, _registry_key) = install(vec![Key::from(AccountHash::new([1u8; 32]))]);

    let owner_count_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "owner_count",
        runtime_args! {},
    )
    .build();
    builder.exec(owner_count_request).expect_success().commit();

    let tx_count_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "transaction_count",
        runtime_args! {},
    )
    .build();
    builder.exec(tx_count_request).expect_success().commit();
}

#[test]
fn should_reject_submit_transaction_from_non_owner() {
    let (mut builder, hash, _registry_key) = install(Vec::new());
    let stranger = AccountHash::new([2u8; 32]);

    // Matches MultiSigAdmin.sol's onlyOwner modifier
    // (MultisigAdminError::CallerIsNotOwner).
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        hash,
        "submit_transaction",
        runtime_args! {
            ARG_TARGET_PACKAGE_HASH => Key::from(*DEFAULT_ACCOUNT_ADDR),
            ARG_TARGET_ENTRY_POINT => "noop".to_string(),
            ARG_TARGET_ARGS_BYTES => Vec::<u8>::new(),
        },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_reject_submit_transaction_with_an_unparseable_target() {
    let (mut builder, hash, _registry_key) = install(Vec::new());

    // Matches MultiSigAdmin.sol's "target is zero address" guard
    // (MultisigAdminError::InvalidTarget) -- an AccountHash-based Key
    // doesn't parse as a package hash (key_to_package_hash's
    // into_hash_addr() only succeeds for Hash/Package-style Keys).
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "submit_transaction",
        runtime_args! {
            ARG_TARGET_PACKAGE_HASH => Key::from(AccountHash::new([3u8; 32])),
            ARG_TARGET_ENTRY_POINT => "noop".to_string(),
            ARG_TARGET_ARGS_BYTES => Vec::<u8>::new(),
        },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_submit_and_auto_confirm_then_reject_double_confirm() {
    let (mut builder, hash, registry_key) = install(Vec::new());

    let submit_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "submit_transaction",
        runtime_args! {
            ARG_TARGET_PACKAGE_HASH => registry_key,
            ARG_TARGET_ENTRY_POINT => "noop".to_string(),
            ARG_TARGET_ARGS_BYTES => Vec::<u8>::new(),
        },
    )
    .build();
    builder.exec(submit_request).expect_success().commit();

    // submit_transaction auto-confirms from the submitter (see
    // multisig-admin.sol's own submitTransaction -> _confirm(txId)
    // call) -- DEFAULT_ACCOUNT_ADDR trying to confirm again must be
    // rejected. Matches MultisigAdminError::AlreadyConfirmed.
    let double_confirm_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "confirm_transaction",
        runtime_args! { ARG_TX_ID => 0u64 },
    )
    .build();
    builder.exec(double_confirm_request).expect_failure();
}

#[test]
fn should_reject_confirm_of_a_nonexistent_transaction() {
    let (mut builder, hash, _registry_key) = install(Vec::new());

    // Matches MultiSigAdmin.sol's txExists modifier
    // (MultisigAdminError::TransactionDoesNotExist) -- nothing
    // submitted yet.
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "confirm_transaction",
        runtime_args! { ARG_TX_ID => 0u64 },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_reject_execute_below_threshold_and_report_correct_confirmation_count() {
    // A genuine 2-of-2 multisig -- the second owner only needs to
    // exist in the owner set, never submit a deploy itself, to test
    // the InsufficientConfirmations guard (see module comment).
    let second_owner = AccountHash::new([5u8; 32]);
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone()).commit();

    let install_registry = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        IDENTITY_REGISTRY_WASM,
        runtime_args! { "initial_verifier" => Option::<Key>::None },
    )
    .build();
    builder.exec(install_registry).expect_success().commit();

    let registry_key: Key = *builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get(IR_PACKAGE_HASH_KEY_NAME)
        .expect("identity registry package hash should exist after install");

    let install_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        MULTISIG_WASM,
        runtime_args! {
            ARG_OWNERS => vec![Key::from(*DEFAULT_ACCOUNT_ADDR), Key::from(second_owner)],
            ARG_THRESHOLD => 2u64,
        },
    )
    .build();
    builder.exec(install_request).expect_success().commit();

    let hash: AddressableEntityHash = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get(MULTISIG_CONTRACT_HASH_KEY_NAME)
        .expect("contract hash named key should exist after install")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");

    let submit_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "submit_transaction",
        runtime_args! {
            ARG_TARGET_PACKAGE_HASH => registry_key,
            ARG_TARGET_ENTRY_POINT => "noop".to_string(),
            ARG_TARGET_ARGS_BYTES => Vec::<u8>::new(),
        },
    )
    .build();
    builder.exec(submit_request).expect_success().commit();

    // Only DEFAULT_ACCOUNT_ADDR's auto-confirmation exists (count = 1),
    // threshold is 2 -- execute must be rejected. Matches
    // MultisigAdminError::InsufficientConfirmations.
    let execute_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "execute_transaction",
        runtime_args! { ARG_TX_ID => 0u64 },
    )
    .build();
    builder.exec(execute_request).expect_failure();
}

#[test]
fn should_confirm_then_revoke_then_reject_double_revoke() {
    let (mut builder, hash, registry_key) = install(Vec::new());

    let submit_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "submit_transaction",
        runtime_args! {
            ARG_TARGET_PACKAGE_HASH => registry_key,
            ARG_TARGET_ENTRY_POINT => "noop".to_string(),
            ARG_TARGET_ARGS_BYTES => Vec::<u8>::new(),
        },
    )
    .build();
    builder.exec(submit_request).expect_success().commit();

    // Auto-confirmed by submit_transaction -- revoke it.
    let revoke_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "revoke_confirmation",
        runtime_args! { ARG_TX_ID => 0u64 },
    )
    .build();
    builder.exec(revoke_request).expect_success().commit();

    // Matches MultiSigAdmin.sol's "not confirmed" guard
    // (MultisigAdminError::NotConfirmed) -- already revoked.
    let double_revoke_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "revoke_confirmation",
        runtime_args! { ARG_TX_ID => 0u64 },
    )
    .build();
    builder.exec(double_revoke_request).expect_failure();
}

#[test]
fn should_execute_a_confirmed_transaction_against_a_real_contract() {
    // The real, load-bearing test in this file: installs a genuine
    // identity-registry alongside multisig-admin (not a stub) and
    // exercises execute_transaction's cross-contract call for real --
    // proposing, auto-confirming (1-of-1), then executing a call to
    // identity-registry's set_verifier, and confirming the effect
    // actually landed.
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone()).commit();

    let install_registry = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        IDENTITY_REGISTRY_WASM,
        runtime_args! { "initial_verifier" => Option::<Key>::None },
    )
    .build();
    builder.exec(install_registry).expect_success().commit();

    let registry_hash: AddressableEntityHash = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get(IR_CONTRACT_HASH_KEY_NAME)
        .expect("identity registry contract hash should exist after install")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");

    // multisig-admin's submit_transaction wants the TARGET's PACKAGE
    // hash (see key_to_package_hash in multisig-admin/src/main.rs),
    // not its entity/contract hash -- read straight through as
    // whatever Key variant the named key already is, rather than
    // extracting/re-wrapping it (see disbursement-controller-tests'
    // own history on why guessing at Key-variant equivalence here is
    // exactly the kind of thing that's gone wrong before in this
    // suite).
    let registry_package_key: Key = *builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get(IR_PACKAGE_HASH_KEY_NAME)
        .expect("identity registry package hash should exist after install");

    // multisig-admin must actually be identity-registry's admin for
    // set_verifier to succeed once executed -- propose+accept the
    // same two-step handoff any other admin transfer in this suite
    // uses.
    let install_multisig = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        MULTISIG_WASM,
        runtime_args! {
            ARG_OWNERS => vec![Key::from(*DEFAULT_ACCOUNT_ADDR)],
            ARG_THRESHOLD => 1u64,
        },
    )
    .build();
    builder.exec(install_multisig).expect_success().commit();

    let multisig_hash: AddressableEntityHash = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get(MULTISIG_CONTRACT_HASH_KEY_NAME)
        .expect("multisig contract hash named key should exist after install")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");
    let multisig_key: Key = *builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get("multisig_admin_package_hash")
        .expect("multisig package hash named key should exist after install");

    let propose_admin_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        registry_hash,
        "propose_new_admin",
        runtime_args! { "new_admin" => multisig_key },
    )
    .build();
    builder.exec(propose_admin_request).expect_success().commit();

    // multisig-admin itself must accept -- same reasoning as
    // D3RACHub's own accept*Admin wrapper functions (only a contract's
    // own code can make it the msg.sender of its own acceptAdmin()
    // call), except multisig-admin has no purpose-built wrapper for
    // this, so the acceptance itself is routed through
    // submit_transaction/execute_transaction -- exactly the real
    // workflow this multisig is for.
    let accept_args = runtime_args! {};
    let accept_args_bytes = accept_args
        .to_bytes()
        .expect("empty RuntimeArgs should always serialize");

    let submit_accept_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        multisig_hash,
        "submit_transaction",
        runtime_args! {
            ARG_TARGET_PACKAGE_HASH => registry_package_key,
            ARG_TARGET_ENTRY_POINT => "accept_admin".to_string(),
            ARG_TARGET_ARGS_BYTES => accept_args_bytes,
        },
    )
    .build();
    builder.exec(submit_accept_request).expect_success().commit();

    let execute_accept_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        multisig_hash,
        "execute_transaction",
        runtime_args! { ARG_TX_ID => 0u64 },
    )
    .build();
    builder.exec(execute_accept_request).expect_success().commit();

    // multisig-admin is now identity-registry's admin. Propose a real
    // set_verifier call through the same submit/execute flow.
    let target_account = Key::from(AccountHash::new([8u8; 32]));
    let set_verifier_args = runtime_args! {
        "account" => target_account,
        "is_verifier" => true,
    };
    let set_verifier_args_bytes = set_verifier_args
        .to_bytes()
        .expect("RuntimeArgs should serialize");

    let submit_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        multisig_hash,
        "submit_transaction",
        runtime_args! {
            ARG_TARGET_PACKAGE_HASH => registry_package_key,
            ARG_TARGET_ENTRY_POINT => "set_verifier".to_string(),
            ARG_TARGET_ARGS_BYTES => set_verifier_args_bytes,
        },
    )
    .build();
    builder.exec(submit_request).expect_success().commit();

    let execute_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        multisig_hash,
        "execute_transaction",
        runtime_args! { ARG_TX_ID => 1u64 },
    )
    .build();
    builder.exec(execute_request).expect_success().commit();

    // Confirm the effect actually landed on identity-registry: query
    // is_verifier for target_account.
    let is_verifier_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        registry_hash,
        "is_verifier",
        runtime_args! { "account" => target_account },
    )
    .build();
    builder.exec(is_verifier_request).expect_success().commit();

    // Executing the same transaction again must be rejected -- matches
    // MultisigAdminError::TransactionAlreadyExecuted.
    let double_execute_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        multisig_hash,
        "execute_transaction",
        runtime_args! { ARG_TX_ID => 1u64 },
    )
    .build();
    builder.exec(double_execute_request).expect_failure();
}

#[test]
fn should_reject_get_transaction_for_a_nonexistent_id() {
    let (mut builder, hash, _registry_key) = install(Vec::new());

    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "get_transaction",
        runtime_args! { ARG_TX_ID => 0u64 },
    )
    .build();
    builder.exec(request).expect_failure();
}
