//! Canonical wallet chain, asset, account, and amount records.

extern crate alloc;

use alloc::string::String;

use edgerun_wire::{Archive, Deserialize, Serialize};
use edgerun_work::PublicKey;

use crate::DecimalAmount;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
#[repr(u8)]
pub enum WalletChainFamily {
    EdgeRun = 1,
    BitcoinLike = 2,
    EvmLike = 3,
    SolanaLike = 4,
    TronLike = 5,
    Other = 255,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct WalletChainId {
    pub abi_version: u16,
    pub family: WalletChainFamily,
    pub network: String,
    pub namespace: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct WalletAssetId {
    pub abi_version: u16,
    pub symbol: String,
    pub chain: WalletChainId,
    pub contract: Option<String>,
    pub decimals: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct WalletAccountId {
    pub abi_version: u16,
    pub owner: PublicKey,
    pub subaccount: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct WalletAddress {
    pub abi_version: u16,
    pub chain: WalletChainId,
    pub address: String,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
pub struct WalletAmount {
    pub mantissa: u128,
    pub scale: u8,
}

impl WalletAmount {
    pub fn new(mantissa: u128, scale: u8) -> Self {
        Self { mantissa, scale }
    }

    pub fn from_decimal(value: DecimalAmount) -> Self {
        Self {
            mantissa: value.mantissa(),
            scale: value.scale(),
        }
    }

    pub fn to_decimal(self) -> DecimalAmount {
        DecimalAmount::from_parts(self.mantissa, self.scale)
    }

    pub fn is_zero(self) -> bool {
        self.mantissa == 0
    }
}
