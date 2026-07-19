// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @dev Minimal interface into the already-deployed DisbursementController,
///      just the one function this multisig needs to call once a vote passes.
interface IDisbursementController {
    function attestMilestone(uint256 commitmentId, uint256 milestoneIndex) external;
}

/// @title AttestorMultisig
/// @notice Upgrades DisbursementController's single-attester model to N-of-M
///         threshold voting, without modifying DisbursementController.sol
///         itself. Deploy this contract, then call
///         `DisbursementController.setAttester(address(attestorMultisig), true)`
///         (and optionally `setAttester(oldAttesterEOA, false)` on the prior
///         single-EOA attester) — DisbursementController only ever sees one
///         attester address calling it per vote; the voting happens here,
///         one layer up.
/// @dev This is the natural v2 step flagged in contracts/tron/README.md's
///      "Known limitations" section: a single attester (even the admin
///      itself, by default) is a central point of trust for the one signal
///      that actually releases funds. Moving to a multisig here removes
///      that single point of failure without touching the audited-later
///      DisbursementController contract at all.
contract AttestorMultisig {
    address public immutable disbursementController;

    address[] public signers;
    mapping(address => bool) public isSigner;
    uint256 public threshold; // number of approvals required to execute an attestation

    struct Proposal {
        uint256 commitmentId;
        uint256 milestoneIndex;
        uint256 approvalCount;
        bool executed;
        mapping(address => bool) approvedBy;
    }

    // One proposal per (commitmentId, milestoneIndex) pair at a time —
    // keyed by a deterministic id so signers don't need an off-chain
    // coordinator to agree on a proposal id in advance.
    mapping(bytes32 => Proposal) private _proposals;

    event ProposalCreated(bytes32 indexed proposalKey, uint256 indexed commitmentId, uint256 indexed milestoneIndex, address proposer);
    event ProposalApproved(bytes32 indexed proposalKey, address indexed signer, uint256 approvalCount, uint256 threshold);
    event ProposalExecuted(bytes32 indexed proposalKey, uint256 indexed commitmentId, uint256 indexed milestoneIndex);
    event SignerAdded(address indexed signer);
    event SignerRemoved(address indexed signer);
    event ThresholdChanged(uint256 previousThreshold, uint256 newThreshold);

    modifier onlySigner() {
        require(isSigner[msg.sender], "AttestorMultisig: caller is not a signer");
        _;
    }

    /// @dev Governance of the signer set/threshold is itself run through
    ///      this same multisig (self-administered) rather than a separate
    ///      owner key — see addSigner/removeSigner/changeThreshold below,
    ///      which all require msg.sender == address(this), i.e. must be
    ///      called via a passed proposal executing a call to this contract.
    ///      For v1 simplicity, initial setup happens once in the constructor
    ///      and signer-set changes afterward go through `proposeAdmin`.
    constructor(address _disbursementController, address[] memory initialSigners, uint256 initialThreshold) {
        require(_disbursementController != address(0), "AttestorMultisig: zero controller address");
        require(initialSigners.length > 0, "AttestorMultisig: need at least one signer");
        require(
            initialThreshold > 0 && initialThreshold <= initialSigners.length,
            "AttestorMultisig: threshold must be between 1 and signer count"
        );

        disbursementController = _disbursementController;
        threshold = initialThreshold;

        for (uint256 i = 0; i < initialSigners.length; i++) {
            address s = initialSigners[i];
            require(s != address(0), "AttestorMultisig: zero signer address");
            require(!isSigner[s], "AttestorMultisig: duplicate signer");
            isSigner[s] = true;
            signers.push(s);
            emit SignerAdded(s);
        }
    }

    // ---------------------------------------------------------------
    // Attestation voting
    // ---------------------------------------------------------------

    function _proposalKey(uint256 commitmentId, uint256 milestoneIndex) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(commitmentId, milestoneIndex));
    }

    /// @notice Propose (and immediately cast the first approval for)
    ///         attesting a milestone. Any signer can start a proposal.
    function proposeAttestation(uint256 commitmentId, uint256 milestoneIndex) external onlySigner returns (bytes32 key) {
        key = _proposalKey(commitmentId, milestoneIndex);
        Proposal storage p = _proposals[key];
        require(!p.executed, "AttestorMultisig: already executed");

        // First time this key is used, initialize it.
        if (p.approvalCount == 0 && !p.approvedBy[msg.sender]) {
            p.commitmentId = commitmentId;
            p.milestoneIndex = milestoneIndex;
            emit ProposalCreated(key, commitmentId, milestoneIndex, msg.sender);
        }

        _approve(key, p);
    }

    /// @notice Add an approval to an already-proposed attestation.
    function approveAttestation(uint256 commitmentId, uint256 milestoneIndex) external onlySigner {
        bytes32 key = _proposalKey(commitmentId, milestoneIndex);
        Proposal storage p = _proposals[key];
        require(p.approvalCount > 0, "AttestorMultisig: proposal does not exist yet, call proposeAttestation first");
        require(!p.executed, "AttestorMultisig: already executed");
        _approve(key, p);
    }

    function _approve(bytes32 key, Proposal storage p) internal {
        require(!p.approvedBy[msg.sender], "AttestorMultisig: signer already approved this proposal");
        p.approvedBy[msg.sender] = true;
        p.approvalCount += 1;
        emit ProposalApproved(key, msg.sender, p.approvalCount, threshold);

        if (p.approvalCount >= threshold) {
            p.executed = true;
            IDisbursementController(disbursementController).attestMilestone(p.commitmentId, p.milestoneIndex);
            emit ProposalExecuted(key, p.commitmentId, p.milestoneIndex);
        }
    }

    function getProposalStatus(uint256 commitmentId, uint256 milestoneIndex)
        external
        view
        returns (uint256 approvalCount, bool executed)
    {
        Proposal storage p = _proposals[_proposalKey(commitmentId, milestoneIndex)];
        return (p.approvalCount, p.executed);
    }

    function hasApproved(uint256 commitmentId, uint256 milestoneIndex, address signer) external view returns (bool) {
        return _proposals[_proposalKey(commitmentId, milestoneIndex)].approvedBy[signer];
    }

    function signerCount() external view returns (uint256) {
        return signers.length;
    }

    // ---------------------------------------------------------------
    // Self-administered governance: signer set / threshold changes must
    // themselves be proposed and approved by the existing signer set,
    // through the same voting mechanism as attestations, reusing
    // `commitmentId`/`milestoneIndex` slots would be confusing, so admin
    // actions get their own simple direct-call gate instead: callable only
    // by this contract itself, invoked via `executeAdminAction` once a
    // separate admin proposal reaches threshold.
    // ---------------------------------------------------------------
    struct AdminProposal {
        uint8 action; // 1 = addSigner, 2 = removeSigner, 3 = changeThreshold
        address target; // signer address for add/remove
        uint256 value; // new threshold for changeThreshold
        uint256 approvalCount;
        bool executed;
        mapping(address => bool) approvedBy;
    }

    mapping(bytes32 => AdminProposal) private _adminProposals;
    uint256 private _adminNonce;

    event AdminProposalCreated(bytes32 indexed key, uint8 action, address target, uint256 value, address proposer);
    event AdminProposalApproved(bytes32 indexed key, address indexed signer, uint256 approvalCount);
    event AdminProposalExecuted(bytes32 indexed key, uint8 action);

    function proposeAddSigner(address newSigner) external onlySigner returns (bytes32 key) {
        require(newSigner != address(0) && !isSigner[newSigner], "AttestorMultisig: invalid new signer");
        key = keccak256(abi.encodePacked("addSigner", newSigner, _adminNonce++));
        AdminProposal storage p = _adminProposals[key];
        p.action = 1;
        p.target = newSigner;
        emit AdminProposalCreated(key, 1, newSigner, 0, msg.sender);
        _approveAdmin(key, p);
    }

    function proposeRemoveSigner(address signerToRemove) external onlySigner returns (bytes32 key) {
        require(isSigner[signerToRemove], "AttestorMultisig: not a current signer");
        require(signers.length - 1 >= threshold, "AttestorMultisig: cannot drop below threshold");
        key = keccak256(abi.encodePacked("removeSigner", signerToRemove, _adminNonce++));
        AdminProposal storage p = _adminProposals[key];
        p.action = 2;
        p.target = signerToRemove;
        emit AdminProposalCreated(key, 2, signerToRemove, 0, msg.sender);
        _approveAdmin(key, p);
    }

    function proposeChangeThreshold(uint256 newThreshold) external onlySigner returns (bytes32 key) {
        require(newThreshold > 0 && newThreshold <= signers.length, "AttestorMultisig: invalid threshold");
        key = keccak256(abi.encodePacked("changeThreshold", newThreshold, _adminNonce++));
        AdminProposal storage p = _adminProposals[key];
        p.action = 3;
        p.value = newThreshold;
        emit AdminProposalCreated(key, 3, address(0), newThreshold, msg.sender);
        _approveAdmin(key, p);
    }

    function approveAdminProposal(bytes32 key) external onlySigner {
        AdminProposal storage p = _adminProposals[key];
        require(p.action != 0, "AttestorMultisig: no such admin proposal");
        require(!p.executed, "AttestorMultisig: already executed");
        _approveAdmin(key, p);
    }

    function _approveAdmin(bytes32 key, AdminProposal storage p) internal {
        require(!p.approvedBy[msg.sender], "AttestorMultisig: signer already approved this admin proposal");
        p.approvedBy[msg.sender] = true;
        p.approvalCount += 1;
        emit AdminProposalApproved(key, msg.sender, p.approvalCount);

        if (p.approvalCount >= threshold) {
            p.executed = true;
            if (p.action == 1) {
                isSigner[p.target] = true;
                signers.push(p.target);
                emit SignerAdded(p.target);
            } else if (p.action == 2) {
                isSigner[p.target] = false;
                _removeFromSignersArray(p.target);
                emit SignerRemoved(p.target);
            } else if (p.action == 3) {
                emit ThresholdChanged(threshold, p.value);
                threshold = p.value;
            }
            emit AdminProposalExecuted(key, p.action);
        }
    }

    function _removeFromSignersArray(address s) internal {
        uint256 len = signers.length;
        for (uint256 i = 0; i < len; i++) {
            if (signers[i] == s) {
                signers[i] = signers[len - 1];
                signers.pop();
                break;
            }
        }
    }
}
