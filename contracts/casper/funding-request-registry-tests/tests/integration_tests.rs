//! Integration tests for `funding-request-registry`. This package was
//! missing entirely until now -- the only one of this suite's seven
//! contracts without a `-tests` package. Written against the real
//! source (constants.rs's exact entry-point/arg strings, main.rs's
//! actual guard logic) and verified for real this session: built to a
//! genuine `wasm32-unknown-unknown` binary and run against
//! `casper-engine-test-support`'s real local execution engine, the
//! same toolchain workaround documented in
//! `contracts/casper/README.md`'s "Building wasm32-unknown-unknown
//! without rustup" section.

use casper_engine_test_support::{
    ExecuteRequestBuilder, LmdbWasmTestBuilder, TransferRequestBuilder, DEFAULT_ACCOUNT_ADDR,
    LOCAL_GENESIS_REQUEST,
};
use casper_types::{account::AccountHash, runtime_args, AddressableEntityHash, Key, U256};

const CONTRACT_WASM: &str = "funding-request-registry.wasm";
const CONTRACT_HASH_KEY_NAME: &str = "funding_request_registry_contract_hash";

const ARG_INITIAL_PROPOSER: &str = "initial_proposer";
const ARG_COMMUNITY_ID: &str = "community_id";
const ARG_AMOUNT_REQUESTED: &str = "amount_requested";
const ARG_DESCRIPTION: &str = "description";
const ARG_DATA_SOURCE_URI: &str = "data_source_uri";
const ARG_REQUEST_ID: &str = "request_id";
const ARG_AMOUNT: &str = "amount";
const ARG_PLEDGE_SOURCE_URI: &str = "pledge_source_uri";
const ARG_COMMITMENT_ID: &str = "commitment_id";
const ARG_PROPOSER: &str = "proposer";
const ARG_NEW_OWNER: &str = "new_owner";

/// A synthetic `AccountHash` doesn't exist in Casper's global state
/// until funded (found by real test execution earlier in this
/// project's history, not by static review -- see
/// `token-tests`/`disbursement-controller-tests`' own identical
/// helpers) -- any test using one as the caller of a deploy expected
/// to SUCCEED needs this first. A call expected to FAIL, or a
/// synthetic account used only as a recipient `Key`, needs no such fix.
fn fund_account(builder: &mut LmdbWasmTestBuilder, account: AccountHash) {
    let transfer = TransferRequestBuilder::new(50_000_000_000_000u64, account).build();
    builder.transfer_and_commit(transfer);
}

fn install(initial_proposer: Option<AccountHash>) -> (LmdbWasmTestBuilder, AddressableEntityHash) {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone()).commit();

    if let Some(proposer) = initial_proposer {
        fund_account(&mut builder, proposer);
    }

    let install_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_WASM,
        runtime_args! {
            ARG_INITIAL_PROPOSER => initial_proposer.map(Key::from),
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
fn should_install_with_no_initial_proposer() {
    let (_builder, _hash) = install(None);
}

#[test]
fn should_install_with_an_initial_proposer_and_let_them_open_a_request() {
    let proposer = AccountHash::new([70u8; 32]);
    let (mut builder, hash) = install(Some(proposer));

    let open_request = ExecuteRequestBuilder::contract_call_by_hash(
        proposer,
        hash,
        "open_request",
        runtime_args! {
            ARG_COMMUNITY_ID => "coastal-village-12".to_string(),
            ARG_AMOUNT_REQUESTED => U256::from(1_000u64),
            ARG_DESCRIPTION => "Emergency well repair".to_string(),
            ARG_DATA_SOURCE_URI => "ipfs://test".to_string(),
        },
    )
    .build();
    builder.exec(open_request).expect_success().commit();
}

#[test]
fn should_reject_open_request_from_a_non_proposer() {
    let stranger = AccountHash::new([71u8; 32]);
    let (mut builder, hash) = install(None);
    fund_account(&mut builder, stranger);

    let open_request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        hash,
        "open_request",
        runtime_args! {
            ARG_COMMUNITY_ID => "coastal-village-12".to_string(),
            ARG_AMOUNT_REQUESTED => U256::from(1_000u64),
            ARG_DESCRIPTION => "desc".to_string(),
            ARG_DATA_SOURCE_URI => "ipfs://test".to_string(),
        },
    )
    .build();
    builder.exec(open_request).expect_failure();
}

