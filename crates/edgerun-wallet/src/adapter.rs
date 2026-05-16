//! Chain adapter interface for wallet edge code.

use crate::WalletError;
use crate::chain::{WalletAddress, WalletChainId};
use crate::observation::ExternalTxObservation;

pub trait ChainAdapter {
    fn chain(&self) -> &WalletChainId;
    fn validate_address(&self, address: &WalletAddress) -> Result<(), WalletError>;
    fn normalize_address(&self, address: &WalletAddress) -> Result<WalletAddress, WalletError>;
    fn validate_observation(&self, observation: &ExternalTxObservation) -> Result<(), WalletError>;
}
