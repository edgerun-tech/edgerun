//! linux-drm-syncobj-v1 protocol — explicit DRM synchronization objects.
//!
//! Allows clients to provide explicit synchronization via DRM syncobj timelines.
//! The client creates syncobj timelines (via DRM ioctls) and imports them to the
//! compositor via FD. The compositor waits on acquire points before scanning out
//! and signals release points when done with buffers.
//!
//! Protocol: wp_linux_drm_syncobj_manager_v1 / surface_v1 / timeline_v1
//! Spec: /usr/share/wayland-protocols/staging/linux-drm-syncobj/linux-drm-syncobj-v1.xml

use crate::wayland::ArgType;

// ─── wp_linux_drm_syncobj_manager_v1 ────────────────────────

pub const MANAGER_V1: &str = "wp_linux_drm_syncobj_manager_v1";
pub const MANAGER_V1_VERSION: u32 = 1;

/// Surface extension protocol version.
pub const SURFACE_V1: &str = "wp_linux_drm_syncobj_surface_v1";
pub const SURFACE_V1_VERSION: u32 = 1;

/// Timeline protocol version.
pub const TIMELINE_V1: &str = "wp_linux_drm_syncobj_timeline_v1";
pub const TIMELINE_V1_VERSION: u32 = 1;

/// Protocol errors.
pub mod error {
    pub const SURFACE_EXISTS: u32 = 0;
    pub const INVALID_TIMELINE: u32 = 1;
}

// ─── Manager requests ───────────────────────────────────────

pub mod manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    /// get_surface — extend wl_surface for explicit synchronization.
    /// id: new_id(wp_linux_drm_syncobj_surface_v1), surface: object(wl_surface)
    pub const GET_SURFACE: u16 = 1;
    pub const GET_SURFACE_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];

    /// import_timeline — import a DRM syncobj timeline from client FD.
    /// id: new_id(wp_linux_drm_syncobj_timeline_v1), fd: fd
    pub const IMPORT_TIMELINE: u16 = 2;
    pub const IMPORT_TIMELINE_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Fd];
}

// ─── wp_linux_drm_syncobj_surface_v1 requests ───────────────

pub mod surface_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    /// set_acquire_point — timeline point before compositor may sample buffer.
    /// timeline: object(wp_linux_drm_syncobj_timeline_v1), point_hi: uint, point_lo: uint
    pub const SET_ACQUIRE_POINT: u16 = 1;
    pub const SET_ACQUIRE_POINT_SIG: &[ArgType] = &[ArgType::Object, ArgType::Uint, ArgType::Uint];

    /// set_release_point — timeline point compositor signals when done with buffer.
    /// timeline: object(wp_linux_drm_syncobj_timeline_v1), point_hi: uint, point_lo: uint
    pub const SET_RELEASE_POINT: u16 = 2;
    pub const SET_RELEASE_POINT_SIG: &[ArgType] = &[ArgType::Object, ArgType::Uint, ArgType::Uint];
}

// ─── Surface errors ─────────────────────────────────────────

pub mod surface_error {
    pub const NO_SURFACE: u32 = 1;
    pub const UNSUPPORTED_BUFFER: u32 = 2;
    pub const NO_BUFFER: u32 = 3;
    pub const NO_ACQUIRE_POINT: u32 = 4;
    pub const NO_RELEASE_POINT: u32 = 5;
    pub const CONFLICTING_POINTS: u32 = 6;
}

// ─── wp_linux_drm_syncobj_timeline_v1 requests ──────────────

pub mod timeline_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];
}
