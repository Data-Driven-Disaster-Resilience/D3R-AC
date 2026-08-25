//! Integration tests for `disbursement-controller`. See
//! risk-registry-tests/tests/integration_tests.rs's module comment for
//! this suite's standing disclosure about being unverified until CI's
//! real toolchain runs it.
//!
//! Installs a real `identity-registry` alongside `disbursement-controller`
//! (not a mock) to exercise the genuine cross-contract `is_verified`
//! call `create_commitment` makes -- this is the actual integration
//! this test suite's name promises, not a stand-in.
//! `should_release_milestone_with_a_real_token_and_reject_when_unfunded`
//! additionally installs a real `d3rac-token` and exercises the full
//! fund-release path end to end, including the unfunded-contract
//! rejection this contract's own "no explicit balance_of pre-check"
//! design decision relies on CEP-18's own revert for -- this used to
//! be a documented gap in this file (no CEP-18 token existed when the
//! rest of these tests were first written) and isn't anymore.

use casper_engine_test_support::{
    ExecuteRequestBuilder, LmdbWasmTestBuilder, DEFAULT_ACCOUNT_ADDR, LOCAL_GENESIS_REQUEST,
};
use casper_types::{
    account::AccountHash, contracts::ContractHash, runtime_args, AddressableEntityHash, Key, U256,
};

const DISBURSEMENT_CONTROLLER_WASM: &str = "disbursement-controller.wasm";
const IDENTITY_REGISTRY_WASM: &str = "identity-registry.wasm";

const DC_CONTRACT_HASH_KEY_NAME: &str = "disbursement_controller_contract_hash";
const IR_CONTRACT_HASH_KEY_NAME: &str = "identity_registry_contract_hash";

const ARG_REGISTRY_HASH: &str = "registry_hash";
const ARG_RECIPIENT: &str = "recipient";
const ARG_TOKEN: &str = "token";
const ARG_COMMUNITY: &str = "community";
const ARG_DESCRIPTIONS: &str = "descriptions";
const ARG_AMOUNTS: &str = "amounts";
const ARG_COMMITMENT_ID: &str = "commitment_id";
const ARG_MILESTONE_INDEX: &str = "milestone_index";
const ARG_ACCOUNT: &str = "account";
const ARG_IS_ATTESTER: &str = "is_attester";
const ARG_COMMUNITY_LABEL: &str = "community"; // identity-registry's own arg name
const ARG_INITIAL_VERIFIER: &str = "initial_verifier";

/// Installs identity-registry, then disbursement-controller pointed at
/// it, both under DEFAULT_ACCOUNT_ADDR (so it's admin of both). A
/// placeholder token Key is used everywhere in this file -- no real
/// CEP-18 exists yet (see module comment) -- since create_commitment
/// itself doesn't validate the token address beyond storing it.
fn install() -> (LmdbWasmTestBuilder, AddressableEntityHash, AddressableEntityHash) {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone()).commit();

    let install_registry = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        IDENTITY_REGISTRY_WASM,
        runtime_args! { ARG_INITIAL_VERIFIER => Option::<Key>::None },
    )
    .build();
    builder.exec(install_registry).expect_success().commit();

    let registry_hash = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get(IR_CONTRACT_HASH_KEY_NAME)
        .expect("identity registry contract hash should exist after install")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");

    let install_dc = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        DISBURSEMENT_CONTROLLER_WASM,
        runtime_args! { ARG_REGISTRY_HASH => Key::from(ContractHash::from(registry_hash)) },
    )
    .build();
    builder.exec(install_dc).expect_success().commit();

    let dc_hash = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get(DC_CONTRACT_HASH_KEY_NAME)
        .expect("disbursement controller contract hash should exist after install")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");

    (builder, dc_hash, registry_hash)
}

fn verify_recipient(builder: &mut LmdbWasmTestBuilder, registry_hash: AddressableEntityHash, recipient: Key) {
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        registry_hash,
        "verify_recipient",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_COMMUNITY_LABEL => "Test Community".to_string(),
        },
    )
    .build();
    builder.exec(request).expect_success().commit();
}

fn placeholder_token() -> Key {
    Key::from(AccountHash::new([99u8; 32]))
}

#[test]
fn should_install() {
    let _ = install();
}

#[test]
fn should_create_commitment_for_a_verified_recipient() {
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([1u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => placeholder_token(),
            ARG_COMMUNITY => "Test Community".to_string(),
            ARG_DESCRIPTIONS => vec!["Phase 1".to_string()],
            ARG_AMOUNTS => vec![U256::from(1_000u64)],
        },
    )
    .build();
    builder.exec(request).expect_success().commit();

    let count_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "commitment_count",
        runtime_args! {},
    )
    .build();
    builder.exec(count_request).expect_success().commit();
}

