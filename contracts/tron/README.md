# TRON Contracts

## Layout

```
contracts/tron/
├── contracts/            # .sol sources (TronBox's and Hardhat's shared default)
│   ├── D3RACToken.sol
│   ├── IdentityRegistry.sol
│   ├── DisbursementController.sol
│   └── MultiSigAdmin.sol
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

Four contracts, dependency-free (no OpenZeppelin import — see each
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
  `admin`/`owner` role on the three contracts above instead of a single
  EOA (the "consider a multisig for any contract-owner or admin role"
  item from `docs/deployment-guide.md`'s security checklist). Deploy it
  first and pass its address as the constructor's admin/owner argument
  on the others. Generic (submits arbitrary `to`/`value`/`data`), so
  it isn't coupled to the other contracts' ABIs.

This is **not deployed or audited**. See Known limitations below and
[`docs/deployment-guide.md`](../../docs/deployment-guide.md) before
targeting even testnet with anything resembling real funds.

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

`test/` has a logic-level Hardhat/Mocha/Chai test suite (50 tests)
covering all four contracts, including the failure paths
`docs/deployment-guide.md`'s checklist calls out by name — zero-amount
milestones, unauthorized callers, double-attestation, double-release,
insufficient contract balance, and unverified recipients — plus a
`MultiSigAdmin` integration test proving it can genuinely hold
`IdentityRegistry`'s admin role and that a call routed through it
reverts (and stays re-executable) if the underlying call reverts.

Run it with:

```bash
cd contracts/tron
npm install
npx hardhat test
```

**Why Hardhat and not TronBox here:** these contracts use no
TRON-specific precompiles or opcodes, so they're exactly as testable
against a standard EVM as against the TVM — Hardhat's in-process network
is faster to iterate against for logic tests. All 50 tests were run and
passed against solc 0.8.20 during development. This validates contract
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
  the 50-test Hardhat suite validates behavior against a standard EVM
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

## Testnets

Development and testing should target:

- **Shasta** — https://shasta.tronscan.org/
- **Nile** — https://nile.tronscan.org/

Do not target TRON mainnet until the checklist in
[`docs/deployment-guide.md`](../../docs/deployment-guide.md) is
satisfied.
