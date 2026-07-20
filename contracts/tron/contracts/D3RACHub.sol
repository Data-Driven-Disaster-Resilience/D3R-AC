// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./IdentityRegistry.sol";

/// @dev Minimal interface onto DisbursementController — kept local rather
///      than importing the contract, matching the pattern DisbursementController.sol
///      itself uses for ITRC20. Only the surface D3RACHub actually calls.
interface IDisbursementControllerHub {
    function createCommitment(
        address recipient,
        address token,
        string calldata community,
        string[] calldata descriptions,
        uint256[] calldata amounts
    ) external returns (uint256 commitmentId);

    function attestMilestone(uint256 commitmentId, uint256 milestoneIndex) external;
    function cancelCommitment(uint256 commitmentId) external;
    function commitmentCount() external view returns (uint256);
    function setAttester(address account, bool isAttester) external;
    function transferAdmin(address newAdmin) external;
}

/// @dev Minimal interface onto D3RACToken — only what D3RACHub calls.
interface IMintableToken {
    function totalSupply() external view returns (uint256);
    function balanceOf(address account) external view returns (uint256);
    function mint(address to, uint256 value) external;
    function setMinter(address account, bool canMint) external;
    function transferOwnership(address newOwner) external;
}

/// @dev Minimal interface onto RiskRegistry — only what D3RACHub calls.
interface IRiskRegistryHub {
    function registerCommunity(bytes32 communityId, string calldata name_, string calldata region) external;
    function updateRisk(bytes32 communityId, uint256 hazard, uint256 exposure, uint256 vulnerability) external;
    function communityCount() external view returns (uint256);
    function addDataFeeder(address feeder) external;
    function removeDataFeeder(address feeder) external;
    function setRiskThreshold(uint256 newThreshold) external;
    function transferOwnership(address newOwner) external;
}

/// @dev Minimal interface onto FundingRequestRegistry — only what D3RACHub calls.
interface IFundingRequestRegistryHub {
    function openRequest(
        bytes32 communityId,
        uint256 amountRequested,
        string calldata description,
        string calldata dataSourceURI
    ) external returns (uint256 requestId);

    function closeRequest(uint256 requestId) external;
    function requestCount() external view returns (uint256);
    function recordPledge(uint256 requestId, uint256 amount, string calldata pledgeSourceURI) external;
    function linkToCommitment(uint256 requestId, uint256 commitmentId) external;
    function addProposer(address proposer) external;
    function removeProposer(address proposer) external;
    function transferOwnership(address newOwner) external;
}

