//! On-chain record type for a single recipient's verification status --
//! Casper-dictionary-value analog of `IdentityRegistry.sol`'s
//! `struct Recipient` / `mapping(address => Recipient)`.
//!
//! Must implement `CLTyped` + `ToBytes` + `FromBytes` to be storable as
//! a dictionary value -- see risk-registry/src/model.rs's module
//! comment for why (same reasoning applies here).

use alloc::string::String;

use casper_types::bytesrepr::{Error as BytesReprError, FromBytes, ToBytes};
use casper_types::{CLType, CLTyped, Key};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipient {
    pub verified: bool,
    /// Human-readable community/org name -- same field as
    /// `IdentityRegistry.sol`'s `community`.
    pub community: String,
    pub verified_by: Key,
    /// Casper block time (ms since epoch) -- see risk-registry's
    /// `CommunityRisk::last_updated` comment for why this is kept as
    /// raw milliseconds rather than converted to seconds.
    pub verified_at: u64,
    /// 0 while active, matching `IdentityRegistry.sol`'s
    /// `revokedAt`/`revokedAt == 0` convention exactly.
    pub revoked_at: u64,
}

impl CLTyped for Recipient {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl ToBytes for Recipient {
    fn to_bytes(&self) -> Result<alloc::vec::Vec<u8>, BytesReprError> {
        let mut result = alloc::vec::Vec::with_capacity(self.serialized_length());
        result.extend(self.verified.to_bytes()?);
        result.extend(self.community.to_bytes()?);
        result.extend(self.verified_by.to_bytes()?);
        result.extend(self.verified_at.to_bytes()?);
        result.extend(self.revoked_at.to_bytes()?);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        self.verified.serialized_length()
            + self.community.serialized_length()
            + self.verified_by.serialized_length()
            + self.verified_at.serialized_length()
            + self.revoked_at.serialized_length()
    }
}

impl FromBytes for Recipient {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), BytesReprError> {
        let (verified, rem) = bool::from_bytes(bytes)?;
        let (community, rem) = String::from_bytes(rem)?;
        let (verified_by, rem) = Key::from_bytes(rem)?;
        let (verified_at, rem) = u64::from_bytes(rem)?;
        let (revoked_at, rem) = u64::from_bytes(rem)?;
        Ok((
            Recipient {
                verified,
                community,
                verified_by,
                verified_at,
                revoked_at,
            },
            rem,
        ))
    }
}

/// Default/empty recipient record -- returned by `get_recipient` for an
/// address that was never verified, matching Solidity's own behavior of
/// `mapping(address => Recipient)` returning a zero-valued struct for
/// any key never written, rather than reverting.
impl Default for Recipient {
    fn default() -> Self {
        Recipient {
            verified: false,
            community: String::new(),
            verified_by: Key::from(casper_types::AccountHash::new([0u8; 32])),
            verified_at: 0,
            revoked_at: 0,
        }
    }
}
