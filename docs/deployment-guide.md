# Deployment Guide

This guide covers deploying D3R·AC's two deployable pieces: TRON smart
contracts and the frontend. **Read the [Security](#security-checklist)
section before deploying anything with real funds.**

> **Status note:** contract source (seven contracts), a passing logic
> test suite (83 tests, see
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

`contracts/tron/tronbox-config.js` already exists but is compile-only
(no `networks` entry yet) — don't run `tronbox init` over it, just add
a network/private-key section:

```bash
cd contracts/tron
npm install -g tronbox
# add a `networks.shasta` entry with your key to tronbox-config.js (see Security below)
tronbox compile
tronbox migrate --network shasta
```

TronBox is preferable once there's more than one contract or you need
repeatable deployments (CI, multiple environments) — it scripts what
TronIDE does by hand.

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
