//! Codec between exchange domain events and existing protocol stream events.
//!
//! This module deliberately does not introduce a new event sink, store, or
//! persistence abstraction. It only defines the boundary:
//!
//! ```text
//! ExchangeEvent -> existing edgerun.v0.stream.EventType + protobuf payload bytes
//! existing edgerun.v0.stream.EventType + payload bytes -> ExchangeEvent
//! ```
//!
//! The caller remains responsible for storing payload bytes as an object and
//! appending an `EventEnvelope` through the existing stream writer.

extern crate alloc;

use alloc::vec::Vec;

use prost::Message;

use edgerun_proto::edgerun::v0::common::{Digest, ObjectKind, ObjectRef};
use edgerun_proto::edgerun::v0::stream::{EventEnvelope, EventType};
use edgerun_proto::edgerun::v0::wallet::v0::{
    WalletDepositObservedPayload, WalletManualReviewRequiredPayload,
    WalletOrderCompletedPayload, WalletOrderCreatedPayload, WalletOrderFailedPayload,
    WalletOrderStatusChangedPayload, WalletProviderStatusObservedPayload,
    WalletQuoteCreatedPayload,
};

use crate::events::ExchangeEvent;

/// Encoded exchange stream payload plus the existing protocol event type that
/// should be placed in `EventEnvelope.event_type`.
#[derive(Debug, Clone)]
pub struct ExchangeStreamPayload {
    pub event_type: i32,
    pub payload_bytes: Vec<u8>,
}

fn encode_message<M: Message>(message: &M) -> Vec<u8> {
    let mut out = Vec::with_capacity(message.encoded_len());
    message.encode(&mut out).expect("encoding to Vec cannot fail");
    out
}

/// Object kind that exchange payload bytes should use when stored as an
/// `ObjectRef` for `EventEnvelope.payload_object`.
pub fn exchange_payload_object_kind() -> i32 {
    ObjectKind::Payload as i32
}

/// Return the existing stream event type for an exchange event.
pub fn exchange_event_type(event: &ExchangeEvent) -> i32 {
    match event {
        ExchangeEvent::QuoteCreated { .. } => EventType::WalletQuoteCreated as i32,
        ExchangeEvent::OrderCreated { .. } => EventType::WalletOrderCreated as i32,
        ExchangeEvent::DepositObserved { .. } => EventType::WalletDepositObserved as i32,
        ExchangeEvent::ProviderStatusObserved { .. } => {
            EventType::WalletProviderStatusObserved as i32
        }
        ExchangeEvent::OrderStatusChanged { .. } => EventType::WalletOrderStatusChanged as i32,
        ExchangeEvent::OrderCompleted { .. } => EventType::WalletOrderCompleted as i32,
        ExchangeEvent::OrderFailed { .. } => EventType::WalletOrderFailed as i32,
        ExchangeEvent::ManualReviewRequired { .. } => {
            EventType::WalletManualReviewRequired as i32
        }
    }
}

/// Build the existing protocol event envelope for an exchange event.
///
/// `payload_object` should point at an object containing `encode_exchange_event(event).payload_bytes`.
/// The returned envelope is unsigned and unappended; callers must sign/append it
/// through the existing stream writer.
pub fn build_exchange_event_envelope(
    event: &ExchangeEvent,
    stream_id: Vec<u8>,
    seq: u64,
    prev_event_hash: Option<Digest>,
    payload_object: ObjectRef,
) -> EventEnvelope {
    EventEnvelope {
        envelope_version: 1,
        stream_id,
        seq,
        prev_event_hash,
        event_type: exchange_event_type(event),
        event_version: 1,
        recorded_at: None,
        effective_at: None,
        payload_object: Some(payload_object.clone()),
        related_events: Vec::new(),
        related_commands: Vec::new(),
        related_objects: vec![payload_object],
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    }
}

