# D3R·AC Smart Contract Suite — Self-Review, 2026-09-09

**Date:** 2026-09-09
**Scope:** everything that changed since
[`docs/audit-pass-2026-07-25.md`](audit-pass-2026-07-25.md) — the M-2/M-3
TRON fixes, the entire Casper suite (all seven contracts), the systemic
Casper caller-resolution fix, and both real testnet deployments
(TRON Shasta, Casper testnet) — plus manual triage of the three
findings still open in the most recent automated Slither report
([`docs/audit-reports/2026-09-07-automated.md`](audit-reports/2026-09-07-automated.md)).
**Method:** same as the July review — manual line-by-line reading for
standard vulnerability classes, this time also cross-referencing each
flagged Slither finding against the actual surrounding code rather than
taking severity labels at face value.

> ⚠️ **This is not a professional audit, and does not become one by
> being thorough.** A professional audit means a licensed security
> firm, named auditors, a paid engagement, and accountability that an
> AI-assisted self-review cannot provide regardless of method or
> effort. No project should move real donor/relief funds to mainnet on
> the strength of this document. Treat it as a scoping aid for a firm,
> not a substitute for one — same posture as the July review, restated
> because this document exists specifically to be read alongside a
> "audit complete" request it does not satisfy.

---

## Summary

No new critical or fund-draining findings. The three items still open
in the automated Slither report were each individually re-examined
against the real surrounding code (not just the tool's flat summary);
two are very likely false positives given context Slither's detector
doesn't fully capture, and the third is a real but low-practical-impact
precision issue in fixed-point arithmetic. One genuinely new,
actionable finding came out of researching the exact Solidity version
in use against TRON's own opcode-support documentation — not something
the existing Slither/cargo-audit CI already checks.

| Severity | Count |
|---|---|
| High | 0 |
| Medium | 1 (precision, not correctness) |
| Low | 1 (chain-compatibility hardening) |
| Informational / re-triaged findings | 2 |

---

## Medium — Fixed-point precision loss in `RiskRegistry.R(c,t)`

`RiskRegistry.sol`'s `((hazard * exposure) / SCALE) * vulnerability) /
SCALE` divides by `SCALE` (1e18) between each multiplication, matching
Slither's `divide-before-multiply` flag
([finding](audit-reports/2026-09-07-automated.md)). Real, not a false
positive — each division truncates a remainder before the next
multiplication compounds it.

Real-world impact is low: `uint256` has enough headroom (≈256 bits)
that `hazard * exposure * vulnerability` computed in one pass before
any division — `(hazard * exposure * vulnerability) / SCALE / SCALE`
— would not overflow for any realistic 1e18-scaled input, and would be
strictly more precise. The current version's rounding error is at most
a few parts in 1e18 per division, compounding to a handful of atomic
units — not enough to flip a threshold comparison in any scenario this
system is actually used for. Worth fixing since it's a one-line,
strictly-better change, not because the current version is dangerous.

## Low — No explicit `evmVersion` pin; relying on solc 0.8.20's default (Shanghai / `PUSH0`)

New finding, not from Slither. `contracts/tron/hardhat.config.js`'s
`solidity` block sets `version: "0.8.20"` with no `evmVersion`
override, so compilation targets solc 0.8.20's default EVM version —
Shanghai, which emits the `PUSH0` opcode by default for zero-constant
pushes. TRON's own TVM documentation gates Shanghai-era opcodes
(including `PUSH0`) behind a chain parameter
(`ALLOW_TVM_SHANGHAI`, mainnet-activated via committee proposal #89) —
support isn't automatic just because a network is EVM-compatible in
general.

The real Shasta deployment already succeeded, which is reassuring but
not conclusive: a `PUSH0` instruction only fails at the moment a code
path containing one actually executes, not necessarily at deploy time,
so a rarely-hit branch could still surprise someone later even though
today's deployment worked. Recommended hardening, not an emergency
fix: pin `evmVersion: "paris"` (the pre-Shanghai target) explicitly in
`hardhat.config.js`'s `solidity.settings`, which guarantees TVM
compatibility regardless of which specific chain parameters are active
on whichever network gets deployed to next, rather than depending on
inference from one successful deployment.

## Re-triaged — `MultiSigAdmin.executeTransaction`'s Slither `reentrancy-eth` (High)

Slither's flat summary reads alarmingly (a state write, `t.executed =
false`, positioned textually after an external call) — but reading the
actual function shows real protection the detector's summary doesn't
convey: `t.executed = true` is set **before** the external call
(correct checks-effects-interactions order); the flagged `false` write
only happens inside the `if (!success)` branch, which immediately
`revert()`s — and a revert undoes every state change in that branch,
including the reset itself, so it never actually persists. The
function also already carries a real, correctly-implemented
`nonReentrant` guard
([`D3RACProperties.sol`](../contracts/tron/tronbox/contracts/base/D3RACProperties.sol),
a standard status-flag pattern, not just a modifier name with no
logic behind it) — checked directly, not assumed from the name.

Given both of those, a malicious `to` target's callback genuinely
cannot re-enter `executeTransaction` for the same or a different `txId`
during the external call. Assessed as a false positive, or at minimum
a finding whose real risk is far below "High" — Slither's reentrancy
heuristics are known to sometimes not fully credit custom guards; this
looks like exactly that case, not evidence of an actual exploit path.
**Not filed as resolved** — genuinely uncertain enough that a
professional auditor should form their own view rather than take this
document's word for it, same reasoning the July review's own
methodology note states.

## Re-triaged — `DisbursementController._safeTransfer`'s Slither `incorrect-equality` (Medium)

The flagged pattern — `returnData.length == 0 || abi.decode(returnData,
(bool))` when checking a token transfer's result — is the standard,
widely-used idiom for safely handling both fully-compliant TRC20/ERC20
tokens (which return `true`) and non-compliant-but-common ones (which
return no data at all), matching OpenZeppelin's own `SafeERC20`
pattern. Slither's `incorrect-equality` detector flags strict `==`
generically without necessarily recognizing this specific,
industry-standard safe context. Assessed as a false positive.

---

## Casper suite — security-relevant context, not re-derived here

The most significant Casper-side security finding this project has had
was already found, fixed, and documented in detail elsewhere — not
repeated in full here, just cross-referenced: a systemic
caller-resolution bug (`runtime::get_caller()`, which resolves to the
*original transaction-signing account* rather than an intermediate
calling contract, used where `runtime::get_immediate_caller()` was
needed) affected every contract's admin/owner access-control check
until fixed across all five that had it — see
[`contracts/casper/README.md`](../contracts/casper/README.md)'s own
"Systemic fix" section for the full writeup, and
`d3rac-hub-tests/tests/integration_tests.rs` for the integration test
that now exercises the fix for real (a Hub-mediated call correctly
rejected before the fix would have applied, and correctly accepted
after).

## What this document does not cover

- Casper-side static analysis equivalent to Slither/cargo-audit's TRON
  coverage — `cargo-audit` runs in CI for dependency CVEs, but there is
  no Casper-specific equivalent of Slither's contract-logic detectors
  in this project yet.
- Gas/resource-cost analysis on either chain.
- Anything about the frontend, data pipeline, or agent fleet — this
  document is scoped to the two on-chain contract suites only, matching
  the July review's own scope.
