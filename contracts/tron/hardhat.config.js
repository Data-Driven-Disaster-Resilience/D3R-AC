import hardhatEthers from "@nomicfoundation/hardhat-ethers";
import hardhatEthersChaiMatchers from "@nomicfoundation/hardhat-ethers-chai-matchers";
import hardhatMocha from "@nomicfoundation/hardhat-mocha";
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
/// Migrated to Hardhat 3 (2026), initially to try to resolve the
/// transitive `elliptic` CVE-2025-14505 advisory. That attempt didn't
/// actually clear it (Hardhat 3 itself still pulls elliptic in via
/// @nomicfoundation/hardhat-verify -> legacy @ethersproject/signing-key,
/// v5). The real fix: this project imports the individual plugins it
/// actually uses (hardhat-ethers, hardhat-ethers-chai-matchers,
/// hardhat-mocha, hardhat-network-helpers) instead of the
/// `@nomicfoundation/hardhat-toolbox-mocha-ethers` bundle, which
/// unconditionally wires in hardhat-verify (contract-verification
/// against Etherscan/Blockscout -- not something this project uses, and
/// not meaningful for TRON contracts anyway) along with hardhat-keystore,
/// hardhat-typechain, and hardhat-ignition-ethers, none of which are
/// used here either. Dropping the unused bundle removes elliptic (and
/// the whole legacy ethers-v5 dependency chain) from the tree entirely
/// -- no patch, official or otherwise, needed. Hardhat 3 unconditionally
/// requires "type": "module" in package.json (not just ESM config
/// syntax), AND requires its sources to live inside its own project
/// directory -- so Hardhat stays flat at this level (package.json here
/// is "type": "module") and TronBox is nested instead, in ./tronbox, as
/// its own isolated CommonJS package (see ./tronbox/tronbox-config.js).
/// Node resolves module type from the nearest package.json, so the two
/// tools' conflicting requirements never collide.
export default {
  plugins: [
    hardhatNetworkHelpers,
    hardhatEthers,
    hardhatMocha,
    hardhatEthersChaiMatchers,
  ],
  solidity: {
    version: "0.8.20",
    settings: {
      optimizer: { enabled: true, runs: 200 },
      // Pinned to "paris" (pre-Shanghai), not solc 0.8.20's own default
      // (Shanghai, which emits PUSH0 for zero-constant pushes) --
      // TRON's TVM gates Shanghai-era opcodes including PUSH0 behind a
      // chain parameter (ALLOW_TVM_SHANGHAI, mainnet-activated via
      // committee proposal #89), not automatically just because a
      // network is EVM-compatible in general. The real Shasta
      // deployment already succeeded under the unpinned default, which
      // is reassuring but not conclusive on its own: a PUSH0
      // instruction only fails at the moment a code path containing one
      // actually executes, not necessarily at deploy time, so a
      // rarely-hit branch could still surprise someone later even
      // though a given deployment worked. This pin guarantees TVM
      // compatibility regardless of which specific chain parameters are
      // active on whichever network gets deployed to next, rather than
      // depending on inference from one successful deployment. See
      // docs/audit-pass-2026-09-09.md's "No explicit evmVersion pin"
      // finding for the full writeup this fix responds to.
      evmVersion: "paris",
    },
  },
};
