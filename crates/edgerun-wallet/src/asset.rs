//! Asset model for EdgeRun Wallet.
//!
//! AssetRef is a reference to an asset.
//! Asset is the full asset record.
//! Canonical asset ID: SYMBOL:network[:contract]

use alloc::string::{String, ToString};

pub use edgerun_protocols::core_protocol::protocol::edgerun_wallet_v0::Asset;
pub use edgerun_protocols::core_protocol::protocol::edgerun_wallet_v0::AssetRef;

/// Build canonical asset ID.
/// Format: SYMBOL:network[:contract]
pub fn canonical_asset_id(symbol: &str, network: &str, contract: Option<&str>) -> String {
    match contract {
        Some(c) => alloc::format!("{}:{}:{}", symbol.to_uppercase(), network.to_lowercase(), c),
        None => alloc::format!("{}:{}", symbol.to_uppercase(), network.to_lowercase()),
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
    fn asset_ref_canonical_id() {
        let asset_ref = AssetRef {
            symbol: "BTC".into(),
            network: "bitcoin".into(),
            contract: None,
            decimals: Some(8),
        };
        assert_eq!(
            canonical_asset_id(
                &asset_ref.symbol,
                &asset_ref.network,
                asset_ref.contract.as_deref()
            ),
            "BTC:bitcoin"
        );
    }
}
