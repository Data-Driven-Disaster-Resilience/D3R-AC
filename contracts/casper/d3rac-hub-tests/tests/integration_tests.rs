//! Integration tests for `d3rac-hub` -- the first time this suite's
//! Hub-wiring model has been exercised at all, locally or otherwise.
//! Casper analog of the ethers.js dry-run this same review pass did
//! against the TRON suite (deploy all seven, wire the Hub to all five
//! modules, hand the Hub's own admin to a 1-of-1 multisig, then
//! execute a real Hub-mediated write through the whole chain) --
//! written in Rust against `casper-engine-test-support` instead,
//! since that's the tool that's actually available here. See
//! `risk-registry-tests/tests/integration_tests.rs`'s module comment
//! for this suite's standing disclosure about being unverified until
//! CI's real toolchain runs it -- this is the first real compiler/test
//! pass this file gets, same as every contract in this suite before
//! its own first CI round.
//!
//! Every deploy in this file is submitted by `DEFAULT_ACCOUNT_ADDR` --
//! same funding-account limitation `multisig-admin-tests`' own module
//! comment documents (no confirmed way to fund a fresh `AccountHash`
//! for a deploy that's supposed to SUCCEED in this dependency
//! version). This is also, conveniently, exactly what the real Shasta
//! deployment actually used: a "deliberately minimal 1-of-1
//! MultiSigAdmin" (see `docs/deployment-guide.md`) -- so this test's
//! shape isn't a workaround away from the real topology, it matches it.
//!
//! One real, load-bearing gap this file found while being written, not
//! by being written: `d3rac-hub` has no entry point that reads back
//! its own current `admin` -- `system_status()` returns the five
//! module addresses, `is_paused`, and some aggregate counts, but not
//! `admin` itself. So this test can't directly assert "the Hub's admin
//! is now the multisig" by querying it. It proves the same fact
//! indirectly instead, which is arguably more convincing anyway: after
//! the handoff, the exact same `register_community` call that used to
//! succeed when submitted directly by `DEFAULT_ACCOUNT_ADDR` now FAILS
//! when submitted the same way, and SUCCEEDS when routed through the
//! multisig -- that combination only holds if the admin really moved,
//! not just if some unrelated thing broke. Filed as a real
//! follow-up in `contracts/casper/README.md`, not silently worked
//! around.

use casper_engine_test_support::{
    ExecuteRequestBuilder, LmdbWasmTestBuilder, DEFAULT_ACCOUNT_ADDR, LOCAL_GENESIS_REQUEST,
};
use casper_types::bytesrepr::ToBytes;
use casper_types::{
    contracts::ContractHash, runtime_args, AddressableEntityHash, Key, U256,
};

const RISK_REGISTRY_WASM: &str = "risk-registry.wasm";
const IDENTITY_REGISTRY_WASM: &str = "identity-registry.wasm";
const DISBURSEMENT_CONTROLLER_WASM: &str = "disbursement-controller.wasm";
const D3RAC_TOKEN_WASM: &str = "d3rac-token.wasm";
const FUNDING_REQUEST_REGISTRY_WASM: &str = "funding-request-registry.wasm";
const D3RAC_HUB_WASM: &str = "d3rac-hub.wasm";
const MULTISIG_ADMIN_WASM: &str = "multisig-admin.wasm";

const RR_CONTRACT_HASH_KEY: &str = "risk_registry_contract_hash";
const IR_CONTRACT_HASH_KEY: &str = "identity_registry_contract_hash";
const DC_CONTRACT_HASH_KEY: &str = "disbursement_controller_contract_hash";
const TOKEN_CONTRACT_HASH_KEY: &str = "d3rac_token_contract_hash";
const FRR_CONTRACT_HASH_KEY: &str = "funding_request_registry_contract_hash";
const HUB_CONTRACT_HASH_KEY: &str = "d3rac_hub_contract_hash";
const HUB_PACKAGE_HASH_KEY: &str = "d3rac_hub_package_hash";
const MULTISIG_CONTRACT_HASH_KEY: &str = "multisig_admin_contract_hash";
const MULTISIG_PACKAGE_HASH_KEY: &str = "multisig_admin_package_hash";

