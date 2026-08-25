//! On-chain record types -- Casper-dictionary-value analogs of
//! `DisbursementController.sol`'s `struct Milestone` / `struct
//! Commitment` / `Commitment[] private _commitments`.
//!
//! Must implement `CLTyped` + `ToBytes` + `FromBytes` to be storable
//! as a dictionary value -- see risk-registry/src/model.rs's module
//! comment for why (same reasoning applies here). `Commitment` embeds
//! `Vec<Milestone>` directly in one dictionary entry per commitment
//! (rather than a second, milestone-indexed dictionary) -- simpler,
//! and Solidity's own storage layout does the equivalent thing
//! (`Milestone[] milestones` living inside the `Commitment` struct);
//! the only cost is that `attest_milestone`/`release_milestone`
//! rewrite the whole `Commitment` (including every other milestone)
//! on each call, which is fine at the milestone-counts this suite
//! anticipates (single digits per commitment, matching
//! `docs/casper-contracts-srs.md`'s FR-3 description) and not the
//! access pattern that would need a second dictionary instead.
//!
//! `created_at`/`attested_at`/`released_at` use Casper's own
//! `runtime::get_blocktime()` (milliseconds since epoch) as the
//! `block.timestamp`-equivalent -- close enough for the "when did
//! this happen" auditability purpose these fields serve, not claimed
//! to be identical semantics to Ethereum/TRON's own block timestamp
//! mechanics.

use alloc::string::String;
use alloc::vec::Vec;

use casper_types::bytesrepr::{Error as BytesReprError, FromBytes, ToBytes};
use casper_types::{CLType, CLTyped, Key, U256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Milestone {
    pub description: String,
    pub amount: U256,
    pub attested: bool,
    pub released: bool,
    pub attested_by: Option<Key>,
    pub attested_at: u64,
    pub released_at: u64,
}

impl CLTyped for Milestone {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl ToBytes for Milestone {
    fn to_bytes(&self) -> Result<Vec<u8>, BytesReprError> {
        let mut result = Vec::with_capacity(self.serialized_length());
        result.extend(self.description.to_bytes()?);
        result.extend(self.amount.to_bytes()?);
        result.extend(self.attested.to_bytes()?);
        result.extend(self.released.to_bytes()?);
        result.extend(self.attested_by.to_bytes()?);
        result.extend(self.attested_at.to_bytes()?);
        result.extend(self.released_at.to_bytes()?);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        self.description.serialized_length()
            + self.amount.serialized_length()
            + self.attested.serialized_length()
            + self.released.serialized_length()
            + self.attested_by.serialized_length()
            + self.attested_at.serialized_length()
            + self.released_at.serialized_length()
    }
}

impl FromBytes for Milestone {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), BytesReprError> {
        let (description, rem) = String::from_bytes(bytes)?;
        let (amount, rem) = U256::from_bytes(rem)?;
        let (attested, rem) = bool::from_bytes(rem)?;
        let (released, rem) = bool::from_bytes(rem)?;
        let (attested_by, rem) = Option::<Key>::from_bytes(rem)?;
        let (attested_at, rem) = u64::from_bytes(rem)?;
        let (released_at, rem) = u64::from_bytes(rem)?;
        Ok((
            Milestone {
                description,
                amount,
                attested,
                released,
                attested_by,
                attested_at,
                released_at,
            },
            rem,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commitment {
    pub recipient: Key,
    pub token_package_hash: Key,
    pub community: String,
    pub active: bool,
    pub cancelled: bool,
    pub created_at: u64,
    pub total_amount: U256,
    pub released_amount: U256,
    pub milestones: Vec<Milestone>,
}

impl CLTyped for Commitment {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl ToBytes for Commitment {
    fn to_bytes(&self) -> Result<Vec<u8>, BytesReprError> {
        let mut result = Vec::with_capacity(self.serialized_length());
        result.extend(self.recipient.to_bytes()?);
        result.extend(self.token_package_hash.to_bytes()?);
        result.extend(self.community.to_bytes()?);
        result.extend(self.active.to_bytes()?);
        result.extend(self.cancelled.to_bytes()?);
        result.extend(self.created_at.to_bytes()?);
        result.extend(self.total_amount.to_bytes()?);
        result.extend(self.released_amount.to_bytes()?);
        result.extend(self.milestones.to_bytes()?);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        self.recipient.serialized_length()
            + self.token_package_hash.serialized_length()
            + self.community.serialized_length()
            + self.active.serialized_length()
            + self.cancelled.serialized_length()
            + self.created_at.serialized_length()
            + self.total_amount.serialized_length()
            + self.released_amount.serialized_length()
            + self.milestones.serialized_length()
    }
}

impl FromBytes for Commitment {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), BytesReprError> {
        let (recipient, rem) = Key::from_bytes(bytes)?;
        let (token_package_hash, rem) = Key::from_bytes(rem)?;
        let (community, rem) = String::from_bytes(rem)?;
        let (active, rem) = bool::from_bytes(rem)?;
        let (cancelled, rem) = bool::from_bytes(rem)?;
        let (created_at, rem) = u64::from_bytes(rem)?;
        let (total_amount, rem) = U256::from_bytes(rem)?;
        let (released_amount, rem) = U256::from_bytes(rem)?;
        let (milestones, rem) = Vec::<Milestone>::from_bytes(rem)?;
        Ok((
            Commitment {
                recipient,
                token_package_hash,
                community,
                active,
                cancelled,
                created_at,
                total_amount,
                released_amount,
                milestones,
            },
            rem,
        ))
    }
}
