# TRON Contracts

## Layout

```
contracts/tron/
├── contracts/            # .sol sources (TronBox's and Hardhat's shared default)
│   ├── D3RACToken.sol
│   ├── IdentityRegistry.sol
│   ├── DisbursementController.sol
│   ├── MultiSigAdmin.sol
│   ├── D3RACHub.sol
│   ├── RiskRegistry.sol
│   └── FundingRequestRegistry.sol
├── test/                 # Hardhat/Mocha/Chai logic tests — see "Test suite" below
├── tronbox-config.js      # compile-only config (no network/private-key section —
│                           # add one before using `tronbox migrate` to deploy)
├── hardhat.config.js
└── package.json
```

The nested `contracts/` subfolder isn't optional — TronBox refuses to
treat the project root itself as `contracts_directory`, and it happens
to match Hardhat's own default sources path, so both tools find the
same files with no extra config.

## Current status

Seven contracts, dependency-free (no OpenZeppelin import — see each
file's header comment for why), compiled clean against solc 0.8.20
with the optimizer on:

- **`D3RACToken.sol`** — the TRC-20 relief-fund token. Implements the
  full standard surface, including the minimal slice the frontend
  (`frontend/src/lib/tronAdapter.ts`) already calls against
  (`balanceOf`, `decimals`, `symbol`, `transfer`), plus `approve` /
  `transferFrom` / `allowance` and an owner-gated `mint`/`setMinter` so a
  `DisbursementController` (or a treasury process) can be authorized to
  mint without opening minting to anyone.
- **`IdentityRegistry.sol`** — the wallet/identity layer. An admin
  designates verifiers, who verify recipient wallets (communities / NGO
  coordinators) with a human-readable label. This is the "who is allowed
  to receive relief funds at all" gate, separate from the milestone logic
  below.
- **`DisbursementController.sol`** — the milestone-release logic this
  file previously described as still-needed. A commitment is created for
  a recipient the `IdentityRegistry` has verified, split into milestones.
  Each milestone needs an `attester`-role attestation before its funds
  can be released; release itself is permissionless once attested (the
  attestation is the real gate, not who submits the transaction). Every
  state change — commitment created, milestone attested, milestone
  released, commitment cancelled — is an event.
- **`MultiSigAdmin.sol`** — an N-of-M multisig meant to *hold* the
  `admin`/`owner` role on the contracts below instead of a single
  EOA (the "consider a multisig for any contract-owner or admin role"
  item from `docs/deployment-guide.md`'s security checklist). Deploy it
  first and pass its address as the constructor's admin/owner argument
  on the others. Generic (submits arbitrary `to`/`value`/`data`), so
  it isn't coupled to the other contracts' ABIs.
- **`D3RACHub.sol`** — the central coordinator ("brain box"). One admin
  surface, one emergency pause, one aggregate status call, sitting in
  front of `D3RACToken`, `IdentityRegistry`, `DisbursementController`,
  and — as of this update — `RiskRegistry` and `FundingRequestRegistry`
  too. See "The Hub" below for how it's wired in and what it does and
  doesn't protect against.
- **`RiskRegistry.sol`** — puts the exact risk model from
  [`docs/risk-model.md`](../../docs/risk-model.md) /
  `frontend/src/lib/riskModel.ts`, R(c,t) = H(t)·E(c)·V(c), on-chain per
  community. A restricted `dataFeeders` role pushes fresh
  hazard/exposure/vulnerability values (fixed-point, 1e18 scale); the
  contract recomputes R deterministically and emits `ThresholdCrossed`
  the moment a community's score meets or exceeds θ. It cannot sense
  hazard data itself — a smart contract has no way to observe the real
  world — so someone (an oracle, a designated NGO reporter, an off-chain
  job reading public disaster datasets) has to call `updateRisk`. What
  it guarantees is that once that data lands on-chain, the scoring and
  threshold logic is deterministic, public, and impossible to fudge
  after the fact. Fully standalone — no dependency on any other
  contract in this directory.
- **`FundingRequestRegistry.sol`** — a contract cannot browse the web,
  call a donor's API, or "request assistance" on its own initiative.
  What it can do is provide a single, public, permissionless-to-read
  coordination point: an authorized `proposers` address opens a funding
  request for a community (linked to a `RiskRegistry` community ID and
  a `dataSourceURI` pointing at the open dataset justifying the ask),
  and anyone — a donor platform, an NGO dashboard, an indexer bot, a
  grant-matching service — can watch `RequestOpened` events and act on
  them off-chain. Pledges and links to actual `DisbursementController`
  commitments (by ID) are recorded here too, so the whole funding
  lifecycle (ask → pledge → escrow → release) is traceable from one
  place without trusting anyone's private summary of it. Also fully
  standalone — references `DisbursementController` commitment IDs and
  `RiskRegistry` community IDs only as plain values, no contract
  dependency.

This is **not deployed or audited**. See Known limitations below and
[`docs/deployment-guide.md`](../../docs/deployment-guide.md) before
targeting even testnet with anything resembling real funds.

### How RiskRegistry and FundingRequestRegistry connect to the rest

```
RiskRegistry.updateRisk()  →  R(c,t) crosses θ  →  ThresholdCrossed event
        (via Hub.updateRisk(), or direct if you hold dataFeeder status)
                                                          │
                                                          ▼
FundingRequestRegistry.openRequest()  (references communityId, cites data)
        (via Hub.openFundingRequest(), or direct if you hold proposer status)
                                                          │
                                          (off-chain: donor sees it, pledges)
                                                          │
                                                          ▼
FundingRequestRegistry.recordPledge() / linkToCommitment()
                                                          │
                                                          ▼
D3RACToken.mint()  →  DisbursementController.createCommitment()
        (via Hub.mintTokens() / Hub.createCommitment())
                                                          │
                                                          ▼
                    MultiSigAdmin / attester  →  attestMilestone()
                          (via Hub.attestMilestone())
                                                          │
                                                          ▼
                          DisbursementController.releaseMilestone()
```

Neither `RiskRegistry` nor `FundingRequestRegistry` imports or calls
into the other, or into `D3RACToken`/`IdentityRegistry`/
`DisbursementController` — they're linked only by convention (matching
community IDs, commitment IDs passed as plain `uint256`/`bytes32`
values), so they can be deployed, upgraded, or replaced independently.
`D3RACHub` is the one contract that *does* know about all five — see
below.

## The Hub

`D3RACHub.sol` is D3R·AC's control panel — it exists so an operator
doesn't have to separately manage admin keys on five contracts, and so
the frontend has one place to read overall system state instead of
five. As of this update, it covers **both** the day-to-day operational
writes on all five contracts **and** their role/ownership management —
once fully wired (see "Wiring the Hub" below), every write path on
every one of the five underlying contracts is reachable through the
Hub, with one deliberate exception noted below.

**What it gives you:**
- **One admin surface, covering everything.** Operational writes —
  `verifyRecipient`, `createCommitment`, `attestMilestone`,
  `cancelCommitment`, `mintTokens`, `registerCommunity`, `updateRisk`,
  `openFundingRequest`, `closeFundingRequest` — and role/ownership
  management — `setIdentityVerifier`, `revokeRecipient`,
  `transferIdentityRegistryAdmin`, `setDisbursementAttester`,
  `transferDisbursementControllerAdmin`, `setTokenMinter`,
  `transferTokenOwnership`, `setRiskDataFeeder`, `setRiskThreshold`,
  `transferRiskRegistryOwnership`, `setFundingProposer`,
  `recordFundingPledge`, `linkFundingRequestToCommitment`,
  `transferFundingRequestRegistryOwnership` — all route through the Hub
  instead of calling each contract directly.
- **One emergency stop** — `pause()` blocks the *operational* writes
  (`verifyRecipient`, `createCommitment`, `attestMilestone`,
  `mintTokens`, `registerCommunity`, `updateRisk`, `openFundingRequest`)
  in a single call. `cancelCommitment`, `closeFundingRequest`, every
  role/ownership-management function above, and all
  admin/module-management functions (`setToken`, `setIdentityRegistry`,
  `setDisbursementController`, `setRiskRegistry`,
  `setFundingRequestRegistry`, `transferAdmin`) stay callable while
  paused deliberately — these are config/defensive actions, not the
  fund/data-moving operations a pause exists to halt, and you need them
  available *during* an incident (e.g. revoking a compromised
  attester's status without having to unpause first).
- **One status call** — `systemStatus()` returns all five module
  addresses, the paused flag, token total supply, total commitment
  count, total registered-community count, and total funding-request
  count in a single call instead of five separate contract reads.

**The one deliberate exception:** `DisbursementController.releaseMilestone`
is permissionless by that contract's own design — callable by anyone
once a milestone is attested, specifically so payout doesn't bottleneck
on a single admin key (see `DisbursementController.sol`'s own docs).
The Hub doesn't proxy it, on purpose; there's no role to hold for a
function nobody needs permission to call.

**What "full control" does and doesn't mean:** once every underlying
contract's admin/owner role has been transferred to the Hub (see
"Wiring the Hub"), the Hub is *capable* of every write on every
contract. It does not *revoke* access from anyone who already holds a
role directly — a verifier added before the Hub existed, or a
data-feeder some other address was granted later, can still call the
underlying contract directly, bypassing the Hub and its pause entirely.
"Full control" means the Hub is the intended single operational
surface once wired, not that direct access to the underlying contracts
becomes impossible. Treat it as the project's front door, not a lock on
every other door.

`RiskRegistry` and `FundingRequestRegistry` are **optional** at the
Hub's construction — pass `address(0)` for either (or both) to deploy
the Hub before those two exist yet, and wire them in later with
`setRiskRegistry`/`setFundingRequestRegistry`. Every function that
touches one of these two reverts with a clear "not set" message until
its address is configured, rather than assuming it's always present.

### Wiring the Hub

Deploying `D3RACHub` does **not** automatically give it any authority —
it's just another address until you explicitly grant it access on each
underlying contract, and the *way* you grant it differs by function, in
a way that matters. For full coverage, **every** underlying contract's
admin/owner role needs to end up on the Hub, not just the subset the
original (pre-full-control) wiring required:

- **Role mappings** (`verifiers`, `attesters`, `dataFeeders`,
  `proposers`, `minters`) gate the *operational* writes
  (`verifyRecipient`, `attestMilestone`, `updateRisk`, `openRequest`,
  `mint`). Granting the Hub one of these is **additive** — the original
  admin/owner keeps working too:
  ```solidity
  identityRegistry.setVerifier(hubAddress, true);
  disbursementController.setAttester(hubAddress, true);
  riskRegistry.addDataFeeder(hubAddress);
  fundingRequestRegistry.addProposer(hubAddress);
  token.setMinter(hubAddress, true);
  ```
- **Single `admin`/`owner` addresses** gate both a subset of the
  operational writes (`createCommitment`/`cancelCommitment`,
  `registerCommunity`) *and* every role-management proxy
  (`setIdentityVerifier`, `setDisbursementAttester`, `setTokenMinter`,
  `setRiskDataFeeder`, `setRiskThreshold`, `setFundingProposer`, every
  `transfer*` function). For any of these to work through the Hub, the
  Hub must actually *become* that admin/owner — which is **exclusive**,
  not additive. The previous holder loses direct access to
  admin-gated functions the moment this runs:
  ```solidity
  identityRegistry.transferAdmin(hubAddress);
  disbursementController.transferAdmin(hubAddress);
  token.transferOwnership(hubAddress);
  riskRegistry.transferOwnership(hubAddress);
  fundingRequestRegistry.transferOwnership(hubAddress);
  ```
  Note `D3RACToken.transferOwnership` and
  `FundingRequestRegistry.transferOwnership` are *new* requirements for
  full coverage — the original Hub wiring only needed `minters`/
  `proposers` (additive) for `mintTokens`/`openFundingRequest` to work;
  `setTokenMinter` and `setFundingProposer` need the exclusive owner
  role on top of that.
- `FundingRequestRegistry.recordPledge`/`linkToCommitment`/
  `closeRequest` check `msg.sender == request.requester || msg.sender
  == owner`. Because `openRequest` records the *caller* as `requester`,
  a request opened **through the Hub** is automatically manageable
  through the Hub for all three — no extra wiring needed for those
  specific requests. A request opened directly (bypassing the Hub) can
  only be managed through the Hub if the Hub has *also* been made the
  registry's owner via `transferOwnership` (which full-control wiring
  already requires, so this resolves itself once you've done the step
  above).

Once every admin/owner role above has been transferred once, the Hub
can bootstrap its own remaining role grants through its own proxies —
e.g. `hub.setRiskDataFeeder(hubAddress, true)` to grant the Hub itself
data-feeder status, callable by the Hub's own admin, no separate direct
call to `RiskRegistry` needed.

Mixing these up — assuming a `transferAdmin`/`transferOwnership` is
additive, or that granting a role covers a function that actually needs
exclusive ownership — is exactly the kind of mistake
`test/D3RACHub.test.js` was written to catch; see its `beforeEach` for
the full wiring sequence exercised in the test suite, and its "full
control end-to-end" test for a single sequence exercising every write
on every contract through the Hub alone.

## Compiling with TronBox

```bash
cd contracts/tron
npm install -g tronbox
tronbox compile
```

`tronbox-config.js` is compile-only right now (no `networks` entry) —
add a network/private-key section before running `tronbox migrate` to
actually deploy. CI runs this same command on every push/PR that
touches `contracts/tron/**` (see `contracts-tron` job in
`.github/workflows/d3rac-ci.yml`).

## Test suite

`test/` has a logic-level Hardhat/Mocha/Chai test suite (**115 tests**
across `D3RACToken`, `IdentityRegistry`, `DisbursementController`,
`MultiSigAdmin`, `RiskRegistry`, `FundingRequestRegistry`, and
`D3RACHub`) covering the failure paths `docs/deployment-guide.md`'s
checklist calls out by name — zero-amount milestones/requests,
unauthorized callers, double-attestation, double-release, insufficient
contract balance, unverified recipients, out-of-range risk inputs, and
unauthorized pledge recording — plus a `MultiSigAdmin` integration test
proving it can genuinely hold `IdentityRegistry`'s admin role and that a
call routed through it reverts (and stays re-executable) if the
underlying call reverts, a `RiskRegistry` test that reproduces
`docs/risk-model.md`'s own example figures (H=0.81, E=0.66, V=0.74 →
R≈0.3956) using the contract's exact fixed-point arithmetic rather than
a rounded re-derivation, and a `D3RACHub` suite (now 40 tests on its
own) proving: the pause actually blocks the seven operational writes
and doesn't block role-management or admin actions; every operational
AND role-management function genuinely fails without its exact
required wiring, individually, module by module; the
additive-vs-exclusive distinction holds across every function that
needs it (an earlier version of this suite got it wrong once for
`DisbursementController.createCommitment` and had to be fixed — every
subsequent addition, including this full-control pass, checks both
paths explicitly rather than repeating that mistake); and a dedicated
end-to-end test that performs every single write on all five
underlying contracts using nothing but `hub.*` calls in one continuous
sequence, confirming full coverage holds together as a whole and not
just function-by-function in isolation.

