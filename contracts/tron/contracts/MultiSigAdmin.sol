// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title MultiSigAdmin
/// @notice Minimal N-of-M multisig, meant to hold the `admin` role on
///         IdentityRegistry and DisbursementController (and/or the
///         `owner` role on D3RACToken) instead of a single EOA — this is
///         the "consider a multisig for any contract-owner or admin role"
///         item from docs/deployment-guide.md's security checklist.
///
///         Deploy this first, then pass its address as the `admin_` /
///         `owner_` constructor argument on the other contracts. Any call
///         those contracts would normally receive from a single admin key
///         (setAttester, setVerifier, createCommitment, transferOwnership,
///         etc.) instead gets proposed here via `submitTransaction` and
///         only executes once `threshold` owners have confirmed it.
///
/// @dev Deliberately small and auditable rather than feature-complete —
///      no owner-management-through-itself, no daily limits, no batching.
///      Owners are fixed at deployment; rotate by deploying a new
///      MultiSigAdmin and re-pointing the other contracts' admin role to
///      it (each of those contracts supports transferAdmin/
///      transferOwnership for exactly this).
contract MultiSigAdmin {
    struct Transaction {
        address to;
        uint256 value;
        bytes data;
        bool executed;
        uint256 confirmationCount;
    }

    address[] public owners;
    mapping(address => bool) public isOwner;
    uint256 public threshold;

    Transaction[] private _transactions;
    mapping(uint256 => mapping(address => bool)) private _confirmations;

    event OwnerAdded(address indexed owner);
    event TransactionSubmitted(uint256 indexed txId, address indexed submitter, address indexed to, uint256 value);
    event TransactionConfirmed(uint256 indexed txId, address indexed owner);
    event ConfirmationRevoked(uint256 indexed txId, address indexed owner);
    event TransactionExecuted(uint256 indexed txId, address indexed executor);
    event TransactionExecutionFailed(uint256 indexed txId);

    modifier onlyOwner() {
        require(isOwner[msg.sender], "MultiSigAdmin: caller is not an owner");
        _;
    }

    modifier txExists(uint256 txId) {
        require(txId < _transactions.length, "MultiSigAdmin: transaction does not exist");
        _;
    }

    modifier notExecuted(uint256 txId) {
        require(!_transactions[txId].executed, "MultiSigAdmin: transaction already executed");
        _;
    }

    /// @param owners_ Initial owner set. No duplicates, no zero addresses.
    /// @param threshold_ Number of confirmations required to execute,
    ///        1 <= threshold_ <= owners_.length.
    constructor(address[] memory owners_, uint256 threshold_) {
        require(owners_.length > 0, "MultiSigAdmin: owners required");
        require(threshold_ > 0 && threshold_ <= owners_.length, "MultiSigAdmin: invalid threshold");

        for (uint256 i = 0; i < owners_.length; i++) {
            address o = owners_[i];
            require(o != address(0), "MultiSigAdmin: zero address owner");
            require(!isOwner[o], "MultiSigAdmin: duplicate owner");
            isOwner[o] = true;
            owners.push(o);
            emit OwnerAdded(o);
        }
        threshold = threshold_;
    }

    /// @notice Propose a call (e.g. IdentityRegistry.setVerifier(...) or
    ///         DisbursementController.attestMilestone(...)), encoded as
    ///         `data`. Auto-confirms from the submitter.
    function submitTransaction(address to, uint256 value, bytes calldata data) external onlyOwner returns (uint256 txId) {
        require(to != address(0), "MultiSigAdmin: target is zero address");
        txId = _transactions.length;
        _transactions.push(Transaction({ to: to, value: value, data: data, executed: false, confirmationCount: 0 }));
        emit TransactionSubmitted(txId, msg.sender, to, value);
        _confirm(txId);
    }

    function confirmTransaction(uint256 txId) external onlyOwner txExists(txId) notExecuted(txId) {
        require(!_confirmations[txId][msg.sender], "MultiSigAdmin: already confirmed");
        _confirm(txId);
    }

    function revokeConfirmation(uint256 txId) external onlyOwner txExists(txId) notExecuted(txId) {
        require(_confirmations[txId][msg.sender], "MultiSigAdmin: not confirmed");
        _confirmations[txId][msg.sender] = false;
        _transactions[txId].confirmationCount -= 1;
        emit ConfirmationRevoked(txId, msg.sender);
    }

    /// @notice Execute a transaction once it has >= threshold confirmations.
    ///         Reverts if the underlying call reverts, so a failed
    ///         execution never silently marks the transaction as done.
    function executeTransaction(uint256 txId) external onlyOwner txExists(txId) notExecuted(txId) {
        Transaction storage t = _transactions[txId];
        require(t.confirmationCount >= threshold, "MultiSigAdmin: insufficient confirmations");

        t.executed = true;
        (bool success, ) = t.to.call{ value: t.value }(t.data);
        if (!success) {
            t.executed = false;
            emit TransactionExecutionFailed(txId);
            revert("MultiSigAdmin: underlying call reverted");
        }
        emit TransactionExecuted(txId, msg.sender);
    }

    // ── Views ────────────────────────────────────────────────────────────

    function ownerCount() external view returns (uint256) {
        return owners.length;
    }

    function transactionCount() external view returns (uint256) {
        return _transactions.length;
    }

    function getTransaction(uint256 txId) external view txExists(txId) returns (
        address to, uint256 value, bytes memory data, bool executed, uint256 confirmationCount
    ) {
        Transaction storage t = _transactions[txId];
        return (t.to, t.value, t.data, t.executed, t.confirmationCount);
    }

    function isConfirmed(uint256 txId, address owner) external view returns (bool) {
        return _confirmations[txId][owner];
    }

    // ── Internals ────────────────────────────────────────────────────────

    function _confirm(uint256 txId) private {
        _confirmations[txId][msg.sender] = true;
        _transactions[txId].confirmationCount += 1;
        emit TransactionConfirmed(txId, msg.sender);
    }

    receive() external payable {}
}
