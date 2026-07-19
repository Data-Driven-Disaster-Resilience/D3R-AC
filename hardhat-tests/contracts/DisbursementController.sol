// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./IdentityRegistry.sol";

/// @dev Minimal TRC-20 interface — matches the surface D3RACToken.sol
///      implements and the one frontend/src/lib/tronAdapter.ts already
///      calls against. Kept local (no import of D3RACToken) so this
///      contract works against *any* TRC-20 token, not just D3RACToken.
interface ITRC20 {
    function transfer(address to, uint256 value) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

/// @title DisbursementController
/// @notice Conditional, milestone-based, transparent fund release — the
///         "smart contract layer" described in the top-level README and
///         contracts/tron/README.md. A commitment is created for a
///         verified recipient (checked against IdentityRegistry), split
///         into milestones. Each milestone must be attested by an
///         authorized attester (the trusted "milestone met" signal
///         docs/risk-model.md describes as deployment-specific — an
///         oracle, an authorized off-chain reporter, or a multisig) before
///         its funds can be released. Every state change is an event, so
///         the full lifecycle of a commitment is inspectable on-chain
///         without trusting an intermediary.
/// @dev Dependency-free by design — see D3RACToken.sol for rationale.
///      Uses checks-effects-interactions and a reentrancy guard around the
///      external token transfer in releaseMilestone, since that's the one
///      point where control leaves this contract.
contract DisbursementController {
    struct Milestone {
        string description;
        uint256 amount;
        bool attested;
        bool released;
        address attestedBy;
        uint256 attestedAt;
        uint256 releasedAt;
    }

    struct Commitment {
        address recipient;
        address token;
        string community;
        bool active;        // false once cancelled — no further releases
        bool cancelled;
        uint256 createdAt;
        uint256 totalAmount;   // sum of all milestone amounts
        uint256 releasedAmount;
        Milestone[] milestones;
    }

    IdentityRegistry public immutable registry;
    address public admin;
    mapping(address => bool) public attesters;

    Commitment[] private _commitments;
    bool private _locked; // reentrancy guard

    event CommitmentCreated(
        uint256 indexed commitmentId,
        address indexed recipient,
        address indexed token,
        string community,
        uint256 totalAmount,
        uint256 milestoneCount
    );
    event MilestoneAttested(uint256 indexed commitmentId, uint256 indexed milestoneIndex, address indexed attestedBy);
    event MilestoneReleased(uint256 indexed commitmentId, uint256 indexed milestoneIndex, address indexed recipient, uint256 amount);
    event CommitmentCancelled(uint256 indexed commitmentId, address indexed cancelledBy, uint256 unreleasedAmount);
    event AdminTransferred(address indexed previousAdmin, address indexed newAdmin);
    event AttesterUpdated(address indexed account, bool isAttester);

    modifier onlyAdmin() {
        require(msg.sender == admin, "DisbursementController: caller is not admin");
        _;
    }

    modifier onlyAttester() {
        require(attesters[msg.sender], "DisbursementController: caller is not an attester");
        _;
    }

    modifier nonReentrant() {
        require(!_locked, "DisbursementController: reentrant call");
        _locked = true;
        _;
        _locked = false;
    }

    /// @param registry_ Deployed IdentityRegistry address — recipients
    ///        must be verified there before a commitment can be created.
    /// @param admin_ Should be a multisig before mainnet use — see
    ///        docs/deployment-guide.md.
    constructor(address registry_, address admin_) {
        require(registry_ != address(0), "DisbursementController: registry is zero address");
        require(admin_ != address(0), "DisbursementController: admin is zero address");
        registry = IdentityRegistry(registry_);
        admin = admin_;
        attesters[admin_] = true;
        emit AdminTransferred(address(0), admin_);
        emit AttesterUpdated(admin_, true);
    }

    // ── Admin / attester management ─────────────────────────────────────

    function setAttester(address account, bool isAttester) external onlyAdmin {
        require(account != address(0), "DisbursementController: zero address");
        attesters[account] = isAttester;
        emit AttesterUpdated(account, isAttester);
    }

    function transferAdmin(address newAdmin) external onlyAdmin {
        require(newAdmin != address(0), "DisbursementController: new admin is zero address");
        emit AdminTransferred(admin, newAdmin);
        admin = newAdmin;
    }

    // ── Commitment lifecycle ─────────────────────────────────────────────

    /// @notice Create a milestone-based funding commitment for a verified
    ///         recipient. This does not move any tokens — it only records
    ///         the schedule. This contract must hold (or later receive)
    ///         enough of `token` to cover `totalAmount` before any
    ///         milestone can be released; releaseMilestone checks its own
    ///         balance and reverts rather than partially paying.
    /// @param recipient Must be currently verified in `registry`.
    /// @param token TRC-20 token address funds will be paid out in.
    /// @param community Human-readable label, stored for auditability
    ///        (mirrors IdentityRegistry's recipient label so a commitment
    ///        is legible on its own without a registry lookup).
    /// @param descriptions One entry per milestone (e.g. "Phase 1: potable
    ///        water restored").
    /// @param amounts One entry per milestone, same length as
    ///        `descriptions`, in the token's smallest unit.
    function createCommitment(
        address recipient,
        address token,
        string calldata community,
        string[] calldata descriptions,
        uint256[] calldata amounts
    ) external onlyAdmin returns (uint256 commitmentId) {
        require(registry.isVerified(recipient), "DisbursementController: recipient not verified");
        require(token != address(0), "DisbursementController: token is zero address");
        require(descriptions.length > 0, "DisbursementController: at least one milestone required");
        require(descriptions.length == amounts.length, "DisbursementController: length mismatch");

        commitmentId = _commitments.length;
        _commitments.push();
        Commitment storage c = _commitments[commitmentId];
        c.recipient = recipient;
        c.token = token;
        c.community = community;
        c.active = true;
        c.createdAt = block.timestamp;

        uint256 total = 0;
        for (uint256 i = 0; i < amounts.length; i++) {
            require(amounts[i] > 0, "DisbursementController: milestone amount must be > 0");
            c.milestones.push(Milestone({
                description: descriptions[i],
                amount: amounts[i],
                attested: false,
                released: false,
                attestedBy: address(0),
                attestedAt: 0,
                releasedAt: 0
            }));
            total += amounts[i];
        }
        c.totalAmount = total;

        emit CommitmentCreated(commitmentId, recipient, token, community, total, amounts.length);
    }

    /// @notice Attest that a milestone's condition has been met. This is
    ///         the trust point docs/risk-model.md flags as
    ///         deployment-specific — wire an oracle, an authorized
    ///         off-chain reporter, or a multisig's calls in as an
    ///         attester via setAttester.
    function attestMilestone(uint256 commitmentId, uint256 milestoneIndex) external onlyAttester {
        Commitment storage c = _getActiveCommitment(commitmentId);
        Milestone storage m = _getMilestone(c, milestoneIndex);
        require(!m.attested, "DisbursementController: milestone already attested");

        m.attested = true;
        m.attestedBy = msg.sender;
        m.attestedAt = block.timestamp;

        emit MilestoneAttested(commitmentId, milestoneIndex, msg.sender);
    }

    /// @notice Release an attested milestone's funds to the recipient.
    ///         Callable by anyone once attested — the attestation is the
    ///         real gate, not who happens to submit the release
    ///         transaction, which keeps the process from bottlenecking on
    ///         a single admin key for the payout step itself.
    function releaseMilestone(uint256 commitmentId, uint256 milestoneIndex) external nonReentrant {
        Commitment storage c = _getActiveCommitment(commitmentId);
        Milestone storage m = _getMilestone(c, milestoneIndex);
        require(m.attested, "DisbursementController: milestone not attested");
        require(!m.released, "DisbursementController: milestone already released");
        require(
            ITRC20(c.token).balanceOf(address(this)) >= m.amount,
            "DisbursementController: insufficient contract balance for milestone"
        );

        // Effects before interaction.
        m.released = true;
        m.releasedAt = block.timestamp;
        c.releasedAmount += m.amount;

        emit MilestoneReleased(commitmentId, milestoneIndex, c.recipient, m.amount);

        require(ITRC20(c.token).transfer(c.recipient, m.amount), "DisbursementController: token transfer failed");
    }

    /// @notice Cancel a commitment, halting any further milestone
    ///         releases. Already-released milestones are untouched — this
    ///         only stops future payouts. Funds already held by this
    ///         contract for the cancelled, unreleased milestones stay in
    ///         the contract (they are not auto-swept anywhere) so the
    ///         admin can decide the appropriate next step — e.g. redirect
    ///         to a new commitment — with its own on-chain record, rather
    ///         than this function silently moving funds on cancellation.
    function cancelCommitment(uint256 commitmentId) external onlyAdmin {
        Commitment storage c = _getActiveCommitment(commitmentId);
        c.active = false;
        c.cancelled = true;
        uint256 unreleased = c.totalAmount - c.releasedAmount;
        emit CommitmentCancelled(commitmentId, msg.sender, unreleased);
    }

    // ── Views ────────────────────────────────────────────────────────────

    function commitmentCount() external view returns (uint256) {
        return _commitments.length;
    }

    function getCommitment(uint256 commitmentId) external view returns (
        address recipient,
        address token,
        string memory community,
        bool active,
        bool cancelled,
        uint256 createdAt,
        uint256 totalAmount,
        uint256 releasedAmount,
        uint256 milestoneCount
    ) {
        Commitment storage c = _requireCommitment(commitmentId);
        return (
            c.recipient,
            c.token,
            c.community,
            c.active,
            c.cancelled,
            c.createdAt,
            c.totalAmount,
            c.releasedAmount,
            c.milestones.length
        );
    }

    function getMilestone(uint256 commitmentId, uint256 milestoneIndex) external view returns (Milestone memory) {
        Commitment storage c = _requireCommitment(commitmentId);
        return _getMilestone(c, milestoneIndex);
    }

    // ── Internals ────────────────────────────────────────────────────────

    function _requireCommitment(uint256 commitmentId) private view returns (Commitment storage) {
        require(commitmentId < _commitments.length, "DisbursementController: commitment does not exist");
        return _commitments[commitmentId];
    }

    function _getActiveCommitment(uint256 commitmentId) private view returns (Commitment storage) {
        Commitment storage c = _requireCommitment(commitmentId);
        require(c.active, "DisbursementController: commitment not active");
        return c;
    }

    function _getMilestone(Commitment storage c, uint256 milestoneIndex) private view returns (Milestone storage) {
        require(milestoneIndex < c.milestones.length, "DisbursementController: milestone does not exist");
        return c.milestones[milestoneIndex];
    }
}
