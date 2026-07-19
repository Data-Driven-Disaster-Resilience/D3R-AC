# TRON Contracts

## Current status

Contract source is now committed for **six** contracts, all dependency-free
(no OpenZeppelin import — see each file's header comment for why),
compiled clean against solc 0.8.20 with the optimizer on, and
**logic-tested** (25 passing tests for the v2 additions below; see
Testing). Still **not deployed or audited**.

### Core suite (v1)

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

### v2 additions

- **`RiskRegistry.sol`** — puts the exact risk model from
  [`docs/risk-model.md`](../../docs/risk-model.md) /
  `frontend/src/lib/riskModel.ts`, R(c,t) = H(t)·E(c)·V(c), on-chain per
  community. A restricted `dataFeeders` role pushes fresh
  hazard/exposure/vulnerability values (fixed-point, 1e18 scale); the
  contract recomputes R deterministically and emits `ThresholdCrossed`
  the moment a community's score meets or exceeds θ. **This contract
  cannot sense hazard data itself** — a smart contract has no way to
  observe the real world. Someone (an oracle, a designated NGO reporter,
  an off-chain job reading public disaster datasets) has to call
  `updateRisk`. What it guarantees is that once that data lands on-chain,
  the scoring and threshold-crossing logic is deterministic, public, and
  impossible to fudge after the fact.
- **`FundingRequestRegistry.sol`** — a contract cannot browse the web,
  call a donor's API, or "request assistance" on its own initiative. What
  it can do is provide a single, public, permissionless-to-read
  coordination point: an authorized `proposers` address opens a funding
  request for a community (linked to a `RiskRegistry` community ID and a
  `dataSourceURI` pointing at the open dataset justifying the ask), and
  anyone — a donor platform, an NGO dashboard, an indexer bot, a
  grant-matching service — can watch `RequestOpened` events and act on
  them off-chain. Pledges and links to actual `DisbursementController`
  commitments are recorded here too, so the whole funding lifecycle (ask
  → pledge → escrow → release) is traceable from one place without
  trusting anyone's private summary of it.
- **`AttestorMultisig.sol`** — upgrades `DisbursementController`'s
  single-attester model to N-of-M threshold voting, **without modifying
  `DisbursementController.sol` itself** — deploy this contract, then call
  `DisbursementController.setAttester(address(attestorMultisig), true)`
  (and `setAttester(oldAttesterEOA, false)` to retire the prior
  single-EOA attester). Any signer proposes an attestation; once enough
  other signers approve (reaching `threshold`), the multisig calls
  `DisbursementController.attestMilestone` itself. Signer-set and
  threshold changes are self-administered through the same N-of-M voting
  mechanism, not a separate owner key. This directly addresses this
  file's own previous note: *"Start with a small multisig as the
  attester, not a single EOA"* — this contract is that multisig.

This is **not deployed or audited**. See Known limitations below and
[`docs/deployment-guide.md`](../../docs/deployment-guide.md) before
targeting even testnet with anything resembling real funds.

## How the pieces connect

```
RiskRegistry.updateRisk()  →  R(c,t) crosses θ  →  ThresholdCrossed event
                                                          │
                                                          ▼
FundingRequestRegistry.openRequest()  (references communityId, cites data)
                                                          │
                                          (off-chain: donor sees it, pledges)
                                                          │
                                                          ▼
FundingRequestRegistry.recordPledge() / linkToCommitment()
                                                          │
                                                          ▼
D3RACToken.mint()  →  DisbursementController.createCommitment()
                                                          │
                                                          ▼
AttestorMultisig (N-of-M vote)  →  DisbursementController.attestMilestone()
                                                          │
                                                          ▼
                          DisbursementController.releaseMilestone()
```

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

## Testing

- **25 tests, currently passing**, covering the v2 additions
  (`RiskRegistry`, `FundingRequestRegistry`, `AttestorMultisig`) — fixed
  point risk-score math against known values, threshold-crossing events,
  access control on every feeder/proposer/signer function, funding-request
  status transitions, and the multisig's threshold voting (including its
  self-administered signer governance), run as a full integration against
  the actual `DisbursementController` / `IdentityRegistry` / `D3RACToken`
  contracts — not mocks. These ran on Hardhat's in-memory EVM rather than
  a live TVM node — see `../../hardhat-tests/README.md` for exactly what
  that does and doesn't prove for a TRON deployment.
- **`test/v2suite.test.js`** in this directory is a TronBox-native port of
  the key integration paths, for running against a real TVM node. It has
  not been executed against a live node yet; treat it as ready-to-run
  scaffolding until you've run it yourself.
- **The core v1 suite (`D3RACToken`, `IdentityRegistry`,
  `DisbursementController`) still has no dedicated test suite of its
  own** — the v2 tests exercise it indirectly (as a real dependency, not
  a mock) but don't independently cover its own failure paths
  (zero-amount, unauthorized caller, insufficient balance, double-release,
  double-attestation). That gap from the original README is still open.

## Known limitations

- **No professional security audit has been performed on any contract.**
  Do not deploy to mainnet with real funds without both an implementation
  review and a professional audit first — see
  [`docs/deployment-guide.md`](../../docs/deployment-guide.md).
- **Not yet deployed to any network** (Shasta, Nile, or mainnet). No
  deployed address exists to point the frontend at yet.
- **The core v1 suite has no dedicated test suite** — see Testing above.
- `admin` / `owner` / `verifiers` are still single-key roles on
  `IdentityRegistry` and `D3RACToken` as written.
  `DisbursementController`'s attester role can now be a multisig
  (`AttestorMultisig.sol`), which addresses that one role specifically —
  the others still need the same treatment before mainnet, per the
  deployment guide's security checklist.
- **A decision on who the real data feeders / proposers / attestor
  signers are** — this update builds the roles and access control; it
  does not decide who holds those keys in production, and that's a real
  organizational decision (TAAD ops, a partner NGO, a dedicated oracle
  service) that shouldn't be an afterthought.
- **An actual oracle/relayer job** to call `RiskRegistry.updateRisk` from
  real hazard data — the contract has no data pipeline of its own, and
  `data-pipeline/` still doesn't exist in this repo (see the main
  README's structure diagram).

## Deploying

See [`docs/deployment-guide.md`](../../docs/deployment-guide.md) for the
full process. In short:

```bash
cd contracts/tron
tronbox compile
tronbox migrate --network shasta   # testnet first, always
```

`migrations/2_deploy_core_suite.js` deploys the v1 suite
(`IdentityRegistry`, `D3RACToken`, `DisbursementController`) — added
retroactively by this update, since the original merge that added those
contracts didn't include TronBox scaffolding.
`migrations/3_deploy_v2_suite.js` deploys the v2 additions and wires
`AttestorMultisig` against the deployed `DisbursementController` — see
that file's header comment for the environment variables it reads
(initial feeder/proposer addresses, risk threshold, multisig signer set).
It deliberately does **not** automatically call
`DisbursementController.setAttester()` — that's a security-relevant role
change left as an explicit manual step after you've confirmed the
multisig deployed as expected.

## Testnets

Development and testing should target:

- **Shasta** — https://shasta.tronscan.org/
- **Nile** — https://nile.tronscan.org/

Do not target TRON mainnet until the checklist in
[`docs/deployment-guide.md`](../../docs/deployment-guide.md) is
satisfied.
