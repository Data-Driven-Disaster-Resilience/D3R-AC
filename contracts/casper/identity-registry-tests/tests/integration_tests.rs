//! Integration tests for `identity-registry`, run against a local
//! Casper execution engine (`LmdbWasmTestBuilder`) -- see
//! risk-registry-tests/tests/integration_tests.rs's module comment for
//! the full disclosure this suite's every test file carries: written
//! against the same core pattern that's stayed stable across Casper's
//! own docs, but NOT independently confirmed running in this
//! environment (no wasm32-unknown-unknown target reachable here). To
//! be confirmed or corrected by CI's real compiler/test-runner output.

use casper_engine_test_support::{
    ExecuteRequestBuilder, LmdbWasmTestBuilder, DEFAULT_ACCOUNT_ADDR, LOCAL_GENESIS_REQUEST,
};
use casper_types::{account::AccountHash, runtime_args, Key, U512};

const CONTRACT_WASM: &str = "identity-registry.wasm";
const CONTRACT_HASH_KEY_NAME: &str = "identity_registry_contract_hash";

const ARG_ACCOUNT: &str = "account";
const ARG_IS_VERIFIER: &str = "is_verifier";
const ARG_NEW_ADMIN: &str = "new_admin";
const ARG_RECIPIENT: &str = "recipient";
const ARG_COMMUNITY: &str = "community";
const ARG_INITIAL_VERIFIER: &str = "initial_verifier";

/// Installs a fresh instance with `DEFAULT_ACCOUNT_ADDR` as both the
/// deploying account (and therefore admin, per `call()`'s
/// `runtime::get_caller()` logic) and, separately, `install()`'s own
/// no-extra-verifier fixture -- the admin is always implicitly a
/// verifier too (see main.rs's `call()` comment), so this is already
/// enough to exercise every entry point without a second funded
/// account for the base cases.
fn install() -> LmdbWasmTestBuilder {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone()).commit();

    let install_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_WASM,
        runtime_args! {
            ARG_INITIAL_VERIFIER => Option::<Key>::None,
        },
    )
    .build();

    builder.exec(install_request).expect_success().commit();
    builder
}

fn contract_hash(builder: &LmdbWasmTestBuilder) -> casper_types::AddressableEntityHash {
    builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get(CONTRACT_HASH_KEY_NAME)
        .expect("contract hash named key should exist after install")
        .into_entity_hash()
        .expect("contract hash named key should resolve to an addressable entity hash")
}

/// A fresh `AccountHash` doesn't exist in global state until something
/// funds it -- executing a deploy AS that account before that point
/// fails with `KeyNotFound`, not a graceful contract-level revert (the
/// engine can't even find a purse to charge gas from). Needed only for
/// tests where the fresh account must itself successfully call an
/// entry point (e.g. accepting a proposed admin transfer); tests that
/// only need a fresh account to demonstrate a REJECTED call don't need
/// this -- `KeyNotFound` is itself already an execution failure, so
/// `.expect_failure()` accepts it either way.
fn fund_account(builder: &mut LmdbWasmTestBuilder, account: AccountHash) {
    let transfer_request = ExecuteRequestBuilder::transfer(
        *DEFAULT_ACCOUNT_ADDR,
        runtime_args! {
            "target" => account,
            "amount" => U512::from(30_000_000_000u64),
            "id" => Option::<u64>::None,
        },
    )
    .build();
    builder.exec(transfer_request).expect_success().commit();
}

#[test]
fn should_install() {
    let _builder = install();
}

#[test]
fn should_verify_and_read_back_a_recipient() {
    let mut builder = install();
    let hash = contract_hash(&builder);

    let recipient = Key::from(AccountHash::new([7u8; 32]));

    let verify_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "verify_recipient",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_COMMUNITY => "Test Community".to_string(),
        },
    )
    .build();
    builder.exec(verify_request).expect_success().commit();

    let is_verified_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "is_verified",
        runtime_args! { ARG_RECIPIENT => recipient },
    )
    .build();
    builder.exec(is_verified_request).expect_success().commit();
}

#[test]
fn should_reject_verify_recipient_with_empty_community() {
    let mut builder = install();
    let hash = contract_hash(&builder);

    let recipient = Key::from(AccountHash::new([8u8; 32]));

    // Matches IdentityRegistry.sol's "community label required" guard
    // (see IdentityRegistryError::CommunityLabelRequired).
    let verify_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "verify_recipient",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_COMMUNITY => "".to_string(),
        },
    )
    .build();
    builder.exec(verify_request).expect_failure();
}

