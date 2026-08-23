# D3R·AC Frontend

The **community access layer** of D3R·AC (see the [top-level README](../README.md)) —
the web dashboard NGOs and coordinators use to see community risk scores, funding
progress, and (where a connected wallet permits it) trigger disbursements.

React 19 + TypeScript + Vite, installable as a PWA (offline shell + install
prompt via `vite-plugin-pwa`) so field coordinators with unreliable connectivity
can still load the shell and see last-synced data.

## What's here

```
src/
  pages/
    Landing.tsx       # public-facing overview
    Dashboard.tsx      # per-community risk scores, funding progress, KPIs
    Disburse.tsx        # wallet-gated: trigger a disbursement
    NotFound.tsx
  components/
    RiskOverview.tsx, RiskChart.tsx, RiskPulseGrid.tsx   # R(c,t) = H(t)·E(c)·V(c) visualization
    FundingProgress.tsx, KpiCards.tsx
    ChainSelector.tsx    # switch between TRON / Casper
    NavBar.tsx, Footer.tsx, ErrorBoundary.tsx
  context/
    WalletContext.tsx    # wraps whichever ChainAdapter is active
  lib/
    chainAdapter.ts       # the interface every chain integration implements
    tronAdapter.ts          # TRON/TronLink implementation — the only one live today
    casperAdapter.ts         # Casper Wallet implementation — stubbed, throws
                              # "not deployed yet" until contracts/casper ships
    units.ts                  # formatUnits/parseUnits — BigInt-based, precision-safe
                                # token amount conversion (see Security below)
```

## Chain support status

- **TRON**: live. Connects via TronLink, reads balances, submits disbursement
  transactions. Network is set via `VITE_TRON_NETWORK` (`shasta` testnet by
  default — see `.env.example`).
- **Casper**: UI-ready but functionally stubbed. `casperAdapter.ts` implements
  the same `ChainAdapter` interface so the Casper option already appears in
  `ChainSelector`, but every method throws "not deployed yet" — correctly,
  since `contracts/casper` isn't deployed. See
  [`contracts/casper/README.md`](../contracts/casper/README.md) for that
  suite's actual status.

Adding a new chain means implementing `ChainAdapter` (`src/lib/chainAdapter.ts`)
— never special-case a new chain inside a page or component. See
[`CONTRIBUTING.md`](../CONTRIBUTING.md) for the project-wide rules this
repo enforces on chain adapters (precision-safe math, no sensitive data in
browser storage).

## Setup

```bash
npm install
cp .env.example .env   # edit VITE_TRON_NETWORK if targeting mainnet later
npm run dev
```

## Scripts

```bash
npm run dev       # local dev server
npm run build     # tsc -b (strict) + vite build + PWA precache generation
npm run test      # vitest — currently covers src/lib/units.ts
npm run lint      # oxlint
npm run preview   # preview a production build locally
```

## Testing

`npm run test` runs [Vitest](https://vitest.dev). Coverage today is
`src/lib/units.ts` — the BigInt-based `formatUnits`/`parseUnits` pair every
`ChainAdapter` should route token-amount math through, per
[`CONTRIBUTING.md`](../CONTRIBUTING.md)'s "must stay precision-safe" rule.
There's no component/page-level test coverage yet; that's the natural next
addition (Testing Library + Vitest, `jsdom`/`happy-dom` environment) once more
of the app's interactive logic (wallet connect flows, disbursement form
validation) grows complex enough to be worth locking down.

## Security

- No `localStorage`/`sessionStorage` of wallet addresses, balances, or
  transaction/recipient data — see `CONTRIBUTING.md`.
- Token amounts are never passed through JS `Number` for on-chain math —
  `Number(raw) / 10**decimals` silently loses precision above
  `Number.MAX_SAFE_INTEGER`, which real token balances can exceed. Use
  `formatUnits`/`parseUnits` from `src/lib/units.ts` instead.
- This app has not been professionally security-reviewed — see the
  top-level README's Security section.