/// Encode an exchange event into protobuf payload bytes for the existing event
/// stream. The returned `event_type` is the value for `EventEnvelope.event_type`.
pub fn encode_exchange_event(event: &ExchangeEvent) -> ExchangeStreamPayload {
    let payload_bytes = match event {
        ExchangeEvent::QuoteCreated {
            quote_id,
            settlement_asset_id,
            pay_asset_id,
            settlement_amount,
            pay_amount,
            rate,
            expires_at_ms,
            provider_code,
        } => encode_message(&WalletQuoteCreatedPayload {
            payload_version: 1,
            quote_id: quote_id.clone(),
            settlement_asset_id: settlement_asset_id.clone(),
            pay_asset_id: pay_asset_id.clone(),
            settlement_amount: settlement_amount.clone(),
            pay_amount: pay_amount.clone(),
            rate: rate.clone(),
            expires_at_ms: *expires_at_ms,
            provider_code_internal: Some(provider_code.clone()),
        }),
        ExchangeEvent::OrderCreated {
            order_id,
            quote_id,
            provider_order_id,
            deposit_address,
            recipient_address,
            refund_address,
            provider_code,
            created_at_ms,
        } => encode_message(&WalletOrderCreatedPayload {
            payload_version: 1,
            order_id: order_id.clone(),
            quote_id: quote_id.clone(),
            provider_order_id_internal: provider_order_id.clone(),
            deposit_address: deposit_address.clone(),
            recipient_address: recipient_address.clone(),
            refund_address: refund_address.clone(),
            provider_code_internal: Some(provider_code.clone()),
            created_at_ms: *created_at_ms,
        }),
        ExchangeEvent::DepositObserved {
            order_id,
            tx_id,
            network,
            amount,
            confirmations,
            observed_at_ms,
        } => encode_message(&WalletDepositObservedPayload {
            payload_version: 1,
            order_id: order_id.clone(),
            tx_id: tx_id.clone(),
            network: network.clone(),
            amount: amount.clone(),
            confirmations: *confirmations,
            observed_at_ms: *observed_at_ms,
        }),
        ExchangeEvent::ProviderStatusObserved {
            order_id,
            provider_code,
            provider_order_id,
            provider_status_string,
            mapped_canonical_status,
            observed_at_ms,
        } => encode_message(&WalletProviderStatusObservedPayload {
            payload_version: 1,
            order_id: order_id.clone(),
            provider_code_internal: Some(provider_code.clone()),
            provider_order_id_internal: provider_order_id.clone(),
            provider_status_string: provider_status_string.clone(),
            mapped_canonical_status: *mapped_canonical_status,
            observed_at_ms: *observed_at_ms,
        }),
        ExchangeEvent::OrderStatusChanged {
            order_id,
            from_status,
            to_status,
            reason,
            changed_at_ms,
        } => encode_message(&WalletOrderStatusChangedPayload {
            payload_version: 1,
            order_id: order_id.clone(),
            from_status: *from_status,
            to_status: *to_status,
            reason: reason.clone(),
            changed_at_ms: *changed_at_ms,
        }),
        ExchangeEvent::OrderCompleted {
            order_id,
            payout_tx_id,
            payout_network,
            completed_at_ms,
        } => encode_message(&WalletOrderCompletedPayload {
            payload_version: 1,
            order_id: order_id.clone(),
            payout_tx_id: payout_tx_id.clone(),
            payout_network: payout_network.clone(),
            completed_at_ms: *completed_at_ms,
        }),
        ExchangeEvent::OrderFailed {
            order_id,
            reason,
            failed_at_ms,
        } => encode_message(&WalletOrderFailedPayload {
            payload_version: 1,
            order_id: order_id.clone(),
            reason: reason.clone(),
            failed_at_ms: *failed_at_ms,
        }),
        ExchangeEvent::ManualReviewRequired {
            order_id,
            reason,
            canonical_status,
            provider_status,
            flagged_at_ms,
        } => encode_message(&WalletManualReviewRequiredPayload {
            payload_version: 1,
            order_id: order_id.clone(),
            reason: reason.clone(),
            canonical_status: *canonical_status,
            provider_status: *provider_status,
            flagged_at_ms: *flagged_at_ms,
        }),
    };

    ExchangeStreamPayload {
        event_type: exchange_event_type(event),
        payload_bytes,
    }
}

