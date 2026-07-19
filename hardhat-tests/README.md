# Why a Hardhat test harness in a TRON project?

The contracts in `../contracts/tron/` deploy and run on TRON (TVM), via
TronBox — see `../contracts/tron/tronbox-config.js` and
`../contracts/tron/test/v2suite.test.js` for the real deployment/testing
path.

This directory exists because **running TronBox's own tests requires a
live TVM node** (TRE via Docker, or a Shasta/Nile testnet connection) —
something not available in the environment these contracts were first
built and verified in. TVM is Solidity/EVM-compatible for everything
these contracts actually use (no TRON-specific precompiles, no
energy/bandwidth-dependent logic) — so Hardhat's in-memory EVM is a valid
way to execute and verify contract *logic* (access control, ordering,
fixed-point math, threshold voting) before you ever touch a TRON node.

**What this proves:** 25 tests all pass, covering `RiskRegistry`,
`FundingRequestRegistry`, and `AttestorMultisig` — including a full
integration test of `AttestorMultisig` calling the *real*
`DisbursementController` contract (not a mock), verifying that 2-of-3
threshold approval correctly gates milestone attestation and that the
old single-EOA attester is genuinely revoked once the multisig takes
over.

**What this does NOT prove:** anything TRON-specific — actual gas/energy
costs, TronLink transaction signing, TRC-20 tokens with non-standard
`transfer` return values. Run the TronBox test suite
(`../contracts/tron/test/v2suite.test.js`) against a real TVM node before
any testnet deployment; treat this Hardhat suite as a fast pre-check, not
a substitute.

## Running this

```bash
npm install
npm run compile   # compiles the full contract set with solc 0.8.20
npm test          # runs the 25 tests against Hardhat's in-memory EVM
```

`artifacts-raw/` contains the pre-compiled ABI + bytecode used by the
tests directly (bypassing Hardhat's own compiler download step, which
needs network access to `binaries.soliditylang.org` — not available in
every sandboxed environment). If you have normal internet access, you can
ignore this and just run `npx hardhat test` after `npx hardhat compile`
the usual way.
