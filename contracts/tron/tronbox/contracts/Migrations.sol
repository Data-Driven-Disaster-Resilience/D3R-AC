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
    address public immutable owner;
    // NOTE: `last_completed_migration` deliberately keeps TronBox/Truffle's
    // own snake_case convention rather than "fixing" it to mixedCase --
    // this is boilerplate scaffolding TronBox itself generates and reads
    // by this exact name; renaming it here would just be cosmetic and
    // risks silently breaking `tronbox migrate`'s own convention-matching.
    // Slither's naming-convention finding on this line is reviewed and
    // intentionally not applied.
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
