---

**PROPRIETARY — NOT FOR USE WITHOUT PERMISSION**

Copyright (c) 2026 TAAD (The Abuja Algorithmic Defenders). All rights
reserved.

This document — and the specific software requirements, interfaces,
and design decisions it describes — may not be copied, distributed,
implemented, or otherwise used, in whole or in part, without prior
written permission from TAAD (The Abuja Algorithmic Defenders).

This notice applies **only to this document** and is distinct from,
and takes precedence over, the MIT License that otherwise governs the
D3R·AC repository's source code (see the top-level
[`LICENSE`](../LICENSE) and [`CONTRIBUTING.md`](../CONTRIBUTING.md)).
The rest of this repository remains MIT-licensed and open to
contribution unless TAAD states otherwise in writing.

To request permission to use this document or implement the
requirements it describes, contact TAAD (The Abuja Algorithmic
Defenders), Abuja, Nigeria.

---

# Casper Contracts — Software Requirements Specification

**Status:** Draft — no implementation exists yet.
**Component:** `contracts/casper/` (declared in the top-level
[`README.md`](../README.md)'s repository structure, not yet created).

## 1. Purpose

D3R·AC's TRON contract suite is complete and tested: seven contracts
(`D3RACToken`, `IdentityRegistry`, `DisbursementController`,
`MultiSigAdmin`, `D3RACHub`, `RiskRegistry`, `FundingRequestRegistry`
— see [`contracts/tron/README.md`](../contracts/tron/README.md)) with
a logic-tested suite passing on TRON/TVM. The top-level README
describes D3R·AC as deployed across **both** TRON and Casper; today
only the TRON half exists. The frontend already anticipates this —
[`frontend/src/lib/chainAdapter.ts`](../frontend/src/lib/chainAdapter.ts)
defines a chain-agnostic `ChainAdapter` interface, and
[`frontend/src/lib/casperAdapter.ts`](../frontend/src/lib/casperAdapter.ts)
is a working stub against it whose `getTokenBalance`/`disburse` methods
deliberately throw "not deployed yet" until a real Casper contract
suite exists to call.

This document specifies the Casper-side contract suite that closes
that gap: functional and interface parity with the TRON suite,
expressed in Casper's contract model (WASM via Rust, session code,
named keys, dictionaries) rather than Solidity's, and matched to the
exact surface `casperAdapter.ts` already commits to.

## 2. Scope

**In scope:** a Casper contract package per TRON contract (see §4),
implementing equivalent state and access-control semantics; wiring
equivalent to `D3RACHub`'s five-module coordination
(`token`, `identityRegistry`, `disbursementController`, `riskRegistry`,
`fundingRequestRegistry`); an on-chain event mechanism usable by the
frontend and by the not-yet-built data pipeline (see
[`docs/data-pipeline-srs.md`](data-pipeline-srs.md), which targets
TRON's `RiskRegistry.updateRisk` today and will need a Casper
equivalent once this suite exists — tracked as an open decision in
§8, not solved here); replacing `casperAdapter.ts`'s two throwing
stubs (`getTokenBalance`, `disburse`) with real `casper-js-sdk` calls
against the deployed contracts, with no changes to `chainAdapter.ts`'s
interface or to any page component.

**Out of scope:** any change to the TRON contracts, which this suite
must match behaviorally but does not extend or depend on; mainnet
deployment (testnet — Casper Testnet — first, matching the TRON side's
own Shasta/Nile-first posture in
[`docs/deployment-guide.md`](deployment-guide.md)); the data pipeline
itself; a security audit (the TRON suite is explicitly unaudited per
the top-level README, and this suite inherits the same status until
one happens).

## 3. Definitions

| Term | Meaning |
|---|---|
| Contract package | Casper's unit of upgradeable on-chain code — a package hash with one or more versioned contract hashes under it, the Casper analog to an upgradeable proxy pattern |
| Session code | Client-side WASM executed as part of a deploy, used here to invoke contract entry points with the caller's own context |
| Named keys | A contract's on-chain key-value directory (its rough analog to Solidity storage variables declared `public`) |
| Dictionary | Casper's on-chain mapping type, used for indexed lookups too large to enumerate as named keys (e.g. per-community risk records, per-account balances) |
| Deploy | A signed unit of execution submitted to the network — Casper's analog to an Ethereum/TRON transaction |
| CEP-18 | The Casper community token standard, the closest analog to TRC-20; this suite's token contract must implement it so wallets and explorers recognize it as a standard token |
| Public key / account hash | A Casper account's two addressable forms — signing key and its derived on-chain hash — either of which may need to appear in access-control checks, unlike TRON/EVM's single-address model |

## 4. Functional requirements

