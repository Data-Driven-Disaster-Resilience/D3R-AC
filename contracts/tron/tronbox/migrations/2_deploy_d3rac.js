const D3RACToken = artifacts.require("D3RACToken");
const IdentityRegistry = artifacts.require("IdentityRegistry");
const DisbursementController = artifacts.require("DisbursementController");
const RiskRegistry = artifacts.require("RiskRegistry");
const FundingRequestRegistry = artifacts.require("FundingRequestRegistry");
const D3RACHub = artifacts.require("D3RACHub");
const MultiSigAdmin = artifacts.require("MultiSigAdmin");

const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000";

/// Deploys D3R-AC's full TRON contract suite and wires D3RACHub up for
/// "full control" per contracts/tron/README.md's "Wiring the Hub"
/// section, then hands the Hub's own admin role to a MultiSigAdmin --
/// see docs/deployment-guide.md's security checklist ("consider a
/// multisig for any contract-owner or admin role that can move funds").
///
/// This exact sequence (role grant before ownership transfer, per
/// contract) mirrors test/D3RACHub.test.js's `beforeEach`, which is the
/// project's own executable spec for correct wiring -- see that file
/// and the README section above if you're modifying this script.
///
/// Required environment variables (see .env.example):
///   MULTISIG_OWNERS    comma-separated TRON addresses, e.g.
///                       "TAbc...,TDef...,TGhi..."
///   MULTISIG_THRESHOLD  confirmations required, e.g. "2"
///
/// Optional:
///   D3RAC_INITIAL_SUPPLY  whole-token initial supply minted to the
///                         deployer at construction (default: 0 --
///                         mint via hub.mintTokens once the Hub is
///                         wired, instead of a large constructor mint)
///   RISK_THRESHOLD        RiskRegistry's initial threshold, in the same
///                         fixed-point units docs/risk-model.md uses
///                         (default: 0 -- set it via
///                         hub.setRiskThreshold once deployed)
module.exports = async function (deployer, network, accounts) {
  const deployerAddress = accounts[0];

  const initialSupply = process.env.D3RAC_INITIAL_SUPPLY || "0";
  const riskThreshold = process.env.RISK_THRESHOLD || "0";

  const multisigOwners = (process.env.MULTISIG_OWNERS || "")
    .split(",")
    .map((a) => a.trim())
    .filter(Boolean);
  const multisigThreshold = parseInt(process.env.MULTISIG_THRESHOLD || "0", 10);

  if (multisigOwners.length === 0 || !multisigThreshold) {
    throw new Error(
      "2_deploy_d3rac: set MULTISIG_OWNERS (comma-separated TRON addresses) " +
        "and MULTISIG_THRESHOLD in your environment before running this " +
        "migration -- see .env.example. This migration deliberately " +
        "refuses to deploy without a real multisig configured: testing " +
        "the exact production admin topology on testnet first is the " +
        "whole point of running this here rather than improvising it " +
        "before mainnet."
    );
  }

  console.log(`\nDeploying D3R-AC to network: ${network}`);
  console.log(`Deployer (interim admin/owner during wiring): ${deployerAddress}\n`);

  // ---- 1. Deploy the five underlying contracts + Hub, deployer as ------
  //         interim admin/owner on everything, since the wiring step
  //         below needs the deployer's about-to-be-transferred authority
  //         to run at all.
  await deployer.deploy(D3RACToken, initialSupply, deployerAddress);
  const token = await D3RACToken.deployed();

  await deployer.deploy(IdentityRegistry, deployerAddress);
  const identityRegistry = await IdentityRegistry.deployed();

  await deployer.deploy(DisbursementController, identityRegistry.address, deployerAddress);
  const disbursementController = await DisbursementController.deployed();

  await deployer.deploy(RiskRegistry, riskThreshold, ZERO_ADDRESS);
  const riskRegistry = await RiskRegistry.deployed();

  await deployer.deploy(FundingRequestRegistry, ZERO_ADDRESS);
  const fundingRequestRegistry = await FundingRequestRegistry.deployed();

  await deployer.deploy(
    D3RACHub,
    deployerAddress,
    token.address,
    identityRegistry.address,
    disbursementController.address,
    riskRegistry.address,
    fundingRequestRegistry.address
  );
  const hub = await D3RACHub.deployed();

  // ---- 2. Full Hub wiring -----------------------------------------------
  //   Role mappings (verifier/attester/dataFeeder/proposer/minter) are
  //   ADDITIVE -- granting the Hub one doesn't remove the deployer's own
  //   access. Single admin/owner addresses are EXCLUSIVE -- transferring
  //   one to the Hub replaces the deployer as the holder immediately.
  //   Grant the additive role first, transfer exclusive admin/owner
  //   second, per contract -- matches the order in
  //   test/D3RACHub.test.js's beforeEach exactly.
  console.log("Wiring the Hub...");

  await identityRegistry.setVerifier(hub.address, true);
  await identityRegistry.transferAdmin(hub.address);

  await disbursementController.setAttester(hub.address, true);
  await disbursementController.transferAdmin(hub.address);

  await token.setMinter(hub.address, true);
  await token.transferOwnership(hub.address);

  await riskRegistry.addDataFeeder(hub.address);
  await riskRegistry.transferOwnership(hub.address);

  await fundingRequestRegistry.addProposer(hub.address);
  await fundingRequestRegistry.transferOwnership(hub.address);

  console.log("Hub wiring complete -- Hub now holds admin/owner on all five modules.\n");

  // ---- 3. Deploy MultiSigAdmin and move the Hub's own admin role to it -
  //         Everything above already funnels through the Hub, so the Hub
  //         is the single admin surface that needs to move off a lone
  //         deployer key and onto the multisig.
  await deployer.deploy(MultiSigAdmin, multisigOwners, multisigThreshold);
  const multisig = await MultiSigAdmin.deployed();

  await hub.transferAdmin(multisig.address);

  console.log("Hub admin transferred to MultiSigAdmin.\n");

  // ---- 4. Record what was deployed --------------------------------------
  //   docs/deployment-guide.md's post-deployment step: record the
  //   deployed address and source/commit hash publicly. This console
  //   output is the starting point for that -- copy it into the repo or
  //   release notes, don't just leave it in a terminal scrollback.
  console.log("=== D3R-AC deployment complete ===");
  console.log("Network:                 ", network);
  console.log("D3RACToken:              ", token.address);
  console.log("IdentityRegistry:        ", identityRegistry.address);
  console.log("DisbursementController:  ", disbursementController.address);
  console.log("RiskRegistry:            ", riskRegistry.address);
  console.log("FundingRequestRegistry:  ", fundingRequestRegistry.address);
  console.log("D3RACHub:                ", hub.address);
  console.log("MultiSigAdmin:           ", multisig.address, "(now the Hub's admin)");
  console.log("Multisig owners:         ", multisigOwners.join(", "));
  console.log("Multisig threshold:      ", multisigThreshold);
  console.log("===================================\n");
  console.log(
    "Next: verify each contract's bytecode/constructor args on Tronscan " +
      "before pointing the frontend or any real funds at these addresses " +
      "-- see docs/deployment-guide.md's Post-deployment section.\n"
  );
};
