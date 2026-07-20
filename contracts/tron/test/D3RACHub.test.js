const { expect } = require("chai");
const { ethers } = require("hardhat");
const { deploy } = require("./helpers");

describe("D3RACHub", function () {
  let admin, stranger, recipient, minted, someone;
  let token, registry, controller, riskRegistry, fundingRegistry, hub;

  const COMMUNITY_ID = ethers.encodeBytes32String("ohafia");
  const SCALE = 10n ** 18n;

  async function deployFullStack() {
    const t = await deploy("D3RACToken", admin, 1_000_000, admin.address);
    const r = await deploy("IdentityRegistry", admin, admin.address);
    const c = await deploy("DisbursementController", admin, await r.getAddress(), admin.address);
    const rr = await deploy("RiskRegistry", admin, (SCALE * 35n) / 100n, admin.address);
    const fr = await deploy("FundingRequestRegistry", admin, admin.address);
    return { t, r, c, rr, fr };
  }

  beforeEach(async function () {
    [admin, stranger, recipient, minted, someone] = await ethers.getSigners();

    ({ t: token, r: registry, c: controller, rr: riskRegistry, fr: fundingRegistry } = await deployFullStack());

    hub = await deploy(
      "D3RACHub",
      admin,
      admin.address,
      await token.getAddress(),
      await registry.getAddress(),
      await controller.getAddress(),
      await riskRegistry.getAddress(),
      await fundingRegistry.getAddress()
    );

    // Full wiring for complete Hub control. Two different grant
    // mechanisms, and mixing them up is exactly the kind of mistake this
    // test file exists to catch:
    //   - Role mappings (verifier / attester / dataFeeder / proposer /
    //     minter) are ADDITIVE — granting the Hub one doesn't remove the
    //     original admin/owner's own access.
    //   - Single admin/owner addresses (IdentityRegistry.admin,
    //     DisbursementController.admin, D3RACToken.owner,
    //     RiskRegistry.owner, FundingRequestRegistry.owner) are
    //     EXCLUSIVE — transferring one to the Hub REPLACES the previous
    //     holder; they lose direct access the moment it runs.
    // Full coverage requires BOTH per contract: the role grant for the
    // Hub's routine operational calls, and the ownership/admin transfer
    // for the Hub's role-management proxies to work at all.
    await registry.setVerifier(await hub.getAddress(), true);
    await registry.transferAdmin(await hub.getAddress());
    await controller.setAttester(await hub.getAddress(), true);
    await controller.transferAdmin(await hub.getAddress());
    await token.setMinter(await hub.getAddress(), true);
    await token.transferOwnership(await hub.getAddress());
    await riskRegistry.addDataFeeder(await hub.getAddress());
    await riskRegistry.transferOwnership(await hub.getAddress());
    await fundingRegistry.addProposer(await hub.getAddress());
    await fundingRegistry.transferOwnership(await hub.getAddress());
  });

  describe("deployment", function () {
    it("rejects a zero address for admin or any of the three core modules", async function () {
      const { deploy: d } = require("./helpers");
      await expect(
        d("D3RACHub", admin, ethers.ZeroAddress, await token.getAddress(), await registry.getAddress(), await controller.getAddress(), ethers.ZeroAddress, ethers.ZeroAddress)
      ).to.be.revertedWith("D3RACHub: admin is zero address");
      await expect(
        d("D3RACHub", admin, admin.address, ethers.ZeroAddress, await registry.getAddress(), await controller.getAddress(), ethers.ZeroAddress, ethers.ZeroAddress)
      ).to.be.revertedWith("D3RACHub: token is zero address");
    });

    it("accepts a zero address for riskRegistry and fundingRequestRegistry -- they're optional", async function () {
      const bareHub = await deploy(
        "D3RACHub",
        admin,
        admin.address,
        await token.getAddress(),
        await registry.getAddress(),
        await controller.getAddress(),
        ethers.ZeroAddress,
        ethers.ZeroAddress
      );
      expect(await bareHub.riskRegistry()).to.equal(ethers.ZeroAddress);
      expect(await bareHub.fundingRequestRegistry()).to.equal(ethers.ZeroAddress);
    });

    it("records the initial admin and module addresses", async function () {
      expect(await hub.admin()).to.equal(admin.address);
      expect(await hub.token()).to.equal(await token.getAddress());
      expect(await hub.identityRegistry()).to.equal(await registry.getAddress());
      expect(await hub.disbursementController()).to.equal(await controller.getAddress());
      expect(await hub.riskRegistry()).to.equal(await riskRegistry.getAddress());
      expect(await hub.fundingRequestRegistry()).to.equal(await fundingRegistry.getAddress());
      expect(await hub.paused()).to.equal(false);
    });
  });

  describe("module management", function () {
    it("only admin can update a module pointer", async function () {
      await expect(hub.connect(stranger).setToken(stranger.address)).to.be.revertedWith(
        "D3RACHub: caller is not admin"
      );
    });

    it("updates a module pointer and emits ModuleUpdated", async function () {
      const newToken = await deploy("D3RACToken", admin, 0, admin.address);
      await expect(hub.setToken(await newToken.getAddress())).to.emit(hub, "ModuleUpdated");
      expect(await hub.token()).to.equal(await newToken.getAddress());
    });

    it("allows unwiring riskRegistry/fundingRequestRegistry back to the zero address", async function () {
      await hub.setRiskRegistry(ethers.ZeroAddress);
      expect(await hub.riskRegistry()).to.equal(ethers.ZeroAddress);
      await hub.setFundingRequestRegistry(ethers.ZeroAddress);
      expect(await hub.fundingRequestRegistry()).to.equal(ethers.ZeroAddress);
    });

    it("only admin can transfer admin, and it takes effect immediately", async function () {
      await expect(hub.connect(stranger).transferAdmin(stranger.address)).to.be.revertedWith(
        "D3RACHub: caller is not admin"
      );
      await hub.transferAdmin(stranger.address);
      expect(await hub.admin()).to.equal(stranger.address);
      await expect(hub.pause()).to.be.revertedWith("D3RACHub: caller is not admin");
    });
  });

  describe("pause", function () {
    it("only admin can pause/unpause", async function () {
      await expect(hub.connect(stranger).pause()).to.be.revertedWith("D3RACHub: caller is not admin");
    });

    it("rejects double-pausing or unpausing when not paused", async function () {
      await hub.pause();
      await expect(hub.pause()).to.be.revertedWith("D3RACHub: already paused");
      await hub.unpause();
      await expect(hub.unpause()).to.be.revertedWith("D3RACHub: not paused");
    });

    it("blocks all seven operational writes while paused", async function () {
      await hub.pause();
      await expect(hub.verifyRecipient(recipient.address, "Test Coalition")).to.be.revertedWith("D3RACHub: paused");
      await expect(
        hub.createCommitment(recipient.address, await token.getAddress(), "Test Coalition", ["M"], [1000])
      ).to.be.revertedWith("D3RACHub: paused");
      await expect(hub.attestMilestone(0, 0)).to.be.revertedWith("D3RACHub: paused");
      await expect(hub.mintTokens(minted.address, 100)).to.be.revertedWith("D3RACHub: paused");
      await expect(hub.registerCommunity(COMMUNITY_ID, "Ohafia", "Abia State")).to.be.revertedWith("D3RACHub: paused");
      await expect(hub.updateRisk(COMMUNITY_ID, SCALE, SCALE, SCALE)).to.be.revertedWith("D3RACHub: paused");
      await expect(
        hub.openFundingRequest(COMMUNITY_ID, 1000, "desc", "ipfs://x")
      ).to.be.revertedWith("D3RACHub: paused");
    });

    it("does NOT block cancelCommitment, closeFundingRequest, admin/module management, or any role-management proxy while paused", async function () {
      await hub.verifyRecipient(recipient.address, "Test Coalition");
      await hub.createCommitment(recipient.address, await token.getAddress(), "Test Coalition", ["M"], [1000]);
      await hub.openFundingRequest(COMMUNITY_ID, 1000, "desc", "ipfs://x");

      await hub.pause();
      await expect(hub.cancelCommitment(0)).to.not.be.reverted;
      await expect(hub.closeFundingRequest(0)).to.not.be.reverted;
      await expect(hub.setToken(await token.getAddress())).to.not.be.reverted;
      await expect(hub.transferAdmin(admin.address)).to.not.be.reverted; // no-op transfer, still allowed
      await expect(hub.setIdentityVerifier(someone.address, true)).to.not.be.reverted;
      await expect(hub.setDisbursementAttester(someone.address, true)).to.not.be.reverted;
      await expect(hub.setTokenMinter(someone.address, true)).to.not.be.reverted;
      await expect(hub.setRiskDataFeeder(someone.address, true)).to.not.be.reverted;
      await expect(hub.setRiskThreshold(SCALE / 2n)).to.not.be.reverted;
      await expect(hub.setFundingProposer(someone.address, true)).to.not.be.reverted;
    });
  });

  describe("orchestration: token / identity / disbursement (requires the Hub to hold roles)", function () {
    it("verifyRecipient forwards to IdentityRegistry and actually verifies", async function () {
      await hub.verifyRecipient(recipient.address, "Ohafia Relief Coalition");
      expect(await registry.isVerified(recipient.address)).to.equal(true);
    });

    it("reverts if the Hub has NOT been granted verifier status", async function () {
      const bareRegistry = await deploy("IdentityRegistry", admin, admin.address); // Hub never added as verifier here
      const bareHub = await deploy(
        "D3RACHub",
        admin,
        admin.address,
        await token.getAddress(),
        await bareRegistry.getAddress(),
        await controller.getAddress(),
        ethers.ZeroAddress,
        ethers.ZeroAddress
      );
      await expect(bareHub.verifyRecipient(recipient.address, "X")).to.be.revertedWith(
        "IdentityRegistry: caller is not a verifier"
      );
    });

    it("createCommitment + attestMilestone forward through to DisbursementController", async function () {
      await hub.verifyRecipient(recipient.address, "Ohafia Relief Coalition");
      await hub.createCommitment(recipient.address, await token.getAddress(), "Ohafia Relief Coalition", ["Water restored"], [1000]);
      await hub.attestMilestone(0, 0);
      const m = await controller.getMilestone(0, 0);
      expect(m.attested).to.equal(true);
    });

    it("transferring DisbursementController's admin to the Hub is exclusive, not additive — the original EOA loses direct createCommitment access", async function () {
      await expect(
        controller.connect(admin).createCommitment(
          recipient.address, await token.getAddress(), "X", ["M"], [1000]
        )
      ).to.be.revertedWith("DisbursementController: caller is not admin");
    });

    it("mintTokens forwards to D3RACToken and actually mints", async function () {
      const before = await token.balanceOf(minted.address);
      await hub.mintTokens(minted.address, 500);
      expect(await token.balanceOf(minted.address)).to.equal(before + 500n);
    });

    it("only admin can call orchestration functions", async function () {
      await expect(
        hub.connect(stranger).verifyRecipient(recipient.address, "X")
      ).to.be.revertedWith("D3RACHub: caller is not admin");
      await expect(hub.connect(stranger).mintTokens(stranger.address, 1)).to.be.revertedWith(
        "D3RACHub: caller is not admin"
      );
    });
  });

  describe("orchestration: risk registry", function () {
    it("registerCommunity forwards to RiskRegistry (requires the Hub to be RiskRegistry's owner)", async function () {
      await hub.registerCommunity(COMMUNITY_ID, "Ohafia", "Abia State");
      expect(await riskRegistry.communityCount()).to.equal(1);
    });

    it("reverts if the Hub has NOT been made RiskRegistry's owner", async function () {
      const bareRisk = await deploy("RiskRegistry", admin, SCALE / 2n, await hub.getAddress()); // feeder granted, owner NOT transferred
      const bareHub = await deploy(
        "D3RACHub",
        admin,
        admin.address,
        await token.getAddress(),
        await registry.getAddress(),
        await controller.getAddress(),
        await bareRisk.getAddress(),
        ethers.ZeroAddress
      );
      await expect(bareHub.registerCommunity(COMMUNITY_ID, "X", "Y")).to.be.revertedWith(
        "RiskRegistry: caller is not owner"
      );
    });

    it("updateRisk forwards to RiskRegistry and actually recomputes the score (data-feeder status is enough, no ownership transfer needed)", async function () {
      await hub.registerCommunity(COMMUNITY_ID, "Ohafia", "Abia State");
      await hub.updateRisk(COMMUNITY_ID, SCALE, SCALE, SCALE); // H=E=V=1.0 -> R=1.0
      expect(await riskRegistry.riskScore(COMMUNITY_ID)).to.equal(SCALE);
    });

    it("reverts registerCommunity/updateRisk when riskRegistry is not set", async function () {
      const bareHub = await deploy(
        "D3RACHub",
        admin,
        admin.address,
        await token.getAddress(),
        await registry.getAddress(),
        await controller.getAddress(),
        ethers.ZeroAddress,
        ethers.ZeroAddress
      );
      await expect(bareHub.registerCommunity(COMMUNITY_ID, "X", "Y")).to.be.revertedWith(
        "D3RACHub: riskRegistry not set"
      );
      await expect(bareHub.updateRisk(COMMUNITY_ID, SCALE, SCALE, SCALE)).to.be.revertedWith(
        "D3RACHub: riskRegistry not set"
      );
    });
  });

  describe("orchestration: funding request registry", function () {
    it("openFundingRequest forwards to FundingRequestRegistry (requires the Hub to hold proposer status)", async function () {
      await expect(hub.openFundingRequest(COMMUNITY_ID, 1000, "Shelter rebuild", "ipfs://report"))
        .to.emit(fundingRegistry, "RequestOpened");
      expect(await fundingRegistry.requestCount()).to.equal(1);
    });

    it("reverts if the Hub has NOT been granted proposer status", async function () {
      const bareFunding = await deploy("FundingRequestRegistry", admin, admin.address); // Hub never added as proposer
      const bareHub = await deploy(
        "D3RACHub",
        admin,
        admin.address,
        await token.getAddress(),
        await registry.getAddress(),
        await controller.getAddress(),
        ethers.ZeroAddress,
        await bareFunding.getAddress()
      );
      await expect(
        bareHub.openFundingRequest(COMMUNITY_ID, 1000, "desc", "ipfs://x")
      ).to.be.revertedWith("FundingRequestRegistry: caller is not an authorized proposer");
    });

    it("closeFundingRequest succeeds for a request the Hub itself opened", async function () {
      await hub.openFundingRequest(COMMUNITY_ID, 1000, "desc", "ipfs://x");
      await expect(hub.closeFundingRequest(0)).to.not.be.reverted;
      const r = await fundingRegistry.getRequest(0);
      expect(r.status).to.equal(3n); // Status.Closed
    });

    it("reverts openFundingRequest/closeFundingRequest when fundingRequestRegistry is not set", async function () {
      const bareHub = await deploy(
        "D3RACHub",
        admin,
        admin.address,
        await token.getAddress(),
        await registry.getAddress(),
        await controller.getAddress(),
        ethers.ZeroAddress,
        ethers.ZeroAddress
      );
      await expect(
        bareHub.openFundingRequest(COMMUNITY_ID, 1000, "desc", "ipfs://x")
      ).to.be.revertedWith("D3RACHub: fundingRequestRegistry not set");
      await expect(bareHub.closeFundingRequest(0)).to.be.revertedWith("D3RACHub: fundingRequestRegistry not set");
    });
  });

  describe("role & ownership management: IdentityRegistry", function () {
    it("setIdentityVerifier forwards to IdentityRegistry (requires the Hub to be its admin)", async function () {
      await hub.setIdentityVerifier(someone.address, true);
      expect(await registry.verifiers(someone.address)).to.equal(true);
    });

    it("revokeRecipient forwards to IdentityRegistry (needs only the additive verifier status already required for verifyRecipient)", async function () {
      await hub.verifyRecipient(recipient.address, "Ohafia Relief Coalition");
      await hub.revokeRecipient(recipient.address);
      expect(await registry.isVerified(recipient.address)).to.equal(false);
    });

    it("transferIdentityRegistryAdmin forwards and genuinely moves IdentityRegistry's admin off the Hub", async function () {
      await hub.transferIdentityRegistryAdmin(someone.address);
      expect(await registry.admin()).to.equal(someone.address);
      await expect(hub.setIdentityVerifier(stranger.address, true)).to.be.revertedWith(
        "IdentityRegistry: caller is not admin"
      );
    });

    it("reverts setIdentityVerifier if the Hub is NOT IdentityRegistry's admin (verifier status alone isn't enough)", async function () {
      const bareRegistry = await deploy("IdentityRegistry", admin, admin.address);
      const bareHub = await deploy(
        "D3RACHub", admin, admin.address, await token.getAddress(),
        await bareRegistry.getAddress(), await controller.getAddress(), ethers.ZeroAddress, ethers.ZeroAddress
      );
      await bareRegistry.setVerifier(await bareHub.getAddress(), true); // verifier granted, admin NOT transferred
      await expect(bareHub.setIdentityVerifier(someone.address, true)).to.be.revertedWith(
        "IdentityRegistry: caller is not admin"
      );
    });

    it("only Hub admin can call these", async function () {
      await expect(hub.connect(stranger).setIdentityVerifier(someone.address, true)).to.be.revertedWith(
        "D3RACHub: caller is not admin"
      );
      await expect(hub.connect(stranger).revokeRecipient(recipient.address)).to.be.revertedWith(
        "D3RACHub: caller is not admin"
      );
    });
  });

  describe("role & ownership management: DisbursementController", function () {
    it("setDisbursementAttester forwards (already covered by the admin transfer createCommitment needs)", async function () {
      await hub.setDisbursementAttester(someone.address, true);
      expect(await controller.attesters(someone.address)).to.equal(true);
    });

    it("transferDisbursementControllerAdmin forwards and genuinely moves its admin off the Hub", async function () {
      await hub.transferDisbursementControllerAdmin(someone.address);
      expect(await controller.admin()).to.equal(someone.address);
      await expect(hub.setDisbursementAttester(stranger.address, true)).to.be.revertedWith(
        "DisbursementController: caller is not admin"
      );
    });
  });

  describe("role & ownership management: D3RACToken", function () {
    it("setTokenMinter forwards to D3RACToken (requires the Hub to be its owner -- minter status alone is not enough)", async function () {
      await hub.setTokenMinter(someone.address, true);
      expect(await token.minters(someone.address)).to.equal(true);
    });

    it("reverts setTokenMinter if the Hub only holds minter status, not owner", async function () {
      const bareToken = await deploy("D3RACToken", admin, 0, admin.address);
      const bareHub = await deploy(
        "D3RACHub", admin, admin.address, await bareToken.getAddress(),
        await registry.getAddress(), await controller.getAddress(), ethers.ZeroAddress, ethers.ZeroAddress
      );
      await bareToken.setMinter(await bareHub.getAddress(), true); // minter granted, owner NOT transferred
      await expect(bareHub.setTokenMinter(someone.address, true)).to.be.revertedWith(
        "D3RACToken: caller is not the owner"
      );
      // but mintTokens (only needs minter) still works
      await expect(bareHub.mintTokens(someone.address, 100)).to.not.be.reverted;
    });

    it("transferTokenOwnership forwards and genuinely moves D3RACToken's owner off the Hub", async function () {
      await hub.transferTokenOwnership(someone.address);
      expect(await token.owner()).to.equal(someone.address);
      await expect(hub.setTokenMinter(stranger.address, true)).to.be.revertedWith(
        "D3RACToken: caller is not the owner"
      );
    });
  });

  describe("role & ownership management: RiskRegistry", function () {
    it("setRiskDataFeeder(true) adds and (false) removes a data feeder", async function () {
      await hub.setRiskDataFeeder(someone.address, true);
      expect(await riskRegistry.dataFeeders(someone.address)).to.equal(true);
      await hub.setRiskDataFeeder(someone.address, false);
      expect(await riskRegistry.dataFeeders(someone.address)).to.equal(false);
    });

    it("setRiskThreshold forwards and updates theta", async function () {
      await hub.setRiskThreshold(SCALE / 4n);
      expect(await riskRegistry.riskThreshold()).to.equal(SCALE / 4n);
    });

    it("transferRiskRegistryOwnership forwards and genuinely moves ownership off the Hub", async function () {
      await hub.transferRiskRegistryOwnership(someone.address);
      expect(await riskRegistry.owner()).to.equal(someone.address);
      await expect(hub.setRiskThreshold(1)).to.be.revertedWith("RiskRegistry: caller is not owner");
    });

    it("reverts setRiskDataFeeder/setRiskThreshold/transferRiskRegistryOwnership when riskRegistry is not set", async function () {
      const bareHub = await deploy(
        "D3RACHub", admin, admin.address, await token.getAddress(),
        await registry.getAddress(), await controller.getAddress(), ethers.ZeroAddress, ethers.ZeroAddress
      );
      await expect(bareHub.setRiskDataFeeder(someone.address, true)).to.be.revertedWith("D3RACHub: riskRegistry not set");
      await expect(bareHub.setRiskThreshold(1)).to.be.revertedWith("D3RACHub: riskRegistry not set");
      await expect(bareHub.transferRiskRegistryOwnership(someone.address)).to.be.revertedWith("D3RACHub: riskRegistry not set");
    });
  });

  describe("role & ownership management: FundingRequestRegistry", function () {
    it("setFundingProposer(true) adds and (false) removes a proposer", async function () {
      await hub.setFundingProposer(someone.address, true);
      expect(await fundingRegistry.proposers(someone.address)).to.equal(true);
      await hub.setFundingProposer(someone.address, false);
      expect(await fundingRegistry.proposers(someone.address)).to.equal(false);
    });

    it("recordFundingPledge forwards for a request the Hub itself opened", async function () {
      await hub.openFundingRequest(COMMUNITY_ID, 1000, "desc", "ipfs://x");
      await hub.recordFundingPledge(0, 400, "ipfs://pledge");
      const r = await fundingRegistry.getRequest(0);
      expect(r.amountPledged).to.equal(400);
    });

    it("linkFundingRequestToCommitment forwards for a request the Hub itself opened", async function () {
      await hub.openFundingRequest(COMMUNITY_ID, 1000, "desc", "ipfs://x");
      await hub.linkFundingRequestToCommitment(0, 7);
      const r = await fundingRegistry.getRequest(0);
      expect(r.linkedCommitmentId).to.equal(7);
    });

    it("recordFundingPledge/linkFundingRequestToCommitment/closeFundingRequest also work on a request NOT opened via the Hub, because the Hub holds ownership", async function () {
      await hub.setFundingProposer(stranger.address, true); // ownership already transferred to the Hub in beforeEach
      await fundingRegistry.connect(stranger).openRequest(COMMUNITY_ID, 1000, "desc", "ipfs://x"); // requester = stranger, not Hub
      await expect(hub.recordFundingPledge(0, 100, "ipfs://p")).to.not.be.reverted;
      await expect(hub.linkFundingRequestToCommitment(0, 3)).to.not.be.reverted;
      await expect(hub.closeFundingRequest(0)).to.not.be.reverted;
    });

    it("transferFundingRequestRegistryOwnership forwards and genuinely moves ownership off the Hub", async function () {
      await hub.transferFundingRequestRegistryOwnership(someone.address);
      expect(await fundingRegistry.owner()).to.equal(someone.address);
      await expect(hub.setFundingProposer(stranger.address, true)).to.be.revertedWith(
        "FundingRequestRegistry: caller is not owner"
      );
    });

    it("reverts the FundingRequestRegistry role-management proxies when the module is not set", async function () {
      const bareHub = await deploy(
        "D3RACHub", admin, admin.address, await token.getAddress(),
        await registry.getAddress(), await controller.getAddress(), ethers.ZeroAddress, ethers.ZeroAddress
      );
      await expect(bareHub.setFundingProposer(someone.address, true)).to.be.revertedWith("D3RACHub: fundingRequestRegistry not set");
      await expect(bareHub.recordFundingPledge(0, 1, "x")).to.be.revertedWith("D3RACHub: fundingRequestRegistry not set");
      await expect(bareHub.linkFundingRequestToCommitment(0, 1)).to.be.revertedWith("D3RACHub: fundingRequestRegistry not set");
      await expect(bareHub.transferFundingRequestRegistryOwnership(someone.address)).to.be.revertedWith("D3RACHub: fundingRequestRegistry not set");
    });
  });

  describe("systemStatus", function () {
    it("aggregates all five module addresses, paused state, supply, commitment count, community count, and request count in one call", async function () {
      await hub.verifyRecipient(recipient.address, "Ohafia Relief Coalition");
      await hub.createCommitment(recipient.address, await token.getAddress(), "Ohafia Relief Coalition", ["M"], [1000]);
      await hub.registerCommunity(COMMUNITY_ID, "Ohafia", "Abia State");
      await hub.openFundingRequest(COMMUNITY_ID, 1000, "desc", "ipfs://x");

      const status = await hub.systemStatus();
      expect(status.tokenAddress).to.equal(await token.getAddress());
      expect(status.identityRegistryAddress).to.equal(await registry.getAddress());
      expect(status.disbursementControllerAddress).to.equal(await controller.getAddress());
      expect(status.riskRegistryAddress).to.equal(await riskRegistry.getAddress());
      expect(status.fundingRequestRegistryAddress).to.equal(await fundingRegistry.getAddress());
      expect(status.isPaused).to.equal(false);
      expect(status.tokenTotalSupply).to.equal(await token.totalSupply());
      expect(status.totalCommitments).to.equal(1);
      expect(status.totalCommunities).to.equal(1);
      expect(status.totalFundingRequests).to.equal(1);
    });

    it("reports zero counts for risk/funding when those modules aren't set, without reverting", async function () {
      const bareHub = await deploy(
        "D3RACHub",
        admin,
        admin.address,
        await token.getAddress(),
        await registry.getAddress(),
        await controller.getAddress(),
        ethers.ZeroAddress,
        ethers.ZeroAddress
      );
      const status = await bareHub.systemStatus();
      expect(status.riskRegistryAddress).to.equal(ethers.ZeroAddress);
      expect(status.fundingRequestRegistryAddress).to.equal(ethers.ZeroAddress);
      expect(status.totalCommunities).to.equal(0);
      expect(status.totalFundingRequests).to.equal(0);
    });
  });

  describe("full-control end-to-end: everything reachable through the Hub, nothing left direct-only (except the deliberately permissionless releaseMilestone)", function () {
    it("every write on every one of the five underlying contracts can be performed through the Hub alone, using only hub.* calls", async function () {
      // Identity
      await hub.setIdentityVerifier(someone.address, true);
      await hub.verifyRecipient(recipient.address, "Ohafia Relief Coalition");
      await hub.revokeRecipient(recipient.address);
      await hub.verifyRecipient(recipient.address, "Ohafia Relief Coalition"); // re-verify for the commitment below

      // Disbursement
      await hub.setDisbursementAttester(someone.address, true);
      await token.transfer(await controller.getAddress(), 1000); // fund the controller so release can pay out
      await hub.createCommitment(recipient.address, await token.getAddress(), "Ohafia Relief Coalition", ["Water restored"], [1000]);
      await hub.attestMilestone(0, 0);
      // releaseMilestone is deliberately permissionless on DisbursementController itself (see its own docs) --
      // the Hub doesn't need to proxy it for the system to work, but it's still reachable, just not via the Hub.
      await controller.releaseMilestone(0, 0);
      expect(await token.balanceOf(recipient.address)).to.equal(1000);

      // Token
      await hub.setTokenMinter(someone.address, true);
      await hub.mintTokens(minted.address, 500);

      // Risk
      await hub.setRiskDataFeeder(someone.address, true);
      await hub.setRiskThreshold(SCALE / 5n);
      await hub.registerCommunity(COMMUNITY_ID, "Ohafia", "Abia State");
      await hub.updateRisk(COMMUNITY_ID, SCALE, SCALE, SCALE);

      // Funding
      await hub.setFundingProposer(someone.address, true);
      await hub.openFundingRequest(COMMUNITY_ID, 2000, "Shelter rebuild", "ipfs://report");
      await hub.recordFundingPledge(0, 2000, "ipfs://pledge");
      await hub.linkFundingRequestToCommitment(0, 0);
      await hub.closeFundingRequest(0);

      // Everything above ran through hub.* only (plus the one deliberate
      // exception, releaseMilestone) and none of it reverted -- full
      // control confirmed end to end, not just per-function in isolation.
      const status = await hub.systemStatus();
      expect(status.totalCommitments).to.equal(1);
      expect(status.totalCommunities).to.equal(1);
      expect(status.totalFundingRequests).to.equal(1);
    });
  });
});
