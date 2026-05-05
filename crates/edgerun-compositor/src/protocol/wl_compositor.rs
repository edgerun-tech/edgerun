//! wl_compositor, wl_surface, wl_region, wl_subcompositor.

use crate::wire::encode::*;
use crate::wire::{ArgType, Message};
use surface_event::*;

// ─── wl_compositor ───────────────────────────────────────────

pub const WL_COMPOSITOR: &str = "wl_compositor";
pub const WL_COMPOSITOR_VERSION: u32 = 4;

pub mod compositor_request {
    use super::*;

    /// create_surface
    pub const CREATE_SURFACE: u16 = 0;
    pub const CREATE_SURFACE_SIG: &[ArgType] = &[ArgType::NewId];

    /// create_region
    pub const CREATE_REGION: u16 = 1;
    pub const CREATE_REGION_SIG: &[ArgType] = &[ArgType::NewId];
}

pub mod compositor_event {
    // No events from wl_compositor.
}

// ─── wl_surface ──────────────────────────────────────────────

pub const WL_SURFACE: &str = "wl_surface";
pub const WL_SURFACE_VERSION: u32 = 4;

pub mod surface_request {
    use super::*;

    /// destroy
    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    /// attach — set the buffer for the surface.
    pub const ATTACH: u16 = 1;
    pub const ATTACH_SIG: &[ArgType] = &[ArgType::Object, ArgType::Int, ArgType::Int];
    // buffer: object (nullable), x: int, y: int

    /// damage — mark a region as damaged.
    pub const DAMAGE: u16 = 2;
    pub const DAMAGE_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int, ArgType::Int, ArgType::Int];
    // x, y, width, height

    /// frame — request a frame callback.
    pub const FRAME: u16 = 3;
    pub const FRAME_SIG: &[ArgType] = &[ArgType::NewId]; // callback: new_id

    /// set_opaque_region
    pub const SET_OPAQUE_REGION: u16 = 4;
    pub const SET_OPAQUE_REGION_SIG: &[ArgType] = &[ArgType::Object]; // nullable

    /// set_input_region
    pub const SET_INPUT_REGION: u16 = 5;
    pub const SET_INPUT_REGION_SIG: &[ArgType] = &[ArgType::Object]; // nullable

    /// get_subsurface
    pub const GET_SUBSURFACE: u16 = 6;
    pub const GET_SUBSURFACE_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // subsurface: new_id, parent: object

    /// set_buffer_transform
    pub const SET_BUFFER_TRANSFORM: u16 = 7;
    pub const SET_BUFFER_TRANSFORM_SIG: &[ArgType] = &[ArgType::Int];

    /// set_buffer_scale
    pub const SET_BUFFER_SCALE: u16 = 8;
    pub const SET_BUFFER_SCALE_SIG: &[ArgType] = &[ArgType::Int];

    /// damage_buffer — like damage but in buffer coordinates.
    pub const DAMAGE_BUFFER: u16 = 9;
    pub const DAMAGE_BUFFER_SIG: &[ArgType] =
        &[ArgType::Int, ArgType::Int, ArgType::Int, ArgType::Int];

    /// offset — set the surface offset.
    pub const OFFSET: u16 = 10;
    pub const OFFSET_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int];

    /// commit — commit pending surface state.
    pub const COMMIT: u16 = 11;
    pub const COMMIT_SIG: &[ArgType] = &[];
}

pub mod surface_event {
    /// enter — surface entered an output.
    pub const ENTER: u16 = 0;
    // sig: object (output)

    /// leave — surface left an output.
    pub const LEAVE: u16 = 1;
    // sig: object (output)

    /// preferred_buffer_scale
    pub const PREFERRED_BUFFER_SCALE: u16 = 2;
    // sig: uint

    /// preferred_buffer_format
    pub const PREFERRED_BUFFER_FORMAT: u16 = 3;
    // sig: uint
}

/// Create an `enter` event.
pub fn surface_enter_event(surface_id: u32, output_id: u32) -> Message {
    message_uint(surface_id, ENTER, output_id)
}

/// Create a `leave` event.
pub fn surface_leave_event(surface_id: u32, output_id: u32) -> Message {
    message_uint(surface_id, LEAVE, output_id)
}

/// Create a `preferred_buffer_scale` event.
pub fn surface_preferred_buffer_scale_event(surface_id: u32, scale: u32) -> Message {
    message_uint(surface_id, PREFERRED_BUFFER_SCALE, scale)
}

/// Create a `preferred_buffer_format` event.
pub fn surface_preferred_buffer_format_event(surface_id: u32, format: u32) -> Message {
    message_uint(surface_id, PREFERRED_BUFFER_FORMAT, format)
}

// ─── wl_region ───────────────────────────────────────────────

pub const WL_REGION: &str = "wl_region";
pub const WL_REGION_VERSION: u32 = 1;

pub mod region_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const ADD: u16 = 1;
    pub const ADD_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int, ArgType::Int, ArgType::Int];

    pub const SUBTRACT: u16 = 2;
    pub const SUBTRACT_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int, ArgType::Int, ArgType::Int];
}

// ─── wl_subcompositor ────────────────────────────────────────

pub const WL_SUBCOMPOSITOR: &str = "wl_subcompositor";
pub const WL_SUBCOMPOSITOR_VERSION: u32 = 1;

pub mod subcompositor_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_SUBSURFACE: u16 = 1;
    pub const GET_SUBSURFACE_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object, ArgType::Object];
    // subsurface: new_id, surface: object, parent: object
}

pub mod subcompositor_event {
    pub const INCOMPLETE: u16 = 0;
    // sig: none (error event)
}

// ─── wl_subsurface ───────────────────────────────────────────

pub const WL_SUBSURFACE: &str = "wl_subsurface";
pub const WL_SUBSURFACE_VERSION: u32 = 1;

pub mod subsurface_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_POSITION: u16 = 1;
    pub const SET_POSITION_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int];

    pub const PLACE_ABOVE: u16 = 2;
    pub const PLACE_ABOVE_SIG: &[ArgType] = &[ArgType::Object];

    pub const PLACE_BELOW: u16 = 3;
    pub const PLACE_BELOW_SIG: &[ArgType] = &[ArgType::Object];

    pub const SET_SYNC: u16 = 4;
    pub const SET_SYNC_SIG: &[ArgType] = &[];

    pub const SET_DESYNC: u16 = 5;
    pub const SET_DESYNC_SIG: &[ArgType] = &[];
}
