# D3R·AC — Data-Driven Disaster Resilience for All Communities

**Blockchain-powered disaster resilience — predicting crises, delivering aid, and protecting communities before disaster strikes.**

Built by **TAAD (The Abuja Algorithmic Defenders)**

---

## What is D3R·AC?

D3R·AC is an open-source, blockchain-based disaster resilience framework. It treats disaster relief as a data and infrastructure problem, not just a fundraising problem — using on-chain smart contracts to make fund disbursement transparent, auditable, and fast, instead of routed through opaque layers of intermediaries.

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
├── docs/                 # Architecture, risk model, deployment guides
└── scripts/deploy/        # Deployment scripts
```

## Tech Stack

- **Smart contracts:** Solidity (TRON/TVM), [Casper contract language — TBD]
- **Chains:** TRON, Casper Network
- **Frontend:** React + Vite + TypeScript
- **Data pipeline:** [TBD]

## Getting Started

```bash
git clone https://github.com/Data-Driven-Disaster-Resilience/D3R-AC.git
cd D3R-AC
```

### Smart contracts (TRON)

See [`docs/deployment-guide.md`](docs/deployment-guide.md) for full deployment steps using TronIDE or TronBox. **Always deploy to testnet (Shasta/Nile) first.**

### Frontend

```bash
cd frontend
npm install
npm run dev
```

## Status

🚧 **Active development.** TRON smart contract suite implemented — token,
identity registry, milestone-based disbursement controller, a multisig
admin role, a central coordinator ("Hub"), an on-chain risk registry, and
a funding-request board (seven contracts total; see
[`contracts/tron/README.md`](contracts/tron/README.md)) — with a
**logic-tested suite (83 passing tests)**, but **not yet deployed to any
network and not yet professionally audited.** Frontend community access
layer implemented (TRON live, Casper adapter in place pending Casper
contract deployment). Casper contracts and the data pipeline are still
not started.

## Contributing

D3R·AC is open-source and welcomes contributions from developers, humanitarian-tech practitioners, and NGO partners. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for guidelines.

## Security

This contract has **not** been professionally audited. Do not deploy to mainnet with real funds without a proper security review. See [`contracts/tron/README.md`](contracts/tron/README.md) for known limitations.

## License

MIT — see [`LICENSE`](LICENSE)

## Contact

Built by TAAD (The Abuja Algorithmic Defenders), Abuja, Nigeria.