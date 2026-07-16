# TRON Contracts

## Current status

**No contract source is committed to this repository yet.** This file
documents the interface the rest of the system (specifically the
frontend's `ChainAdapter`) already expects, so contract implementation
can slot in against a known contract without requiring frontend changes.
It does not describe deployed, tested, or audited code, because none
exists here at time of writing — the top-level README's status line will
be updated once that changes.

If TRON contracts for this project exist in a different repository or
were developed outside this one, they haven't been merged into
`Data-Driven-Disaster-Resilience/D3R-AC` as of this commit — link them
here once that's sorted out, rather than assuming this note is wrong.

## Expected interface

The frontend (`frontend/src/lib/tronAdapter.ts`) is written against a
standard **TRC-20** token interface for reading balances and moving
funds:

```solidity
function balanceOf(address _owner) external view returns (uint256 balance);
function decimals() external view returns (uint8);
function symbol() external view returns (string);
function transfer(address _to, uint256 _value) external returns (bool);
```

This is intentionally the minimal, standard TRC-20 surface — nothing
D3R·AC-specific yet. It lets the frontend read a balance and execute a
transfer against any TRC-20 token today, which is enough to build and
test the UI, but it is **not** the milestone-based disbursement logic
the project's README describes ("conditional, milestone-based,
transparent fund release"). That logic — verifying a milestone condition
on-chain before releasing funds, rather than an unconditional transfer —
still needs to be designed and implemented as an actual contract.

## What the real contract needs to add

Based on the architecture described in the main README and
[`docs/risk-model.md`](../../docs/risk-model.md), the milestone-release
contract will need at minimum:

- **Milestone definitions** per funding recipient/community — what
  condition releases which tranche.
- **A trusted way to attest a milestone was met** — an oracle, an
  authorized reporter role, or a multisig attestation. This is a real
  design decision with trust and centralization trade-offs; it shouldn't
  be an afterthought.
- **Access control** — who can create a funding commitment, who can
  attest milestones, who (if anyone) can cancel or claw back funds, and
  under what conditions.
- **Auditable events** for every state change (commitment created,
  milestone attested, funds released) — the whole point of the on-chain
  approach is that this is inspectable without trusting an intermediary.

## Known limitations

- No contract source exists in this repository yet — see Status above.
- Whatever contract is eventually added here should assume **it will be
  handling real disaster-relief funds** and be reviewed accordingly
  before any mainnet use.
- **No professional security audit has been performed**, because there
  is nothing here yet to audit. Do not deploy anything derived from this
  document to mainnet with real funds without both an actual
  implementation review and a professional audit first — see
  [`docs/deployment-guide.md`](../../docs/deployment-guide.md).

## Testnets

Development and testing should target:

- **Shasta** — https://shasta.tronscan.org/
- **Nile** — https://nile.tronscan.org/

Do not target TRON mainnet until the checklist in
[`docs/deployment-guide.md`](../../docs/deployment-guide.md) is
satisfied.
