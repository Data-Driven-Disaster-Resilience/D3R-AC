# D3R·AC — Casper Contract Suite

**Status: early, in progress — four of seven contracts now compile and
pass their local-network tests.** This is not a parallel, complete
implementation of the TRON suite yet — `risk-registry`,
`identity-registry`, `disbursement-controller`, and `d3rac-token` are
confirmed via real CI to build against `wasm32-unknown-unknown` and
pass every one of their integration tests against a local Casper
network (41 tests total). Not yet deployed to testnet, and not
audited. See "What's actually done" below for the honest, itemized
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
      edit going forward. Also generalized to stage EVERY built
      contract's `.wasm` into EVERY `*-tests` package (not just its
      own same-named pair) — needed once `disbursement-controller-tests`
      had to install a real `identity-registry` alongside it to
      exercise a genuine cross-contract call, not a stub.
- [x] `disbursement-controller/` — full source written, implementing
      SRS FR-3: milestone-based fund release gated by an attester
      role, checking recipient verification via a real cross-contract
      call into `identity-registry`'s `is_verified` (confirmed against
      Casper's own cross-contract-communication docs, not guessed).
      No SafeERC20-style tolerant-decode wrapper needed the way
      `DisbursementController.sol`'s `_safeTransfer` (M-3) requires —
      CEP-18's `transfer` entry point simply reverts on failure
      (confirmed against the CEP-18 standard text directly), unlike
      ERC-20/TRC-20's return-bool ambiguity. Same two-step admin
      transfer and reentrancy guard as the rest of this suite.
- [x] **Confirmed compiling** — `disbursement-controller.wasm` builds
      successfully against `wasm32-unknown-unknown` in CI. Took three
      real, CI-verified fix rounds: `casper_types::ContractHash` isn't
      exported from the crate root post-Entity-model migration (needed
      `AddressableEntityHash` in most places, with a `.into()` at the
      couple of spots `runtime::call_contract` itself still wants the
      legacy `ContractHash` type), plus the same mistake repeated in
      the test file before `ContractHash`'s real path
      (`casper_types::contracts::ContractHash`) was confirmed.
- [x] Unit/integration tests against a local Casper network — **all 14
      passing, CI-confirmed**
      (`disbursement-controller-tests/tests/integration_tests.rs`),
      installing a genuine `identity-registry` alongside it rather
      than stubbing the cross-contract call. The previous "no CEP-18
      token existed yet" gap in this test file's own coverage is
      closed as of `d3rac-token` landing (see below) — one of these 14
      tests now installs a real `d3rac-token` too and exercises the
      full fund-release path end to end, including the
      unfunded-contract rejection `release_milestone`'s "no explicit
      `balance_of` pre-check" design decision relies on CEP-18's own
      revert for.
- [x] `d3rac-token/` — full source written, implementing SRS FR-1: a
      complete CEP-18 token (`ceps/text/0018-token-standard.md`) — all
      11 standard entry points, all 7 standard events, and the
      standard's own exact error codes (`InsufficientBalance=60001`,
      `InsufficientAllowance=60002`, `CannotTargetSelfUser=60003`),
      confirmed directly against the spec text rather than guessed.
      Plus `D3RACToken.sol`'s own extensions: minter-role-gated `mint`,
      public `burn`, two-step ownership transfer. **One documented,
      deliberate point of non-compliance**: `balances`' dictionary
      values are keyed with this suite's own established
      `Key::to_string()` pattern rather than the standard's
      base64-CLType-bytes scheme — full entry-point composability
      (what this suite actually needs) is preserved, only raw-storage
      introspection by generic external CEP-18 tooling isn't. This
      was a real, deliberate choice for `balances` (which only ever
      stores one `Key`'s worth of bytes per entry, well under Casper's
      dictionary-item-key length limit either way) — but turned out
      **not** to be optional for `allowances`: concatenating two `Key`
      Display strings for that dictionary's keys exceeded the length
      limit in practice (a real `ApiError::DictionaryItemKeyTooLarge`
      from CI), so `allowances` uses the standard's exact blake2b-hash
      derivation instead, confirming the standard's own reasoning for
      picking that scheme in the first place.
- [x] **Confirmed compiling** — `d3rac-token.wasm` builds successfully
      against `wasm32-unknown-unknown` in CI. Two real, CI-verified fix
      rounds beyond the dictionary-key one above: a missing
      `bytesrepr::ToBytes` trait import (needed to call
      `Key::to_bytes()`), and sidestepping an unconfirmed
      `U256::pow()` signature by hardcoding `10^18` as a literal for
      supply scaling rather than guessing at another API.
- [x] Unit/integration tests against a local Casper network — **all 13
      passing, CI-confirmed**
      (`d3rac-token-tests/tests/integration_tests.rs`).
- [ ] The other three contracts (`MultiSigAdmin`, `D3RACHub`,
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
cargo build --release --target wasm32-unknown-unknown -p risk-registry -p identity-registry -p disbursement-controller -p d3rac-token
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
for pkg in risk-registry identity-registry disbursement-controller d3rac-token; do
  wasm-opt target/wasm32-unknown-unknown/release/${pkg}.wasm \
    --enable-bulk-memory --llvm-memory-copy-fill-lowering -Oz \
    -o target/wasm32-unknown-unknown/release/${pkg}.wasm
done
```

The compiled contracts will be at
`target/wasm32-unknown-unknown/release/{risk-registry,identity-registry,disbursement-controller,d3rac-token}.wasm`.
