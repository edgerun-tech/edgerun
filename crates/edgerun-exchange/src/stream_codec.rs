//! Codec between exchange domain events and the existing protocol event stream.
//!
//! This module does not introduce a new event sink, store, or persistence path.
//! It only defines the boundary:
//!
//! ```text
//! ExchangeEvent -> existing edgerun.v0.stream.EventType + WalletExchangeEventPayload bytes
//! existing EventEnvelope.event_type + payload object bytes -> ExchangeEvent
//! ```
//!
//! Callers store `payload_bytes` as a normal protocol object and append the
//! returned event type through the existing `EventEnvelope` stream writer.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use edgerun_core::protocol::edgerun_wallet_v0::{
    wallet_exchange_event_payload, WalletDepositObservedPayload, WalletExchangeEventPayload,
    WalletManualReviewRequiredPayload, WalletOrderCompletedPayload, WalletOrderCreatedPayload,
    WalletOrderFailedPayload, WalletOrderStatusChangedPayload, WalletProviderStatusObservedPayload,
    WalletQuoteCreatedPayload,
};
use edgerun_core::protocol::{Digest, ObjectKind, ObjectRef};
use edgerun_core::protocol::{EventEnvelope, EventType};

use crate::events::ExchangeEvent;

#[derive(Debug, Clone)]
pub struct ExchangeStreamPayload {
    /// Existing protocol `EventEnvelope.event_type` value.
    pub event_type: i32,
    /// Rkyv-normalized native `WalletExchangeEventPayload` bytes.
    pub payload_bytes: Vec<u8>,
}

const EXCHANGE_PAYLOAD_MAGIC: &[u8; 4] = b"ERXE";

fn put_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_string(out: &mut Vec<u8>, value: &str) {
    put_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

fn put_option_string(out: &mut Vec<u8>, value: &Option<String>) {
    match value {
        Some(value) => {
            put_u8(out, 1);
            put_string(out, value);
        }
        None => put_u8(out, 0),
    }
}

struct PayloadReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> PayloadReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(len)?;
        let chunk = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(chunk)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(*self.take(1)?.first()?)
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    fn string(&mut self) -> Option<String> {
        let len = self.u32()? as usize;
        String::from_utf8(self.take(len)?.to_vec()).ok()
    }

    fn option_string(&mut self) -> Option<Option<String>> {
        match self.u8()? {
            0 => Some(None),
            1 => Some(Some(self.string()?)),
            _ => None,
        }
    }

    fn done(&self) -> bool {
        self.pos == self.bytes.len()
    }
}

