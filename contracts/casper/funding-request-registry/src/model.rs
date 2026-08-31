//! On-chain record type for a single funding request -- Casper analog
//! of `FundingRequestRegistry.sol`'s `struct FundingRequest` /
//! `FundingRequest[] private _requests`. Stored as a dictionary value
//! keyed by request id (a `u64` string-formatted the same way every
//! other numeric dictionary key in this suite is -- see
//! `request_id_to_dict_key` in main.rs), rather than as an actual
//! growable on-chain array/list the way `d3rac-token`'s allowance
//! dictionary or `disbursement-controller`'s commitments dictionary
//! already do it: Casper dictionaries don't support iteration/length
//! the way a Solidity storage array does, so `request_count` is
//! tracked as its own separate `u64` counter (`KEY_REQUEST_COUNT`),
//! same pattern `disbursement-controller`'s `commitment_count` uses.
//!
//! `bytes32 communityId` on the Solidity side is a `String` here, not
//! a raw 32-byte hash -- matching every other contract in this suite
//! that references a community id (`risk-registry`'s own
//! `community_id: String`), not re-litigated here.

use alloc::string::String;

use casper_types::bytesrepr::{Error as BytesReprError, FromBytes, ToBytes};
use casper_types::{CLType, CLTyped, Key, U256};

/// Casper analog of `FundingRequestRegistry.sol`'s `enum Status`.
/// Explicit `u8` discriminants (rather than relying on Rust's default
/// enum layout, which isn't part of its stable ABI) so the on-chain
/// byte representation is a documented, stable fact rather than a
/// compiler implementation detail -- same reasoning
/// `error.rs`'s `#[repr(u16)]` uses for the same class of concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RequestStatus {
    Open = 0,
    PartiallyFunded = 1,
    Funded = 2,
    Closed = 3,
}

impl CLTyped for RequestStatus {
    fn cl_type() -> CLType {
        CLType::U8
    }
}

impl ToBytes for RequestStatus {
    fn to_bytes(&self) -> Result<alloc::vec::Vec<u8>, BytesReprError> {
        (*self as u8).to_bytes()
    }

    fn serialized_length(&self) -> usize {
        (*self as u8).serialized_length()
    }
}

impl FromBytes for RequestStatus {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), BytesReprError> {
        let (tag, rem) = u8::from_bytes(bytes)?;
        let status = match tag {
            0 => RequestStatus::Open,
            1 => RequestStatus::PartiallyFunded,
            2 => RequestStatus::Funded,
            3 => RequestStatus::Closed,
            // Same "should be unreachable outside a corrupted deploy"
            // reasoning risk-registry/error.rs documents for its own
            // storage-layer error variants -- a tag this contract
            // never itself writes can only appear here via tampered
            // storage, not a real code path, so reverting via
            // `BytesReprError::Formatting` (bytesrepr's own generic
            // "couldn't parse this" variant) rather than inventing a
            // 5th status is the honest way to surface that.
            _ => return Err(BytesReprError::Formatting),
        };
        Ok((status, rem))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundingRequest {
    pub community_id: String,
    pub requester: Key,
    pub amount_requested: U256,
    pub amount_pledged: U256,
    pub description: String,
    pub data_source_uri: String,
    /// `None` == `FundingRequestRegistry.sol`'s `NO_COMMITMENT`
    /// sentinel (`type(uint256).max`). `Option<u64>` (`None` for
    /// unlinked) is the more idiomatic Rust encoding of that same
    /// "not yet linked" state, and `u64` rather than `U256` matches
    /// every other id (as opposed to amount) in this suite --
    /// `disbursement-controller`'s own `commitment_id` is `u64`.
    pub linked_commitment_id: Option<u64>,
    pub status: RequestStatus,
    pub created_at: u64,
    pub closed_at: u64,
}

impl CLTyped for FundingRequest {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl ToBytes for FundingRequest {
    fn to_bytes(&self) -> Result<alloc::vec::Vec<u8>, BytesReprError> {
        let mut result = alloc::vec::Vec::with_capacity(self.serialized_length());
        result.extend(self.community_id.to_bytes()?);
        result.extend(self.requester.to_bytes()?);
        result.extend(self.amount_requested.to_bytes()?);
        result.extend(self.amount_pledged.to_bytes()?);
        result.extend(self.description.to_bytes()?);
        result.extend(self.data_source_uri.to_bytes()?);
        result.extend(self.linked_commitment_id.to_bytes()?);
        result.extend(self.status.to_bytes()?);
        result.extend(self.created_at.to_bytes()?);
        result.extend(self.closed_at.to_bytes()?);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        self.community_id.serialized_length()
            + self.requester.serialized_length()
            + self.amount_requested.serialized_length()
            + self.amount_pledged.serialized_length()
            + self.description.serialized_length()
            + self.data_source_uri.serialized_length()
            + self.linked_commitment_id.serialized_length()
            + self.status.serialized_length()
            + self.created_at.serialized_length()
            + self.closed_at.serialized_length()
    }
}

impl FromBytes for FundingRequest {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), BytesReprError> {
        let (community_id, rem) = String::from_bytes(bytes)?;
        let (requester, rem) = Key::from_bytes(rem)?;
        let (amount_requested, rem) = U256::from_bytes(rem)?;
        let (amount_pledged, rem) = U256::from_bytes(rem)?;
        let (description, rem) = String::from_bytes(rem)?;
        let (data_source_uri, rem) = String::from_bytes(rem)?;
        let (linked_commitment_id, rem) = Option::<u64>::from_bytes(rem)?;
        let (status, rem) = RequestStatus::from_bytes(rem)?;
        let (created_at, rem) = u64::from_bytes(rem)?;
        let (closed_at, rem) = u64::from_bytes(rem)?;
        Ok((
            FundingRequest {
                community_id,
                requester,
                amount_requested,
                amount_pledged,
                description,
                data_source_uri,
                linked_commitment_id,
                status,
                created_at,
                closed_at,
            },
            rem,
        ))
    }
}
