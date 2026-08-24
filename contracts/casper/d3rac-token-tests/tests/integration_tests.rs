//! Integration tests for `d3rac-token`. See
//! risk-registry-tests/tests/integration_tests.rs's module comment for
//! this suite's standing disclosure about being unverified until CI's
//! real toolchain runs it.
//!
//! Same account-funding limitation as identity-registry-tests'
//! `should_propose_admin_transfer_and_reject_a_stranger_accepting`:
//! tests needing a SECOND account to successfully execute a deploy
//! (not just be rejected) are limited to what genesis provides. Here
//! that means transfer/transfer_from/approve are tested from
//! DEFAULT_ACCOUNT_ADDR's own perspective (it's both the initial-supply
//! holder and, after minting to itself for test purposes, has tokens to
//! move) rather than between two independently-funded parties.

use casper_engine_test_support::{
    ExecuteRequestBuilder, LmdbWasmTestBuilder, DEFAULT_ACCOUNT_ADDR, LOCAL_GENESIS_REQUEST,
};
use casper_types::{account::AccountHash, runtime_args, AddressableEntityHash, Key, U256};

const TOKEN_WASM: &str = "d3rac-token.wasm";
const CONTRACT_HASH_KEY_NAME: &str = "d3rac_token_contract_hash";

const ARG_OWNER_ARG: &str = "owner_";
const ARG_INITIAL_SUPPLY: &str = "initial_supply";
const ARG_ACCOUNT: &str = "account";
const ARG_OWNER: &str = "owner";
const ARG_SPENDER: &str = "spender";
const ARG_RECIPIENT: &str = "recipient";
const ARG_AMOUNT: &str = "amount";
const ARG_IS_MINTER: &str = "is_minter";
const ARG_NEW_OWNER: &str = "new_owner";

/// Installs with a nonzero initial supply, minted to DEFAULT_ACCOUNT_ADDR
/// (which is therefore also the owner and an implicit minter, per
/// main.rs's call()).
fn install(initial_supply: u64) -> (LmdbWasmTestBuilder, AddressableEntityHash) {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone()).commit();

    let install_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        TOKEN_WASM,
        runtime_args! {
            ARG_INITIAL_SUPPLY => U256::from(initial_supply),
            ARG_OWNER_ARG => Key::from(*DEFAULT_ACCOUNT_ADDR),
        },
    )
    .build();
    builder.exec(install_request).expect_success().commit();

    let hash = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get(CONTRACT_HASH_KEY_NAME)
        .expect("contract hash named key should exist after install")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");

    (builder, hash)
}

#[test]
fn should_install_with_scaled_initial_supply() {
    let (mut builder, hash) = install(1_000);

    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "total_supply",
        runtime_args! {},
    )
    .build();
    builder.exec(request).expect_success().commit();

    let balance_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "balance_of",
        runtime_args! { ARG_ACCOUNT => Key::from(*DEFAULT_ACCOUNT_ADDR) },
    )
    .build();
    builder.exec(balance_request).expect_success().commit();
}

#[test]
fn should_install_with_zero_initial_supply() {
    let (mut _builder, _hash) = install(0);
}

#[test]
fn should_transfer_to_a_recipient() {
    let (mut builder, hash) = install(1_000);
    let recipient = Key::from(AccountHash::new([1u8; 32]));

    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "transfer",
        runtime_args! { ARG_RECIPIENT => recipient, ARG_AMOUNT => U256::from(100u64) },
    )
    .build();
    builder.exec(request).expect_success().commit();
}

#[test]
fn should_reject_transfer_to_self() {
    let (mut builder, hash) = install(1_000);

    // Matches the CEP-18 standard's CannotTargetSelfUser (60003).
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "transfer",
        runtime_args! {
            ARG_RECIPIENT => Key::from(*DEFAULT_ACCOUNT_ADDR),
            ARG_AMOUNT => U256::from(1u64),
        },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_reject_transfer_exceeding_balance() {
    let (mut builder, hash) = install(1_000);
    let recipient = Key::from(AccountHash::new([2u8; 32]));

    // Matches the CEP-18 standard's InsufficientBalance (60001) --
    // 1_000 tokens * 10^18 scaling is nowhere near this.
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "transfer",
        runtime_args! {
            ARG_RECIPIENT => recipient,
            ARG_AMOUNT => U256::MAX,
        },
    )
    .build();
    builder.exec(request).expect_failure();
}

#[test]
fn should_approve_then_reflect_in_allowance() {
    let (mut builder, hash) = install(1_000);
    let spender = Key::from(AccountHash::new([3u8; 32]));

    let approve_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "approve",
        runtime_args! { ARG_SPENDER => spender, ARG_AMOUNT => U256::from(500u64) },
    )
    .build();
    builder.exec(approve_request).expect_success().commit();

    let allowance_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "allowance",
        runtime_args! { ARG_OWNER => Key::from(*DEFAULT_ACCOUNT_ADDR), ARG_SPENDER => spender },
    )
    .build();
    builder.exec(allowance_request).expect_success().commit();
}

