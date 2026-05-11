use alloc::vec::Vec;

use crate::channel::{RouteAdvertisement, RouteSnapshot, ROUTE_STATUS_AVAILABLE};
use crate::protocol::{Hash, NodeId};
use crate::route_auth::{route_advertisement_preimage, verify_route_advertisement, verify_route_snapshot};
use crate::codec::blake3_hash;

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
        self.snapshot.routes.iter().find(|route| route.node.node_id == node_id)
    }

    pub fn first_route_for_department(&self, department: u16) -> Option<&RouteAdvertisement> {
        self.snapshot.routes.iter().find(|route| {
            route.status == ROUTE_STATUS_AVAILABLE && route.departments.contains(&department)
        })
    }

    pub fn routes_for_department(&self, department: u16) -> Vec<&RouteAdvertisement> {
        self.snapshot
            .routes
            .iter()
            .filter(|route| route.status == ROUTE_STATUS_AVAILABLE && route.departments.contains(&department))
            .collect()
    }

    pub fn require_department_route(&self, department: u16) -> Result<&RouteAdvertisement, RoutePlanError> {
        self.first_route_for_department(department).ok_or(RoutePlanError::NoRoute)
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
