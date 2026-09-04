//! `system_status()`'s return type -- Casper analog of
//! `D3RACHub.sol::systemStatus()`'s 10-field tuple return. Needs a real
//! struct with its own `CLTyped`/`ToBytes`/`FromBytes` impls, same
//! reasoning as `risk-registry`'s `CommunityView` (`CLType` has no
//! `TupleN` beyond `Tuple3`).

use casper_types::bytesrepr::{Error as BytesReprError, FromBytes, ToBytes};
use casper_types::{CLType, CLTyped, Key, U256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemStatusView {
    pub token_address: Key,
    pub identity_registry_address: Key,
    pub disbursement_controller_address: Key,
    /// `None` if unconfigured -- Casper analog of `D3RACHub.sol`
    /// returning `address(0)` for `riskRegistryAddress` when unset.
    pub risk_registry_address: Option<Key>,
    pub funding_request_registry_address: Option<Key>,
    pub is_paused: bool,
    pub token_total_supply: U256,
    pub total_commitments: u64,
    /// `0` if `risk_registry` is unconfigured, same as
    /// `D3RACHub.sol`'s ternary
    /// (`address(riskRegistry) != address(0) ? riskRegistry.communityCount() : 0`).
    pub total_communities: u64,
    /// `0` if `funding_request_registry` is unconfigured, same
    /// ternary pattern.
    pub total_funding_requests: u64,
}

impl CLTyped for SystemStatusView {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl ToBytes for SystemStatusView {
    fn to_bytes(&self) -> Result<alloc::vec::Vec<u8>, BytesReprError> {
        let mut result = alloc::vec::Vec::with_capacity(self.serialized_length());
        result.extend(self.token_address.to_bytes()?);
        result.extend(self.identity_registry_address.to_bytes()?);
        result.extend(self.disbursement_controller_address.to_bytes()?);
        result.extend(self.risk_registry_address.to_bytes()?);
        result.extend(self.funding_request_registry_address.to_bytes()?);
        result.extend(self.is_paused.to_bytes()?);
        result.extend(self.token_total_supply.to_bytes()?);
        result.extend(self.total_commitments.to_bytes()?);
        result.extend(self.total_communities.to_bytes()?);
        result.extend(self.total_funding_requests.to_bytes()?);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        self.token_address.serialized_length()
            + self.identity_registry_address.serialized_length()
            + self.disbursement_controller_address.serialized_length()
            + self.risk_registry_address.serialized_length()
            + self.funding_request_registry_address.serialized_length()
            + self.is_paused.serialized_length()
            + self.token_total_supply.serialized_length()
            + self.total_commitments.serialized_length()
            + self.total_communities.serialized_length()
            + self.total_funding_requests.serialized_length()
    }
}

impl FromBytes for SystemStatusView {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), BytesReprError> {
        let (token_address, rem) = Key::from_bytes(bytes)?;
        let (identity_registry_address, rem) = Key::from_bytes(rem)?;
        let (disbursement_controller_address, rem) = Key::from_bytes(rem)?;
        let (risk_registry_address, rem) = Option::<Key>::from_bytes(rem)?;
        let (funding_request_registry_address, rem) = Option::<Key>::from_bytes(rem)?;
        let (is_paused, rem) = bool::from_bytes(rem)?;
        let (token_total_supply, rem) = U256::from_bytes(rem)?;
        let (total_commitments, rem) = u64::from_bytes(rem)?;
        let (total_communities, rem) = u64::from_bytes(rem)?;
        let (total_funding_requests, rem) = u64::from_bytes(rem)?;
        Ok((
            SystemStatusView {
                token_address,
                identity_registry_address,
                disbursement_controller_address,
                risk_registry_address,
                funding_request_registry_address,
                is_paused,
                token_total_supply,
                total_commitments,
                total_communities,
                total_funding_requests,
            },
            rem,
        ))
    }
}
