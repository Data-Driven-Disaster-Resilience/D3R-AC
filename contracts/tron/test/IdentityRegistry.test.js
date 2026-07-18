const { expect } = require("chai");
const { ethers } = require("hardhat");
const { deploy } = require("./helpers");

describe("IdentityRegistry", function () {
  let admin, verifier, recipient, stranger;
  let registry;

  beforeEach(async function () {
    [admin, verifier, recipient, stranger] = await ethers.getSigners();
    registry = await deploy("IdentityRegistry", admin, admin.address);
  });

  it("makes the deployer admin and a verifier from the start", async function () {
    expect(await registry.admin()).to.equal(admin.address);
    expect(await registry.verifiers(admin.address)).to.equal(true);
  });

  it("only admin can grant/revoke verifier status", async function () {
    await expect(registry.connect(stranger).setVerifier(verifier.address, true)).to.be.revertedWith(
      "IdentityRegistry: caller is not admin"
    );
    await registry.setVerifier(verifier.address, true);
    expect(await registry.verifiers(verifier.address)).to.equal(true);
  });

  it("only a verifier can verify a recipient", async function () {
    await expect(
      registry.connect(stranger).verifyRecipient(recipient.address, "Ohafia Relief Coalition")
    ).to.be.revertedWith("IdentityRegistry: caller is not a verifier");
  });

  it("verifies a recipient and emits RecipientVerified", async function () {
    await expect(registry.verifyRecipient(recipient.address, "Ohafia Relief Coalition"))
      .to.emit(registry, "RecipientVerified")
      .withArgs(recipient.address, "Ohafia Relief Coalition", admin.address);

    expect(await registry.isVerified(recipient.address)).to.equal(true);
    const r = await registry.getRecipient(recipient.address);
    expect(r.community).to.equal("Ohafia Relief Coalition");
    expect(r.verifiedBy).to.equal(admin.address);
  });

  it("rejects verifying the zero address or an empty community label", async function () {
    await expect(registry.verifyRecipient(ethers.ZeroAddress, "X")).to.be.revertedWith(
      "IdentityRegistry: zero address"
    );
    await expect(registry.verifyRecipient(recipient.address, "")).to.be.revertedWith(
      "IdentityRegistry: community label required"
    );
  });

  it("revokes a verified recipient, preserving history", async function () {
    await registry.verifyRecipient(recipient.address, "Ohafia Relief Coalition");
    await expect(registry.revokeRecipient(recipient.address))
      .to.emit(registry, "RecipientRevoked")
      .withArgs(recipient.address, admin.address);

    expect(await registry.isVerified(recipient.address)).to.equal(false);
    const r = await registry.getRecipient(recipient.address);
    expect(r.community).to.equal("Ohafia Relief Coalition"); // history preserved
    expect(r.revokedAt).to.not.equal(0);
  });

  it("rejects revoking a recipient that was never verified", async function () {
    await expect(registry.revokeRecipient(recipient.address)).to.be.revertedWith(
      "IdentityRegistry: recipient not verified"
    );
  });

  it("only admin can transfer admin, and it takes effect immediately", async function () {
    await expect(registry.connect(stranger).transferAdmin(verifier.address)).to.be.revertedWith(
      "IdentityRegistry: caller is not admin"
    );
    await registry.transferAdmin(verifier.address);
    expect(await registry.admin()).to.equal(verifier.address);
    await expect(registry.setVerifier(stranger.address, true)).to.be.revertedWith(
      "IdentityRegistry: caller is not admin"
    );
  });
});
