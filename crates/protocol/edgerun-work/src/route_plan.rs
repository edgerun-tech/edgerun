use alloc::vec::Vec;

use crate::channel::{RouteAdvertisement, RouteSnapshot};
use crate::protocol::{Hash, NodeId};
use crate::route_auth::{route_advertisement_preimage, verify_route_advertisement, verify_route_snapshot};
use crate::codec::blake3_hash;
use crate::memory_channel::route_is_available;

#[cfg(feature = "std")]
fn current_unix_ms() -> u64 {
    crate::std_runtime::unix_ms()
}

#[cfg(not(feature = "std"))]
fn current_unix_ms() -> u64 {
    0
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

    pub fn route_for_node_at(&self, node_id: NodeId, now_unix_ms: u64) -> Option<&RouteAdvertisement> {
        self.snapshot
            .routes
            .iter()
            .find(|route| route.node.node_id == node_id && route_is_available(route, now_unix_ms))
    }

    pub fn first_route_for_department(&self, department: u16) -> Option<&RouteAdvertisement> {
        self.first_route_for_department_at(department, current_unix_ms())
    }

    pub fn first_route_for_department_at(&self, department: u16, now_unix_ms: u64) -> Option<&RouteAdvertisement> {
        self.snapshot.routes.iter().find(|route| {
            route_is_available(route, now_unix_ms) && route.departments.contains(&department)
        })
    }

    pub fn routes_for_department(&self, department: u16) -> Vec<&RouteAdvertisement> {
        self.routes_for_department_at(department, current_unix_ms())
    }

    pub fn routes_for_department_at(&self, department: u16, now_unix_ms: u64) -> Vec<&RouteAdvertisement> {
        self.snapshot
            .routes
            .iter()
            .filter(|route| route_is_available(route, now_unix_ms) && route.departments.contains(&department))
            .collect()
    }

    pub fn require_department_route(&self, department: u16) -> Result<&RouteAdvertisement, RoutePlanError> {
        self.first_route_for_department(department).ok_or(RoutePlanError::NoRoute)
    }

    pub fn require_department_route_at(&self, department: u16, now_unix_ms: u64) -> Result<&RouteAdvertisement, RoutePlanError> {
        self.first_route_for_department_at(department, now_unix_ms).ok_or(RoutePlanError::NoRoute)
    }
}

pub fn route_commitment(route: &RouteAdvertisement) -> Hash {
    blake3_hash(&route_advertisement_preimage(route))
}

pub fn route_root_hash(routes: &[RouteAdvertisement]) -> Hash {
    let mut bytes = Vec::new();
    for route in routes {
        bytes.extend_from_slice(&route_commitment(route));
    }
    blake3_hash(&bytes)
}
