//! On-chain record types -- Casper analogs of `DisbursementController.sol`'s
//! `struct Milestone` / `struct Commitment`. Each implements `CLTyped` +
//! `ToBytes` + `FromBytes` by hand, same approach as
//! identity-registry/src/model.rs's `Recipient` (see that file's module
//! comment for why).
//!
//! `Commitment` embeds its own `milestones: Vec<Milestone>` directly --
//! `Vec<T>` has a blanket `ToBytes`/`FromBytes` impl in
//! `casper_types::bytesrepr` for any `T: ToBytes + FromBytes`, so storing
//! one `Commitment` (milestones and all) as a single dictionary value
//! works cleanly, without needing a second, separately-keyed milestones
//! dictionary the way a relational schema might. `get_milestone`'s entry
//! point still reads a single milestone out of that Vec by index rather
//! than returning the whole commitment, mirroring
//! `DisbursementController.sol`'s own `getMilestone` view exactly.

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
    /// `None` until attested -- Casper has no zero-address to default
    /// to the way `DisbursementController.sol`'s `attestedBy` does.
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
    pub token: Key,
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
        result.extend(self.token.to_bytes()?);
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
            + self.token.serialized_length()
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
        let (token, rem) = Key::from_bytes(rem)?;
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
                token,
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

/// Returned by `get_commitment` -- same shape as
/// `DisbursementController.sol`'s `getCommitment` view: everything
/// except the full milestones list (`milestone_count` instead), so a
/// commitment with many milestones doesn't force every caller to pull
/// the whole thing. Use `get_milestone` for one milestone's detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitmentView {
    pub recipient: Key,
    pub token: Key,
    pub community: String,
    pub active: bool,
    pub cancelled: bool,
    pub created_at: u64,
    pub total_amount: U256,
    pub released_amount: U256,
    pub milestone_count: u64,
}

impl CLTyped for CommitmentView {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl ToBytes for CommitmentView {
    fn to_bytes(&self) -> Result<Vec<u8>, BytesReprError> {
        let mut result = Vec::with_capacity(self.serialized_length());
        result.extend(self.recipient.to_bytes()?);
        result.extend(self.token.to_bytes()?);
        result.extend(self.community.to_bytes()?);
        result.extend(self.active.to_bytes()?);
        result.extend(self.cancelled.to_bytes()?);
        result.extend(self.created_at.to_bytes()?);
        result.extend(self.total_amount.to_bytes()?);
        result.extend(self.released_amount.to_bytes()?);
        result.extend(self.milestone_count.to_bytes()?);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        self.recipient.serialized_length()
            + self.token.serialized_length()
            + self.community.serialized_length()
            + self.active.serialized_length()
            + self.cancelled.serialized_length()
            + self.created_at.serialized_length()
            + self.total_amount.serialized_length()
            + self.released_amount.serialized_length()
            + self.milestone_count.serialized_length()
    }
}

impl FromBytes for CommitmentView {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), BytesReprError> {
        let (recipient, rem) = Key::from_bytes(bytes)?;
        let (token, rem) = Key::from_bytes(rem)?;
        let (community, rem) = String::from_bytes(rem)?;
        let (active, rem) = bool::from_bytes(rem)?;
        let (cancelled, rem) = bool::from_bytes(rem)?;
        let (created_at, rem) = u64::from_bytes(rem)?;
        let (total_amount, rem) = U256::from_bytes(rem)?;
        let (released_amount, rem) = U256::from_bytes(rem)?;
        let (milestone_count, rem) = u64::from_bytes(rem)?;
        Ok((
            CommitmentView {
                recipient,
                token,
                community,
                active,
                cancelled,
                created_at,
                total_amount,
                released_amount,
                milestone_count,
            },
            rem,
        ))
    }
}

impl From<&Commitment> for CommitmentView {
    fn from(c: &Commitment) -> Self {
        CommitmentView {
            recipient: c.recipient,
            token: c.token,
            community: c.community.clone(),
            active: c.active,
            cancelled: c.cancelled,
            created_at: c.created_at,
            total_amount: c.total_amount,
            released_amount: c.released_amount,
            milestone_count: c.milestones.len() as u64,
        }
    }
}
