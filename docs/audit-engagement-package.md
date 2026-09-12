# D3R·AC — Security Audit Engagement Package

**Prepared:** September 2026
**Repository:** `Data-Driven-Disaster-Resilience/D3R-AC`
**HEAD at time of writing:** `ae8a81f217d6eba2c72162de7545a7e9e2cf1101`
(already on the real `main` branch — every reference below resolves
directly, no push required first, unlike an earlier version of this
document).

---

## 0. How to read this document

Every specific claim below (line counts, test counts, commit
references, deployment run links) was pulled directly from the
repository's actual state at the commit hash above — not estimated,
not carried over from memory. Section 7 shows exactly how to reproduce
every number.

This package was assembled by an AI assistant (Claude) working
directly in this repository, not by a licensed human security
professional. Every finding, fix, and test result below is either (a)
something a real, deterministic tool produced (a compiler, a test
runner, Slither, cargo-audit) and is cited directly, or (b) explicitly
labeled as reasoning rather than an executed check. **This document is
preparation for a professional audit — it is explicitly not a
substitute for one**, the same posture this repository's own
[`docs/audit-pass-2026-07-25.md`](audit-pass-2026-07-25.md) and
[`docs/audit-pass-2026-09-09.md`](audit-pass-2026-09-09.md) already
take toward their own internal reviews.

---

## 1. What this project is

D3R·AC ("Data-Driven Disaster Resilience for All Communities") is a
multi-chain relief-fund disbursement platform, deployed in parallel on
two chains:

- **TRON** (Solidity, EVM-compatible) — 9 contracts, real Hardhat test
  suite, **deployed to Shasta testnet** (2026-09-03).
- **Casper** (Rust, WASM, non-EVM) — 7 contracts, real integration
  test suite against actual compiled `.wasm` binaries, **deployed to
  Casper's public testnet** (2026-09-07).

Both deployments used a deliberately minimal 1-of-1 multisig as the
admin/owner across the system — a real, intentional starting
configuration (no second trusted keyholder was available yet at
deploy time), not a placeholder or an oversight. **Neither deployment
should be mistaken for production-readiness.**

The system coordinates: a fungible relief token, an identity/KYC-style
recipient registry, a milestone-gated disbursement controller, a
multisig admin layer, a community risk-scoring registry, a
funding-request coordination layer, and a Hub contract wiring all of
the above together under one admin surface. **This is pre-audit
software that has never held real funds and is not deployed to any
mainnet.**

---

## 2. Codebase inventory (exact, as of `ae8a81f`)

### 2.1 TRON / Solidity — `contracts/tron/tronbox/contracts/`

| Contract | Purpose |
|---|---|
| `D3RACHub.sol` | Coordinator — wires all other modules, single admin surface |
| `DisbursementController.sol` | Milestone-based, attester-gated fund release |
| `FundingRequestRegistry.sol` | Funding-request lifecycle, thin coordination layer |
| `RiskRegistry.sol` | Community risk scoring (hazard × exposure × vulnerability) |
| `MultiSigAdmin.sol` | N-of-M multisig, generic target/entry-point dispatch |
| `D3RACToken.sol` | Relief-fund token (custom, not a stock OpenZeppelin ERC-20) |
| `IdentityRegistry.sol` | Recipient verification / revocation |
| `D3RACTokenFactory.sol` | CREATE2 vanity-address token deployment (optional, additive) |
| `Migrations.sol` | TronBox scaffolding |

**~1,960 lines total** across these 9 files. **123 tests**, real
Hardhat suite (`contracts/tron/test/`), executed against solc 0.8.20
with `evmVersion: "paris"` pinned (see Finding TRON-L3 below for why
that pin matters). Reproduce:
```bash
cd contracts/tron && npm install && npm test
```

### 2.2 Casper / Rust — `contracts/casper/*/src/main.rs`

| Contract | Purpose |
|---|---|
| `d3rac-hub` | Coordinator — ~30 pass-through entry points to the other 5 modules |
| `disbursement-controller` | Milestone-based fund release; calls `identity-registry` and `d3rac-token` |
| `d3rac-token` | CEP-18-shaped relief token |
| `risk-registry` | Community risk scoring |
| `multisig-admin` | N-of-M multisig, generic dispatch via `call_versioned_contract` |
| `funding-request-registry` | Funding-request lifecycle |
| `identity-registry` | Recipient verification |

