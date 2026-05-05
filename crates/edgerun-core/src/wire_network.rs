//! Rkyv-only network advisory wire boundary.
//!
//! Network record byte helpers are explicit breakpoints until callers archive
//! the concrete rkyv record directly.

use alloc::vec::Vec;

use crate::protocol::{ReachabilityHint, RouteAdvertisement};

#[cold]
fn removed_network_wire_path() -> ! {
    panic!("network wire helpers were removed; use the rkyv wire boundary")
}

#[must_use]
pub fn route_advertisement_signable_bytes(_value: &RouteAdvertisement) -> Vec<u8> {
    removed_network_wire_path()
}

#[must_use]
pub fn route_advertisement_full_bytes(_value: &RouteAdvertisement) -> Vec<u8> {
    removed_network_wire_path()
}

#[must_use]
pub fn reachability_hint_signable_bytes(_value: &ReachabilityHint) -> Vec<u8> {
    removed_network_wire_path()
}

#[must_use]
pub fn reachability_hint_full_bytes(_value: &ReachabilityHint) -> Vec<u8> {
    removed_network_wire_path()
}
