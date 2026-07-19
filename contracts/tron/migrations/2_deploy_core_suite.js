const IdentityRegistry = artifacts.require("IdentityRegistry");
const D3RACToken = artifacts.require("D3RACToken");
const DisbursementController = artifacts.require("DisbursementController");

// Deploys the core v1 suite (IdentityRegistry, D3RACToken,
// DisbursementController) that was merged into contracts/tron/ directly
// without TronBox scaffolding of its own. This migration adds that
// scaffolding retroactively so `tronbox migrate` can deploy the whole
// suite in one pass; migrations/3_deploy_v2_suite.js then builds the
// v2 additions (RiskRegistry, FundingRequestRegistry, AttestorMultisig)
// on top of what this migration deploys.
//
// Env vars (all optional, default to the deployer account for fast
// testnet iteration — replace with real multisig/treasury addresses
// before any mainnet use, per docs/deployment-guide.md):
//   IDENTITY_REGISTRY_ADMIN     - admin/verifier for IdentityRegistry
//   D3RAC_TOKEN_INITIAL_SUPPLY  - initial D3RACToken supply (default: 0, mint as needed)
//   D3RAC_TOKEN_OWNER           - owner of D3RACToken
//   DISBURSEMENT_ADMIN          - admin/default attester for DisbursementController
module.exports = async function (deployer, network, accounts) {
  const identityAdmin = process.env.IDENTITY_REGISTRY_ADMIN || accounts[0];
  const tokenSupply = process.env.D3RAC_TOKEN_INITIAL_SUPPLY || "0";
  const tokenOwner = process.env.D3RAC_TOKEN_OWNER || accounts[0];
  const disbursementAdmin = process.env.DISBURSEMENT_ADMIN || accounts[0];

  console.log(`[${network}] Deploying IdentityRegistry, admin=${identityAdmin}`);
  await deployer.deploy(IdentityRegistry, identityAdmin);

  console.log(`[${network}] Deploying D3RACToken, initialSupply=${tokenSupply}, owner=${tokenOwner}`);
  await deployer.deploy(D3RACToken, tokenSupply, tokenOwner);

  const identityRegistry = await IdentityRegistry.deployed();
  console.log(`[${network}] Deploying DisbursementController, registry=${identityRegistry.address}, admin=${disbursementAdmin}`);
  await deployer.deploy(DisbursementController, identityRegistry.address, disbursementAdmin);
};
