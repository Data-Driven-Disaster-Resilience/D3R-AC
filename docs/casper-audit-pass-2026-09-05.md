# D3R·AC Casper Contract Suite — Internal Audit Pass

**Date:** 2026-09-05
**Scope:** `risk-registry`, `identity-registry`, `disbursement-controller`,
`d3rac-token`, `multisig-admin`, `d3rac-hub`, `funding-request-registry`
(commit `93cab72` on `main` — all 7 contracts, the point at which the
suite first became fully compiling with a passing full-wiring
integration test).
**Method:** Manual line-by-line review for Rust/Casper-specific
vulnerability classes (integer overflow/underflow, access control via
`get_caller()`/`get_immediate_caller()`, reentrancy, cross-contract call
safety, `Key`-variant confusion) plus cross-contract interaction review
of the Hub's orchestration layer. Grounded against a real external
precedent (Halborn's professional Casper 2.0 audit for the Casper
Association) and general 2025 on-chain loss data (access control ≈53%
of losses) rather than a generic EVM checklist ported over uncritically
— Casper's actual risk profile differs from Solidity's in specific,
consequential ways (see M-1 and M-2 below).

> ⚠️ **This is not a substitute for a professional third-party audit.**
> No project should move real donor/relief funds to a Casper deployment
> on the strength of a self-review alone. Treat this as a first pass
> that should materially reduce (not eliminate) findings a paid audit
> firm would raise, and as a scoping document to bring to that firm —
> the same standing this repo's TRON audit
> ([`docs/audit-pass-2026-07-25.md`](audit-pass-2026-07-25.md)) has.

---

## Summary

No critical, fund-draining bugs were found. The suite correctly applies
reentrancy guards only where an external call actually happens
(`disbursement-controller::release_milestone`,
`multisig-admin::execute_transaction`), and the systemic
`get_caller()`-vs-immediate-caller access-control bug found and fixed
earlier in this suite's history (see commit history around
`fix/get-caller-systemic-immediate-caller`) is now applied consistently
across all seven contracts, including `funding-request-registry`, which
was written after that fix landed and had silently regressed it until
caught by this suite's own integration test — a good sign the test
coverage is doing real work, not just padding a count.

The one genuinely new class of finding this pass surfaces, not present
in the TRON audit because Solidity ≥0.8 makes it moot: **this workspace
does not enable `overflow-checks` in its release profile, and several
financial-value updates use raw, unchecked arithmetic** (M-1). This is
a real gap with no Solidity equivalent to reason about by analogy —
Rust's default release-mode behavior is silent wraparound, not a
panic/revert.

| Severity | Count |
|---|---|
| High | 0 |
| Medium | 2 |
| Low | 4 |
| Informational | 4 |

---

## Medium Severity

### M-1: `overflow-checks` not enabled in the release profile; several financial fields use unchecked arithmetic

**Status: fixed in this same pass** — `overflow-checks = true` added to
`contracts/casper/Cargo.toml`'s `[profile.release]` (suite-wide), plus
`checked_add`-with-typed-revert applied at the four specific call sites
below, rather than left as a recommendation only.

**Where:** `contracts/casper/Cargo.toml`'s `[profile.release]` (no
`overflow-checks` key at all); `d3rac-token::mint`
(`total_supply + amount`, `balance + amount`),
`d3rac-token::move_balance` (`to_balance + amount`),
`disbursement-controller::create_commitment` (`total_amount += amount`
inside the milestone loop), `disbursement-controller::release_milestone`
(`commitment.released_amount += amount`).

**Issue:** Rust's arithmetic operators (`+`, `-`, `*`) panic on
overflow only in the `dev` profile; in `release` — what actually gets
compiled to the deployed `.wasm` — they silently wrap by default unless
`overflow-checks = true` is set, or the code explicitly uses
`checked_add`/`saturating_add`/etc. This workspace's `[profile.release]`
sets `codegen-units = 1` and `opt-level = "z"` but never touches
`overflow-checks`, so it inherits the default: **off**. Every addition
cited above can, in principle, wrap `U256`'s ~1.15×10⁷⁷ range back to a
small number instead of reverting.

