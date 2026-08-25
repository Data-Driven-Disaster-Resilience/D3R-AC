# D3R·AC — Casper Contract Suite

**Status: early, in progress — two contracts confirmed via real CI
(`risk-registry`, `identity-registry`), two more merged after CI
rounds (`multisig-admin`, `d3rac-token`), a fifth written but not yet
CI-confirmed and carrying a real open design question
(`disbursement-controller`).** This is not a parallel, complete
implementation of the TRON suite yet. Not yet deployed to testnet, and
not audited. See "What's actually done" below for the honest, itemized
breakdown, and
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
- [x] **Confirmed compiling** — `risk-registry.wasm` builds successfully
      against `wasm32-unknown-unknown` in CI (`contracts-casper` job,
      commit `f4d1c2c`). Reached through several real, CI-verified
      iteration rounds (casper-types version conflicts, `EntryPoint`
      import paths, the addressable-entity `EntryPointType`/
      `EntityEntryPoint` naming, an unmaintained `wee_alloc` dependency
      swapped for `dlmalloc`, `wasm-ld` undefined-symbol errors) — the
      same iterate-on-real-compiler-feedback approach that got
      `contracts/tron`'s Hardhat 3 migration green, and worth noting:
      this file ended up being worked on by two parallel Claude
      sessions (this one, and a separate Claude Code session) across
      that iteration, which is *why* it converged as fast as it did.
- [x] Unit/integration tests against a local Casper network — **all 5
      passing, CI-confirmed**
      (`risk-registry-tests/tests/integration_tests.rs`: install,
      community registration + duplicate-rejection, risk-score
      computation, non-feeder rejection), in their own workspace
      package deliberately (not `risk-registry/tests/`) — building
      `risk-registry`'s `#![no_std]` binary for the native host target,
      which `cargo test` does by default for every workspace member
      unless scoped away, collided with the native toolchain's own
      panic handler (a real, confirmed CI error: "duplicate lang item
      `panic_impl`"). Getting to green took several real, CI-verified
      rounds beyond the build itself: a Wasm-level bug (Casper's engine
      rejects "bulk memory operations," which modern Rust emits by
      default — a known, tracked ecosystem gap, fixed via
      `wasm-opt --llvm-memory-copy-fill-lowering` post-processing) and
      an execution-context bug in this contract's own `call()` (Casper
      Event Standard's `init`/first `emit` were running in the
      installing account's context rather than the newly-created
      contract's own, silently breaking every later `emit()` call --
      found and fixed by a parallel Claude Code session working on this
      same file, traced against the actual crate source rather than
      guessed).
- [x] `identity-registry/` — full source written (installer, all entry
      points, error type, event definitions, on-chain record type),
      implementing SRS FR-2: admin-designated verifiers can verify a
      recipient account against a community label and revoke that
      verification. Two-step admin transfer (`propose_new_admin`/
      `accept_admin`) matches TRON's `IdentityRegistry.sol`
      `proposeNewAdmin`/`acceptAdmin` exactly (the M-2 fix on `main`).
      Followed `risk-registry/src/main.rs`'s already-CI-confirmed
      template for every hard-won pattern (global allocator, panic
      handler, CES event registration, `new_locked_contract`'s 5-arg
      signature, dictionary-backed storage, AccountHash-normalized
      addressing).
- [x] **Confirmed compiling** — `identity-registry.wasm` builds
      successfully against `wasm32-unknown-unknown` in CI, using the
      now-generalized `contracts-casper` job (see below).
- [x] Unit/integration tests against a local Casper network — **all 9
      passing, CI-confirmed**
      (`identity-registry-tests/tests/integration_tests.rs`).
- [x] CI generalized: the Casper build/test job was hardcoded to
      `-p risk-registry`; it now discovers every contract package in
      the workspace and builds/lowers-bulk-memory-ops/stages/tests all
      of them, so each new contract in the SRS doesn't need its own CI
      edit going forward.