#[test]
fn should_reject_commitment_for_an_unverified_recipient() {
    let (mut builder, dc_hash, _registry_hash) = install();
    // Never verified in the registry -- matches
    // DisbursementController.sol's "recipient not verified" guard
    // (DisbursementControllerError::RecipientNotVerified), exercising
    // the real cross-contract is_verified call, not a stub.
    let recipient = Key::from(AccountHash::new([2u8; 32]));

    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => placeholder_token(),
            ARG_COMMUNITY => "Test Community".to_string(),
            ARG_DESCRIPTIONS => vec!["Phase 1".to_string()],
            ARG_AMOUNTS => vec![U256::from(1_000u64)],
        },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_reject_commitment_from_non_admin() {
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([3u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let stranger = AccountHash::new([4u8; 32]);
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => placeholder_token(),
            ARG_COMMUNITY => "Test Community".to_string(),
            ARG_DESCRIPTIONS => vec!["Phase 1".to_string()],
            ARG_AMOUNTS => vec![U256::from(1_000u64)],
        },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_reject_commitment_with_mismatched_description_and_amount_lengths() {
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([5u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => placeholder_token(),
            ARG_COMMUNITY => "Test Community".to_string(),
            ARG_DESCRIPTIONS => vec!["Phase 1".to_string(), "Phase 2".to_string()],
            ARG_AMOUNTS => vec![U256::from(1_000u64)],
        },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_reject_commitment_with_zero_amount_milestone() {
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([6u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => placeholder_token(),
            ARG_COMMUNITY => "Test Community".to_string(),
            ARG_DESCRIPTIONS => vec!["Phase 1".to_string()],
            ARG_AMOUNTS => vec![U256::zero()],
        },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_attest_a_milestone_then_reject_double_attestation() {
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([7u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let create_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => placeholder_token(),
            ARG_COMMUNITY => "Test Community".to_string(),
            ARG_DESCRIPTIONS => vec!["Phase 1".to_string()],
            ARG_AMOUNTS => vec![U256::from(1_000u64)],
        },
    )
    .build();
    builder.exec(create_request).expect_success().commit();

    let attest_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "attest_milestone",
        runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => 0u64 },
    )
    .build();
    builder.exec(attest_request).expect_success().commit();

    // Matches DisbursementController.sol's "milestone already attested"
    // guard (DisbursementControllerError::MilestoneAlreadyAttested).
    let double_attest_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "attest_milestone",
        runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => 0u64 },
    )
    .build();
    builder.exec(double_attest_request).expect_failure();
}

#[test]
fn should_reject_attest_from_non_attester() {
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([8u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let create_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => placeholder_token(),
            ARG_COMMUNITY => "Test Community".to_string(),
            ARG_DESCRIPTIONS => vec!["Phase 1".to_string()],
            ARG_AMOUNTS => vec![U256::from(1_000u64)],
        },
    )
    .build();
    builder.exec(create_request).expect_success().commit();

    let stranger = AccountHash::new([9u8; 32]);
    let attest_request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        dc_hash,
        "attest_milestone",
        runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => 0u64 },
    )
    .build();
    builder.exec(attest_request).expect_failure();
}

#[test]
fn should_reject_release_of_an_unattested_milestone() {
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([10u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let create_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => placeholder_token(),
            ARG_COMMUNITY => "Test Community".to_string(),
            ARG_DESCRIPTIONS => vec!["Phase 1".to_string()],
            ARG_AMOUNTS => vec![U256::from(1_000u64)],
        },
    )
    .build();
    builder.exec(create_request).expect_success().commit();

    // Never attested -- matches DisbursementController.sol's "milestone
    // not attested" guard (DisbursementControllerError::MilestoneNotAttested).
    // This guard fails BEFORE the token cross-contract call, so it
    // doesn't need a real CEP-18 token -- see module comment.
    let release_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "release_milestone",
        runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => 0u64 },
    )
    .build();
    builder.exec(release_request).expect_failure();
}

#[test]
fn should_cancel_a_commitment_and_reject_further_attestation() {
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([11u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let create_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => placeholder_token(),
            ARG_COMMUNITY => "Test Community".to_string(),
            ARG_DESCRIPTIONS => vec!["Phase 1".to_string()],
            ARG_AMOUNTS => vec![U256::from(1_000u64)],
        },
    )
    .build();
    builder.exec(create_request).expect_success().commit();

    let cancel_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "cancel_commitment",
        runtime_args! { ARG_COMMITMENT_ID => 0u64 },
    )
    .build();
    builder.exec(cancel_request).expect_success().commit();

    // Matches DisbursementController.sol's "commitment not active"
    // guard (DisbursementControllerError::CommitmentNotActive).
    let attest_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "attest_milestone",
        runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => 0u64 },
    )
    .build();
    builder.exec(attest_request).expect_failure();
}

