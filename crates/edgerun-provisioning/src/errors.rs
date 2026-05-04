use edgerun_core::protocol::{Signature, Timestamp};
type EventId = Vec<u8>;
type StreamId = Vec<u8>;
type Identity = edgerun_core::protocol::IdentityRef;
use crate::contract::{verify, PublicKey};
/// Provisioning verification errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvisioningError {
    ContractExpired,
    ContractRevoked,
    ContractKindInvalid,
    ContractSignatureInvalid,
    ContractAlreadyConsumed,
    GenesisNotCommittedToContract,
    GenesisSignatureInvalid,
    BuildHashMismatch,
    ConfigHashMismatch,
    NodeIdentityMismatch,
    UnspecifiedKind,
}

impl std::fmt::Display for ProvisioningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProvisioningError::ContractExpired => write!(f, "Provisioning contract has expired"),
            ProvisioningError::ContractRevoked => {
                write!(f, "Provisioning contract has been revoked")
            }
            ProvisioningError::ContractKindInvalid => {
                write!(f, "Provisioning contract kind is invalid")
            }
            ProvisioningError::ContractSignatureInvalid => {
                write!(f, "Controller signature on contract is invalid")
            }
            ProvisioningError::ContractAlreadyConsumed => write!(
                f,
                "Provisioning contract has already been consumed (replay rejected)"
            ),
            ProvisioningError::GenesisNotCommittedToContract => {
                write!(f, "Node genesis does not commit to this contract")
            }
            ProvisioningError::GenesisSignatureInvalid => {
                write!(f, "Node signature on genesis claim is invalid")
            }
            ProvisioningError::BuildHashMismatch => {
                write!(f, "Build artifact hash does not match contract")
            }
            ProvisioningError::ConfigHashMismatch => {
                write!(f, "Config hash does not match contract")
            }
            ProvisioningError::NodeIdentityMismatch => write!(f, "Node identity mismatch"),
            ProvisioningError::UnspecifiedKind => {
                write!(f, "Provisioning kind is unspecified (rejected)")
            }
        }
    }
}

impl std::error::Error for ProvisioningError {}
