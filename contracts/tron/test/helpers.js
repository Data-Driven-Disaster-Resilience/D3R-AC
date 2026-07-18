const { ethers } = require("hardhat");

/// Deploy a contract by name (must have been compiled via `npx hardhat
/// compile`, which `npx hardhat test` runs automatically) from the given
/// signer, with constructor args passed through.
async function deploy(name, signer, ...args) {
  const factory = await ethers.getContractFactory(name, signer);
  const contract = await factory.deploy(...args);
  await contract.waitForDeployment();
  return contract;
}

module.exports = { deploy };
