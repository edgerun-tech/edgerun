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

pub use bounce::{send_bounce, BounceConfig};
pub use queue::MailIndex;
pub use relay::OutboundRelay;
pub use worker::{DeliveryWorker, DeliveryWorkerConfig};
