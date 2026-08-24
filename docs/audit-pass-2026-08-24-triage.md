# Manual Triage — 2026-08-24 Automated Audit Findings

Reviews the automated findings in
[`docs/audit-reports/2026-08-24-automated.md`](audit-reports/2026-08-24-automated.md)
(Slither, cargo-audit, npm-audit, pip-audit) and records what was
applied vs. reviewed-and-rejected, with reasoning. Automated reports
are regenerated daily and shouldn't be hand-edited; this file is where
the human/agent judgment calls about them live.

## Applied

- **`MultiSigAdmin.threshold` → `immutable`** (Slither: immutable-states).
  Assigned exactly once, in the constructor, never reassigned. Verified
  by grep across the file — only one `threshold = threshold_;`.
- **`Migrations.owner` → `immutable`** (Slither: immutable-states). Same
  reasoning — one `owner = msg.sender;` in the constructor, no other
  writes.
- **`setuptools` minimum bumped to `>=83.0.0`** in `agents/python/pyproject.toml`
  (pip-audit: PYSEC-2026-3447, 2 CVEs in the 79.0.1 that had been
  resolving under the old `>=68` floor). 83.0.0 is upstream's fix
  version.

## Reviewed and NOT applied (false positives / accepted risk)

- **`Migrations.last_completed_migration` naming-convention** (Slither).
  This is TronBox/Truffle's own standard scaffold naming — TronBox's
  migration tooling reads this contract by convention. Renaming it
  would be cosmetic and risks silently breaking `tronbox migrate`.
  Left as-is with a comment explaining why.
- **`D3RACProperties._setRole` / `_revokeRole` "dead code"** (Slither).
  Checked with `grep -rn` across `contracts/tron/tronbox/contracts/`:
  both are actively called — `_setRole` from `IdentityRegistry.sol`,
  `D3RACToken.sol`, `DisbursementController.sol`; `_revokeRole` from
  `RiskRegistry.sol`, `FundingRequestRegistry.sol`. This looks like an
  artifact of how Slither was invoked (e.g. per-file rather than
  whole-project analysis) rather than a real finding. **Not removed** —
  removing them would have broken five contracts.
- **`MultiSigAdmin.executeTransaction` reentrancy-eth** (Slither).
  Flags `t.executed = false` being written after the external call, in
  the failure branch. That branch immediately `revert()`s, which undoes
  the write along with everything else in the transaction — and the
  function already carries a `nonReentrant` guard on top of that. Not a
  real exploit path; left as-is.
- **`bincode` unmaintained (RUSTSEC-2025-0141)** (cargo-audit). Traced
  via `Cargo.lock`: it's a transitive dependency of `casper-types`
  6.1.0 / `casper-execution-engine` / `casper-storage` — the exact
  pinned Casper SDK versions this suite depends on for
  `wasm32-unknown-unknown` compatibility (see
  `contracts/casper/README.md` for how hard-won that pin was). Not
  fixable from this repo without an upstream Casper SDK release;
  tracked as an accepted, upstream-owned risk rather than "fixed."

## Not attempted here (out of scope for this pass)

- `solc-version` warnings (`^0.8.20` flagged for known compiler bugs) —
  a compiler-version bump is a bigger, separately-tested change, not a
  same-PR drive-by fix.
- `timestamp`-for-comparisons findings — in every flagged case the
  comparison isn't actually using `block.timestamp` for time-windowing
  logic (e.g. comparing `msg.sender`, array length), so these read as
  Slither's detector matching on unrelated `require()` calls in the
  same functions as genuine timestamp usage elsewhere. Worth a closer
  read in a dedicated pass, not bundled into this cleanup.