/// Decode a protobuf payload object body from the existing event stream into an
/// `ExchangeEvent`. Unknown/non-wallet event types return `None`.
pub fn decode_exchange_event(event_type: i32, payload_bytes: &[u8]) -> Option<ExchangeEvent> {
    let event_type = EventType::try_from(event_type).ok()?;
    match event_type {
        EventType::WalletQuoteCreated => {
            let payload = WalletQuoteCreatedPayload::decode(payload_bytes).ok()?;
            Some(ExchangeEvent::QuoteCreated {
                quote_id: payload.quote_id,
                settlement_asset_id: payload.settlement_asset_id,
                pay_asset_id: payload.pay_asset_id,
                settlement_amount: payload.settlement_amount,
                pay_amount: payload.pay_amount,
                rate: payload.rate,
                expires_at_ms: payload.expires_at_ms,
                provider_code: payload.provider_code_internal.unwrap_or_default(),
            })
        }
        EventType::WalletOrderCreated => {
            let payload = WalletOrderCreatedPayload::decode(payload_bytes).ok()?;
            Some(ExchangeEvent::OrderCreated {
                order_id: payload.order_id,
                quote_id: payload.quote_id,
                provider_order_id: payload.provider_order_id_internal,
                deposit_address: payload.deposit_address,
                recipient_address: payload.recipient_address,
                refund_address: payload.refund_address,
                provider_code: payload.provider_code_internal.unwrap_or_default(),
                created_at_ms: payload.created_at_ms,
            })
        }
        EventType::WalletDepositObserved => {
            let payload = WalletDepositObservedPayload::decode(payload_bytes).ok()?;
            Some(ExchangeEvent::DepositObserved {
                order_id: payload.order_id,
                tx_id: payload.tx_id,
                network: payload.network,
                amount: payload.amount,
                confirmations: payload.confirmations,
                observed_at_ms: payload.observed_at_ms,
            })
        }
        EventType::WalletProviderStatusObserved => {
            let payload = WalletProviderStatusObservedPayload::decode(payload_bytes).ok()?;
            Some(ExchangeEvent::ProviderStatusObserved {
                order_id: payload.order_id,
                provider_code: payload.provider_code_internal.unwrap_or_default(),
                provider_order_id: payload.provider_order_id_internal,
                provider_status_string: payload.provider_status_string,
                mapped_canonical_status: payload.mapped_canonical_status,
                observed_at_ms: payload.observed_at_ms,
            })
        }
        EventType::WalletOrderStatusChanged => {
            let payload = WalletOrderStatusChangedPayload::decode(payload_bytes).ok()?;
            Some(ExchangeEvent::OrderStatusChanged {
                order_id: payload.order_id,
                from_status: payload.from_status,
                to_status: payload.to_status,
                reason: payload.reason,
                changed_at_ms: payload.changed_at_ms,
            })
        }
        EventType::WalletOrderCompleted => {
            let payload = WalletOrderCompletedPayload::decode(payload_bytes).ok()?;
            Some(ExchangeEvent::OrderCompleted {
                order_id: payload.order_id,
                payout_tx_id: payload.payout_tx_id,
                payout_network: payload.payout_network,
                completed_at_ms: payload.completed_at_ms,
            })
        }
        EventType::WalletOrderFailed => {
            let payload = WalletOrderFailedPayload::decode(payload_bytes).ok()?;
            Some(ExchangeEvent::OrderFailed {
                order_id: payload.order_id,
                reason: payload.reason,
                failed_at_ms: payload.failed_at_ms,
            })
        }
        EventType::WalletManualReviewRequired => {
            let payload = WalletManualReviewRequiredPayload::decode(payload_bytes).ok()?;
            Some(ExchangeEvent::ManualReviewRequired {
                order_id: payload.order_id,
                reason: payload.reason,
                canonical_status: payload.canonical_status,
                provider_status: payload.provider_status,
                flagged_at_ms: payload.flagged_at_ms,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(event: ExchangeEvent) -> ExchangeEvent {
        let encoded = encode_exchange_event(&event);
        decode_exchange_event(encoded.event_type, &encoded.payload_bytes).expect("decode event")
    }

    #[test]
    fn quote_created_roundtrips_through_stream_payload() {
        let event = ExchangeEvent::QuoteCreated {
            quote_id: "eq-1".into(),
            settlement_asset_id: "USDT:tron".into(),
            pay_asset_id: "BTC:bitcoin".into(),
            settlement_amount: "100".into(),
            pay_amount: "0.001".into(),
            rate: "100000".into(),
            expires_at_ms: 123,
            provider_code: "SIDESHIFT".into(),
        };
        assert!(matches!(roundtrip(event), ExchangeEvent::QuoteCreated { .. }));
    }

    #[test]
    fn order_created_roundtrips_through_stream_payload() {
        let event = ExchangeEvent::OrderCreated {
            order_id: "ex-1".into(),
            quote_id: "eq-1".into(),
            provider_order_id: "provider-1".into(),
            deposit_address: "deposit".into(),
            recipient_address: "recipient".into(),
            refund_address: Some("refund".into()),
            provider_code: "CHANGENOW".into(),
            created_at_ms: 456,
        };
        assert!(matches!(roundtrip(event), ExchangeEvent::OrderCreated { .. }));
    }

    #[test]
    fn status_changed_maps_to_existing_wallet_event_type() {
        let event = ExchangeEvent::OrderStatusChanged {
            order_id: "ex-1".into(),
            from_status: 3,
            to_status: 4,
            reason: "provider_initial_status".into(),
            changed_at_ms: 789,
        };
        let encoded = encode_exchange_event(&event);
        assert_eq!(encoded.event_type, EventType::WalletOrderStatusChanged as i32);
        assert!(matches!(decode_exchange_event(encoded.event_type, &encoded.payload_bytes), Some(ExchangeEvent::OrderStatusChanged { .. })));
    }

    #[test]
    fn envelope_uses_existing_event_type_and_payload_object() {
        let event = ExchangeEvent::OrderFailed {
            order_id: "ex-1".into(),
            reason: "provider_failed".into(),
            failed_at_ms: 999,
        };
        let payload_object = ObjectRef {
            object_id: vec![1, 2, 3],
            object_kind: Some(exchange_payload_object_kind()),
        };
        let envelope = build_exchange_event_envelope(
            &event,
            vec![9, 9, 9],
            7,
            None,
            payload_object.clone(),
        );
        assert_eq!(envelope.event_type, EventType::WalletOrderFailed as i32);
        assert_eq!(envelope.payload_object, Some(payload_object.clone()));
        assert_eq!(envelope.related_objects, vec![payload_object]);
    }
}
