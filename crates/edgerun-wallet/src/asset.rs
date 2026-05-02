//! Asset model for EdgeRun Wallet.
//! 
//! AssetRef is a reference to an asset.
//! Asset is the full asset record.
//! Canonical asset ID: SYMBOL:network[:contract]

use alloc::string::{String, ToString};
use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetRef {
    pub symbol: String,
    pub network: String,
    pub contract: Option<String>,
    pub decimals: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub id: String, // canonical: SYMBOL:network[:contract]
    pub symbol: String,
    pub network: String,
    pub contract: Option<String>,
    pub decimals: u32,
    pub name: Option<String>,
}

/// Build canonical asset ID.
/// Format: SYMBOL:network[:contract]
pub fn canonical_asset_id(symbol: &str, network: &str, contract: Option<&str>) -> String {
    match contract {
        Some(c) => alloc::format!("{}:{}:{}", symbol.to_uppercase(), network.to_lowercase(), c),
        None => alloc::format!("{}:{}", symbol.to_uppercase(), network.to_lowercase()),
    }
}

impl fmt::Display for AssetRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", canonical_asset_id(&self.symbol, &self.network, self.contract.as_deref()))
    }
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_id_simple() {
        let id = canonical_asset_id("USDT", "tron", None);
        assert_eq!(id, "USDT:tron");
    }

    #[test]
    fn canonical_id_with_contract() {
        let id = canonical_asset_id("USDT", "ethereum", Some("0xdac17f..."));
        assert_eq!(id, "USDT:ethereum:0xdac17f...");
    }

    #[test]
    fn canonical_id_doge() {
        let id = canonical_asset_id("DOGE", "dogecoin", None);
        assert_eq!(id, "DOGE:dogecoin");
    }

    #[test]
    fn asset_ref_display() {
        let asset_ref = AssetRef {
            symbol: "BTC".into(),
            network: "bitcoin".into(),
            contract: None,
            decimals: Some(8),
        };
        assert_eq!(asset_ref.to_string(), "BTC:bitcoin");
    }
}
