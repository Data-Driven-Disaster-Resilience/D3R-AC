import { expect } from "chai";
import { ethers, deploy } from "./helpers.js";

describe("MultiSigAdmin", function () {
  let ownerA, ownerB, ownerC, stranger, target;
  let multisig;

  beforeEach(async function () {
    [ownerA, ownerB, ownerC, stranger, target] = await ethers.getSigners();
    multisig = await deploy("MultiSigAdmin", ownerA, [ownerA.address, ownerB.address, ownerC.address], 2);
  });

  it("rejects deployment with an invalid threshold", async function () {
    await expect(
      deploy("MultiSigAdmin", ownerA, [ownerA.address, ownerB.address], 0)
    ).to.be.reverted;
    await expect(
      deploy("MultiSigAdmin", ownerA, [ownerA.address, ownerB.address], 3)
    ).to.be.reverted;
  });

  it("rejects a duplicate or zero-address owner at deployment", async function () {
    await expect(
      deploy("MultiSigAdmin", ownerA, [ownerA.address, ownerA.address], 1)
    ).to.be.revertedWith("MultiSigAdmin: duplicate owner");
    await expect(
      deploy("MultiSigAdmin", ownerA, [ownerA.address, ethers.ZeroAddress], 1)
    ).to.be.revertedWith("MultiSigAdmin: zero address owner");
  });

  it("only an owner can submit a transaction", async function () {
    await expect(
      multisig.connect(stranger).submitTransaction(target.address, 0, "0x")
    ).to.be.revertedWith("MultiSigAdmin: caller is not an owner");
  });

  it("auto-confirms from the submitter but does not execute below threshold", async function () {
    await multisig.connect(ownerA).submitTransaction(target.address, 0, "0x");
    const tx = await multisig.getTransaction(0);
    expect(tx.confirmationCount).to.equal(1);
    expect(tx.executed).to.equal(false);

    await expect(multisig.connect(ownerA).executeTransaction(0)).to.be.revertedWith(
      "MultiSigAdmin: insufficient confirmations"
    );
  });

  it("executes once threshold confirmations are reached", async function () {
    await multisig.connect(ownerA).submitTransaction(target.address, 0, "0x");
    await multisig.connect(ownerB).confirmTransaction(0);
    await expect(multisig.connect(ownerA).executeTransaction(0)).to.emit(multisig, "TransactionExecuted");
    const tx = await multisig.getTransaction(0);
    expect(tx.executed).to.equal(true);
  });

  it("rejects a double confirmation from the same owner", async function () {
    await multisig.connect(ownerA).submitTransaction(target.address, 0, "0x");
    await expect(multisig.connect(ownerA).confirmTransaction(0)).to.be.revertedWith(
      "MultiSigAdmin: already confirmed"
    );
  });

  it("allows revoking a confirmation before execution", async function () {
    await multisig.connect(ownerA).submitTransaction(target.address, 0, "0x");
    await multisig.connect(ownerB).confirmTransaction(0);
    await multisig.connect(ownerB).revokeConfirmation(0);
    await expect(multisig.connect(ownerA).executeTransaction(0)).to.be.revertedWith(
      "MultiSigAdmin: insufficient confirmations"
    );
  });

  it("rejects executing the same transaction twice", async function () {
    await multisig.connect(ownerA).submitTransaction(target.address, 0, "0x");
    await multisig.connect(ownerB).confirmTransaction(0);
    await multisig.connect(ownerA).executeTransaction(0);
    await expect(multisig.connect(ownerA).executeTransaction(0)).to.be.revertedWith(
      "MultiSigAdmin: transaction already executed"
    );
  });

  it("reverts the whole execution (and leaves it re-executable) if the underlying call reverts", async function () {
    // Target a call that will revert: calling setVerifier on a registry
    // the multisig does NOT administer.
    const registry = await deploy("IdentityRegistry", ownerA, ownerA.address); // admin = ownerA, not the multisig
    const iface = new ethers.Interface(["function setVerifier(address account, bool isVerifier)"]);
    const data = iface.encodeFunctionData("setVerifier", [target.address, true]);

    await multisig.connect(ownerA).submitTransaction(await registry.getAddress(), 0, data);
    await multisig.connect(ownerB).confirmTransaction(0);
    await expect(multisig.connect(ownerA).executeTransaction(0)).to.be.revertedWith(
      "MultiSigAdmin: underlying call reverted"
    );
    const tx = await multisig.getTransaction(0);
    expect(tx.executed).to.equal(false); // rolled back, not stuck
  });

  it("integration: can genuinely hold the admin role on IdentityRegistry", async function () {
    const registry = await deploy("IdentityRegistry", ownerA, await multisig.getAddress());
    expect(await registry.admin()).to.equal(await multisig.getAddress());

    const iface = new ethers.Interface(["function setVerifier(address account, bool isVerifier)"]);
    const data = iface.encodeFunctionData("setVerifier", [target.address, true]);

    await multisig.connect(ownerA).submitTransaction(await registry.getAddress(), 0, data);
    await multisig.connect(ownerB).confirmTransaction(0);
    await multisig.connect(ownerA).executeTransaction(0);

    expect(await registry.verifiers(target.address)).to.equal(true);
  });

  it("integration: can genuinely hold D3RACHub's admin role, so a single confirmed EOA cannot act alone -- the intended top-of-chain architecture (MultiSigAdmin administers the Hub, the Hub administers everything else)", async function () {
    const token = await deploy("D3RACToken", ownerA, 1000, ownerA.address);
    const registry = await deploy("IdentityRegistry", ownerA, ownerA.address);
    const controller = await deploy("DisbursementController", ownerA, await registry.getAddress(), ownerA.address);

    const hub = await deploy(
      "D3RACHub", ownerA,
      await multisig.getAddress(), // admin = the multisig, not an EOA
      await token.getAddress(),
      await registry.getAddress(),
      await controller.getAddress(),
      ethers.ZeroAddress,
      ethers.ZeroAddress
    );
    expect(await hub.admin()).to.equal(await multisig.getAddress());

    // A single multisig owner cannot act on the Hub directly -- the Hub's
    // admin is the multisig contract, not any one of its owners.
    await expect(hub.connect(ownerA).pause()).to.be.revertedWith("D3RACHub: caller is not admin");

    const hubIface = new ethers.Interface(["function pause()"]);
    const pauseData = hubIface.encodeFunctionData("pause", []);
    await multisig.connect(ownerA).submitTransaction(await hub.getAddress(), 0, pauseData);

    // One confirmation (threshold is 2) is not enough.
    await expect(multisig.connect(ownerA).executeTransaction(0)).to.be.revertedWith(
      "MultiSigAdmin: insufficient confirmations"
    );

    await multisig.connect(ownerB).confirmTransaction(0);
    await multisig.connect(ownerA).executeTransaction(0);

    expect(await hub.paused()).to.equal(true); // real Hub state changed, via 2-of-3 multisig confirmation
  });
});
