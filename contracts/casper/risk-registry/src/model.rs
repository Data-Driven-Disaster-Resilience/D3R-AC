//! On-chain record type for a single community's risk data --
//! Casper-dictionary-value analog of `RiskRegistry.sol`'s
//! `struct CommunityRisk` / `mapping(bytes32 => CommunityRisk)`.
//!
//! Must implement `CLTyped` + `ToBytes` + `FromBytes` to be storable as
//! a dictionary value (Casper's dictionaries serialize values through
//! the same `bytesrepr` machinery as everything else in global state).

use alloc::string::String;

use casper_types::bytesrepr::{Error as BytesReprError, FromBytes, ToBytes};
use casper_types::CLType;
use casper_types::CLTyped;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommunityRisk {
    pub name: String,
    pub region: String,
    pub hazard: u64,
    pub exposure: u64,
    pub vulnerability: u64,
    /// Casper block time (ms since epoch) of the last `update_risk`
    /// call -- analog of `RiskRegistry.sol`'s `lastUpdated`
    /// (`block.timestamp`, seconds since epoch on TRON/EVM; kept as raw
    /// milliseconds here rather than converted, since nothing in this
    /// contract does arithmetic on it beyond storing/returning it).
    pub last_updated: u64,
    pub registered: bool,
}

impl CLTyped for CommunityRisk {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl ToBytes for CommunityRisk {
    fn to_bytes(&self) -> Result<alloc::vec::Vec<u8>, BytesReprError> {
        let mut result = alloc::vec::Vec::with_capacity(self.serialized_length());
        result.extend(self.name.to_bytes()?);
        result.extend(self.region.to_bytes()?);
        result.extend(self.hazard.to_bytes()?);
        result.extend(self.exposure.to_bytes()?);
        result.extend(self.vulnerability.to_bytes()?);
        result.extend(self.last_updated.to_bytes()?);
        result.extend(self.registered.to_bytes()?);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        self.name.serialized_length()
            + self.region.serialized_length()
            + self.hazard.serialized_length()
            + self.exposure.serialized_length()
            + self.vulnerability.serialized_length()
            + self.last_updated.serialized_length()
            + self.registered.serialized_length()
    }
}

impl FromBytes for CommunityRisk {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), BytesReprError> {
        let (name, rem) = String::from_bytes(bytes)?;
        let (region, rem) = String::from_bytes(rem)?;
        let (hazard, rem) = u64::from_bytes(rem)?;
        let (exposure, rem) = u64::from_bytes(rem)?;
        let (vulnerability, rem) = u64::from_bytes(rem)?;
        let (last_updated, rem) = u64::from_bytes(rem)?;
        let (registered, rem) = bool::from_bytes(rem)?;
        Ok((
            CommunityRisk {
                name,
                region,
                hazard,
                exposure,
                vulnerability,
                last_updated,
                registered,
            },
            rem,
        ))
    }
}
