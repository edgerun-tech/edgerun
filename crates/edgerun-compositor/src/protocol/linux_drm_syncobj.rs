//! linux-drm-syncobj-v1 protocol — DRM synchronization objects.
//!
//! This is a stub implementation. We accept the requests but don't implement
//! actual fence synchronization. This allows Chromium and other modern clients
//! to proceed without failing on missing syncobj support.

use crate::wire::ArgType;

pub const LINUX_DRM_SYNCOBJ_V1: &str = "linux_drm_syncobj_v1";
pub const LINUX_DRM_SYNCOBJ_V1_VERSION: u32 = 1;

/// Surface protocol version.
pub const SURFACE_V1: &str = "linux_drm_syncobj_surface_v1";
pub const SURFACE_V1_VERSION: u32 = 1;

/// Timeline protocol version.
pub const TIMELINE_V1: &str = "linux_drm_syncobj_timeline_v1";
pub const TIMELINE_V1_VERSION: u32 = 1;

// ─── linux_drm_syncobj_v1 ───────────────────────────────────

pub mod syncobj_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    /// get_surface
    pub const GET_SURFACE: u16 = 1;
    pub const GET_SURFACE_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // id: new_id, surface: object(wl_surface)

    /// create_timeline
    pub const CREATE_TIMELINE: u16 = 2;
    pub const CREATE_TIMELINE_SIG: &[ArgType] = &[ArgType::NewId];
}

// ─── linux_drm_syncobj_surface_v1 ───────────────────────────

pub mod surface_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    /// set_acquire_point
    pub const SET_ACQUIRE_POINT: u16 = 1;
    pub const SET_ACQUIRE_POINT_SIG: &[ArgType] = &[ArgType::Object, ArgType::Uint, ArgType::Uint];
    // timeline: object, handle_lo: uint, handle_hi: uint

    /// set_release_point
    pub const SET_RELEASE_POINT: u16 = 2;
    pub const SET_RELEASE_POINT_SIG: &[ArgType] = &[ArgType::Object, ArgType::Uint, ArgType::Uint];
}

// ─── linux_drm_syncobj_timeline_v1 ──────────────────────────

pub mod timeline_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    /// import_sync_file
    pub const IMPORT_SYNC_FILE: u16 = 1;
    pub const IMPORT_SYNC_FILE_SIG: &[ArgType] = &[ArgType::Uint, ArgType::Uint, ArgType::String, ArgType::Fd];
    // handle_lo: uint, handle_hi: uint, name: string, sync_file: fd

    /// export_sync_file
    pub const EXPORT_SYNC_FILE: u16 = 2;
    pub const EXPORT_SYNC_FILE_SIG: &[ArgType] = &[ArgType::Uint, ArgType::Uint];
    // handle_lo: uint, handle_hi: uint
}

// No events for this protocol — just request handling.
