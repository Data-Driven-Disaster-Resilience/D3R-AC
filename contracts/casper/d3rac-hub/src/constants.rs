//! Named-key, entry-point, and arg-name string constants -- see
//! risk-registry/src/constants.rs's header for why these are collected
//! here rather than inlined. This file additionally collects the
//! entry-point-name constants for every OTHER contract in this suite
//! that the Hub calls into via `runtime::call_contract` (each callee's
//! own `pub const ENTRY_POINT_X` lives in its own crate and isn't
//! importable across these separately-compiled wasm binaries -- same
//! reasoning `disbursement-controller`'s own local
//! `IDENTITY_REGISTRY_ENTRY_POINT_IS_VERIFIED` constant already
//! established, extended here to every callee the Hub uses).

pub const KEY_ADMIN: &str = "admin";
pub const KEY_PENDING_ADMIN: &str = "pending_admin";
pub const KEY_PAUSED: &str = "paused";
pub const KEY_TOKEN: &str = "token";
pub const KEY_IDENTITY_REGISTRY: &str = "identity_registry";
pub const KEY_DISBURSEMENT_CONTROLLER: &str = "disbursement_controller";
/// `Option<Key>` -- `None` means "not configured", the Casper-native
/// equivalent of `FundingRequestRegistry`/`RiskRegistry` being
/// `address(0)` on the TRON side. See main.rs's header for why an
/// `Option` rather than attempting a "zero Key" sentinel.
pub const KEY_RISK_REGISTRY: &str = "risk_registry";
pub const KEY_FUNDING_REQUEST_REGISTRY: &str = "funding_request_registry";

pub const PACKAGE_HASH_KEY_NAME: &str = "d3rac_hub_package_hash";
pub const ACCESS_UREF_KEY_NAME: &str = "d3rac_hub_access_uref";
pub const CONTRACT_HASH_KEY_NAME: &str = "d3rac_hub_contract_hash";

// --- This contract's own entry points ---
pub const ENTRY_POINT_PROPOSE_NEW_ADMIN: &str = "propose_new_admin";
pub const ENTRY_POINT_ACCEPT_ADMIN: &str = "accept_admin";
pub const ENTRY_POINT_SET_TOKEN: &str = "set_token";
pub const ENTRY_POINT_SET_IDENTITY_REGISTRY: &str = "set_identity_registry";
pub const ENTRY_POINT_SET_DISBURSEMENT_CONTROLLER: &str = "set_disbursement_controller";
pub const ENTRY_POINT_SET_RISK_REGISTRY: &str = "set_risk_registry";
pub const ENTRY_POINT_SET_FUNDING_REQUEST_REGISTRY: &str = "set_funding_request_registry";
pub const ENTRY_POINT_PAUSE: &str = "pause";
pub const ENTRY_POINT_UNPAUSE: &str = "unpause";
pub const ENTRY_POINT_VERIFY_RECIPIENT: &str = "verify_recipient";
pub const ENTRY_POINT_CREATE_COMMITMENT: &str = "create_commitment";
pub const ENTRY_POINT_ATTEST_MILESTONE: &str = "attest_milestone";
pub const ENTRY_POINT_CANCEL_COMMITMENT: &str = "cancel_commitment";
pub const ENTRY_POINT_MINT_TOKENS: &str = "mint_tokens";
pub const ENTRY_POINT_REGISTER_COMMUNITY: &str = "register_community";
pub const ENTRY_POINT_UPDATE_RISK: &str = "update_risk";
pub const ENTRY_POINT_OPEN_FUNDING_REQUEST: &str = "open_funding_request";
pub const ENTRY_POINT_CLOSE_FUNDING_REQUEST: &str = "close_funding_request";
pub const ENTRY_POINT_SET_IDENTITY_VERIFIER: &str = "set_identity_verifier";
pub const ENTRY_POINT_PROPOSE_IDENTITY_REGISTRY_ADMIN: &str = "propose_identity_registry_admin";
pub const ENTRY_POINT_ACCEPT_IDENTITY_REGISTRY_ADMIN: &str = "accept_identity_registry_admin";
pub const ENTRY_POINT_REVOKE_RECIPIENT: &str = "revoke_recipient";
pub const ENTRY_POINT_SET_DISBURSEMENT_ATTESTER: &str = "set_disbursement_attester";
pub const ENTRY_POINT_PROPOSE_DISBURSEMENT_CONTROLLER_ADMIN: &str =
    "propose_disbursement_controller_admin";
pub const ENTRY_POINT_ACCEPT_DISBURSEMENT_CONTROLLER_ADMIN: &str =
    "accept_disbursement_controller_admin";
