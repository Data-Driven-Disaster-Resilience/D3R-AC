# D3R·AC — Casper Contract Suite

**Status: early, in progress — all seven contracts now have source
written and confirmed compiling via real CI. Six have integration
test suites (56 tests total: `risk-registry` 5, `identity-registry` 9,
`disbursement-controller` 14, `d3rac-token` 13, `multisig-admin` 14 --
including a genuine cross-contract `execute_transaction` call against
a real `identity-registry` -- and `d3rac-hub` 1: a single
comprehensive test installing all seven contracts, wiring the Hub to
all five modules, and proving a full admin handoff to a 1-of-1
multisig via a real Hub-mediated call). `funding-request-registry`
has no integration test suite yet. A systemic
caller-resolution bug (`runtime::get_caller()` instead of
`runtime::get_immediate_caller()`, meaning a contract caller like
`multisig-admin` or the Hub couldn't be correctly recognized by an
admin/owner check) was found and fixed across all five contracts that
had it.** Not yet deployed to Casper testnet, and not audited. See
"What's actually done" below for the honest, itemized breakdown, and
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
      edit going forward.      edit going forward. Also generalized to stage EVERY built
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
      than stubbing the cross-contract call. One test additionally
      installs a real `d3rac-token` and confirms `release_milestone`
      rejects when this contract holds none of the configured token
      — the guard `release_milestone`'s "no explicit `balance_of`
      pre-check" design decision relies on CEP-18's own revert for.
      **The funded-success path is not yet covered** — see the ⚠️
      below.
- [ ] **⚠️ Funded-success path for `release_milestone` still not
      tested, though its root cause is now fixed** — an earlier
      attempt to fund `disbursement-controller` and release surfaced
      two real, separate issues. First: Casper's
      `runtime::get_caller()` always resolves to the originating
      deploy-signing account, never the immediate calling contract —
      this is now fixed suite-wide (see the "Systemic fix" bullet
      below), including in `d3rac-token`'s own `transfer`. Second, once
      that fix was in place: the fix resolves a calling contract's
      identity to `Key::from(ContractPackageHash)` (the contract's
      *package* identity), but the original test attempt minted to
      `Key::from(ContractHash::from(dc_hash))` (the contract's
      specific-*version*/entity identity) — two different identifiers
      in Casper's model that don't compare equal as dictionary keys.
      That second, narrower `Key`-encoding question is the only thing
      still blocking this specific test from being rewritten to assert
      the real success path — real, worthwhile follow-up, not
      abandoned.
