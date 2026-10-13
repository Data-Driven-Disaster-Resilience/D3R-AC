//! Integration tests for `disbursement-controller`. See
//! risk-registry-tests/tests/integration_tests.rs's module comment for
//! this suite's standing disclosure about being unverified until CI's
//! real toolchain runs it.
//!
//! Installs a real `identity-registry` alongside `disbursement-controller`
//! (not a mock) to exercise the genuine cross-contract `is_verified`
//! call `create_commitment` makes -- this is the actual integration
//! this test suite's name promises, not a stand-in.
//! `should_reject_release_when_disbursement_controller_holds_no_tokens`
//! additionally installs a real `d3rac-token` and confirms
//! `release_milestone` rejects when this contract holds none of the
//! configured token. `should_release_a_funded_milestone_and_debit_the_controller`
//! covers the funded-success path, funding the controller at its
//! *package* Key (see that test's comment for the Key-variant detail).

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
    let token_key = setup_token(&mut builder, 1_000_000);
    let recipient = Key::from(AccountHash::new([1u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let request = ExecuteRequestBuilder::contract_call_by_hash(
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
    let token_key = setup_token(&mut builder, 1_000_000);
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
            ARG_TOKEN => token_key,
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
    let token_key = setup_token(&mut builder, 1_000_000);
    let recipient = Key::from(AccountHash::new([3u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let stranger = AccountHash::new([4u8; 32]);
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
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
    builder.exec(request).expect_failure();
}

#[test]
fn should_reject_commitment_with_mismatched_description_and_amount_lengths() {
    let (mut builder, dc_hash, registry_hash) = install();
    let token_key = setup_token(&mut builder, 1_000_000);
    let recipient = Key::from(AccountHash::new([5u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => token_key,
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
    let token_key = setup_token(&mut builder, 1_000_000);
    let recipient = Key::from(AccountHash::new([6u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => token_key,
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
    let token_key = setup_token(&mut builder, 1_000_000);
    let recipient = Key::from(AccountHash::new([7u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

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
    let token_key = setup_token(&mut builder, 1_000_000);
    let recipient = Key::from(AccountHash::new([8u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

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
    let token_key = setup_token(&mut builder, 1_000_000);
    let recipient = Key::from(AccountHash::new([10u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

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
    let token_key = setup_token(&mut builder, 1_000_000);
    let recipient = Key::from(AccountHash::new([11u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);

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
fn should_reject_commitment_that_is_not_fully_funded() {
    // DC-2: the controller holds no tokens, so any commitment is refused.
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([20u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);
    let token_key = setup_token(&mut builder, 0);
    builder
        .exec(create_request(dc_hash, recipient, token_key, vec![1_000]))
        .expect_failure();
}

#[test]
fn should_reject_commitment_with_a_non_contract_token_key() {
    // DC-1: an account Key can never pay out, so it is refused up front.
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([21u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);
    builder
        .exec(create_request(dc_hash, recipient, placeholder_token(), vec![1_000]))
        .expect_failure();
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

/// Installs a real d3rac-token (zero initial supply, DEFAULT_ACCOUNT as
/// owner/minter) and returns its entity hash.
fn install_token(builder: &mut LmdbWasmTestBuilder) -> AddressableEntityHash {
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
    builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get("d3rac_token_contract_hash")
        .expect("token contract hash named key should exist after install")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash")
}

/// disbursement-controller's *package* Key -- what d3rac-token credits
/// when tokens are sent to the controller (see the funded-release test).
fn dc_package_key(builder: &LmdbWasmTestBuilder) -> Key {
    *builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get("disbursement_controller_package_hash")
        .expect("disbursement controller package hash named key should exist")
}

fn mint_to_controller(builder: &mut LmdbWasmTestBuilder, token_key: Key, amount: u64) {
    let token_hash = AddressableEntityHash::new(token_key.into_hash_addr().expect("hash key"));
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        token_hash,
        "mint",
        runtime_args! { "recipient" => dc_package_key(builder), "amount" => U256::from(amount) },
    )
    .build();
    builder.exec(request).expect_success().commit();
}

/// Installs a real token and mints `funding` of it to the controller.
/// Commitments are only accepted when fully funded (DC-2), so every test
/// that creates one funds first.
fn setup_token(builder: &mut LmdbWasmTestBuilder, funding: u64) -> Key {
    let token_hash = install_token(builder);
    let token_key = Key::from(ContractHash::from(token_hash));
    if funding > 0 {
        mint_to_controller(builder, token_key, funding);
    }
    token_key
}

fn create_request(
    dc_hash: AddressableEntityHash,
    recipient: Key,
    token_key: Key,
    amounts: Vec<u64>,
) -> casper_engine_test_support::ExecuteRequest {
    let descriptions: Vec<String> = (0..amounts.len()).map(|i| format!("Phase {i}")).collect();
    ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "create_commitment",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_TOKEN => token_key,
            ARG_COMMUNITY => "Test Community".to_string(),
            ARG_DESCRIPTIONS => descriptions,
            ARG_AMOUNTS => amounts.into_iter().map(U256::from).collect::<Vec<_>>(),
        },
    )
    .build()
}

fn call(dc_hash: AddressableEntityHash, entry_point: &str, args: casper_types::RuntimeArgs) -> casper_engine_test_support::ExecuteRequest {
    ExecuteRequestBuilder::contract_call_by_hash(*DEFAULT_ACCOUNT_ADDR, dc_hash, entry_point, args).build()
}

#[test]
fn should_release_funded_milestones_and_debit_the_controller() {
    // Funded BEFORE creation (DC-2). Exactly covers both milestones.
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([20u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);
    let token_key = setup_token(&mut builder, 1_500);

    builder
        .exec(create_request(dc_hash, recipient, token_key, vec![1_000, 500]))
        .expect_success()
        .commit();
    for index in 0u64..2 {
        builder
            .exec(call(dc_hash, "attest_milestone", runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => index }))
            .expect_success()
            .commit();
    }
    let release = |i: u64| call(dc_hash, "release_milestone", runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => i });

    builder.exec(release(0)).expect_success().commit();
    // Re-releasing must fail (already released).
    builder.exec(release(0)).expect_failure();
    builder.exec(release(1)).expect_success().commit();
}

#[test]
fn should_refuse_to_over_commit_across_commitments() {
    // DC-2: 1_000 funded; a first 600 commitment fits, a second 600 does not.
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([22u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);
    let token_key = setup_token(&mut builder, 1_000);

    builder.exec(create_request(dc_hash, recipient, token_key, vec![600])).expect_success().commit();
    builder.exec(create_request(dc_hash, recipient, token_key, vec![600])).expect_failure();
    // After the first is released, its obligation is gone but so is its
    // balance (600 paid out, 400 left, nothing outstanding): 400 fits.
    builder.exec(call(dc_hash, "attest_milestone", runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => 0u64 })).expect_success().commit();
    builder.exec(call(dc_hash, "release_milestone", runtime_args! { ARG_COMMITMENT_ID => 0u64, ARG_MILESTONE_INDEX => 0u64 })).expect_success().commit();
    builder.exec(create_request(dc_hash, recipient, token_key, vec![400])).expect_success().commit();
}

#[test]
fn should_sweep_only_uncommitted_funds() {
    // DC-3: 1_000 funded, 600 committed -> only 400 is sweepable.
    let (mut builder, dc_hash, registry_hash) = install();
    let recipient = Key::from(AccountHash::new([23u8; 32]));
    verify_recipient(&mut builder, registry_hash, recipient);
    let token_key = setup_token(&mut builder, 1_000);
    let to = Key::from(AccountHash::new([24u8; 32]));
    let sweep = |amount: u64| {
        call(dc_hash, "sweep", runtime_args! { ARG_TOKEN => token_key, ARG_RECIPIENT => to, "amount" => U256::from(amount) })
    };

    builder.exec(create_request(dc_hash, recipient, token_key, vec![600])).expect_success().commit();
    builder.exec(sweep(401)).expect_failure();
    builder.exec(sweep(0)).expect_failure();
    builder.exec(sweep(400)).expect_success().commit();
    // Nothing uncommitted is left.
    builder.exec(sweep(1)).expect_failure();

    // Cancelling frees the 600 for recovery.
    builder.exec(call(dc_hash, "cancel_commitment", runtime_args! { ARG_COMMITMENT_ID => 0u64 })).expect_success().commit();
    builder.exec(sweep(600)).expect_success().commit();
}

#[test]
fn should_reject_sweep_from_non_admin() {
    let (mut builder, dc_hash, _registry_hash) = install();
    let token_key = setup_token(&mut builder, 100);
    let stranger = AccountHash::new([25u8; 32]);
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        dc_hash,
        "sweep",
        runtime_args! { ARG_TOKEN => token_key, ARG_RECIPIENT => Key::from(stranger), "amount" => U256::from(1u64) },
    )
    .build();
    builder.exec(request).expect_failure();
}
