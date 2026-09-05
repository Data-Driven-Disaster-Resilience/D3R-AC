# D3R·AC — Data-Driven Disaster Resilience for All Communities

**Blockchain-powered disaster resilience — predicting crises, delivering aid, and protecting communities before disaster strikes.**

Built by **TAAD (The Abuja Algorithmic Defenders)**

---

## What is D3R·AC?

D3R·AC is a proprietary, blockchain-based disaster resilience framework, built and owned by TAAD (The Abuja Algorithmic Defenders). It treats disaster relief as a data and infrastructure problem, not just a fundraising problem — using on-chain smart contracts to make fund disbursement transparent, auditable, and fast, instead of routed through opaque layers of intermediaries.

The system is built on three layers:

1. **Data layer** — ingests disaster-risk signals (hazard data, displacement indicators, infrastructure damage reports) to determine when and where resilience funding should be pre-positioned.
2. **Smart contract layer** — deployed across **TRON** and **Casper**, handling conditional, milestone-based, transparent fund release.
3. **Community access layer** — an interface for NGOs and local coordinators requiring zero blockchain literacy.

## The Risk Model

Disaster risk is modeled as a function of hazard, exposure, and vulnerability:

```
R(c, t) = H(t) · E(c) · V(c)
```

Where:
- `R(c, t)` — resilience-funding priority score for community `c` at time `t`
- `H(t)` — hazard probability at time `t`
- `E(c)` — exposure factor for community `c`
- `V(c)` — vulnerability index (infrastructure + socioeconomic data)

When `R(c, t)` crosses a defined threshold `θ`, a smart contract condition can trigger fund pre-positioning. Full derivation in [`docs/risk-model.md`](docs/risk-model.md).

## Repository Structure

```
d3rac/
├── contracts/
│   ├── tron/            # TRON smart contracts (TRC-20, TVM/Solidity)
│   └── casper/          # Casper smart contracts
├── frontend/             # Community access layer (web interface)
├── data-pipeline/         # Risk-scoring pipeline (R(c,t) implementation)
├── agents/               # Manifest-driven AI agent fleet (data, risk-model, brainbox,
│                          # coordination, contract-trigger) — see agents/README.md
├── docs/                 # Architecture, risk model, deployment guides
└── scripts/deploy/        # Deployment scripts
```

## Tech Stack

- **Smart contracts:** Solidity (TRON/TVM), Rust/WASM (Casper — `casper-contract`, `casper-types`, `casper-event-standard`; see [`contracts/casper/README.md`](contracts/casper/README.md))
- **Chains:** TRON, Casper Network
- **Frontend:** React + Vite + TypeScript
- **Data pipeline:** Python, satellite/sensor ingestion (NASA FIRMS, USGS, NASA EONET, GDACS), Africa-prioritized — see [`data-pipeline/README.md`](data-pipeline/README.md)
- **AI agent fleet:** Python (data + risk-model agents, Claude-backed `brainbox` directive controller) and Node/TypeScript (coordination + contract-trigger agents), sharing one manifest and message bus — see [`agents/README.md`](agents/README.md)

## Getting Started

```bash
git clone https://github.com/D3RAC/D3R-AC.git
cd D3R-AC
```

### Smart contracts (TRON)

See [`docs/deployment-guide.md`](docs/deployment-guide.md) for full deployment steps using TronIDE or TronBox. **Always deploy to testnet (Shasta/Nile) first.**

### Smart contracts (Casper)

See [`contracts/casper/README.md`](contracts/casper/README.md) for current status — four contracts (`risk-registry`, `identity-registry`, `disbursement-controller`, `d3rac-token`) written, compiling, and passing their local-network tests in CI; the other three (`multisig-admin`, `funding-request-registry`, `d3rac-hub`) now have source written too but are not yet confirmed compiling. Requires a `wasm32-unknown-unknown`-capable Rust toolchain to build.

### Frontend

```bash
cd frontend
npm install
npm run dev
```

### AI agent fleet

```bash
cd agents
make build   # validate agents.config.yaml, codegen typed bindings, install + build both stacks
make dev     # runs the full fleet once against the local file-based bus
```

See [`agents/README.md`](agents/README.md) for the full agent list, the manifest-driven build system, and how `brainbox` (the Claude-backed directive controller) degrades gracefully without `ANTHROPIC_API_KEY`.

## Status

