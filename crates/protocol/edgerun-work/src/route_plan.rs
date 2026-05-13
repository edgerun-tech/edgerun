use alloc::vec::Vec;

use crate::channel::{
    RouteAdvertisement, RouteSnapshot, CHANNEL_KIND_MEMORY, CHANNEL_KIND_QUIC, CHANNEL_KIND_TCP,
    CHANNEL_KIND_WASM_HOST, CHANNEL_KIND_WEBSOCKET, CHANNEL_KIND_WEBTRANSPORT,
};
use crate::memory_channel::route_is_available;
use crate::preimage::HashBuilder;
use crate::protocol::{Hash, NodeId};
use crate::route_auth::{
    route_advertisement_preimage, verify_route_advertisement, verify_route_snapshot,
};

const ROUTE_COMMITMENT_DOMAIN: &[u8] = b"edgerun:v1:work:route-commitment";
const ROUTE_ROOT_DOMAIN: &[u8] = b"edgerun:v1:work:route-root";

#[cfg(feature = "std")]
fn current_unix_ms() -> u64 {
    crate::std_runtime::unix_ms()
}

#[cfg(not(feature = "std"))]
fn current_unix_ms() -> u64 {
    0
}

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

    pub fn allows(&self, route: &RouteAdvertisement) -> bool {
        self.allowed_channel_kinds.contains(&route.endpoint.kind)
    }

    pub fn priority(&self, route: &RouteAdvertisement) -> Option<usize> {
        self.allowed_channel_kinds
            .iter()
            .position(|kind| *kind == route.endpoint.kind)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoutePlanError {
    InvalidSnapshot,
    InvalidRoute,
    RouteRootMismatch,
    NoRoute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedRoutePlan {
    pub snapshot: RouteSnapshot,
}

impl VerifiedRoutePlan {
    pub fn from_snapshot(snapshot: RouteSnapshot) -> Result<Self, RoutePlanError> {
        if !verify_route_snapshot(&snapshot) {
            return Err(RoutePlanError::InvalidSnapshot);
        }
        for route in &snapshot.routes {
            if !verify_route_advertisement(route) {
                return Err(RoutePlanError::InvalidRoute);
            }
        }
        if route_root_hash(&snapshot.routes) != snapshot.route_root {
            return Err(RoutePlanError::RouteRootMismatch);
        }
        Ok(Self { snapshot })
    }

    pub fn route_for_node(&self, node_id: NodeId) -> Option<&RouteAdvertisement> {
        self.route_for_node_at(node_id, current_unix_ms())
    }

    pub fn route_for_node_at(
        &self,
        node_id: NodeId,
        now_unix_ms: u64,
    ) -> Option<&RouteAdvertisement> {
        self.snapshot
            .routes
            .iter()
            .find(|route| route.node.node_id == node_id && route_is_available(route, now_unix_ms))
    }

    pub fn preferred_route_for_node(
        &self,
        node_id: NodeId,
        policy: &RouteSelectionPolicy,
    ) -> Option<&RouteAdvertisement> {
        self.preferred_route_for_node_at(node_id, policy, current_unix_ms())
    }

    pub fn preferred_route_for_node_at(
        &self,
        node_id: NodeId,
        policy: &RouteSelectionPolicy,
        now_unix_ms: u64,
    ) -> Option<&RouteAdvertisement> {
        self.snapshot
            .routes
            .iter()
            .filter(|route| {
                route.node.node_id == node_id
                    && route_is_available(route, now_unix_ms)
                    && policy.allows(route)
            })
            .min_by_key(|route| policy.priority(route).unwrap_or(usize::MAX))
    }

    pub fn first_route_for_department(&self, department: u16) -> Option<&RouteAdvertisement> {
        self.first_route_for_department_at(department, current_unix_ms())
    }

    pub fn first_route_for_department_at(
        &self,
        department: u16,
        now_unix_ms: u64,
    ) -> Option<&RouteAdvertisement> {
        self.snapshot.routes.iter().find(|route| {
            route_is_available(route, now_unix_ms) && route.departments.contains(&department)
        })
    }

    pub fn preferred_route_for_department(
        &self,
        department: u16,
        policy: &RouteSelectionPolicy,
    ) -> Option<&RouteAdvertisement> {
        self.preferred_route_for_department_at(department, policy, current_unix_ms())
    }

    pub fn preferred_route_for_department_at(
        &self,
        department: u16,
        policy: &RouteSelectionPolicy,
        now_unix_ms: u64,
    ) -> Option<&RouteAdvertisement> {
        self.snapshot
            .routes
            .iter()
            .filter(|route| {
                route_is_available(route, now_unix_ms)
                    && route.departments.contains(&department)
                    && policy.allows(route)
            })
            .min_by_key(|route| policy.priority(route).unwrap_or(usize::MAX))
    }

    pub fn routes_for_department(&self, department: u16) -> Vec<&RouteAdvertisement> {
        self.routes_for_department_at(department, current_unix_ms())
    }

    pub fn routes_for_department_at(
        &self,
        department: u16,
        now_unix_ms: u64,
    ) -> Vec<&RouteAdvertisement> {
        self.snapshot
            .routes
            .iter()
            .filter(|route| {
                route_is_available(route, now_unix_ms) && route.departments.contains(&department)
            })
            .collect()
    }

    pub fn routes_for_department_by_policy(
        &self,
        department: u16,
        policy: &RouteSelectionPolicy,
    ) -> Vec<&RouteAdvertisement> {
        self.routes_for_department_by_policy_at(department, policy, current_unix_ms())
    }

    pub fn routes_for_department_by_policy_at(
        &self,
        department: u16,
        policy: &RouteSelectionPolicy,
        now_unix_ms: u64,
    ) -> Vec<&RouteAdvertisement> {
        let mut routes = self
            .snapshot
            .routes
            .iter()
            .filter(|route| {
                route_is_available(route, now_unix_ms)
                    && route.departments.contains(&department)
                    && policy.allows(route)
            })
            .collect::<Vec<_>>();
        routes.sort_by_key(|route| policy.priority(route).unwrap_or(usize::MAX));
        routes
    }

    pub fn require_department_route(
        &self,
        department: u16,
    ) -> Result<&RouteAdvertisement, RoutePlanError> {
        self.first_route_for_department(department)
            .ok_or(RoutePlanError::NoRoute)
    }

    pub fn require_department_route_at(
        &self,
        department: u16,
        now_unix_ms: u64,
    ) -> Result<&RouteAdvertisement, RoutePlanError> {
        self.first_route_for_department_at(department, now_unix_ms)
            .ok_or(RoutePlanError::NoRoute)
    }

    pub fn require_preferred_department_route(
        &self,
        department: u16,
        policy: &RouteSelectionPolicy,
    ) -> Result<&RouteAdvertisement, RoutePlanError> {
        self.preferred_route_for_department(department, policy)
            .ok_or(RoutePlanError::NoRoute)
    }
}

pub fn route_commitment(route: &RouteAdvertisement) -> Hash {
    HashBuilder::domain(ROUTE_COMMITMENT_DOMAIN)
        .bytes(&route_advertisement_preimage(route))
        .finish()
}

pub fn route_root_hash(routes: &[RouteAdvertisement]) -> Hash {
    let mut builder = HashBuilder::domain(ROUTE_ROOT_DOMAIN).u64(routes.len() as u64);
    for route in routes {
        builder = builder.hash(&route_commitment(route));
    }
    builder.finish()
}
