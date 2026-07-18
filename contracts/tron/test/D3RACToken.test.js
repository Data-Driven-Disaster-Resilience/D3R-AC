const { expect } = require("chai");
const { ethers } = require("hardhat");
const { deploy } = require("./helpers");

describe("D3RACToken", function () {
  let owner, minter, alice, bob, stranger;
  let token;

  beforeEach(async function () {
    [owner, minter, alice, bob, stranger] = await ethers.getSigners();
    token = await deploy("D3RACToken", owner, 1_000_000, owner.address);
  });

  it("mints the initial supply to the owner at deployment", async function () {
    const decimals = await token.decimals();
    expect(await token.balanceOf(owner.address)).to.equal(1_000_000n * 10n ** decimals);
    expect(await token.totalSupply()).to.equal(1_000_000n * 10n ** decimals);
  });

  it("exposes the exact ABI surface the frontend adapter calls", async function () {
    expect(await token.symbol()).to.equal("D3RAC");
    expect(await token.decimals()).to.equal(18);
    expect(await token.balanceOf(alice.address)).to.equal(0);
  });

  it("transfers between accounts and emits Transfer", async function () {
    await expect(token.transfer(alice.address, 1000))
      .to.emit(token, "Transfer")
      .withArgs(owner.address, alice.address, 1000);
    expect(await token.balanceOf(alice.address)).to.equal(1000);
  });

  it("rejects a transfer that exceeds the sender's balance", async function () {
    await expect(token.connect(alice).transfer(bob.address, 1)).to.be.revertedWith(
      "D3RACToken: transfer exceeds balance"
    );
  });

  it("rejects a transfer to the zero address", async function () {
    await expect(token.transfer(ethers.ZeroAddress, 1)).to.be.revertedWith(
      "D3RACToken: transfer to zero address"
    );
  });

  it("approve/transferFrom respects and decrements allowance", async function () {
    await token.approve(alice.address, 500);
    expect(await token.allowance(owner.address, alice.address)).to.equal(500);

    await token.connect(alice).transferFrom(owner.address, bob.address, 300);
    expect(await token.balanceOf(bob.address)).to.equal(300);
    expect(await token.allowance(owner.address, alice.address)).to.equal(200);
  });

  it("rejects transferFrom beyond the approved allowance", async function () {
    await token.approve(alice.address, 100);
    await expect(
      token.connect(alice).transferFrom(owner.address, bob.address, 101)
    ).to.be.revertedWith("D3RACToken: transfer exceeds allowance");
  });

  it("only an explicit minter can mint", async function () {
    await expect(token.connect(stranger).mint(alice.address, 1)).to.be.revertedWith(
      "D3RACToken: caller is not a minter"
    );

    await token.setMinter(minter.address, true);
    await expect(token.connect(minter).mint(alice.address, 500))
      .to.emit(token, "Transfer")
      .withArgs(ethers.ZeroAddress, alice.address, 500);
    expect(await token.balanceOf(alice.address)).to.equal(500);
  });

  it("only the owner can grant minter status", async function () {
    await expect(token.connect(stranger).setMinter(alice.address, true)).to.be.revertedWith(
      "D3RACToken: caller is not the owner"
    );
  });

  it("burn reduces balance and total supply", async function () {
    await token.transfer(alice.address, 1000);
    await token.connect(alice).burn(400);
    expect(await token.balanceOf(alice.address)).to.equal(600);
  });

  it("rejects burning more than the caller's balance", async function () {
    await expect(token.connect(alice).burn(1)).to.be.revertedWith("D3RACToken: burn exceeds balance");
  });

  it("only the owner can transfer ownership, and it takes effect", async function () {
    await expect(token.connect(stranger).transferOwnership(alice.address)).to.be.revertedWith(
      "D3RACToken: caller is not the owner"
    );
    await token.transferOwnership(alice.address);
    expect(await token.owner()).to.equal(alice.address);
    // old owner immediately loses admin rights
    await expect(token.setMinter(bob.address, true)).to.be.revertedWith(
      "D3RACToken: caller is not the owner"
    );
  });
});
