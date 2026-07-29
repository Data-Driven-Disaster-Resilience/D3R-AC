# D3R·AC Smart Contract Suite — Internal Audit Pass

**Date:** 2026-07-25
**Scope:** `D3RACProperties.sol`, `D3RACToken.sol`, `IdentityRegistry.sol`,
`MultiSigAdmin.sol`, `RiskRegistry.sol`, `FundingRequestRegistry.sol`,
`DisbursementController.sol`, `D3RACHub.sol` (commit `d71da30` on `main`).
**Method:** Manual line-by-line review for standard EVM vulnerability
classes (reentrancy, access control, integer handling, external call
safety, front-running, single points of failure) plus cross-contract
interaction review of the Hub's orchestration layer.

> ⚠️ **This is not a substitute for a professional third-party audit.**
> No project should move real donor/relief funds to mainnet on the
> strength of a self-review alone. Treat this as a first pass that should
> materially reduce (not eliminate) findings a paid audit firm would
> raise, and as a scoping document to bring to that firm.

---

## Summary

No critical, fund-draining bugs were found. The suite consistently
follows checks-effects-interactions, uses a shared reentrancy guard on
the one external-call path that matters (`DisbursementController.
releaseMilestone`), and checks return values on external token transfers.
The findings below are mostly **medium/low severity, centralization and
robustness issues** — expected in a v1 suite with single-admin control,
but worth deciding on deliberately before mainnet, not by default.

| Severity | Count |
|---|---|
| High | 0 |
| Medium | 3 |
| Low | 5 |
| Informational | 4 |

---

## Medium Severity

### M-1: Single-key admin/owner on 6 of 7 contracts (no built-in multisig requirement)
**Where:** `IdentityRegistry.admin`, `DisbursementController.admin`,
`D3RACToken.owner`, `RiskRegistry.owner`, `FundingRequestRegistry.owner`,
`D3RACHub.admin`.
**Issue:** Every contract's docs/comments *recommend* the admin be a
`MultiSigAdmin` address, but nothing on-chain enforces it — any EOA can
be passed at deployment. If that EOA's key is lost or compromised, an
attacker (or nobody) controls verifier/attester/minter roles, community
registration, and (via the Hub) commitment creation. Since
`DisbursementController.releaseMilestone` is permissionless once
attested, a compromised attester key is the most direct path to
draining committed relief funds.
**Recommendation:** Before mainnet, deploy `MultiSigAdmin` first and
pass its address as every `admin_`/`owner_` constructor argument — do
not deploy any contract with a single EOA as admin, even temporarily.
Consider adding a `require` in each constructor that checks the address
has code (`extcodesize > 0`) as a cheap guard against passing an EOA by
mistake — imperfect (a multisig-shaped contract could still misbehave)
but catches the most common mistake.

