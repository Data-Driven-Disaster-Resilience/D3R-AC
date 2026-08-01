// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title Migrations
/// @notice Standard TronBox/Truffle-style migration-tracking contract.
///         This is NOT part of the D3R-AC system (D3RACHub, D3RACToken,
///         etc.) -- TronBox deploys it automatically via
///         `migrations/1_initial_migration.js` and uses it internally to
///         record which numbered migration script has already run, so
///         `tronbox migrate` doesn't re-run completed migrations on a
///         second invocation against the same network.
contract Migrations {
    address public owner;
    uint256 public last_completed_migration;

    modifier restricted() {
        require(msg.sender == owner, "Migrations: caller is not owner");
        _;
    }

    constructor() {
        owner = msg.sender;
    }

    function setCompleted(uint256 completed) external restricted {
        last_completed_migration = completed;
    }
}
