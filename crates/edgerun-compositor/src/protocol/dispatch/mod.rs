//! Protocol message dispatch — routes Wayland messages to per-interface handlers.
//!
//! Each interface group is handled by a separate module. The `DispatchContext`
//! struct bundles all state so modules don't need a 40-parameter function signature.

use std::collections::HashMap;

use crate::compositor::surface::{BufferRegistry, SurfaceTree};
use crate::compositor::shell::Shell;
use crate::compositor::seat::Seat;

// Re-export constraint types from seat module for use in main.rs
pub use crate::compositor::seat::{PointerConstraint, ConstraintType};
use crate::input::keymap::{Keymap, Modifiers};
use crate::protocol::wp_presentation_time::PresentationFeedbackTracker;
use crate::render::cursor::Cursor;
use crate::render::shm::ShmManager;
use crate::resource::Registry;
use crate::server::WaylandServer;
use crate::wire;
use crate::compositor::dmabuf::DmabufParams;

mod core;
mod compositor;
mod seat_handlers;
mod shm;
mod data_device;
mod subsurface;
mod xdg_shell;
mod xdg_ext;
mod xdg_foreign;
mod wp_ext;
mod linux_ext;
mod zwp_ext;
mod wlroots_ext;
mod ime;

/// Touch input state — tracks active touch slots and their positions.
pub struct TouchState {
    /// Map from evdev touch slot (ABS_MT_SLOT) to touch position and surface.
    pub slots: HashMap<i32, TouchSlot>,
    /// Current active slot.
    pub current_slot: i32,
    /// Next touch object ID to assign.
    pub next_touch_id: u32,

    // Gesture detection state
    /// Number of active touch points.
    pub active_fingers: u32,
    /// Whether a swipe gesture is active.
    pub swipe_active: bool,
    /// Whether a pinch gesture is active.
    pub pinch_active: bool,
    /// Initial touch positions for gesture calculation.
    pub gesture_start_positions: Vec<(f64, f64)>,
    /// Last centroid position for gesture deltas.
    pub last_centroid_x: f64,
    pub last_centroid_y: f64,
    /// Initial distance between touch points (for pinch scale).
    pub initial_pinch_distance: f64,
    /// Current average distance between touch points.
    pub current_pinch_distance: f64,
    /// Serial for gesture events.
    pub gesture_serial: u32,
    /// Number of fingers when gesture started.
    pub gesture_finger_count: u32,
    /// Time when gesture started.
    pub gesture_start_time: u32,
    /// Previous centroid X for delta calculation.
    pub prev_centroid_x: f64,
    pub prev_centroid_y: f64,
    /// Whether gesture is in progress.
    pub gesture_in_progress: bool,
}

/// A single touch slot state.
#[derive(Debug, Clone)]
pub struct TouchSlot {
    pub touch_id: u32,
    pub surface_id: Option<u32>,
    pub client_id: Option<u32>,
    pub x: f64,
    pub y: f64,
    pub active: bool,
}

impl TouchState {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
            current_slot: 0,
            next_touch_id: 1,
            active_fingers: 0,
            swipe_active: false,
            pinch_active: false,
            gesture_start_positions: Vec::new(),
            last_centroid_x: 0.0,
            last_centroid_y: 0.0,
            initial_pinch_distance: 0.0,
            current_pinch_distance: 0.0,
            gesture_serial: 1,
            gesture_finger_count: 0,
            gesture_start_time: 0,
            prev_centroid_x: 0.0,
            prev_centroid_y: 0.0,
            gesture_in_progress: false,
        }
    }
}

/// Clipboard data source.
pub struct DataSource {
    pub id: u32,
    pub owner_client_id: u32,
    pub mime_types: Vec<String>,
}

/// Primary selection (middle-click paste) data source.
pub struct PrimarySelectionSource {
    pub id: u32,
    pub owner_client_id: u32,
    pub mime_types: Vec<String>,
}

