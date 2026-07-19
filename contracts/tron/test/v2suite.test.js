// TronBox/TVM-native port of the executed Hardhat test suites in
// hardhat-tests/test/{RiskRegistry,FundingRequestRegistry,AttestorMultisig}.test.js.
// Run this against a real TVM node (TRE via Docker, or Shasta/Nile) before
// trusting this contract suite with real funds — it was NOT executed
// against a live node in the environment this code was built in, only the
// Hardhat-side suite was (see hardhat-tests/README.md for what that does
// and doesn't prove).
//
//   tronbox test --network development

const IdentityRegistry = artifacts.require("IdentityRegistry");
const D3RACToken = artifacts.require("D3RACToken");
const DisbursementController = artifacts.require("DisbursementController");
const RiskRegistry = artifacts.require("RiskRegistry");
const FundingRequestRegistry = artifacts.require("FundingRequestRegistry");
const AttestorMultisig = artifacts.require("AttestorMultisig");

contract("D3R\u00b7AC v2 suite", (accounts) => {
  const [admin, feeder, proposer, signer1, signer2, recipient, stranger] = accounts;
  const c3 = "0x" + Buffer.from("c3").toString("hex").padEnd(64, "0");

  describe("RiskRegistry", () => {
    let registry;
    beforeEach(async () => {
      registry = await RiskRegistry.new("350000000000000000", feeder, { from: admin }); // theta = 0.35
      await registry.registerCommunity(c3, "Maiduguri Corridor", "Borno, NG", { from: admin });
    });

    it("emits ThresholdCrossed once R(c,t) exceeds theta", async () => {
      const tx = await registry.updateRisk(
        c3,
        "810000000000000000", // 0.81
        "660000000000000000", // 0.66
        "740000000000000000", // 0.74
        { from: feeder }
      );
      const crossed = tx.logs.some((l) => l.event === "ThresholdCrossed");
      assert.isTrue(crossed);
    });

    it("blocks non-feeders from updating risk", async () => {
      try {
        await registry.updateRisk(c3, "1", "1", "1", { from: stranger });
        assert.fail("expected revert");
      } catch (err) {
        assert.include(err.message, "not a data feeder");
      }
    });
  });

  describe("FundingRequestRegistry", () => {
    let registry;
    beforeEach(async () => {
      registry = await FundingRequestRegistry.new(proposer, { from: admin });
    });

    it("opens a request and tracks pledge status transitions", async () => {
      await registry.openRequest(c3, "500000000", "Shelter kits", "ipfs://example", { from: proposer });
      await registry.recordPledge(0, "500000000", "donor-ref", { from: proposer });
      const r = await registry.getRequest(0);
      assert.equal(r.status.toString(), "2"); // Funded
    });
  });

  describe("AttestorMultisig integration with the real DisbursementController", () => {
    let identityRegistry, token, controller, multisig;

    beforeEach(async () => {
      identityRegistry = await IdentityRegistry.new(admin, { from: admin });
      await identityRegistry.verifyRecipient(recipient, "Maiduguri Corridor", { from: admin });

      token = await D3RACToken.new(0, admin, { from: admin });
      controller = await DisbursementController.new(identityRegistry.address, admin, { from: admin });
      multisig = await AttestorMultisig.new(controller.address, [signer1, signer2], 2, { from: admin });

      await controller.setAttester(multisig.address, true, { from: admin });
      await controller.setAttester(admin, false, { from: admin });

      await controller.createCommitment(recipient, token.address, "Maiduguri Corridor", ["phase 1"], ["100000000"], {
        from: admin,
      });
      await token.mint(controller.address, "100000000", { from: admin });
    });

    it("attests on DisbursementController only after 2-of-2 approval, then release succeeds", async () => {
      await multisig.proposeAttestation(0, 0, { from: signer1 });
      let m = await controller.getMilestone(0, 0);
      assert.isFalse(m.attested);

      await multisig.approveAttestation(0, 0, { from: signer2 });
      m = await controller.getMilestone(0, 0);
      assert.isTrue(m.attested);

      await controller.releaseMilestone(0, 0, { from: admin });
      const bal = await token.balanceOf(recipient);
      assert.equal(bal.toString(), "100000000");
    });

    it("confirms the old single-EOA attester (admin) was actually revoked", async () => {
      try {
        await controller.attestMilestone(0, 0, { from: admin });
        assert.fail("expected revert");
      } catch (err) {
        assert.include(err.message, "not an attester");
      }
    });
  });
});
