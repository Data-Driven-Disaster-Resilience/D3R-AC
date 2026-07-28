import { network } from "hardhat";

/// Hardhat 3 removed the global `ethers` object that Hardhat 2's
/// hardhat-ethers plugin injected into the HRE; connections are now
/// explicit. A single shared connection is opened here (top-level await,
/// ESM-only) and re-exported so every test file gets the same `ethers`
/// instance it used to get via `require("hardhat")`.
const { ethers } = await network.connect();

/// Deploy a contract by name (must have been compiled via `npx hardhat
/// build`, which `npx hardhat test` runs automatically) from the given
/// signer, with constructor args passed through.
export async function deploy(name, signer, ...args) {
  const factory = await ethers.getContractFactory(name, signer);
  const contract = await factory.deploy(...args);
  await contract.waitForDeployment();
  return contract;
}

export { ethers };
