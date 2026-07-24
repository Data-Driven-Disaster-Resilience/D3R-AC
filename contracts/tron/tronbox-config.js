// Loads TRON_PRIVATE_KEY_* and other deploy-time config from a local
// .env file (see .env.example). That file is gitignored -- never commit
// a real private key or seed phrase, per docs/deployment-guide.md's
// security checklist.
//
// Wrapped in try/catch deliberately: `tronbox compile` needs none of
// this (no network/key required to just compile), so it shouldn't fail
// in a context that hasn't run `npm install` locally -- e.g. CI's
// contracts-tron job, which only installs the tronbox CLI globally.
// `tronbox migrate` does need real env vars, and `npm install` (see
// docs/deployment-guide.md) gets you dotenv for that.
try {
  require('dotenv').config();
} catch (_) {
  // No local node_modules/dotenv -- fine for compile-only use, since
  // process.env.TRON_PRIVATE_KEY_* etc. simply stay undefined below.
}

module.exports = {
  networks: {
    // Shasta testnet -- get funded here from
    // https://www.trongrid.io/shasta before deploying.
    shasta: {
      privateKey: process.env.TRON_PRIVATE_KEY_SHASTA,
      userFeePercentage: 50,
      feeLimit: 1_000_000_000,
      fullHost: 'https://api.shasta.trongrid.io',
      network_id: '2',
    },
    // Nile testnet -- get funded here from
    // https://nileex.io/join/getJoinPage before deploying.
    nile: {
      privateKey: process.env.TRON_PRIVATE_KEY_NILE,
      userFeePercentage: 100,
      feeLimit: 1_000_000_000,
      fullHost: 'https://nile.trongrid.io',
      network_id: '3',
    },
    // Mainnet is deliberately NOT configured here. Per
    // docs/deployment-guide.md, this project has not been professionally
    // audited -- do not add a mainnet network block or deploy real funds
    // against these contracts until that changes.
  },
  contracts_directory: './contracts',
  contracts_build_directory: './build',
  compilers: {
    solc: {
      version: '0.8.20',
      settings: {
        optimizer: { enabled: true, runs: 200 },
      },
    },
  },
};