- [x] `multisig-admin/` -- full source written (installer, all entry
      points, error type, event definitions, on-chain record type),
      targeting behavioral parity with FR-4 /
      `contracts/tron/tronbox/contracts/MultiSigAdmin.sol`: fixed
      N-of-M owner set, `submit_transaction` (auto-confirms from the
      submitter), `confirm_transaction`/`revoke_confirmation`,
      `execute_transaction` once a transaction clears `threshold`
      confirmations. Followed `identity-registry/src/main.rs`'s
      already-CI-confirmed template for every hard-won pattern.
- [ ] **NOT yet confirmed compiling.** Unlike `risk-registry` and
      `identity-registry`, this one hasn't had a CI round yet at all --
      see `src/main.rs`'s own module comment for specifics on what's
      least certain about it. Two things worth flagging explicitly:
      - It has one real behavioral difference from the TRON contract,
        not just a translation detail: `MultiSigAdmin.sol` takes
        `to`/`value`/`bytes data` and does a raw, dynamically-typed
        EVM `call`; Casper contract calls are typed and
        entry-point-addressed. `execute_transaction` bridges this by
        having `submit_transaction` take a target *entry-point name*
        plus bytesrepr-serialized `RuntimeArgs`, deserialized back at
        execution time -- see `main.rs`'s `execute_transaction` doc
        comment for the full reasoning. This specific mechanism
        (`RuntimeArgs::from_bytes` round-tripping, then
        `runtime::call_versioned_contract`) is the least-tested part
        of this contract -- no test in this suite has exercised a real
        cross-contract call yet, on either the TRON or Casper side.
      - This session tried to get a real compile in its own sandbox
        (further than "not possible at all" -- `apt`'s `cargo`/`rustc`
        1.75.0 *can* resolve most of this workspace's dependency
        graph, further than expected) but hit a hard wall: `zeroize`
        1.9.0, already pinned in `Cargo.lock` by the earlier
        CI-verified contracts, requires Cargo's `edition2024` feature,
        which stabilized after 1.75. Getting a newer toolchain needs
        `rustup` (`static.rust-lang.org`, still unreachable from this
        sandbox) -- so this remains a sandbox-only limitation, not a
        statement about the code. GitHub's CI runners already handle
        this fine for the other two contracts; this is the real,
        first test for `multisig-admin`.
- [x] `d3rac-token/` -- full source written, targeting CEP-18-standard
      parity (FR-1) with `contracts/tron/tronbox/contracts/D3RACToken.sol`:
      standard `transfer`/`approve`/`transfer_from`/`balance_of`/
      `allowance`/`total_supply`/`name`/`symbol`/`decimals`, plus the
      non-standard owner-gated `mint`/`set_minter` and two-step
      ownership this suite's other contracts already use. Uses
      `casper_types::U256` for amounts. Standard CEP-18 semantics
      (revert-on-failure, `Unit` returns) rather than `D3RACToken.sol`'s
      ERC-20-style `bool` returns -- see `src/main.rs`'s module comment.