### M-2: Single-step ownership/admin transfer (no two-step handoff)
**Where:** `transferAdmin`/`transferOwnership` in every contract that has
one (`IdentityRegistry`, `DisbursementController`, `D3RACToken`,
`RiskRegistry`, `FundingRequestRegistry`, `D3RACHub`, and implicitly
`MultiSigAdmin`'s owner set at construction).
**Issue:** All transfers are one-directional and immediate — if the
`newAdmin`/`newOwner` address is mistyped (a real risk when copy-pasting
TRON base58 addresses, which have no checksum comparable to EIP-55), the
contract's admin role is unrecoverably lost the moment the transaction
confirms. This is a "no dry run" version of the exact class of mistake
this whole audit exists to prevent for donation addresses.
**Recommendation:** Add a two-step pattern (`proposeNewAdmin` /
`acceptAdmin`, called by the new address) to every admin/owner transfer
function before mainnet use. This is a small, contained change with a
low blast radius to add.

### M-3: Non-standard TRC-20/ERC-20 tokens can break `DisbursementController.releaseMilestone`
**Where:** `DisbursementController.sol` lines 208, 219 —
`ITRC20(c.token).balanceOf(...)` and
`require(ITRC20(c.token).transfer(...), ...)`.
**Issue:** `createCommitment` accepts *any* address as `token` with no
validation that it actually implements TRC-20 correctly. Some real-world
tokens (most famously USDT's Ethereum implementation, and forks that
copy it) do not return a `bool` from `transfer` at all — calling
`require(ITRC20(token).transfer(...))` against such a token will revert
on a successful transfer (ABI decoding failure), permanently soft-locking
that milestone's funds in the contract (recoverable only via
`cancelCommitment`, which doesn't move funds either).
**Recommendation:** Before mainnet, confirm the exact `USDT-TRC20`
contract you intend to disburse in fully returns `bool` from `transfer`
(most TRON-native TRC-20 tokens do; this differs from Ethereum's USDT).
If there's any chance of non-standard tokens being used, use a
SafeERC20-style low-level `call` + separate success/return-data check
instead of a direct interface call, so a missing return value doesn't
brick the milestone.

---

## Low Severity

### L-1: `D3RACToken.approve` classic front-running window
**Where:** `D3RACToken.sol` `_approve`.
**Issue:** Standard ERC-20/TRC-20 approve race condition — changing an
allowance from N to M in two calls has a window where a spender could
front-run and spend both. Extremely well-known, low real-world impact
here since this token isn't meant to be traded on an AMM, but worth
noting since it's not addressed.
**Recommendation:** Optional: add `increaseAllowance`/
`decreaseAllowance` convenience functions. Not blocking.

### L-2: `MultiSigAdmin` owner set is immutable post-deployment
**Where:** `MultiSigAdmin.sol` constructor; no `addOwner`/`removeOwner`.
**Issue:** This is explicitly documented as a deliberate design choice
("rotate by deploying a new MultiSigAdmin"), so it's not a bug — but it
means losing one owner key with a tight threshold (e.g. 2-of-3) reduces
effective security margin permanently until a full re-deployment and
re-wiring of every dependent contract's admin role. Worth confirming
this operational cost is acceptable before mainnet, given re-wiring
touches every one of the other 6 contracts.

### L-3: `D3RACHub` orchestration functions don't re-check `whenNotPaused` consistency with underlying contracts
**Where:** `D3RACHub.sol` — e.g. `cancelCommitment`, `closeFundingRequest`
deliberately bypass `whenNotPaused` (by design, documented), but the
underlying contracts (`DisbursementController.cancelCommitment`,
`FundingRequestRegistry.closeRequest`) have no pause of their own.
**Issue:** If a role holder was granted directly on the underlying
contract *before* full Hub wiring (explicitly called out as possible in
the Hub's own docstring), pausing the Hub does **not** stop that holder
from acting directly on the underlying contract — the pause only covers
the Hub's own call path. This is documented as expected behavior, but
it's worth being certain every operator understands "paused" means
"paused at the Hub," not "paused system-wide," before relying on it
during an actual incident.
**Recommendation:** No code change strictly required; recommend adding
this caveat explicitly to any incident-response runbook, not just the
code comments.

### L-4: `RiskRegistry.updateRisk` / `FundingRequestRegistry.recordPledge` trust off-chain-reported values with no bounds beyond `[0, SCALE]`
**Where:** `RiskRegistry.sol` `updateRisk`; `FundingRequestRegistry.sol`
`recordPledge`.
**Issue:** Both are intentionally thin coordination layers trusting an
authorized feeder/proposer's input — by design, per the contracts'
own docs. Flagging only because a compromised or careless data-feeder
key can push a fabricated risk score that triggers `ThresholdCrossed`
(intended to gate real fund movement downstream) or fabricate pledge
progress. This is a trust-model choice, not a code bug — but it means
the data-feeder and proposer roles deserve the same multisig-before-
mainnet treatment as the admin roles in M-1.

### L-5: No maximum on `descriptions`/`amounts` array length in `DisbursementController.createCommitment`
**Where:** `DisbursementController.sol` `createCommitment` loop.
**Issue:** An admin could submit an unbounded number of milestones,
which — while gated by `onlyAdmin` so not exploitable by an outsider —
could hit TRON's per-transaction energy/bandwidth limits and fail
unpredictably for a large commitment. Low severity since it requires
admin error, not attacker action.
**Recommendation:** Optional sanity cap (e.g. 50 milestones) with a
clear revert message, mainly to fail fast and legibly rather than
hitting an opaque out-of-energy error.

---

## Informational

- **I-1:** `IdentityRegistry.verifyRecipient` fully overwrites the
  `Recipient` struct on re-verification, resetting `revokedAt` to 0 —
  correct behavior, just confirm this is the intended semantics for a
  previously-revoked-then-reverified recipient (it appears to be, per
  the contract's own comments).
- **I-2:** Every contract emits solid events for its full lifecycle —
  this is a genuine strength; makes off-chain monitoring/indexing
  straightforward and gives real transparency for donors/NGOs auditing
  fund flow, which matters given the project's stated purpose.
- **I-3:** `D3RACHub`'s interfaces (`IDisbursementControllerHub`,
  `IMintableToken`, etc.) are minimal and hand-rolled rather than
  imported from the concrete contracts — correct defensive pattern, just
  confirm each interface is kept in lockstep if the underlying contracts
  change signatures (a mismatch would fail at the ABI-encoding level,
  likely loudly, but worth a checklist item for future changes).
- **I-4:** No contract in this suite uses `selfdestruct` or `delegatecall`
  — removes two entire classes of common exploit vectors by simply not
  having the surface. Worth explicitly confirming this stays true in the
  Casper suite (Rust/WASM has different but analogous footguns).

---

## Recommended before mainnet (in priority order)

1. Resolve M-1: deploy `MultiSigAdmin` and use it as the sole admin/owner
   for every contract — no direct-EOA admin, even temporarily.
2. Resolve M-3: confirm the actual USDT-TRC20 token's `transfer` return
   behavior, or defensively rewrite the transfer call to tolerate a
   non-standard return.
3. Resolve M-2: add two-step admin/owner transfer.
4. Get a professional third-party audit on the finalized version —
   this pass is a starting point for that engagement's scope, not a
   replacement for it.
5. Re-run the full Hardhat test suite (115 tests) against any changes
   made in response to the above before merging.
