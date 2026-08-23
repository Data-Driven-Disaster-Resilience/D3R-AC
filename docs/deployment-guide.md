# Deployment Guide

This guide covers deploying D3R·AC's two deployable pieces: TRON smart
contracts and the frontend. **Read the [Security](#security-checklist)
section before deploying anything with real funds.**

> **Status note:** contract source (seven contracts), a passing logic
> test suite (116 tests, see
> [`contracts/tron/README.md`](../contracts/tron/README.md)), and a
> working TronBox compile config all exist now — but there is still no
> testnet deployment and no professional audit. The steps below describe
> the process this project will use for that next step; treat this as a
> process reference, not confirmation of a currently-deployed contract.
> The frontend deployment section reflects what's actually built.

## Smart contracts (TRON)

### Prerequisites

- A TRON wallet (TronLink) funded with testnet TRX for gas — get testnet
  TRX from the [Shasta testnet faucet](https://www.trongrid.io/shasta)
  or the [Nile testnet faucet](https://nileex.io/join/getJoinPage).
- Either [TronIDE](https://www.tronide.io/) (browser-based, no install)
  or [TronBox](https://developers.tron.network/docs/tronbox-quick-start)
  (CLI, for scripted/repeatable deployments).

### Always deploy to testnet first

Deploy and exercise the full contract lifecycle — including milestone
release and edge cases like zero-amount or unauthorized-caller
transactions — on **Shasta or Nile testnet** before mainnet ever enters
the conversation. There is no acceptable shortcut here: this contract
moves real disaster-relief funds, and testnet is free.

### Deploying with TronIDE

1. Open [tronide.io](https://www.tronide.io/) and connect TronLink,
   switched to Shasta or Nile testnet.
2. Load or paste the contract source.
3. Compile, review any compiler warnings (don't ignore them), then deploy.
4. Verify the deployed contract on
   [Shasta Tronscan](https://shasta.tronscan.org/) or the equivalent Nile
   explorer — confirm the bytecode and constructor arguments match what
   you intended before doing anything else with it.

### Deploying with TronBox

`contracts/tron/tronbox/tronbox-config.js` has `shasta`/`nile` network
entries already, and `contracts/tron/tronbox/migrations/2_deploy_d3rac.js`
deploys the full contract suite and performs the complete Hub-wiring
sequence (see `contracts/tron/README.md`'s "Wiring the Hub" section) in
one run, ending with `D3RACHub`'s admin handed to a freshly-deployed
`MultiSigAdmin`. Don't run `tronbox init` over any of this — it already
exists.

```bash
cd contracts/tron/tronbox
npm install -g tronbox
npm install                      # installs dotenv, used by tronbox-config.js
cp .env.example .env             # fill in your deploy key + multisig config
tronbox compile
tronbox migrate --network shasta
```

`.env` needs, at minimum, `TRON_PRIVATE_KEY_SHASTA` (or `_NILE`) and
`MULTISIG_OWNERS`/`MULTISIG_THRESHOLD` — the migration deliberately
refuses to run without a real multisig configured, since testing the
exact production admin topology on testnet first is the point of doing
this here rather than improvising it before mainnet. See
`contracts/tron/tronbox/.env.example` for the full list and
`migrations/2_deploy_d3rac.js`'s header comment for what each step does.

TronBox is preferable once there's more than one contract or you need
repeatable deployments (CI, multiple environments) — it scripts what
TronIDE does by hand.

### Deploying via GitHub Actions

`.github/workflows/deploy-tron-testnet.yml` runs the exact same
`tronbox compile && tronbox migrate` sequence above, but from CI. It is
**`workflow_dispatch`-only, deliberately** — it never runs on a push,
PR, or merge, and it also requires typing `deploy` into a text input
before it will do anything. Nothing about opening, reviewing, or
merging a PR (including this one) will trigger it.

One-time setup, before first use:

1. In the repo's **Settings → Environments**, create two environments
   named `shasta` and `nile` (matching the workflow's network choices).
   Adding required reviewers to these environments is recommended —
   that turns "click Run workflow" into "click Run workflow, then a
   second person has to approve it," which is a good property for
   anything that broadcasts transactions.
2. On each environment, add the same secrets `.env.example` lists:
   `TRON_PRIVATE_KEY_SHASTA` (or `_NILE` on that environment),
   `MULTISIG_OWNERS`, `MULTISIG_THRESHOLD`, and optionally
   `D3RAC_INITIAL_SUPPLY`/`RISK_THRESHOLD`. Nobody associated with this
   repo (including any AI assistant working on it) should ever
   generate, see, or be given this private key — fund a fresh address
   yourself from the testnet faucets linked in `.env.example` and paste
   only that key into the environment secret.
3. From the **Actions** tab, run **Deploy TRON contracts (testnet)**,
   pick a network, type `deploy`, and confirm.

The workflow publishes the deployed addresses (including
`MultiSigAdmin`'s — the address that becomes `D3RACHub`'s admin) to
the run's job summary and as a downloadable artifact. It does not
commit anything back to the repo automatically; recording the address
publicly (the Post-deployment step below) stays a deliberate, reviewed
action.

### Post-deployment

- Record the deployed contract address and the exact source/commit hash
  it corresponds to, publicly, in the repo or release notes — this is
  part of what makes the system auditable, per the project's stated goal.
- Point the frontend at it via `VITE_TRON_NETWORK` and the token contract
  address entered in the disbursement console (see below) — the frontend
  doesn't hardcode a contract address, it's supplied per-session.

## Frontend

The frontend is a static Vite build — deployable to any static host
(Vercel, Netlify, Cloudflare Pages, GitHub Pages, S3+CloudFront, etc.).

```bash
cd frontend
npm install
cp .env.example .env    # set VITE_TRON_NETWORK ("shasta" or "mainnet")
npm run build            # outputs to frontend/dist/
```

Deploy the contents of `frontend/dist/` to your static host of choice.
Since this is a single-page app using client-side routing, configure your
host to serve `index.html` for unmatched paths (a "SPA fallback" or
rewrite rule) so routes like `/dashboard` work on direct load/refresh,
not just client-side navigation.

No backend/server component is required for the frontend as it currently
exists — it talks directly to the browser-injected wallet extension
(TronLink, Casper Wallet) and to the chain via whatever RPC endpoint that
extension is configured to use.

## Security checklist

Before deploying anything beyond testnet:

- [ ] **Never commit private keys, seed phrases, or `.env` files with
      real values.** Use `.env.example` as the template; keep actual
      secrets out of git entirely, including git history — a key
      committed and later removed is still compromised.
- [ ] **Use environment variables or a secrets manager**, not hardcoded
      values, for any deployment key (TronBox config, CI secrets).
- [ ] **Get a professional security audit** before moving fund-handling
      contracts to mainnet. This project has not been audited — see the
      main [README's Security section](../README.md#security) and
      [`contracts/tron/README.md`](../contracts/tron/README.md).
- [ ] **Consider a multisig** for any contract-owner or admin role that
      can move funds or change disbursement conditions — a single key
      compromise shouldn't be able to redirect relief funds.
      `contracts/tron/contracts/MultiSigAdmin.sol` is available for
      this; deploy it and point `D3RACToken`/`IdentityRegistry`/
      `DisbursementController`'s admin/owner role at it before mainnet.
- [ ] **Rate-limit and monitor** contract calls in production — sudden
      spikes in disbursement calls are worth alerting on, not just
      logging.
- [ ] **Test the failure paths**, not just the happy path: what happens
      if a milestone condition is never met, if a recipient address is
      wrong, if the contract runs low on gas mid-transaction.
- [ ] **Document the deployed address and version** publicly (see
      Post-deployment above) so the community and auditors can verify
      what's actually running against what's in this repo.