fn encode_exchange_payload(payload: &WalletExchangeEventPayload) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(EXCHANGE_PAYLOAD_MAGIC);
    put_u32(&mut out, payload.payload_version);

    let tag: u8 = match payload.event.as_ref() {
        None => 0,
        Some(wallet_exchange_event_payload::Event::QuoteCreated(_)) => 1,
        Some(wallet_exchange_event_payload::Event::OrderCreated(_)) => 2,
        Some(wallet_exchange_event_payload::Event::DepositObserved(_)) => 3,
        Some(wallet_exchange_event_payload::Event::ProviderStatusObserved(_)) => 4,
        Some(wallet_exchange_event_payload::Event::OrderStatusChanged(_)) => 5,
        Some(wallet_exchange_event_payload::Event::OrderCompleted(_)) => 6,
        Some(wallet_exchange_event_payload::Event::OrderFailed(_)) => 7,
        Some(wallet_exchange_event_payload::Event::ManualReviewRequired(_)) => 8,
    };
    put_u8(&mut out, tag);
    match payload.event.as_ref() {
        None => {}
        Some(wallet_exchange_event_payload::Event::QuoteCreated(payload)) => {
            put_u32(&mut out, payload.payload_version);
            put_string(&mut out, &payload.quote_id);
            put_string(&mut out, &payload.settlement_asset_id);
            put_string(&mut out, &payload.pay_asset_id);
            put_string(&mut out, &payload.settlement_amount);
            put_string(&mut out, &payload.pay_amount);
            put_string(&mut out, &payload.rate);
            put_u64(&mut out, payload.expires_at_ms);
            put_option_string(&mut out, &payload.provider_code_internal);
        }
        Some(wallet_exchange_event_payload::Event::OrderCreated(payload)) => {
            put_u32(&mut out, payload.payload_version);
            put_string(&mut out, &payload.order_id);
            put_string(&mut out, &payload.quote_id);
            put_string(&mut out, &payload.provider_order_id_internal);
            put_string(&mut out, &payload.deposit_address);
            put_string(&mut out, &payload.recipient_address);
            put_option_string(&mut out, &payload.refund_address);
            put_option_string(&mut out, &payload.provider_code_internal);
            put_u64(&mut out, payload.created_at_ms);
        }
        Some(wallet_exchange_event_payload::Event::DepositObserved(payload)) => {
            put_u32(&mut out, payload.payload_version);
            put_string(&mut out, &payload.order_id);
            put_string(&mut out, &payload.tx_id);
            put_string(&mut out, &payload.network);
            put_string(&mut out, &payload.amount);
            put_u64(&mut out, payload.confirmations);
            put_u64(&mut out, payload.observed_at_ms);
        }
        Some(wallet_exchange_event_payload::Event::ProviderStatusObserved(payload)) => {
            put_u32(&mut out, payload.payload_version);
            put_string(&mut out, &payload.order_id);
            put_option_string(&mut out, &payload.provider_code_internal);
            put_string(&mut out, &payload.provider_order_id_internal);
            put_string(&mut out, &payload.provider_status_string);
            put_i32(&mut out, payload.mapped_canonical_status);
            put_u64(&mut out, payload.observed_at_ms);
        }
        Some(wallet_exchange_event_payload::Event::OrderStatusChanged(payload)) => {
            put_u32(&mut out, payload.payload_version);
            put_string(&mut out, &payload.order_id);
            put_i32(&mut out, payload.from_status);
            put_i32(&mut out, payload.to_status);
            put_string(&mut out, &payload.reason);
            put_u64(&mut out, payload.changed_at_ms);
        }
        Some(wallet_exchange_event_payload::Event::OrderCompleted(payload)) => {
            put_u32(&mut out, payload.payload_version);
            put_string(&mut out, &payload.order_id);
            put_option_string(&mut out, &payload.payout_tx_id);
            put_option_string(&mut out, &payload.payout_network);
            put_u64(&mut out, payload.completed_at_ms);
        }
        Some(wallet_exchange_event_payload::Event::OrderFailed(payload)) => {
            put_u32(&mut out, payload.payload_version);
            put_string(&mut out, &payload.order_id);
            put_string(&mut out, &payload.reason);
            put_u64(&mut out, payload.failed_at_ms);
        }
        Some(wallet_exchange_event_payload::Event::ManualReviewRequired(payload)) => {
            put_u32(&mut out, payload.payload_version);
            put_string(&mut out, &payload.order_id);
            put_string(&mut out, &payload.reason);
            put_i32(&mut out, payload.canonical_status);
            put_i32(&mut out, payload.provider_status);
            put_u64(&mut out, payload.flagged_at_ms);
        }
    }
    out
}

fn decode_exchange_payload(payload_bytes: &[u8]) -> Option<WalletExchangeEventPayload> {
    let mut reader = PayloadReader::new(payload_bytes);
    if reader.take(EXCHANGE_PAYLOAD_MAGIC.len())? != EXCHANGE_PAYLOAD_MAGIC {
        return None;
    }
    let payload_version = reader.u32()?;
    let tag = reader.u8()?;
    let event = match tag {
        0 => None,
        1 => Some(wallet_exchange_event_payload::Event::QuoteCreated(
            WalletQuoteCreatedPayload {
                payload_version: reader.u32()?,
                quote_id: reader.string()?,
                settlement_asset_id: reader.string()?,
                pay_asset_id: reader.string()?,
                settlement_amount: reader.string()?,
                pay_amount: reader.string()?,
                rate: reader.string()?,
                expires_at_ms: reader.u64()?,
                provider_code_internal: reader.option_string()?,
            },
        )),
        2 => Some(wallet_exchange_event_payload::Event::OrderCreated(
            WalletOrderCreatedPayload {
                payload_version: reader.u32()?,
                order_id: reader.string()?,
                quote_id: reader.string()?,
                provider_order_id_internal: reader.string()?,
                deposit_address: reader.string()?,
                recipient_address: reader.string()?,
                refund_address: reader.option_string()?,
                provider_code_internal: reader.option_string()?,
                created_at_ms: reader.u64()?,
            },
        )),
        3 => Some(wallet_exchange_event_payload::Event::DepositObserved(
            WalletDepositObservedPayload {
                payload_version: reader.u32()?,
                order_id: reader.string()?,
                tx_id: reader.string()?,
                network: reader.string()?,
                amount: reader.string()?,
                confirmations: reader.u64()?,
                observed_at_ms: reader.u64()?,
            },
        )),
        4 => Some(wallet_exchange_event_payload::Event::ProviderStatusObserved(
            WalletProviderStatusObservedPayload {
                payload_version: reader.u32()?,
                order_id: reader.string()?,
                provider_code_internal: reader.option_string()?,
                provider_order_id_internal: reader.string()?,
                provider_status_string: reader.string()?,
                mapped_canonical_status: reader.i32()?,
                observed_at_ms: reader.u64()?,
            },
        )),
        5 => Some(wallet_exchange_event_payload::Event::OrderStatusChanged(
            WalletOrderStatusChangedPayload {
                payload_version: reader.u32()?,
                order_id: reader.string()?,
                from_status: reader.i32()?,
                to_status: reader.i32()?,
                reason: reader.string()?,
                changed_at_ms: reader.u64()?,
            },
        )),
        6 => Some(wallet_exchange_event_payload::Event::OrderCompleted(
            WalletOrderCompletedPayload {
                payload_version: reader.u32()?,
                order_id: reader.string()?,
                payout_tx_id: reader.option_string()?,
                payout_network: reader.option_string()?,
                completed_at_ms: reader.u64()?,
            },
        )),
        7 => Some(wallet_exchange_event_payload::Event::OrderFailed(
            WalletOrderFailedPayload {
                payload_version: reader.u32()?,
                order_id: reader.string()?,
                reason: reader.string()?,
                failed_at_ms: reader.u64()?,
            },
        )),
        8 => Some(wallet_exchange_event_payload::Event::ManualReviewRequired(
            WalletManualReviewRequiredPayload {
                payload_version: reader.u32()?,
                order_id: reader.string()?,
                reason: reader.string()?,
                canonical_status: reader.i32()?,
                provider_status: reader.i32()?,
                flagged_at_ms: reader.u64()?,
            },
        )),
        _ => return None,
    };
    if !reader.done() {
        return None;
    }
    Some(WalletExchangeEventPayload {
        payload_version,
        event,
    })
}