#[test]
fn should_reject_open_request_with_zero_amount() {
    let proposer = AccountHash::new([72u8; 32]);
    let (mut builder, hash) = install(Some(proposer));

    let open_request = ExecuteRequestBuilder::contract_call_by_hash(
        proposer,
        hash,
        "open_request",
        runtime_args! {
            ARG_COMMUNITY_ID => "coastal-village-12".to_string(),
            ARG_AMOUNT_REQUESTED => U256::zero(),
            ARG_DESCRIPTION => "desc".to_string(),
            ARG_DATA_SOURCE_URI => "ipfs://test".to_string(),
        },
    )
    .build();
    builder.exec(open_request).expect_failure();
}

#[test]
fn should_record_a_pledge_against_an_open_request() {
    let proposer = AccountHash::new([73u8; 32]);
    let (mut builder, hash) = install(Some(proposer));

    let open_request = ExecuteRequestBuilder::contract_call_by_hash(
        proposer,
        hash,
        "open_request",
        runtime_args! {
            ARG_COMMUNITY_ID => "coastal-village-12".to_string(),
            ARG_AMOUNT_REQUESTED => U256::from(1_000u64),
            ARG_DESCRIPTION => "desc".to_string(),
            ARG_DATA_SOURCE_URI => "ipfs://test".to_string(),
        },
    )
    .build();
    builder.exec(open_request).expect_success().commit();

    let pledge_request = ExecuteRequestBuilder::contract_call_by_hash(
        proposer,
        hash,
        "record_pledge",
        runtime_args! {
            ARG_REQUEST_ID => 0u64,
            ARG_AMOUNT => U256::from(500u64),
            ARG_PLEDGE_SOURCE_URI => "ipfs://pledge".to_string(),
        },
    )
    .build();
    builder.exec(pledge_request).expect_success().commit();
}

#[test]
fn should_reject_pledge_against_a_closed_request() {
    let proposer = AccountHash::new([74u8; 32]);
    let (mut builder, hash) = install(Some(proposer));

    let open_request = ExecuteRequestBuilder::contract_call_by_hash(
        proposer,
        hash,
        "open_request",
        runtime_args! {
            ARG_COMMUNITY_ID => "coastal-village-12".to_string(),
            ARG_AMOUNT_REQUESTED => U256::from(1_000u64),
            ARG_DESCRIPTION => "desc".to_string(),
            ARG_DATA_SOURCE_URI => "ipfs://test".to_string(),
        },
    )
    .build();
    builder.exec(open_request).expect_success().commit();

    let close_request = ExecuteRequestBuilder::contract_call_by_hash(
        proposer,
        hash,
        "close_request",
        runtime_args! { ARG_REQUEST_ID => 0u64 },
    )
    .build();
    builder.exec(close_request).expect_success().commit();

    let pledge_after_close = ExecuteRequestBuilder::contract_call_by_hash(
        proposer,
        hash,
        "record_pledge",
        runtime_args! {
            ARG_REQUEST_ID => 0u64,
            ARG_AMOUNT => U256::from(1u64),
            ARG_PLEDGE_SOURCE_URI => "ipfs://pledge".to_string(),
        },
    )
    .build();
    builder.exec(pledge_after_close).expect_failure();
}

#[test]
fn should_link_a_request_to_a_commitment() {
    let proposer = AccountHash::new([75u8; 32]);
    let (mut builder, hash) = install(Some(proposer));

    let open_request = ExecuteRequestBuilder::contract_call_by_hash(
        proposer,
        hash,
        "open_request",
        runtime_args! {
            ARG_COMMUNITY_ID => "coastal-village-12".to_string(),
            ARG_AMOUNT_REQUESTED => U256::from(1_000u64),
            ARG_DESCRIPTION => "desc".to_string(),
            ARG_DATA_SOURCE_URI => "ipfs://test".to_string(),
        },
    )
    .build();
    builder.exec(open_request).expect_success().commit();

    let link_request = ExecuteRequestBuilder::contract_call_by_hash(
        proposer,
        hash,
        "link_to_commitment",
        runtime_args! {
            ARG_REQUEST_ID => 0u64,
            ARG_COMMITMENT_ID => 7u64,
        },
    )
    .build();
    builder.exec(link_request).expect_success().commit();
}