- [ ] **NOT yet confirmed compiling.** Written against the same
      casper-types 6.1.0 API surface `multisig-admin` needed two real
      CI-caught fix rounds for (see that contract's PR #20 history) --
      applied here from the start rather than re-discovered, but that's
      not a substitute for this file's own CI round, which hasn't
      happened yet as of this write-up.
- [x] `disbursement-controller/` -- full source written, targeting
      parity (FR-3) with `contracts/tron/tronbox/contracts/
      DisbursementController.sol`: milestone-based commitments for a
      recipient verified live against `identity-registry` (a real
      cross-contract call, not a cached flag), attester-gated
      milestone attestation, permissionless release once attested.
      Two real simplifications, not just translation details -- see
      `src/main.rs`'s module comment: no `_safeTransfer`
      return-value-tolerance workaround needed (CEP-18's
      revert-on-failure semantics don't have the ambiguity that
      protects against), and this contract's own package hash stands
      in for Solidity's `address(this)` when tracking its own token
      balance.
- [ ] **NOT yet confirmed compiling, and with a real open question**
      beyond the usual "first CI round" uncertainty: `release_milestone`
      assumes `d3rac-token`'s `runtime::get_caller()` resolves to
      *this* contract's own identity during the nested `transfer`
      call, not the original externally-owned account. This is new,
      untested territory for this suite -- every cross-contract call
      so far (`multisig-admin`) was about getting the call to compile
      and execute, not about caller-identity semantics inside the
      callee. If this assumption is wrong, the token debit would
      target the wrong account. Needs a real local-network integration
      test to confirm, not just a compile pass.
- [ ] The other two contracts (`D3RACHub`, `FundingRequestRegistry`)
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
cargo build --release --target wasm32-unknown-unknown -p risk-registry
```

**One required post-processing step, not optional.** Rust's default
codegen for `wasm32-unknown-unknown` can emit "bulk memory operations"
(`memory.copy`/`memory.fill` — via the `dlmalloc` allocator, or LLVM's
own optimizer under this workspace's LTO release profile), which
Casper's execution engine rejects outright at deploy time:
`WasmPreprocessing(Deserialize("Bulk memory operations are not
supported"))`. This is a known, tracked ecosystem gap (see
[casper-network/casper-node#4367](https://github.com/casper-network/casper-node/issues/4367):
"Modern compilers i.e. rust and golang are emitting these extensions
by default"), not something specific to this contract's code —
disabling the target-feature at the `rustc` level (see
`.cargo/config.toml`) does **not** fix it, since the instructions can
come from a prebuilt object blob bundled in the Rust toolchain's own
sysroot for this target (confirmed via
[rust-lang/rust#140971](https://github.com/rust-lang/rust/issues/140971),
a real upstream issue about exactly this), immune to this crate's own
`rustflags`.

The fix: post-process the compiled `.wasm` with
[Binaryen](https://github.com/WebAssembly/binaryen)'s `wasm-opt
--llvm-memory-copy-fill-lowering`, which rewrites `memory.copy`/
`memory.fill` into equivalent byte-loop instructions using only WASM
MVP-compatible opcodes. This needs a newer Binaryen than most package
managers currently ship (this flag needs v109+; e.g. Ubuntu noble's
`apt` package is v108) — download a
[recent release](https://github.com/WebAssembly/binaryen/releases)
directly if your package manager's version is too old:

```bash
wasm-opt target/wasm32-unknown-unknown/release/risk-registry.wasm \
  --enable-bulk-memory --llvm-memory-copy-fill-lowering -Oz \
  -o target/wasm32-unknown-unknown/release/risk-registry.wasm
```

The compiled contract will be at
`target/wasm32-unknown-unknown/release/risk-registry.wasm`.

## Default testnet deployer (casper-test)

A throwaway testnet-only account is the project's default deployer for
CI-driven testnet deploys, once there's a deploy script to drive
(there isn't one yet — see "What's left"). Its public key and account
hash are public information and safe to record here; its secret key is
**not** in this repo — it's stored as the `CASPER_TESTNET_SECRET_KEY`
encrypted GitHub Actions secret, consistent with NFR-1 in
[`docs/casper-contracts-srs.md`](../../docs/casper-contracts-srs.md).

| | |
|---|---|
| Public key (account key) | `014e7ce46b68c09af0f7be462bd13bf73ae018f527604ff78641ca00ca4d6b0e6f` |
| Account hash | `account-hash-23b198073de7006164021ea69f7901482c272499889e05424a8b3eba59d3acf9` |

Fund it via the [Casper testnet faucet](https://testnet.cspr.live/tools/faucet)
before any workflow that deploys with it. Do not reuse this key for
mainnet.
