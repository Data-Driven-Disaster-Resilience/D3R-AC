//! On-chain record type for a single proposed transaction --
//! Casper-dictionary-value analog of `MultiSigAdmin.sol`'s
//! `struct Transaction` / `Transaction[] public transactions`.
//!
//! Must implement `CLTyped` + `ToBytes` + `FromBytes` to be storable as
//! a dictionary value -- see risk-registry/src/model.rs's module
//! comment for why (same reasoning applies here).
//!
//! Shape differs from `MultiSigAdmin.sol`'s version in one place:
//! there's no `bytes data` field carrying an EVM-style calldata blob.
//! Instead `target_entry_point` (a `String`, the callee's entry point
//! name) and `target_args_bytes` (a bytesrepr-serialized `RuntimeArgs`)
//! stand in for it -- see `main.rs`'s `execute_transaction` doc
//! comment for the full reasoning. There's also no `value` (native
//! token amount) field: this suite's disbursement flows move
//! `D3RACToken`/CEP-18 balances via the token contract's own entry
//! points, not the network's native token, so a submitted transaction
//! carries no native-token payment of its own -- matches
//! `MultiSigAdmin.sol`'s own comment that `value` exists for API
//! completeness rather than active use in this suite today.

use alloc::string::String;
use alloc::vec::Vec;

use casper_types::bytesrepr::{Error as BytesReprError, FromBytes, ToBytes};
use casper_types::{CLType, CLTyped, Key};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub target_package_hash: Key,
    pub target_entry_point: String,
    pub target_args_bytes: Vec<u8>,
    pub executed: bool,
    pub confirmation_count: u64,
}

impl CLTyped for Transaction {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl ToBytes for Transaction {
    fn to_bytes(&self) -> Result<Vec<u8>, BytesReprError> {
        let mut result = Vec::with_capacity(self.serialized_length());
        result.extend(self.target_package_hash.to_bytes()?);
        result.extend(self.target_entry_point.to_bytes()?);
        result.extend(self.target_args_bytes.to_bytes()?);
        result.extend(self.executed.to_bytes()?);
        result.extend(self.confirmation_count.to_bytes()?);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        self.target_package_hash.serialized_length()
            + self.target_entry_point.serialized_length()
            + self.target_args_bytes.serialized_length()
            + self.executed.serialized_length()
            + self.confirmation_count.serialized_length()
    }
}

impl FromBytes for Transaction {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), BytesReprError> {
        let (target_package_hash, rem) = Key::from_bytes(bytes)?;
        let (target_entry_point, rem) = String::from_bytes(rem)?;
        let (target_args_bytes, rem) = Vec::<u8>::from_bytes(rem)?;
        let (executed, rem) = bool::from_bytes(rem)?;
        let (confirmation_count, rem) = u64::from_bytes(rem)?;
        Ok((
            Transaction {
                target_package_hash,
                target_entry_point,
                target_args_bytes,
                executed,
                confirmation_count,
            },
            rem,
        ))
    }
}

/// Default/empty transaction record -- returned by `get_transaction`
/// for a `tx_id` that was never submitted. Unlike `get_recipient`'s
/// use of the same pattern in identity-registry, callers here should
/// generally prefer treating this as "does not exist" (see
/// `TransactionDoesNotExist` in error.rs) rather than trusting the
/// zero value directly -- `get_transaction` itself doesn't
/// distinguish "never submitted" from "submitted with an empty entry
/// point name," so `main.rs` guards mutating entry points with an
/// explicit existence check instead of relying on this default.
impl Default for Transaction {
    fn default() -> Self {
        Transaction {
            target_package_hash: Key::from(casper_types::account::AccountHash::new([0u8; 32])),
            target_entry_point: String::new(),
            target_args_bytes: Vec::new(),
            executed: false,
            confirmation_count: 0,
        }
    }
}