🚧 **Active development.** TRON smart contract suite implemented — token,
identity registry, milestone-based disbursement controller, a multisig
admin role, a central coordinator ("Hub") with full role/ownership
control over the other five contracts, an on-chain risk registry, and
a funding-request board (seven contracts total; see
[`contracts/tron/README.md`](contracts/tron/README.md)) — with a
**logic-tested suite (116 passing tests)**, and, as of 2026-09-03,
**deployed to TRON's Shasta testnet** (see
[`docs/deployment-guide.md`](docs/deployment-guide.md)'s status note
for the run and current admin topology — a deliberately minimal 1-of-1
multisig, not a production configuration). **Still not professionally
audited** (see
[`docs/audit-pass-2026-07-25.md`](docs/audit-pass-2026-07-25.md) for an
internal self-review pass — explicitly not a substitute for one) —
don't treat a testnet deployment as mainnet-readiness.
Frontend community access layer implemented (TRON live, Casper adapter
in place pending Casper contract deployment; the TRON adapter targets
Shasta generically via `VITE_TRON_NETWORK` and hasn't yet been pointed
at this specific deployment's contract addresses), with offline/
low-connectivity resilience (service-worker caching of the app shell and
last-known live data, timeout+retry on the live feed) so the app stays
usable on a slow or intermittent connection — satellite (e.g. Starlink)
or terrestrial. Data pipeline implemented per
[`docs/data-pipeline-srs.md`](docs/data-pipeline-srs.md) — satellite/sensor
hazard ingestion (NASA FIRMS, USGS, NASA EONET, GDACS), Africa-prioritized,
with a 32-test suite (see [`data-pipeline/README.md`](data-pipeline/README.md))
— but **not yet run against the now-deployed Hub/RiskRegistry** on
Shasta; the pipeline's own on-chain submission path is still untested
against a real network, deployment having only just happened. Casper
contracts: all seven now have source written — `risk-registry`
(chosen as the SRS's own standalone/no-dependency starting point,
**confirmed compiling** against `wasm32-unknown-unknown`, **passing
all 5 of its integration tests**), `identity-registry` (SRS FR-2,
**passing all 9 of its integration tests**), `disbursement-controller`
(SRS FR-3, milestone-based fund release with a genuine cross-contract
call into `identity-registry`'s `is_verified`, **passing all 14 of its
integration tests** — though the funded-success path for its actual
fund release isn't one of them yet, see
[`contracts/casper/README.md`](contracts/casper/README.md) for why),
`d3rac-token` (SRS FR-1, a full CEP-18
token — all 11 standard entry points, standard events, and the
standard's own exact error codes, **passing all 13 of its
integration tests**), `multisig-admin` (SRS FR-4, **passing all 14
of its integration tests**, including a genuine cross-contract
`execute_transaction` call against a real `identity-registry`), and
`d3rac-hub` (SRS FR-8, one comprehensive integration test: installs
all seven contracts, wires the Hub to all five modules, and proves a
full admin handoff to a 1-of-1 multisig via a real Hub-mediated call)
— **56 Casper tests total**, all against a local Casper network. A systemic
finding surfaced along the way — every contract's
admin/owner check used `runtime::get_caller()`, which can't recognize
a *contract* (like `multisig-admin` or the Hub itself) as the caller
after a two-step admin transfer — was fixed across all five contracts
that had it (see
[`contracts/casper/README.md`](contracts/casper/README.md) for the
full writeup), and the Hub's own test now exercises that fix for real
rather than only reasoning about it. `funding-request-registry` (SRS
FR-6) is now also **confirmed
compiling**, using the fixed caller-resolution pattern from the start;
no integration test suite yet. Casper testnet deployment — actually
deploying all seven together and wiring the real, on-chain instances —
is still undone. See
[`contracts/casper/README.md`](contracts/casper/README.md)
for the honest, itemized status; Hub wiring,
frontend adapter completion, testnet testing, and any deployment are
all still pending.
The data pipeline SRS carries its own additional, even more restrictive
notice on top of the proprietary [`LICENSE`](LICENSE) that already
governs this entire repository.

## Contributing

D3R·AC is proprietary software owned by TAAD (The Abuja Algorithmic Defenders). Contributions from developers, humanitarian-tech practitioners, and NGO partners are welcome **by prior arrangement with TAAD** — see [`CONTRIBUTING.md`](CONTRIBUTING.md) for the process, and [`LICENSE`](LICENSE) for the terms any contribution is made under.

## Donate

D3R·AC accepts direct crypto donations to help fund disaster relief for
communities on the platform. See [`docs/donations.md`](docs/donations.md)
for addresses (USDT-TRC20, TRX, BTC, USDT-ERC20). Always double-check the
network matches the asset before sending.

## Security

This contract has **not** been professionally audited. Do not deploy to mainnet with real funds without a proper security review. See [`contracts/tron/README.md`](contracts/tron/README.md) for known limitations.

## License

Proprietary — **TAAD D3R·AC Proprietary License**, all rights reserved.
Not to be used, copied, modified, distributed, or deployed by any
party without prior express written permission from TAAD (The Abuja
Algorithmic Defenders) / Founder Armstrong Usang Monday. See
[`LICENSE`](LICENSE) for full terms.

## Contact

Built by TAAD (The Abuja Algorithmic Defenders), Abuja, Nigeria.