//! zwp_idle_inhibit_manager_v1 — idle inhibition protocol.
//!
//! This protocol allows clients to inhibit the compositor's idle behavior
//! (e.g., screen blanking, DPMS) while a surface is visible.

use crate::wire::ArgType;

pub const ZWP_IDLE_INHIBIT_MANAGER_V1: &str = "zwp_idle_inhibit_manager_v1";
pub const ZWP_IDLE_INHIBIT_MANAGER_V1_VERSION: u32 = 1;

pub mod idle_inhibit_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const CREATE_INHIBITOR: u16 = 1;
    pub const CREATE_INHIBITOR_SIG: &[ArgType] = &[ArgType::Object, ArgType::NewId];
}

pub mod idle_inhibitor_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];
}
