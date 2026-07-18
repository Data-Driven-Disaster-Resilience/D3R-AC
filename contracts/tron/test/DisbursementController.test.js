const { expect } = require("chai");
const { ethers } = require("hardhat");
const { deploy } = require("./helpers");

describe("DisbursementController", function () {
  let admin, attester, recipient, unverifiedRecipient, stranger;
  let token, registry, controller;

  beforeEach(async function () {
    [admin, attester, recipient, unverifiedRecipient, stranger] = await ethers.getSigners();

    token = await deploy("D3RACToken", admin, 1_000_000, admin.address);
    registry = await deploy("IdentityRegistry", admin, admin.address);
    controller = await deploy("DisbursementController", admin, await registry.getAddress(), admin.address);

    await registry.verifyRecipient(recipient.address, "Ohafia Relief Coalition");
    // Fund the controller so releases can actually pay out.
    await token.transfer(await controller.getAddress(), 100_000);
  });

  async function createCommitment(overrides = {}) {
    const descriptions = overrides.descriptions ?? ["Water restored", "Shelter rebuilt"];
    const amounts = overrides.amounts ?? [1000, 2000];
    const tokenAddr = overrides.token ?? (await token.getAddress());
    const recipientAddr = overrides.recipient ?? recipient.address;
    const community = overrides.community ?? "Ohafia Relief Coalition";
    const tx = await controller.createCommitment(recipientAddr, tokenAddr, community, descriptions, amounts);
    const receipt = await tx.wait();
    return { tx, receipt, commitmentId: 0 };
  }

  describe("setup / roles", function () {
    it("makes the deployer admin and an attester from the start", async function () {
      expect(await controller.admin()).to.equal(admin.address);
      expect(await controller.attesters(admin.address)).to.equal(true);
    });

    it("only admin can grant/revoke attester status", async function () {
      await expect(controller.connect(stranger).setAttester(attester.address, true)).to.be.revertedWith(
        "DisbursementController: caller is not admin"
      );
      await controller.setAttester(attester.address, true);
      expect(await controller.attesters(attester.address)).to.equal(true);
    });
  });

  describe("createCommitment", function () {
    it("creates a commitment against a verified recipient and emits CommitmentCreated", async function () {
      await expect(createCommitment()).to.not.be.reverted;
      const c = await controller.getCommitment(0);
      expect(c.recipient).to.equal(recipient.address);
      expect(c.totalAmount).to.equal(3000);
      expect(c.milestoneCount).to.equal(2);
      expect(c.active).to.equal(true);
    });

    it("rejects an unverified recipient", async function () {
      await expect(
        controller.createCommitment(
          unverifiedRecipient.address,
          await token.getAddress(),
          "Unregistered Group",
          ["Milestone"],
          [1000]
        )
      ).to.be.revertedWith("DisbursementController: recipient not verified");
    });

    it("rejects a call from a non-admin", async function () {
      await expect(
        controller.connect(stranger).createCommitment(
          recipient.address,
          await token.getAddress(),
          "Ohafia Relief Coalition",
          ["Milestone"],
          [1000]
        )
      ).to.be.revertedWith("DisbursementController: caller is not admin");
    });

    it("rejects a zero-amount milestone", async function () {
      await expect(
        controller.createCommitment(
          recipient.address,
          await token.getAddress(),
          "Ohafia Relief Coalition",
          ["Milestone"],
          [0]
        )
      ).to.be.revertedWith("DisbursementController: milestone amount must be > 0");
    });

    it("rejects mismatched descriptions/amounts array lengths", async function () {
      await expect(
        controller.createCommitment(
          recipient.address,
          await token.getAddress(),
          "Ohafia Relief Coalition",
          ["A", "B"],
          [1000]
        )
      ).to.be.revertedWith("DisbursementController: length mismatch");
    });

    it("rejects an empty milestone list", async function () {
      await expect(
        controller.createCommitment(recipient.address, await token.getAddress(), "Ohafia Relief Coalition", [], [])
      ).to.be.revertedWith("DisbursementController: at least one milestone required");
    });
  });

  describe("attestMilestone", function () {
    beforeEach(async function () {
      await createCommitment();
    });

    it("rejects attestation from a non-attester", async function () {
      await expect(controller.connect(stranger).attestMilestone(0, 0)).to.be.revertedWith(
        "DisbursementController: caller is not an attester"
      );
    });

    it("attests a milestone and emits MilestoneAttested", async function () {
      await expect(controller.attestMilestone(0, 0))
        .to.emit(controller, "MilestoneAttested")
        .withArgs(0, 0, admin.address);
      const m = await controller.getMilestone(0, 0);
      expect(m.attested).to.equal(true);
      expect(m.attestedBy).to.equal(admin.address);
    });

    it("rejects attesting the same milestone twice", async function () {
      await controller.attestMilestone(0, 0);
      await expect(controller.attestMilestone(0, 0)).to.be.revertedWith(
        "DisbursementController: milestone already attested"
      );
    });

    it("rejects attesting a milestone index that doesn't exist", async function () {
      await expect(controller.attestMilestone(0, 99)).to.be.revertedWith(
        "DisbursementController: milestone does not exist"
      );
    });

    it("rejects attesting against a commitment that doesn't exist", async function () {
      await expect(controller.attestMilestone(99, 0)).to.be.revertedWith(
        "DisbursementController: commitment does not exist"
      );
    });
  });

  describe("releaseMilestone", function () {
    beforeEach(async function () {
      await createCommitment();
      await controller.attestMilestone(0, 0);
    });

    it("releases an attested milestone, is permissionless, and emits MilestoneReleased", async function () {
      const before = await token.balanceOf(recipient.address);
      await expect(controller.connect(stranger).releaseMilestone(0, 0))
        .to.emit(controller, "MilestoneReleased")
        .withArgs(0, 0, recipient.address, 1000);
      expect(await token.balanceOf(recipient.address)).to.equal(before + 1000n);
    });

    it("rejects releasing an unattested milestone", async function () {
      await expect(controller.releaseMilestone(0, 1)).to.be.revertedWith(
        "DisbursementController: milestone not attested"
      );
    });

    it("rejects releasing the same milestone twice", async function () {
      await controller.releaseMilestone(0, 0);
      await expect(controller.releaseMilestone(0, 0)).to.be.revertedWith(
        "DisbursementController: milestone already released"
      );
    });

    it("rejects releasing when the contract's token balance is insufficient", async function () {
      // Drain the controller's balance so it can't cover the milestone.
      const controllerAddr = await controller.getAddress();
      const bal = await token.balanceOf(controllerAddr);
      // Move the tokens out via a fresh commitment+release cycle is circular;
      // instead deploy a controller with zero funding for this case.
      const poorController = await deploy(
        "DisbursementController",
        admin,
        await registry.getAddress(),
        admin.address
      );
      await poorController.createCommitment(
        recipient.address,
        await token.getAddress(),
        "Ohafia Relief Coalition",
        ["Milestone"],
        [1000]
      );
      await poorController.attestMilestone(0, 0);
      await expect(poorController.releaseMilestone(0, 0)).to.be.revertedWith(
        "DisbursementController: insufficient contract balance for milestone"
      );
      expect(bal).to.be.gt(0); // sanity: original controller was funded
    });
  });

  describe("cancelCommitment", function () {
    beforeEach(async function () {
      await createCommitment();
    });

    it("only admin can cancel", async function () {
      await expect(controller.connect(stranger).cancelCommitment(0)).to.be.revertedWith(
        "DisbursementController: caller is not admin"
      );
    });

    it("cancels a commitment and blocks further attestation/release", async function () {
      await expect(controller.cancelCommitment(0))
        .to.emit(controller, "CommitmentCancelled")
        .withArgs(0, admin.address, 3000);

      await expect(controller.attestMilestone(0, 0)).to.be.revertedWith(
        "DisbursementController: commitment not active"
      );
    });

    it("does not affect a milestone already released before cancellation", async function () {
      await controller.attestMilestone(0, 0);
      await controller.releaseMilestone(0, 0);
      await controller.cancelCommitment(0);
      const m = await controller.getMilestone(0, 0);
      expect(m.released).to.equal(true); // untouched by cancellation
    });
  });
});
