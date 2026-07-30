# D3R·AC — Casper Contract Suite

**Status: early, unverified, in progress.** This is not a parallel,
complete implementation of the TRON suite yet — it's the first
contract of seven, written carefully against documented patterns but
**not yet confirmed to compile**, let alone tested against a local
Casper network or deployed to testnet. See "What's actually done"
below for the honest breakdown, and
[`docs/casper-contracts-srs.md`](../../docs/casper-contracts-srs.md)
for the full requirements this suite is being built against.

## Why this suite can't be verified the way `contracts/tron/` was

`contracts/tron/`'s Hardhat 3 migration hit real, repeated failures
across several CI round-trips before going green — that's the
expected, normal way to get unfamiliar toolchain/API surface right,
not a sign of unusual brokenness. This suite is starting from the same
place, with one added constraint: **there is no way to compile Rust to
`wasm32-unknown-unknown` in the sandbox this was written in at all** —
`rustup`'s installer domain isn't reachable from it, and the OS
package manager doesn't ship Rust cross-compilation targets. Even
`cargo check` against the *native* target doesn't fully exercise a
`#![no_std]` contract's real compile path.

That means the code in `risk-registry/` was written against
documented `casper-contract`/`casper-types` patterns (the official
Casper docs' counter-contract example, `docs.rs` API references, and
the exact pinned crate versions below, all checked against currently
published crates.io versions) but has **not** been confirmed to
actually compile. The CI job added for this
(`.github/workflows/d3rac-ci.yml`'s `contracts-casper` job) is the
first real check — it installs the `wasm32-unknown-unknown` target
(something GitHub's runners, unlike this sandbox, can actually do) and
attempts a real build. Treat early red runs here as expected first-pass
feedback to iterate on, the same way the Hardhat 3 migration was.

## Pinned crate versions

**Correction from this project's earlier notes**: `casper-contract`
5.1.1 requires `casper-types ^6.0.1` (confirmed via crates.io's own
dependency listing) — **not** 7.0.0, which is what prior notes said
was pinned alongside it. Using 7.0.0 directly caused two incompatible
`casper-types` versions in the same dependency graph (a real CI build
failure — "expected `EntryPoints`, found `casper_types::EntryPoints`",
two structurally-identical but distinct types from different crate
versions). Fixed; verified via `cargo generate-lockfile` that exactly
one `casper-types` version now resolves across the whole graph.

| Crate | Version |
|---|---|
| `casper-contract` | 5.1.1 |
| `casper-types` | 6.1.0 |
| `casper-event-standard` | 0.7.0 |
| `casper-engine-test-support` (dev/test only) | 8.1.1 |

## What's actually done

- [x] Cargo workspace scaffold (`contracts/casper/Cargo.toml`)
- [x] `risk-registry/` — full source written (installer, all entry
      points, error type, event definitions, on-chain record type),
      targeting behavioral parity with
      `contracts/tron/tronbox/contracts/RiskRegistry.sol` (FR-6). This
      is the SRS's own pick for "start here" — standalone, no
      dependency on any other contract in the suite.
- [x] CI job to actually attempt a `wasm32-unknown-unknown` build
- [ ] **Confirmed compiling** — pending first real CI run
- [ ] Unit/integration tests against a local Casper network
      (`casper-engine-test-support`)
- [ ] The other six contracts (`D3RACToken`/CEP-18, `IdentityRegistry`,
      `DisbursementController`, `MultiSigAdmin`, `D3RACHub`,
      `FundingRequestRegistry`)
- [ ] Hub wiring (FR-8)
- [ ] `casperAdapter.ts` completion (FR-9) — still throws "not deployed
      yet", correctly, since nothing is deployed yet
- [ ] Casper Testnet deployment
- [ ] Professional security audit (matches the TRON suite's own
      status — see [`docs/audit-pass-2026-07-25.md`](../../docs/audit-pass-2026-07-25.md),
      which is explicitly a self-review, not a substitute for one)
- [ ] Mainnet deployment of anything in this suite — not to be
      considered until every item above is done, the same "no
      deploying real funds without a proper security review" posture
      `contracts/tron/README.md` states for the TRON side

## Design decisions (SRS §8 "Open decisions," resolved for this contract)

- **Event mechanism**: [Casper Event Standard](https://github.com/make-software/casper-event-standard)
  (CES), the community standard, over a hand-rolled dictionary
  emulation — readable by any CES-aware indexer or future data-pipeline
  integration without D3R·AC-specific tooling.
- **Upgradeability (NFR-2)**: this package is installed *locked*
  (`storage::new_locked_contract`, not `new_contract`) — the closest
  analog to the TRON suite's immutable-by-default Solidity contracts,
  and the safer default for something that gates real disaster-relief
  funding decisions. A future, deliberate decision to make a different
  Casper contract in this suite upgradeable should be its own
  documented choice made per-contract, not this one's default leaking
  across the suite.
- **Addressing**: roles (owner, data feeders) are keyed by whichever
  `Key` form `runtime::get_caller()` resolves to, formatted to a string
  for dictionary-item-key use — Casper's dictionary-item keys must be
  strings, unlike a Solidity `mapping(address => bool)`.

## Building locally

Requires a machine (or CI runner) that can actually target
`wasm32-unknown-unknown` — this sandbox cannot, per above.

```bash
cd contracts/casper
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

The compiled contract will be at
`target/wasm32-unknown-unknown/release/risk-registry.wasm`.
