//! DRM device management — open, authenticate, get resources.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};
use std::path::Path;

use super::ioctl::*;

/// A DRM device (e.g., /dev/dri/card0 or /dev/dri/renderD128).
pub struct DrmDevice {
    file: File,
    /// DRM node path.
    pub path: String,
}

impl DrmDevice {
    /// Open a DRM device. Prefer render nodes for rendering-only work,
    /// card nodes for KMS/mode setting.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        Ok(Self {
            file,
            path: path.to_string_lossy().to_string(),
        })
    }

    pub fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }

    /// Take ownership of the file descriptor.
    pub fn into_fd(self) -> OwnedFd {
        self.file.into()
    }

    /// Get driver version.
    pub fn version(&self) -> io::Result<DrmVersion> {
        let mut ver = DrmVersion {
            version_major: 0,
            version_minor: 0,
            version_patchlevel: 0,
            name_len: 0,
            name: std::ptr::null_mut(),
            date_len: 0,
            date: std::ptr::null_mut(),
            desc_len: 0,
            desc: std::ptr::null_mut(),
            _pad: [0; 8],
        };
        let ret = unsafe { libc::ioctl(self.file.as_raw_fd(), DRM_IOCTL_VERSION, &mut ver) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(ver)
    }

    /// Set DRM master (required for mode setting on card nodes).
    pub fn set_master(&self) -> io::Result<()> {
        let ret = unsafe { libc::ioctl(self.file.as_raw_fd(), DRM_IOCTL_SET_MASTER) };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Drop DRM master.
    pub fn drop_master(&self) -> io::Result<()> {
        let ret = unsafe { libc::ioctl(self.file.as_raw_fd(), DRM_IOCTL_DROP_MASTER) };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Set client capability (e.g., universal planes, atomic).
    pub fn set_client_cap(&self, cap: u64, value: u64) -> io::Result<()> {
        let mut cc = DrmSetClientCap {
            capability: cap,
            value,
        };
        let ret = unsafe { libc::ioctl(self.file.as_raw_fd(), DRM_IOCTL_SET_CLIENT_CAP, &mut cc) };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Get resources: CRTC IDs, connector IDs, encoder IDs, FB IDs.
    pub fn get_resources(&self) -> io::Result<DrmResources> {
        // First call with null pointers to get counts
        let mut res = DrmModeCardRes {
            fb_id_ptr: std::ptr::null_mut(),
            crtc_id_ptr: std::ptr::null_mut(),
            connector_id_ptr: std::ptr::null_mut(),
            encoder_id_ptr: std::ptr::null_mut(),
            count_fbs: 0,
            count_crtcs: 0,
            count_connectors: 0,
            count_encoders: 0,
            min_width: 0,
            max_width: 0,
            min_height: 0,
            max_height: 0,
        };

        let ret =
            unsafe { libc::ioctl(self.file.as_raw_fd(), DRM_IOCTL_MODE_GETRESOURCES, &mut res) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // Allocate arrays
        let mut crtcs = vec![0u32; res.count_crtcs as usize];
        let mut connectors = vec![0u32; res.count_connectors as usize];
        let mut encoders = vec![0u32; res.count_encoders as usize];
        let mut fbs = vec![0u32; res.count_fbs as usize];

        res.crtc_id_ptr = crtcs.as_mut_ptr();
        res.connector_id_ptr = connectors.as_mut_ptr();
        res.encoder_id_ptr = encoders.as_mut_ptr();
        res.fb_id_ptr = fbs.as_mut_ptr();

        let ret =
            unsafe { libc::ioctl(self.file.as_raw_fd(), DRM_IOCTL_MODE_GETRESOURCES, &mut res) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(DrmResources {
            crtcs,
            connectors,
            encoders,
            fbs,
            count_connectors: res.count_connectors,
            count_crtcs: res.count_crtcs,
            count_encoders: res.count_encoders,
            count_fbs: res.count_fbs,
            min_width: res.min_width,
            max_width: res.max_width,
            min_height: res.min_height,
            max_height: res.max_height,
        })
    }

    /// Get connector info including modes.
    pub fn get_connector(&self, connector_id: u32) -> io::Result<DrmConnector> {
        // First pass: get mode count
        let mut conn = DrmModeGetConnector {
            connector_id,
            encoders_ptr: std::ptr::null_mut(),
            modes_ptr: std::ptr::null_mut(),
            props_ptr: std::ptr::null_mut(),
            prop_values_ptr: std::ptr::null_mut(),
            count_modes: 0,
            count_props: 0,
            count_encoders: 0,
            encoder_id: 0,
            connector_type: 0,
            connector_type_id: 0,
            connection: 0,
            mm_width: 0,
            mm_height: 0,
            subpixel: 0,
            pad: 0,
        };

        let ret = unsafe {
            libc::ioctl(
                self.file.as_raw_fd(),
                DRM_IOCTL_MODE_GETCONNECTOR,
                &mut conn,
            )
        };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        let num_modes = conn.count_modes as usize;
        let num_encoders = conn.count_encoders as usize;
        let num_props = conn.count_props as usize;

        // Allocate arrays
        let mut modes: Vec<DrmModeModeInfo> = Vec::with_capacity(num_modes);
        unsafe {
            modes.set_len(num_modes);
        }

        let mut encoder_ids = vec![0u32; num_encoders];
        let mut prop_ids = vec![0u32; num_props];
        let mut prop_values = vec![0u64; num_props];

        conn.modes_ptr = modes.as_mut_ptr();
        conn.encoders_ptr = encoder_ids.as_mut_ptr();
        conn.props_ptr = prop_ids.as_mut_ptr();
        conn.prop_values_ptr = prop_values.as_mut_ptr();

        let ret = unsafe {
            libc::ioctl(
                self.file.as_raw_fd(),
                DRM_IOCTL_MODE_GETCONNECTOR,
                &mut conn,
            )
        };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(DrmConnector {
            connector_id: conn.connector_id,
            encoder_id: conn.encoder_id,
            connector_type: conn.connector_type,
            connector_type_id: conn.connector_type_id,
            connection: conn.connection,
            mm_width: conn.mm_width,
            mm_height: conn.mm_height,
            subpixel: conn.subpixel,
            encoder_ids,
            modes,
            prop_ids,
            prop_values,
        })
    }

    /// Get CRTC info.
    pub fn get_crtc(&self, crtc_id: u32) -> io::Result<DrmModeCrtcInfo> {
        let mut crtc = DrmModeCrtc {
            set_connectors_ptr: std::ptr::null_mut(),
            count_connectors: 0,
            crtc_id,
            fb_id: 0,
            x: 0,
            y: 0,
            gamma_size: 0,
            mode_valid: 0,
            mode: unsafe { std::mem::MaybeUninit::zeroed().assume_init() },
        };
        let ret = unsafe { libc::ioctl(self.file.as_raw_fd(), DRM_IOCTL_MODE_GETCRTC, &mut crtc) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(DrmModeCrtcInfo {
            crtc_id: crtc.crtc_id,
            fb_id: crtc.fb_id,
            x: crtc.x,
            y: crtc.y,
            width: crtc.mode.hdisplay as u32,
            height: crtc.mode.vdisplay as u32,
            mode_valid: crtc.mode_valid,
            mode: crtc.mode,
            gamma_size: crtc.gamma_size,
        })
    }

    /// Get plane resources.
    pub fn get_plane_resources(&self) -> io::Result<Vec<u32>> {
        let mut res = DrmModeGetPlaneRes {
            plane_id_ptr: std::ptr::null_mut(),
            count_planes: 0,
            pad: 0,
        };

        let ret = unsafe { libc::ioctl(self.file.as_raw_fd(), DRM_IOCTL_MODE_GETPLANES, &mut res) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        let mut planes = vec![0u32; res.count_planes as usize];
        res.plane_id_ptr = planes.as_mut_ptr();

        let ret = unsafe { libc::ioctl(self.file.as_raw_fd(), DRM_IOCTL_MODE_GETPLANES, &mut res) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(planes)
    }
}

/// Discovered DRM resources.
#[derive(Debug)]
pub struct DrmResources {
    pub crtcs: Vec<u32>,
    pub connectors: Vec<u32>,
    pub encoders: Vec<u32>,
    pub fbs: Vec<u32>,
    pub count_connectors: i32,
    pub count_crtcs: i32,
    pub count_encoders: i32,
    pub count_fbs: i32,
    pub min_width: c_uint,
    pub max_width: c_uint,
    pub min_height: c_uint,
    pub max_height: c_uint,
}

/// A connector with its modes.
#[derive(Debug, Clone)]
pub struct DrmConnector {
    pub connector_id: u32,
    pub encoder_id: u32,
    pub connector_type: u32,
    pub connector_type_id: i32,
    pub connection: i32,
    pub subpixel: i32,
    pub mm_width: u32,
    pub mm_height: u32,
    pub encoder_ids: Vec<u32>,
    pub modes: Vec<DrmModeModeInfo>,
    pub prop_ids: Vec<u32>,
    pub prop_values: Vec<u64>,
}

/// Simplified CRTC info extracted from the kernel's drm_mode_crtc.
#[derive(Debug, Clone)]
pub struct DrmModeCrtcInfo {
    pub crtc_id: u32,
    pub fb_id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub mode_valid: c_uint,
    pub mode: DrmModeModeInfo,
    pub gamma_size: c_uint,
}

impl DrmConnector {
    pub fn is_connected(&self) -> bool {
        self.connection == connector_status::CONNECTED
    }

    /// Get the connector type as a human-readable string.
    pub fn type_name(&self) -> &'static str {
        match self.connector_type {
            1 => "VGA",
            2 => "DVI-I",
            3 => "DVI-D",
            4 => "DVI-A",
            5 => "Composite",
            6 => "SVIDEO",
            7 => "LVDS",
            8 => "Component",
            9 => "9PinDIN",
            10 => "DisplayPort",
            11 => "HDMI-A",
            12 => "HDMI-B",
            13 => "TV",
            14 => "eDP",
            15 => "Virtual",
            16 => "DSI",
            17 => "DPI",
            18 => "Writeback",
            19 => "SPI",
            20 => "USB",
            _ => "Unknown",
        }
    }
}

/// Connector type constants (from drm_mode.h).
mod connector_status {
    pub const CONNECTED: i32 = 1;
    #[allow(dead_code)]
    pub const DISCONNECTED: i32 = 2;
    #[allow(dead_code)]
    pub const UNKNOWN_CONNECTION: i32 = 3;
}

use libc::c_uint;