/// Global objects advertised to clients via wl_registry.global.
const GLOBALS: &[(&str, u32)] = &[
    ("wl_compositor", 4),
    ("wl_shm", 1),
    ("wl_seat", 7),
    ("xdg_wm_base", 6),
    ("wl_output", 4),
    ("zwp_linux_dmabuf_v1", 4),
    ("wl_data_device_manager", 3),
    ("wl_subcompositor", 1),
    ("zxdg_decoration_manager_v1", 1),
    ("wp_viewporter", 1),
    ("wp_cursor_shape_manager_v1", 1),
    ("xdg_activation_v1", 1),
    ("wp_presentation", 1),
    ("zwp_relative_pointer_manager_v1", 1),
    ("zwp_pointer_gestures_v1", 1),
    ("zwp_text_input_manager_v1", 1),
    ("zwp_idle_inhibit_manager_v1", 1),
    ("zwp_pointer_constraints_v1", 1),
    ("zxdg_output_manager_v1", 3),
    ("zxdg_exporter_v2", 1),
    ("zxdg_importer_v2", 1),
    ("linux_drm_syncobj_v1", 1),
    ("linux_drm_syncobj_surface_v1", 1),
    ("linux_drm_syncobj_timeline_v1", 1),
    ("zwlr_screencopy_manager_v1", 3),
    ("zwp_text_input_manager_v3", 1),
    ("zwp_input_method_manager_v2", 1),
    ("zwlr_primary_selection_manager_v1", 1),
    ("zwlr_data_control_manager_v1", 2),
    ("wp_single_pixel_buffer_manager_v1", 1),
    ("wp_fractional_scale_manager_v1", 1),
    ("wp_tearing_control_manager_v1", 1),
];

/// Screencopy state — holds the last rendered framebuffer snapshot.
pub struct ScreencopyState {
    /// Last rendered frame pixels (XRGB8888, little-endian).
    pub pixels: Vec<u8>,
    /// Frame dimensions.
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    /// Pending screencopy frames awaiting COPY.
    pub pending_frames: HashMap<u32, screencopy::ScreencopyFrame>,
    /// Frame flags (Y_INVERT, etc.).
    pub flags: u32,
}

impl ScreencopyState {
    pub fn new() -> Self {
        Self {
            pixels: Vec::new(),
            width: 0,
            height: 0,
            stride: 0,
            pending_frames: HashMap::new(),
            flags: 0,
        }
    }

    /// Update the framebuffer snapshot from the dumb buffer pixels.
    pub fn update(&mut self, width: u32, height: u32, stride: u32, pixels: &[u8]) {
        self.width = width;
        self.height = height;
        self.stride = stride;
        self.pixels.clear();
        self.pixels.extend_from_slice(pixels);
    }
}

/// Bundled state passed to per-interface dispatch handlers.
pub struct DispatchContext<'a> {
    pub server: &'a mut WaylandServer,
    pub client_id: u32,
    pub msg: wire::Message,
    pub surfaces: &'a mut SurfaceTree,
    pub buffers: &'a mut BufferRegistry,
    pub shm: &'a mut ShmManager,
    pub shell: &'a mut Shell,
    pub seat: &'a mut Seat,
    pub _keymap: &'a mut Keymap,
    pub modifiers: &'a mut Modifiers,
    pub cursor: &'a mut Cursor,
    pub presentation_tracker: &'a mut PresentationFeedbackTracker,
    pub client_registries: &'a mut HashMap<u32, Registry>,
    pub client_registry_ids: &'a mut HashMap<u32, u32>,
    pub client_compositor_ids: &'a mut HashMap<u32, u32>,
    pub client_shm_ids: &'a mut HashMap<u32, u32>,
    pub client_seat_ids: &'a mut HashMap<u32, u32>,
    pub client_xdg_base_ids: &'a mut HashMap<u32, u32>,
    pub client_output_ids: &'a mut HashMap<u32, u32>,
    pub client_keyboard_ids: &'a mut HashMap<u32, u32>,
    pub client_pointer_ids: &'a mut HashMap<u32, u32>,
    pub client_touch_ids: &'a mut HashMap<u32, u32>,
    pub client_dmabuf_ids: &'a mut HashMap<u32, u32>,
    pub client_pool_map: &'a mut HashMap<u32, HashMap<u32, u32>>,
    pub client_data_device_ids: &'a mut HashMap<u32, u32>,
    pub client_data_source_ids: &'a mut HashMap<u32, u32>,
    pub client_subcompositor_ids: &'a mut HashMap<u32, u32>,
    pub client_decoration_manager_ids: &'a mut HashMap<u32, u32>,
    pub client_decoration_ids: &'a mut HashMap<u32, u32>,
    pub client_viewporter_ids: &'a mut HashMap<u32, u32>,
    pub client_cursor_shape_manager_ids: &'a mut HashMap<u32, u32>,
    pub client_cursor_shape_device_ids: &'a mut HashMap<u32, u32>,
    pub _client_cursor_surfaces: &'a mut HashMap<u32, u32>,
    pub client_relative_pointer_ids: &'a mut HashMap<u32, u32>,
    pub xdg_surface_to_wl_surface: &'a mut HashMap<u32, u32>,
    pub _dmabuf_pending: &'a mut HashMap<u32, DmabufParams>,
    pub current_data_source: &'a mut Option<DataSource>,
    pub selection_offer_counter: &'a mut u32,
    pub config_serial: &'a mut u32,
    pub pointer_constraints: &'a mut HashMap<u32, PointerConstraint>,
    pub constraint_type_map: &'a mut HashMap<u32, ConstraintType>,
    pub current_primary_selection: &'a mut Option<PrimarySelectionSource>,
    pub primary_selection_offer_counter: &'a mut u32,
    pub screencopy_state: &'a mut ScreencopyState,
    pub text_input_state: &'a mut Option<text_input_v3::TextInputState>,
    pub ime_state: &'a mut Option<input_method_v2::IMEState>,
    pub text_input_serial: &'a mut u32,
    pub client_tearing_control_ids: &'a mut HashMap<u32, u32>,
}

