// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title FundingRequestRegistry
/// @notice The on-chain half of "seek funding sources and request
///         assistance for communities": a public, permissionless-to-read
///         request board. A contract cannot itself browse the web, call an
///         API, or contact a donor — so instead of pretending it can, this
///         contract gives off-chain actors (donor platforms, NGO dashboards,
///         indexer bots, grant-matching services) a single reliable,
///         auditable place to discover which communities need funding,
///         why (linked risk score), and how much (linked milestone amounts)
///         — "open data engagement" in the sense that anyone, anywhere, can
///         read this without permission and act on it off-chain.
/// @dev Designed to reference RiskRegistry and DisbursementController by ID/address
///      rather than duplicating their data, so this stays a thin
///      coordination layer, not a third source of truth.
contract FundingRequestRegistry {
    enum Status {
        Open,
        PartiallyFunded,
        Funded,
        Closed
    }

    struct FundingRequest {
        bytes32 communityId;      // cross-references RiskRegistry's communityId
        address requester;        // who opened the request (NGO coordinator, TAAD ops)
        uint256 amountRequested;  // raw token units requested, informational
        uint256 amountPledged;    // running total of off-chain-reported pledges
        string description;       // what the funding is for
        string dataSourceURI;     // pointer to the open dataset/report justifying the request (IPFS URI, https URL, etc.)
        uint256 linkedCommitmentId; // DisbursementController commitment id once funding is actually committed, or NO_COMMITMENT if none yet
        Status status;
        uint256 createdAt;
        uint256 closedAt;
    }

    address public owner;
    mapping(address => bool) public proposers; // who may open requests (kept restricted so the board isn't spammable)

    FundingRequest[] private _requests;

    uint256 public constant NO_COMMITMENT = type(uint256).max;

    event RequestOpened(
        uint256 indexed requestId,
        bytes32 indexed communityId,
        address indexed requester,
        uint256 amountRequested,
        string dataSourceURI
    );
    event PledgeRecorded(uint256 indexed requestId, uint256 amount, string pledgeSourceURI, address indexed recordedBy);
    event RequestLinkedToCommitment(uint256 indexed requestId, uint256 indexed commitmentId);
    event RequestStatusChanged(uint256 indexed requestId, Status previousStatus, Status newStatus);
    event ProposerAdded(address indexed proposer);
    event ProposerRemoved(address indexed proposer);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    modifier onlyOwner() {
        require(msg.sender == owner, "FundingRequestRegistry: caller is not owner");
        _;
    }

    modifier onlyProposer() {
        require(proposers[msg.sender], "FundingRequestRegistry: caller is not an authorized proposer");
        _;
    }

    constructor(address initialProposer) {
        owner = msg.sender;
        if (initialProposer != address(0)) {
            proposers[initialProposer] = true;
            emit ProposerAdded(initialProposer);
        }
    }

    // ---------------------------------------------------------------
    // Admin
    // ---------------------------------------------------------------
    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "FundingRequestRegistry: zero address");
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }

    function addProposer(address proposer) external onlyOwner {
        require(proposer != address(0), "FundingRequestRegistry: zero address");
        proposers[proposer] = true;
        emit ProposerAdded(proposer);
    }

    function removeProposer(address proposer) external onlyOwner {
        proposers[proposer] = false;
        emit ProposerRemoved(proposer);
    }

    // ---------------------------------------------------------------
    // Request lifecycle
    // ---------------------------------------------------------------

    /// @notice Open a public funding request for a community.
    /// @param dataSourceURI a pointer (IPFS hash, https URL, dataset DOI) to
    ///        the open data justifying this request — e.g. the hazard report
    ///        or RiskRegistry snapshot that shows why this community needs
    ///        support. Keeping this a URI rather than inline data keeps the
    ///        request cheap to open while still making the justification
    ///        independently checkable by anyone.
    function openRequest(
        bytes32 communityId,
        uint256 amountRequested,
        string calldata description,
        string calldata dataSourceURI
    ) external onlyProposer returns (uint256 requestId) {
        require(amountRequested > 0, "FundingRequestRegistry: amount must be > 0");

        requestId = _requests.length;
        _requests.push(
            FundingRequest({
                communityId: communityId,
                requester: msg.sender,
                amountRequested: amountRequested,
                amountPledged: 0,
                description: description,
                dataSourceURI: dataSourceURI,
                linkedCommitmentId: NO_COMMITMENT,
                status: Status.Open,
                createdAt: block.timestamp,
                closedAt: 0
            })
        );

        emit RequestOpened(requestId, communityId, msg.sender, amountRequested, dataSourceURI);
    }

    /// @notice Record a pledge toward a request. This does NOT move funds —
    ///         it's a public ledger entry that a pledge was made (e.g. a
    ///         donor platform integration reporting "$5,000 committed"),
    ///         separate from the actual on-chain transfer, which happens via
    ///         DisbursementController once a commitment is created.
    /// @dev Only the request's own requester or the registry owner can
    ///      record pledges, so a stranger can't fabricate funding progress
    ///      on someone else's request.
    function recordPledge(uint256 requestId, uint256 amount, string calldata pledgeSourceURI) external {
        FundingRequest storage r = _getRequest(requestId);
        require(
            msg.sender == r.requester || msg.sender == owner,
            "FundingRequestRegistry: not authorized to record a pledge on this request"
        );
        require(r.status == Status.Open || r.status == Status.PartiallyFunded, "FundingRequestRegistry: request not open");
        require(amount > 0, "FundingRequestRegistry: pledge amount must be > 0");

        r.amountPledged += amount;
        emit PledgeRecorded(requestId, amount, pledgeSourceURI, msg.sender);

        Status previous = r.status;
        if (r.amountPledged >= r.amountRequested) {
            r.status = Status.Funded;
        } else if (r.amountPledged > 0) {
            r.status = Status.PartiallyFunded;
        }
        if (r.status != previous) {
            emit RequestStatusChanged(requestId, previous, r.status);
        }
    }

    /// @notice Link this request to an actual DisbursementController commitment id,
    ///         once funding has moved from "pledged" to "escrowed on-chain."
    function linkToCommitment(uint256 requestId, uint256 commitmentId) external {
        FundingRequest storage r = _getRequest(requestId);
        require(
            msg.sender == r.requester || msg.sender == owner,
            "FundingRequestRegistry: not authorized to link this request"
        );
        r.linkedCommitmentId = commitmentId;
        emit RequestLinkedToCommitment(requestId, commitmentId);
    }

    function closeRequest(uint256 requestId) external {
        FundingRequest storage r = _getRequest(requestId);
        require(
            msg.sender == r.requester || msg.sender == owner,
            "FundingRequestRegistry: not authorized to close this request"
        );
        Status previous = r.status;
        r.status = Status.Closed;
        r.closedAt = block.timestamp;
        emit RequestStatusChanged(requestId, previous, Status.Closed);
    }

    // ---------------------------------------------------------------
    // Views
    // ---------------------------------------------------------------
    function requestCount() external view returns (uint256) {
        return _requests.length;
    }

    function getRequest(uint256 requestId)
        external
        view
        returns (
            bytes32 communityId,
            address requester,
            uint256 amountRequested,
            uint256 amountPledged,
            string memory description,
            string memory dataSourceURI,
            uint256 linkedCommitmentId,
            Status status,
            uint256 createdAt,
            uint256 closedAt
        )
    {
        FundingRequest storage r = _getRequest(requestId);
        return (
            r.communityId,
            r.requester,
            r.amountRequested,
            r.amountPledged,
            r.description,
            r.dataSourceURI,
            r.linkedCommitmentId,
            r.status,
            r.createdAt,
            r.closedAt
        );
    }

    function _getRequest(uint256 requestId) internal view returns (FundingRequest storage) {
        require(requestId < _requests.length, "FundingRequestRegistry: invalid request id");
        return _requests[requestId];
    }
}
