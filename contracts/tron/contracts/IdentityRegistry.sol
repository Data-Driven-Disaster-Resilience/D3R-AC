// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title IdentityRegistry
/// @notice On-chain allow-list of verified fund recipients (communities /
///         NGO coordinators) and the verifiers trusted to vouch for them.
///         DisbursementController checks this registry before creating a
///         funding commitment, so funds can only ever be routed to an
///         address someone with verifier authority has actually attested
///         to — this is the "who is allowed to receive relief funds at
///         all" layer, separate from "has this specific milestone been met"
///         (that's DisbursementController's job).
/// @dev Dependency-free by design — see D3RACToken.sol for rationale.
contract IdentityRegistry {
    struct Recipient {
        bool verified;
        string community;      // human-readable community/org name
        address verifiedBy;
        uint256 verifiedAt;
        uint256 revokedAt;     // 0 while active
    }

    address public admin;
    mapping(address => bool) public verifiers;
    mapping(address => Recipient) public recipients;

    event AdminTransferred(address indexed previousAdmin, address indexed newAdmin);
    event VerifierUpdated(address indexed account, bool isVerifier);
    event RecipientVerified(address indexed recipient, string community, address indexed verifiedBy);
    event RecipientRevoked(address indexed recipient, address indexed revokedBy);

    modifier onlyAdmin() {
        require(msg.sender == admin, "IdentityRegistry: caller is not admin");
        _;
    }

    modifier onlyVerifier() {
        require(verifiers[msg.sender], "IdentityRegistry: caller is not a verifier");
        _;
    }

    /// @param admin_ Should be a multisig before mainnet use — see
    ///        docs/deployment-guide.md's "consider a multisig" item.
    constructor(address admin_) {
        require(admin_ != address(0), "IdentityRegistry: admin is zero address");
        admin = admin_;
        verifiers[admin_] = true;
        emit AdminTransferred(address(0), admin_);
        emit VerifierUpdated(admin_, true);
    }

    // ── Verifier management (admin only) ────────────────────────────────

    function setVerifier(address account, bool isVerifier) external onlyAdmin {
        require(account != address(0), "IdentityRegistry: zero address");
        verifiers[account] = isVerifier;
        emit VerifierUpdated(account, isVerifier);
    }

    function transferAdmin(address newAdmin) external onlyAdmin {
        require(newAdmin != address(0), "IdentityRegistry: new admin is zero address");
        emit AdminTransferred(admin, newAdmin);
        admin = newAdmin;
    }

    // ── Recipient verification (verifier role) ──────────────────────────

    /// @notice Verify a wallet as an eligible relief-fund recipient.
    /// @param recipient The wallet to verify.
    /// @param community Human-readable label (community name, NGO name,
    ///        coordinator role) — stored on-chain for auditability so
    ///        anyone inspecting a disbursement can see who it was meant for
    ///        without an off-chain lookup.
    function verifyRecipient(address recipient, string calldata community) external onlyVerifier {
        require(recipient != address(0), "IdentityRegistry: zero address");
        require(bytes(community).length > 0, "IdentityRegistry: community label required");

        recipients[recipient] = Recipient({
            verified: true,
            community: community,
            verifiedBy: msg.sender,
            verifiedAt: block.timestamp,
            revokedAt: 0
        });

        emit RecipientVerified(recipient, community, msg.sender);
    }

    /// @notice Revoke a previously verified recipient. Does not erase
    ///         history — `recipients[recipient]` still shows who verified
    ///         it and when, plus the revocation timestamp, for auditability.
    function revokeRecipient(address recipient) external onlyVerifier {
        require(recipients[recipient].verified, "IdentityRegistry: recipient not verified");
        recipients[recipient].verified = false;
        recipients[recipient].revokedAt = block.timestamp;
        emit RecipientRevoked(recipient, msg.sender);
    }

    // ── Views ────────────────────────────────────────────────────────────

    function isVerified(address recipient) external view returns (bool) {
        return recipients[recipient].verified;
    }

    function getRecipient(address recipient) external view returns (Recipient memory) {
        return recipients[recipient];
    }
}
