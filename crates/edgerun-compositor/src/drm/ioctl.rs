//! DRM/KMS raw ioctl definitions — direct Linux kernel, no C libraries.
//!
//! Ioctl numbers verified from kernel headers (`/usr/include/drm/drm_mode.h`).

use libc::{c_int, c_uint};

/// Type alias for ioctl request codes — works on both glibc and musl.
/// glibc uses c_ulong, musl uses c_int for the request parameter.
/// We use c_int which is the kernel's actual type for ioctl request codes.
pub type IoctlRequest = c_int;

// DRM ioctl numbers (from kernel headers on this system, 64-bit x86_64)
pub const DRM_IOCTL_VERSION: IoctlRequest = 0xc0406400u32 as c_int;
pub const DRM_IOCTL_SET_CLIENT_CAP: IoctlRequest = 0x4010640du32 as c_int;
pub const DRM_IOCTL_SET_MASTER: IoctlRequest = 0x0000641eu32 as c_int;
pub const DRM_IOCTL_DROP_MASTER: IoctlRequest = 0x0000641fu32 as c_int;
pub const DRM_IOCTL_PRIME_FD_TO_HANDLE: IoctlRequest = 0xc00c642eu32 as c_int;
pub const DRM_IOCTL_PRIME_HANDLE_TO_FD: IoctlRequest = 0xc00c642du32 as c_int;
pub const DRM_IOCTL_MODE_GETRESOURCES: IoctlRequest = 0xc04064a0u32 as c_int;
pub const DRM_IOCTL_MODE_GETCRTC: IoctlRequest = 0xc06864a1u32 as c_int;
pub const DRM_IOCTL_MODE_SETCRTC: IoctlRequest = 0xc06864a2u32 as c_int;
pub const DRM_IOCTL_MODE_PAGE_FLIP: IoctlRequest = 0xc01864b0u32 as c_int;
pub const DRM_IOCTL_MODE_MAP_DUMB: IoctlRequest = 0xc01064b3u32 as c_int;
pub const DRM_IOCTL_MODE_RMFB: IoctlRequest = 0xc00464afu32 as c_int;
pub const DRM_IOCTL_MODE_CREATE_DUMB: IoctlRequest = 0xc02064b2u32 as c_int;
pub const DRM_IOCTL_MODE_GETCONNECTOR: IoctlRequest = 0xc05064a7u32 as c_int;
pub const DRM_IOCTL_MODE_ADDFB2: IoctlRequest = 0xc06864b8u32 as c_int;
pub const DRM_IOCTL_MODE_DESTROY_DUMB: IoctlRequest = 0xc00464b4u32 as c_int;
pub const DRM_IOCTL_MODE_ATOMIC: IoctlRequest = 0xc03864bcu32 as c_int;
pub const DRM_IOCTL_MODE_GETPLANE: IoctlRequest = 0xc02064b6u32 as c_int;
// Alias for backward compatibility
pub const DRM_IOCTL_MODE_GETPLANES: IoctlRequest = DRM_IOCTL_MODE_GETPLANE;

// DRM syncobj ioctls (kernel 4.12+ — explicit synchronization)
pub const DRM_IOCTL_SYNCOBJ_CREATE: IoctlRequest = 0xc00c644bu32 as c_int;
pub const DRM_IOCTL_SYNCOBJ_DESTROY: IoctlRequest = 0xc004644cu32 as c_int;
pub const DRM_IOCTL_SYNCOBJ_HANDLE_TO_FD: IoctlRequest = 0xc010644du32 as c_int;
pub const DRM_IOCTL_SYNCOBJ_FD_TO_HANDLE: IoctlRequest = 0xc00c644eu32 as c_int;
pub const DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT: IoctlRequest = 0x40286457u32 as c_int;
pub const DRM_IOCTL_SYNCOBJ_TIMELINE_SIGNAL: IoctlRequest = 0x40286458u32 as c_int;
pub const DRM_IOCTL_SYNCOBJ_TIMELINE_QUERY: IoctlRequest = 0xc018645cu32 as c_int;
pub const DRM_IOCTL_SYNCOBJ_IMPORT_SYNC_FILE: IoctlRequest = 0xc008645du32 as c_int;
pub const DRM_IOCTL_SYNCOBJ_EXPORT_SYNC_FILE: IoctlRequest = 0xc008645eu32 as c_int;
pub const DRM_IOCTL_SYNCOBJ_WAIT: IoctlRequest = 0xc018644fu32 as c_int;
pub const DRM_IOCTL_SYNCOBJ_RESET: IoctlRequest = 0xc0086450u32 as c_int;
pub const DRM_IOCTL_SYNCOBJ_SIGNAL: IoctlRequest = 0xc0086451u32 as c_int;