In practice this needs either a privileged, malicious/compromised actor
(`mint` is minter-gated; `create_commitment` is admin-gated) or an
extreme, unrealistic value to actually trigger — `U256`'s range makes
accidental overflow through normal use effectively impossible. That's
why this is Medium, not High: there's no permissionless path to
triggering it today. But "requires a malicious privileged actor" is
exactly the failure mode defense-in-depth exists for, and the fix is
essentially free.

**Recommendation (partially already applied above):** replace any
*remaining* unchecked arithmetic on financial fields elsewhere in the
suite with the same `checked_add`/typed-revert pattern as new code is
added. Note `funding-request-registry::record_pledge` already does the
right thing here (`saturating_add`, with its own honest comment about
the saturating-vs-reverting tradeoff) — that function was not touched
by this fix and remains a good pattern to match, though `checked_add`
+revert is arguably still preferable to saturating for a value this
security-relevant, since saturating silently caps rather than surfacing
the anomaly.

### M-2: Every contract still deploys with a single-key admin/owner by default

**Where:** `IdentityRegistry.admin`, `DisbursementController.admin`,
`D3RACToken.owner`, `RiskRegistry.owner`,
`FundingRequestRegistry.owner`, `D3RACHub.admin` — all set from a
single `Key` constructor argument at install time.

**Issue:** Same underlying finding as the TRON audit's own M-1, for the
same reason: nothing on-chain *requires* that argument be a multisig,
so any single EOA can be passed. Materially better here than on the
TRON side at the time of that audit, though: `d3rac-hub-tests`'
comprehensive wiring test now proves, with a real passing test rather
than only a documented recommendation, that the full
propose-then-accept handoff to a `multisig-admin` package identity
works correctly end to end across every module. The risk is now purely
about deployment-time discipline (does whoever runs the actual
`casper-test`/mainnet deploy script follow the two-step handoff before
treating the deployment as production-representative?), not an
unproven or undocumented pattern.

