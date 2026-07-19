const { expect } = require("chai");
const { ethers } = require("hardhat");
const { deploy } = require("./helpers");

describe("FundingRequestRegistry", function () {
  let registry, owner, proposer, stranger;
  const c3 = ethers.encodeBytes32String("c3");

  beforeEach(async function () {
    [owner, proposer, stranger] = await ethers.getSigners();
    registry = await deploy("FundingRequestRegistry", owner, proposer.address);
  });

  it("opens a request and emits RequestOpened", async function () {
    const tx = await registry
      .connect(proposer)
      .openRequest(c3, ethers.parseUnits("500", 6), "Shelter kits for flood-displaced families", "ipfs://bafybeigd.../hazard-report.json");
    await expect(tx).to.emit(registry, "RequestOpened");

    const r = await registry.getRequest(0);
    expect(r.communityId).to.equal(c3);
    expect(r.amountRequested).to.equal(ethers.parseUnits("500", 6));
    expect(r.status).to.equal(0); // Open
  });

  it("blocks non-proposers from opening a request", async function () {
    await expect(registry.connect(stranger).openRequest(c3, 100, "desc", "uri")).to.be.revertedWith(
      "FundingRequestRegistry: caller is not an authorized proposer"
    );
  });

  it("rejects a zero-amount request", async function () {
    await expect(registry.connect(proposer).openRequest(c3, 0, "desc", "uri")).to.be.revertedWith(
      "FundingRequestRegistry: amount must be > 0"
    );
  });

  describe("pledges and status transitions", function () {
    beforeEach(async function () {
      await registry.connect(proposer).openRequest(c3, ethers.parseUnits("500", 6), "desc", "uri");
    });

    it("moves Open -> PartiallyFunded -> Funded as pledges accumulate", async function () {
      await registry.connect(proposer).recordPledge(0, ethers.parseUnits("200", 6), "donor-platform-ref-1");
      let r = await registry.getRequest(0);
      expect(r.status).to.equal(1); // PartiallyFunded

      await registry.connect(proposer).recordPledge(0, ethers.parseUnits("300", 6), "donor-platform-ref-2");
      r = await registry.getRequest(0);
      expect(r.status).to.equal(2); // Funded
      expect(r.amountPledged).to.equal(ethers.parseUnits("500", 6));
    });

    it("blocks a stranger from recording a pledge on someone else's request", async function () {
      await expect(
        registry.connect(stranger).recordPledge(0, ethers.parseUnits("100", 6), "fake")
      ).to.be.revertedWith("FundingRequestRegistry: not authorized to record a pledge on this request");
    });

    it("links a request to a DisbursementController commitment id", async function () {
      const tx = await registry.connect(proposer).linkToCommitment(0, 7);
      await expect(tx).to.emit(registry, "RequestLinkedToCommitment").withArgs(0, 7);
      const r = await registry.getRequest(0);
      expect(r.linkedCommitmentId).to.equal(7);
    });

    it("closes a request and blocks further pledges", async function () {
      await registry.connect(proposer).closeRequest(0);
      const r = await registry.getRequest(0);
      expect(r.status).to.equal(3); // Closed

      await expect(
        registry.connect(proposer).recordPledge(0, ethers.parseUnits("10", 6), "late")
      ).to.be.revertedWith("FundingRequestRegistry: request not open");
    });

    it("allows the registry owner (not just the requester) to manage a request", async function () {
      await expect(registry.recordPledge(0, ethers.parseUnits("50", 6), "owner-recorded")).to.not.be.reverted;
    });
  });

  it("only owner can add/remove proposers", async function () {
    await expect(registry.connect(stranger).addProposer(stranger.address)).to.be.revertedWith(
      "FundingRequestRegistry: caller is not owner"
    );
    await registry.addProposer(stranger.address);
    expect(await registry.proposers(stranger.address)).to.equal(true);
  });
});
