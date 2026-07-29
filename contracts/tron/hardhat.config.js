import hardhatToolboxMochaEthersPlugin from "@nomicfoundation/hardhat-toolbox-mocha-ethers";
import hardhatEthers from "@nomicfoundation/hardhat-ethers";
import hardhatEthersChaiMatchers from "@nomicfoundation/hardhat-ethers-chai-matchers";
import hardhatNetworkHelpers from "@nomicfoundation/hardhat-network-helpers";

/// Hardhat is used here purely as a local test harness — it validates
/// contract logic against a standard EVM, which these contracts target
/// exactly (no TRON-specific precompiles or opcodes are used anywhere in
/// ./contracts). It is NOT how you deploy to TRON; for that, use
/// TronBox or TronIDE against Shasta/Nile as described in
/// docs/deployment-guide.md. Running the logic tests here first, then a
/// manual/TronBox pass on testnet before anything resembling real funds
/// touches these contracts, is the intended workflow.
///
/// Migrated to Hardhat 3 (2026) to resolve the transitive `elliptic`
/// CVE-2025-14505 advisory, which only clears via the Hardhat 3 upgrade
/// path. Hardhat 3 unconditionally requires "type": "module" in
/// package.json (not just ESM config syntax), AND requires its sources
/// to live inside its own project directory -- so Hardhat stays flat at
/// this level (package.json here is "type": "module") and TronBox is
/// nested instead, in ./tronbox, as its own isolated CommonJS package
/// (see ./tronbox/tronbox-config.js). Node resolves module type from
/// the nearest package.json, so the two tools' conflicting
/// requirements never collide.
export default {
  plugins: [
    hardhatNetworkHelpers,
    hardhatEthers,
    hardhatToolboxMochaEthersPlugin,
    hardhatEthersChaiMatchers,
  ],
  solidity: {
    version: "0.8.20",
    settings: {
      optimizer: { enabled: true, runs: 200 },
    },
  },
};