pub const ENTRY_POINT_SET_TOKEN_MINTER: &str = "set_token_minter";
pub const ENTRY_POINT_PROPOSE_TOKEN_OWNERSHIP: &str = "propose_token_ownership";
pub const ENTRY_POINT_ACCEPT_TOKEN_OWNERSHIP: &str = "accept_token_ownership";
pub const ENTRY_POINT_SET_RISK_DATA_FEEDER: &str = "set_risk_data_feeder";
pub const ENTRY_POINT_SET_RISK_THRESHOLD: &str = "set_risk_threshold";
pub const ENTRY_POINT_TRANSFER_RISK_REGISTRY_OWNERSHIP: &str = "transfer_risk_registry_ownership";
pub const ENTRY_POINT_SET_FUNDING_PROPOSER: &str = "set_funding_proposer";
pub const ENTRY_POINT_RECORD_FUNDING_PLEDGE: &str = "record_funding_pledge";
pub const ENTRY_POINT_LINK_FUNDING_REQUEST_TO_COMMITMENT: &str =
    "link_funding_request_to_commitment";
pub const ENTRY_POINT_PROPOSE_FUNDING_REQUEST_REGISTRY_OWNERSHIP: &str =
    "propose_funding_request_registry_ownership";
pub const ENTRY_POINT_ACCEPT_FUNDING_REQUEST_REGISTRY_OWNERSHIP: &str =
    "accept_funding_request_registry_ownership";
pub const ENTRY_POINT_SYSTEM_STATUS: &str = "system_status";
pub const ENTRY_POINT_INIT: &str = "init";

// --- identity-registry's entry points (callee-side names) ---
pub const IDENTITY_REGISTRY_ENTRY_POINT_VERIFY_RECIPIENT: &str = "verify_recipient";
pub const IDENTITY_REGISTRY_ENTRY_POINT_REVOKE_RECIPIENT: &str = "revoke_recipient";
pub const IDENTITY_REGISTRY_ENTRY_POINT_SET_VERIFIER: &str = "set_verifier";
pub const IDENTITY_REGISTRY_ENTRY_POINT_PROPOSE_NEW_ADMIN: &str = "propose_new_admin";
pub const IDENTITY_REGISTRY_ENTRY_POINT_ACCEPT_ADMIN: &str = "accept_admin";

// --- disbursement-controller's entry points ---
pub const DISBURSEMENT_CONTROLLER_ENTRY_POINT_CREATE_COMMITMENT: &str = "create_commitment";
pub const DISBURSEMENT_CONTROLLER_ENTRY_POINT_ATTEST_MILESTONE: &str = "attest_milestone";
pub const DISBURSEMENT_CONTROLLER_ENTRY_POINT_CANCEL_COMMITMENT: &str = "cancel_commitment";
pub const DISBURSEMENT_CONTROLLER_ENTRY_POINT_SET_ATTESTER: &str = "set_attester";
pub const DISBURSEMENT_CONTROLLER_ENTRY_POINT_PROPOSE_NEW_ADMIN: &str = "propose_new_admin";
pub const DISBURSEMENT_CONTROLLER_ENTRY_POINT_ACCEPT_ADMIN: &str = "accept_admin";

// --- d3rac-token's entry points ---
pub const TOKEN_ENTRY_POINT_MINT: &str = "mint";
pub const TOKEN_ENTRY_POINT_SET_MINTER: &str = "set_minter";
pub const TOKEN_ENTRY_POINT_PROPOSE_NEW_OWNER: &str = "propose_new_owner";
pub const TOKEN_ENTRY_POINT_ACCEPT_OWNERSHIP: &str = "accept_ownership";
pub const TOKEN_ENTRY_POINT_TOTAL_SUPPLY: &str = "total_supply";

// --- risk-registry's entry points ---
// NOTE: risk-registry's own ownership transfer is single-step
// (`transfer_ownership`), NOT the two-step `proposeNewOwner`/
// `acceptOwnership` pair `D3RACHub.sol` was written against on TRON
// (`RiskRegistry.sol` itself IS two-step -- this is a gap in the
// existing Casper risk-registry port, not a TRON<->Casper design
// choice). Not fixed here -- out of scope for the Hub -- but the
// Hub's own `transfer_risk_registry_ownership` entry point (singular,
// not a propose/accept pair) reflects what risk-registry actually
// exposes today. See main.rs's header for the full explanation.
pub const RISK_REGISTRY_ENTRY_POINT_REGISTER_COMMUNITY: &str = "register_community";
pub const RISK_REGISTRY_ENTRY_POINT_UPDATE_RISK: &str = "update_risk";
pub const RISK_REGISTRY_ENTRY_POINT_COMMUNITY_COUNT: &str = "community_count";
pub const RISK_REGISTRY_ENTRY_POINT_ADD_DATA_FEEDER: &str = "add_data_feeder";
pub const RISK_REGISTRY_ENTRY_POINT_REMOVE_DATA_FEEDER: &str = "remove_data_feeder";
pub const RISK_REGISTRY_ENTRY_POINT_SET_RISK_THRESHOLD: &str = "set_risk_threshold";
pub const RISK_REGISTRY_ENTRY_POINT_TRANSFER_OWNERSHIP: &str = "transfer_ownership";

