const RiskRegistry = artifacts.require("RiskRegistry");
const FundingRequestRegistry = artifacts.require("FundingRequestRegistry");
const AttestorMultisig = artifacts.require("AttestorMultisig");
const DisbursementController = artifacts.require("DisbursementController");

// Deploys the v2 contract suite described in contracts/tron/README.md,
// built against the already-deployed DisbursementController /
// IdentityRegistry / D3RACToken (see migrations/1 and /2 for those).
//
// Env vars (all optional, sensible defaults for testnet iteration):
//   INITIAL_DATA_FEEDER_ADDRESS  - who can push risk updates (default: deployer)
//   INITIAL_PROPOSER_ADDRESS     - who can open funding requests (default: deployer)
//   RISK_THRESHOLD_1E18          - theta at 1e18 scale (default: 0.35e18, matches riskModel.ts)
//   ATTESTOR_SIGNERS             - comma-separated addresses for the multisig (default: deployer only)
//   ATTESTOR_THRESHOLD           - approvals required (default: 1, i.e. behaves like a single
//                                  attester until you actually add more signers via governance)
//
// This migration does NOT redeploy DisbursementController — it expects an
// already-deployed instance and wires the new AttestorMultisig into it as
// the attester, replacing whatever attester is currently set (by default,
// DisbursementController's own admin address).
module.exports = async function (deployer, network, accounts) {
  const feeder = process.env.INITIAL_DATA_FEEDER_ADDRESS || accounts[0];
  const proposer = process.env.INITIAL_PROPOSER_ADDRESS || accounts[0];
  const threshold1e18 = process.env.RISK_THRESHOLD_1E18 || "350000000000000000"; // 0.35
  const signers = process.env.ATTESTOR_SIGNERS
    ? process.env.ATTESTOR_SIGNERS.split(",").map((s) => s.trim())
    : [accounts[0]];
  const attestorThreshold = process.env.ATTESTOR_THRESHOLD || "1";

  console.log(`[${network}] Deploying RiskRegistry, theta=${threshold1e18}, feeder=${feeder}`);
  await deployer.deploy(RiskRegistry, threshold1e18, feeder);

  console.log(`[${network}] Deploying FundingRequestRegistry, proposer=${proposer}`);
  await deployer.deploy(FundingRequestRegistry, proposer);

  const controller = await DisbursementController.deployed();
  console.log(`[${network}] Deploying AttestorMultisig against DisbursementController at ${controller.address}`);
  console.log(`[${network}]   signers=${signers.join(", ")} threshold=${attestorThreshold}`);
  await deployer.deploy(AttestorMultisig, controller.address, signers, attestorThreshold);

  console.log(
    "\nNext manual step (not automated by this migration on purpose): " +
      "call DisbursementController.setAttester(AttestorMultisig.address, true) from the current admin account " +
      "once you've confirmed the multisig deployed as expected, then " +
      "setAttester(currentAttesterEOA, false) to retire the single-EOA attester."
  );
};
