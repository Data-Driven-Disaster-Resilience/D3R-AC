const { expect } = require("chai");
const { ethers } = require("hardhat");
const { deploy } = require("./helpers");

const SCALE = 10n ** 18n;
const bps = (n) => (BigInt(Math.round(n * 1000)) * SCALE) / 1000n; // e.g. bps(0.72) -> 0.72 * 1e18

describe("RiskRegistry", function () {
  let registry, owner, feeder, stranger;
  const c3 = ethers.encodeBytes32String("c3");

  beforeEach(async function () {
    [owner, feeder, stranger] = await ethers.getSigners();
    // theta = 0.35, matching docs/risk-model.md / riskModel.ts's threshold
    registry = await deploy("RiskRegistry", owner, bps(0.35), feeder.address);
    await registry.registerCommunity(c3, "Maiduguri Corridor", "Borno, NG");
  });

  it("registers a community", async function () {
    const c = await registry.getCommunity(c3);
    expect(c.name_).to.equal("Maiduguri Corridor");
    expect(c.region).to.equal("Borno, NG");
  });

  it("rejects double registration of the same community", async function () {
    await expect(registry.registerCommunity(c3, "dup", "dup")).to.be.revertedWith(
      "RiskRegistry: community already registered"
    );
  });

  it("computes R(c,t) = H*E*V using the exact fixed-point arithmetic the contract performs", async function () {
    const h = bps(0.81);
    const e = bps(0.66);
    const v = bps(0.74);
    await registry.connect(feeder).updateRisk(c3, h, e, v);
    const score = await registry.riskScore(c3);
    const expected = ((h * e) / SCALE) * v / SCALE;
    expect(score).to.equal(expected);
    expect(score).to.be.closeTo(bps(0.3956), SCALE / 1000n); // ~0.395604, matches the docs example
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
    const txLow = await registry.connect(feeder).updateRisk(c3, bps(0.3), bps(0.3), bps(0.3)); // 0.027, below theta
    await expect(txLow).to.not.emit(registry, "ThresholdCrossed");

    const txHigh = await registry.connect(feeder).updateRisk(c3, bps(0.81), bps(0.66), bps(0.74)); // ~0.3956, above theta
    await expect(txHigh).to.emit(registry, "ThresholdCrossed");
  });

  it("isAboveThreshold reflects the current score correctly", async function () {
    await registry.connect(feeder).updateRisk(c3, bps(0.81), bps(0.66), bps(0.74));
    expect(await registry.isAboveThreshold(c3)).to.equal(true);
    await registry.connect(feeder).updateRisk(c3, bps(0.2), bps(0.2), bps(0.2));
    expect(await registry.isAboveThreshold(c3)).to.equal(false);
  });

  it("only owner can add/remove data feeders or change the threshold", async function () {
    await expect(registry.connect(stranger).addDataFeeder(stranger.address)).to.be.revertedWith(
      "RiskRegistry: caller is not owner"
    );
    await registry.addDataFeeder(stranger.address);
    expect(await registry.dataFeeders(stranger.address)).to.equal(true);

    await expect(registry.connect(stranger).setRiskThreshold(bps(0.5))).to.be.revertedWith(
      "RiskRegistry: caller is not owner"
    );
    await registry.setRiskThreshold(bps(0.5));
    expect(await registry.riskThreshold()).to.equal(bps(0.5));
  });
});
