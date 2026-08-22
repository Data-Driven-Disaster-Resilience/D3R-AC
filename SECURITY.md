# Security Policy

D3R·AC is humanitarian infrastructure that, once deployed with real funds
behind it, gates disaster-relief disbursements. Security issues here are
taken seriously, and issues involving fund-transfer logic, private key
handling, or contract admin/ownership are treated as the highest priority.

## Current audit status — read this first

**This project has not been professionally audited.** See
[`docs/audit-pass-2026-07-25.md`](docs/audit-pass-2026-07-25.md) for the
existing self-review (not a substitute for a third-party audit) and
[`docs/audit-reports/`](docs/audit-reports/) for the automated static/
dependency-analysis history (Slither, cargo-audit, npm audit, pip-audit —
see `.github/workflows/security-audit.yml`). Nothing in this repo should be
deployed with real funds behind it before a professional audit — see
`docs/deployment-guide.md`'s Security checklist.

## Reporting a vulnerability

**Do not open a public GitHub issue** for a security vulnerability,
especially anything touching:
- Fund-transfer or milestone-disbursement logic (`DisbursementController`,
  `FundingRequestRegistry`, `D3RACHub`, and their Casper equivalents)
- Admin/ownership/access-control logic (`MultiSigAdmin`, `IdentityRegistry`)
- Private key handling anywhere in `data-pipeline/`, `agents/`, or the
  frontend's wallet integration
- A leaked secret or credential (in code, in CI logs, or elsewhere)

Instead, contact TAAD (The Abuja Algorithmic Defenders) directly through a
private channel — see the maintainer contact in the top-level
[README](README.md). Please include:

- A description of the issue and its potential impact
- Steps to reproduce, or a proof of concept if you have one
- Which component is affected (contract, chain, file/line if known)
- Whether you believe it's exploitable on the current testnet deployment,
  if any exists at the time of your report

We'll acknowledge reports and work with you on disclosure timing. Since
this project isn't yet on mainnet with real funds at stake, most reports
will be addressed as normal (if urgent) bugs rather than incident-response
situations — but treat anything you're unsure about as sensitive by
default, and let us make that call.

## Scope

This policy covers everything in this repository: `contracts/tron`,
`contracts/casper`, `data-pipeline/`, `agents/`, and `frontend/`. It does
not cover third-party infrastructure D3R·AC depends on (TRON network,
Casper network, NASA FIRMS/USGS/EONET/GDACS data sources, npm/PyPI/crates.io
supply chain) — report issues in those to their own maintainers, though
we'd still appreciate a heads-up if one materially affects D3R·AC.

## Supported versions

Pre-testnet-deployment, there is exactly one supported line: `main`. There
are no released/tagged versions yet to maintain security support windows
for.