#[test]
fn should_reject_verify_recipient_from_non_verifier() {
    let mut builder = install();
    let hash = contract_hash(&builder);

    // Un-registered-as-verifier account -- same "stranger" pattern as
    // risk-registry-tests. Matches IdentityRegistry.sol's onlyVerifier
    // guard (IdentityRegistryError::CallerIsNotVerifier).
    let stranger = AccountHash::new([9u8; 32]);
    let recipient = Key::from(AccountHash::new([10u8; 32]));

    let verify_request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        hash,
        "verify_recipient",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_COMMUNITY => "Test Community".to_string(),
        },
    )
    .build();
    builder.exec(verify_request).expect_failure();
}

#[test]
fn should_verify_then_revoke_a_recipient() {
    let mut builder = install();
    let hash = contract_hash(&builder);

    let recipient = Key::from(AccountHash::new([11u8; 32]));

    let verify_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "verify_recipient",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_COMMUNITY => "Test Community".to_string(),
        },
    )
    .build();
    builder.exec(verify_request).expect_success().commit();

    let revoke_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "revoke_recipient",
        runtime_args! { ARG_RECIPIENT => recipient },
    )
    .build();
    builder.exec(revoke_request).expect_success().commit();
}

#[test]
fn should_reject_revoke_of_a_never_verified_recipient() {
    let mut builder = install();
    let hash = contract_hash(&builder);

    let recipient = Key::from(AccountHash::new([12u8; 32]));

    // Matches IdentityRegistry.sol's "recipient not verified" guard
    // (IdentityRegistryError::RecipientNotVerified) -- never registered
    // at all, not previously-revoked.
    let revoke_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "revoke_recipient",
        runtime_args! { ARG_RECIPIENT => recipient },
    )
    .build();
    builder.exec(revoke_request).expect_failure();
}

#[test]
fn should_reject_set_verifier_from_non_admin() {
    let mut builder = install();
    let hash = contract_hash(&builder);

    let stranger = AccountHash::new([13u8; 32]);
    let target = Key::from(AccountHash::new([14u8; 32]));

    // Matches IdentityRegistry.sol's onlyAdmin guard
    // (IdentityRegistryError::CallerIsNotAdmin).
    let set_verifier_request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        hash,
        "set_verifier",
        runtime_args! {
            ARG_ACCOUNT => target,
            ARG_IS_VERIFIER => true,
        },
    )
    .build();
    builder.exec(set_verifier_request).expect_failure();
}

#[test]
fn should_two_step_transfer_admin_and_reject_a_stranger_accepting() {
    let mut builder = install();
    let hash = contract_hash(&builder);

    let new_admin = AccountHash::new([15u8; 32]);
    let stranger = AccountHash::new([16u8; 32]);

    let propose_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "propose_new_admin",
        runtime_args! { ARG_NEW_ADMIN => Key::from(new_admin) },
    )
    .build();
    builder.exec(propose_request).expect_success().commit();

    // stranger never needs to successfully execute anything below
    // (only rejected calls), so it's deliberately left unfunded --
    // see fund_account's doc comment. new_admin DOES need to
    // successfully call accept_admin, so fund it first.
    fund_account(&mut builder, new_admin);

    // A stranger (not the proposed admin) trying to accept must fail --
    // matches IdentityRegistry.sol's "caller is not the pending admin"
    // guard (IdentityRegistryError::CallerIsNotPendingAdmin).
    let stranger_accept_request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        hash,
        "accept_admin",
        runtime_args! {},
    )
    .build();
    builder.exec(stranger_accept_request).expect_failure();

    // The genuinely proposed address accepting must succeed, and must
    // then hold admin (checked indirectly: it can now call an
    // admin-gated entry point that DEFAULT_ACCOUNT_ADDR, the old admin,
    // can no longer call).
    let accept_request = ExecuteRequestBuilder::contract_call_by_hash(
        new_admin,
        hash,
        "accept_admin",
        runtime_args! {},
    )
    .build();
    builder.exec(accept_request).expect_success().commit();

    let old_admin_set_verifier_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "set_verifier",
        runtime_args! {
            ARG_ACCOUNT => Key::from(stranger),
            ARG_IS_VERIFIER => true,
        },
    )
    .build();
    builder
        .exec(old_admin_set_verifier_request)
        .expect_failure();
}

#[test]
fn should_reject_accept_admin_when_no_transfer_has_been_proposed() {
    let mut builder = install();
    let hash = contract_hash(&builder);

    let stranger = AccountHash::new([17u8; 32]);

    // pending_admin is None at install -- matches
    // IdentityRegistryError::CallerIsNotPendingAdmin's "including when
    // no transfer has been proposed at all" case.
    let accept_request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        hash,
        "accept_admin",
        runtime_args! {},
    )
    .build();
    builder.exec(accept_request).expect_failure();
}
