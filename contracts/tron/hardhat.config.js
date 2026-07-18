require("@nomicfoundation/hardhat-chai-matchers");
require("@nomicfoundation/hardhat-ethers");

/// Hardhat is used here purely as a local test harness — it validates
/// contract logic against a standard EVM, which these contracts target
/// exactly (no TRON-specific precompiles or opcodes are used anywhere in
/// this directory). It is NOT how you deploy to TRON; for that, use
/// TronBox or TronIDE against Shasta/Nile as described in
/// docs/deployment-guide.md. Running the logic tests here first, then a
/// manual/TronBox pass on testnet before anything resembling real funds
/// touches these contracts, is the intended workflow.
module.exports = {
  solidity: {
    version: "0.8.20",
    settings: {
      optimizer: { enabled: true, runs: 200 },
    },
  },
};
