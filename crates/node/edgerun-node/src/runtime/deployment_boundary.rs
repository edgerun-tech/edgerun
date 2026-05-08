use alloc::vec::Vec;

use crate::network::{
    decide_bindings, NodeTransportSurface, ServiceBindingDecision, ServiceBindingIntent,
};
use crate::storage::{
    decide_storage_intents, RuntimeStorageDecision, RuntimeStorageIntent, RuntimeStorageSurface,
};

use super::RuntimeServicePlan;

/// Host/runtime adapter surfaces used to realize a deployment plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeBoundarySurface {
    pub transport: NodeTransportSurface,
    pub storage: RuntimeStorageSurface,
}

impl RuntimeBoundarySurface {
    pub const NATIVE: Self = Self {
        transport: NodeTransportSurface::NativeSocket,
        storage: RuntimeStorageSurface::HostDurable,
    };

    pub const WASM: Self = Self {
        transport: NodeTransportSurface::BrowserMessage,
        storage: RuntimeStorageSurface::BrowserDurable,
    };

    pub const REPLAY: Self = Self {
        transport: NodeTransportSurface::InProcess,
        storage: RuntimeStorageSurface::Memory,
    };
}

/// Boundary-neutral deployment intent derived from the rkyv runtime config.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeBoundaryIntent {
    pub bindings: Vec<ServiceBindingIntent>,
    pub storage: Vec<RuntimeStorageIntent>,
}

/// Host/runtime-specific realization decisions for the same deployment intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeBoundaryDecision {
    pub bindings: Vec<ServiceBindingDecision>,
    pub storage: Vec<RuntimeStorageDecision>,
}

impl RuntimeBoundaryIntent {
    pub fn from_service_plan(plan: &RuntimeServicePlan, surface: RuntimeBoundarySurface) -> Self {
        Self {
            bindings: plan.requested_bindings(surface.transport),
            storage: plan.requested_storage(surface.storage),
        }
    }

    pub fn decide(self) -> RuntimeBoundaryDecision {
        RuntimeBoundaryDecision {
            bindings: decide_bindings(self.bindings),
            storage: decide_storage_intents(self.storage),
        }
    }
}

pub fn decide_runtime_boundary(
    plan: &RuntimeServicePlan,
    surface: RuntimeBoundarySurface,
) -> RuntimeBoundaryDecision {
    RuntimeBoundaryIntent::from_service_plan(plan, surface).decide()
}
