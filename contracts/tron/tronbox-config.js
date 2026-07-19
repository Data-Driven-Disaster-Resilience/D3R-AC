// TronBox configuration for D3R·AC's TRON contract suite (IdentityRegistry,
// D3RACToken, DisbursementController, RiskRegistry, FundingRequestRegistry,
// AttestorMultisig).
//
// Per docs/deployment-guide.md's security checklist: never put a real
// private key directly in this file. Set PRIVATE_KEY_SHASTA / PRIVATE_KEY_NILE /
// PRIVATE_KEY_MAINNET as environment variables (or Replit Secrets / GitHub
// Actions secrets) — this file only reads from process.env.
//
// Usage:
//   tronbox compile
//   tronbox migrate --network shasta
//   tronbox migrate --network nile
//   tronbox migrate --network mainnet   (only after the security checklist
//                                        in docs/deployment-guide.md is done)

module.exports = {
  networks: {
    shasta: {
      privateKey: process.env.PRIVATE_KEY_SHASTA,
      userFeePercentage: 100,
      feeLimit: 1_000_000_000,
      fullHost: "https://api.shasta.trongrid.io",
      network_id: "2",
    },
    nile: {
      privateKey: process.env.PRIVATE_KEY_NILE,
      userFeePercentage: 100,
      feeLimit: 1_000_000_000,
      fullHost: "https://nile.trongrid.io",
      network_id: "3",
    },
    mainnet: {
      privateKey: process.env.PRIVATE_KEY_MAINNET,
      userFeePercentage: 100,
      feeLimit: 1_000_000_000,
      fullHost: "https://api.trongrid.io",
      network_id: "1",
    },
  },
  compilers: {
    solc: {
      version: "0.8.20",
    },
  },
};
