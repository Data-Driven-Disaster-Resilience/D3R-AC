// SPDX-License-Identifier: MIT
pragma solidity 0.8.20;

import "./D3RACToken.sol";

/// @title D3RACTokenFactory
/// @notice Deploys `D3RACToken` instances via `CREATE2`, so the
///         resulting token address can be pre-computed off-chain and
///         mined for a vanity prefix/suffix before the actual deploy
///         transaction is sent (see `scripts/find-vanity-create2-salt.js`).
/// @dev Purely additive: nothing else in this suite depends on tokens
///      being deployed through this factory. `D3RACHub`/
///      `DisbursementController` accept any ERC-20/TRC-20-shaped token
///      address regardless of how it was deployed.
///
///      TVM CREATE2 ADDRESS DERIVATION -- the one genuinely
///      TRON-specific detail in this whole file, verified against
///      TRON's own documentation/audit checklists rather than assumed
///      from EVM habit: EVM's CREATE2 address formula uses a `0xff`
///      leading byte
///      (`address(keccak256(0xff ++ deployer ++ salt ++ initCodeHash)[12:])`),
///      but the TVM's equivalent uses `0x41` instead (TRON's own address
///      version-prefix byte). Getting this wrong wouldn't fail loudly --
///      it would silently predict the WRONG address off-chain while the
///      real on-chain deploy still succeeds at a different address than
///      expected, which is exactly the kind of mismatch a vanity-mining
///      script needs to get right or it mines salts for addresses that
///      are never actually produced.
contract D3RACTokenFactory {
    event TokenDeployed(address indexed token, address indexed owner, bytes32 salt);

    /// @notice Deploys a new `D3RACToken` at a `CREATE2`-derived address.
    /// @param initialSupply Initial supply minted to `owner_` at deploy time.
    /// @param owner_ The new token's owner (see `D3RACToken`'s own owner-gated `mint`/`setMinter`).
    /// @param salt Caller-chosen salt -- mine this off-chain with
    ///        `scripts/find-vanity-create2-salt.js` for a vanity address.
    function deployToken(uint256 initialSupply, address owner_, bytes32 salt) external returns (address token) {
        bytes memory bytecode = abi.encodePacked(
            type(D3RACToken).creationCode,
            abi.encode(initialSupply, owner_)
        );
        assembly {
            token := create2(0, add(bytecode, 0x20), mload(bytecode), salt)
        }
        require(token != address(0), "D3RACTokenFactory: CREATE2 deployment failed");
        emit TokenDeployed(token, owner_, salt);
    }

    /// @notice Precomputes the address `deployToken` would produce for a
    ///         given salt/args, without deploying anything -- matches
    ///         the derivation `scripts/find-vanity-create2-salt.js` uses
    ///         off-chain, so mined salts and on-chain results agree.
    function computeAddress(uint256 initialSupply, address owner_, bytes32 salt) external view returns (address predicted) {
        bytes memory bytecode = abi.encodePacked(
            type(D3RACToken).creationCode,
            abi.encode(initialSupply, owner_)
        );
        bytes32 bytecodeHash = keccak256(bytecode);
        // TVM CREATE2: 0x41 prefix, not EVM's 0xff -- see this contract's
        // own top-of-file doc comment for why that distinction is real
        // and not a typo.
        bytes32 hash = keccak256(abi.encodePacked(bytes1(0x41), address(this), salt, bytecodeHash));
        predicted = address(uint160(uint256(hash)));
    }
}