**FR-1 — Token contract (CEP-18 parity with `D3RACToken.sol`).** Must
implement the CEP-18 standard entry points (`transfer`, `approve`,
`transfer_from`, `balance_of`, `allowance`, `total_supply`) plus the
non-standard owner-gated `mint` / `set_minter` pair `D3RACToken.sol`
adds on top of TRC-20. `casperAdapter.ts`'s `getTokenBalance` must be
implementable as a single `balance_of` query returning the same
`TokenBalance` shape (`symbol`, human-readable `amount`, raw integer
`amount`) the TRON adapter already returns.

**FR-2 — Identity registry parity with `IdentityRegistry.sol`.** An
admin-designated `verifiers` role must be able to verify a recipient
account (by public key or account hash) against a community label, and
revoke that verification, with the same admin-transfer semantics
(`transferAdmin`) as the TRON contract.

**FR-3 — Disbursement controller parity with
`DisbursementController.sol`.** Commitment creation (recipient, token,
community, milestone descriptions, milestone amounts), per-milestone
attestation by an `attesters` role, and permissionless release once
attested, must all be reproduced — including that release itself
stays callable by anyone once the attestation gate is satisfied,
matching the TRON contract's explicit design choice (the attestation
is the real gate, not who submits the deploy).

**FR-4 — Multisig admin parity with `MultiSigAdmin.sol`.** An N-of-M
multisig contract package capable of holding admin/owner status on
every other contract in this suite, generic enough (arbitrary
target/entry-point/args, not just this suite's own ABI) to match the
TRON contract's unconstrained design.

**FR-5 — Hub parity with `D3RACHub.sol`.** A single coordinator
contract wired to the four contracts above plus the two below,
reproducing: one admin surface for both day-to-day operational calls
and role/ownership management; one `pause`/`unpause` gate over the
fund/data-moving operations only (verify, create-commitment, attest,
mint, register-community, update-risk, open-funding-request), leaving
role and module-pointer management callable while paused, exactly as
`D3RACHub.sol` documents; and one aggregate status query matching
`systemStatus()`'s ten-value return, adapted to whatever equivalent
Casper offers for a multi-value entry-point response.

**FR-6 — Risk registry parity with `RiskRegistry.sol`.** The same
deterministic on-chain scoring, `R(c,t) = H(t)·E(c)·V(c)`, at the same
1e18 fixed-point scale as the TRON contract and
[`docs/risk-model.md`](risk-model.md), keyed by the same community
identifier scheme, with a `dataFeeders` role restricted to pushing
updates and a threshold-crossing signal equivalent to
`ThresholdCrossed` (see §8 on Casper's event mechanism). Must remain
standalone — no dependency on any other contract in this suite, same
as the TRON original.

**FR-7 — Funding request registry parity with
`FundingRequestRegistry.sol`.** Request open/close, pledge recording
against off-chain-sourced amounts with a citation URI, linking a
request to a `DisbursementController` commitment id, and a
`proposers` role restricted to opening requests — including the same
requester-or-owner gate on close/pledge/link that the TRON contract
uses, and the same caveat that only requests opened through the Hub
can be closed through it unless the Hub separately holds this
contract's owner role.

**FR-8 — Wiring parity.** The deployment sequence (deploy
`MultiSigAdmin` first, pass its address as the constructor admin/owner
argument to the others, then transfer or grant each contract's
admin/owner role to the Hub per contract) must be reproduced and
documented the way `contracts/tron/README.md`'s "Wiring the Hub"
section documents it for TRON, including the same additive-vs-exclusive
distinction (e.g. `updateRisk` needs only a data-feeder grant;
`registerCommunity`/`setRiskThreshold` need the Hub to hold
`RiskRegistry`'s exclusive owner role).

**FR-9 — Frontend adapter completion.** `casperAdapter.ts`'s
`getTokenBalance` and `disburse` must be implemented against the
deployed contract package hashes using `casper-js-sdk`, with no change
to `chainAdapter.ts`'s interface and no change to any page component —
the entire point of the existing adapter abstraction.

## 5. Non-functional requirements

**NFR-1 — Key custody.** Any admin/multisig-signer key for this suite
is a real operational secret with the same blast radius as its TRON
equivalent (able to mint, re-point Hub modules, or drain a
role-gated function) — must not be stored in plaintext in application
config, source control, or logs, matching NFR-1 in
[`docs/data-pipeline-srs.md`](data-pipeline-srs.md#5-non-functional-requirements).

**NFR-2 — Upgradeability decision is explicit, not accidental.**
Casper's contract-package model makes upgradeable contracts the
path of least resistance, unlike the TRON suite's immutable-by-default
Solidity contracts. Whichever choice is made per contract (upgradeable
package vs. locked/immutable package) must be a deliberate, documented
decision — not left to whatever `casper-client` defaults to — because
it changes the suite's trust model relative to its TRON counterpart.

**NFR-3 — Auditability.** Every state-changing entry point must emit
something an off-chain observer (the frontend, a future data pipeline,
a block explorer) can read without needing the calling account's own
records — matching the TRON suite's "every state change is an event"
property, expressed via whatever mechanism §8 resolves.

**NFR-4 — Gas/payment estimation.** Casper deploys require an
explicit payment amount set by the caller, unlike TRON/EVM's
gas-estimation-then-execute flow. The frontend and any deployment
tooling must have a documented, tested payment-amount figure (or
estimation method) per entry point, or callers will intermittently
fail with `OutOfGas` on correct, well-formed calls.

## 6. Interfaces this suite must match

Already implemented and tested on TRON (115/115 passing, see
[`contracts/tron/README.md`](../contracts/tron/README.md)) — this
suite is a re-implementation targeting behavioral parity, not a
consumer or a fork:

```solidity
// D3RACHub.sol — the coordination surface this suite's Hub must mirror
function verifyRecipient(address recipient, string calldata community) external;
function createCommitment(address recipient, address token, string calldata community,
    string[] calldata descriptions, uint256[] calldata amounts) external returns (uint256);
function attestMilestone(uint256 commitmentId, uint256 milestoneIndex) external;
function mintTokens(address to, uint256 value) external;
function registerCommunity(bytes32 communityId, string calldata name_, string calldata region) external;
function updateRisk(bytes32 communityId, uint256 hazard, uint256 exposure, uint256 vulnerability) external;
function openFundingRequest(bytes32 communityId, uint256 amountRequested,
    string calldata description, string calldata dataSourceURI) external returns (uint256);
function systemStatus() external view returns (address, address, address, address, address,
    bool, uint256, uint256, uint256, uint256);
```

```typescript
// frontend/src/lib/chainAdapter.ts — the frontend surface this suite's
// adapter must satisfy (already defined, not to be modified)
interface ChainAdapter {
  connect(): Promise<string>;
  getAddress(): string | null;
  getTokenBalance(tokenContract: string): Promise<TokenBalance>;
  disburse(params: { tokenContract: string; to: string; amount: string }): Promise<DisbursementResult>;
  explorerAddressUrl(address: string): string;
}
```

## 7. Acceptance criteria

- [ ] Every FR-1 through FR-8 behavior has a passing test against a
      local Casper network (e.g. `casper-node` in standalone/NCTL
      mode), mirroring the TRON suite's Hardhat test coverage.
- [ ] `casperAdapter.ts`'s `getTokenBalance` and `disburse` no longer
      throw "not deployed yet" and return values matching
      `chainAdapter.ts`'s existing types, with zero changes to that
      file or to any page component (FR-9).
- [ ] The Hub-wiring sequence in the eventual
      `contracts/casper/README.md` produces a Hub capable of every
      write path described in FR-5, verified the same way
      `contracts/tron/README.md` verifies TRON's Hub wiring.
- [ ] A risk update that crosses θ on Casper produces an observable
      signal equivalent to TRON's `ThresholdCrossed`, resolvable by
      whatever mechanism §8 selects.
- [ ] No admin/multisig-signer key appears in plaintext in logs,
      committed config, or error messages (NFR-1).
- [ ] The upgradeable-vs-locked choice for each contract package is
      written down in `contracts/casper/README.md`, not left implicit
      (NFR-2).

## 8. Open decisions

These need someone with Casper-specific engineering experience, not
a straight port of TRON decisions:

- **Event mechanism.** Casper has no native EVM-style event log;
  candidates include the community Casper Event Standard (CES) or a
  dictionary-based emulation. Whichever is chosen has to be readable
  by both the frontend and the future data pipeline (see
  [`docs/data-pipeline-srs.md`](data-pipeline-srs.md), which currently
  only specifies against TRON's `RiskRegistry.updateRisk`/
  `ThresholdCrossed` and will need a Casper-side amendment once this
  is resolved).
- **Upgradeability posture per contract** (NFR-2) — locked package
  hash (closest analog to TRON's immutability) vs. upgradeable
  package, and if upgradeable, who holds the upgrade key.
- **`casper-client` / `casper-js-sdk` version pinning** and which
  Casper Testnet endpoint the frontend and any deployment scripts
  target by default.
- **Payment amount figures** (NFR-4) for each entry point — these are
  determined empirically per contract and don't have a TRON-side
  equivalent to carry over.
- **Account-hash vs. public-key addressing** in every role list
  (verifiers, attesters, data feeders, proposers, minters) — Casper
  exposes both forms for an account, and the TRON suite's single-address
  model doesn't dictate which this suite should standardize on.