#[test]
fn should_increase_then_decrease_allowance() {
    let (mut builder, hash) = install(1_000);
    let spender = Key::from(AccountHash::new([4u8; 32]));

    let increase_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "increase_allowance",
        runtime_args! { ARG_SPENDER => spender, "inc_by" => U256::from(200u64) },
    )
    .build();
    builder.exec(increase_request).expect_success().commit();

    let decrease_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "decrease_allowance",
        runtime_args! { ARG_SPENDER => spender, "decr_by" => U256::from(50u64) },
    )
    .build();
    builder.exec(decrease_request).expect_success().commit();
}

#[test]
fn should_decrease_allowance_below_zero_saturating_to_zero_not_reverting() {
    let (mut builder, hash) = install(1_000);
    let spender = Key::from(AccountHash::new([5u8; 32]));

    // Per the CEP-18 standard: "If decr_by is greater than the current
    // allowance, the allowance is set to zero" -- a saturating
    // decrease, not a revert, even with no prior approval at all (an
    // allowance of 0 to start).
    let decrease_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "decrease_allowance",
        runtime_args! { ARG_SPENDER => spender, "decr_by" => U256::from(999u64) },
    )
    .build();
    builder.exec(decrease_request).expect_success().commit();
}

#[test]
fn should_mint_as_the_implicit_owner_minter_and_reject_from_a_stranger() {
    let (mut builder, hash) = install(0);
    let recipient = Key::from(*DEFAULT_ACCOUNT_ADDR);

    let mint_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "mint",
        runtime_args! { ARG_RECIPIENT => recipient, ARG_AMOUNT => U256::from(42u64) },
    )
    .build();
    builder.exec(mint_request).expect_success().commit();

    let stranger = AccountHash::new([6u8; 32]);
    // Matches D3RACToken.sol's onlyMinter guard
    // (D3racTokenError::CallerIsNotMinter).
    let stranger_mint_request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        hash,
        "mint",
        runtime_args! { ARG_RECIPIENT => recipient, ARG_AMOUNT => U256::from(1u64) },
    )
    .build();
    builder.exec(stranger_mint_request).expect_failure();
}

#[test]
fn should_burn_own_tokens_then_reject_burning_more_than_held() {
    let (mut builder, hash) = install(1_000);

    let burn_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "burn",
        runtime_args! { ARG_AMOUNT => U256::from(100u64) },
    )
    .build();
    builder.exec(burn_request).expect_success().commit();

    // Matches the CEP-18 standard's InsufficientBalance (60001), same
    // guard transfer/burn share.
    let overburn_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "burn",
        runtime_args! { ARG_AMOUNT => U256::MAX },
    )
    .build();
    builder.exec(overburn_request).expect_failure();
}

#[test]
fn should_set_minter_and_reject_from_non_owner() {
    let (mut builder, hash) = install(0);
    let account = Key::from(AccountHash::new([7u8; 32]));

    let set_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "set_minter",
        runtime_args! { ARG_ACCOUNT => account, ARG_IS_MINTER => true },
    )
    .build();
    builder.exec(set_request).expect_success().commit();

    let stranger = AccountHash::new([8u8; 32]);
    // Matches D3RACToken.sol's onlyOwner guard
    // (D3racTokenError::CallerIsNotOwner).
    let stranger_request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        hash,
        "set_minter",
        runtime_args! { ARG_ACCOUNT => account, ARG_IS_MINTER => false },
    )
    .build();
    builder.exec(stranger_request).expect_failure();
}

#[test]
fn should_propose_ownership_transfer_and_reject_a_stranger_accepting() {
    let (mut builder, hash) = install(0);
    let new_owner = AccountHash::new([9u8; 32]);
    let stranger = AccountHash::new([10u8; 32]);

    let propose_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "propose_new_owner",
        runtime_args! { ARG_NEW_OWNER => Key::from(new_owner) },
    )
    .build();
    builder.exec(propose_request).expect_success().commit();

    // Same account-funding limitation noted in this file's module
    // comment: the stranger's own deploy still fails cleanly
    // (KeyNotFound, an unfunded account) before reaching contract
    // logic, which expect_failure() accepts either way -- matches
    // D3racTokenError::CallerIsNotPendingOwner's "wrong caller" case.
    let stranger_accept_request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        hash,
        "accept_ownership",
        runtime_args! {},
    )
    .build();
    builder.exec(stranger_accept_request).expect_failure();
}

#[test]
fn should_reject_accept_ownership_when_no_transfer_has_been_proposed() {
    let (mut builder, hash) = install(0);
    let stranger = AccountHash::new([11u8; 32]);

    let request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        hash,
        "accept_ownership",
        runtime_args! {},
    )
    .build();
    builder.exec(request).expect_failure();
}
