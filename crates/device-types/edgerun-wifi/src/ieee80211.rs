//! IEEE 802.11 frame core.
//!
//! Protocol ownership lives in `edgerun-protocols`; this module reexports it
//! from the Wi-Fi capability crate for existing callers.

pub use edgerun_protocols::ieee80211::*;
