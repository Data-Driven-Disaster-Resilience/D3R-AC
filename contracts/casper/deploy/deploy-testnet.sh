#!/usr/bin/env bash
# Deploys and wires all seven D3R·AC Casper contracts to testnet.
#
# STATUS: written, never run. No Casper network access and no local
# casper-client exist in the sandbox that wrote this -- every
# casper-client invocation below is sourced from a specific
# https://docs.casper.network page (linked per command), not verified
# by execution. The WIRING SEQUENCE itself (which contract gets which
# role, in what order, ending in a 1-of-1 multisig admin handoff) IS
# proven correct -- it's a direct translation of
# contracts/casper/d3rac-hub-tests/tests/integration_tests.rs, which
# real CI confirms passes against a local Casper network. What's
# unverified here is purely the casper-client CLI syntax layer on top
# of that already-correct sequence.
#
# casper-client's CLI is mid-transition: `put-deploy` is deprecated
# since casper-client 3.0.0 in favor of `put-transaction`/`put-txn`
# (https://docs.casper.network/concepts/transactions). This script
# uses the newer, non-deprecated form throughout.
#
# Treat the first real run of this script as its actual test, not as
# a routine deployment. Recommended: run it once by hand against
# Shasta -- sorry, against Casper testnet -- reading each step's
# output before letting it proceed unattended, rather than trusting
# a single unattended CI run the first time.
#
# Required environment variables (set by deploy-casper-testnet.yml):
#   CASPER_SECRET_KEY_PATH  -- path to a PEM secret key file
#   CASPER_NODE_ADDRESS     -- e.g. https://node.testnet.casper.network
#   CASPER_CHAIN_NAME       -- casper-test

set -euo pipefail

NODE="${CASPER_NODE_ADDRESS:?CASPER_NODE_ADDRESS not set}"
CHAIN="${CASPER_CHAIN_NAME:?CASPER_CHAIN_NAME not set}"
KEY="${CASPER_SECRET_KEY_PATH:?CASPER_SECRET_KEY_PATH not set}"
WASM_DIR="target/wasm32-unknown-unknown/release"

# Same "no confirmed way to fund a fresh account" reasoning as
# d3rac-hub-tests: this script deploys and wires everything under a
# SINGLE deployer key throughout, then hands the Hub's admin to a
# 1-of-1 multisig whose sole owner is that same key -- the same
# topology docs/deployment-guide.md documents as the real Shasta
# deployment's own approach, not a script-only simplification.
DEPLOYER_ACCOUNT_HASH=$(casper-client account-address --secret-key "$KEY" | sed 's/^account-hash-//')
echo "Deployer account hash: $DEPLOYER_ACCOUNT_HASH"

# ---------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------

# Source: https://docs.casper.network/developers/cli/installing-contracts
# (put-transaction session / put-txn session)
install_contract() {
  local wasm_file="$1"
  shift
  local args=("$@")
  echo "Installing $wasm_file..."
  local result
  result=$(casper-client put-txn session \
    --node-address "$NODE" \
    --chain-name "$CHAIN" \
    --secret-key "$KEY" \
    --pricing-mode fixed \
    --gas-price-tolerance 3 \
    --transaction-path "$WASM_DIR/$wasm_file" \
    --session-entry-point call \
    --category install-upgrade \
    "${args[@]}")
  echo "$result"
  local txn_hash
  txn_hash=$(echo "$result" | jq -r '.result.transaction_hash // .result.deploy_hash // empty')
  if [ -z "$txn_hash" ]; then
    echo "::error::Could not extract a transaction hash from put-txn's response for $wasm_file"
    exit 1
  fi
  wait_for_transaction "$txn_hash"
}

# Source: https://docs.casper.network/concepts/transactions
# (put-txn invocable-entity)
call_entry_point() {
  local entity_hash="$1"
  local entry_point="$2"
  shift 2
  local args=("$@")
  echo "Calling $entry_point on entity-hash-$entity_hash..."
  local result
  result=$(casper-client put-txn invocable-entity \
    --node-address "$NODE" \
    --chain-name "$CHAIN" \
    --secret-key "$KEY" \
    --pricing-mode fixed \
    --gas-price-tolerance 3 \
    --entity-address "entity-contract-$entity_hash" \
    --session-entry-point "$entry_point" \
    "${args[@]}")
  echo "$result"
  local txn_hash
  txn_hash=$(echo "$result" | jq -r '.result.transaction_hash // .result.deploy_hash // empty')
  if [ -z "$txn_hash" ]; then
    echo "::error::Could not extract a transaction hash from put-txn's response for $entry_point"
    exit 1
  fi
  wait_for_transaction "$txn_hash"
}

