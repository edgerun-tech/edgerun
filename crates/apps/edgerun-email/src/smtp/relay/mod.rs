//! Outbound mail relay and queue system.
//!
//! # Architecture
//! ```text
//! SMTP server → accept_mail() → MailIndex::enqueue_message()
//!                                     ↓
//!                            DeliveryWorker (background)
//!                                     ↓
//!                            OutboundRelay (MX lookup → SMTP delivery)
//!                                     ↓
//!               success → mark_delivered()  |  failure → retry or bounce
//!                                     ↓
//!                            BounceSender (RFC 3464 DSN to original sender)
//! ```

pub mod bounce;
pub mod queue;
pub mod relay;
pub mod worker;

pub use bounce::{BounceConfig, send_bounce};
pub use queue::MailIndex;
pub use relay::OutboundRelay;
pub use worker::{DeliveryWorker, DeliveryWorkerConfig};

pub(crate) fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