// --- funding-request-registry's entry points ---
pub const FRR_ENTRY_POINT_OPEN_REQUEST: &str = "open_request";
pub const FRR_ENTRY_POINT_CLOSE_REQUEST: &str = "close_request";
pub const FRR_ENTRY_POINT_REQUEST_COUNT: &str = "request_count";
pub const FRR_ENTRY_POINT_ADD_PROPOSER: &str = "add_proposer";
pub const FRR_ENTRY_POINT_REMOVE_PROPOSER: &str = "remove_proposer";
pub const FRR_ENTRY_POINT_RECORD_PLEDGE: &str = "record_pledge";
pub const FRR_ENTRY_POINT_LINK_TO_COMMITMENT: &str = "link_to_commitment";
pub const FRR_ENTRY_POINT_PROPOSE_NEW_OWNER: &str = "propose_new_owner";
pub const FRR_ENTRY_POINT_ACCEPT_OWNERSHIP: &str = "accept_ownership";

// --- Args this contract's own entry points take ---
pub const ARG_ADMIN: &str = "admin_";
pub const ARG_TOKEN: &str = "token_";
pub const ARG_IDENTITY_REGISTRY: &str = "identity_registry_";
pub const ARG_DISBURSEMENT_CONTROLLER: &str = "disbursement_controller_";
pub const ARG_RISK_REGISTRY: &str = "risk_registry_";
pub const ARG_FUNDING_REQUEST_REGISTRY: &str = "funding_request_registry_";
pub const ARG_NEW_ADMIN: &str = "new_admin";
pub const ARG_NEW_TOKEN: &str = "new_token";
pub const ARG_NEW_REGISTRY: &str = "new_registry";
pub const ARG_NEW_CONTROLLER: &str = "new_controller";
pub const ARG_NEW_RISK_REGISTRY: &str = "new_risk_registry";
pub const ARG_NEW_FUNDING_REQUEST_REGISTRY: &str = "new_funding_request_registry";
pub const ARG_RECIPIENT: &str = "recipient";
pub const ARG_COMMITMENT_TOKEN: &str = "commitment_token";
pub const ARG_COMMUNITY: &str = "community";
pub const ARG_DESCRIPTIONS: &str = "descriptions";
pub const ARG_AMOUNTS: &str = "amounts";
pub const ARG_COMMITMENT_ID: &str = "commitment_id";
pub const ARG_MILESTONE_INDEX: &str = "milestone_index";
pub const ARG_TO: &str = "to";
pub const ARG_VALUE: &str = "value";
pub const ARG_COMMUNITY_ID: &str = "community_id";
pub const ARG_NAME: &str = "name";
pub const ARG_REGION: &str = "region";
pub const ARG_HAZARD: &str = "hazard";
pub const ARG_EXPOSURE: &str = "exposure";
pub const ARG_VULNERABILITY: &str = "vulnerability";
pub const ARG_AMOUNT_REQUESTED: &str = "amount_requested";
pub const ARG_DESCRIPTION: &str = "description";
pub const ARG_DATA_SOURCE_URI: &str = "data_source_uri";
pub const ARG_REQUEST_ID: &str = "request_id";
pub const ARG_ACCOUNT: &str = "account";
pub const ARG_IS_VERIFIER: &str = "is_verifier";
pub const ARG_IS_ATTESTER: &str = "is_attester";
pub const ARG_CAN_MINT: &str = "can_mint";
pub const ARG_IS_FEEDER: &str = "is_feeder";
pub const ARG_NEW_THRESHOLD: &str = "new_threshold";
pub const ARG_NEW_OWNER: &str = "new_owner";
pub const ARG_IS_PROPOSER: &str = "is_proposer";
pub const ARG_AMOUNT: &str = "amount";
pub const ARG_PLEDGE_SOURCE_URI: &str = "pledge_source_uri";
