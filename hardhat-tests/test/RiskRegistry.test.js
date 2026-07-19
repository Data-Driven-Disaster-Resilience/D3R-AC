const { expect } = require("chai");
const { ethers } = require("hardhat");
const fs = require("fs");
const path = require("path");

function loadArtifact(name) {
  return JSON.parse(fs.readFileSync(path.join(__dirname, "..", "artifacts-raw", `${name}.json`), "utf8"));
}

const SCALE = 10n ** 18n;
const bps = (n) => (BigInt(Math.round(n * 1000)) * SCALE) / 1000n; // e.g. bps(0.72) -> 0.72 * 1e18

describe("RiskRegistry", function () {
  let registry, owner, feeder, stranger;
  const c3 = ethers.encodeBytes32String("c3");

  beforeEach(async function () {
    [owner, feeder, stranger] = await ethers.getSigners();
    const artifact = loadArtifact("RiskRegistry");
    const Factory = new ethers.ContractFactory(artifact.abi, artifact.bytecode, owner);
    // threshold theta = 0.35, matching docs/risk-model.md / riskModel.ts's RISK_THRESHOLD
    registry = await Factory.deploy(bps(0.35), feeder.address);
    await registry.waitForDeployment();

    await registry.connect(owner).registerCommunity(c3, "Maiduguri Corridor", "Borno, NG");
  });

  it("registers a community", async function () {
    const c = await registry.getCommunity(c3);
    expect(c.name_).to.equal("Maiduguri Corridor");
    expect(c.region).to.equal("Borno, NG");
  });

  it("rejects double registration of the same community", async function () {
    await expect(
      registry.connect(owner).registerCommunity(c3, "dup", "dup")
    ).to.be.revertedWith("RiskRegistry: community already registered");
  });

  it("computes R(c,t) = H*E*V correctly, matching riskModel.ts's Maiduguri Corridor figures", async function () {
    // hazard 0.81, exposure 0.66, vulnerability 0.74 -> R ~= 0.395604
    const h = bps(0.81);
    const e = bps(0.66);
    const v = bps(0.74);
    await registry.connect(feeder).updateRisk(c3, h, e, v);
    const score = await registry.riskScore(c3);
    // Same exact fixed-point arithmetic the contract itself performs,
    // rather than re-deriving via the lossy bps() helper (which only
    // keeps 3 decimal digits of precision and would introduce a rounding
    // mismatch bigger than a reasonable tolerance).
    const expected = ((h * e) / SCALE) * v / SCALE;
    expect(score).to.equal(expected);
    // Sanity-check it's in the right ballpark vs. the human-readable formula
    expect(score).to.be.closeTo(bps(0.3956), SCALE / 1000n);
  });

  it("blocks non-feeders from updating risk", async function () {
    await expect(
      registry.connect(stranger).updateRisk(c3, bps(0.5), bps(0.5), bps(0.5))
    ).to.be.revertedWith("RiskRegistry: caller is not a data feeder");
  });

  it("rejects out-of-range [0,1] values", async function () {
    await expect(
      registry.connect(feeder).updateRisk(c3, SCALE + 1n, bps(0.5), bps(0.5))
    ).to.be.revertedWith("RiskRegistry: value out of [0,1] range");
  });

  it("emits ThresholdCrossed only when R crosses theta", async function () {
    // Below threshold: 0.3 * 0.3 * 0.3 = 0.027, well under 0.35
    const txLow = await registry.connect(feeder).updateRisk(c3, bps(0.3), bps(0.3), bps(0.3));
    const receiptLow = await txLow.wait();
    const crossedLow = receiptLow.logs.some((l) => {
      try {
        return registry.interface.parseLog(l)?.name === "ThresholdCrossed";
      } catch {
        return false;
      }
    });
    expect(crossedLow).to.equal(false);

    // Above threshold: 0.81 * 0.66 * 0.74 ~= 0.3956 > 0.35
    const txHigh = await registry.connect(feeder).updateRisk(c3, bps(0.81), bps(0.66), bps(0.74));
    await expect(txHigh).to.emit(registry, "ThresholdCrossed");
  });

  it("isAboveThreshold reflects the current score correctly", async function () {
    await registry.connect(feeder).updateRisk(c3, bps(0.81), bps(0.66), bps(0.74));
    expect(await registry.isAboveThreshold(c3)).to.equal(true);

    await registry.connect(feeder).updateRisk(c3, bps(0.2), bps(0.2), bps(0.2));
    expect(await registry.isAboveThreshold(c3)).to.equal(false);
  });

  it("only owner can add/remove data feeders or change threshold", async function () {
    await expect(registry.connect(stranger).addDataFeeder(stranger.address)).to.be.revertedWith(
      "RiskRegistry: caller is not owner"
    );
    await registry.connect(owner).addDataFeeder(stranger.address);
    expect(await registry.dataFeeders(stranger.address)).to.equal(true);

    await expect(registry.connect(stranger).setRiskThreshold(bps(0.5))).to.be.revertedWith(
      "RiskRegistry: caller is not owner"
    );
    await registry.connect(owner).setRiskThreshold(bps(0.5));
    expect(await registry.riskThreshold()).to.equal(bps(0.5));
  });
});