**~5,350 lines total.** **67 tests** across 7 `-tests` packages, all
genuinely executed — built to real `wasm32-unknown-unknown` binaries
and run against `casper-engine-test-support`'s local execution engine,
not just compile-checked. Reproduce: see
[`contracts/casper/README.md`](../contracts/casper/README.md)'s
"Building `wasm32-unknown-unknown` without `rustup`" section for the
exact toolchain steps (this project's own sandbox history needed a
documented workaround; a normal `rustup`-equipped machine or this
repo's own CI doesn't).

### 2.3 Everything else in scope

`frontend/` (React/Vite/TypeScript), `agents/` (Python + Node agent
fleet), `data-pipeline/` (Python satellite/sensor ingestion),
`.github/workflows/` (4 CI pipelines). **A professional audit's
primary scope should be §2.1 and §2.2 only** — the rest moves no
funds and holds no custody logic.

---

## 3. What already exists (so an auditor doesn't re-discover it)

### 3.1 Automated tooling, running continuously

`.github/workflows/security-audit.yml`, every PR and weekly: Slither,
Mythril, cargo-audit, npm audit (now including `contracts/tron`'s own
dependency tree, not just `frontend`/`agents/node`), pip-audit. Dated
reports in `docs/audit-reports/`. Known findings with no available fix
yet are tracked explicitly in [`SECURITY.md`](../SECURITY.md) rather
than silently ignored.

### 3.2 Two internal review passes (TRON), both re-verified against live code while preparing this document

| ID | Severity (informal) | Status |
|---|---|---|
| TRON-M1 | Medium | **Open** — no on-chain check that an admin/owner address is a contract (vs. an EOA); an operational-discipline item (deploy `MultiSigAdmin` first, use it everywhere), not a pure code defect |
| TRON-M2 | Medium | Fixed — two-step `propose*`/`accept*` transfer, confirmed present on `D3RACToken.sol`, `IdentityRegistry.sol`, `DisbursementController.sol`, `RiskRegistry.sol`, `FundingRequestRegistry.sol` |
| TRON-M3 | Medium | Fixed — `DisbursementController.sol::_safeTransfer` uses return-data-tolerant low-level `.call()`, not a direct `.transfer()` |
| TRON-L1 | Low (false positive, not fully resolved) | `MultiSigAdmin.sol::executeTransaction`'s Slither `reentrancy-eth` finding — two independent reviews concluded the flagged write sits in a branch that unconditionally reverts, leaving nothing for a reentrant call to observe, but both reviews deliberately stopped short of declaring it conclusively resolved; a professional auditor should form their own view |
| TRON-L2 | Low | Fixed — `RiskRegistry.sol::riskScore` divide-before-multiply precision loss |
| TRON-L3 | Low | Fixed — no explicit `evmVersion` pin meant compilation defaulted to Shanghai/`PUSH0`, which TRON's TVM gates behind a chain parameter rather than supporting unconditionally; pinned to `"paris"` in both `hardhat.config.js` and `tronbox-config.js` |

### 3.3 Casper-specific findings

