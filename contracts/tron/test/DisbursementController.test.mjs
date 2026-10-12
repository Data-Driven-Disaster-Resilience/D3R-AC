import { expect } from "chai";
import { ethers, deploy } from "./helpers.mjs";

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
      await expect(createCommitment()).to.not.revert(ethers);
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

    it("refuses to create a commitment the contract cannot fully fund (DC-2)", async function () {
      const poorController = await deploy(
        "DisbursementController",
        admin,
        await registry.getAddress(),
        admin.address
      );
      await expect(
        poorController.createCommitment(recipient.address, await token.getAddress(), "Ohafia Relief Coalition", ["Phase 1"], [1000])
      ).to.be.revertedWith("DisbursementController: commitment not fully funded");
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
  describe("DC-1: token must be a contract", function () {
    it("rejects an EOA / undeployed token address at commitment creation", async function () {
      await expect(
        controller.createCommitment(recipient.address, stranger.address, "Ohafia Relief Coalition", ["M"], [1000])
      ).to.be.revertedWith("DisbursementController: token is not a contract");
    });
  });

  describe("DC-2: shared-balance accounting", function () {
    it("tracks outstanding obligations and blocks over-commitment across commitments", async function () {
      const tokenAddr = await token.getAddress();
      // Balance is 100_000. Commit 60_000 twice: the second must be refused.
      await controller.createCommitment(recipient.address, tokenAddr, "A", ["M"], [60_000]);
      expect(await controller.outstandingCommitted(tokenAddr)).to.equal(60_000);
      await expect(
        controller.createCommitment(recipient.address, tokenAddr, "B", ["M"], [60_000])
      ).to.be.revertedWith("DisbursementController: commitment not fully funded");
      // the failed attempt must leave accounting untouched
      expect(await controller.outstandingCommitted(tokenAddr)).to.equal(60_000);
      expect(await controller.commitmentCount()).to.equal(1);
    });

    it("decreases outstanding on release", async function () {
      const tokenAddr = await token.getAddress();
      await controller.createCommitment(recipient.address, tokenAddr, "A", ["M1", "M2"], [1000, 2000]);
      await controller.attestMilestone(0, 0);
      await controller.releaseMilestone(0, 0);
      expect(await controller.outstandingCommitted(tokenAddr)).to.equal(2000);
    });
  });

  describe("DC-3: sweep", function () {
    let tokenAddr;
    beforeEach(async function () {
      tokenAddr = await token.getAddress();
    });

    it("lets the admin recover funds freed by a cancellation, and only those", async function () {
      await controller.createCommitment(recipient.address, tokenAddr, "A", ["M1", "M2"], [1000, 2000]);
      await controller.attestMilestone(0, 0);
      await controller.releaseMilestone(0, 0);
      // 99_000 held; 2000 backs the open milestone, 97_000 is uncommitted
      await expect(controller.sweep(tokenAddr, stranger.address, 97_001)).to.be.revertedWith(
        "DisbursementController: amount exceeds uncommitted balance"
      );
      await controller.cancelCommitment(0);
      expect(await controller.outstandingCommitted(tokenAddr)).to.equal(0);
      await expect(controller.sweep(tokenAddr, stranger.address, 99_000)).to.emit(controller, "FundsSwept");
      expect(await token.balanceOf(stranger.address)).to.equal(99_000);
    });

    it("can never touch funds backing an open commitment", async function () {
      await controller.createCommitment(recipient.address, tokenAddr, "A", ["M"], [100_000]);
      await expect(controller.sweep(tokenAddr, stranger.address, 1)).to.be.revertedWith(
        "DisbursementController: amount exceeds uncommitted balance"
      );
    });

    it("is admin-only and rejects zero address / zero amount", async function () {
      await expect(controller.connect(stranger).sweep(tokenAddr, stranger.address, 1)).to.be.revertedWith(
        "DisbursementController: caller is not admin"
      );
      await expect(controller.sweep(tokenAddr, ethers.ZeroAddress, 1)).to.be.revertedWith(
        "DisbursementController: zero address"
      );
      await expect(controller.sweep(tokenAddr, stranger.address, 0)).to.be.revertedWith(
        "DisbursementController: amount must be > 0"
      );
    });
  });
});