# There is no single, universally-documented "wait for inclusion"
# casper-client subcommand across CLI versions (some have
# `get-deploy`/`get-transaction` with a --finalized-approvals-style
# poll; this just polls get-transaction until it stops erroring, a
# deliberately simple approach given the uncertainty). Source pattern:
# https://docs.casper.network/developers/cli/calling-contracts
# ("verify a deploy" via get-deploy).
wait_for_transaction() {
  local txn_hash="$1"
  echo "Waiting for transaction $txn_hash..."
  for _ in $(seq 1 30); do
    if casper-client get-transaction --node-address "$NODE" "$txn_hash" >/tmp/txn-result.json 2>/dev/null; then
      if jq -e '.result.execution_info.execution_result' /tmp/txn-result.json >/dev/null 2>&1; then
        echo "Transaction $txn_hash included."
        return 0
      fi
    fi
    sleep 5
  done
  echo "::error::Timed out waiting for transaction $txn_hash to be included"
  exit 1
}

# Source: https://docs.casper.network/developers/cli/querying-global-state
read_named_key() {
  local key_name="$1"
  local state_root_hash
  state_root_hash=$(casper-client get-state-root-hash --node-address "$NODE" | jq -r '.result.state_root_hash')
  casper-client query-global-state \
    --node-address "$NODE" \
    --state-root-hash "$state_root_hash" \
    --key "account-hash-$DEPLOYER_ACCOUNT_HASH" \
    -q "$key_name" \
    | jq -r '.result.stored_value.CLValue.parsed'
}

# Named keys are stored as e.g. "entity-contract-<hash>" or
# "package-<hash>" strings once parsed -- strips the prefix.
strip_hash_prefix() {
  echo "$1" | sed -E 's/^(entity-contract-|contract-package-wasmV1-|package-|hash-)//'
}

# ---------------------------------------------------------------
# 1. Install the five underlying modules (each defaults owner/admin
#    to the installing account -- see funding-request-registry/src/
#    main.rs's call() etc. for confirmation; d3rac-token needs an
#    explicit owner_ arg instead, same as d3rac-hub-tests' own note).
# ---------------------------------------------------------------

install_contract "risk-registry.wasm" \
  --session-arg "initial_threshold:u64='0'" \
  --session-arg "initial_data_feeder:key=null"

install_contract "identity-registry.wasm" \
  --session-arg "initial_verifier:key=null"

IDENTITY_REGISTRY_CONTRACT_RAW=$(read_named_key "identity_registry_contract_hash")
IDENTITY_REGISTRY_CONTRACT=$(strip_hash_prefix "$IDENTITY_REGISTRY_CONTRACT_RAW")
echo "identity-registry contract hash: $IDENTITY_REGISTRY_CONTRACT"

install_contract "disbursement-controller.wasm" \
  --session-arg "registry_hash:key='entity-contract-$IDENTITY_REGISTRY_CONTRACT'"

install_contract "d3rac-token.wasm" \
  --session-arg "initial_supply:u256='0'" \
  --session-arg "owner_:key='account-hash-$DEPLOYER_ACCOUNT_HASH'"

install_contract "funding-request-registry.wasm" \
  --session-arg "initial_proposer:key=null"

RISK_REGISTRY_CONTRACT=$(strip_hash_prefix "$(read_named_key "risk_registry_contract_hash")")
DISBURSEMENT_CONTROLLER_CONTRACT=$(strip_hash_prefix "$(read_named_key "disbursement_controller_contract_hash")")
TOKEN_CONTRACT=$(strip_hash_prefix "$(read_named_key "d3rac_token_contract_hash")")
FRR_CONTRACT=$(strip_hash_prefix "$(read_named_key "funding_request_registry_contract_hash")")

echo "risk-registry: $RISK_REGISTRY_CONTRACT"
echo "disbursement-controller: $DISBURSEMENT_CONTROLLER_CONTRACT"
echo "d3rac-token: $TOKEN_CONTRACT"
echo "funding-request-registry: $FRR_CONTRACT"

# ---------------------------------------------------------------
# 2. Install the Hub, pointing at all five modules' CONTRACT hashes
#    up front -- NOT package hashes, see
#    d3rac-hub-tests/tests/integration_tests.rs's own hard-won note
#    on exactly this distinction (a real CI failure caught it there).
# ---------------------------------------------------------------

install_contract "d3rac-hub.wasm" \
  --session-arg "admin_:key='account-hash-$DEPLOYER_ACCOUNT_HASH'" \
  --session-arg "token_:key='entity-contract-$TOKEN_CONTRACT'" \
  --session-arg "identity_registry_:key='entity-contract-$IDENTITY_REGISTRY_CONTRACT'" \
  --session-arg "disbursement_controller_:key='entity-contract-$DISBURSEMENT_CONTROLLER_CONTRACT'" \
  --session-arg "risk_registry_:key='entity-contract-$RISK_REGISTRY_CONTRACT'" \
  --session-arg "funding_request_registry_:key='entity-contract-$FRR_CONTRACT'"

HUB_CONTRACT=$(strip_hash_prefix "$(read_named_key "d3rac_hub_contract_hash")")
HUB_PACKAGE=$(strip_hash_prefix "$(read_named_key "d3rac_hub_package_hash")")
echo "d3rac-hub contract hash: $HUB_CONTRACT"
echo "d3rac-hub package hash: $HUB_PACKAGE"

