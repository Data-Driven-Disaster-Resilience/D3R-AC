import { expect } from "chai";
import { ethers, deploy } from "./helpers.mjs";

describe("D3RACTokenFactory", function () {
  let owner, tokenOwner;
  let factory;

  beforeEach(async function () {
    [owner, tokenOwner] = await ethers.getSigners();
    factory = await deploy("D3RACTokenFactory", owner);
  });

  it("deploys a real D3RACToken via CREATE2", async function () {
    const salt = ethers.id("test-salt-1");
    const tx = await factory.deployToken(1_000_000, tokenOwner.address, salt);
    const receipt = await tx.wait();

    const event = receipt.logs
      .map((log) => { try { return factory.interface.parseLog(log); } catch { return null; } })
      .find((parsed) => parsed && parsed.name === "TokenDeployed");
    expect(event).to.not.be.undefined;

    const tokenAddress = event.args.token;
    const token = await ethers.getContractAt("D3RACToken", tokenAddress);
    expect(await token.symbol()).to.equal("D3RAC");
    expect(await token.balanceOf(tokenOwner.address)).to.equal(1_000_000n * 10n ** (await token.decimals()));
  });

  it("computeAddress matches an independent JS implementation of the TVM CREATE2 formula", async function () {
    // NOT compared against the real deployed address from deployToken --
    // this test runs on Hardhat's EVM, whose real `create2` opcode uses
    // EVM's 0xff-prefixed formula, while this factory's computeAddress
    // deliberately implements the TVM's 0x41-prefixed formula (see
    // D3RACTokenFactory.sol's own top-of-file doc comment for why that
    // distinction is real, not a typo). Those two are DIFFERENT
    // addresses on DIFFERENT virtual machines by design -- asserting
    // they match here would either be wrong, or would mean
    // computeAddress had been quietly changed to the WRONG formula for
    // its real, intended TVM deployment target just to pass a test on
    // the wrong VM. Instead, this independently re-derives the same
    // 0x41-prefixed formula in JS (mirroring exactly what
    // scripts/find-vanity-create2-salt.js does off-chain) and checks
    // the on-chain Solidity implementation agrees with it.
    const salt = ethers.id("test-salt-2");
    const initialSupply = 500_000n;
    const tokenOwnerAddr = tokenOwner.address;

    const D3RACToken = await ethers.getContractFactory("D3RACToken", owner);
    const creationCode = D3RACToken.bytecode;
    const encodedArgs = ethers.AbiCoder.defaultAbiCoder().encode(
      ["uint256", "address"],
      [initialSupply, tokenOwnerAddr]
    );
    const bytecodeHash = ethers.keccak256(ethers.concat([creationCode, encodedArgs]));

    const factoryAddress = await factory.getAddress();
    const expected = ethers.getAddress(
      "0x" +
        ethers
          .keccak256(
            ethers.concat([
              "0x41", // TVM's CREATE2 prefix byte, not EVM's 0xff
              factoryAddress,
              salt,
              bytecodeHash,
            ])
          )
          .slice(-40)
    );

    const predicted = await factory.computeAddress(initialSupply, tokenOwnerAddr, salt);
    expect(predicted).to.equal(expected);
  });

  it("reverts deploying twice with the same salt and args", async function () {
    const salt = ethers.id("test-salt-3");
    await factory.deployToken(1, tokenOwner.address, salt);
    await expect(factory.deployToken(1, tokenOwner.address, salt)).to.revert(ethers);
  });

  it("produces different addresses for different salts with identical args", async function () {
    const saltA = ethers.id("salt-a");
    const saltB = ethers.id("salt-b");
    const addrA = await factory.computeAddress(1, tokenOwner.address, saltA);
    const addrB = await factory.computeAddress(1, tokenOwner.address, saltB);
    expect(addrA).to.not.equal(addrB);
  });

  it("produces different addresses for identical salt with different constructor args", async function () {
    const salt = ethers.id("shared-salt");
    const addrA = await factory.computeAddress(1, tokenOwner.address, salt);
    const addrB = await factory.computeAddress(2, tokenOwner.address, salt);
    expect(addrA).to.not.equal(addrB);
  });

  it("emits TokenDeployed with the correct owner and salt", async function () {
    const salt = ethers.id("test-salt-4");
    const tx = await factory.deployToken(42, tokenOwner.address, salt);
    const receipt = await tx.wait();
    const event = receipt.logs
      .map((log) => { try { return factory.interface.parseLog(log); } catch { return null; } })
      .find((parsed) => parsed && parsed.name === "TokenDeployed");
    expect(event.args.owner).to.equal(tokenOwner.address);
    expect(event.args.salt).to.equal(salt);
  });

  it("the deployed token is fully functional (mint gated to owner)", async function () {
    const salt = ethers.id("test-salt-5");
    const tx = await factory.deployToken(0, tokenOwner.address, salt);
    const receipt = await tx.wait();
    const event = receipt.logs
      .map((log) => { try { return factory.interface.parseLog(log); } catch { return null; } })
      .find((parsed) => parsed && parsed.name === "TokenDeployed");
    const token = await ethers.getContractAt("D3RACToken", event.args.token);

    await expect(token.connect(owner).mint(owner.address, 100)).to.revert(ethers); // owner signer here is not the token's owner
    await token.connect(tokenOwner).mint(tokenOwner.address, 100);
    expect(await token.balanceOf(tokenOwner.address)).to.equal(100n);
  });
});