/// @title D3RACHub
/// @notice The central coordinator for D3R·AC — the "brain box" that sits
///         in front of D3RACToken, IdentityRegistry, DisbursementController,
///         RiskRegistry, and FundingRequestRegistry, covering BOTH their
///         day-to-day operational writes AND their role/ownership
///         management. Once each underlying contract's admin/owner role
///         has been transferred to the Hub (see "Wiring the Hub" in
///         contracts/tron/README.md), every write path on every one of
///         the five contracts is reachable through the Hub — nothing is
///         left as a direct-call-only escape hatch by design (the
///         permissionless `DisbursementController.releaseMilestone` is
///         the one deliberate exception; see its own note below). It
///         exists to give the project three things none of those
///         contracts provide on their own:
///
///         1. **One admin surface.** Instead of separately managing admin
///            keys on five contracts, an operator (ideally
///            MultiSigAdmin.sol) administers the Hub, and the Hub is
///            granted (or, for full coverage, made the actual admin/owner
///            of) each underlying contract. Day-to-day actions (verify a
///            recipient, attest a milestone, create a commitment, mint,
///            register a community, push a risk update, open a funding
///            request) AND role management (grant/revoke verifier,
///            attester, minter, data-feeder, or proposer status; change
///            RiskRegistry's threshold; transfer any underlying
///            contract's admin/owner role onward) all go through the Hub.
///         2. **One emergency stop.** `pause()` halts the Hub's
///            operational write-paths (verify, createCommitment, attest,
///            mint, registerCommunity, updateRisk, openFundingRequest) in
///            one call. Role/ownership management and all
///            admin/module-management functions stay callable while
///            paused deliberately — those are config actions, not the
///            fund/data-moving operations a pause exists to halt, and you
///            need them available *during* an incident (e.g. revoking a
///            compromised attester).
///         3. **One place to read system status.** `systemStatus()`
///            aggregates state that would otherwise take five separate
///            calls (and five separate contract addresses) for the
///            frontend or a block explorer to assemble.
///
/// @dev The Hub does NOT replace the underlying contracts' own access
///      control — it's an additional caller that must itself be granted
///      (or made the outright holder of) the relevant role after
///      deployment (see contracts/tron/README.md's "Wiring the Hub"
///      section for the full transfer sequence now required for complete
///      coverage, and the additive-vs-exclusive distinction that applies
///      throughout: e.g. updateRisk/openFundingRequest need only additive
///      role grants, while registerCommunity/setRiskThreshold/
///      setRiskDataFeeder need the Hub to hold RiskRegistry's exclusive
///      owner role via transferOwnership). Calling the underlying
///      contracts directly, bypassing the Hub, is still possible for
///      anyone who already holds a role there outside the Hub (e.g. a
///      role granted before the Hub existed, or granted to some other
///      address later) — full wiring makes the Hub *capable* of every
///      write, it doesn't revoke access from whoever else might still
///      hold a role directly. Treat the Hub as the intended single
///      operational surface once wired, not an automatically-enforced
///      security boundary. RiskRegistry and FundingRequestRegistry are
///      optional at construction (address(0) is accepted and can be
///      wired in later) — matching the "connect by convention, not by
///      hard dependency" design already used between those two contracts
///      themselves; their role-management proxies simply revert with a
///      clear "not set" message until an address is configured.
///      Dependency-free by design — see D3RACToken.sol for rationale.
contract D3RACHub {
    address public admin;
    bool public paused;

    IMintableToken public token;
    IdentityRegistry public identityRegistry;
    IDisbursementControllerHub public disbursementController;
    IRiskRegistryHub public riskRegistry;
    IFundingRequestRegistryHub public fundingRequestRegistry;

    event AdminTransferred(address indexed previousAdmin, address indexed newAdmin);
    event ModuleUpdated(bytes32 indexed module, address indexed previousAddress, address indexed newAddress);
    event Paused(address indexed by);
    event Unpaused(address indexed by);

    modifier onlyAdmin() {
        require(msg.sender == admin, "D3RACHub: caller is not admin");
        _;
    }

    modifier whenNotPaused() {
        require(!paused, "D3RACHub: paused");
        _;
    }

    /// @param admin_ Should be a multisig (see MultiSigAdmin.sol) before
    ///        mainnet use — this address controls every module pointer
    ///        and the emergency pause.
    /// @param token_ Deployed D3RACToken (or any IMintableToken-compatible
    ///        token) address. Required.
    /// @param identityRegistry_ Deployed IdentityRegistry address. Required.
    /// @param disbursementController_ Deployed DisbursementController
    ///        address. Required.
    /// @param riskRegistry_ Deployed RiskRegistry address, or address(0)
    ///        to leave it unconfigured for now (set later via
    ///        `setRiskRegistry`) — optional, connect-by-convention rather
    ///        than a hard dependency of the core triad.
    /// @param fundingRequestRegistry_ Deployed FundingRequestRegistry
    ///        address, or address(0) to leave it unconfigured for now
    ///        (set later via `setFundingRequestRegistry`) — same
    ///        optionality as riskRegistry_.
    constructor(
        address admin_,
        address token_,
        address identityRegistry_,
        address disbursementController_,
        address riskRegistry_,
        address fundingRequestRegistry_
    ) {
        require(admin_ != address(0), "D3RACHub: admin is zero address");
        require(token_ != address(0), "D3RACHub: token is zero address");
        require(identityRegistry_ != address(0), "D3RACHub: identityRegistry is zero address");
        require(disbursementController_ != address(0), "D3RACHub: disbursementController is zero address");

        admin = admin_;
        token = IMintableToken(token_);
        identityRegistry = IdentityRegistry(identityRegistry_);
        disbursementController = IDisbursementControllerHub(disbursementController_);
        riskRegistry = IRiskRegistryHub(riskRegistry_);
        fundingRequestRegistry = IFundingRequestRegistryHub(fundingRequestRegistry_);

        emit AdminTransferred(address(0), admin_);
        emit ModuleUpdated("token", address(0), token_);
        emit ModuleUpdated("identityRegistry", address(0), identityRegistry_);
        emit ModuleUpdated("disbursementController", address(0), disbursementController_);
        emit ModuleUpdated("riskRegistry", address(0), riskRegistry_);
        emit ModuleUpdated("fundingRequestRegistry", address(0), fundingRequestRegistry_);
    }

    // ── Admin / module management (always callable, even while paused —
    //    an incident that requires re-pointing a module or changing admin
    //    shouldn't be blocked by the pause meant to contain it) ──────────

    function transferAdmin(address newAdmin) external onlyAdmin {
        require(newAdmin != address(0), "D3RACHub: new admin is zero address");
        emit AdminTransferred(admin, newAdmin);
        admin = newAdmin;
    }

    function setToken(address newToken) external onlyAdmin {
        require(newToken != address(0), "D3RACHub: token is zero address");
        emit ModuleUpdated("token", address(token), newToken);
        token = IMintableToken(newToken);
    }

    function setIdentityRegistry(address newRegistry) external onlyAdmin {
        require(newRegistry != address(0), "D3RACHub: identityRegistry is zero address");
        emit ModuleUpdated("identityRegistry", address(identityRegistry), newRegistry);
        identityRegistry = IdentityRegistry(newRegistry);
    }

    function setDisbursementController(address newController) external onlyAdmin {
        require(newController != address(0), "D3RACHub: disbursementController is zero address");
        emit ModuleUpdated("disbursementController", address(disbursementController), newController);
        disbursementController = IDisbursementControllerHub(newController);
    }

    /// @notice Set or clear the RiskRegistry module. Unlike the three
    ///         core setters above, address(0) is accepted here — this
    ///         pairing is optional, so unwiring it back to "not
    ///         configured" is a legitimate admin action, not an error.
    function setRiskRegistry(address newRiskRegistry) external onlyAdmin {
        emit ModuleUpdated("riskRegistry", address(riskRegistry), newRiskRegistry);
        riskRegistry = IRiskRegistryHub(newRiskRegistry);
    }

    /// @notice Set or clear the FundingRequestRegistry module. Same
    ///         optionality as setRiskRegistry above.
    function setFundingRequestRegistry(address newFundingRequestRegistry) external onlyAdmin {
        emit ModuleUpdated("fundingRequestRegistry", address(fundingRequestRegistry), newFundingRequestRegistry);
        fundingRequestRegistry = IFundingRequestRegistryHub(newFundingRequestRegistry);
    }

    // ── Emergency pause ──────────────────────────────────────────────────

    function pause() external onlyAdmin {
        require(!paused, "D3RACHub: already paused");
        paused = true;
        emit Paused(msg.sender);
    }

    function unpause() external onlyAdmin {
        require(paused, "D3RACHub: not paused");
        paused = false;
        emit Unpaused(msg.sender);
    }

    // ── Orchestration (gated behind admin + pause; requires the Hub to
    //    itself hold verifier/attester/minter status on the underlying
    //    contract — see "Wiring the Hub" in contracts/tron/README.md) ────

    /// @notice Verify a recipient via IdentityRegistry, routed through the
    ///         Hub so it's subject to the pause and shows up alongside
    ///         every other admin action from one contract's event log.
    function verifyRecipient(address recipient, string calldata community) external onlyAdmin whenNotPaused {
        identityRegistry.verifyRecipient(recipient, community);
    }

    /// @notice Create a milestone-based funding commitment via
    ///         DisbursementController. See DisbursementController.sol's
    ///         createCommitment for parameter details.
    function createCommitment(
        address recipient,
        address commitmentToken,
        string calldata community,
        string[] calldata descriptions,
        uint256[] calldata amounts
    ) external onlyAdmin whenNotPaused returns (uint256 commitmentId) {
        return disbursementController.createCommitment(recipient, commitmentToken, community, descriptions, amounts);
    }

    /// @notice Attest a milestone via DisbursementController.
    function attestMilestone(uint256 commitmentId, uint256 milestoneIndex) external onlyAdmin whenNotPaused {
        disbursementController.attestMilestone(commitmentId, milestoneIndex);
    }

    /// @notice Cancel a commitment. Deliberately NOT gated by
    ///         whenNotPaused — halting a bad commitment is exactly the
    ///         kind of defensive action a pause shouldn't block.
    function cancelCommitment(uint256 commitmentId) external onlyAdmin {
        disbursementController.cancelCommitment(commitmentId);
    }

    /// @notice Mint tokens via D3RACToken. Requires the Hub to hold
    ///         minter status on the token (D3RACToken.setMinter).
    function mintTokens(address to, uint256 value) external onlyAdmin whenNotPaused {
        token.mint(to, value);
    }

    /// @notice Register a community via RiskRegistry. Requires riskRegistry
    ///         to be set AND the Hub to actually be RiskRegistry's `owner`
    ///         (RiskRegistry.transferOwnership) — this is exclusive, not
    ///         additive, same as DisbursementController's createCommitment.
    function registerCommunity(bytes32 communityId, string calldata name_, string calldata region)
        external
        onlyAdmin
        whenNotPaused
    {
        require(address(riskRegistry) != address(0), "D3RACHub: riskRegistry not set");
        riskRegistry.registerCommunity(communityId, name_, region);
    }

    /// @notice Push a risk update via RiskRegistry. Requires riskRegistry
    ///         to be set AND the Hub to hold data-feeder status
    ///         (RiskRegistry.addDataFeeder) — additive, same pattern as
    ///         verifier/attester/minter.
    function updateRisk(bytes32 communityId, uint256 hazard, uint256 exposure, uint256 vulnerability)
        external
        onlyAdmin
        whenNotPaused
    {
        require(address(riskRegistry) != address(0), "D3RACHub: riskRegistry not set");
        riskRegistry.updateRisk(communityId, hazard, exposure, vulnerability);
    }

    /// @notice Open a funding request via FundingRequestRegistry. Requires
    ///         fundingRequestRegistry to be set AND the Hub to hold
    ///         proposer status (FundingRequestRegistry.addProposer) —
    ///         additive. Because FundingRequestRegistry records the caller
    ///         as the request's `requester`, a request opened this way is
    ///         attributed to the Hub itself — which is what makes
    ///         closeFundingRequest below work with no further wiring, but
    ///         also means only requests opened *through the Hub* can be
    ///         closed through it (see closeFundingRequest's note).
    function openFundingRequest(
        bytes32 communityId,
        uint256 amountRequested,
        string calldata description,
        string calldata dataSourceURI
    ) external onlyAdmin whenNotPaused returns (uint256 requestId) {
        require(address(fundingRequestRegistry) != address(0), "D3RACHub: fundingRequestRegistry not set");
        return fundingRequestRegistry.openRequest(communityId, amountRequested, description, dataSourceURI);
    }

    /// @notice Close a funding request. FundingRequestRegistry.closeRequest
    ///         only allows the request's own requester or
    ///         FundingRequestRegistry's owner to close it — so this only
    ///         succeeds for requests the Hub itself opened (see
    ///         openFundingRequest), unless the Hub has separately been
    ///         made FundingRequestRegistry's owner via transferOwnership.
    ///         Deliberately NOT gated by whenNotPaused, matching
    ///         cancelCommitment — closing a bad request is a defensive
    ///         action a pause shouldn't block.
    function closeFundingRequest(uint256 requestId) external onlyAdmin {
        require(address(fundingRequestRegistry) != address(0), "D3RACHub: fundingRequestRegistry not set");
        fundingRequestRegistry.closeRequest(requestId);
    }

    // ── Role & ownership management on the underlying contracts (always
    //    callable, even while paused — same reasoning as module
    //    management above: these are config/administration, not the
    //    operational writes the pause exists to halt). Each function here
    //    requires the Hub to already hold the underlying contract's
    //    admin/owner role — see "Wiring the Hub" in
    //    contracts/tron/README.md. Once that transfer has happened once,
    //    the Hub can bootstrap its own remaining role grants through
    //    these same functions (e.g. call setRiskDataFeeder(hubAddress,
    //    true) on itself) rather than requiring the original owner to do
    //    it separately. ──────────────────────────────────────────────────

    /// @notice Grant/revoke verifier status on IdentityRegistry. Requires
    ///         the Hub to be IdentityRegistry's admin (transferAdmin).
    function setIdentityVerifier(address account, bool isVerifier) external onlyAdmin {
        identityRegistry.setVerifier(account, isVerifier);
    }

    /// @notice Transfer IdentityRegistry's own admin role elsewhere.
    ///         Requires the Hub to currently hold it. Note this can move
    ///         IdentityRegistry's admin OFF the Hub entirely if misused —
    ///         same caution as any admin-transfer function.
    function transferIdentityRegistryAdmin(address newAdmin) external onlyAdmin {
        identityRegistry.transferAdmin(newAdmin);
    }

    /// @notice Revoke a previously verified recipient via IdentityRegistry.
    ///         Requires the Hub to hold verifier status (additive — see
    ///         verifyRecipient above; no separate wiring needed beyond
    ///         what verifyRecipient already requires).
    function revokeRecipient(address recipient) external onlyAdmin {
        identityRegistry.revokeRecipient(recipient);
    }

    /// @notice Grant/revoke attester status on DisbursementController.
    ///         Requires the Hub to be DisbursementController's admin —
    ///         already required for createCommitment/cancelCommitment, so
    ///         no additional wiring beyond what those already need.
    function setDisbursementAttester(address account, bool isAttester) external onlyAdmin {
        disbursementController.setAttester(account, isAttester);
    }

    /// @notice Transfer DisbursementController's own admin role elsewhere.
    ///         Requires the Hub to currently hold it.
    function transferDisbursementControllerAdmin(address newAdmin) external onlyAdmin {
        disbursementController.transferAdmin(newAdmin);
    }

    /// @notice Grant/revoke minter status on D3RACToken. Requires the Hub
    ///         to be D3RACToken's owner (transferOwnership) — mintTokens
    ///         above only requires minter status, which is a lower bar
    ///         than this function needs.
    function setTokenMinter(address account, bool canMint) external onlyAdmin {
        token.setMinter(account, canMint);
    }

    /// @notice Transfer D3RACToken's own owner role elsewhere. Requires
    ///         the Hub to currently hold it.
    function transferTokenOwnership(address newOwner) external onlyAdmin {
        token.transferOwnership(newOwner);
    }

    /// @notice Grant/revoke data-feeder status on RiskRegistry. Requires
    ///         the Hub to be RiskRegistry's owner — already required for
    ///         registerCommunity, so no additional wiring beyond that.
    function setRiskDataFeeder(address feeder, bool isFeeder) external onlyAdmin {
        require(address(riskRegistry) != address(0), "D3RACHub: riskRegistry not set");
        if (isFeeder) {
            riskRegistry.addDataFeeder(feeder);
        } else {
            riskRegistry.removeDataFeeder(feeder);
        }
    }

    /// @notice Set RiskRegistry's threshold θ. Requires the Hub to be
    ///         RiskRegistry's owner.
    function setRiskThreshold(uint256 newThreshold) external onlyAdmin {
        require(address(riskRegistry) != address(0), "D3RACHub: riskRegistry not set");
        riskRegistry.setRiskThreshold(newThreshold);
    }

    /// @notice Transfer RiskRegistry's own owner role elsewhere. Requires
    ///         the Hub to currently hold it.
    function transferRiskRegistryOwnership(address newOwner) external onlyAdmin {
        require(address(riskRegistry) != address(0), "D3RACHub: riskRegistry not set");
        riskRegistry.transferOwnership(newOwner);
    }

    /// @notice Grant/revoke proposer status on FundingRequestRegistry.
    ///         Requires the Hub to be FundingRequestRegistry's owner —
    ///         NOT automatically true just because the Hub can call
    ///         openFundingRequest (that only needs proposer status,
    ///         additive). This function specifically needs the exclusive
    ///         owner role, via transferOwnership.
    function setFundingProposer(address proposer, bool isProposer) external onlyAdmin {
        require(address(fundingRequestRegistry) != address(0), "D3RACHub: fundingRequestRegistry not set");
        if (isProposer) {
            fundingRequestRegistry.addProposer(proposer);
        } else {
            fundingRequestRegistry.removeProposer(proposer);
        }
    }

    /// @notice Record a pledge against a funding request via
    ///         FundingRequestRegistry. FundingRequestRegistry.recordPledge
    ///         only allows the request's own requester or the registry's
    ///         owner to call it — so, same caveat as closeFundingRequest,
    ///         this only succeeds for requests the Hub itself opened
    ///         unless the Hub has separately been made the registry's
    ///         owner.
    function recordFundingPledge(uint256 requestId, uint256 amount, string calldata pledgeSourceURI)
        external
        onlyAdmin
    {
        require(address(fundingRequestRegistry) != address(0), "D3RACHub: fundingRequestRegistry not set");
        fundingRequestRegistry.recordPledge(requestId, amount, pledgeSourceURI);
    }

    /// @notice Link a funding request to a DisbursementController
    ///         commitment id. Same requester-or-owner caveat as
    ///         recordFundingPledge/closeFundingRequest above.
    function linkFundingRequestToCommitment(uint256 requestId, uint256 commitmentId) external onlyAdmin {
        require(address(fundingRequestRegistry) != address(0), "D3RACHub: fundingRequestRegistry not set");
        fundingRequestRegistry.linkToCommitment(requestId, commitmentId);
    }

    /// @notice Transfer FundingRequestRegistry's own owner role elsewhere.
    ///         Requires the Hub to currently hold it.
    function transferFundingRequestRegistryOwnership(address newOwner) external onlyAdmin {
        require(address(fundingRequestRegistry) != address(0), "D3RACHub: fundingRequestRegistry not set");
        fundingRequestRegistry.transferOwnership(newOwner);
    }

    // ── Aggregate status (one call instead of five contracts) ─────────

    function systemStatus() external view returns (
        address tokenAddress,
        address identityRegistryAddress,
        address disbursementControllerAddress,
        address riskRegistryAddress,
        address fundingRequestRegistryAddress,
        bool isPaused,
        uint256 tokenTotalSupply,
        uint256 totalCommitments,
        uint256 totalCommunities,
        uint256 totalFundingRequests
    ) {
        return (
            address(token),
            address(identityRegistry),
            address(disbursementController),
            address(riskRegistry),
            address(fundingRequestRegistry),
            paused,
            token.totalSupply(),
            disbursementController.commitmentCount(),
            address(riskRegistry) != address(0) ? riskRegistry.communityCount() : 0,
            address(fundingRequestRegistry) != address(0) ? fundingRequestRegistry.requestCount() : 0
        );
    }
}