# ---------------------------------------------------------------
# 3. Wire each module to the Hub. Sequence matches
#    d3rac-hub-tests/tests/integration_tests.rs exactly (CI-proven);
#    only the casper-client invocation syntax is new here.
#    risk-registry: single-step, no accept phase -- see
#    d3rac-hub/src/main.rs's own header for why.
# ---------------------------------------------------------------

call_entry_point "$IDENTITY_REGISTRY_CONTRACT" "set_verifier" \
  --session-arg "account:key='entity-contract-package-$HUB_PACKAGE'" \
  --session-arg "is_verifier:bool='true'"
call_entry_point "$IDENTITY_REGISTRY_CONTRACT" "propose_new_admin" \
  --session-arg "new_admin:key='entity-contract-package-$HUB_PACKAGE'"
call_entry_point "$HUB_CONTRACT" "accept_identity_registry_admin"

call_entry_point "$DISBURSEMENT_CONTROLLER_CONTRACT" "set_attester" \
  --session-arg "account:key='entity-contract-package-$HUB_PACKAGE'" \
  --session-arg "is_attester:bool='true'"
call_entry_point "$DISBURSEMENT_CONTROLLER_CONTRACT" "propose_new_admin" \
  --session-arg "new_admin:key='entity-contract-package-$HUB_PACKAGE'"
call_entry_point "$HUB_CONTRACT" "accept_disbursement_controller_admin"

call_entry_point "$TOKEN_CONTRACT" "set_minter" \
  --session-arg "account:key='entity-contract-package-$HUB_PACKAGE'" \
  --session-arg "is_minter:bool='true'"
call_entry_point "$TOKEN_CONTRACT" "propose_new_owner" \
  --session-arg "new_owner:key='entity-contract-package-$HUB_PACKAGE'"
call_entry_point "$HUB_CONTRACT" "accept_token_ownership"

call_entry_point "$RISK_REGISTRY_CONTRACT" "add_data_feeder" \
  --session-arg "feeder:key='entity-contract-package-$HUB_PACKAGE'"
call_entry_point "$RISK_REGISTRY_CONTRACT" "transfer_ownership" \
  --session-arg "new_owner:key='entity-contract-package-$HUB_PACKAGE'"

call_entry_point "$FRR_CONTRACT" "add_proposer" \
  --session-arg "proposer:key='entity-contract-package-$HUB_PACKAGE'"
call_entry_point "$FRR_CONTRACT" "propose_new_owner" \
  --session-arg "new_owner:key='entity-contract-package-$HUB_PACKAGE'"
call_entry_point "$HUB_CONTRACT" "accept_funding_request_registry_ownership"

echo "All five modules wired to the Hub."

# ---------------------------------------------------------------
# 4. Install a 1-of-1 multisig-admin and hand the Hub's admin to it
#    -- same topology as the real Shasta deployment.
# ---------------------------------------------------------------

install_contract "multisig-admin.wasm" \
  --session-arg "owners:list='[Key::Account(account-hash-$DEPLOYER_ACCOUNT_HASH)]'" \
  --session-arg "threshold:u64='1'"

MULTISIG_CONTRACT=$(strip_hash_prefix "$(read_named_key "multisig_admin_contract_hash")")
MULTISIG_PACKAGE=$(strip_hash_prefix "$(read_named_key "multisig_admin_package_hash")")
echo "multisig-admin contract hash: $MULTISIG_CONTRACT"
echo "multisig-admin package hash: $MULTISIG_PACKAGE"

call_entry_point "$HUB_CONTRACT" "propose_new_admin" \
  --session-arg "new_admin:key='entity-contract-package-$MULTISIG_PACKAGE'"

# NOTE: completing the handoff (submit_transaction + execute_transaction
# through the multisig, calling accept_admin on the Hub) is
# deliberately NOT automated here, unlike d3rac-hub-tests' own test.
# That test could safely automate it because it runs against a local,
# throwaway network; doing so unattended against testnet, in the very
# first real run of this untested script, adds one more unverified
# step (RuntimeArgs::to_bytes() serialization fed through a shell
# variable) at exactly the point where the Hub's admin control
# actually changes hands. Run that step by hand once this script's
# install+wiring portion is confirmed working -- see
# docs/deployment-guide.md's Post-deployment section for the general
# pattern (TRON-flavored there, same idea here).

echo ""
echo "=== Deployment complete ==="
echo "d3rac-hub:                 $HUB_CONTRACT"
echo "risk-registry:             $RISK_REGISTRY_CONTRACT"
echo "identity-registry:         $IDENTITY_REGISTRY_CONTRACT"
echo "disbursement-controller:   $DISBURSEMENT_CONTROLLER_CONTRACT"
echo "d3rac-token:               $TOKEN_CONTRACT"
echo "funding-request-registry:  $FRR_CONTRACT"
echo "multisig-admin:            $MULTISIG_CONTRACT (admin transfer PROPOSED, not yet accepted -- see note above)"