use crate::protocol::screencopy;
use crate::protocol::text_input_v3;
use crate::protocol::text_input_v3::TextInputState;
use crate::protocol::input_method_v2;
use crate::protocol::input_method_v2::IMEState;

/// Dispatch a single Wayland message to the appropriate protocol handler.
#[allow(clippy::too_many_arguments)]
pub fn process_message(
    server: &mut WaylandServer,
    client_id: u32,
    msg: wire::Message,
    surfaces: &mut SurfaceTree,
    buffers: &mut BufferRegistry,
    shm: &mut ShmManager,
    shell: &mut Shell,
    seat: &mut Seat,
    _keymap: &mut Keymap,
    modifiers: &mut Modifiers,
    cursor: &mut Cursor,
    presentation_tracker: &mut PresentationFeedbackTracker,
    client_registries: &mut HashMap<u32, Registry>,
    client_registry_ids: &mut HashMap<u32, u32>,
    client_compositor_ids: &mut HashMap<u32, u32>,
    client_shm_ids: &mut HashMap<u32, u32>,
    client_seat_ids: &mut HashMap<u32, u32>,
    client_xdg_base_ids: &mut HashMap<u32, u32>,
    client_output_ids: &mut HashMap<u32, u32>,
    client_keyboard_ids: &mut HashMap<u32, u32>,
    client_pointer_ids: &mut HashMap<u32, u32>,
    client_touch_ids: &mut HashMap<u32, u32>,
    client_dmabuf_ids: &mut HashMap<u32, u32>,
    client_pool_map: &mut HashMap<u32, HashMap<u32, u32>>,
    client_data_device_ids: &mut HashMap<u32, u32>,
    client_data_source_ids: &mut HashMap<u32, u32>,
    client_subcompositor_ids: &mut HashMap<u32, u32>,
    client_decoration_manager_ids: &mut HashMap<u32, u32>,
    client_decoration_ids: &mut HashMap<u32, u32>,
    client_viewporter_ids: &mut HashMap<u32, u32>,
    client_cursor_shape_manager_ids: &mut HashMap<u32, u32>,
    client_cursor_shape_device_ids: &mut HashMap<u32, u32>,
    _client_cursor_surfaces: &mut HashMap<u32, u32>,
    client_relative_pointer_ids: &mut HashMap<u32, u32>,
    xdg_surface_to_wl_surface: &mut HashMap<u32, u32>,
    _dmabuf_pending: &mut HashMap<u32, DmabufParams>,
    current_data_source: &mut Option<DataSource>,
    selection_offer_counter: &mut u32,
    config_serial: &mut u32,
    pointer_constraints: &mut HashMap<u32, PointerConstraint>,
    constraint_type_map: &mut HashMap<u32, ConstraintType>,
    current_primary_selection: &mut Option<PrimarySelectionSource>,
    primary_selection_offer_counter: &mut u32,
    screencopy_state: &mut ScreencopyState,
    text_input_state: &mut Option<TextInputState>,
    ime_state: &mut Option<IMEState>,
    text_input_serial: &mut u32,
    client_tearing_control_ids: &mut HashMap<u32, u32>,
) {
    let interface: String = {
        let reg = match client_registries.get(&client_id) {
            Some(r) => r,
            None => return,
        };
        reg.interface(msg.sender_id).unwrap_or("").to_string()
    };

    let mut ctx = DispatchContext {
        server,
        client_id,
        msg,
        surfaces,
        buffers,
        shm,
        shell,
        seat,
        _keymap,
        modifiers,
        cursor,
        presentation_tracker,
        client_registries,
        client_registry_ids,
        client_compositor_ids,
        client_shm_ids,
        client_seat_ids,
        client_xdg_base_ids,
        client_output_ids,
        client_keyboard_ids,
        client_pointer_ids,
        client_touch_ids,
        client_dmabuf_ids,
        client_pool_map,
        client_data_device_ids,
        client_data_source_ids,
        client_subcompositor_ids,
        client_decoration_manager_ids,
        client_decoration_ids,
        client_viewporter_ids,
        client_cursor_shape_manager_ids,
        client_cursor_shape_device_ids,
        _client_cursor_surfaces,
        client_relative_pointer_ids,
        xdg_surface_to_wl_surface,
        _dmabuf_pending,
        current_data_source,
        selection_offer_counter,
        config_serial,
        pointer_constraints,
        constraint_type_map,
        current_primary_selection,
        primary_selection_offer_counter,
        screencopy_state,
        text_input_state,
        ime_state,
        text_input_serial,
        client_tearing_control_ids,
    };

    match interface.as_str() {
        "wl_display" => core::handle_display(&mut ctx),
        "wl_registry" => core::handle_registry(&mut ctx),
        "wl_callback" => {}
        "wl_compositor" => compositor::handle_compositor(&mut ctx),
        "wl_surface" => compositor::handle_surface(&mut ctx),
        "wl_region" => compositor::handle_region(&mut ctx),
        "wl_buffer" => shm::handle_buffer(&mut ctx),
        "wl_shm" => shm::handle_shm(&mut ctx),
        "wl_shm_pool" => shm::handle_shm_pool(&mut ctx),
        "wl_seat" => seat_handlers::handle_seat(&mut ctx),
        "wl_keyboard" => seat_handlers::handle_keyboard(&mut ctx),
        "wl_pointer" => seat_handlers::handle_pointer(&mut ctx),
        "wl_touch" => seat_handlers::handle_touch(&mut ctx),
        "wl_data_device_manager" => data_device::handle_manager(&mut ctx),
        "wl_data_source" => data_device::handle_source(&mut ctx),
        "wl_data_offer" => data_device::handle_offer(&mut ctx),
        "wl_data_device" => data_device::handle_device(&mut ctx),
        "wl_subcompositor" => subsurface::handle_subcompositor(&mut ctx),
        "wl_subsurface" => subsurface::handle_subsurface(&mut ctx),
        "xdg_wm_base" => xdg_shell::handle_wm_base(&mut ctx),
        "xdg_surface" => xdg_shell::handle_surface(&mut ctx),
        "xdg_toplevel" => xdg_shell::handle_toplevel(&mut ctx),
        "xdg_positioner" => xdg_shell::handle_positioner(&mut ctx),
        "xdg_popup" => xdg_shell::handle_popup(&mut ctx),
        "zxdg_decoration_manager_v1" => xdg_ext::handle_decoration_manager(&mut ctx),
        "zxdg_toplevel_decoration_v1" => xdg_ext::handle_toplevel_decoration(&mut ctx),
        "xdg_activation_v1" => xdg_ext::handle_activation(&mut ctx),
        "xdg_activation_token_v1" => xdg_ext::handle_activation_token(&mut ctx),
        "zxdg_output_manager_v1" => xdg_ext::handle_output_manager(&mut ctx),
        "zxdg_output_v1" => xdg_ext::handle_output(&mut ctx),
        "zxdg_exporter_v2" => xdg_foreign::handle_exporter(&mut ctx),
        "zxdg_exported_v2" => xdg_foreign::handle_exported(&mut ctx),
        "zxdg_importer_v2" => xdg_foreign::handle_importer(&mut ctx),
        "zxdg_imported_v2" => xdg_foreign::handle_imported(&mut ctx),
        "wp_viewporter" => wp_ext::handle_viewporter(&mut ctx),
        "wp_viewport" => wp_ext::handle_viewport(&mut ctx),
        "wp_cursor_shape_manager_v1" => wp_ext::handle_cursor_shape_manager(&mut ctx),
        "wp_cursor_shape_device_v1" => wp_ext::handle_cursor_shape_device(&mut ctx),
        "wp_presentation" => wp_ext::handle_presentation(&mut ctx),
        "wp_presentation_feedback" => wp_ext::handle_presentation_feedback(&mut ctx),
        "wp_single_pixel_buffer_manager_v1" => wp_ext::handle_single_pixel_buffer_manager(&mut ctx),
        "wp_single_pixel_buffer_v1" => wp_ext::handle_single_pixel_buffer(&mut ctx),
        "wp_fractional_scale_manager_v1" => wp_ext::handle_fractional_scale_manager(&mut ctx),
        "wp_fractional_scale_v1" => wp_ext::handle_fractional_scale(&mut ctx),
        "wp_tearing_control_manager_v1" => wp_ext::handle_tearing_control_manager(&mut ctx),
        "wp_tearing_control_v1" => wp_ext::handle_tearing_control(&mut ctx),
        "zwp_linux_dmabuf_v1" => linux_ext::handle_dmabuf(&mut ctx),
        "linux_drm_syncobj_v1" => linux_ext::handle_syncobj(&mut ctx),
        "linux_drm_syncobj_surface_v1" => linux_ext::handle_syncobj_surface(&mut ctx),
        "linux_drm_syncobj_timeline_v1" => linux_ext::handle_syncobj_timeline(&mut ctx),
        "zwp_pointer_constraints_v1" => zwp_ext::handle_pointer_constraints(&mut ctx),
        "zwp_locked_pointer_v1" => zwp_ext::handle_locked_pointer(&mut ctx),
        "zwp_confined_pointer_v1" => zwp_ext::handle_confined_pointer(&mut ctx),
        "zwp_relative_pointer_manager_v1" => zwp_ext::handle_relative_pointer_manager(&mut ctx),
        "zwp_relative_pointer_v1" => zwp_ext::handle_relative_pointer(&mut ctx),
        "zwp_pointer_gestures_v1" => zwp_ext::handle_pointer_gestures(&mut ctx),
        "zwp_gesture_swipe_v1" | "zwp_gesture_pinch_v1" => zwp_ext::handle_gesture(&mut ctx),
        "zwp_text_input_manager_v1" => zwp_ext::handle_text_input_manager(&mut ctx),
        "zwp_text_input_v1" => zwp_ext::handle_text_input(&mut ctx),
        "zwp_idle_inhibit_manager_v1" => zwp_ext::handle_idle_inhibit_manager(&mut ctx),
        "zwp_idle_inhibitor_v1" => zwp_ext::handle_idle_inhibitor(&mut ctx),
        "zwlr_screencopy_manager_v1" => wlroots_ext::handle_screencopy_manager(&mut ctx),
        "zwlr_screencopy_frame_v1" => wlroots_ext::handle_screencopy_frame(&mut ctx),
        "zwlr_primary_selection_manager_v1" => wlroots_ext::handle_primary_selection_manager(&mut ctx),
        "zwlr_primary_selection_device_v1" => wlroots_ext::handle_primary_selection_device(&mut ctx),
        "zwlr_primary_selection_offer_v1" => wlroots_ext::handle_primary_selection_offer(&mut ctx),
        "zwlr_primary_selection_source_v1" => wlroots_ext::handle_primary_selection_source(&mut ctx),
        "zwlr_data_control_manager_v1" => wlroots_ext::handle_data_control_manager(&mut ctx),
        "zwlr_data_control_device_v1" => wlroots_ext::handle_data_control_device(&mut ctx),
        "zwlr_data_control_offer_v1" => wlroots_ext::handle_data_control_offer(&mut ctx),
        "zwlr_data_control_source_v1" => wlroots_ext::handle_data_control_source(&mut ctx),
        "zwp_text_input_manager_v3" => ime::handle_text_input_manager(&mut ctx),
        "zwp_text_input_v3" => ime::handle_text_input(&mut ctx),
        "zwp_input_method_manager_v2" => ime::handle_input_method_manager(&mut ctx),
        "zwp_input_method_v2" => ime::handle_input_method(&mut ctx),
        "zwp_input_method_keyboard_grab_v2" => ime::handle_keyboard_grab(&mut ctx),
        "edgerun_test_overlay" => {}
        _ => {}
    }
}

// Re-export process_input_for_device from the original module
pub use super::dispatch_legacy::process_input_for_device;