- [x] **Systemic fix: `runtime::get_caller()` → immediate-caller
      resolution, across all five contracts with source**. Building
      `multisig-admin-tests`' own cross-contract test
      (`should_execute_a_confirmed_transaction_against_a_real_contract`,
      installing a real `identity-registry` and exercising
      `execute_transaction`'s call for real) surfaced that the fix
      above wasn't actually applied everywhere it needed to be —
      `identity-registry`'s `only_admin`/`only_verifier`/`accept_admin`,
      `disbursement-controller`'s `only_admin`/`only_attester`/
      `accept_admin`, `risk-registry`'s `only_owner`/`only_data_feeder`,
      `multisig-admin`'s own `only_owner`, and even `d3rac-token`'s
      `only_owner`/`only_minter`/`accept_ownership` (the earlier fix
      only covered `transfer`/`transfer_from`/`approve`) all still used
      plain `get_caller()`. Every one of these gates an admin/owner/role
      check that can legitimately need to recognize a CONTRACT (most
      importantly `multisig-admin` itself, after a two-step admin
      transfer — the production pattern
      [`docs/deployment-guide.md`](../../docs/deployment-guide.md)
      recommends) as the acting party, not just a human account — with
      plain `get_caller()`, that transfer-to-a-multisig pattern this
      whole suite recommends would have silently and permanently locked
      every affected contract's admin functions the moment ownership
      actually moved to a multisig, since the multisig's own
      `execute_transaction` calls would never be recognized as coming
      from the right caller. Fixed identically across all five files
      (a shared `immediate_caller_key()` pattern, each with its own
      `UnrecognizedCallerKind` error variant) — **not yet independently
      compiled or CI-confirmed as of this write-up.**
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
- [x] `multisig-admin/` -- full source written and **confirmed
      compiling and passing all 14 of its integration tests**,
      including a genuine cross-contract `execute_transaction` call
      against a real `identity-registry` -- see the "Systemic fix"
      entry above for what that test surfaced and how it was fixed.
      Behavioral parity with FR-4 /
      `contracts/tron/tronbox/contracts/MultiSigAdmin.sol`: fixed
      N-of-M owner set, `submit_transaction` (auto-confirms from the
      submitter), `confirm_transaction`/`revoke_confirmation`,
      `execute_transaction` once a transaction clears `threshold`
      confirmations. One real behavioral difference from the TRON
      contract, not just a translation detail: `MultiSigAdmin.sol`
      takes `to`/`value`/`bytes data` and does a raw, dynamically-typed
      EVM `call`; Casper contract calls are typed and
      entry-point-addressed. `execute_transaction` bridges this by
      having `submit_transaction` take a target *entry-point name*
      plus bytesrepr-serialized `RuntimeArgs`, deserialized back at
      execution time -- see `main.rs`'s `execute_transaction` doc
      comment for the full reasoning.
- [x] `funding-request-registry/` -- full source written (installer,
      all entry points, error type, event definitions, on-chain record
      type), targeting behavioral parity with
      `contracts/tron/tronbox/contracts/FundingRequestRegistry.sol`: a
      public funding-request board, proposer-role-gated `open_request`,
      owner-or-requester-gated `record_pledge`/`link_to_commitment`/
      `close_request` with the same automatic
      Open->PartiallyFunded->Funded status transition. Two-step owner
      transfer (`propose_new_owner`/`accept_ownership`), matching
      `FundingRequestRegistry.sol`'s own already-two-step
      `proposeNewOwner`/`acceptOwnership` -- copied from
      `identity-registry`'s already-CI-confirmed
      `propose_new_admin`/`accept_admin` implementation, not
      re-derived.
- [x] **Confirmed compiling** against `wasm32-unknown-unknown` in
      CI (took two real fixes after the first CI run: `PackageHash`
      isn't `contracts::PackageHash`, and `ContractHash` -- a
      *different* mismatch, opposite direction, found on `d3rac-hub`
      below, not this file -- isn't at the crate root; both confirmed
      against `casper-types`' and `casper-contract`'s actual published
      source, not re-guessed a third time). No integration test suite
      yet -- deliberately scoped out of this same pass rather than
      rushed.
- [x] `d3rac-hub/` -- full source written (installer, all ~37 entry
      points, error type, event definitions, aggregate-status view
      type), targeting behavioral parity with FR-8 /
      `contracts/tron/tronbox/contracts/D3RACHub.sol`: single admin
      surface (two-step transfer), emergency pause, module
      get/set for all five other contracts (`risk_registry`/
      `funding_request_registry` as `Option<Key>` -- Casper's
      idiomatic "may be absent" over trying to construct a fake "zero
      address" `Key`), and a thin pass-through orchestration layer over
      every underlying contract's own entry points via
      `runtime::call_contract`.
- [x] **Confirmed compiling** against `wasm32-unknown-unknown` in
      CI, after two real fixes: `runtime::call_contract` needs a
      `ContractHash`, not the `AddressableEntityHash` this file's
      `key_to_contract_hash` helper originally returned (missed
      copying `disbursement-controller`'s own call-site `.into()`);
      and once fixed to return `ContractHash` explicitly, that type
      turned out to live under `casper_types::contracts`, not the
      crate root (confirmed against the real published source both
      times, not re-guessed).
- [x] `d3rac-hub-tests/` -- one comprehensive integration test:
      installs all seven contracts, wires all five modules to the Hub
      (grant-role -> propose -> accept, matching
      `2_deploy_d3rac.js`'s own sequence; `risk-registry` single-step,
      per this file's own header), hands the Hub's admin to a 1-of-1
      `multisig-admin` (the same topology the real Shasta deployment
      actually uses), then proves the handoff really took effect --
      not by reading the Hub's admin back (see the next entry), but by
      showing the exact same `register_community` call that succeeded
      when `DEFAULT_ACCOUNT_ADDR` submitted it directly now fails the
      same way, and succeeds again when routed through the multisig.
      That contrast exercises immediate-caller resolution at both hops
      at once (multisig -> Hub, then Hub -> risk-registry) -- the exact
      mechanism the systemic fix above provides and this Hub was
      written to depend on from the start, now actually exercised for
      real rather than only reasoned about.
- [ ] **Real gap found while writing that test, not by writing it:**
      `d3rac-hub` has no entry point that reads back its own current
      `admin` -- `system_status()` returns the five module addresses,
      `is_paused`, and some aggregate counts, but not `admin` itself.
      A worthwhile small follow-up (an `admin()` view, mirroring
      `risk-registry`'s own plain getters), not attempted in this same
      pass.
- [x] The caller-resolution dependency this entry originally flagged
      (Hub calls depending on the callee recognizing the Hub itself as
      caller, not the original signing account) is resolved -- see the
      "Systemic fix" entry above -- and now actually exercised by a
      real test (previous entry), not just reasoned about. This file's
      own `only_admin` used the correct pattern from the start
      (`immediate_caller_key`, written into this file before the
      systemic fix above was even found, let alone merged).
- [x] Hub wiring (FR-8) -- proven to work by
      `d3rac-hub-tests` above, against a local Casper network. Still
      undone: actually deploying to Casper testnet and wiring the real,
      on-chain instances together the same way.
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
`target/wasm32-unknown-unknown/release/{risk-registry,identity-registry,multisig-admin,disbursement-controller,d3rac-token}.wasm`.

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
