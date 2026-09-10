#!/usr/bin/env node
// Mines a salt for D3RACTokenFactory.deployToken(...) such that the
// resulting token's address matches a chosen vanity prefix/suffix.
//
// TVM CREATE2 ADDRESS DERIVATION -- verified directly against
// D3RACTokenFactory.sol's own computeAddress() implementation
// (contracts/tron/tronbox/contracts/D3RACTokenFactory.sol), and
// cross-checked by that contract's own test file
// (test/D3RACTokenFactory.test.mjs's "computeAddress matches an
// independent JS implementation" test, which performs this exact
// derivation and asserts it agrees with the on-chain Solidity result).
// The one detail that matters and is easy to get wrong: the TVM's
// CREATE2 formula uses a 0x41 prefix byte, not the EVM's 0xff -- get
// this wrong and every salt mined here would predict an address that's
// never actually produced by a real deploy on TRON.
//
// Usage:
//   node scripts/find-vanity-create2-salt.js \
//     --factory <factory address, hex form, 0x...> \
//     --initial-supply 1000000 \
//     --owner <owner address, hex form, 0x...> \
//     --prefix d3r
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { ethers } from "ethers";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function parseArgs() {
  const args = process.argv.slice(2);
  const opts = { prefix: "", suffix: "", caseSensitive: false };
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--factory") opts.factory = args[++i];
    else if (args[i] === "--initial-supply") opts.initialSupply = args[++i];
    else if (args[i] === "--owner") opts.owner = args[++i];
    else if (args[i] === "--prefix") opts.prefix = args[++i];
    else if (args[i] === "--suffix") opts.suffix = args[++i];
    else if (args[i] === "--case-sensitive") opts.caseSensitive = true;
  }
  return opts;
}

function loadTokenCreationCode() {
  // Reads the compiled D3RACToken artifact directly rather than
  // re-encoding constructor bytecode by hand -- run `npm run compile`
  // first if this file doesn't exist yet.
  const artifactPath = path.join(
    __dirname,
    "..",
    "artifacts",
    "contracts",
    "D3RACToken.sol",
    "D3RACToken.json"
  );
  const artifact = JSON.parse(readFileSync(artifactPath, "utf8"));
  return artifact.bytecode;
}

function computeCreate2Address(factoryAddress, salt, bytecodeHash) {
  // TVM: 0x41 prefix, not EVM's 0xff -- see this file's top-of-file
  // comment for why.
  const hash = ethers.keccak256(
    ethers.concat(["0x41", factoryAddress, salt, bytecodeHash])
  );
  return ethers.getAddress("0x" + hash.slice(-40));
}

function matches(address, opts) {
  const body = address.slice(2); // strip "0x"
  const target = opts.caseSensitive ? body : body.toLowerCase();
  const prefix = opts.caseSensitive ? opts.prefix : opts.prefix.toLowerCase();
  const suffix = opts.caseSensitive ? opts.suffix : opts.suffix.toLowerCase();
  if (prefix && !target.startsWith(prefix)) return false;
  if (suffix && !target.endsWith(suffix)) return false;
  return true;
}

function main() {
  const opts = parseArgs();
  if (!opts.factory || opts.initialSupply === undefined || !opts.owner) {
    console.error(
      "Usage: node find-vanity-create2-salt.js --factory <0x...> --initial-supply <n> --owner <0x...> [--prefix <str>] [--suffix <str>] [--case-sensitive]"
    );
    process.exit(1);
  }
  if (!opts.prefix && !opts.suffix) {
    console.error("Provide --prefix and/or --suffix.");
    process.exit(1);
  }

  const creationCode = loadTokenCreationCode();
  const encodedArgs = ethers.AbiCoder.defaultAbiCoder().encode(
    ["uint256", "address"],
    [BigInt(opts.initialSupply), opts.owner]
  );
  const bytecodeHash = ethers.keccak256(ethers.concat([creationCode, encodedArgs]));

  console.log(`Mining a CREATE2 salt for factory=${opts.factory}, initialSupply=${opts.initialSupply}, owner=${opts.owner}`);
  console.log(`Target: prefix="${opts.prefix}" suffix="${opts.suffix}" (case-sensitive: ${opts.caseSensitive})`);

  const startedAt = Date.now();
  let attempts = 0;

  while (true) {
    attempts++;
    const salt = ethers.hexlify(ethers.randomBytes(32));
    const predicted = computeCreate2Address(opts.factory, salt, bytecodeHash);
    if (matches(predicted, opts)) {
      const elapsedSeconds = ((Date.now() - startedAt) / 1000).toFixed(1);
      console.log(`\nFound after ${attempts.toLocaleString()} attempts (${elapsedSeconds}s):`);
      console.log(`  Salt:             ${salt}`);
      console.log(`  Predicted address: ${predicted}`);
      console.log("\nDeploy with: factory.deployToken(initialSupply, owner, salt) using this exact salt.");
      console.log("Verify before deploying: factory.computeAddress(initialSupply, owner, salt) should return the same address printed above.");
      return;
    }
    if (attempts % 50000 === 0) {
      const elapsedSeconds = ((Date.now() - startedAt) / 1000).toFixed(1);
      console.log(`  ...${attempts.toLocaleString()} attempts (${elapsedSeconds}s)`);
    }
  }
}

main();