**Recommendation:** Same as the TRON audit's own M-1: deploy
`multisig-admin` first, and make the full grant-then-propose-then-accept
sequence (matching `d3rac-hub-tests`' own proven order) a mandatory,
scripted step in whatever deploy tooling eventually drives a
`casper-test` deployment — never a manual, optional afterthought. No
Casper equivalent of an `extcodesize` guard applies here the way it
might on the EVM side (Casper's addressing model doesn't offer an
equivalent cheap runtime check), so this has to be enforced by deploy
process, not by a constructor-level guard.

---

## Low Severity

### L-1: `d3rac-hub` and `funding-request-registry` have no per-contract integration test suite

**Where:** `contracts/casper/d3rac-hub-tests/` and the absence of a
`contracts/casper/funding-request-registry-tests/` package entirely.

**Issue:** `d3rac-hub` is covered by exactly one test (the
comprehensive full-wiring scenario) — real and valuable, but it
exercises the "everything works when used correctly" path only. None
of Hub's individual guards (calling an operational passthrough while
paused, calling a role-management proxy for a module that was never
set, a non-admin attempting any of Hub's ~15+ entry points) have
dedicated tests the way every other contract's own guards do.
`funding-request-registry` has no dedicated test package at all — its
only verification is indirect, via that same single Hub test, which
only exercises `propose_new_owner`/`accept_ownership` and
`open_request`/`record_pledge`, not `link_to_commitment`,
`close_request`, or `only_requester_or_owner`'s guard.

**Recommendation:** Give both a real per-contract test suite matching
the depth already established for the other five (5–14 tests each) —
particularly for `funding-request-registry`, since this is exactly the
kind of contract where an untested guard already regressed once (the
`get_caller()` fix — see Summary) without a dedicated suite to catch it
sooner.

### L-2: No process guard against a future contract missing the `immediate_caller_key()` pattern

**Where:** Suite-wide convention, not a single file.

**Issue:** `funding-request-registry` shipped without the
`immediate_caller_key()` fix already applied everywhere else, simply
because it was written in the same PR that introduced `d3rac-hub`
*after* the systemic fix, and nobody re-derived the need for it in the
new file. This is a real, already-observed recurrence risk, not a
hypothetical one.

**Recommendation:** A short, explicit note in
`contracts/casper/README.md`'s contributor-facing guidance (a "new
contract checklist") stating plainly: any entry point gating on
admin/owner/role identity must resolve the caller via
`immediate_caller_key()`, never plain `Key::from(runtime::get_caller())`
— with a one-line reason and a pointer to whichever contract's
implementation to copy. Cheaper than relying on a wiring test to catch
the omission after the fact each time.

### L-3: `Key`-variant confusion (`ContractHash`/`AddressableEntityHash`/`ContractPackageHash`) is a recurring, easy-to-hit footgun

**Where:** Suite-wide — this exact mismatch has independently caused
real, CI-caught bugs at least three separate times in this suite's
history (a test minting to the wrong hash variant, a Hub cross-contract
call passing the wrong hash variant, an import-path error for
`PackageHash`/`ContractHash` in two different files).

**Issue:** Casper's post-Entity-model API surface makes it easy to
construct a syntactically valid `Key` that represents the wrong kind of
identity (a specific contract *version* vs. its *package*) for a given
call site, with no compiler error — only a runtime revert, discovered
only when actually exercised. Given how consistently this has bitten
this specific codebase, it's a documented, real pattern risk, not a
one-off.

**Recommendation:** A short, explicit note (same location as L-2)
naming this exact failure mode with a concrete example of getting it
right, so the next contributor (or contract) doesn't have to
rediscover it via another CI round-trip.

### L-4: `d3rac-token`'s `balances` dictionary intentionally diverges from the CEP-18 standard's own storage-key derivation

**Where:** `d3rac-token/src/main.rs`'s own module comment, already
self-documented.

**Issue:** Not a new finding — already disclosed candidly in-code — but
worth restating in an audit context: a generic CEP-18 block explorer or
tool expecting the standard's exact base64/CLType-bytes balance-key
derivation won't be able to read balances by raw storage query against
this contract, only via its `balance_of` entry point. Every
entry-point-level interaction (what this suite itself needs) is
unaffected.

**Recommendation:** No action required unless third-party CEP-18
tooling compatibility becomes a real requirement later; if so, revisit
the already-documented tradeoff.

---

## Informational

### I-1: `risk-registry`'s fixed-point `H×E×V` math is a genuinely good pattern, worth using as the suite's own reference

`risk-registry`'s risk-score calculation deliberately widens to `u128`
for the intermediate `H×E×V` product before scaling back down,
specifically to avoid the same class of overflow M-1 flags elsewhere —
already reasoned about and mitigated in that one function. Worth
holding up as the pattern any future arithmetic-heavy entry point in
this suite should match, rather than treating M-1's fix as a
box-ticking exercise.

### I-2: Reentrancy guards are applied precisely where needed, not everywhere reflexively

Only `disbursement-controller::release_milestone` (calls out to a
CEP-18 token) and `multisig-admin::execute_transaction` (calls out to
an arbitrary target) make external cross-contract calls from within an
entry point, and both — and only both — carry a reentrancy guard. No
contract without an outbound call carries a needless guard. Correct,
minimal application of the pattern.

### I-3: Not yet deployed to `casper-test`, and no professional third-party audit

Same standing caveat the TRON audit carries. See
`contracts/casper/README.md` for current, itemized deployment status —
as of this writing, no deploy script or CI workflow exists yet to drive
an actual `casper-test` deployment (unlike TRON, which has both,
deployed as of 2026-09-03 — see
[`docs/deployment-guide.md`](deployment-guide.md)).

### I-4: This pass, like the TRON one, is a self-review, not a substitute for a funded, independent audit

Repeating the TRON audit's own framing deliberately: a self-review by
the same team (or, here, the same AI assistant across many sessions)
that wrote the code is structurally limited in exactly the ways an
independent reviewer isn't — shared blind spots, familiarity bias, and
no adversarial incentive to find the worst possible framing of a
finding. Treat this document as narrowing the search space for a paid
audit, not replacing the need for one.

---

## Recommended order before treating this suite as production-representative

1. ~~Resolve M-1~~ — **done in this pass** (`overflow-checks = true`,
   plus `checked_add` at the four cited call sites).
2. Add the missing test suites for `d3rac-hub` and
   `funding-request-registry` (L-1) — cheapest way to catch the next
   instance of L-2/L-3 before it reaches `main`.
3. Add the contributor-facing checklist entries for L-2/L-3.
4. Build and run an actual `casper-test` deployment (no script/workflow
   exists yet — see I-3), using the exact wiring order
   `d3rac-hub-tests` already proves correct.
5. Bring this document, the TRON one, and both suites to a funded,
   independent audit firm before either touches real funds.
