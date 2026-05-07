//! Protocol-native settlement intents for direct node routing.
//!
//! HTTP provider APIs are upstream adapters only. Inside EdgeRun, marketplace
//! settlement requests move as signed `CommandEnvelope` values targeted at a
//! node identity, carrying rkyv archives of concrete wallet protocol types.

extern crate alloc;

use alloc::vec::Vec;

use edgerun_protocols::core_protocol::protocol::edgerun_wallet_v0::{
    Order, PaymentRequest, Quote, QuoteRequest, Receipt,
};
use edgerun_protocols::core_protocol::protocol::{
    command_envelope, AppIntent, CommandEnvelope, IdentityRef, NodeRef, Timestamp,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SettlementCommandDraft {
    pub command_id: Vec<u8>,
    pub target_node: NodeRef,
    pub issuer: Option<IdentityRef>,
    pub command_type: i32,
    pub issued_at: Option<Timestamp>,
    pub expires_at: Option<Timestamp>,
    pub idempotency_key: Vec<u8>,
    pub app_intent: AppIntent,
    pub payload_bytes: Vec<u8>,
}

pub fn archive_payment_request_intent(request: &PaymentRequest) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(request)
        .expect("payment request must serialize through the rkyv wire boundary")
        .into_vec()
}

pub fn archive_quote_request_intent(request: &QuoteRequest) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(request)
        .expect("quote request must serialize through the rkyv wire boundary")
        .into_vec()
}

pub fn archive_quote_intent(quote: &Quote) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(quote)
        .expect("quote must serialize through the rkyv wire boundary")
        .into_vec()
}

pub fn archive_order_intent(order: &Order) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(order)
        .expect("order must serialize through the rkyv wire boundary")
        .into_vec()
}

pub fn archive_receipt_intent(receipt: &Receipt) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(receipt)
        .expect("receipt must serialize through the rkyv wire boundary")
        .into_vec()
}

pub fn build_app_intent(app_id: Vec<u8>, payload: Vec<u8>, signature: Vec<u8>) -> AppIntent {
    AppIntent {
        app_id,
        payload,
        signature,
    }
}

pub fn encode_app_intent(intent: &AppIntent) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(intent)
        .expect("app intent must serialize through the rkyv wire boundary")
        .into_vec()
}

pub fn build_identity_routed_settlement_command(draft: SettlementCommandDraft) -> CommandEnvelope {
    CommandEnvelope {
        envelope_version: 1,
        command_id: draft.command_id,
        target_node: Some(draft.target_node),
        issuer: draft.issuer,
        command_type: draft.command_type,
        command_version: 1,
        issued_at: draft.issued_at,
        not_before: None,
        expires_at: draft.expires_at,
        idempotency_key: draft.idempotency_key,
        delegation_chain: Vec::new(),
        requested_assurance: None,
        command_metadata: None,
        signatures: Vec::new(),
        app_intent: encode_app_intent(&draft.app_intent),
        payload: Some(command_envelope::Payload::InlinePayload(
            draft.payload_bytes,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_protocols::core_protocol::protocol::CommandType;

    fn sample_payment_request() -> PaymentRequest {
        PaymentRequest {
            request_id: "pr-test".into(),
            settlement_asset_id: "USDT:tron".into(),
            settlement_amount: "100.00".into(),
            recipient_address: Some("seller-settlement-address".into()),
            description: Some("store order 42".into()),
            created_at_ms: 1_700_000_000_000,
            expires_at_ms: 1_700_000_900_000,
            pay_asset_id: Some("BTC:bitcoin".into()),
        }
    }

    #[test]
    fn payment_request_intent_is_direct_rkyv_wallet_type() {
        let request = sample_payment_request();
        let payload = archive_payment_request_intent(&request);

        let decoded = edgerun_protocols::wire::from_bytes::<
            PaymentRequest,
            edgerun_protocols::wire::WireError,
        >(&payload)
        .expect("payment request archive should decode");

        assert_eq!(decoded, request);
    }

    #[test]
    fn settlement_command_targets_node_and_carries_app_intent() {
        let request = sample_payment_request();
        let payload = archive_payment_request_intent(&request);
        let app_intent = build_app_intent(
            b"marketplace-app".to_vec(),
            payload.clone(),
            b"app-signature-placeholder".to_vec(),
        );

        let command = build_identity_routed_settlement_command(SettlementCommandDraft {
            command_id: b"cmd-settlement-1".to_vec(),
            target_node: NodeRef {
                node_id: b"seller-node".to_vec(),
            },
            issuer: Some(IdentityRef {
                identity_id: b"buyer-identity".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            command_type: CommandType::StoreAndForward as i32,
            issued_at: Some(Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            expires_at: Some(Timestamp {
                seconds: 1_700_000_900,
                nanos: 0,
            }),
            idempotency_key: b"order-42".to_vec(),
            app_intent,
            payload_bytes: payload.clone(),
        });

        assert_eq!(
            command.target_node.unwrap().node_id,
            b"seller-node".to_vec()
        );
        assert_eq!(
            command.issuer.unwrap().identity_id,
            b"buyer-identity".to_vec()
        );

        let decoded_intent = edgerun_protocols::wire::from_bytes::<
            AppIntent,
            edgerun_protocols::wire::WireError,
        >(&command.app_intent)
        .expect("app intent archive should decode");
        assert_eq!(decoded_intent.app_id, b"marketplace-app".to_vec());
        assert_eq!(decoded_intent.payload, payload);

        match command.payload {
            Some(command_envelope::Payload::InlinePayload(bytes)) => {
                let decoded = edgerun_protocols::wire::from_bytes::<
                    PaymentRequest,
                    edgerun_protocols::wire::WireError,
                >(&bytes)
                .expect("inline wallet payload should decode");
                assert_eq!(decoded, request);
            }
            _ => panic!("settlement command should carry inline rkyv wallet payload"),
        }
    }
}