Run it with:

```bash
cd contracts/tron
npm install
npx hardhat test
```

**Why Hardhat and not TronBox here:** these contracts use no
TRON-specific precompiles or opcodes, so they're exactly as testable
against a standard EVM as against the TVM — Hardhat's in-process network
is faster to iterate against for logic tests. All 115 tests were run and
passed against solc 0.8.20 during development, including the Hub's
full-control wiring tests. This validates contract
*logic*; it does not replace an actual TronBox/TronIDE deployment and
exercise on Shasta or Nile, which is still required before mainnet (see
`docs/deployment-guide.md`) to catch anything TVM-specific and to
produce a real deployed address to test against.

## How the interface maps to what the frontend expects

The frontend (`frontend/src/lib/tronAdapter.ts`) is written against a
standard **TRC-20** token interface for reading balances and moving
funds:

```solidity
function balanceOf(address _owner) external view returns (uint256 balance);
function decimals() external view returns (uint8);
function symbol() external view returns (string);
function transfer(address _to, uint256 _value) external returns (bool);
```

`D3RACToken.sol` implements exactly this (plus the rest of standard
TRC-20), so the existing frontend adapter works against it unmodified —
just point `VITE_TRON_NETWORK` / the disbursement console's token-address
field at wherever it gets deployed.

