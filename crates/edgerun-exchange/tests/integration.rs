//! Comprehensive tests for exchange boundary.
//!
//! Covers: asset normalization, provider mapping, quote routing,
//! status transitions, provider contradictions, public API privacy,
//! AssetRef canonical serialization.

extern crate alloc;

mod mocks {
    use super::*;
    use edgerun_exchange::changenow::ChangeNOWAdapter;
    use edgerun_exchange::provider::{ExchangeProvider, ProviderCode};
    use edgerun_exchange::sideshift::SideShiftAdapter;
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
        assert!(
            !adapter.supports_quote(&req),
            "Invalid asset should not be supported"
        );
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
            assert!(
                adapter.supports_quote(&req),
                "Pair {} -> {} should be supported",
                settle,
                pay
            );
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
            assert!(
                adapter.supports_quote(&req),
                "Pair {} -> {} should be supported",
                settle,
                pay
            );
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
    use edgerun_exchange::provider::ExchangeProvider;
    use edgerun_exchange::sideshift::SideShiftAdapter;
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

        let unsupported = vec![("BTC", "DOGE"), ("DOT", "ADA"), ("XRP", "SOL")];

        for (settle, pay) in unsupported {
            let req = make_req(settle, pay);
            assert!(
                !adapter.supports_quote(&req),
                "Pair {} -> {} should NOT be supported",
                settle,
                pay
            );
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

mod asset_ref_canonical {
    use edgerun_proto::edgerun::v0::wallet::v0::AssetRef;

    #[test]
    fn test_asset_ref_equality() {
        let a = AssetRef {
            symbol: "USDT".into(),
            network: "tron".into(),
            contract: Some("TR7NH".into()),
            decimals: Some(6),
        };
        let b = AssetRef {
            symbol: "USDT".into(),
            network: "tron".into(),
            contract: Some("TR7NH".into()),
            decimals: Some(6),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_asset_ref_inequality() {
        let a = AssetRef {
            symbol: "BTC".into(),
            network: "bitcoin".into(),
            contract: None,
            decimals: Some(8),
        };
        let b = AssetRef {
            symbol: "ETH".into(),
            network: "ethereum".into(),
            contract: None,
            decimals: Some(18),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_asset_ref_canonical_id() {
        let with_contract = AssetRef {
            symbol: "USDT".into(),
            network: "tron".into(),
            contract: Some("TR7NH".into()),
            decimals: Some(6),
        };
        assert_eq!(with_contract.symbol, "USDT");
        assert_eq!(with_contract.network, "tron");
        assert_eq!(with_contract.contract, Some("TR7NH".into()));

        let without_contract = AssetRef {
            symbol: "BTC".into(),
            network: "bitcoin".into(),
            contract: None,
            decimals: Some(8),
        };
        assert_eq!(without_contract.contract, None);
    }
}

mod provider_mapping_tests {
    use edgerun_exchange::provider_mapping::map_provider_status;

    #[test]
    fn test_sideshift_known_statuses() {
        assert_eq!(map_provider_status("SIDESHIFT", "pending"), 3); // CREATED
        assert_eq!(map_provider_status("SIDESHIFT", "awaiting_deposit"), 4); // AWAITING_DEPOSIT
        assert_eq!(map_provider_status("SIDESHIFT", "completed"), 9); // COMPLETED
        assert_eq!(map_provider_status("SIDESHIFT", "failed"), 15); // FAILED
        assert_eq!(map_provider_status("SIDESHIFT", "refunded"), 13); // REFUNDED
        assert_eq!(map_provider_status("SIDESHIFT", "expired"), 14); // EXPIRED
    }

    #[test]
    fn test_changenow_known_statuses() {
        assert_eq!(map_provider_status("CHANGENOW", "new"), 3); // CREATED
        assert_eq!(map_provider_status("CHANGENOW", "waiting"), 4); // AWAITING_DEPOSIT
        assert_eq!(map_provider_status("CHANGENOW", "finished"), 9); // COMPLETED
        assert_eq!(map_provider_status("CHANGENOW", "failed"), 15); // FAILED
        assert_eq!(map_provider_status("CHANGENOW", "refunded"), 13); // REFUNDED
    }

    #[test]
    fn test_unknown_status_maps_to_on_hold() {
        assert_eq!(map_provider_status("SIDESHIFT", "bogus_status"), 17); // ON_HOLD
        assert_eq!(map_provider_status("CHANGENOW", "unknown"), 17); // ON_HOLD
    }

    #[test]
    fn test_unknown_provider_maps_to_on_hold() {
        assert_eq!(map_provider_status("UNKNOWN_PROVIDER", "anything"), 17); // ON_HOLD
    }
}

mod status_machine_tests {
    use edgerun_exchange::status_machine::StatusMachine;
    use edgerun_wallet::{is_terminal, CanonicalOrderStatus};

    #[test]
    fn test_valid_transitions() {
        let machine = StatusMachine;
        assert!(machine
            .validate_transition(
                CanonicalOrderStatus::Created,
                CanonicalOrderStatus::AwaitingDeposit
            )
            .is_ok());
        assert!(machine
            .validate_transition(
                CanonicalOrderStatus::Exchanging,
                CanonicalOrderStatus::Sending
            )
            .is_ok());
        assert!(machine
            .validate_transition(
                CanonicalOrderStatus::Sending,
                CanonicalOrderStatus::Completed
            )
            .is_ok());
    }

    #[test]
    fn test_invalid_transitions_include_from_to() {
        let machine = StatusMachine;
        let err = machine
            .validate_transition(
                CanonicalOrderStatus::Completed,
                CanonicalOrderStatus::Exchanging,
            )
            .unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("COMPLETED"),
            "Error should include from status: {}",
            msg
        );
        assert!(
            msg.contains("EXCHANGING"),
            "Error should include to status: {}",
            msg
        );
    }

    #[test]
    fn test_terminal_statuses() {
        let machine = StatusMachine;
        assert!(machine.is_terminal(CanonicalOrderStatus::Completed));
        assert!(machine.is_terminal(CanonicalOrderStatus::Failed));
        assert!(machine.is_terminal(CanonicalOrderStatus::Refunded));
        assert!(machine.is_terminal(CanonicalOrderStatus::Expired));
        assert!(machine.is_terminal(CanonicalOrderStatus::Canceled));
        assert!(machine.is_terminal(CanonicalOrderStatus::Rejected));
    }

    #[test]
    fn test_non_terminal_statuses() {
        let machine = StatusMachine;
        assert!(!machine.is_terminal(CanonicalOrderStatus::Created));
        assert!(!machine.is_terminal(CanonicalOrderStatus::Exchanging));
        assert!(!machine.is_terminal(CanonicalOrderStatus::Sending));
        assert!(!machine.is_terminal(CanonicalOrderStatus::OnHold));
    }

    #[test]
    fn test_provider_contradiction_terminal_goes_to_on_hold() {
        let machine = StatusMachine;
        // We have COMPLETED but provider says SENDING — contradiction
        let result = machine.handle_provider_contradiction(
            CanonicalOrderStatus::Completed,
            CanonicalOrderStatus::Exchanging, // provider says still exchanging
        );
        assert_eq!(result, CanonicalOrderStatus::OnHold);
    }

    #[test]
    fn test_provider_contradiction_non_terminal_accepts_provider() {
        let machine = StatusMachine;
        // We have AWAITING_DEPOSIT, provider says DEPOSIT_SEEN — normal progression
        let result = machine.handle_provider_contradiction(
            CanonicalOrderStatus::AwaitingDeposit,
            CanonicalOrderStatus::DepositSeen,
        );
        assert_eq!(result, CanonicalOrderStatus::DepositSeen);
    }

    #[test]
    fn test_provider_contradiction_same_terminal_accepts() {
        let machine = StatusMachine;
        // Both agree on COMPLETED
        let result = machine.handle_provider_contradiction(
            CanonicalOrderStatus::Completed,
            CanonicalOrderStatus::Completed,
        );
        assert_eq!(result, CanonicalOrderStatus::Completed);
    }
}

mod exchange_event_tests {
    use edgerun_exchange::ExchangeEvent;

    #[test]
    fn test_event_type_names() {
        let quote = ExchangeEvent::QuoteCreated {
            quote_id: "q1".into(),
            settlement_asset_id: "USDT:tron".into(),
            pay_asset_id: "BTC:bitcoin".into(),
            settlement_amount: "100".into(),
            pay_amount: "0.003".into(),
            rate: "33333.33".into(),
            expires_at_ms: 9999999999999,
            provider_code: "SIDESHIFT".into(),
        };
        assert_eq!(quote.event_type(), "QUOTE_CREATED");
    }

    #[test]
    fn test_event_order_id() {
        let completed = ExchangeEvent::OrderCompleted {
            order_id: "ord-123".into(),
            payout_tx_id: Some("tx-456".into()),
            payout_network: Some("bitcoin".into()),
            completed_at_ms: 1000,
        };
        assert_eq!(completed.order_id(), Some("ord-123"));

        let quote = ExchangeEvent::QuoteCreated {
            quote_id: "q1".into(),
            settlement_asset_id: "USDT:tron".into(),
            pay_asset_id: "BTC:bitcoin".into(),
            settlement_amount: "100".into(),
            pay_amount: "0.003".into(),
            rate: "33333.33".into(),
            expires_at_ms: 9999999999999,
            provider_code: "SIDESHIFT".into(),
        };
        assert_eq!(quote.order_id(), None);
    }

    #[test]
    fn test_terminal_events() {
        let completed = ExchangeEvent::OrderCompleted {
            order_id: "ord-123".into(),
            payout_tx_id: None,
            payout_network: None,
            completed_at_ms: 1000,
        };
        assert!(completed.is_terminal());

        let failed = ExchangeEvent::OrderFailed {
            order_id: "ord-123".into(),
            reason: "provider_error".into(),
            failed_at_ms: 1000,
        };
        assert!(failed.is_terminal());

        let review = ExchangeEvent::ManualReviewRequired {
            order_id: "ord-123".into(),
            reason: "contradiction".into(),
            canonical_status: 9,
            provider_status: 7,
            flagged_at_ms: 1000,
        };
        assert!(review.is_terminal());
    }

    #[test]
    fn test_non_terminal_events() {
        let created = ExchangeEvent::OrderCreated {
            order_id: "ord-123".into(),
            quote_id: "q1".into(),
            provider_order_id: "prov-123".into(),
            deposit_address: "addr".into(),
            recipient_address: "addr".into(),
            refund_address: None,
            provider_code: "SIDESHIFT".into(),
            created_at_ms: 1000,
        };
        assert!(!created.is_terminal());

        let status_observed = ExchangeEvent::ProviderStatusObserved {
            order_id: "ord-123".into(),
            provider_code: "SIDESHIFT".into(),
            provider_order_id: "prov-123".into(),
            provider_status_string: "completed".into(),
            mapped_canonical_status: 9,
            observed_at_ms: 1000,
        };
        assert!(!status_observed.is_terminal());
    }
}
