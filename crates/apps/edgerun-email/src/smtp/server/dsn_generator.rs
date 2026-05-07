//! DSN bounce message generation re-export.
//!
//! The RFC 3464 formatter lives in `edgerun-protocols`; runtime delivery stays
//! in `edgerun-email`.

pub use edgerun_protocols::smtp::dsn::{DeliveryStatus, DsnAction, DsnBounce};
