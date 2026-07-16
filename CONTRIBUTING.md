# Contributing to D3R·AC

Thanks for your interest in D3R·AC (Data-Driven Disaster Resilience for All
Communities). This is humanitarian infrastructure — code here can affect how
quickly real disaster relief funds reach real communities. Contributions are
welcome from developers, humanitarian-tech practitioners, and NGO partners,
and we ask for a bit more care than a typical open-source repo because of
what's at stake.

## Ways to contribute

D3R·AC has four layers (see the main [README](README.md)), and each has
different needs:

- **Data layer / risk model** (`data-pipeline/`) — hazard signal ingestion,
  the `R(c,t) = H(t)·E(c)·V(c)` scoring implementation, threshold tuning.
- **Smart contract layer** (`contracts/`) — TRON (TVM/Solidity) and Casper
  contract logic for milestone-based fund release.
- **Community access layer** (`frontend/`) — the web interface NGOs and
  coordinators use. See [`frontend/README.md`](frontend/README.md) for setup.
- **Documentation** (`docs/`) — architecture notes, deployment guides,
  the risk model derivation.

Non-code contributions matter too: humanitarian-sector domain expertise on
what data should feed the risk model, translation, and accessibility review
are all valuable and don't require writing code.

## Before you start

- **Open an issue first** for anything beyond a small fix — especially for
  contract logic or anything touching fund disbursement — so the approach
  can be discussed before you invest time in it.
- Check existing issues and open PRs to avoid duplicate work.

## Development setup

```bash
git clone https://github.com/Data-Driven-Disaster-Resilience/D3R-AC.git
cd D3R-AC
```

For the frontend specifically:

```bash
cd frontend
npm install
npm run dev
```

Run these before opening a PR:

```bash
npm run lint
npm run build
```

## Pull request process

1. Fork the repo and branch from `main` (`feat/…`, `fix/…`, `docs/…`).
2. Keep PRs scoped to one concern — a contract change and a frontend change
   should be separate PRs.
3. Write a clear PR description: what changed, why, and how you tested it.
4. Make sure lint and build pass. For frontend changes, TypeScript strict
   mode must stay clean (`tsc -b`) — don't loosen `tsconfig` settings to
   make an error go away.
5. One maintainer review is required before merge.

## Rules specific to this project

**Smart contracts:**
- All contract work targets **testnet only** (TRON Shasta/Nile, Casper
  testnet) unless a PR is explicitly about a reviewed mainnet deployment.
- Never commit private keys, seed phrases, or `.env` files with real
  credentials. Use `.env.example` as the template for what variables are
  expected, without real values.
- Any change to fund-transfer logic (amounts, recipients, milestone
  conditions) needs a second reviewer, not just one approval.
- This project has **not been professionally audited**. Don't imply
  otherwise in code comments, docs, or PR descriptions.

**Frontend:**
- Follow the existing `ChainAdapter` interface (`frontend/src/lib/chainAdapter.ts`)
  when adding or modifying chain integrations — new chains should be a new
  adapter implementing that interface, not special-cased into components.
- No `localStorage`/`sessionStorage` of wallet addresses, balances, or
  transaction history containing recipient data — treat community and
  disbursement data as sensitive by default.
- Token amount math must stay precision-safe (BigInt/string-based), not
  floating point — see `formatUnits`/`parseUnits` in `tronAdapter.ts` for
  the existing pattern.

**Data layer:**
- If you're proposing real hazard/exposure/vulnerability data sources,
  document provenance and licensing — don't hardcode data you don't have
  rights to redistribute.

## Reporting security issues

**Do not open a public issue for security vulnerabilities**, especially
anything related to fund disbursement, private key handling, or contract
logic. See [`SECURITY.md`](SECURITY.md) if present, or contact the
maintainers directly.

## Code of conduct

Be respectful. This project touches disaster response and vulnerable
communities — assume good faith, disagree on substance not people, and
keep discussion focused on what actually helps the mission.

## License

By contributing, you agree your contributions are licensed under the
project's [MIT License](LICENSE).
