// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

// Standard TronBox/Truffle-style migrations tracker. Required scaffolding
// for `tronbox migrate` to track which migration scripts have already run
// against a given network — not part of D3R·AC's application logic.
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

    function setCompleted(uint256 completed) public restricted {
        last_completed_migration = completed;
    }
}
