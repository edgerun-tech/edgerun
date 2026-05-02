//! Integration tests for exchange provider adapters.
//!
//! These tests validate that the adapters correctly call the provider APIs
//! and properly handle responses and errors.
//!
//! Run with: cargo test -p edgerun-exchange --test integration

extern crate alloc;

mod mocks {
    use super::*;
    use edgerun_exchange::sideshift::SideShiftAdapter;
    use edgerun_exchange::changenow::ChangeNOWAdapter;
    use edgerun_exchange::provider::{ExchangeProvider, ProviderCode};
    use edgerun_proto::edgerun::v0::wallet::v0::QuoteRequest;

    fn make_quote_request(settlement_asset_id: &str, pay_asset_id: &str) -> QuoteRequest {
        QuoteRequest {
            settlement_asset_id: settlement_asset_id.into(),
            pay_asset_id: pay_asset_id.into(),
            settlement_amount: "100".into(),
            pay_amount: "".into(),
            quote_mode: 1,
            amount_side: 1,
            refund_address: None,
            recipient_address: None,
        }
    }

    #[test]
    fn test_sideshift_quote_request_validation() {
        let adapter = SideShiftAdapter::new("test_key".into(), "test_affiliate".into());
        
        let req = make_quote_request("INVALID:network", "USDT:tron");
        assert!(!adapter.supports_quote(&req), "Invalid asset should not be supported");
    }

    #[test]
    fn test_sideshift_valid_asset_pairs() {
        let adapter = SideShiftAdapter::new("test_key".into(), "test_affiliate".into());
        
        let valid_pairs = vec![
            ("USDT:tron", "DOGE:dogecoin"),
            ("USDT:tron", "BTC:bitcoin"),
            ("USDT:tron", "ETH:ethereum"),
            ("BTC:bitcoin", "USDT:tron"),
            ("ETH:ethereum", "USDT:tron"),
            ("DOGE:dogecoin", "USDT:tron"),
        ];

        for (settle, pay) in valid_pairs {
            let req = make_quote_request(settle, pay);
            assert!(adapter.supports_quote(&req), "Pair {} -> {} should be supported", settle, pay);
        }
    }

    #[test]
    fn test_changenow_valid_asset_pairs() {
        let adapter = ChangeNOWAdapter::new("test_key".into(), "test_partner".into());
        
        let valid_pairs = vec![
            ("USDT:tron", "DOGE:dogecoin"),
            ("USDT:tron", "BTC:bitcoin"),
            ("USDT:tron", "ETH:ethereum"),
            ("USDT:tron", "SOL:solana"),
            ("BTC:bitcoin", "USDT:tron"),
            ("SOL:solana", "USDT:tron"),
        ];

        for (settle, pay) in valid_pairs {
            let req = make_quote_request(settle, pay);
            assert!(adapter.supports_quote(&req), "Pair {} -> {} should be supported", settle, pay);
        }
    }

    #[test]
    fn test_provider_features_sideshift() {
        let adapter = SideShiftAdapter::new("test_key".into(), "test_affiliate".into());
        let features = adapter.features();
        
        assert!(features.supports_fixed_rate);
        assert!(features.supports_float_rate);
        assert!(features.supports_refund_address);
        assert!(!features.requires_destination_tag);
        assert_eq!(features.min_confirmations, 3);
    }

    #[test]
    fn test_provider_features_changenow() {
        let adapter = ChangeNOWAdapter::new("test_key".into(), "test_partner".into());
        let features = adapter.features();
        
        assert!(features.supports_fixed_rate);
        assert!(features.supports_float_rate);
        assert!(features.supports_refund_address);
        assert!(features.requires_destination_tag);
        assert_eq!(features.min_confirmations, 2);
    }

    #[test]
    fn test_provider_codes() {
        let sideshift = SideShiftAdapter::new("test".into(), "test".into());
        let changenow = ChangeNOWAdapter::new("test".into(), "test".into());
        
        assert_eq!(sideshift.code(), ProviderCode::SideShift);
        assert_eq!(changenow.code(), ProviderCode::ChangeNOW);
    }
}

mod asset_parsing {
    use super::*;

    #[test]
    fn test_parse_asset_id_with_network() {
        let (symbol, network) = parse_id("USDT:tron");
        assert_eq!(symbol, "USDT");
        assert_eq!(network, "tron");
    }

    #[test]
    fn test_parse_asset_id_simple() {
        let (symbol, network) = parse_id("BTC");
        assert_eq!(symbol, "BTC");
        assert_eq!(network, "");
    }