pub fn exchange_payload_object_kind() -> i32 {
    ObjectKind::Payload as i32
}

/// Map exchange facts onto the existing coarse wallet event types.
///
/// More specific variants are carried by `WalletExchangeEventPayload` inside
/// the event payload object, so the stream enum does not need a parallel
/// exchange-specific event range.
pub fn exchange_event_type(event: &ExchangeEvent) -> i32 {
    match event {
        ExchangeEvent::QuoteCreated { .. } => EventType::WalletQuoteCreated as i32,
        ExchangeEvent::OrderCreated { .. } => EventType::WalletOrderCreated as i32,
        ExchangeEvent::DepositObserved { .. }
        | ExchangeEvent::ProviderStatusObserved { .. }
        | ExchangeEvent::OrderStatusChanged { .. }
        | ExchangeEvent::OrderCompleted { .. }
        | ExchangeEvent::OrderFailed { .. }
        | ExchangeEvent::ManualReviewRequired { .. } => EventType::WalletOrderStatusChanged as i32,
    }
}

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

pub fn encode_exchange_event(event: &ExchangeEvent) -> ExchangeStreamPayload {
    let wrapped = match event {
        ExchangeEvent::QuoteCreated {
            quote_id,
            settlement_asset_id,
            pay_asset_id,
            settlement_amount,
            pay_amount,
            rate,
            expires_at_ms,
            provider_code,
        } => wallet_exchange_event_payload::Event::QuoteCreated(WalletQuoteCreatedPayload {
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
        } => wallet_exchange_event_payload::Event::OrderCreated(WalletOrderCreatedPayload {
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
        } => wallet_exchange_event_payload::Event::DepositObserved(WalletDepositObservedPayload {
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
        } => wallet_exchange_event_payload::Event::ProviderStatusObserved(
            WalletProviderStatusObservedPayload {
                payload_version: 1,
                order_id: order_id.clone(),
                provider_code_internal: Some(provider_code.clone()),
                provider_order_id_internal: provider_order_id.clone(),
                provider_status_string: provider_status_string.clone(),
                mapped_canonical_status: *mapped_canonical_status,
                observed_at_ms: *observed_at_ms,
            },
        ),
        ExchangeEvent::OrderStatusChanged {
            order_id,
            from_status,
            to_status,
            reason,
            changed_at_ms,
        } => wallet_exchange_event_payload::Event::OrderStatusChanged(
            WalletOrderStatusChangedPayload {
                payload_version: 1,
                order_id: order_id.clone(),
                from_status: *from_status,
                to_status: *to_status,
                reason: reason.clone(),
                changed_at_ms: *changed_at_ms,
            },
        ),
        ExchangeEvent::OrderCompleted {
            order_id,
            payout_tx_id,
            payout_network,
            completed_at_ms,
        } => wallet_exchange_event_payload::Event::OrderCompleted(WalletOrderCompletedPayload {
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
        } => wallet_exchange_event_payload::Event::OrderFailed(WalletOrderFailedPayload {
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
        } => wallet_exchange_event_payload::Event::ManualReviewRequired(
            WalletManualReviewRequiredPayload {
                payload_version: 1,
                order_id: order_id.clone(),
                reason: reason.clone(),
                canonical_status: *canonical_status,
                provider_status: *provider_status,
                flagged_at_ms: *flagged_at_ms,
            },
        ),
    };

    let payload = WalletExchangeEventPayload {
        payload_version: 1,
        event: Some(wrapped),
    };

    ExchangeStreamPayload {
        event_type: exchange_event_type(event),
        payload_bytes: encode_exchange_payload(&payload),
    }
}

pub fn decode_exchange_event(event_type: i32, payload_bytes: &[u8]) -> Option<ExchangeEvent> {
    let event_type = edgerun_core::protocol::enum_from_i32::<EventType>(event_type)?;
    if !matches!(
        event_type,
        EventType::WalletQuoteCreated
            | EventType::WalletOrderCreated
            | EventType::WalletOrderStatusChanged
            | EventType::WalletPaymentRequestCreated
            | EventType::WalletReceiptCreated
    ) {
        return None;
    }

    let payload = decode_exchange_payload(payload_bytes)?;
    match payload.event? {
        wallet_exchange_event_payload::Event::QuoteCreated(payload) => {
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
        wallet_exchange_event_payload::Event::OrderCreated(payload) => {
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
        wallet_exchange_event_payload::Event::DepositObserved(payload) => {
            Some(ExchangeEvent::DepositObserved {
                order_id: payload.order_id,
                tx_id: payload.tx_id,
                network: payload.network,
                amount: payload.amount,
                confirmations: payload.confirmations,
                observed_at_ms: payload.observed_at_ms,
            })
        }
        wallet_exchange_event_payload::Event::ProviderStatusObserved(payload) => {
            Some(ExchangeEvent::ProviderStatusObserved {
                order_id: payload.order_id,
                provider_code: payload.provider_code_internal.unwrap_or_default(),
                provider_order_id: payload.provider_order_id_internal,
                provider_status_string: payload.provider_status_string,
                mapped_canonical_status: payload.mapped_canonical_status,
                observed_at_ms: payload.observed_at_ms,
            })
        }
        wallet_exchange_event_payload::Event::OrderStatusChanged(payload) => {
            Some(ExchangeEvent::OrderStatusChanged {
                order_id: payload.order_id,
                from_status: payload.from_status,
                to_status: payload.to_status,
                reason: payload.reason,
                changed_at_ms: payload.changed_at_ms,
            })
        }
        wallet_exchange_event_payload::Event::OrderCompleted(payload) => {
            Some(ExchangeEvent::OrderCompleted {
                order_id: payload.order_id,
                payout_tx_id: payload.payout_tx_id,
                payout_network: payload.payout_network,
                completed_at_ms: payload.completed_at_ms,
            })
        }
        wallet_exchange_event_payload::Event::OrderFailed(payload) => {
            Some(ExchangeEvent::OrderFailed {
                order_id: payload.order_id,
                reason: payload.reason,
                failed_at_ms: payload.failed_at_ms,
            })
        }
        wallet_exchange_event_payload::Event::ManualReviewRequired(payload) => {
            Some(ExchangeEvent::ManualReviewRequired {
                order_id: payload.order_id,
                reason: payload.reason,
                canonical_status: payload.canonical_status,
                provider_status: payload.provider_status,
                flagged_at_ms: payload.flagged_at_ms,
            })
        }
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
        assert!(matches!(
            roundtrip(event),
            ExchangeEvent::QuoteCreated { .. }
        ));
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
        assert!(matches!(
            roundtrip(event),
            ExchangeEvent::OrderCreated { .. }
        ));
    }

    #[test]
    fn provider_observation_uses_existing_status_event_type() {
        let event = ExchangeEvent::ProviderStatusObserved {
            order_id: "ex-1".into(),
            provider_code: "SIDESHIFT".into(),
            provider_order_id: "provider-1".into(),
            provider_status_string: "waiting".into(),
            mapped_canonical_status: 4,
            observed_at_ms: 789,
        };
        let encoded = encode_exchange_event(&event);
        assert_eq!(
            encoded.event_type,
            EventType::WalletOrderStatusChanged as i32
        );
        assert!(matches!(
            decode_exchange_event(encoded.event_type, &encoded.payload_bytes),
            Some(ExchangeEvent::ProviderStatusObserved { .. })
        ));
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
        let envelope =
            build_exchange_event_envelope(&event, vec![9, 9, 9], 7, None, payload_object.clone());
        assert_eq!(
            envelope.event_type,
            EventType::WalletOrderStatusChanged as i32
        );
        assert_eq!(envelope.payload_object, Some(payload_object.clone()));
        assert_eq!(envelope.related_objects, vec![payload_object]);
    }
}