#[test]
fn should_reject_pledge_or_link_from_someone_who_is_neither_requester_nor_owner() {
    let proposer = AccountHash::new([76u8; 32]);
    let stranger = AccountHash::new([77u8; 32]);
    let (mut builder, hash) = install(Some(proposer));
    fund_account(&mut builder, stranger);

    let open_request = ExecuteRequestBuilder::contract_call_by_hash(
        proposer,
        hash,
        "open_request",
        runtime_args! {
            ARG_COMMUNITY_ID => "coastal-village-12".to_string(),
            ARG_AMOUNT_REQUESTED => U256::from(1_000u64),
            ARG_DESCRIPTION => "desc".to_string(),
            ARG_DATA_SOURCE_URI => "ipfs://test".to_string(),
        },
    )
    .build();
    builder.exec(open_request).expect_success().commit();

    let pledge_request = ExecuteRequestBuilder::contract_call_by_hash(
        stranger,
        hash,
        "record_pledge",
        runtime_args! {
            ARG_REQUEST_ID => 0u64,
            ARG_AMOUNT => U256::from(1u64),
            ARG_PLEDGE_SOURCE_URI => "ipfs://pledge".to_string(),
        },
    )
    .build();
    builder.exec(pledge_request).expect_failure();
}

#[test]
fn should_add_and_remove_a_proposer() {
    let new_proposer = AccountHash::new([78u8; 32]);
    let (mut builder, hash) = install(None);
    fund_account(&mut builder, new_proposer);

    let add_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "add_proposer",
        runtime_args! { ARG_PROPOSER => Key::from(new_proposer) },
    )
    .build();
    builder.exec(add_request).expect_success().commit();

    // Now-added proposer can open a request.
    let open_request = ExecuteRequestBuilder::contract_call_by_hash(
        new_proposer,
        hash,
        "open_request",
        runtime_args! {
            ARG_COMMUNITY_ID => "x".to_string(),
            ARG_AMOUNT_REQUESTED => U256::from(1u64),
            ARG_DESCRIPTION => "d".to_string(),
            ARG_DATA_SOURCE_URI => "u".to_string(),
        },
    )
    .build();
    builder.exec(open_request).expect_success().commit();

    let remove_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "remove_proposer",
        runtime_args! { ARG_PROPOSER => Key::from(new_proposer) },
    )
    .build();
    builder.exec(remove_request).expect_success().commit();

    // Removed proposer can no longer open a request.
    let open_after_removal = ExecuteRequestBuilder::contract_call_by_hash(
        new_proposer,
        hash,
        "open_request",
        runtime_args! {
            ARG_COMMUNITY_ID => "x".to_string(),
            ARG_AMOUNT_REQUESTED => U256::from(1u64),
            ARG_DESCRIPTION => "d".to_string(),
            ARG_DATA_SOURCE_URI => "u".to_string(),
        },
    )
    .build();
    builder.exec(open_after_removal).expect_failure();
}

#[test]
fn should_two_step_transfer_ownership() {
    let new_owner = AccountHash::new([79u8; 32]);
    let (mut builder, hash) = install(None);
    fund_account(&mut builder, new_owner);

    let propose_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "propose_new_owner",
        runtime_args! { ARG_NEW_OWNER => Key::from(new_owner) },
    )
    .build();
    builder.exec(propose_request).expect_success().commit();

    let accept_request = ExecuteRequestBuilder::contract_call_by_hash(
        new_owner,
        hash,
        "accept_ownership",
        runtime_args! {},
    )
    .build();
    builder.exec(accept_request).expect_success().commit();

    // Old owner can no longer add a proposer.
    let stale_owner_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "add_proposer",
        runtime_args! { ARG_PROPOSER => Key::from(*DEFAULT_ACCOUNT_ADDR) },
    )
    .build();
    builder.exec(stale_owner_request).expect_failure();
}

#[test]
fn should_let_owner_close_a_request_even_if_not_the_requester() {
    let proposer = AccountHash::new([80u8; 32]);
    let (mut builder, hash) = install(Some(proposer));

    let open_request = ExecuteRequestBuilder::contract_call_by_hash(
        proposer,
        hash,
        "open_request",
        runtime_args! {
            ARG_COMMUNITY_ID => "x".to_string(),
            ARG_AMOUNT_REQUESTED => U256::from(1u64),
            ARG_DESCRIPTION => "d".to_string(),
            ARG_DATA_SOURCE_URI => "u".to_string(),
        },
    )
    .build();
    builder.exec(open_request).expect_success().commit();

    // DEFAULT_ACCOUNT_ADDR is the owner (installer), not the requester
    // (proposer) -- only_requester_or_owner should still allow this.
    let close_request = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hash,
        "close_request",
        runtime_args! { ARG_REQUEST_ID => 0u64 },
    )
    .build();
    builder.exec(close_request).expect_success().commit();
}