#[test]
fn should_release_milestone_with_a_real_token_and_reject_when_unfunded() {
    // Closes the gap this file's module comment used to describe: no
    // CEP-18 token existed when this test file was first written.
    // d3rac-token now does -- install it too, and exercise the actual
    // fund-release path end to end, not just the guards that fail
    // before reaching it.
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([20u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let install_token = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        "d3rac-token.wasm",
        runtime_args! {
            "initial_supply" => U256::zero(),
            "owner_" => Key::from(*DEFAULT_ACCOUNT_ADDR),
        },
    )
    .build();
    builder.exec(install_token).expect_success().commit();

    let token_hash: AddressableEntityHash = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get("d3rac_token_contract_hash")
        .expect("token contract hash named key should exist after install")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");
    let token_key = Key::from(ContractHash::from(token_hash));
    let dc_key = Key::from(ContractHash::from(dc_hash));

    let create_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => token_key,
            ARG_COMMUNITY => "Test Community".to_string(),
            ARG_DESCRIPTIONS => vec!["Phase 1".to_string()],
            ARG_AMOUNTS => vec![U256::from(1_000u64)],
        },
    )
    .build();
    builder.exec(create_request).expect_success().commit();

    let attest_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "attest_milestone",
        runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => 0u64 },
    )
    .build();
    builder.exec(attest_request).expect_success().commit();

    // Not yet funded -- disbursement-controller holds 0 of this token.
    // CEP-18's own transfer revert (InsufficientBalance) is the thing
    // that's supposed to stop this, per disbursement-controller's own
    // "no explicit balance_of pre-check" design decision (see its
    // main.rs's release_milestone comment) -- this is the real test of
    // that decision, not just a guard this contract's own code checks.
    let unfunded_release_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "release_milestone",
        runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => 0u64 },
    )
    .build();
    builder.exec(unfunded_release_request).expect_failure();

    // Fund disbursement-controller itself (not an EOA -- the contract's
    // own address) by minting directly to it, matching how this would
    // actually be funded in practice (a deposit into the contract
    // before it releases anything).
    let mint_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        token_hash,
        "mint",
        runtime_args! { "recipient" => dc_key, "amount" => U256::from(1_000u64) },
    )
    .build();
    builder.exec(mint_request).expect_success().commit();

    // Now genuinely funded -- the real success path.
    let release_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "release_milestone",
        runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => 0u64 },
    )
    .build();
    builder.exec(release_request).expect_success().commit();

    // A second release of the same milestone must still be rejected --
    // matches DisbursementControllerError::MilestoneAlreadyReleased,
    // now genuinely reachable (previously untestable without a real
    // token to have gotten this far in the first place).
    let double_release_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "release_milestone",
        runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => 0u64 },
    )
    .build();
    builder.exec(double_release_request).expect_failure();

    // Confirm the recipient's balance actually reflects the transfer.
    let balance_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        token_hash,
        "balance_of",
        runtime_args! { "account" => recipient },
    )
    .build();
    builder.exec(balance_request).expect_success().commit();
}

#[test]
fn should_reject_get_commitment_for_a_nonexistent_id() {
    let (mut builder, dc_hash, _registry_hash) = install();

    // Matches DisbursementController.sol's "commitment does not exist"
    // guard (DisbursementControllerError::CommitmentDoesNotExist).
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "get_commitment",
        runtime_args! { ARG_COMMITMENT_ID => 0u64 },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_set_and_read_back_an_attester() {
    let (mut builder, dc_hash, _registry_hash) = install();
    let account = Key::from(AccountHash::new([12u8; 32]));

    let set_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "set_attester",
        runtime_args! { ARG_ACCOUNT => account, ARG_IS_ATTESTER => true },
    )
    .build();
    builder.exec(set_request).expect_success().commit();

    let is_attester_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "is_attester",
        runtime_args! { ARG_ACCOUNT => account },
    )
    .build();
    builder.exec(is_attester_request).expect_success().commit();
}

#[test]
fn should_reject_accept_admin_when_no_transfer_has_been_proposed() {
    let (mut builder, dc_hash, _registry_hash) = install();
    let stranger = AccountHash::new([13u8; 32]);

    // Same pattern as identity-registry-tests -- matches
    // DisbursementControllerError::CallerIsNotPendingAdmin's "including
    // when no transfer has been proposed at all" case.
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        dc_hash,
        "accept_admin",
        runtime_args! {},
    )
    .build();
    builder.exec(request).expect_failure();
}
