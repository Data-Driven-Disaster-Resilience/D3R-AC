// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title D3RACProperties
/// @notice Shared base for every D3R·AC TRON contract. Consolidates two
///         things every contract in this suite previously hand-rolled on
///         its own:
///           1. A generic `bytes32`-keyed role registry, replacing the
///              one-off `mapping(address => bool)` each contract kept for
///              its own secondary role (minters, verifiers, attesters,
///              data feeders, proposers).
///           2. A single, audited reentrancy guard, replacing the
///              per-contract copy DisbursementController used to carry
///              alone.
///         Consolidating both here means: one place to audit instead of
///         seven, smaller deployed bytecode per contract (less duplicated
///         logic baked into each one), and a new role — or a brand new
///         contract — is a couple of lines instead of copy-pasting a
///         mapping + modifier + setter + event again.
/// @dev Deliberately does NOT touch the existing single owner/admin
///      pattern (owner/admin + transferOwnership/transferAdmin) already
///      used consistently across the suite — that already works, and
///      folding a single, non-multi-holder role into a multi-holder role
///      registry would be a needless behavior change. This contract only
///      generalizes the *secondary* roles.
///
///      Every check here reverts with the exact message the calling
///      contract passes in (e.g. "IdentityRegistry: caller is not a
///      verifier") — never a message this contract invents — so each
///      contract's existing revert strings, and therefore its existing
///      tests, are unchanged. Inheriting contracts should keep exposing
///      their old named getters (e.g. `verifiers(address)`) as thin views
///      over `hasRole`, for the same reason — see the contracts that
///      inherit this one for the pattern.
abstract contract D3RACProperties {
    mapping(bytes32 => mapping(address => bool)) private _roles;

    event RoleGranted(bytes32 indexed role, address indexed account, address indexed sender);
    event RoleRevoked(bytes32 indexed role, address indexed account, address indexed sender);

    /// @dev Call from a contract-specific modifier, e.g.:
    ///      modifier onlyVerifier() {
    ///          _checkRole(VERIFIER_ROLE, msg.sender, "IdentityRegistry: caller is not a verifier");
    ///          _;
    ///      }
    function _checkRole(bytes32 role, address account, string memory message) internal view {
        require(_roles[role][account], message);
    }

    /// @notice Generic role lookup, usable by frontends/indexers for any
    ///         role across any contract in the suite without needing a
    ///         contract-specific ABI for it.
    function hasRole(bytes32 role, address account) public view returns (bool) {
        return _roles[role][account];
    }

    function _grantRole(bytes32 role, address account) internal {
        if (!_roles[role][account]) {
            _roles[role][account] = true;
            emit RoleGranted(role, account, msg.sender);
        }
    }

    function _revokeRole(bytes32 role, address account) internal {
        if (_roles[role][account]) {
            _roles[role][account] = false;
            emit RoleRevoked(role, account, msg.sender);
        }
    }

    /// @dev Convenience for setters that take a bool, e.g. setVerifier(account, true/false).
    function _setRole(bytes32 role, address account, bool granted) internal {
        if (granted) {
            _grantRole(role, account);
        } else {
            _revokeRole(role, account);
        }
    }

    // ── Shared reentrancy guard ─────────────────────────────────────────
    // Same status-flag pattern DisbursementController used locally;
    // hoisted here so every contract that ever needs it (now or later)
    // shares one implementation instead of re-copying it.
    uint256 private constant _NOT_ENTERED = 1;
    uint256 private constant _ENTERED = 2;
    uint256 private _reentrancyStatus = _NOT_ENTERED;

    modifier nonReentrant() {
        require(_reentrancyStatus != _ENTERED, "D3RACProperties: reentrant call");
        _reentrancyStatus = _ENTERED;
        _;
        _reentrancyStatus = _NOT_ENTERED;
    }
}
