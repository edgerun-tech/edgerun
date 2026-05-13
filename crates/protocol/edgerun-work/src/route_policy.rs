use alloc::vec::Vec;

use crate::channel::{
    CHANNEL_KIND_MEMORY, CHANNEL_KIND_QUIC, CHANNEL_KIND_TCP, CHANNEL_KIND_WASM_HOST,
    CHANNEL_KIND_WEBSOCKET, CHANNEL_KIND_WEBTRANSPORT, RouteBinding,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteRuntimeProfile {
    Native,
    Browser,
    WasmHost,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteSelectionPolicy {
    pub allowed_channel_kinds: Vec<u16>,
}

impl RouteSelectionPolicy {
    pub fn native() -> Self {
        Self {
            allowed_channel_kinds: Vec::from([
                CHANNEL_KIND_QUIC,
                CHANNEL_KIND_TCP,
                CHANNEL_KIND_WEBSOCKET,
                CHANNEL_KIND_WEBTRANSPORT,
                CHANNEL_KIND_MEMORY,
                CHANNEL_KIND_WASM_HOST,
            ]),
        }
    }

    pub fn browser() -> Self {
        Self {
            allowed_channel_kinds: Vec::from([
                CHANNEL_KIND_WEBTRANSPORT,
                CHANNEL_KIND_WEBSOCKET,
                CHANNEL_KIND_WASM_HOST,
                CHANNEL_KIND_MEMORY,
            ]),
        }
    }

    pub fn wasm_host() -> Self {
        Self {
            allowed_channel_kinds: Vec::from([
                CHANNEL_KIND_WASM_HOST,
                CHANNEL_KIND_MEMORY,
                CHANNEL_KIND_WEBTRANSPORT,
                CHANNEL_KIND_WEBSOCKET,
            ]),
        }
    }

    pub fn for_profile(profile: RouteRuntimeProfile) -> Self {
        match profile {
            RouteRuntimeProfile::Native => Self::native(),
            RouteRuntimeProfile::Browser => Self::browser(),
            RouteRuntimeProfile::WasmHost => Self::wasm_host(),
        }
    }

    pub fn allows(&self, route: &RouteBinding) -> bool {
        self.allowed_channel_kinds.contains(&route.endpoint.kind)
    }

    pub fn priority(&self, route: &RouteBinding) -> Option<usize> {
        self.allowed_channel_kinds
            .iter()
            .position(|kind| *kind == route.endpoint.kind)
    }
}
