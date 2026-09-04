//! Named-key names, entry-point names, and arg names -- string
//! constants collected here so a typo is a single-definition-site
//! compile-time fact, not a copy-pasted literal that can silently
//! diverge between `call()`, an entry point, and a helper. Same
//! convention as every other contract in this suite.

pub const KEY_OWNER: &str = "owner";
pub const KEY_PENDING_OWNER: &str = "pending_owner";
pub const KEY_PROPOSERS_DICT: &str = "proposers";
pub const KEY_REQUESTS_DICT: &str = "requests";
pub const KEY_REQUEST_COUNT: &str = "request_count";

pub const PACKAGE_HASH_KEY_NAME: &str = "funding_request_registry_package_hash";
pub const ACCESS_UREF_KEY_NAME: &str = "funding_request_registry_access_uref";
pub const CONTRACT_HASH_KEY_NAME: &str = "funding_request_registry_contract_hash";

pub const ENTRY_POINT_OPEN_REQUEST: &str = "open_request";
pub const ENTRY_POINT_RECORD_PLEDGE: &str = "record_pledge";
pub const ENTRY_POINT_LINK_TO_COMMITMENT: &str = "link_to_commitment";
pub const ENTRY_POINT_CLOSE_REQUEST: &str = "close_request";
pub const ENTRY_POINT_REQUEST_COUNT: &str = "request_count";
pub const ENTRY_POINT_GET_REQUEST: &str = "get_request";
pub const ENTRY_POINT_ADD_PROPOSER: &str = "add_proposer";
pub const ENTRY_POINT_REMOVE_PROPOSER: &str = "remove_proposer";
pub const ENTRY_POINT_IS_PROPOSER: &str = "is_proposer";
pub const ENTRY_POINT_PROPOSE_NEW_OWNER: &str = "propose_new_owner";
pub const ENTRY_POINT_ACCEPT_OWNERSHIP: &str = "accept_ownership";
pub const ENTRY_POINT_INIT: &str = "init";

pub const ARG_INITIAL_PROPOSER: &str = "initial_proposer";
pub const ARG_COMMUNITY_ID: &str = "community_id";
pub const ARG_AMOUNT_REQUESTED: &str = "amount_requested";
pub const ARG_DESCRIPTION: &str = "description";
pub const ARG_DATA_SOURCE_URI: &str = "data_source_uri";
pub const ARG_REQUEST_ID: &str = "request_id";
pub const ARG_AMOUNT: &str = "amount";
pub const ARG_PLEDGE_SOURCE_URI: &str = "pledge_source_uri";
pub const ARG_COMMITMENT_ID: &str = "commitment_id";
pub const ARG_PROPOSER: &str = "proposer";
pub const ARG_NEW_OWNER: &str = "new_owner";