// ─── Syncobj structs ────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DrmSyncobjCreate {
    pub flags: u32,
    pub handle: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DrmSyncobjDestroy {
    pub handle: u32,
    pub pad: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DrmSyncobjFdToHandle {
    pub fd: c_int,
    pub handle: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DrmSyncobjImportSyncFile {
    pub handle: u32,
    pub fd: c_int,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DrmSyncobjExportSyncFile {
    pub handle: u32,
    pub fd: c_int,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmSyncobjTimelineWait {
    pub handles_ptr: *mut u32,
    pub timelines_ptr: *mut u64,
    pub timeout_nsec: u64,
    pub flags: u32,
    pub count_handles: u32,
    pub pad: [u8; 8],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmSyncobjTimelineSignal {
    pub handles_ptr: *mut u32,
    pub timelines_ptr: *mut u64,
    pub count_handles: u32,
    pub flags: u32,
}

// ─── Struct definitions (matching kernel uapi exactly for 64-bit x86_64) ──

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmVersion {
    pub version_major: c_int,
    pub version_minor: c_int,
    pub version_patchlevel: c_int,
    pub name_len: c_int,
    pub name: *mut u8,
    pub date_len: c_int,
    pub date: *mut u8,
    pub desc_len: c_int,
    pub desc: *mut u8,
    pub _pad: [u8; 8],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModeCardRes {
    pub fb_id_ptr: *mut c_uint,
    pub crtc_id_ptr: *mut c_uint,
    pub connector_id_ptr: *mut c_uint,
    pub encoder_id_ptr: *mut c_uint,
    pub count_fbs: c_int,
    pub count_crtcs: c_int,
    pub count_connectors: c_int,
    pub count_encoders: c_int,
    pub min_width: c_uint,
    pub max_width: c_uint,
    pub min_height: c_uint,
    pub max_height: c_uint,
}
// sizeof = 64 ✓

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModeModeInfo {
    pub clock: c_uint,
    pub hdisplay: u16,
    pub hsync_start: u16,
    pub hsync_end: u16,
    pub htotal: u16,
    pub hskew: u16,
    pub vdisplay: u16,
    pub vsync_start: u16,
    pub vsync_end: u16,
    pub vtotal: u16,
    pub vscan: u16,
    pub vrefresh: c_uint,
    pub flags: c_uint,
    pub type_: c_uint,
    pub name: [u8; 32],
}
// sizeof = 68 ✓

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModeGetConnector {
    // Pointers first (kernel layout)
    pub encoders_ptr: *mut c_uint,
    pub modes_ptr: *mut DrmModeModeInfo,
    pub props_ptr: *mut c_uint,
    pub prop_values_ptr: *mut u64,
    // Counts
    pub count_modes: c_int,
    pub count_props: c_int,
    pub count_encoders: c_int,
    // IDs and properties
    pub encoder_id: c_uint,
    pub connector_id: c_uint,
    pub connector_type: c_uint,
    pub connector_type_id: c_int,
    pub connection: c_int,
    pub mm_width: c_uint,
    pub mm_height: c_uint,
    pub subpixel: c_int,
    pub pad: c_uint,
}
// sizeof = 4*8 + 12*4 = 32 + 48 = 80 ✓

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModeCrtc {
    // Same struct used for both GET and SET
    pub set_connectors_ptr: *mut c_uint,
    pub count_connectors: c_uint,
    pub crtc_id: c_uint,
    pub fb_id: c_uint,
    pub x: c_uint,
    pub y: c_uint,
    pub gamma_size: c_uint,
    pub mode_valid: c_uint,
    pub mode: DrmModeModeInfo,
}
// sizeof = 8 + 4*7 + 68 = 8 + 28 + 68 = 104 ✓

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModeFbCmd2 {
    pub fb_id: c_uint,
    pub width: c_uint,
    pub height: c_uint,
    pub pixel_format: c_uint,
    pub flags: c_uint,
    pub handles: [c_uint; 4],
    pub pitches: [c_uint; 4],
    pub offsets: [c_uint; 4],
    pub modifier: [u64; 4],
    // trailing padding to 8-byte alignment: 100 + 4 = 104
    pub _pad: [u8; 4],
}
// sizeof = 104 ✓

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModeCreateDumb {
    pub height: c_uint,
    pub width: c_uint,
    pub bpp: c_uint,
    pub flags: c_uint,
    pub handle: c_uint,
    pub pitch: c_uint,
    pub size: u64,
}
// sizeof = 6*4 + 8 = 32 ✓

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModeMapDumb {
    pub handle: c_uint,
    pub pad: c_uint,
    pub offset: u64,
}
// sizeof = 16 ✓

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModeDestroyDumb {
    pub handle: c_uint,
}
// sizeof = 4 ✓

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModePageFlip {
    pub crtc_id: c_uint,
    pub fb_id: c_uint,
    pub flags: c_uint,
    pub reserved: c_uint,
    pub user_data: u64,
}
// sizeof = 24 ✓

pub mod page_flip {
    pub const PAGE_FLIP_EVENT: libc::c_uint = 0x01;
    pub const PAGE_FLIP_ASYNC: libc::c_uint = 0x02;
}

pub mod connector_status {
    pub const CONNECTED: i32 = 1;
    pub const DISCONNECTED: i32 = 2;
    pub const UNKNOWN: i32 = 3;
}

pub mod atomic {
    pub const TEST_ONLY: libc::c_uint = 0x01;
    pub const NONBLOCK: libc::c_uint = 0x02;
    pub const ALLOW_MODESET: libc::c_uint = 0x04;
}

pub mod client_cap {
    pub const UNIVERSAL_PLANES: u64 = 2;
    pub const ATOMIC: u64 = 3;
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmSetClientCap {
    pub capability: u64,
    pub value: u64,
}
// sizeof = 16 ✓

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmPrimeFdToHandle {
    pub fd: c_int,
    pub handle: c_uint,
    pub pad: c_uint,
}
// sizeof = 12 ✓

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmPrimeHandleToFd {
    pub handle: c_uint,
    pub flags: c_uint,
    pub fd: c_int,
}
// sizeof = 12 ✓

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModeAtomic {
    pub flags: c_uint,
    pub count_objs: c_uint,
    pub objs_ptr: *mut c_uint,
    pub count_props_ptr: *mut c_uint,
    pub props_ptr: *mut c_uint,
    pub prop_values_ptr: *mut u64,
    pub reserved: u64,
    pub user_data: u64,
}
// sizeof = 56 ✓

pub mod syncobj {
    /// Timeline is a monotonic counter (not binary semaphore).
    pub const CREATE_SIGNALED: u32 = 1 << 0;
    /// Wait for any handle to be signaled (vs all).
    pub const WAIT_ANY: u32 = 1 << 0;
    /// Wait for submission (not yet signaled).
    pub const WAIT_FOR_SUBMIT: u32 = 1 << 1;
    /// Wait available (monotonic timeline).
    pub const WAIT_AVAILABLE: u32 = 1 << 2;
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModeGetPlaneRes {
    pub plane_id_ptr: *mut c_uint,
    pub count_planes: c_uint,
    pub pad: c_uint,
}
// sizeof = 16 ✓
