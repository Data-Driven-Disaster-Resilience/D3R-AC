// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title RiskRegistry
/// @notice Puts D3R·AC's risk model, R(c,t) = H(t)·E(c)·V(c), on-chain per
///         community — the same formula and threshold θ documented in
///         docs/risk-model.md and implemented off-chain in
///         frontend/src/lib/riskModel.ts.
/// @dev This contract does not compute hazard/exposure/vulnerability data
///      itself — a smart contract has no way to sense the real world.
///      Someone (a data feeder: a hazard-data oracle, a designated NGO
///      reporter, or an off-chain job reading public disaster datasets)
///      must call `updateRisk` with fresh H/E/V values. What this contract
///      DOES do reliably on-chain is: store those inputs immutably per
///      update, compute R deterministically the same way every time, and
///      emit a public, timestamped event the moment a community crosses
///      the funding threshold — so anyone watching the chain (a donor bot,
///      an NGO dashboard, FundingRequestRegistry below) can react without
///      trusting a middleman's summary of the data.
contract RiskRegistry {
    uint256 public constant SCALE = 1e18; // fixed-point scale for H, E, V, R (all in [0, 1] represented as [0, 1e18])
    uint256 public riskThreshold; // theta, same scale

    address public owner;
    mapping(address => bool) public dataFeeders;

    struct CommunityRisk {
        string name;
        string region;
        uint256 hazard;        // H(t), 0-1e18
        uint256 exposure;      // E(c), 0-1e18
        uint256 vulnerability; // V(c), 0-1e18
        uint256 lastUpdated;   // block timestamp of last updateRisk call
        bool registered;
    }

    mapping(bytes32 => CommunityRisk) public communities;
    bytes32[] public communityIds;

    event CommunityRegistered(bytes32 indexed communityId, string name, string region);
    event RiskUpdated(
        bytes32 indexed communityId,
        uint256 hazard,
        uint256 exposure,
        uint256 vulnerability,
        uint256 riskScore,
        address indexed feeder
    );
    event ThresholdCrossed(bytes32 indexed communityId, uint256 riskScore, uint256 threshold, uint256 timestamp);
    event DataFeederAdded(address indexed feeder);
    event DataFeederRemoved(address indexed feeder);
    event ThresholdUpdated(uint256 previousThreshold, uint256 newThreshold);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    modifier onlyOwner() {
        require(msg.sender == owner, "RiskRegistry: caller is not owner");
        _;
    }

    modifier onlyDataFeeder() {
        require(dataFeeders[msg.sender], "RiskRegistry: caller is not a data feeder");
        _;
    }

    /// @param initialThreshold theta at 1e18 scale, e.g. 0.35 * 1e18 = 350000000000000000
    constructor(uint256 initialThreshold, address initialDataFeeder) {
        owner = msg.sender;
        riskThreshold = initialThreshold;
        if (initialDataFeeder != address(0)) {
            dataFeeders[initialDataFeeder] = true;
            emit DataFeederAdded(initialDataFeeder);
        }
    }

    // ---------------------------------------------------------------
    // Admin
    // ---------------------------------------------------------------
    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "RiskRegistry: zero address");
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }

    function addDataFeeder(address feeder) external onlyOwner {
        require(feeder != address(0), "RiskRegistry: zero address");
        dataFeeders[feeder] = true;
        emit DataFeederAdded(feeder);
    }

    function removeDataFeeder(address feeder) external onlyOwner {
        dataFeeders[feeder] = false;
        emit DataFeederRemoved(feeder);
    }

    function setRiskThreshold(uint256 newThreshold) external onlyOwner {
        emit ThresholdUpdated(riskThreshold, newThreshold);
        riskThreshold = newThreshold;
    }

    // ---------------------------------------------------------------
    // Community registration & risk updates
    // ---------------------------------------------------------------
    function registerCommunity(bytes32 communityId, string calldata name_, string calldata region) external onlyOwner {
        require(!communities[communityId].registered, "RiskRegistry: community already registered");
        communities[communityId].name = name_;
        communities[communityId].region = region;
        communities[communityId].registered = true;
        communityIds.push(communityId);
        emit CommunityRegistered(communityId, name_, region);
    }

    /// @notice Push fresh hazard/exposure/vulnerability data for a community.
    /// @dev Values must be scaled to 1e18 (e.g. 0.72 -> 720000000000000000).
    ///      Recomputes R(c,t) and emits ThresholdCrossed if it now meets or
    ///      exceeds riskThreshold — the on-chain trigger point the project's
    ///      README describes as gating "milestone-based fund pre-positioning."
    function updateRisk(
        bytes32 communityId,
        uint256 hazard,
        uint256 exposure,
        uint256 vulnerability
    ) external onlyDataFeeder {
        require(communities[communityId].registered, "RiskRegistry: community not registered");
        require(hazard <= SCALE && exposure <= SCALE && vulnerability <= SCALE, "RiskRegistry: value out of [0,1] range");

        CommunityRisk storage c = communities[communityId];
        c.hazard = hazard;
        c.exposure = exposure;
        c.vulnerability = vulnerability;
        c.lastUpdated = block.timestamp;

        uint256 score = riskScore(communityId);
        emit RiskUpdated(communityId, hazard, exposure, vulnerability, score, msg.sender);

        if (score >= riskThreshold) {
            emit ThresholdCrossed(communityId, score, riskThreshold, block.timestamp);
        }
    }

    // ---------------------------------------------------------------
    // Views
    // ---------------------------------------------------------------

    /// @notice R(c,t) = H(t) * E(c) * V(c), all fixed-point at 1e18 scale.
    /// @dev Two divisions by SCALE keep intermediate products from
    ///      overflowing while preserving 1e18 fixed-point precision
    ///      throughout — matches the formula in docs/risk-model.md exactly.
    function riskScore(bytes32 communityId) public view returns (uint256) {
        CommunityRisk storage c = communities[communityId];
        return (((c.hazard * c.exposure) / SCALE) * c.vulnerability) / SCALE;
    }

    function isAboveThreshold(bytes32 communityId) external view returns (bool) {
        return riskScore(communityId) >= riskThreshold;
    }

    function communityCount() external view returns (uint256) {
        return communityIds.length;
    }

    function getCommunity(bytes32 communityId)
        external
        view
        returns (
            string memory name_,
            string memory region,
            uint256 hazard,
            uint256 exposure,
            uint256 vulnerability,
            uint256 lastUpdated,
            uint256 score
        )
    {
        CommunityRisk storage c = communities[communityId];
        require(c.registered, "RiskRegistry: community not registered");
        return (c.name, c.region, c.hazard, c.exposure, c.vulnerability, c.lastUpdated, riskScore(communityId));
    }
}
