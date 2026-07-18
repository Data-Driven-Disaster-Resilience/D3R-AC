// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title D3RACToken
/// @notice TRC-20 relief-fund token for D3R·AC. Implements the standard
///         surface the frontend already calls against
///         (frontend/src/lib/tronAdapter.ts): balanceOf, decimals, symbol,
///         transfer — plus the rest of the standard TRC-20/ERC-20 surface
///         (approve, transferFrom, allowance) so it composes with the
///         DisbursementController and any wallet/tooling that expects a
///         normal token.
/// @dev Deliberately dependency-free (no OpenZeppelin import) so it drops
///      into TronBox/TronIDE without a package resolution step. Logic
///      mirrors the audited OpenZeppelin ERC-20 pattern closely.
contract D3RACToken {
    string public name = "D3R-AC Relief Token";
    string public symbol = "D3RAC";
    uint8 public decimals = 18;
    uint256 public totalSupply;

    address public owner;
    mapping(address => bool) public minters;

    mapping(address => uint256) private _balances;
    mapping(address => mapping(address => uint256)) private _allowances;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event MinterUpdated(address indexed account, bool canMint);

    modifier onlyOwner() {
        require(msg.sender == owner, "D3RACToken: caller is not the owner");
        _;
    }

    modifier onlyMinter() {
        require(minters[msg.sender], "D3RACToken: caller is not a minter");
        _;
    }

    /// @param initialSupply Minted to `owner_` at deployment, in whole
    ///        tokens (will be scaled by `decimals`). Pass 0 if funding
    ///        should happen entirely through `mint` later (e.g. from a
    ///        treasury/grant process instead of a pre-mine).
    /// @param owner_ Admin address. Should be a multisig before any
    ///        mainnet deployment — see docs/deployment-guide.md.
    constructor(uint256 initialSupply, address owner_) {
        require(owner_ != address(0), "D3RACToken: owner is zero address");
        owner = owner_;
        minters[owner_] = true;
        emit OwnershipTransferred(address(0), owner_);
        emit MinterUpdated(owner_, true);

        if (initialSupply > 0) {
            uint256 scaled = initialSupply * (10 ** uint256(decimals));
            totalSupply = scaled;
            _balances[owner_] = scaled;
            emit Transfer(address(0), owner_, scaled);
        }
    }

    // ── Standard TRC-20 / ERC-20 surface ────────────────────────────────

    function balanceOf(address account) external view returns (uint256) {
        return _balances[account];
    }

    function allowance(address tokenOwner, address spender) external view returns (uint256) {
        return _allowances[tokenOwner][spender];
    }

    function transfer(address to, uint256 value) external returns (bool) {
        _transfer(msg.sender, to, value);
        return true;
    }

    function approve(address spender, uint256 value) external returns (bool) {
        _approve(msg.sender, spender, value);
        return true;
    }

    function transferFrom(address from, address to, uint256 value) external returns (bool) {
        uint256 currentAllowance = _allowances[from][msg.sender];
        require(currentAllowance >= value, "D3RACToken: transfer exceeds allowance");
        unchecked {
            _approve(from, msg.sender, currentAllowance - value);
        }
        _transfer(from, to, value);
        return true;
    }

    // ── Admin / supply management ───────────────────────────────────────

    /// @notice Mint new tokens. Restricted to addresses explicitly granted
    ///         minter status — intended for a DisbursementController or a
    ///         treasury process, not open minting.
    function mint(address to, uint256 value) external onlyMinter {
        require(to != address(0), "D3RACToken: mint to zero address");
        totalSupply += value;
        _balances[to] += value;
        emit Transfer(address(0), to, value);
    }

    function burn(uint256 value) external {
        _burn(msg.sender, value);
    }

    function setMinter(address account, bool canMint) external onlyOwner {
        minters[account] = canMint;
        emit MinterUpdated(account, canMint);
    }

    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "D3RACToken: new owner is zero address");
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }

    // ── Internals ────────────────────────────────────────────────────────

    function _transfer(address from, address to, uint256 value) private {
        require(to != address(0), "D3RACToken: transfer to zero address");
        uint256 fromBalance = _balances[from];
        require(fromBalance >= value, "D3RACToken: transfer exceeds balance");
        unchecked {
            _balances[from] = fromBalance - value;
        }
        _balances[to] += value;
        emit Transfer(from, to, value);
    }

    function _approve(address tokenOwner, address spender, uint256 value) private {
        require(tokenOwner != address(0), "D3RACToken: approve from zero address");
        require(spender != address(0), "D3RACToken: approve to zero address");
        _allowances[tokenOwner][spender] = value;
        emit Approval(tokenOwner, spender, value);
    }

    function _burn(address from, uint256 value) private {
        uint256 fromBalance = _balances[from];
        require(fromBalance >= value, "D3RACToken: burn exceeds balance");
        unchecked {
            _balances[from] = fromBalance - value;
        }
        totalSupply -= value;
        emit Transfer(from, address(0), value);
    }
}
