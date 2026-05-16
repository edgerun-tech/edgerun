//! xdg-foreign-v2 protocol — create surfaces that outlive their parent.
//!
//! Used by GTK4 to create popups/tooltips that persist after the parent
//! surface is destroyed.

use alloc::vec::Vec;

use crate::wayland::{ArgType, Message};

pub const ZXDG_EXPORTER_V2: &str = "zxdg_exporter_v2";
pub const ZXDG_EXPORTER_V2_VERSION: u32 = 1;

pub const ZXDG_IMPORTER_V2: &str = "zxdg_importer_v2";
pub const ZXDG_IMPORTER_V2_VERSION: u32 = 1;

// ─── zxdg_exporter_v2 ────────────────────────────────────────

pub mod exporter_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const EXPORT: u16 = 1;
    pub const EXPORT_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // id: new_id(zxdg_exported_v2), surface: object(wl_surface)
}

pub const ZXDG_EXPORTED_V2: &str = "zxdg_exported_v2";
pub const ZXDG_EXPORTED_V2_VERSION: u32 = 1;

pub mod exported_event {
    /// handle — the string handle that can be used to import this surface.
    pub const HANDLE: u16 = 0;
    // sig: string
}

pub mod exported_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

/// Build a handle event.
pub fn exported_handle_event(exported_id: u32, handle: &str) -> Message {
    let mut args = Vec::new();
    crate::wayland::encode::encode_string(&mut args, handle);
    Message {
        sender_id: exported_id,
        opcode: exported_event::HANDLE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

// ─── zxdg_importer_v2 ────────────────────────────────────────

pub mod importer_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const IMPORT: u16 = 1;
    pub const IMPORT_SIG: &[ArgType] = &[ArgType::NewId, ArgType::String];
    // id: new_id(zxdg_imported_v2), handle: string
}

pub const ZXDG_IMPORTED_V2: &str = "zxdg_imported_v2";
pub const ZXDG_IMPORTED_V2_VERSION: u32 = 1;

pub mod imported_event {
    /// destroyed — the exported surface was destroyed.
    pub const DESTROYED: u16 = 0;
}

pub mod imported_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_PARENT_OF: u16 = 1;
    pub const SET_PARENT_OF_SIG: &[ArgType] = &[ArgType::Object];
    // surface: object(wl_surface)
}
