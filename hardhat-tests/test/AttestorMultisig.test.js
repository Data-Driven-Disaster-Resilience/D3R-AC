const { expect } = require("chai");
const { ethers } = require("hardhat");
const fs = require("fs");
const path = require("path");

function loadArtifact(name) {
  return JSON.parse(fs.readFileSync(path.join(__dirname, "..", "artifacts-raw", `${name}.json`), "utf8"));
}

describe("AttestorMultisig (integration with the merged DisbursementController)", function () {
  let identityRegistry, token, controller, multisig;
  let admin, signer1, signer2, signer3, recipient;

  beforeEach(async function () {
    [admin, signer1, signer2, signer3, recipient] = await ethers.getSigners();

    const idArtifact = loadArtifact("IdentityRegistry");
    identityRegistry = await new ethers.ContractFactory(idArtifact.abi, idArtifact.bytecode, admin).deploy(
      admin.address
    );
    await identityRegistry.waitForDeployment();
    await identityRegistry.connect(admin).verifyRecipient(recipient.address, "Maiduguri Corridor");

    const tokenArtifact = loadArtifact("D3RACToken");
    token = await new ethers.ContractFactory(tokenArtifact.abi, tokenArtifact.bytecode, admin).deploy(
      0,
      admin.address
    );
    await token.waitForDeployment();

    const controllerArtifact = loadArtifact("DisbursementController");
    controller = await new ethers.ContractFactory(controllerArtifact.abi, controllerArtifact.bytecode, admin).deploy(
      await identityRegistry.getAddress(),
      admin.address
    );
    await controller.waitForDeployment();

    const multisigArtifact = loadArtifact("AttestorMultisig");
    multisig = await new ethers.ContractFactory(multisigArtifact.abi, multisigArtifact.bytecode, admin).deploy(
      await controller.getAddress(),
      [signer1.address, signer2.address, signer3.address],
      2
    );
    await multisig.waitForDeployment();

    await controller.connect(admin).setAttester(await multisig.getAddress(), true);
    await controller.connect(admin).setAttester(admin.address, false);

    await controller
      .connect(admin)
      .createCommitment(recipient.address, await token.getAddress(), "Maiduguri Corridor", ["phase 1"], [
        ethers.parseUnits("100", 6),
      ]);

    await token.connect(admin).mint(await controller.getAddress(), ethers.parseUnits("100", 6));
  });

  it("deploys with the correct signer set and threshold", async function () {
    expect(await multisig.signerCount()).to.equal(3);
    expect(await multisig.threshold()).to.equal(2);
  });

  it("does NOT attest after only one of two required approvals", async function () {
    await multisig.connect(signer1).proposeAttestation(0, 0);
    const [approvalCount, executed] = await multisig.getProposalStatus(0, 0);
    expect(approvalCount).to.equal(1);
    expect(executed).to.equal(false);

    const m = await controller.getMilestone(0, 0);
    expect(m.attested).to.equal(false);
  });

  it("attests on DisbursementController once the 2-of-3 threshold is met, and release then succeeds", async function () {
    await multisig.connect(signer1).proposeAttestation(0, 0);
    const tx = await multisig.connect(signer2).approveAttestation(0, 0);
    await expect(tx).to.emit(multisig, "ProposalExecuted");

    const m = await controller.getMilestone(0, 0);
    expect(m.attested).to.equal(true);

    await expect(controller.releaseMilestone(0, 0)).to.not.be.reverted;
    expect(await token.balanceOf(recipient.address)).to.equal(ethers.parseUnits("100", 6));
  });

  it("blocks non-signers from proposing or approving", async function () {
    await expect(multisig.connect(recipient).proposeAttestation(0, 0)).to.be.revertedWith(
      "AttestorMultisig: caller is not a signer"
    );
  });

  it("blocks a signer from approving the same proposal twice", async function () {
    await multisig.connect(signer1).proposeAttestation(0, 0);
    await expect(multisig.connect(signer1).approveAttestation(0, 0)).to.be.revertedWith(
      "AttestorMultisig: signer already approved this proposal"
    );
  });

  it("blocks approval on an already-executed proposal", async function () {
    await multisig.connect(signer1).proposeAttestation(0, 0);
    await multisig.connect(signer2).approveAttestation(0, 0);
    await expect(multisig.connect(signer3).approveAttestation(0, 0)).to.be.revertedWith(
      "AttestorMultisig: already executed"
    );
  });

  it("also confirms the old single-EOA attester was actually revoked", async function () {
    await expect(controller.connect(admin).attestMilestone(0, 0)).to.be.revertedWith(
      "DisbursementController: caller is not an attester"
    );
  });

  describe("self-administered signer governance", function () {
    it("adds a new signer once threshold admin-approvals are met", async function () {
      const tx1 = await multisig.connect(signer1).proposeAddSigner(recipient.address);
      const receipt1 = await tx1.wait();
      const createdEvent = receipt1.logs
        .map((l) => {
          try {
            return multisig.interface.parseLog(l);
          } catch {
            return null;
          }
        })
        .find((e) => e && e.name === "AdminProposalCreated");
      const key = createdEvent.args.key;

      expect(await multisig.isSigner(recipient.address)).to.equal(false);
      await multisig.connect(signer2).approveAdminProposal(key);
      expect(await multisig.isSigner(recipient.address)).to.equal(true);
      expect(await multisig.signerCount()).to.equal(4);
    });
  });
});