## Design decisions worth knowing before you read the code

- **Attestation trust model**: `DisbursementController` doesn't decide
  *how* a milestone is verified — that's deliberately left to whoever
  holds attester status (set via `setAttester`), per
  [`docs/risk-model.md`](../../docs/risk-model.md)'s note that this is
  "deployment-specific." Start with a small multisig as the attester,
  not a single EOA.
- **Funds aren't pulled automatically**: `createCommitment` only records
  a schedule; it doesn't transfer tokens into the contract.
  `releaseMilestone` checks the contract's own token balance and reverts
  rather than partially paying, so the contract needs to actually hold
  (or be funded with) enough of the token before milestones can release.
- **Cancellation doesn't sweep funds**: `cancelCommitment` stops future
  releases but leaves already-deposited, unreleased tokens in the
  contract rather than silently redirecting them — that's left as a
  separate, auditable admin action.
- **Multisig is opt-in, wired via role transfer, not baked in**: none of
  `D3RACToken`, `IdentityRegistry`, or `DisbursementController` know
  `MultiSigAdmin` exists. Deploy `MultiSigAdmin` first, then pass its
  address as the constructor's `admin_`/`owner_` argument on the others
  (or call `transferOwnership`/`transferAdmin` after the fact). From
  then on, every admin action needs `threshold` confirmations submitted
  through `MultiSigAdmin.submitTransaction`/`confirmTransaction`/
  `executeTransaction`, not a single signature.