| ID | Severity (informal) | Status |
|---|---|---|
| CASPER-H1 | **High** (fixed, now proven by an executed test) | Every contract's admin/owner check originally used `runtime::get_caller()`, which resolves to the *originating account* (`tx.origin`-equivalent) rather than the immediate caller (`msg.sender`-equivalent) — meaning ownership could never be usefully transferred to a contract (`multisig-admin` or the Hub), permanently bricking every owner-only function the moment that transfer happened. Fixed using `runtime::get_immediate_caller()`; **proven** by `d3rac-hub-tests`' single comprehensive test, which installs all 7 contracts, wires the Hub, hands its admin to a multisig, and executes a real Hub-mediated write through the whole chain |
| CASPER-M1 | Medium | **Open** — `risk-registry`'s ownership transfer is single-step (`transfer_ownership`), diverging from every other Casper module's two-step propose/accept pattern (matching the TRON side's own two-step convention). A deliberate, documented, but not-yet-reconciled inconsistency |
| CASPER-M2 | Medium (fixed) | `d3rac-token`'s allowance dictionary key exceeded Casper's 128-character `DICTIONARY_ITEM_KEY_MAX_LENGTH` (two concatenated `Key::to_string()` values = 157 characters) — found only by real execution (`ApiError::DictionaryItemKeyTooLarge`), not by compiling. Fixed by hashing both keys via `runtime::blake2b` and hex-encoding the digest |
| CASPER-L1 | Low (test-only) | A test in `multisig-admin-tests` originally self-dispatched to a `u64`-returning view, violating the contract's own documented `Unit`-return-type scope limit for its generic dispatcher — found by real execution (`ApiError::LeftOverBytes`), fixed by retargeting the test |

**CASPER-H1 is the finding an auditor should independently re-verify
first.** It sits squarely on the system's core privilege-delegation
model (who can hold administrative authority over what), and while it
is now backed by a real, executed test rather than reasoning alone,
"proven by one test suite" and "confirmed by an independent human
expert" remain different bars.

### 3.4 Real, executed verification (both chains)

- **123 TRON tests, 67 Casper tests, all genuinely run** — not
  written-and-assumed, not compile-checked-only. The Casper side in
  particular required documenting a real, working `rustup`-free
  toolchain path (`contracts/casper/README.md`) since it needed to be
  independently re-derived more than once across this project's
  history.
- **Both chains have a real, successful testnet deployment**, each via
  a CI workflow with every step green including the actual on-chain
  transactions: TRON→Shasta
  ([run #33703052261](https://github.com/Data-Driven-Disaster-Resilience/D3R-AC/actions/runs/33703052261),
  2026-09-03), Casper→testnet
  ([run #34073617859](https://github.com/Data-Driven-Disaster-Resilience/D3R-AC/actions/runs/34073617859),
  2026-09-07). Exact deployed addresses are in each run's own job
  summary via those links, not reproduced here.

### 3.5 What has explicitly NOT been done

- No professional third-party audit, of any kind, at any tier.
- No fuzzing, property-based testing, or formal verification on either
  side.
- No mainnet deployment, and no deployment on either chain has ever
  held real funds.
- Neither testnet deployment's multisig admin handoff is fully
  complete (both are 1-of-1 as a deliberate starting point, and the
  Casper side's Hub admin transfer was proposed but not yet accepted
  as of the deployment run).
- No verification of behavior specific to a live, multi-validator
  Casper node beyond what a single-node local execution engine and one
  real testnet deployment can show.

---

## 4. Recommended audit approach, with current 2026 market data

Researched directly for this document in September 2026 (see sources
below), not carried over from an earlier, potentially stale version.

**Market context**: 2026 smart-contract audit pricing spans roughly
$5,000 (simple token contracts) to $250,000–$350,000+ (enterprise,
multi-chain, or bridge-grade systems), with most standard DeFi-style
protocols landing $20,000–$60,000, and top-tier firms (Trail of Bits,
OpenZeppelin, ConsenSys Diligence, Halborn, Spearbit, Cantina, Sigma
Prime) typically starting around $80,000 for full engagements. Pricing
scales primarily with **interaction complexity and integration
surface**, not raw line count — relevant here, since this system's two
independently-implemented, closely-mirrored chains (TRON and Casper)
plus their Hub-mediated cross-contract wiring add real complexity
beyond what either contract set's line count alone would suggest.
Rust/WASM audits specifically carry a premium over equivalent EVM
scopes due to a smaller specialist talent pool.

**Code4rena, historically the largest open-contest platform, wound
down operations in May 2026**, with Immunefi absorbing its bug-bounty
side; Sherlock, Cantina, and Hats Finance remain the primary contest
platforms, alongside Immunefi for bounty-style ongoing coverage.

Given this project's actual profile — pre-audit, non-commercial,
disaster-relief nonprofit-style funding, no funds ever held in
production — three realistic paths, ascending in cost and depth:

1. **A time-boxed senior review** (a few thousand dollars for a few
   days of a named, accountable researcher's focused attention),
   scoped specifically to CASPER-H1's fix, the fund-custody paths in
   `DisbursementController.sol`/`disbursement-controller`, and the two
   still-open items (TRON-M1, CASPER-M1). This is the realistic
   minimum bar before either testnet deployment holds anything
   resembling real value, and fits a pre-seed budget.
2. **A Sherlock or Cantina-style engagement** at the lower end of their
   range (roughly $15,000–$40,000) once budget allows — Sherlock's own
   published model matches researchers to a codebase's specific
   language and risk profile from a large vetted network, relevant
   given this project's genuine two-chain, two-language scope.
3. **A firm with explicit multi-chain and Rust/WASM depth** — Hacken
   (explicitly lists Solidity, Rust, and TRON among supported
   stacks/chains) or Halborn (directly audited the Casper Association's
   own Casper 2.0 protocol upgrade — real, directly relevant prior
   Casper experience, using a methodology — manual review, `cargo
   audit`, integration testing, devnet deployment via `casper-client`
   — that closely matches this project's own verification approach) —
   once a $25,000+ budget is available, and worth requesting a single
   quote covering **both chains together** given how closely their
   designs mirror each other.

Whichever path is chosen, hand the auditor this document plus direct
links to `docs/audit-pass-2026-07-25.md`,
`docs/audit-pass-2026-09-09.md`, `SECURITY.md`,
`contracts/tron/README.md`, `contracts/casper/README.md`, and the
specific commit this document names — not just the default branch —
so scope is pinned and unambiguous.

**Sources consulted for the pricing and platform data above** (all
retrieved September 2026): Sherlock's own published 2026 market pricing
reference and audit-cost breakdown; a cross-platform comparison of
2026's top audit firms; Halborn's own published Casper 2.0 audit report
for the Casper Association; multiple independent news sources
confirming Code4rena's May 2026 wind-down and Immunefi's absorption of
its bug-bounty clients.

---

## 5. Findings register (for direct import into an auditor's own tracker)

See §3.2 and §3.3 above for the full table with status and detail —
repeated here as a flat list for tracker import: TRON-M1 (open),
TRON-M2 (fixed), TRON-M3 (fixed), TRON-L1 (false positive, not fully
resolved), TRON-L2 (fixed), TRON-L3 (fixed), CASPER-H1 (fixed, proven),
CASPER-M1 (open), CASPER-M2 (fixed), CASPER-L1 (fixed, test-only).

---

## 6. Known accepted risks (not contract-logic findings, tracked separately)

See [`SECURITY.md`](../SECURITY.md)'s own "Known accepted risks"
section for dependency-level findings with no available fix yet (e.g.
an `adm-zip` transitive advisory via `hardhat`, currently unfixable at
the dependency level since `0.6.0` — its latest published version — is
already what's installed and is still flagged for that specific
advisory). These are dev-tooling/compile-time only and never touch
deployed contract code, tracked with an explicit paper trail rather
than silently ignored or force-suppressed.

---

## 7. How to verify every claim in this document

```bash
git clone https://github.com/Data-Driven-Disaster-Resilience/D3R-AC.git
cd D3R-AC
git checkout ae8a81f217d6eba2c72162de7545a7e9e2cf1101

# TRON: real test suite
cd contracts/tron && npm install && npm test
# Expect: 123 passing

# Casper: see contracts/casper/README.md's "Building wasm32-unknown-unknown
# without rustup" section for exact steps if rustup isn't available;
# otherwise, with rustup:
cd ../casper
rustup target add wasm32-unknown-unknown
for pkg in risk-registry identity-registry multisig-admin d3rac-token \
           disbursement-controller funding-request-registry d3rac-hub; do
  cargo build --release --target wasm32-unknown-unknown -p "$pkg"
done
# stage the resulting .wasm per each *-tests package's own doc comment, then:
cargo test --workspace
# Expect: 67 passing
```

Line/test counts in §2 can be reproduced directly:
```bash
wc -l contracts/tron/tronbox/contracts/*.sol
wc -l contracts/casper/*/src/main.rs
grep -c "it(" contracts/tron/test/*.test.mjs
grep -rc "#\[test\]" contracts/casper/*-tests/tests/*.rs
```

Both testnet deployments can be independently checked via their linked
CI run pages (§3.4), which show every step's real output including the
on-chain transactions.