#[test]
fn should_wire_hub_to_all_five_modules_transfer_admin_to_multisig_and_execute_a_hub_mediated_call()
{
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone()).commit();

    // ---- 1. Install the five modules, each defaulting its own
    //         owner/admin to the installing caller (DEFAULT_ACCOUNT_ADDR)
    //         -- confirmed against each contract's own call(), not
    //         assumed uniform across all five (d3rac-token is the one
    //         exception: it takes owner_ as an explicit arg instead).
    let install_rr = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        RISK_REGISTRY_WASM,
        runtime_args! {
            "initial_threshold" => 0u64,
            "initial_data_feeder" => Option::<Key>::None,
        },
    )
    .build();
    builder.exec(install_rr).expect_success().commit();

    let install_ir = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        IDENTITY_REGISTRY_WASM,
        runtime_args! { "initial_verifier" => Option::<Key>::None },
    )
    .build();
    builder.exec(install_ir).expect_success().commit();

    let ir_contract_hash: AddressableEntityHash = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account")
        .named_keys()
        .get(IR_CONTRACT_HASH_KEY)
        .expect("identity registry contract hash should exist after install")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");

    let install_dc = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        DISBURSEMENT_CONTROLLER_WASM,
        // registry_hash needs identity-registry's CONTRACT hash, not its
        // package hash -- confirmed against disbursement-controller-
        // tests' own already-passing install() (its registry_hash =
        // Key::from(ContractHash::from(registry_hash)), same
        // conversion here). Cross-contract calls made via
        // runtime::call_contract (what disbursement-controller's own
        // is_verified check, and every one of the Hub's calls below,
        // actually use) need a ContractHash specifically; the package
        // hash is what identity-registry expects to see stored as an
        // admin/owner value instead (see the propose_new_admin calls
        // below, which correctly use *package* hashes for that
        // opposite reason). Originally used the package hash here by
        // mistake -- caught by real CI (KeyNotFound), not by review.
        runtime_args! { "registry_hash" => Key::from(ContractHash::from(ir_contract_hash)) },
    )
    .build();
    builder.exec(install_dc).expect_success().commit();

    let install_token = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        D3RAC_TOKEN_WASM,
        runtime_args! {
            "initial_supply" => U256::zero(),
            "owner_" => Key::from(*DEFAULT_ACCOUNT_ADDR),
        },
    )
    .build();
    builder.exec(install_token).expect_success().commit();

    let install_frr = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        FUNDING_REQUEST_REGISTRY_WASM,
        runtime_args! { "initial_proposer" => Option::<Key>::None },
    )
    .build();
    builder.exec(install_frr).expect_success().commit();

    // ---- 2. Read back every module's contract hash + package key
    //         needed for Hub install / wiring / the multisig call
    //         below.
    let account = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account");
    let rr_hash: AddressableEntityHash = account
        .named_keys()
        .get(RR_CONTRACT_HASH_KEY)
        .expect("risk registry contract hash should exist")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");
    let dc_hash: AddressableEntityHash = account
        .named_keys()
        .get(DC_CONTRACT_HASH_KEY)
        .expect("disbursement controller contract hash should exist")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");
    let token_hash: AddressableEntityHash = account
        .named_keys()
        .get(TOKEN_CONTRACT_HASH_KEY)
        .expect("token contract hash should exist")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");
    let frr_hash: AddressableEntityHash = account
        .named_keys()
        .get(FRR_CONTRACT_HASH_KEY)
        .expect("funding request registry contract hash should exist")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");
    // ir_hash reuses ir_contract_hash read earlier (before
    // disbursement-controller's install, which needed it first) --
    // same value, not read twice under two different names.
    let ir_hash = ir_contract_hash;

    // ---- 3. Install the Hub, pointing at all five modules up front
    //         (matching D3RACHub.sol's own constructor-takes-everything
    //         shape -- not wired via setters after the fact). Each
    //         module arg needs that module's CONTRACT hash (wrapped),
    //         not its package hash -- same reasoning as install_dc's
    //         registry_hash above, and the same mistake this line
    //         originally made too, caught by the same CI run.
    let install_hub = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        D3RAC_HUB_WASM,
        runtime_args! {
            "admin_" => Key::from(*DEFAULT_ACCOUNT_ADDR),
            "token_" => Key::from(ContractHash::from(token_hash)),
            "identity_registry_" => Key::from(ContractHash::from(ir_hash)),
            "disbursement_controller_" => Key::from(ContractHash::from(dc_hash)),
            "risk_registry_" => Option::Some(Key::from(ContractHash::from(rr_hash))),
            "funding_request_registry_" => Option::Some(Key::from(ContractHash::from(frr_hash))),
        },
    )
    .build();
    builder.exec(install_hub).expect_success().commit();

    let account = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account");
    let hub_hash: AddressableEntityHash = account
        .named_keys()
        .get(HUB_CONTRACT_HASH_KEY)
        .expect("hub contract hash should exist")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");
    let hub_package_key: Key = *account
        .named_keys()
        .get(HUB_PACKAGE_HASH_KEY)
        .expect("hub package hash should exist");

    // ---- 4. Wire each module to the Hub. Same sequence as
    //         contracts/tron/tronbox/migrations/2_deploy_d3rac.js's
    //         grant-role -> propose -> accept dance for the four
    //         two-step modules; risk-registry is the one exception
    //         (single-step transfer_ownership, no accept phase --
    //         see d3rac-hub/src/main.rs's own header for why).
    let set_verifier_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        ir_hash,
        "set_verifier",
        runtime_args! { "account" => hub_package_key, "is_verifier" => true },
    )
    .build();
    builder.exec(set_verifier_req).expect_success().commit();

    let propose_ir_admin_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        ir_hash,
        "propose_new_admin",
        runtime_args! { "new_admin" => hub_package_key },
    )
    .build();
    builder.exec(propose_ir_admin_req).expect_success().commit();

    let accept_ir_admin_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hub_hash,
        "accept_identity_registry_admin",
        runtime_args! {},
    )
    .build();
    builder.exec(accept_ir_admin_req).expect_success().commit();

    let set_attester_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "set_attester",
        runtime_args! { "account" => hub_package_key, "is_attester" => true },
    )
    .build();
    builder.exec(set_attester_req).expect_success().commit();

    let propose_dc_admin_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        dc_hash,
        "propose_new_admin",
        runtime_args! { "new_admin" => hub_package_key },
    )
    .build();
    builder.exec(propose_dc_admin_req).expect_success().commit();

    let accept_dc_admin_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hub_hash,
        "accept_disbursement_controller_admin",
        runtime_args! {},
    )
    .build();
    builder.exec(accept_dc_admin_req).expect_success().commit();

    let set_minter_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        token_hash,
        "set_minter",
        runtime_args! { "account" => hub_package_key, "is_minter" => true },
    )
    .build();
    builder.exec(set_minter_req).expect_success().commit();

    let propose_token_owner_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        token_hash,
        "propose_new_owner",
        runtime_args! { "new_owner" => hub_package_key },
    )
    .build();
    builder.exec(propose_token_owner_req).expect_success().commit();

    let accept_token_owner_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hub_hash,
        "accept_token_ownership",
        runtime_args! {},
    )
    .build();
    builder.exec(accept_token_owner_req).expect_success().commit();

    // risk-registry: single-step, no accept phase.
    let add_feeder_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        rr_hash,
        "add_data_feeder",
        runtime_args! { "feeder" => hub_package_key },
    )
    .build();
    builder.exec(add_feeder_req).expect_success().commit();

    let transfer_rr_owner_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        rr_hash,
        "transfer_ownership",
        runtime_args! { "new_owner" => hub_package_key },
    )
    .build();
    builder.exec(transfer_rr_owner_req).expect_success().commit();

    let add_proposer_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        frr_hash,
        "add_proposer",
        runtime_args! { "proposer" => hub_package_key },
    )
    .build();
    builder.exec(add_proposer_req).expect_success().commit();

    let propose_frr_owner_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        frr_hash,
        "propose_new_owner",
        runtime_args! { "new_owner" => hub_package_key },
    )
    .build();
    builder.exec(propose_frr_owner_req).expect_success().commit();

    let accept_frr_owner_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hub_hash,
        "accept_funding_request_registry_ownership",
        runtime_args! {},
    )
    .build();
    builder.exec(accept_frr_owner_req).expect_success().commit();

    // ---- 5. Prove the wiring actually took effect BEFORE touching the
    //         multisig at all: register_community, called directly by
    //         DEFAULT_ACCOUNT_ADDR (still the Hub's own admin at this
    //         point), should succeed and land on the real risk-registry.
    let register_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hub_hash,
        "register_community",
        runtime_args! {
            "community_id" => "hub-wiring-test".to_string(),
            "name" => "Hub Wiring Test Community".to_string(),
            "region" => "Test Region".to_string(),
        },
    )
    .build();
    builder.exec(register_req).expect_success().commit();

    let count_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        rr_hash,
        "community_count",
        runtime_args! {},
    )
    .build();
    builder.exec(count_req).expect_success().commit();
    // Not asserting the exact returned count (should be 1) here --
    // same limitation risk-registry-tests' own
    // should_update_risk_and_read_it_back already documents: exact
    // return-value assertion needs builder.get_last_exec_result()'s
    // specific shape under this addressable-entity-era API, not
    // confirmed in this dependency version. What this step actually
    // proves is call success (the Hub-mediated write reached
    // risk-registry and didn't revert); the real proof this test cares
    // about -- that the post-handoff direct call is rejected while the
    // multisig-routed one isn't -- comes from the expect_failure()/
    // expect_success() contrast in steps 7-8 below, not from reading
    // this count.

    // ---- 6. Install a 1-of-1 MultiSigAdmin (same topology the real
    //         Shasta deployment uses) and hand the Hub's admin to it.
    let install_multisig = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        MULTISIG_ADMIN_WASM,
        runtime_args! {
            "owners" => vec![Key::from(*DEFAULT_ACCOUNT_ADDR)],
            "threshold" => 1u64,
        },
    )
    .build();
    builder.exec(install_multisig).expect_success().commit();

    let account = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account");
    let multisig_hash: AddressableEntityHash = account
        .named_keys()
        .get(MULTISIG_CONTRACT_HASH_KEY)
        .expect("multisig contract hash should exist")
        .into_entity_hash()
        .expect("should resolve to an addressable entity hash");
    let multisig_package_key: Key = *account
        .named_keys()
        .get(MULTISIG_PACKAGE_HASH_KEY)
        .expect("multisig package hash should exist");

    let propose_hub_admin_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hub_hash,
        "propose_new_admin",
        runtime_args! { "new_admin" => multisig_package_key },
    )
    .build();
    builder.exec(propose_hub_admin_req).expect_success().commit();

    // Route the Hub's own acceptance through the multisig -- only the
    // multisig's own code can make it the immediate caller of the
    // Hub's accept_admin, same reasoning as every other *-tests file's
    // own accept-via-multisig step.
    let accept_args_bytes = runtime_args! {}
        .to_bytes()
        .expect("empty RuntimeArgs should always serialize");
    let submit_accept_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        multisig_hash,
        "submit_transaction",
        runtime_args! {
            "target_package_hash" => hub_package_key,
            "target_entry_point" => "accept_admin".to_string(),
            "target_args_bytes" => accept_args_bytes,
        },
    )
    .build();
    builder.exec(submit_accept_req).expect_success().commit();

    let execute_accept_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        multisig_hash,
        "execute_transaction",
        runtime_args! { "tx_id" => 0u64 },
    )
    .build();
    builder.exec(execute_accept_req).expect_success().commit();

    // ---- 7. Proof, part one: the exact same direct call that
    //         succeeded in step 5 must now FAIL -- DEFAULT_ACCOUNT_ADDR
    //         is funded and was legitimately admin a moment ago, so a
    //         failure here is real access-control rejection, not a
    //         funding artifact (unlike an unfunded stranger's deploy,
    //         which fails for an unrelated reason -- see this file's
    //         own header).
    let register_req_after_handoff = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        hub_hash,
        "register_community",
        runtime_args! {
            "community_id" => "should-not-land".to_string(),
            "name" => "Should Not Land".to_string(),
            "region" => "N/A".to_string(),
        },
    )
    .build();
    builder.exec(register_req_after_handoff).expect_failure();

    // ---- 8. Proof, part two: the SAME call, routed through the
    //         multisig, must SUCCEED -- exercising immediate-caller
    //         resolution at both hops at once (multisig -> Hub, then
    //         Hub -> risk-registry), the exact mechanism
    //         `fix/get-caller-systemic-immediate-caller` fixed and this
    //         Hub was written to depend on from the start.
    let register_args = runtime_args! {
        "community_id" => "hub-mediated-via-multisig".to_string(),
        "name" => "Routed Through Multisig".to_string(),
        "region" => "Test Region".to_string(),
    };
    let register_args_bytes = register_args
        .to_bytes()
        .expect("RuntimeArgs should serialize");
    let submit_register_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        multisig_hash,
        "submit_transaction",
        runtime_args! {
            "target_package_hash" => hub_package_key,
            "target_entry_point" => "register_community".to_string(),
            "target_args_bytes" => register_args_bytes,
        },
    )
    .build();
    builder.exec(submit_register_req).expect_success().commit();

    let execute_register_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        multisig_hash,
        "execute_transaction",
        runtime_args! { "tx_id" => 1u64 },
    )
    .build();
    builder.exec(execute_register_req).expect_success().commit();

    // Final check: risk-registry's own community_count() -- queried
    // directly, not through the Hub. Same return-value-reading
    // limitation as above applies (asserts call success only, not the
    // actual number) -- what this step demonstrates is that
    // risk-registry itself is still reachable and functioning
    // correctly after everything above, as an end-of-test sanity
    // check, not a numeric proof of exactly how many communities got
    // registered.
    let final_count_req = ExecuteRequestBuilder::contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        rr_hash,
        "community_count",
        runtime_args! {},
    )
    .build();
    builder.exec(final_count_req).expect_success().commit();
}