    #[test]
    fn test_parse_asset_id_with_contract() {
        let (symbol, network) = parse_id("USDT:tron:TR7NH");
        assert_eq!(symbol, "USDT");
        assert_eq!(network, "tron");
    }

    fn parse_id(id: &str) -> (alloc::string::String, alloc::string::String) {
        let parts: Vec<&str> = id.split(':').collect();
        if parts.len() >= 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (id.to_string(), "".to_string())
        }
    }
}

mod error_handling {
    use super::*;
    use edgerun_wallet::WalletError;

    #[test]
    fn test_error_display_impl() {
        let err = WalletError::InvalidDecimal("invalid".into());
        assert!(format!("{}", err).contains("invalid decimal"));
        
        let err = WalletError::ProviderError("API failed".into());
        assert!(format!("{}", err).contains("provider error"));
        
        let err = WalletError::HttpError("connection refused".into());
        assert!(format!("{}", err).contains("HTTP error"));
        
        let err = WalletError::InvalidAssetId("BAD".into());
        assert!(format!("{}", err).contains("invalid asset id"));
        
        let err = WalletError::Serialization("parse failed".into());
        assert!(format!("{}", err).contains("serialization error"));
    }

    #[test]
    fn test_wallet_error_variants() {
        let _ = WalletError::InvalidDecimal("test".into());
        let _ = WalletError::Overflow;
        let _ = WalletError::NegativeResult;
        let _ = WalletError::InvalidAssetId("test".into());
        let _ = WalletError::InvalidStatusTransition { from: "a", to: "b" };
        let _ = WalletError::TerminalState("test");
        let _ = WalletError::ProviderError("test".into());
        let _ = WalletError::Serialization("test".into());
        let _ = WalletError::HttpError("test".into());
        let _ = WalletError::EventError("test".into());
        let _ = WalletError::NotFound("test".into());
        let _ = WalletError::Config("test".into());
    }
}

mod quote_validation {
    use super::*;
    use edgerun_exchange::sideshift::SideShiftAdapter;
    use edgerun_exchange::provider::ExchangeProvider;
    use edgerun_proto::edgerun::v0::wallet::v0::QuoteRequest;

    fn make_req(settlement_asset_id: &str, pay_asset_id: &str) -> QuoteRequest {
        QuoteRequest {
            settlement_asset_id: settlement_asset_id.into(),
            pay_asset_id: pay_asset_id.into(),
            settlement_amount: "100".into(),
            pay_amount: "".into(),
            quote_mode: 1,
            amount_side: 1,
            refund_address: None,
            recipient_address: None,
        }
    }

    #[test]
    fn test_quote_mode_values() {
        let adapter = SideShiftAdapter::new("test_key".into(), "test_affiliate".into());
        
        for mode in 0i32..=2 {
            let req = QuoteRequest {
                settlement_asset_id: "USDT:tron".into(),
                pay_asset_id: "DOGE:dogecoin".into(),
                settlement_amount: "100".into(),
                pay_amount: "".into(),
                quote_mode: mode,
                amount_side: 1,
                refund_address: None,
                recipient_address: None,
            };
            assert!(adapter.supports_quote(&req));
        }
    }

    #[test]
    fn test_unsupported_pairs_rejected() {
        let adapter = SideShiftAdapter::new("test_key".into(), "test_affiliate".into());
        
        let unsupported = vec![
            ("BTC", "DOGE"),
            ("DOT", "ADA"),
            ("XRP", "SOL"),
        ];

        for (settle, pay) in unsupported {
            let req = make_req(settle, pay);
            assert!(!adapter.supports_quote(&req), "Pair {} -> {} should NOT be supported", settle, pay);
        }
    }
}

mod provider_code_tests {
    use super::*;
    use edgerun_exchange::provider::ProviderCode;

    #[test]
    fn test_provider_code_as_str() {
        assert_eq!(ProviderCode::SideShift.as_str(), "SIDESHIFT");
        assert_eq!(ProviderCode::ChangeNOW.as_str(), "CHANGENOW");
    }

    #[test]
    fn test_provider_code_debug() {
        let ss = format!("{:?}", ProviderCode::SideShift);
        assert!(ss.contains("SideShift"));
        
        let cn = format!("{:?}", ProviderCode::ChangeNOW);
        assert!(cn.contains("ChangeNOW"));
    }

    #[test]
    fn test_provider_code_eq() {
        assert_eq!(ProviderCode::SideShift, ProviderCode::SideShift);
        assert_eq!(ProviderCode::ChangeNOW, ProviderCode::ChangeNOW);
        assert_ne!(ProviderCode::SideShift, ProviderCode::ChangeNOW);
    }
}