## Known limitations

- **No professional security audit has been performed.** Do not deploy
  to mainnet with real funds without both an implementation review and a
  professional audit first — see
  [`docs/deployment-guide.md`](../../docs/deployment-guide.md).
- **Not yet deployed to any network** (Shasta, Nile, or mainnet). No
  deployed address exists to point the frontend at yet.
- **Logic tests pass; TVM-specific verification hasn't happened yet** —
  the 115-test Hardhat suite validates behavior against a standard EVM
  (see "Test suite" above), not the actual TVM. Run a TronBox pass
  against TronBox Quickstart (or Shasta/Nile directly) before treating
  this as a TVM-specific gate.
- `admin` / `owner` / `verifiers` / `attesters` default to a single
  deployer key unless you explicitly deploy `MultiSigAdmin` and point
  the other contracts' admin/owner role at it — see "Design decisions"
  above. Don't skip this before mainnet, per the deployment guide's
  security checklist. `MultiSigAdmin.sol` itself is intentionally
  minimal (no owner-replacement-by-vote, no transaction expiry) —
  consider a maintained implementation (e.g. Gnosis Safe) for anything
  beyond early testnet use.
- **The Hub is a convenience, not a security boundary** — see "What it
  does NOT give you" above. Its pause only blocks calls made *through*
  it; a role held directly on `IdentityRegistry` or
  `DisbursementController` still works regardless of the Hub's paused
  state.
- **`RiskRegistry` has no data pipeline behind it** — an actual
  oracle/relayer job to call `updateRisk` from real hazard data doesn't
  exist yet, and `data-pipeline/` still doesn't exist in this repo (see
  the main README's structure diagram). The contract is ready for that
  input; nothing produces it yet.
- **A decision on who the real data feeders / proposers are** —
  `RiskRegistry` and `FundingRequestRegistry` build the roles and access
  control; they don't decide who holds those keys in production. That's
  a real organizational decision (TAAD ops, a partner NGO, a dedicated
  oracle service) that shouldn't be an afterthought before testnet use.

## Testnets

Development and testing should target:

- **Shasta** — https://shasta.tronscan.org/
- **Nile** — https://nile.tronscan.org/

Do not target TRON mainnet until the checklist in
[`docs/deployment-guide.md`](../../docs/deployment-guide.md) is
satisfied.
