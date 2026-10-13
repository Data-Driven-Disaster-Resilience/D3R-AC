# In-house audit pass — 2026-10-12

**Scope:** `contracts/tron/tronbox/contracts/DisbursementController.sol` (fund-custody path), plus the 2026-10-12 CI static-analysis output (`docs/audit-reports/2026-10-12-automated.md`: Mythril clean on all TRON contracts; cargo-audit's only item is the unmaintained-crate warning RUSTSEC-2025-0141 for `bincode`).
**Commit reviewed:** `20cfd92`. **Method:** manual line-by-line review of custody logic. Local Slither/solc could not be downloaded in the review sandbox, so Slither coverage here is the CI run only.
**This is not a third-party audit** and does not replace one.

## Findings

| ID | Severity | Finding | Suggested fix |
|---|---|---|---|
| DC-1 | Low (FIXED) | `_safeTransfer` uses a low-level `call`; if `token` has no code (typo, undeployed address), the call "succeeds" with empty return data and the milestone is marked released with nothing transferred. `createCommitment` only rejects the zero address. Admin-controlled, so low severity. | Require `token.code.length > 0` in `createCommitment` and in `_safeTransfer` (OpenZeppelin `Address.functionCall` pattern). |
| DC-2 | Medium (FIXED) | No accounting of balance against open commitments. `releaseMilestone` checks only the contract's total token balance, so one commitment's release can consume funds intended for another; later releases then revert. | Track `totalCommitted − totalReleased` per token and check solvency in `createCommitment`, or document that over-commitment is an accepted operational risk. |
| DC-3 | Low (FIXED) | `cancelCommitment` leaves unreleased funds in the contract and there is no withdraw/sweep function, so those funds are only reusable via a new commitment to a verified recipient. | Add an admin (multisig-gated) `sweep` with an event, or document the limitation. |
| DC-4 | Info (operational, open) | Admin is the initial attester, so a single key can create, attest and (via anyone) release. Mitigated only if admin is the multisig before mainnet. | Enforce multisig admin and a separate attester before mainnet. |
| DEP-1 | Info | `bincode 1.3.3` unmaintained (RUSTSEC-2025-0141), warning only. | Track upstream; replace if a patched alternative lands in the casper dependency tree. |

Positive observations: checks-effects-interactions plus a reentrancy guard on release; two-step admin transfer (M-2) and tolerant token transfer (M-3) present; no `tx.origin`, no `delegatecall`, Solidity 0.8 checked arithmetic.

## Not covered
Casper contracts (manual), the other TRON contracts (manual), fuzzing/invariant testing, formal verification. These remain scope for the professional audit in `docs/audit-engagement-package.md`.

## Remediation (same day)

- **DC-1:** `createCommitment` and `_safeTransfer` now require the token to have code.
- **DC-2:** new `outstandingCommitted[token]` ledger; `createCommitment` reverts ("commitment not fully funded") unless the contract's balance already covers every open commitment plus the new one. Release and cancel decrease the ledger. Behaviour change: commitments must be funded before creation.
- **DC-3:** new admin-only `sweep(token, to, amount)`, capped at balance minus `outstandingCommitted`, so it can never touch funds backing an open commitment; `D3RACHub.sweepDisbursementFunds` proxies it for when the Hub is the controller's admin.
- **DC-4** is a deployment practice (multisig admin, separate attester), not a code change.
- Tests: 128 -> 135 (new DC-1/2/3 unit tests, a Hub sweep test, invariant harness updated to fund before commit).
- **The Shasta and Casper testnet deployments predate these changes.** Redeploy TRON to pick them up. The Casper `disbursement-controller` has not been reviewed for the equivalent issues.

## Casper review and port (same day)

`contracts/casper/disbursement-controller` had the same three gaps, and DC-3 was worse: it had no way to move tokens out at all except `release_milestone`. Ported:
- **Token must be a contract hash** (`create_commitment` reverts otherwise).
- **Funded-at-creation ledger** (`outstanding` dictionary per token; `create_commitment` calls the token's `balance_of` on the controller's own package Key and reverts with `CommitmentNotFullyFunded`). Release and cancel decrement it.
- **`sweep(token, recipient, amount)`**: admin-only, reentrancy-guarded, capped at balance minus outstanding, emits `FundsSwept`; `d3rac-hub` gained `sweep_disbursement_funds`.
- `init` now records the controller's own package Key (one-time, `AlreadyInitialized` thereafter).
- Tests 68 -> 72: the unfunded-release test became "refuses unfunded commitment", plus non-contract token, over-commit, bounded sweep, and non-admin sweep tests.
- Verification: type-checked natively here; the wasm build and the 72 tests are verified by CI only (no wasm32 sysroot in the review sandbox).
