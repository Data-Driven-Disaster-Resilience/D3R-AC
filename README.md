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

See [`contracts/casper/README.md`](contracts/casper/README.md) for current status — one contract (`risk-registry`) written and now confirmed compiling in CI; the other six are not started. Requires a `wasm32-unknown-unknown`-capable Rust toolchain to build.

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
**logic-tested suite (115 passing tests)**, but **not yet deployed to any
network and not yet professionally audited** (see
[`docs/audit-pass-2026-07-25.md`](docs/audit-pass-2026-07-25.md) for an
internal self-review pass — explicitly not a substitute for one).
Frontend community access layer implemented (TRON live, Casper adapter
in place pending Casper contract deployment), with offline/
low-connectivity resilience (service-worker caching of the app shell and
last-known live data, timeout+retry on the live feed) so the app stays
usable on a slow or intermittent connection — satellite (e.g. Starlink)
or terrestrial. Data pipeline implemented per
[`docs/data-pipeline-srs.md`](docs/data-pipeline-srs.md) — satellite/sensor
hazard ingestion (NASA FIRMS, USGS, NASA EONET, GDACS), Africa-prioritized,
with a 32-test suite (see [`data-pipeline/README.md`](data-pipeline/README.md))
— but **not yet run against a deployed Hub/RiskRegistry**, since neither
is deployed to any network yet. Casper contracts: one of seven
contracts (`risk-registry`, chosen as the SRS's own standalone/
no-dependency starting point) written and **confirmed compiling**
against `wasm32-unknown-unknown` in CI — see
[`contracts/casper/README.md`](contracts/casper/README.md)
for the honest, itemized status; the other six contracts, Hub wiring,
frontend adapter completion, local-network/testnet testing, and any
deployment are all still pending.
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