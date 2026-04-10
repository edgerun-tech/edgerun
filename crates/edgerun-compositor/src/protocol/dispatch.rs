//! Protocol message dispatch — routes Wayland messages to handlers.
//!
//! This module contains the massive `process_message` function that dispatches
//! on interface names and opcodes. It is extracted from main.rs to keep
//! the binary entry point small and readable.

use std::collections::HashMap;

use crate::compositor::surface::{BufferRegistry, ShmBufferInfo, SurfaceBuffer, SurfaceTree};
use crate::compositor::shell::Shell;
use crate::compositor::seat::{self, Seat};

// Re-export constraint types from seat module for use in main.rs
pub use seat::{PointerConstraint, ConstraintType};
use crate::input::evdev::EvdevManager;
use crate::input::keymap::{self, Keymap, Modifiers};
use crate::protocol::linux_dmabuf;
use crate::protocol::linux_drm_syncobj;
use crate::protocol::primary_selection;
use crate::protocol::data_control;
use crate::protocol::screencopy;
use crate::protocol::screencopy::ScreencopyFrame;
use crate::protocol::text_input_v3;
use crate::protocol::text_input_v3::TextInputState;
use crate::protocol::input_method_v2;
use crate::protocol::input_method_v2::IMEState;
use crate::protocol::wl_compositor;
use crate::protocol::wl_core;
use crate::protocol::wl_data_device;
use crate::protocol::wl_seat;
use crate::protocol::wl_shm;
use crate::protocol::wl_subcompositor;
use crate::protocol::wp_cursor_shape;
use crate::protocol::wp_presentation_time;
use crate::protocol::wp_presentation_time::PresentationFeedbackTracker;
use crate::protocol::wp_viewporter;
use crate::protocol::xdg_activation;
use crate::protocol::xdg_decoration;
use crate::protocol::xdg_shell;
use crate::protocol::xdg_output;
use crate::protocol::xdg_foreign;
use crate::protocol::zwp_pointer_constraints;
use crate::protocol::zwp_pointer_gestures;
use crate::protocol::zwp_relative_pointer;
use crate::protocol::zwp_text_input;
use crate::protocol::zxdg_idle_inhibit;
use crate::protocol::single_pixel_buffer;
use crate::protocol::fractional_scale;
use crate::protocol::tearing_control;
use crate::render::cursor::Cursor;
use crate::render::shm::ShmManager;
use crate::resource::Registry;
use crate::server::WaylandServer;
use crate::wire;
use crate::wire::decode::ArgCursor;
use crate::wire::encode::*;
use crate::compositor::dmabuf::DmabufParams;

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

/// Screencopy state — holds the last rendered framebuffer snapshot.
pub struct ScreencopyState {
    /// Last rendered frame pixels (XRGB8888, little-endian).
    pub pixels: Vec<u8>,
    /// Frame dimensions.
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    /// Pending screencopy frames awaiting COPY.
    pub pending_frames: HashMap<u32, ScreencopyFrame>,
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
/// Each entry: (interface_name, version).
/// The global *name* (u32) is assigned dynamically in main.rs, but the
/// interface name and version are static here for sending global events.
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
    let interface: &str = {
        let reg = match client_registries.get(&client_id) {
            Some(r) => r,
            None => return,
        };
        reg.interface(msg.sender_id).unwrap_or("")
    };

    match interface {
        "wl_display" => {
            match msg.opcode {
                wl_core::display_request::SYNC => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let callback_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(callback_id, "wl_callback", 1, client_id);
                    }
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(wl_core::callback_done_event(callback_id, 0));
                    }
                }
                wl_core::display_request::GET_REGISTRY => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let registry_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(registry_id, "wl_registry", 1, client_id);
                    }
                    client_registry_ids.insert(client_id, registry_id);

                    // Send global events for all advertised globals
                    if let Some(client) = server.client_mut(client_id) {
                        for (global_name_idx, (interface, version)) in GLOBALS.iter().enumerate() {
                            let name = (global_name_idx + 1) as u32;
                            let evt = wl_core::global_event(name, interface, *version);
                            client.send_message(evt);
                        }
                    }
                }
                _ => {}
            }
        }

        "wl_registry" => {
            match msg.opcode {
                wl_core::registry_request::BIND => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let _name = cursor_obj.uint().unwrap_or(0);
                    let interface_name = match cursor_obj.string() {
                        Ok(Some(s)) => s.0,
                        _ => return,
                    };
                    let version = cursor_obj.uint().unwrap_or(0);
                    let id = cursor_obj.new_id().unwrap_or(0);

                    eprintln!("[edgerun-compositor] Client {} binding {} (v{}) -> id={}", client_id, interface_name, version, id);

                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(id, &interface_name, version, client_id);
                    }

                    match interface_name.as_str() {
                        "wl_compositor" => {
                            client_compositor_ids.insert(client_id, id);
                        }
                        "wl_shm" => {
                            client_shm_ids.insert(client_id, id);
                            if let Some(client) = server.client_mut(client_id) {
                                client.send_message(wl_shm::shm_format_event(id, wl_shm::format::XRGB8888));
                                client.send_message(wl_shm::shm_format_event(id, wl_shm::format::ARGB8888));
                                let _ = client.flush();
                            }
                        }
                        "wl_seat" => {
                            client_seat_ids.insert(client_id, id);
                            seat.id = id;
                            seat.capabilities = wl_seat::capability::KEYBOARD | wl_seat::capability::POINTER | wl_seat::capability::TOUCH;
                            if let Some(client) = server.client_mut(client_id) {
                                client.send_message(wl_seat::seat_capabilities_event(id, seat.capabilities));
                                // seat_name is v7+ — only send if client bound at v7+
                                if version >= 7 {
                                    client.send_message(wl_seat::seat_name_event(id, "seat0"));
                                }
                                let _ = client.flush();
                            }
                        }
                        "xdg_wm_base" => {
                            client_xdg_base_ids.insert(client_id, id);
                            shell.base_id = id;
                        }
                        "wl_output" => {
                            client_output_ids.insert(client_id, id);
                            // Send output events using actual detected mode data stored in shell
                            let actual_width = shell.output_width;
                            let actual_height = shell.output_height;
                            let actual_refresh = shell.output_refresh_mhz;
                            let mm_width = shell.output_mm_width;
                            let mm_height = shell.output_mm_height;
                            // Send output events — only those supported by the client's version
                            if let Some(client) = server.client_mut(client_id) {
                                // geometry (v1+)
                                let mut geo_args = Vec::new();
                                geo_args.extend_from_slice(&0i32.to_le_bytes()); // x
                                geo_args.extend_from_slice(&0i32.to_le_bytes()); // y
                                geo_args.extend_from_slice(&mm_width.to_le_bytes()); // physical_width
                                geo_args.extend_from_slice(&mm_height.to_le_bytes()); // physical_height
                                geo_args.extend_from_slice(&0i32.to_le_bytes()); // subpixel (unknown)
                                encode_string(&mut geo_args, "edgerun"); // make
                                encode_string(&mut geo_args, "edgerun-output"); // model
                                geo_args.extend_from_slice(&0i32.to_le_bytes()); // transform
                                client.send_message(wire::Message {
                                    sender_id: id,
                                    opcode: 0, // geometry
                                    size: (8 + geo_args.len()) as u16,
                                    args: geo_args,
                                    fds: Vec::new(),
                                });
                                // mode (v1+)
                                let mut mode_args = Vec::new();
                                mode_args.extend_from_slice(&3u32.to_le_bytes()); // CURRENT_PREFERRED
                                mode_args.extend_from_slice(&actual_width.to_le_bytes()); // width
                                mode_args.extend_from_slice(&actual_height.to_le_bytes()); // height
                                mode_args.extend_from_slice(&(actual_refresh).to_le_bytes()); // refresh (mHz)
                                client.send_message(wire::Message {
                                    sender_id: id,
                                    opcode: 1, // mode
                                    size: (8 + mode_args.len()) as u16,
                                    args: mode_args,
                                    fds: Vec::new(),
                                });
                                // scale (v3+)
                                if version >= 3 {
                                    let mut scale_args = Vec::new();
                                    scale_args.extend_from_slice(&shell.output_scale.to_le_bytes());
                                    client.send_message(wire::Message {
                                        sender_id: id,
                                        opcode: 3, // scale
                                        size: (8 + scale_args.len()) as u16,
                                        args: scale_args,
                                        fds: Vec::new(),
                                    });
                                }
                                // name (v4+)
                                if version >= 4 {
                                    let mut name_args = Vec::new();
                                    encode_string(&mut name_args, "edgerun-output-0");
                                    client.send_message(wire::Message {
                                        sender_id: id,
                                        opcode: 5, // name
                                        size: (8 + name_args.len()) as u16,
                                        args: name_args,
                                        fds: Vec::new(),
                                    });
                                }
                                // done (v4+) — but always send it even for v2 clients for compatibility
                                if version >= 4 {
                                    client.send_message(wire::Message {
                                        sender_id: id,
                                        opcode: 4, // done
                                        size: 8,
                                        args: Vec::new(),
                                        fds: Vec::new(),
                                    });
                                }
                                let _ = client.flush();
                            }
                        }
                        "zwp_linux_dmabuf_v1" => {
                            client_dmabuf_ids.insert(client_id, id);
                            // Send format + modifier events (v3+)
                            if let Some(client) = server.client_mut(client_id) {
                                for (format, modifier) in linux_dmabuf::common_formats_with_modifiers() {
                                    let modifier_hi = (modifier >> 32) as u32;
                                    let modifier_lo = (modifier & 0xFFFFFFFF) as u32;
                                    client.send_message(linux_dmabuf::dmabuf_modifier_event(id, format, modifier_hi, modifier_lo));
                                }
                                let _ = client.flush();
                            }
                        }
                        "wl_data_device_manager" => {
                            // No event sent, client just binds
                        }
                        "wl_subcompositor" => {
                            client_subcompositor_ids.insert(client_id, id);
                        }
                        "zxdg_decoration_manager_v1" => {
                            client_decoration_manager_ids.insert(client_id, id);
                        }
                        "wp_viewporter" => {
                            client_viewporter_ids.insert(client_id, id);
                        }
                        "wp_cursor_shape_manager_v1" => {
                            client_cursor_shape_manager_ids.insert(client_id, id);
                        }
                        "xdg_activation_v1" => {
                            // Client bound the activation global, no state needed
                        }
                        "wp_presentation" => {
                            // Client bound the presentation global, no state needed
                        }
                        "zwp_relative_pointer_manager_v1" => {
                            // Client bound the relative pointer manager global, no state needed
                        }
                        "zwp_pointer_gestures_v1" => {
                            // Client bound the pointer gestures global, no state needed
                        }
                        "zwp_text_input_manager_v1" => {
                            // Client bound the text input manager global, no state needed
                        }
                        "zwp_idle_inhibit_manager_v1" => {
                            // Client bound the idle inhibit manager global, no state needed
                        }
                        "zxdg_output_manager_v1" => {
                            client_output_ids.insert(client_id, id);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        "wl_compositor" => {
            match msg.opcode {
                wl_compositor::compositor_request::CREATE_SURFACE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let surface_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(surface_id, "wl_surface", 4, client_id);
                    }
                    surfaces.create(surface_id);
                }
                wl_compositor::compositor_request::CREATE_REGION => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let region_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(region_id, "wl_region", 1, client_id);
                    }
                    // Region created — state tracked per-surface via SET_OPAQUE_REGION/SET_INPUT_REGION
                }
                _ => {}
            }
        }

        "wl_surface" => {
            match msg.opcode {
                wl_compositor::surface_request::ATTACH => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let buffer_id = cursor_obj.object().unwrap_or(0);
                    let x = cursor_obj.int().unwrap_or(0);
                    let y = cursor_obj.int().unwrap_or(0);

                    if buffer_id == 0 {
                        surfaces.attach(msg.sender_id, SurfaceBuffer::Null, x, y);
                    } else if let Some(info) = buffers.get(buffer_id) {
                        surfaces.attach(msg.sender_id, SurfaceBuffer::Shm {
                            pool_fd: info.pool_fd,
                            offset: info.offset,
                            width: info.width,
                            height: info.height,
                            stride: info.stride,
                            format: info.format,
                        }, x, y);
                    }
                }
                wl_compositor::surface_request::DAMAGE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let x = cursor_obj.int().unwrap_or(0);
                    let y = cursor_obj.int().unwrap_or(0);
                    let w = cursor_obj.int().unwrap_or(0);
                    let h = cursor_obj.int().unwrap_or(0);
                    surfaces.damage(msg.sender_id, x, y, w, h);
                }
                wl_compositor::surface_request::FRAME => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let callback_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(callback_id, "wl_callback", 1, client_id);
                    }
                    surfaces.add_frame_callback(msg.sender_id, callback_id);
                }
                wl_compositor::surface_request::COMMIT => {
                    // Supersede any pending presentation feedback for this surface
                    // (new buffer committed before previous one was presented).
                    for (feedback_id, client_id) in presentation_tracker.supersede(msg.sender_id) {
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(wp_presentation_time::feedback_discarded_event(
                                feedback_id,
                                wp_presentation_time::discard_reason::SUPERSEDED,
                            ));
                        }
                    }
                    surfaces.commit(msg.sender_id);

                    // P0: Send enter event for this surface on all output objects
                    if let Some(surface) = surfaces.get(msg.sender_id) {
                        if surface.buffer.is_some() {
                            for (&cid, &output_id) in client_output_ids.iter() {
                                if let Some(client) = server.client_mut(cid) {
                                    client.send_message(wl_compositor::surface_enter_event(
                                        msg.sender_id, output_id,
                                    ));
                                }
                                // Also send preferred_buffer_scale (v4)
                                if let Some(client) = server.client_mut(cid) {
                                    client.send_message(wl_compositor::surface_preferred_buffer_scale_event(
                                        msg.sender_id, surface.buffer_scale as u32,
                                    ));
                                }
                            }
                        }
                    }
                }
                wl_compositor::surface_request::SET_BUFFER_SCALE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let scale = cursor_obj.int().unwrap_or(1);
                    surfaces.set_buffer_scale(msg.sender_id, scale);
                }
                wl_compositor::surface_request::SET_BUFFER_TRANSFORM => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let transform = cursor_obj.int().unwrap_or(0);
                    surfaces.set_buffer_transform(msg.sender_id, transform);
                }
                wl_compositor::surface_request::SET_OPAQUE_REGION => {
                    let region_id = ArgCursor::from_message(&msg).object().unwrap_or(0);
                    // A non-null region_id means the surface is opaque where the region is.
                    // null (0) clears the opaque region → surface is fully transparent.
                    if let Some(surface) = surfaces.get_mut(msg.sender_id) {
                        surface.opaque = region_id != 0;
                    }
                }
                wl_compositor::surface_request::SET_INPUT_REGION => {
                    let _region_id = ArgCursor::from_message(&msg).object().unwrap_or(0);
                    // Input region is tracked for hit-testing. For now we accept it;
                    // a full implementation would store the region and use it for pointer hit tests.
                }
                wl_compositor::surface_request::DESTROY => {
                    // P0: Send leave event for this surface on all output objects
                    if let Some(surface) = surfaces.get(msg.sender_id) {
                        if surface.buffer.is_some() {
                            for (&cid, &output_id) in client_output_ids.iter() {
                                if let Some(client) = server.client_mut(cid) {
                                    client.send_message(wl_compositor::surface_leave_event(
                                        msg.sender_id, output_id,
                                    ));
                                }
                            }
                        }
                    }
                    surfaces.destroy(msg.sender_id);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        "wl_shm" => {
            match msg.opcode {
                wl_shm::shm_request::CREATE_POOL => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let pool_id = cursor_obj.new_id().unwrap_or(0);
                    let size = cursor_obj.int().unwrap_or(0);

                    // FD is passed via SCM_RIGHTS only - no wire placeholder
                    let fd = if !msg.fds.is_empty() { msg.fds[0] } else { -1 };

                    let client_pools = client_pool_map.entry(client_id).or_insert_with(HashMap::new);
                    client_pools.insert(pool_id, 0);

                    if let Ok(_internal_id) = shm.create_pool(pool_id, fd, size) {
                        if let Some(reg) = client_registries.get_mut(&client_id) {
                            reg.register(pool_id, "wl_shm_pool", 1, client_id);
                        }
                    }
                }
                _ => {}
            }
        }

        "wl_shm_pool" => {
            match msg.opcode {
                wl_shm::shm_pool_request::CREATE_BUFFER => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let buffer_id = cursor_obj.new_id().unwrap_or(0);
                    let offset = cursor_obj.int().unwrap_or(0);
                    let width = cursor_obj.int().unwrap_or(0);
                    let height = cursor_obj.int().unwrap_or(0);
                    let stride = cursor_obj.int().unwrap_or(0);
                    let format = cursor_obj.uint().unwrap_or(0);

                    let pool_fd = shm.pool_fd_by_client_id(msg.sender_id).unwrap_or(-1);

                    buffers.register(buffer_id, ShmBufferInfo {
                        pool_fd,
                        offset,
                        width,
                        height,
                        stride,
                        format,
                    });

                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(buffer_id, "wl_buffer", 1, client_id);
                    }
                }
                wl_shm::shm_pool_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                wl_shm::shm_pool_request::RESIZE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let new_size = cursor_obj.int().unwrap_or(0);
                    // Resize the pool — remap if needed
                    if let Some(pool) = shm.get_pool_by_client_id(msg.sender_id) {
                        // Just update our tracking; actual remap happens on next read
                        if new_size as usize > pool.size {
                            // Unmap old, mmap new
                            unsafe { libc::munmap(pool.mapping as *mut libc::c_void, pool.size) };
                            let new_ptr = unsafe {
                                libc::mmap(std::ptr::null_mut(), new_size as usize,
                                    libc::PROT_READ, libc::MAP_SHARED, pool.fd, 0)
                            };
                            if new_ptr != libc::MAP_FAILED {
                                pool.mapping = new_ptr as *mut u8;
                                pool.size = new_size as usize;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        "wl_seat" => {
            match msg.opcode {
                wl_seat::seat_request::GET_KEYBOARD => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let kb_id = cursor_obj.new_id().unwrap_or(0);
                    // Use the seat's version (capped at 7 for wl_keyboard)
                    let kb_version = if let Some(res) = client_registries.get(&client_id).and_then(|r| r.get(msg.sender_id)) {
                        res.version.min(7)
                    } else {
                        7
                    };
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(kb_id, "wl_keyboard", kb_version, client_id);
                    }
                    client_keyboard_ids.insert(client_id, kb_id);
                    eprintln!("[edgerun-compositor] Client {} created wl_keyboard id={} (v{})", client_id, kb_id, kb_version);

                    // Send keymap to the keyboard client
                    let keymap_str = keymap::xkb_keymap_text();  // US layout for maximum compatibility
                    let keymap_size = keymap_str.len() as u32;

                    // Create an anonymous tmpfile for the keymap
                    let fd = unsafe {
                        libc::open(b"/dev/shm/edgerun-km\0".as_ptr() as *const libc::c_char,
                            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
                            0o600)
                    };
                    if fd < 0 {
                        eprintln!("[edgerun-compositor] Failed to create keymap tmpfile");
                    } else {
                        // Unlink immediately (anonymous file)
                        unsafe { libc::unlink(b"/dev/shm/edgerun-km\0".as_ptr() as *const libc::c_char) };

                        // Write keymap data
                        let written = unsafe {
                            libc::write(fd, keymap_str.as_ptr() as *const _, keymap_str.len())
                        };
                        if written < 0 {
                            eprintln!("[edgerun-compositor] Failed to write keymap");
                        } else {
                            // Duplicate FD for sending (SCM_RIGHTS transfers ownership)
                            let dup_fd = unsafe { libc::dup(fd) };
                            if dup_fd >= 0 {
                                if let Some(client) = server.client_mut(client_id) {
                                    client.send_message(wl_seat::keyboard_keymap_event(
                                        kb_id,
                                        1,
                                        dup_fd,
                                        keymap_size,
                                    ));
                                    // P0: REPEAT_INFO — tell client the auto-repeat rate
                                    client.send_message(wl_seat::keyboard_repeat_info_event(
                                        kb_id,
                                        25, // 25 chars/sec
                                        300, // 300ms delay
                                    ));
                                    let _ = client.flush();
                                }
                            }
                        }
                        unsafe { libc::close(fd) };
                    }
                }
                wl_seat::seat_request::GET_POINTER => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let ptr_id = cursor_obj.new_id().unwrap_or(0);
                    let ptr_version = if let Some(res) = client_registries.get(&client_id).and_then(|r| r.get(msg.sender_id)) {
                        res.version.min(7)
                    } else {
                        7
                    };
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(ptr_id, "wl_pointer", ptr_version, client_id);
                    }
                    client_pointer_ids.insert(client_id, ptr_id);
                    eprintln!("[edgerun-compositor] Client {} created wl_pointer id={} (v{})", client_id, ptr_id, ptr_version);
                }
                wl_seat::seat_request::GET_TOUCH => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let touch_id = cursor_obj.new_id().unwrap_or(0);
                    let touch_version = if let Some(res) = client_registries.get(&client_id).and_then(|r| r.get(msg.sender_id)) {
                        res.version.min(7)
                    } else {
                        7
                    };
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(touch_id, "wl_touch", touch_version, client_id);
                    }
                    client_touch_ids.insert(client_id, touch_id);
                    eprintln!("[edgerun-compositor] Client {} created wl_touch id={} (v{})", client_id, touch_id, touch_version);
                }
                wl_seat::seat_request::RELEASE => {}
                _ => {}
            }
        }

        "wl_keyboard" => {
            match msg.opcode {
                wl_seat::keyboard_request::RELEASE => {}
                _ => {}
            }
        }

        "wl_pointer" => {
            match msg.opcode {
                wl_seat::pointer_request::SET_CURSOR => {
                    // Client wants to set a cursor surface — we use a fixed Adwaita
                    // cursor, so just hide the cursor if the client sends surface_id=0.
                    let surface_id = ArgCursor::from_message(&msg).object().unwrap_or(0);
                    cursor.visible = surface_id != 0;
                }
                wl_seat::pointer_request::RELEASE => {}
                _ => {}
            }
        }

        "wl_data_device_manager" => {
            match msg.opcode {
                wl_data_device::dnd_manager_request::CREATE_DATA_SOURCE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let source_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(source_id, "wl_data_source", 3, client_id);
                    }
                    client_data_source_ids.insert(client_id, source_id);
                }
                wl_data_device::dnd_manager_request::GET_DATA_DEVICE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let device_id = cursor_obj.new_id().unwrap_or(0);
                    let _seat_obj_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(device_id, "wl_data_device", 3, client_id);
                    }
                    client_data_device_ids.insert(client_id, device_id);
                }
                wl_data_device::dnd_manager_request::DESTROY => {}
                _ => {}
            }
        }

        "wl_data_source" => {
            match msg.opcode {
                wl_data_device::data_source_request::OFFER => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    if let Ok(Some(mime_type)) = cursor_obj.string() {
                        // Store mime type in the current data source
                        if let Some(source) = current_data_source {
                            source.mime_types.push(mime_type.0);
                        }
                    }
                }
                wl_data_device::data_source_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    client_data_source_ids.remove(&client_id);
                }
                wl_data_device::data_source_request::SET_ACTIONS => {}
                _ => {}
            }
        }

        "wl_data_offer" => {
            match msg.opcode {
                wl_data_device::data_offer_request::ACCEPT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let _serial = cursor_obj.uint().unwrap_or(0);
                    // Client accepts a mime type (or empty string for none)
                    let _mime_type = cursor_obj.string().ok().flatten();
                }
                wl_data_device::data_offer_request::RECEIVE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    if let Ok(Some(mime_type)) = cursor_obj.string() {
                        // FD is passed via SCM_RIGHTS
                        let fd = if !msg.fds.is_empty() { msg.fds[0] } else { -1 };

                        // Forward the RECEIVE request to the source client.
                        // The source client will write data to the fd, which the
                        // requesting client reads.
                        if fd >= 0 {
                            if let Some(ref source) = *current_data_source {
                                // Send SEND event to the source client
                                if let Some(source_client) = server.client_mut(source.owner_client_id) {
                                    source_client.send_message(wl_data_device::data_source_send_event(
                                        source.id,
                                        &mime_type.0,
                                        fd,
                                    ));
                                }
                            }
                        }
                    }
                }
                wl_data_device::data_offer_request::DISCRIPTION => {}
                wl_data_device::data_offer_request::SET_ACTIONS => {}
                wl_data_device::data_offer_request::DESTROY => {}
                _ => {}
            }
        }

        "wl_data_device" => {
            match msg.opcode {
                wl_data_device::data_device_request::SET_SELECTION => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let source_id = cursor_obj.object().unwrap_or(0);
                    let _serial = cursor_obj.uint().unwrap_or(0);

                    if source_id != 0 {
                        *selection_offer_counter += 1;
                        let offer_id = *selection_offer_counter;

                        // Store the current data source with its owner client
                        *current_data_source = Some(DataSource {
                            id: source_id,
                            owner_client_id: client_id,
                            mime_types: Vec::new(),
                        });

                        // Broadcast selection to ALL clients (not just the one that set it)
                        for (&cid, &device_id) in client_data_device_ids.iter() {
                            if let Some(client) = server.client_mut(cid) {
                                // Send data_offer first, then selection
                                client.send_message(wl_data_device::data_device_data_offer_event(device_id, offer_id));
                                client.send_message(wl_data_device::data_device_selection_event(device_id, offer_id));
                                // Send available mime types via the offer
                                if let Some(ref source) = *current_data_source {
                                    for mime in &source.mime_types {
                                        client.send_message(wl_data_device::data_offer_offer_event(offer_id, mime));
                                    }
                                }
                            }
                        }
                    } else {
                        // Clear selection — broadcast to all clients
                        *current_data_source = None;
                        for (&cid, &device_id) in client_data_device_ids.iter() {
                            if let Some(client) = server.client_mut(cid) {
                                client.send_message(wl_data_device::data_device_selection_event(device_id, 0));
                            }
                        }
                    }
                }
                wl_data_device::data_device_request::START_DRAG => {}
                wl_data_device::data_device_request::RELEASE => {}
                _ => {}
            }
        }

        "wl_subcompositor" => {
            match msg.opcode {
                wl_subcompositor::subcompositor_request::GET_SUBSURFACE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let subsurface_id = cursor_obj.new_id().unwrap_or(0);
                    let surface_id = cursor_obj.object().unwrap_or(0);
                    let parent_id = cursor_obj.object().unwrap_or(0);

                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(subsurface_id, "wl_subsurface", 1, client_id);
                    }

                    shell.subsurfaces.create(surface_id, parent_id);
                }
                wl_subcompositor::subcompositor_request::DESTROY => {}
                _ => {}
            }
        }

        "wl_subsurface" => {
            match msg.opcode {
                wl_subcompositor::subsurface_request::SET_POSITION => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let x = cursor_obj.int().unwrap_or(0);
                    let y = cursor_obj.int().unwrap_or(0);

                    if let Some(sub) = shell.subsurfaces.get_mut(msg.sender_id) {
                        sub.x = x;
                        sub.y = y;
                    }
                    // Also update surface position
                    if let Some(surface) = surfaces.get_mut(msg.sender_id) {
                        surface.x = x;
                        surface.y = y;
                    }
                }
                wl_subcompositor::subsurface_request::SET_SYNC => {
                    if let Some(sub) = shell.subsurfaces.get_mut(msg.sender_id) {
                        sub.sync = true;
                    }
                }
                wl_subcompositor::subsurface_request::SET_DESYNC => {
                    if let Some(sub) = shell.subsurfaces.get_mut(msg.sender_id) {
                        sub.sync = false;
                    }
                }
                wl_subcompositor::subsurface_request::PLACE_ABOVE => {}
                wl_subcompositor::subsurface_request::PLACE_BELOW => {}
                wl_subcompositor::subsurface_request::DESTROY => {
                    shell.subsurfaces.destroy(msg.sender_id);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        "zxdg_decoration_manager_v1" => {
            match msg.opcode {
                xdg_decoration::decoration_manager_request::GET_TOPLEVEL_DECORATION => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let decoration_id = cursor_obj.new_id().unwrap_or(0);
                    let toplevel_id = cursor_obj.object().unwrap_or(0);

                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(decoration_id, "zxdg_toplevel_decoration_v1", 1, client_id);
                    }
                    client_decoration_ids.insert(client_id, decoration_id);

                    // Tell the toplevel about the decoration object
                    if let Some(tl) = shell.toplevels.values_mut().find(|tl| tl.id == toplevel_id) {
                        tl.decoration_id = Some(decoration_id);
                        // Default to server-side decorations
                        tl.decoration_mode = Some(xdg_decoration::decoration_mode::SERVER_SIDE);
                    }

                    // Send configure event
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(xdg_decoration::toplevel_decoration_configure_event(
                            decoration_id,
                            xdg_decoration::decoration_mode::SERVER_SIDE,
                        ));
                    }
                }
                xdg_decoration::decoration_manager_request::DESTROY => {}
                _ => {}
            }
        }

        "zxdg_toplevel_decoration_v1" => {
            match msg.opcode {
                xdg_decoration::toplevel_decoration_request::SET_MODE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let mode = cursor_obj.uint().unwrap_or(0);

                    if let Some(tl) = shell.toplevels.values_mut().find(|tl| tl.decoration_id == Some(msg.sender_id)) {
                        tl.decoration_mode = Some(mode);
                        // Acknowledge the mode change
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(xdg_decoration::toplevel_decoration_configure_event(
                                msg.sender_id, mode,
                            ));
                        }
                    }
                }
                xdg_decoration::toplevel_decoration_request::UNSET_MODE => {
                    if let Some(tl) = shell.toplevels.values_mut().find(|tl| tl.decoration_id == Some(msg.sender_id)) {
                        tl.decoration_mode = None;
                    }
                }
                xdg_decoration::toplevel_decoration_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    client_decoration_ids.remove(&client_id);
                }
                _ => {}
            }
        }

        "wp_viewporter" => {
            match msg.opcode {
                wp_viewporter::viewporter_request::GET_VIEWPORT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let viewport_id = cursor_obj.new_id().unwrap_or(0);
                    let _surface_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(viewport_id, "wp_viewport", 1, client_id);
                    }
                    client_viewporter_ids.insert(client_id, viewport_id);
                }
                wp_viewporter::viewporter_request::DESTROY => {}
                _ => {}
            }
        }

        "wp_viewport" => {
            match msg.opcode {
                wp_viewporter::viewport_request::SET_SOURCE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let x = wp_viewporter::fixed_to_f64(cursor_obj.fixed().unwrap_or(0));
                    let y = wp_viewporter::fixed_to_f64(cursor_obj.fixed().unwrap_or(0));
                    let w = wp_viewporter::fixed_to_f64(cursor_obj.fixed().unwrap_or(0));
                    let h = wp_viewporter::fixed_to_f64(cursor_obj.fixed().unwrap_or(0));
                    if w > 0.0 && h > 0.0 {
                        surfaces.set_viewport_source(msg.sender_id, x, y, w, h);
                    } else {
                        // Reset viewport (use full buffer)
                        surfaces.set_viewport_source(msg.sender_id, 0.0, 0.0, 0.0, 0.0);
                    }
                }
                wp_viewporter::viewport_request::SET_DESTINATION => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let width = cursor_obj.int().unwrap_or(-1);
                    let height = cursor_obj.int().unwrap_or(-1);
                    surfaces.set_viewport_destination(msg.sender_id, width, height);
                }
                wp_viewporter::viewport_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    client_viewporter_ids.remove(&client_id);
                }
                _ => {}
            }
        }

        "wp_cursor_shape_manager_v1" => {
            match msg.opcode {
                wp_cursor_shape::cursor_shape_manager_request::GET_POINTER_SHAPE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let device_id = cursor_obj.new_id().unwrap_or(0);
                    let _serial = cursor_obj.uint().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(device_id, "wp_cursor_shape_device_v1", 1, client_id);
                    }
                    client_cursor_shape_device_ids.insert(client_id, device_id);
                }
                wp_cursor_shape::cursor_shape_manager_request::DESTROY => {}
                _ => {}
            }
        }

        "wp_cursor_shape_device_v1" => {
            match msg.opcode {
                wp_cursor_shape::cursor_shape_device_request::SET_SHAPE => {
                    let shape_id = ArgCursor::from_message(&msg).uint().unwrap_or(0);
                    cursor.set_shape(crate::render::cursor::shape_id_to_index(shape_id));
                }
                wp_cursor_shape::cursor_shape_device_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    client_cursor_shape_device_ids.remove(&client_id);
                }
                _ => {}
            }
        }

        "xdg_activation_v1" => {
            match msg.opcode {
                xdg_activation::xdg_activation_request::GET_ACTIVATION_TOKEN => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let token_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(token_id, "xdg_activation_token_v1", 1, client_id);
                    }
                    // Token is created, client can set properties on it before activate
                }
                xdg_activation::xdg_activation_request::ACTIVATE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let _token = cursor_obj.string().ok().flatten().unwrap_or(crate::wire::WireString(String::new()));
                    let surface_id = cursor_obj.object().unwrap_or(0);

                    // Find the toplevel that owns this surface and activate it
                    if let Some(tl) = shell.toplevel_for_surface(surface_id) {
                        let tl_id = tl.id;
                        // Bring to front
                        shell.activate(tl_id);

                        // Set keyboard focus to this surface
                        let serial = *config_serial;
                        *config_serial += 1;
                        if let Some(surface) = surfaces.get_mut(surface_id) {
                            // Surface exists, send keyboard enter if not already focused
                            if seat.keyboard_focus() != Some(surface_id) {
                                if let Some(old_sid) = seat.keyboard_focus() {
                                    for (&cid, &kb_id) in client_keyboard_ids.iter() {
                                        if let Some(client) = server.client_mut(cid) {
                                            client.send_message(wl_seat::keyboard_leave_event(
                                                kb_id, serial, old_sid));
                                        }
                                    }
                                }
                                seat.set_keyboard_focus(Some(surface_id));
                                for (&cid, &kb_id) in client_keyboard_ids.iter() {
                                    if let Some(client) = server.client_mut(cid) {
                                        client.send_message(wl_seat::keyboard_enter_event(
                                            kb_id, serial, surface_id, &[]));
                                        client.send_message(wl_seat::keyboard_modifiers_event(
                                            kb_id, serial,
                                            if modifiers.shift { 1 } else { 0 },
                                            if modifiers.caps { 2 } else { 0 },
                                            0, 0,
                                        ));
                                    }
                                }

                                // Send configure to the activated toplevel
                                let output_w = shell.output_width;
                                let output_h = shell.output_height;
                                let cfg_serial = shell.configure_toplevel_with_state(tl_id, output_w, output_h);
                                if let Some(client) = server.client_mut(client_id) {
                                    let state = shell.toplevel_state_bytes(tl_id).to_vec();
                                    client.send_message(xdg_shell::xdg_toplevel_configure_event(
                                        tl_id, output_w, output_h, &state));
                                }

                                eprintln!("[edgerun-compositor] Activated surface {} (toplevel {})", surface_id, tl_id);
                            }
                        }
                    } else {
                        eprintln!("[edgerun-compositor] Activation failed: surface {} not a toplevel", surface_id);
                    }
                }
                xdg_activation::xdg_activation_request::DESTROY => {}
                _ => {}
            }
        }

        "xdg_activation_token_v1" => {
            match msg.opcode {
                xdg_activation::xdg_activation_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        "wp_presentation" => {
            match msg.opcode {
                wp_presentation_time::presentation_request::FEEDBACK => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let surface_id = cursor_obj.object().unwrap_or(0);
                    let feedback_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(feedback_id, "wp_presentation_feedback", 1, client_id);
                    }
                    // Track this feedback — it will be sent `presented` after the next page flip,
                    // or `discarded` if the surface commits a new buffer before then.
                    presentation_tracker.register(surface_id, feedback_id, client_id);
                }
                wp_presentation_time::presentation_request::DESTROY => {}
                _ => {}
            }
        }

        "wp_presentation_feedback" => {
            match msg.opcode {
                wp_presentation_time::presentation_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        "zwp_relative_pointer_manager_v1" => {
            match msg.opcode {
                zwp_relative_pointer::relative_pointer_manager_request::GET_RELATIVE_POINTER => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let _pointer_id = cursor_obj.object().unwrap_or(0);
                    let relative_pointer_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(relative_pointer_id, "zwp_relative_pointer_v1", 1, client_id);
                    }
                    client_relative_pointer_ids.insert(client_id, relative_pointer_id);
                }
                zwp_relative_pointer::relative_pointer_manager_request::DESTROY => {}
                _ => {}
            }
        }

        "zwp_relative_pointer_v1" => {
            match msg.opcode {
                zwp_relative_pointer::relative_pointer_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        "zwp_pointer_gestures_v1" => {
            match msg.opcode {
                zwp_pointer_gestures::pointer_gestures_request::GET_SWIPE_GESTURE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let _pointer_id = cursor_obj.object().unwrap_or(0);
                    let gesture_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(gesture_id, "zwp_gesture_swipe_v1", 1, client_id);
                    }
                    // Client can now receive swipe gesture events
                }
                zwp_pointer_gestures::pointer_gestures_request::GET_PINCH_GESTURE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let _pointer_id = cursor_obj.object().unwrap_or(0);
                    let gesture_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(gesture_id, "zwp_gesture_pinch_v1", 1, client_id);
                    }
                    // Client can now receive pinch gesture events
                }
                zwp_pointer_gestures::pointer_gestures_request::DESTROY => {}
                _ => {}
            }
        }

        "zwp_gesture_swipe_v1" | "zwp_gesture_pinch_v1" => {
            match msg.opcode {
                zwp_pointer_gestures::gesture_swipe_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        "zwp_text_input_manager_v1" => {
            match msg.opcode {
                zwp_text_input::text_input_manager_request::CREATE_TEXT_INPUT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let text_input_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(text_input_id, "zwp_text_input_v1", 1, client_id);
                    }
                    // Text input object created, client will configure it
                }
                zwp_text_input::text_input_manager_request::DESTROY => {}
                _ => {}
            }
        }

        "zwp_text_input_v1" => {
            match msg.opcode {
                zwp_text_input::text_input_request::ACTIVATE => {
                    // Client activated text input - send enter event
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let _seat_id = cursor_obj.object().unwrap_or(0);
                    let surface_id = cursor_obj.object().unwrap_or(0);
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(zwp_text_input::enter_event(msg.sender_id, surface_id));
                    }
                }
                zwp_text_input::text_input_request::DEACTIVATE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let surface_id = cursor_obj.object().unwrap_or(0);
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(zwp_text_input::leave_event(msg.sender_id, surface_id));
                    }
                }
                zwp_text_input::text_input_request::COMMIT_STATE => {
                    // Client committed text state - process it
                    // In full implementation, track surrounding text, content type, etc.
                }
                zwp_text_input::text_input_request::SHOW_INPUT_PANEL => {
                    // Show virtual keyboard (not applicable for physical keyboards)
                }
                zwp_text_input::text_input_request::HIDE_INPUT_PANEL => {
                    // Hide virtual keyboard
                }
                zwp_text_input::text_input_request::RESET => {
                    // Reset text input state
                }
                zwp_text_input::text_input_request::SET_SURROUNDING_TEXT => {
                    // Track surrounding text for IME context
                }
                zwp_text_input::text_input_request::SET_CONTENT_TYPE => {
                    // Track content type (password, email, etc.) for IME
                }
                zwp_text_input::text_input_request::SET_CURSOR_RECTANGLE => {
                    // Track cursor position for IME candidate window placement
                }
                zwp_text_input::text_input_request::SET_PREFERRED_LANGUAGE => {
                    // Track preferred language
                }
                zwp_text_input::text_input_request::INVOKE_ACTION => {
                    // Invoke action (rarely used)
                }
                zwp_text_input::text_input_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        "zxdg_output_manager_v1" => {
            match msg.opcode {
                xdg_output::output_manager_request::GET_XDG_OUTPUT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let xdg_output_id = cursor_obj.new_id().unwrap_or(0);
                    let _wl_output_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(xdg_output_id, xdg_output::ZXDG_OUTPUT_V1, 3, client_id);
                    }
                    // Send output description events
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(xdg_output::xdg_output_logical_position(xdg_output_id, 0, 0));
                        client.send_message(xdg_output::xdg_output_logical_size(xdg_output_id, shell.output_width, shell.output_height));
                        client.send_message(xdg_output::xdg_output_name_event(xdg_output_id, "edgerun-output-0"));
                        client.send_message(xdg_output::xdg_output_description_event(xdg_output_id, "edgerun compositor output"));
                        client.send_message(xdg_output::xdg_output_done_event(xdg_output_id));
                        let _ = client.flush();
                    }
                }
                xdg_output::output_manager_request::DESTROY => {}
                _ => {}
            }
        }

        "zxdg_output_v1" => {
            match msg.opcode {
                xdg_output::output_manager_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        "zxdg_exporter_v2" => {
            match msg.opcode {
                xdg_foreign::exporter_request::EXPORT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let exported_id = cursor_obj.new_id().unwrap_or(0);
                    let _surface_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(exported_id, xdg_foreign::ZXDG_EXPORTED_V2, 1, client_id);
                    }
                    // Generate a unique handle string
                    let handle = format!("{:x}-{:x}", client_id, exported_id);
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(xdg_foreign::exported_handle_event(exported_id, &handle));
                        let _ = client.flush();
                    }
                }
                xdg_foreign::exporter_request::DESTROY => {}
                _ => {}
            }
        }

        "zxdg_exported_v2" => {
            match msg.opcode {
                xdg_foreign::exported_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        "zxdg_importer_v2" => {
            match msg.opcode {
                xdg_foreign::importer_request::IMPORT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let imported_id = cursor_obj.new_id().unwrap_or(0);
                    let _handle = cursor_obj.string().ok().flatten();
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(imported_id, xdg_foreign::ZXDG_IMPORTED_V2, 1, client_id);
                    }
                }
                xdg_foreign::importer_request::DESTROY => {}
                _ => {}
            }
        }

        "zxdg_imported_v2" => {
            match msg.opcode {
                xdg_foreign::imported_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                xdg_foreign::imported_request::SET_PARENT_OF => {
                    // Client sets this imported surface as parent of another surface
                    // TODO: Validate that the surface handle exists and is visible
                    let _ = msg;
                }
                _ => {}
            }
        }

        "zwp_pointer_constraints_v1" => {
            match msg.opcode {
                zwp_pointer_constraints::constraints_request::LOCK_POINTER => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let locked_id = cursor_obj.new_id().unwrap_or(0);
                    let surface_id = cursor_obj.object().unwrap_or(0);
                    let pointer_id = cursor_obj.object().unwrap_or(0);
                    let lifetime = cursor_obj.uint().unwrap_or(0);
                    let _region = cursor_obj.object().ok();
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(locked_id, zwp_pointer_constraints::ZWP_LOCKED_POINTER_V1, 1, client_id);
                    }
                    // Store constraint
                    pointer_constraints.insert(locked_id, PointerConstraint {
                        constraint_id: locked_id,
                        surface_id,
                        pointer_id,
                        lifetime,
                        region: None,
                        activated: true,
                    });
                    constraint_type_map.insert(locked_id, ConstraintType::Lock);
                    // Send locked event
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(zwp_pointer_constraints::locked_pointer_locked_event(locked_id));
                        let _ = client.flush();
                    }
                }
                zwp_pointer_constraints::constraints_request::CONFINE_POINTER => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let confined_id = cursor_obj.new_id().unwrap_or(0);
                    let surface_id = cursor_obj.object().unwrap_or(0);
                    let pointer_id = cursor_obj.object().unwrap_or(0);
                    let lifetime = cursor_obj.uint().unwrap_or(0);
                    let _region = cursor_obj.object().ok();
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(confined_id, zwp_pointer_constraints::ZWP_CONFINED_POINTER_V1, 1, client_id);
                    }
                    // Store constraint
                    pointer_constraints.insert(confined_id, PointerConstraint {
                        constraint_id: confined_id,
                        surface_id,
                        pointer_id,
                        lifetime,
                        region: None,
                        activated: true,
                    });
                    constraint_type_map.insert(confined_id, ConstraintType::Confine);
                    // Send confined event
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(zwp_pointer_constraints::confined_pointer_confined_event(confined_id));
                        let _ = client.flush();
                    }
                }
                zwp_pointer_constraints::constraints_request::DESTROY => {}
                _ => {}
            }
        }

        "zwp_locked_pointer_v1" => {
            match msg.opcode {
                zwp_pointer_constraints::locked_pointer_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    // Remove constraint
                    pointer_constraints.remove(&msg.sender_id);
                    constraint_type_map.remove(&msg.sender_id);
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(zwp_pointer_constraints::locked_pointer_unlocked_event(msg.sender_id));
                    }
                }
                zwp_pointer_constraints::locked_pointer_request::SET_CURSOR_POSITION_HINT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let _x = cursor_obj.fixed().unwrap_or(0);
                    let _y = cursor_obj.fixed().unwrap_or(0);
                    // Store hint for future use
                }
                zwp_pointer_constraints::locked_pointer_request::SET_REGION => {
                    let _region_id = ArgCursor::from_message(&msg).object().ok();
                    // Store region for future use
                }
                _ => {}
            }
        }

        "zwp_confined_pointer_v1" => {
            match msg.opcode {
                zwp_pointer_constraints::confined_pointer_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    // Remove constraint
                    pointer_constraints.remove(&msg.sender_id);
                    constraint_type_map.remove(&msg.sender_id);
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(zwp_pointer_constraints::confined_pointer_unconfined_event(msg.sender_id));
                    }
                }
                zwp_pointer_constraints::confined_pointer_request::SET_REGION => {
                    let region_id = ArgCursor::from_message(&msg).object().ok();
                    // In a full implementation, parse the region geometry
                    // For now, just acknowledge
                    let _ = region_id;
                }
                _ => {}
            }
        }

        "zwp_idle_inhibit_manager_v1" => {
            match msg.opcode {
                zxdg_idle_inhibit::idle_inhibit_request::CREATE_INHIBITOR => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let surface_id = cursor_obj.object().unwrap_or(0);
                    let inhibitor_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(inhibitor_id, "zwp_idle_inhibitor_v1", 1, client_id);
                    }
                    // Track this surface as inhibiting idle
                    shell.add_idle_inhibitor(inhibitor_id, surface_id);
                    eprintln!("[edgerun-compositor] Idle inhibitor created for surface {}", surface_id);
                }
                zxdg_idle_inhibit::idle_inhibit_request::DESTROY => {}
                _ => {}
            }
        }

        "zwp_idle_inhibitor_v1" => {
            match msg.opcode {
                zxdg_idle_inhibit::idle_inhibitor_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    // Remove the inhibitor using the inhibitor object ID
                    shell.remove_idle_inhibitor(msg.sender_id);
                }
                _ => {}
            }
        }

        // ─── linux-drm-syncobj-v1 (stub) ──────────────────────
        "linux_drm_syncobj_v1" => {
            match msg.opcode {
                linux_drm_syncobj::syncobj_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                linux_drm_syncobj::syncobj_request::GET_SURFACE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let surface_obj_id = cursor_obj.new_id().unwrap_or(0);
                    let _wl_surface_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(surface_obj_id, linux_drm_syncobj::SURFACE_V1, 1, client_id);
                    }
                }
                linux_drm_syncobj::syncobj_request::CREATE_TIMELINE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let timeline_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(timeline_id, linux_drm_syncobj::TIMELINE_V1, 1, client_id);
                    }
                }
                _ => {}
            }
        }

        "linux_drm_syncobj_surface_v1" => {
            match msg.opcode {
                linux_drm_syncobj::surface_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                linux_drm_syncobj::surface_request::SET_ACQUIRE_POINT |
                linux_drm_syncobj::surface_request::SET_RELEASE_POINT => {
                    // Stub: accept but ignore sync points
                }
                _ => {}
            }
        }

        "linux_drm_syncobj_timeline_v1" => {
            match msg.opcode {
                linux_drm_syncobj::timeline_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                linux_drm_syncobj::timeline_request::IMPORT_SYNC_FILE |
                linux_drm_syncobj::timeline_request::EXPORT_SYNC_FILE => {
                    // Stub: accept but ignore sync file operations
                }
                _ => {}
            }
        }

        "xdg_wm_base" => {
            match msg.opcode {
                xdg_shell::xdg_wm_base_request::GET_XDG_SURFACE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let xdg_surface_id = cursor_obj.new_id().unwrap_or(0);
                    let wl_surface_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(xdg_surface_id, "xdg_surface", 6, client_id);
                    }
                    // Map xdg_surface -> wl_surface for keyboard/pointer enter
                    xdg_surface_to_wl_surface.insert(xdg_surface_id, wl_surface_id);
                    println!("[edgerun-compositor] xdg_surface created: id={} wl_surface={}", xdg_surface_id, wl_surface_id);
                }
                xdg_shell::xdg_wm_base_request::DESTROY => {}
                xdg_shell::xdg_wm_base_request::CREATE_POSITIONER => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let pos_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(pos_id, "xdg_positioner", 6, client_id);
                    }
                    // Initialize positioner state
                    shell.set_positioner(pos_id, crate::compositor::shell::PositionerState::default());
                }
                xdg_shell::xdg_wm_base_request::PONG => {}
                _ => {}
            }
        }

        "xdg_surface" => {
            match msg.opcode {
                xdg_shell::xdg_surface_request::GET_TOPLEVEL => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let toplevel_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(toplevel_id, "xdg_toplevel", 6, client_id);
                    }
                    // Look up the wl_surface id for this xdg_surface
                    let wl_surface_id = xdg_surface_to_wl_surface.get(&msg.sender_id).copied().unwrap_or(msg.sender_id);
                    shell.create_toplevel(toplevel_id, wl_surface_id);

                    println!("[edgerun-compositor] xdg_toplevel created: toplevel_id={} wl_surface={}", toplevel_id, wl_surface_id);

                    // Focus this surface for keyboard and pointer
                    let old_kb_focus = seat.set_keyboard_focus(Some(wl_surface_id));
                    let _ = old_kb_focus;
                    let old_ptr_focus = seat.set_pointer_focus(Some(wl_surface_id));
                    let _ = old_ptr_focus;

                    let serial = *config_serial;
                    *config_serial += 1;
                    if let Some(client) = server.client_mut(client_id) {
                        let state = shell.toplevel_state_bytes(toplevel_id);
                        client.send_message(xdg_shell::xdg_surface_configure_event(msg.sender_id, serial));
                        // P0: configure_bounds (v4+) — tells client the available space
                        client.send_message(xdg_shell::xdg_toplevel_configure_bounds_event(
                            toplevel_id,
                            shell.output_width,
                            shell.output_height,
                        ));
                        client.send_message(xdg_shell::xdg_toplevel_configure_event(
                            toplevel_id,
                            shell.output_width,
                            shell.output_height,
                            &state,
                        ));
                        // Send wm_capabilities (v5+)
                        // Capabilities we support:
                        let caps: &[u32] = &[
                            1, // WINDOW_MENU (we support show_window_menu)
                            2, // MAXIMIZE
                            3, // FULLSCREEN
                            4, // MINIMIZE
                        ];
                        client.send_message(xdg_shell::xdg_toplevel_wm_capabilities_event(toplevel_id, caps));
                        let _ = client.flush();
                    }

                    // If keyboard object exists for this client, send enter
                    if let Some(&kb_id) = client_keyboard_ids.get(&client_id) {
                        let serial = seat.next_serial();
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(wl_seat::keyboard_enter_event(
                                kb_id, serial, wl_surface_id, &[]));
                        }
                    }

                    // If pointer object exists for this client, send enter
                    if let Some(&ptr_id) = client_pointer_ids.get(&client_id) {
                        let serial = seat.next_serial();
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(wl_seat::pointer_enter_event(
                                ptr_id, serial, wl_surface_id,
                                seat.pointer_x, seat.pointer_y,
                            ));
                            client.send_message(wl_seat::pointer_frame_event(ptr_id));
                        }
                    }
                }
                xdg_shell::xdg_surface_request::GET_POPUP => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let popup_id = cursor_obj.new_id().unwrap_or(0);
                    let parent_id = cursor_obj.object().ok();
                    let positioner_id = cursor_obj.object().ok();
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(popup_id, "xdg_popup", 6, client_id);
                    }

                    // Find parent surface id
                    let parent_surface = parent_id.and_then(|pid| {
                        shell.toplevels.values().find(|tl| tl.id == pid).map(|tl| tl.surface_id)
                    });

                    shell.create_popup(popup_id, msg.sender_id, parent_surface);

                    // Apply positioner state if available
                    if let Some(pos_id) = positioner_id {
                        if let Some(pos) = shell.positioners.get(&pos_id) {
                            shell.set_popup_positioner(
                                popup_id,
                                pos.anchor_rect_x, pos.anchor_rect_y,
                                pos.anchor_rect_width, pos.anchor_rect_height,
                                pos.anchor, pos.gravity,
                                pos.offset_x, pos.offset_y,
                            );
                        }
                    }
                }
                xdg_shell::xdg_surface_request::ACK_CONFIGURE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let serial = cursor_obj.uint().unwrap_or(0);
                    if let Some(tl) = shell.toplevels.values_mut().find(|tl| tl.surface_id == msg.sender_id) {
                        if tl.configure_serial == Some(serial) {
                            tl.configured = true;
                        }
                    }
                }
                xdg_shell::xdg_surface_request::SET_WINDOW_GEOMETRY => {
                    let _ = &msg;
                }
                xdg_shell::xdg_surface_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        "xdg_toplevel" => {
            match msg.opcode {
                xdg_shell::xdg_toplevel_request::SET_TITLE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    if let Ok(Some(title)) = cursor_obj.string() {
                        if let Some(tl) = shell.toplevels.values_mut().find(|tl| tl.id == msg.sender_id) {
                            tl.title = Some(title.0.clone());
                            println!("[edgerun-compositor] Title: {}", title.0);
                        }
                    }
                }
                xdg_shell::xdg_toplevel_request::SET_APP_ID => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    if let Ok(Some(app_id)) = cursor_obj.string() {
                        if let Some(tl) = shell.toplevels.values_mut().find(|tl| tl.id == msg.sender_id) {
                            tl.app_id = Some(app_id.0.clone());
                            println!("[edgerun-compositor] App ID: {}", app_id.0);
                        }
                    }
                }
                xdg_shell::xdg_toplevel_request::SET_MIN_SIZE => {
                    let _ = &msg;
                }
                xdg_shell::xdg_toplevel_request::SET_MAX_SIZE => {
                    let _ = &msg;
                }
                xdg_shell::xdg_toplevel_request::MAXIMIZE | xdg_shell::xdg_toplevel_request::SET_MAXIMIZED => {
                    let toplevel_id = shell.toplevels.get(&msg.sender_id).map(|tl| tl.id);
                    if let Some(tl_id) = toplevel_id {
                        shell.maximize(tl_id);
                        let serial = shell.configure_toplevel_with_state(tl_id, shell.output_width, shell.output_height);
                        if let Some(client) = server.client_mut(client_id) {
                            let state = shell.toplevel_state_bytes(tl_id);
                            client.send_message(xdg_shell::xdg_surface_configure_event(
                                shell.toplevels.get(&tl_id).map(|tl| tl.surface_id).unwrap_or(0), serial));
                            client.send_message(xdg_shell::xdg_toplevel_configure_event(
                                tl_id, shell.output_width, shell.output_height, &state,
                            ));
                        }
                    }
                }
                xdg_shell::xdg_toplevel_request::UNMAXIMIZE | xdg_shell::xdg_toplevel_request::UNSET_MAXIMIZED => {
                    let toplevel_id = shell.toplevels.get(&msg.sender_id).map(|tl| tl.id);
                    if let Some(tl_id) = toplevel_id {
                        shell.unmaximize(tl_id);
                        let serial = shell.configure_toplevel_with_state(tl_id, shell.output_width, shell.output_height);
                        if let Some(client) = server.client_mut(client_id) {
                            let state = shell.toplevel_state_bytes(tl_id);
                            let surface_id = shell.toplevels.get(&tl_id).map(|tl| tl.surface_id).unwrap_or(0);
                            client.send_message(xdg_shell::xdg_surface_configure_event(surface_id, serial));
                            client.send_message(xdg_shell::xdg_toplevel_configure_event(
                                tl_id, shell.output_width, shell.output_height, &state,
                            ));
                        }
                    }
                }
                xdg_shell::xdg_toplevel_request::MINIMIZE | xdg_shell::xdg_toplevel_request::SET_MINIMIZED => {}
                xdg_shell::xdg_toplevel_request::SET_FULLSCREEN => {
                    let toplevel_id = shell.toplevels.get(&msg.sender_id).map(|tl| tl.id);
                    if let Some(tl_id) = toplevel_id {
                        shell.set_fullscreen(tl_id, true);
                        // Move surface to 0,0
                        let surface_id = shell.toplevels.get(&tl_id).map(|tl| tl.surface_id).unwrap_or(0);
                        if let Some(surface) = surfaces.get_mut(surface_id) {
                            surface.x = 0;
                            surface.y = 0;
                        }
                        let serial = shell.configure_toplevel_with_state(tl_id, shell.output_width, shell.output_height);
                        if let Some(client) = server.client_mut(client_id) {
                            let state = shell.toplevel_state_bytes(tl_id);
                            client.send_message(xdg_shell::xdg_surface_configure_event(surface_id, serial));
                            client.send_message(xdg_shell::xdg_toplevel_configure_event(
                                tl_id, shell.output_width, shell.output_height, &state,
                            ));
                        }
                    }
                }
                xdg_shell::xdg_toplevel_request::UNSET_FULLSCREEN => {
                    let toplevel_id = shell.toplevels.get(&msg.sender_id).map(|tl| tl.id);
                    if let Some(tl_id) = toplevel_id {
                        shell.set_fullscreen(tl_id, false);
                        let surface_id = shell.toplevels.get(&tl_id).map(|tl| tl.surface_id).unwrap_or(0);
                        let serial = shell.configure_toplevel_with_state(tl_id, shell.output_width, shell.output_height);
                        if let Some(client) = server.client_mut(client_id) {
                            let state = shell.toplevel_state_bytes(tl_id);
                            client.send_message(xdg_shell::xdg_surface_configure_event(surface_id, serial));
                            client.send_message(xdg_shell::xdg_toplevel_configure_event(
                                tl_id, shell.output_width, shell.output_height, &state,
                            ));
                        }
                    }
                }
                xdg_shell::xdg_toplevel_request::MOVE => {
                    let _ = &msg;
                }
                xdg_shell::xdg_toplevel_request::RESIZE => {
                    let _ = &msg;
                }
                xdg_shell::xdg_toplevel_request::SET_PARENT => {
                    let _ = &msg;
                }
                xdg_shell::xdg_toplevel_request::DESTROY => {
                    let surface_id = shell.toplevels.get(&msg.sender_id).map(|tl| tl.surface_id);

                    if seat.keyboard_focus() == surface_id {
                        let serial = seat.next_serial();
                        if let Some(sid) = surface_id {
                            for (&cid, &kb_id) in client_keyboard_ids.iter() {
                                if let Some(client) = server.client_mut(cid) {
                                    client.send_message(wl_seat::keyboard_leave_event(kb_id, serial, sid));
                                }
                            }
                        }
                        seat.set_keyboard_focus(None);
                    }
                    if seat.pointer_focus() == surface_id {
                        let serial = seat.next_serial();
                        if let Some(sid) = surface_id {
                            for (&cid, &ptr_id) in client_pointer_ids.iter() {
                                if let Some(client) = server.client_mut(cid) {
                                    client.send_message(wl_seat::pointer_leave_event(ptr_id, serial, sid));
                                }
                            }
                        }
                        seat.set_pointer_focus(None);
                    }

                    shell.destroy_toplevel(msg.sender_id);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        if let Some(sid) = surface_id {
                            reg.destroy(sid);
                        }
                    }
                }
                _ => {}
            }
        }

        "xdg_positioner" => {
            match msg.opcode {
                xdg_shell::xdg_positioner_request::SET_SIZE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let w = cursor_obj.int().unwrap_or(0);
                    let h = cursor_obj.int().unwrap_or(0);
                    if let Some(pos) = shell.positioners.get_mut(&msg.sender_id) {
                        pos.width = w;
                        pos.height = h;
                    }
                }
                xdg_shell::xdg_positioner_request::SET_ANCHOR_RECT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let x = cursor_obj.int().unwrap_or(0);
                    let y = cursor_obj.int().unwrap_or(0);
                    let w = cursor_obj.int().unwrap_or(0);
                    let h = cursor_obj.int().unwrap_or(0);
                    if let Some(pos) = shell.positioners.get_mut(&msg.sender_id) {
                        pos.anchor_rect_x = x;
                        pos.anchor_rect_y = y;
                        pos.anchor_rect_width = w;
                        pos.anchor_rect_height = h;
                    }
                }
                xdg_shell::xdg_positioner_request::SET_ANCHOR => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let anchor = cursor_obj.uint().unwrap_or(0);
                    if let Some(pos) = shell.positioners.get_mut(&msg.sender_id) {
                        pos.anchor = anchor;
                    }
                }
                xdg_shell::xdg_positioner_request::SET_GRAVITY => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let gravity = cursor_obj.uint().unwrap_or(0);
                    if let Some(pos) = shell.positioners.get_mut(&msg.sender_id) {
                        pos.gravity = gravity;
                    }
                }
                xdg_shell::xdg_positioner_request::SET_CONSTRAINT_ADJUSTMENT => {}
                xdg_shell::xdg_positioner_request::SET_OFFSET => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let x = cursor_obj.int().unwrap_or(0);
                    let y = cursor_obj.int().unwrap_or(0);
                    if let Some(pos) = shell.positioners.get_mut(&msg.sender_id) {
                        pos.offset_x = x;
                        pos.offset_y = y;
                    }
                }
                xdg_shell::xdg_positioner_request::SET_REACTIVE => {}
                xdg_shell::xdg_positioner_request::SET_PARENT_SIZE => {}
                xdg_shell::xdg_positioner_request::SET_PARENT_CONFIGURE => {}
                xdg_shell::xdg_positioner_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    shell.positioners.remove(&msg.sender_id);
                }
                _ => {}
            }
        }

        "xdg_popup" => {
            match msg.opcode {
                xdg_shell::xdg_popup_request::DESTROY => {
                    shell.destroy_popup(msg.sender_id);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                xdg_shell::xdg_popup_request::GRAB => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let seat_obj_id = cursor_obj.object().unwrap_or(0);
                    let serial = cursor_obj.uint().unwrap_or(0);
                    let _ = &seat_obj_id; // We use our own seat tracking
                    shell.grab_popup(msg.sender_id, seat_obj_id, serial);
                    // Send configure for the popup
                    if let Some(popup) = shell.popups.get(&msg.sender_id) {
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(xdg_shell::xdg_popup_configure_event(
                                msg.sender_id,
                                popup.x, popup.y,
                                popup.width.max(100), popup.height.max(50),
                            ));
                            let _ = client.flush();
                        }
                    }
                }
                xdg_shell::xdg_popup_request::REPOSITION => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let _positioner_id = cursor_obj.object().unwrap_or(0);
                    let _token = cursor_obj.uint().unwrap_or(0);
                }
                _ => {}
            }
        }

        "wl_buffer" => {
            match msg.opcode {
                wl_shm::buffer_request::DESTROY => {
                    buffers.remove(msg.sender_id);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        "wl_output" => {}
        "wl_region" => {}

        // ─── screencopy-v1 ──────────────────────────────────
        "zwlr_screencopy_manager_v1" => {
            match msg.opcode {
                screencopy::manager_request::CAPTURE_OUTPUT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let frame_id = cursor_obj.new_id().unwrap_or(0);
                    let _overlay_cursor = cursor_obj.uint().unwrap_or(0);
                    let _capture_type = cursor_obj.uint().unwrap_or(0);
                    let _output_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(frame_id, screencopy::ZWLR_SCREENCOPY_FRAME_V1, 3, client_id);
                    }
                    // Create pending frame state
                    screencopy_state.pending_frames.insert(frame_id, ScreencopyFrame {
                        frame_id,
                        client_id,
                        width: screencopy_state.width,
                        height: screencopy_state.height,
                        stride: screencopy_state.stride,
                        format: 0x34325258, // XRGB8888
                        region: None,
                        copy_requested: false,
                        target_pool_fd: None,
                        target_offset: 0,
                        target_size: 0,
                    });
                    // Send buffer event with format info
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(screencopy::frame_buffer_event(
                            frame_id, 0x34325258, // XRGB8888
                            screencopy_state.width, screencopy_state.height, screencopy_state.stride));
                        let _ = client.flush();
                    }
                }
                screencopy::manager_request::CAPTURE_OUTPUT_REGION => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let frame_id = cursor_obj.new_id().unwrap_or(0);
                    let _overlay_cursor = cursor_obj.uint().unwrap_or(0);
                    let _capture_type = cursor_obj.uint().unwrap_or(0);
                    let _output_id = cursor_obj.object().unwrap_or(0);
                    let x = cursor_obj.int().unwrap_or(0);
                    let y = cursor_obj.int().unwrap_or(0);
                    let w = cursor_obj.int().unwrap_or(0);
                    let h = cursor_obj.int().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(frame_id, screencopy::ZWLR_SCREENCOPY_FRAME_V1, 3, client_id);
                    }
                    let capture_w = if w > 0 { w as u32 } else { screencopy_state.width };
                    let capture_h = if h > 0 { h as u32 } else { screencopy_state.height };
                    let capture_stride = capture_w * 4;
                    screencopy_state.pending_frames.insert(frame_id, ScreencopyFrame {
                        frame_id,
                        client_id,
                        width: capture_w,
                        height: capture_h,
                        stride: capture_stride,
                        format: 0x34325258,
                        region: Some((x, y, w, h)),
                        copy_requested: false,
                        target_pool_fd: None,
                        target_offset: 0,
                        target_size: 0,
                    });
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(screencopy::frame_buffer_event(
                            frame_id, 0x34325258, capture_w, capture_h, capture_stride));
                        let _ = client.flush();
                    }
                }
                screencopy::manager_request::DESTROY => {}
                _ => {}
            }
        }
        "zwlr_screencopy_frame_v1" => {
            match msg.opcode {
                screencopy::frame_request::COPY | screencopy::frame_request::COPY_WITH_DAMAGE => {
                    let buffer_id = ArgCursor::from_message(&msg).object().unwrap_or(0);
                    if buffer_id == 0 {
                        // Invalid buffer — send failed
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(screencopy::frame_failed_event(msg.sender_id));
                        }
                        return;
                    }

                    // Look up the buffer in our registry
                    if let Some(buffer_info) = buffers.get(buffer_id) {
                        // Get the pending frame
                        if let Some(frame) = screencopy_state.pending_frames.get(&msg.sender_id) {
                            if frame.copy_requested {
                                // Already used — send failed
                                if let Some(client) = server.client_mut(client_id) {
                                    client.send_message(screencopy::frame_failed_event(msg.sender_id));
                                }
                                return;
                            }

                            // Perform the copy from screencopy_state pixels to the client's SHM buffer
                            let src_pixels = &screencopy_state.pixels;
                            let fb_width = screencopy_state.width as usize;
                            let fb_height = screencopy_state.height as usize;
                            let fb_stride = screencopy_state.stride as usize;

                            let buf_width = buffer_info.width as usize;
                            let buf_height = buffer_info.height as usize;
                            let buf_stride = buffer_info.stride as usize;

                            // Map the client's SHM pool and copy pixels
                            let pool_fd = buffer_info.pool_fd;
                            let offset = buffer_info.offset as usize;

                            if !src_pixels.is_empty() && pool_fd >= 0 {
                                let map_size = buf_stride * buf_height;
                                let buf_ptr = unsafe {
                                    libc::mmap(
                                        std::ptr::null_mut(),
                                        map_size,
                                        libc::PROT_READ | libc::PROT_WRITE,
                                        libc::MAP_SHARED,
                                        pool_fd,
                                        offset as libc::off_t,
                                    )
                                };

                                if buf_ptr != libc::MAP_FAILED {
                                    let dst = unsafe {
                                        std::slice::from_raw_parts_mut(buf_ptr as *mut u8, map_size)
                                    };

                                    // Copy row by row, handling stride differences
                                    let copy_w = fb_width.min(buf_width);
                                    let copy_h = fb_height.min(buf_height);
                                    for row in 0..copy_h {
                                        let src_row = row * fb_stride;
                                        let dst_row = row * buf_stride;
                                        let copy_bytes = copy_w * 4; // 4 bytes per pixel (XRGB8888)
                                        if src_row + copy_bytes <= src_pixels.len()
                                            && dst_row + copy_bytes <= dst.len()
                                        {
                                            dst[dst_row..dst_row + copy_bytes]
                                                .copy_from_slice(&src_pixels[src_row..src_row + copy_bytes]);
                                        }
                                    }

                                    unsafe { libc::munmap(buf_ptr, map_size) };
                                }
                            }

                            // Mark frame as copied and send flags + ready
                            if let Some(frame_mut) = screencopy_state.pending_frames.get_mut(&msg.sender_id) {
                                frame_mut.copy_requested = true;
                            }
                            if let Some(client) = server.client_mut(client_id) {
                                // Send flags (Y_INVERT because our software renderer draws top-to-bottom)
                                client.send_message(screencopy::frame_flags_event(
                                    msg.sender_id, screencopy::frame_flags::Y_INVERT));
                                // Send ready (copy complete)
                                client.send_message(screencopy::frame_ready_event(msg.sender_id));
                                let _ = client.flush();
                            }
                        }
                    } else {
                        // Buffer not found
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(screencopy::frame_failed_event(msg.sender_id));
                        }
                    }
                }
                screencopy::frame_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    screencopy_state.pending_frames.remove(&msg.sender_id);
                }
                _ => {}
            }
        }

        // ─── text-input-v3 ─────────────────────────────────
        "zwp_text_input_manager_v3" => {
            match msg.opcode {
                text_input_v3::manager_request::GET_TEXT_INPUT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let text_input_id = cursor_obj.new_id().unwrap_or(0);
                    let _seat_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(text_input_id, text_input_v3::ZWP_TEXT_INPUT_V3, 1, client_id);
                    }
                    // Create text input state
                    *text_input_state = Some(TextInputState::new(text_input_id, client_id));
                }
                text_input_v3::manager_request::DESTROY => {}
                _ => {}
            }
        }
        "zwp_text_input_v3" => {
            match msg.opcode {
                text_input_v3::text_input_request::ENABLE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let surface_id = cursor_obj.object().unwrap_or(0);
                    let serial = cursor_obj.uint().unwrap_or(0);
                    *text_input_serial = serial;

                    if let Some(ti) = text_input_state.as_mut() {
                        ti.enabled = true;
                        ti.focused_surface = Some(surface_id);

                        // Send enter event
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(text_input_v3::text_input_enter_event(
                                ti.text_input_id, surface_id));
                            client.send_message(text_input_v3::text_input_done_event(
                                ti.text_input_id, serial));
                            let _ = client.flush();
                        }

                        // If IME is connected, send activate
                        if let Some(ime) = &ime_state {
                            if let Some(ime_client) = server.client_mut(ime.client_id) {
                                ime_client.send_message(input_method_v2::input_method_activate_event(
                                    ime.input_method_id, ti.text_input_id));
                                ime_client.send_message(input_method_v2::input_method_done_event(
                                    ime.input_method_id));
                            }
                        }
                    }
                }
                text_input_v3::text_input_request::DISABLE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let surface_id = cursor_obj.object().unwrap_or(0);
                    let serial = cursor_obj.uint().unwrap_or(0);
                    *text_input_serial = serial;

                    if let Some(ti) = text_input_state.as_mut() {
                        ti.enabled = false;

                        // Send leave event
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(text_input_v3::text_input_leave_event(
                                ti.text_input_id, surface_id, serial));
                            client.send_message(text_input_v3::text_input_done_event(
                                ti.text_input_id, serial));
                            let _ = client.flush();
                        }

                        // If IME is connected, send deactivate
                        if let Some(ime) = &ime_state {
                            if let Some(ime_client) = server.client_mut(ime.client_id) {
                                ime_client.send_message(input_method_v2::input_method_deactivate_event(
                                    ime.input_method_id, ti.text_input_id));
                                ime_client.send_message(input_method_v2::input_method_done_event(
                                    ime.input_method_id));
                            }
                        }
                    }
                }
                text_input_v3::text_input_request::SET_SURROUNDING_TEXT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    if let Ok(Some(text)) = cursor_obj.string() {
                        let cursor = cursor_obj.uint().unwrap_or(0);
                        let anchor = cursor_obj.uint().unwrap_or(0);
                        if let Some(ti) = text_input_state.as_mut() {
                            ti.surrounding_text = text.0;
                            ti.cursor = cursor;
                            ti.anchor = anchor;

                            // Forward to IME if connected
                            if let Some(ime) = &ime_state {
                                if let Some(ime_client) = server.client_mut(ime.client_id) {
                                    ime_client.send_message(input_method_v2::input_method_surround_text_event(
                                        ime.input_method_id, &ti.surrounding_text, ti.cursor, ti.anchor));
                                    ime_client.send_message(input_method_v2::input_method_done_event(
                                        ime.input_method_id));
                                }
                            }
                        }
                    }
                }
                text_input_v3::text_input_request::SET_TEXT_CHANGE_CAUSE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let cause = cursor_obj.uint().unwrap_or(0);
                    if let Some(ti) = text_input_state.as_mut() {
                        ti.text_change_cause = cause;

                        // Forward to IME
                        if let Some(ime) = &ime_state {
                            if let Some(ime_client) = server.client_mut(ime.client_id) {
                                ime_client.send_message(input_method_v2::input_method_text_change_cause_event(
                                    ime.input_method_id, cause));
                            }
                        }
                    }
                }
                text_input_v3::text_input_request::COMMIT => {
                    // Client committed text state — IME would have already sent
                    // commit_string/preedit events via the input_method_v2 handlers.
                    let _ = &ime_state;
                }
                text_input_v3::text_input_request::GET_SURROUNDING_TEXT => {
                    // Client wants surrounding text — we've been tracking it
                    // Send back what we have via the text_input state
                }
                text_input_v3::text_input_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    *text_input_state = None;
                }
                _ => {}
            }
        }

        // ─── input-method-v2 ───────────────────────────────
        "zwp_input_method_manager_v2" => {
            match msg.opcode {
                input_method_v2::manager_request::GET_INPUT_METHOD => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let im_id = cursor_obj.new_id().unwrap_or(0);
                    let _seat_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(im_id, input_method_v2::ZWP_INPUT_METHOD_V2, 1, client_id);
                    }
                    // Create IME state
                    *ime_state = Some(IMEState::new(im_id, client_id));
                    eprintln!("[edgerun-compositor] IME server connected, client={}", client_id);
                }
                input_method_v2::manager_request::DESTROY => {}
                _ => {}
            }
        }
        "zwp_input_method_v2" => {
            match msg.opcode {
                input_method_v2::input_method_request::COMMIT_STRING => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    if let Ok(Some(text)) = cursor_obj.string() {
                        if let Some(ti) = text_input_state.as_ref() {
                            if let Some(client) = server.client_mut(ti.client_id) {
                                client.send_message(text_input_v3::text_input_commit_string_event(
                                    ti.text_input_id, &text.0));
                                client.send_message(text_input_v3::text_input_done_event(
                                    ti.text_input_id, *text_input_serial));
                                let _ = client.flush();
                            }
                        }
                    }
                }
                input_method_v2::input_method_request::COMMIT_PREEDIT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    if let Ok(Some(text)) = cursor_obj.string() {
                        let cursor_begin = cursor_obj.int().unwrap_or(0);
                        let cursor_end = cursor_obj.int().unwrap_or(0);
                        if let Some(ti) = text_input_state.as_ref() {
                            if let Some(client) = server.client_mut(ti.client_id) {
                                client.send_message(text_input_v3::text_input_preedit_string_event(
                                    ti.text_input_id, &text.0, cursor_begin, cursor_end));
                            }
                        }
                    }
                }
                input_method_v2::input_method_request::DELETE_SURROUNDING_TEXT => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let before = cursor_obj.int().unwrap_or(0);
                    let after = cursor_obj.int().unwrap_or(0);
                    if let Some(ti) = text_input_state.as_ref() {
                        if let Some(client) = server.client_mut(ti.client_id) {
                            client.send_message(text_input_v3::text_input_delete_surrounding_text_event(
                                ti.text_input_id, before, after));
                        }
                    }
                }
                input_method_v2::input_method_request::COMMIT => {
                    // IME committed text — send done to text input client
                    if let Some(ti) = text_input_state.as_ref() {
                        if let Some(client) = server.client_mut(ti.client_id) {
                            client.send_message(text_input_v3::text_input_done_event(
                                ti.text_input_id, *text_input_serial));
                            let _ = client.flush();
                        }
                    }
                }
                input_method_v2::input_method_request::SET_SURROUNDING_TEXT |
                input_method_v2::input_method_request::SET_TEXT_CHANGE_CAUSE |
                input_method_v2::input_method_request::SET_CONTENT_TYPE |
                input_method_v2::input_method_request::AVAILABLE => {
                    // IME sending state — tracked in IME state if needed
                    let _ = msg;
                }
                input_method_v2::input_method_request::GRAB_KEYBOARD => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let grab_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(grab_id, input_method_v2::ZWP_INPUT_METHOD_KEYBOARD_GRAB_V2, 1, client_id);
                    }
                    if let Some(ime) = ime_state.as_mut() {
                        ime.keyboard_grab_id = Some(grab_id);
                        ime.keyboard_grab_active = true;

                        // Send keymap and repeat info to the IME keyboard grab
                        let keymap_str = keymap::xkb_keymap_text();
                        let keymap_size = keymap_str.len() as u32;

                        let fd = unsafe {
                            libc::open(b"/dev/shm/edgerun-im-km\0".as_ptr() as *const libc::c_char,
                                libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
                                0o600)
                        };
                        if fd >= 0 {
                            unsafe { libc::unlink(b"/dev/shm/edgerun-im-km\0".as_ptr() as *const libc::c_char) };
                            let written = unsafe {
                                libc::write(fd, keymap_str.as_ptr() as *const _, keymap_str.len())
                            };
                            if written >= 0 {
                                let dup_fd = unsafe { libc::dup(fd) };
                                if dup_fd >= 0 {
                                    if let Some(client) = server.client_mut(client_id) {
                                        client.send_message(input_method_v2::keyboard_grab_keymap_event(
                                            grab_id, 1, dup_fd, keymap_size));
                                        client.send_message(input_method_v2::keyboard_grab_repeat_info_event(
                                            grab_id, 25, 300));
                                        let _ = client.flush();
                                    }
                                }
                            }
                            unsafe { libc::close(fd) };
                        }
                    }
                }
                input_method_v2::input_method_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    *ime_state = None;
                }
                _ => {}
            }
        }
        "zwp_input_method_keyboard_grab_v2" => {
            match msg.opcode {
                input_method_v2::keyboard_grab_request::DESTROY |
                input_method_v2::keyboard_grab_request::RELEASE => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    if let Some(ime) = ime_state.as_mut() {
                        ime.keyboard_grab_active = false;
                        ime.keyboard_grab_id = None;
                    }
                }
                _ => {}
            }
        }

        // ─── primary-selection-v1 ──────────────────────────
        "zwlr_primary_selection_manager_v1" => {
            match msg.opcode {
                primary_selection::manager_request::CREATE_DATA_SOURCE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let source_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(source_id, primary_selection::ZWLR_PRIMARY_SELECTION_SOURCE_V1, 1, client_id);
                    }
                }
                primary_selection::manager_request::GET_PRIMARY_SELECTION => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let device_id = cursor_obj.new_id().unwrap_or(0);
                    let _seat_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(device_id, primary_selection::ZWLR_PRIMARY_SELECTION_DEVICE_V1, 1, client_id);
                    }
                    // Send current primary selection if exists
                    if let Some(ref source) = *current_primary_selection {
                        *primary_selection_offer_counter += 1;
                        let offer_id = *primary_selection_offer_counter;
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(primary_selection::device_selection_event(
                                device_id, offer_id));
                            for mime in &source.mime_types {
                                client.send_message(primary_selection::offer_offer_event(offer_id, mime));
                            }
                        }
                    } else {
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(primary_selection::device_selection_event(device_id, 0));
                        }
                    }
                }
                primary_selection::manager_request::DESTROY => {}
                _ => {}
            }
        }
        "zwlr_primary_selection_device_v1" => {
            match msg.opcode {
                primary_selection::device_request::SET_SELECTION => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let source_id = cursor_obj.object().unwrap_or(0);
                    if source_id != 0 {
                        *primary_selection_offer_counter += 1;
                        *current_primary_selection = Some(PrimarySelectionSource {
                            id: source_id,
                            owner_client_id: client_id,
                            mime_types: Vec::new(),
                        });
                    } else {
                        *current_primary_selection = None;
                    }
                }
                primary_selection::device_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }
        "zwlr_primary_selection_offer_v1" => {
            match msg.opcode {
                primary_selection::offer_request::RECEIVE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    if let Ok(Some(mime_type)) = cursor_obj.string() {
                        let fd = if !msg.fds.is_empty() { msg.fds[0] } else { -1 };
                        if fd >= 0 {
                            // Forward to source client
                            if let Some(ref source) = *current_primary_selection {
                                if let Some(source_client) = server.client_mut(source.owner_client_id) {
                                    source_client.send_message(primary_selection::source_send_event(
                                        source.id, &mime_type.0, fd));
                                }
                            }
                        }
                    }
                }
                primary_selection::offer_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }
        "zwlr_primary_selection_source_v1" => {
            match msg.opcode {
                primary_selection::source_request::OFFER => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    if let Ok(Some(mime_type)) = cursor_obj.string() {
                        if let Some(source) = current_primary_selection.as_mut() {
                            source.mime_types.push(mime_type.0);
                        }
                    }
                }
                primary_selection::source_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    // Clear if this was the current source
                    if let Some(ref source) = *current_primary_selection {
                        if source.id == msg.sender_id {
                            *current_primary_selection = None;
                        }
                    }
                }
                _ => {}
            }
        }

        // ─── data-control-v1 ───────────────────────────────
        "zwlr_data_control_manager_v1" => {
            match msg.opcode {
                data_control::manager_request::CREATE_DATA_SOURCE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let source_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(source_id, data_control::ZWLR_DATA_CONTROL_SOURCE_V1, 1, client_id);
                    }
                }
                data_control::manager_request::GET_DATA_DEVICE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let device_id = cursor_obj.new_id().unwrap_or(0);
                    let _seat_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(device_id, data_control::ZWLR_DATA_CONTROL_DEVICE_V1, 2, client_id);
                    }
                    // Send initial selection event if there's a clipboard
                    if let Some(ref _source) = *current_data_source {
                        let offer_id = *selection_offer_counter + 1;
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(data_control::data_control_device_data_offer_event(device_id, offer_id));
                            client.send_message(data_control::data_control_device_selection_event(device_id, offer_id));
                            if let Some(ref source) = *current_data_source {
                                for mime in &source.mime_types {
                                    client.send_message(data_control::data_control_offer_event(offer_id, mime));
                                }
                            }
                        }
                    }
                }
                data_control::manager_request::DESTROY => {}
                _ => {}
            }
        }
        "zwlr_data_control_device_v1" => {
            match msg.opcode {
                data_control::data_control_device_request::SET_SELECTION => {
                    // Same as wl_data_device SET_SELECTION but headless
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let source_id = cursor_obj.object().unwrap_or(0);
                    if source_id != 0 {
                        *selection_offer_counter += 1;
                        let offer_id = *selection_offer_counter;
                        *current_data_source = Some(DataSource {
                            id: source_id,
                            owner_client_id: client_id,
                            mime_types: Vec::new(),
                        });
                        // Broadcast to all data control devices
                        for (&cid, &device_id) in client_data_device_ids.iter() {
                            if let Some(client) = server.client_mut(cid) {
                                client.send_message(data_control::data_control_device_data_offer_event(device_id, offer_id));
                                client.send_message(data_control::data_control_device_selection_event(device_id, offer_id));
                            }
                        }
                    }
                }
                data_control::data_control_device_request::RELEASE => {}
                _ => {}
            }
        }
        "zwlr_data_control_offer_v1" => {
            match msg.opcode {
                data_control::data_control_offer_request::RECEIVE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    if let Ok(Some(mime_type)) = cursor_obj.string() {
                        let fd = if !msg.fds.is_empty() { msg.fds[0] } else { -1 };
                        if fd >= 0 {
                            if let Some(ref source) = *current_data_source {
                                if let Some(source_client) = server.client_mut(source.owner_client_id) {
                                    source_client.send_message(wl_data_device::data_source_send_event(
                                        source.id, &mime_type.0, fd));
                                }
                            }
                        }
                    }
                }
                data_control::data_control_offer_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }
        "zwlr_data_control_source_v1" => {
            match msg.opcode {
                data_control::data_control_source_request::OFFER => {
                    if let Some(source) = current_data_source {
                        let mut cursor_obj = ArgCursor::from_message(&msg);
                        if let Ok(Some(mime_type)) = cursor_obj.string() {
                            source.mime_types.push(mime_type.0);
                        }
                    }
                }
                data_control::data_control_source_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        "zwp_linux_dmabuf_v1" => {
            match msg.opcode {
                linux_dmabuf::dmabuf_request::DESTROY => {}
                linux_dmabuf::dmabuf_request::CREATE_PARAMS => {
                    // Advertise supported formats + modifiers
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(linux_dmabuf::dmabuf_format_event(msg.sender_id, linux_dmabuf::dmabuf_format::XRGB8888));
                        client.send_message(linux_dmabuf::dmabuf_format_event(msg.sender_id, linux_dmabuf::dmabuf_format::ARGB8888));
                        client.send_message(linux_dmabuf::dmabuf_modifier_event(
                            msg.sender_id, linux_dmabuf::dmabuf_format::XRGB8888,
                            (linux_dmabuf::DRM_FORMAT_MOD_LINEAR >> 32) as u32,
                            linux_dmabuf::DRM_FORMAT_MOD_LINEAR as u32,
                        ));
                    }
                    // The actual buffer params come in subsequent FD messages
                    // We accumulate them in dmabuf_pending
                }
                linux_dmabuf::dmabuf_request::CREATE_IMMED => {
                    // Accept the dmabuf fd and register it as a buffer
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let buffer_id = cursor_obj.new_id().unwrap_or(0);
                    let fd = if !msg.fds.is_empty() { msg.fds[0] } else { -1 };
                    let width = cursor_obj.int().unwrap_or(0);
                    let height = cursor_obj.int().unwrap_or(0);
                    let stride = cursor_obj.int().unwrap_or(0);
                    let format = cursor_obj.uint().unwrap_or(0);

                    // Register as an SHM-like buffer but with dmabuf fd
                    buffers.register(buffer_id, ShmBufferInfo {
                        pool_fd: fd,
                        offset: 0,
                        width,
                        height,
                        stride,
                        format,
                    });

                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(buffer_id, "wl_buffer", 1, client_id);
                    }
                }
                _ => {}
            }
        }

        // ─── single-pixel-buffer-v1 ────────────────────────
        "wp_single_pixel_buffer_manager_v1" => {
            match msg.opcode {
                single_pixel_buffer::manager_request::CREATE_SRGB32_BUFFER => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let buffer_id = cursor_obj.new_id().unwrap_or(0);
                    let red = cursor_obj.uint().unwrap_or(0);
                    let green = cursor_obj.uint().unwrap_or(0);
                    let blue = cursor_obj.uint().unwrap_or(0);
                    let alpha = cursor_obj.uint().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(buffer_id, single_pixel_buffer::WP_SINGLE_PIXEL_BUFFER_V1, 1, client_id);
                    }

                    // Create an actual 1x1 SHM buffer with the specified color.
                    // The buffer is 4 bytes (1x1 RGBA8888).
                    let fd = unsafe {
                        libc::memfd_create(b"edgerun-single-pixel\0".as_ptr() as *const libc::c_char, 0)
                    };
                    if fd >= 0 {
                        // Set size to 4 bytes
                        unsafe { libc::ftruncate(fd, 4) };

                        // Write the RGBA pixel to the memfd
                        let pixel: [u8; 4] = [
                            red as u8,
                            green as u8,
                            blue as u8,
                            alpha as u8,
                        ];
                        unsafe {
                            libc::pwrite(fd, pixel.as_ptr() as *const libc::c_void, 4, 0);
                        }

                        // Register as a real SHM buffer (1x1, stride=4, XRGB8888 format)
                        buffers.register(buffer_id, ShmBufferInfo {
                            pool_fd: fd,
                            offset: 0,
                            width: 1,
                            height: 1,
                            stride: 4,
                            format: wl_shm::format::XRGB8888,
                        });

                        eprintln!("[edgerun-compositor] Single pixel buffer created: id={} rgba=({},{},{},{})", buffer_id, red, green, blue, alpha);
                    } else {
                        eprintln!("[edgerun-compositor] Failed to create memfd for single pixel buffer");
                    }
                }
                single_pixel_buffer::manager_request::DESTROY => {}
                _ => {}
            }
        }
        "wp_single_pixel_buffer_v1" => {
            match msg.opcode {
                single_pixel_buffer::buffer_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    buffers.remove(msg.sender_id);
                }
                _ => {}
            }
        }

        // ─── fractional-scale-v1 ───────────────────────────
        "wp_fractional_scale_manager_v1" => {
            match msg.opcode {
                fractional_scale::manager_request::GET_FRACTIONAL_SCALE => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let scale_id = cursor_obj.new_id().unwrap_or(0);
                    let _surface_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(scale_id, fractional_scale::WP_FRACTIONAL_SCALE_V1, 1, client_id);
                    }
                    // Send preferred scale based on actual output scale
                    // scale factor * 120 (e.g., 1.0x = 120, 2.0x = 240, 1.5x = 180)
                    let scale_value = (shell.output_scale * 120) as u32;
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(fractional_scale::preferred_scale_event(scale_id, scale_value));
                        let _ = client.flush();
                    }
                }
                fractional_scale::manager_request::DESTROY => {}
                _ => {}
            }
        }
        "wp_fractional_scale_v1" => {
            match msg.opcode {
                fractional_scale::fractional_scale_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                _ => {}
            }
        }

        // ─── tearing-control-v1 ────────────────────────────
        "wp_tearing_control_manager_v1" => {
            match msg.opcode {
                tearing_control::manager_request::GET_TEARING_CONTROL => {
                    let mut cursor_obj = ArgCursor::from_message(&msg);
                    let control_id = cursor_obj.new_id().unwrap_or(0);
                    let surface_id = cursor_obj.object().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(control_id, tearing_control::WP_TEARING_CONTROL_V1, 1, client_id);
                    }
                    // Track mapping: tearing_control_object -> surface
                    client_tearing_control_ids.insert(control_id, surface_id);
                }
                tearing_control::manager_request::DESTROY => {}
                _ => {}
            }
        }
        "wp_tearing_control_v1" => {
            match msg.opcode {
                tearing_control::tearing_control_request::SET_PRESENTATION_HINT => {
                    let hint = ArgCursor::from_message(&msg).uint().unwrap_or(0);
                    // Apply the hint to the associated surface
                    if let Some(&surface_id) = client_tearing_control_ids.get(&msg.sender_id) {
                        surfaces.set_tearing_hint(surface_id, hint);
                    }
                    match hint {
                        tearing_control::hint::DEFAULT => {
                            eprintln!("[edgerun-compositor] Tearing control: default (surface {})", msg.sender_id);
                        }
                        tearing_control::hint::SYNC => {
                            eprintln!("[edgerun-compositor] Tearing control: sync/VSync (surface {})", msg.sender_id);
                        }
                        tearing_control::hint::ASYNC => {
                            eprintln!("[edgerun-compositor] Tearing control: async/tearing allowed (surface {})", msg.sender_id);
                        }
                        _ => {}
                    }
                }
                tearing_control::tearing_control_request::DESTROY => {
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                    client_tearing_control_ids.remove(&msg.sender_id);
                }
                _ => {}
            }
        }

        _ => {}
    }
}

/// Process input events for a single device (called from epoll event handler).
pub fn process_input_for_device(
    input_mgr: &mut EvdevManager,
    dev_id: u32,
    seat: &mut Seat,
    keymap: &mut Keymap,
    modifiers: &mut Modifiers,
    server: &mut WaylandServer,
    _surfaces: &SurfaceTree,
    shell: &mut Shell,
    client_keyboard_ids: &HashMap<u32, u32>,
    client_pointer_ids: &HashMap<u32, u32>,
    client_touch_ids: &HashMap<u32, u32>,
    client_relative_pointer_ids: &HashMap<u32, u32>,
    client_gesture_swipe_ids: &HashMap<u32, u32>,
    client_gesture_pinch_ids: &HashMap<u32, u32>,
    _client_compositor_ids: &HashMap<u32, u32>,
    cursor: &mut Cursor,
    touch_state: &mut TouchState,
    pointer_constraints: &mut HashMap<u32, PointerConstraint>, // constraint_id -> constraint
    constraint_type_map: &mut HashMap<u32, ConstraintType>, // constraint_id -> type
) {
    for event in input_mgr.read_events(dev_id, 64) {
        match event.kind {
            edgerun_input::InputEventKind::Key => {
                let scancode = event.code as u16;
                let pressed = event.value == 1;

                // Check for mouse buttons (BTN_LEFT=0x110, BTN_RIGHT=0x111, BTN_MIDDLE=0x112)
                if event.code >= 0x110 && event.code <= 0x11f {
                    let serial = seat.next_serial();

                    if pressed {
                        let tl_id_and_sid = shell.frontmost().map(|tl| (tl.id, tl.surface_id));
                        let tl_id_and_sid = tl_id_and_sid; // force copy
                        if let Some((tl_id, sid)) = tl_id_and_sid {
                            if seat.keyboard_focus() != Some(sid) {
                                if let Some(old_sid) = seat.keyboard_focus() {
                                    for (&cid, &kb_id) in client_keyboard_ids.iter() {
                                        if let Some(client) = server.client_mut(cid) {
                                            client.send_message(wl_seat::keyboard_leave_event(
                                                kb_id, serial, old_sid));
                                        }
                                    }
                                }
                                seat.set_keyboard_focus(Some(sid));
                                for (&cid, &kb_id) in client_keyboard_ids.iter() {
                                    if let Some(client) = server.client_mut(cid) {
                                        client.send_message(wl_seat::keyboard_enter_event(
                                            kb_id, serial, sid, &[]));
                                        client.send_message(wl_seat::keyboard_modifiers_event(
                                            kb_id, serial,
                                            if modifiers.shift { 1 } else { 0 },
                                            if modifiers.caps { 2 } else { 0 },
                                            0, 0,
                                        ));
                                    }
                                }

                                shell.activate(tl_id);

                                let output_w = shell.output_width;
                                let output_h = shell.output_height;
                                let cfg_serial = shell.configure_toplevel_with_state(tl_id, output_w, output_h);
                                for (&cid, _) in client_keyboard_ids.iter() {
                                    if let Some(client) = server.client_mut(cid) {
                                        let state = shell.toplevel_state_bytes(tl_id);
                                        client.send_message(xdg_shell::xdg_surface_configure_event(
                                            sid, cfg_serial));
                                        client.send_message(xdg_shell::xdg_toplevel_configure_event(
                                            tl_id, output_w, output_h, &state));
                                    }
                                }
                            }
                        }
                    }

                    // Send button event to pointer focus
                    for (&cid, &ptr_id) in client_pointer_ids.iter() {
                        if let Some(client) = server.client_mut(cid) {
                            client.send_message(wl_seat::pointer_button_event(
                                ptr_id, serial,
                                event.timestamp_sec as u32,
                                event.code as u32,
                                if pressed { wl_seat::button_state::PRESSED } else { wl_seat::button_state::RELEASED },
                            ));
                            client.send_message(wl_seat::pointer_frame_event(ptr_id));
                        }
                    }
                    continue;
                }

                keymap::process_key_event(scancode, pressed, modifiers);
                let keysym = keymap.translate(scancode, modifiers);

                let serial = seat.next_serial();
                for (&client_id, &kb_id) in client_keyboard_ids {
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(wl_seat::keyboard_key_event(
                            kb_id, serial,
                            event.timestamp_sec as u32,
                            scancode as u32,
                            if pressed { wl_seat::key_state::PRESSED } else { wl_seat::key_state::RELEASED },
                        ));
                        client.send_message(wl_seat::keyboard_modifiers_event(
                            kb_id, serial,
                            if modifiers.shift { 1 } else { 0 },
                            if modifiers.caps { 2 } else { 0 },
                            0, 0,
                        ));
                    }
                }

                if pressed && modifiers.shift && matches!(keysym, keymap::Keysym::Special(keymap::SpecialKey::Escape)) {
                    println!("[edgerun-compositor] Shift+ESC pressed, exiting...");
                    std::process::exit(0);
                }
            }
            edgerun_input::InputEventKind::RelativeMotion => {
                // evdev sends REL_X (code=0) and REL_Y (code=1) as separate events.
                // Buffer the X delta, apply both when Y arrives.
                const REL_X: u16 = 0;
                const REL_Y: u16 = 1;
                let dx = event.value as f64;
                if event.code == REL_X {
                    seat.pointer_x += dx;
                } else if event.code == REL_Y {
                    seat.pointer_y += dx;
                }

                // Check for active pointer constraints
                let mut constrained_dx = dx;
                let mut constrained_dy = 0f64;
                let mut is_locked = false;
                let mut locked_constraint_id = None;

                for (constraint_id, constraint) in pointer_constraints.iter_mut() {
                    if constraint.surface_id == seat.pointer_focus().unwrap_or(0) && constraint.activated {
                        match constraint_type_map.get(constraint_id) {
                            Some(ConstraintType::Lock) => {
                                // Locked pointer: send relative motion, don't move cursor
                                is_locked = true;
                                locked_constraint_id = Some(*constraint_id);
                                constrained_dx = dx;
                                constrained_dy = 0f64;
                                // Don't update seat position for locked pointers
                            }
                            Some(ConstraintType::Confine) => {
                                // Confined pointer: clamp to region if specified
                                if let Some((rx, ry, rw, rh)) = constraint.region {
                                    let surface_x = seat.pointer_x;
                                    let surface_y = seat.pointer_y;
                                    // Clamp to region
                                    seat.pointer_x = surface_x.max(rx as f64).min((rx + rw) as f64);
                                    seat.pointer_y = surface_y.max(ry as f64).min((ry + rh) as f64);
                                }
                                // Otherwise just track normally
                            }
                            None => {}
                        }
                    }
                }

                // Only update cursor position if not locked
                if !is_locked {
                    cursor.x = seat.pointer_x as i32;
                    cursor.y = seat.pointer_y as i32;
                }

                let serial = seat.next_serial();
                for (&client_id, &ptr_id) in client_pointer_ids {
                    if let Some(client) = server.client_mut(client_id) {
                        if is_locked {
                            // For locked pointers, send locked event instead of motion
                            if let Some(cid) = locked_constraint_id {
                                // Send relative motion as locked pointer motion
                                client.send_message(zwp_pointer_constraints::locked_pointer_motion_event(
                                    cid, event.timestamp_sec as u32,
                                    (constrained_dx * 65536.0) as i32 as u32,
                                    (constrained_dy * 65536.0) as i32 as u32,
                                ));
                            }
                        } else {
                            client.send_message(wl_seat::pointer_motion_event(
                                ptr_id, event.timestamp_sec as u32,
                                seat.pointer_x, seat.pointer_y,
                            ));
                        }
                        client.send_message(wl_seat::pointer_frame_event(ptr_id));
                    }
                }

                // Send relative pointer events to clients that requested them
                let dx_fixed = (dx * 65536.0) as i32 as u32;
                let dy_fixed = (0f64 * 65536.0) as i32 as u32;
                let utime = event.timestamp_sec as u64;
                let utime_hi = (utime >> 32) as u32;
                let utime_lo = utime as u32;
                for (&client_id, &rel_ptr_id) in client_relative_pointer_ids {
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(zwp_relative_pointer::relative_motion_event(
                            rel_ptr_id, utime_hi, utime_lo,
                            dx_fixed, dy_fixed, dx_fixed, dy_fixed,
                        ));
                    }
                }
            }
            edgerun_input::InputEventKind::AbsoluteMotion => {
                // Touch input: evdev sends ABS_MT_* events
                // ABS_MT_SLOT (0x3f): select touch slot
                // ABS_MT_TRACKING_ID (0x39): touch down/up identifier
                // ABS_MT_POSITION_X (0x35): X position
                // ABS_MT_POSITION_Y (0x36): Y position
                const ABS_MT_SLOT: u16 = 0x3f;
                const ABS_MT_TRACKING_ID: u16 = 0x39;
                const ABS_MT_POSITION_X: u16 = 0x35;
                const ABS_MT_POSITION_Y: u16 = 0x36;
                const ABS_X: u16 = 0x00;
                const ABS_Y: u16 = 0x01;

                match event.code {
                    ABS_MT_SLOT => {
                        // Switch active touch slot
                        touch_state.current_slot = event.value;
                        touch_state.slots.entry(event.value).or_insert_with(|| TouchSlot {
                            touch_id: touch_state.next_touch_id,
                            surface_id: None,
                            client_id: None,
                            x: 0.0,
                            y: 0.0,
                            active: false,
                        });
                    }
                    ABS_MT_TRACKING_ID => {
                        // -1 = touch up, >= 0 = touch down with this ID
                        let slot = touch_state.current_slot;
                        if event.value < 0 {
                            // Touch up
                            if let Some(touch_slot) = touch_state.slots.get_mut(&slot) {
                                if touch_slot.active {
                                    let serial = seat.next_serial();
                                    for (&cid, &touch_obj_id) in client_touch_ids.iter() {
                                        if let Some(client) = server.client_mut(cid) {
                                            client.send_message(wl_seat::touch_up_event(
                                                touch_obj_id, serial,
                                                event.timestamp_sec as u32,
                                                slot,
                                            ));
                                            client.send_message(wl_seat::touch_frame_event(touch_obj_id));
                                        }
                                    }
                                    touch_slot.active = false;
                                }
                            }
                        } else {
                            // Touch down
                            let slot_entry = touch_state.slots.entry(slot).or_insert_with(|| TouchSlot {
                                touch_id: touch_state.next_touch_id,
                                surface_id: seat.touch_focus(),
                                client_id: None,
                                x: 0.0,
                                y: 0.0,
                                active: false,
                            });
                            slot_entry.touch_id = event.value as u32;
                            slot_entry.active = true;
                            slot_entry.surface_id = seat.touch_focus();

                            let serial = seat.next_serial();
                            let surface_id = slot_entry.surface_id.unwrap_or(0);
                            for (&cid, &touch_obj_id) in client_touch_ids.iter() {
                                if let Some(client) = server.client_mut(cid) {
                                    client.send_message(wl_seat::touch_down_event(
                                        touch_obj_id, serial,
                                        event.timestamp_sec as u32,
                                        surface_id, slot,
                                        slot_entry.x, slot_entry.y,
                                    ));
                                    client.send_message(wl_seat::touch_frame_event(touch_obj_id));
                                }
                            }
                            touch_state.next_touch_id += 1;
                        }
                    }
                    ABS_MT_POSITION_X => {
                        let slot = touch_state.current_slot;
                        if let Some(touch_slot) = touch_state.slots.get_mut(&slot) {
                            // Normalize to 0.0-1.0 range (evdev ABS_X max is typically screen width)
                            touch_slot.x = event.value as f64 / 32767.0;
                            if touch_slot.active {
                                let serial = seat.next_serial();
                                for (&cid, &touch_obj_id) in client_touch_ids.iter() {
                                    if let Some(client) = server.client_mut(cid) {
                                        client.send_message(wl_seat::touch_motion_event(
                                            touch_obj_id, event.timestamp_sec as u32,
                                            slot, touch_slot.x, touch_slot.y,
                                        ));
                                        client.send_message(wl_seat::touch_frame_event(touch_obj_id));
                                    }
                                }
                            }
                        }
                        // Check for gestures after position update
                        detect_and_send_gestures(
                            touch_state, seat, server,
                            client_gesture_swipe_ids, client_gesture_pinch_ids,
                            event.timestamp_sec as u32,
                        );
                    }
                    ABS_MT_POSITION_Y => {
                        let slot = touch_state.current_slot;
                        if let Some(touch_slot) = touch_state.slots.get_mut(&slot) {
                            touch_slot.y = event.value as f64 / 32767.0;
                            if touch_slot.active {
                                let serial = seat.next_serial();
                                for (&cid, &touch_obj_id) in client_touch_ids.iter() {
                                    if let Some(client) = server.client_mut(cid) {
                                        client.send_message(wl_seat::touch_motion_event(
                                            touch_obj_id, event.timestamp_sec as u32,
                                            slot, touch_slot.x, touch_slot.y,
                                        ));
                                        client.send_message(wl_seat::touch_frame_event(touch_obj_id));
                                    }
                                }
                            }
                        }
                        // Check for gestures after position update
                        detect_and_send_gestures(
                            touch_state, seat, server,
                            client_gesture_swipe_ids, client_gesture_pinch_ids,
                            event.timestamp_sec as u32,
                        );
                    }
                    ABS_X | ABS_Y => {
                        // Single-touch ABS events (for touchscreens that don't use MT protocol)
                        // Treat as a single touch slot
                        let slot = 0;
                        touch_state.current_slot = slot;
                        let touch_slot = touch_state.slots.entry(slot).or_insert_with(|| TouchSlot {
                            touch_id: 0,
                            surface_id: seat.touch_focus(),
                            client_id: None,
                            x: 0.0,
                            y: 0.0,
                            active: true,
                        });

                        if event.code == ABS_X {
                            touch_slot.x = event.value as f64 / 32767.0;
                        } else {
                            touch_slot.y = event.value as f64 / 32767.0;
                        }

                        // Send motion event
                        let serial = seat.next_serial();
                        for (&cid, &touch_obj_id) in client_touch_ids.iter() {
                            if let Some(client) = server.client_mut(cid) {
                                client.send_message(wl_seat::touch_motion_event(
                                    touch_obj_id, event.timestamp_sec as u32,
                                    slot, touch_slot.x, touch_slot.y,
                                ));
                                client.send_message(wl_seat::touch_frame_event(touch_obj_id));
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// Detect and send pointer gesture events based on multi-touch state.
fn detect_and_send_gestures(
    touch_state: &mut TouchState,
    seat: &mut Seat,
    server: &mut WaylandServer,
    client_gesture_swipe_ids: &HashMap<u32, u32>,
    client_gesture_pinch_ids: &HashMap<u32, u32>,
    time: u32,
) {
    // Count active touch points
    let active_slots: Vec<_> = touch_state.slots.iter()
        .filter(|(_, s)| s.active)
        .map(|(k, v)| (*k, v.clone()))
        .collect();

    let finger_count = active_slots.len() as u32;

    if finger_count < 2 {
        // Less than 2 fingers — end any active gestures
        if touch_state.gesture_in_progress {
            // End swipe if active
            if touch_state.swipe_active {
                let serial = seat.next_serial();
                for (&_cid, &swipe_id) in client_gesture_swipe_ids.iter() {
                    if let Some(client) = server.client_mut(_cid) {
                        client.send_message(zwp_pointer_gestures::swipe_end_event(
                            swipe_id, serial, time, 0)); // not cancelled
                    }
                }
                touch_state.swipe_active = false;
            }
            // End pinch if active
            if touch_state.pinch_active {
                let serial = seat.next_serial();
                for (&_cid, &pinch_id) in client_gesture_pinch_ids.iter() {
                    if let Some(client) = server.client_mut(_cid) {
                        client.send_message(zwp_pointer_gestures::pinch_end_event(
                            pinch_id, serial, time, 0)); // not cancelled
                    }
                }
                touch_state.pinch_active = false;
            }
            touch_state.gesture_in_progress = false;
        }
        touch_state.active_fingers = finger_count;
        return;
    }

    // Calculate centroid of active touch points
    let sum_x: f64 = active_slots.iter().map(|(_, s)| s.x).sum();
    let sum_y: f64 = active_slots.iter().map(|(_, s)| s.y).sum();
    let centroid_x = sum_x / finger_count as f64;
    let centroid_y = sum_y / finger_count as f64;

    // Calculate average distance between touch points (for pinch detection)
    let mut total_dist = 0.0;
    let mut pair_count = 0;
    for (i, (_, s1)) in active_slots.iter().enumerate() {
        for (_, s2) in active_slots.iter().skip(i + 1) {
            let dx = s1.x - s2.x;
            let dy = s1.y - s2.y;
            total_dist += (dx * dx + dy * dy).sqrt();
            pair_count += 1;
        }
    }
    let avg_dist = if pair_count > 0 { total_dist / pair_count as f64 } else { 0.0 };

    // Check if this is the start of a new gesture
    if !touch_state.gesture_in_progress {
        touch_state.gesture_in_progress = true;
        touch_state.gesture_finger_count = finger_count;
        touch_state.gesture_start_time = time;
        touch_state.initial_pinch_distance = avg_dist;
        touch_state.prev_centroid_x = centroid_x;
        touch_state.prev_centroid_y = centroid_y;
        touch_state.swipe_active = true;
        touch_state.pinch_active = true;

        // Send swipe_begin and pinch_begin
        let serial = seat.next_serial();
        // Use the first active touch's surface as the gesture target
        let surface_id = active_slots.first().map(|(_, s)| s.surface_id).flatten().unwrap_or(0);

        for (&cid, &swipe_id) in client_gesture_swipe_ids.iter() {
            if let Some(client) = server.client_mut(cid) {
                client.send_message(zwp_pointer_gestures::swipe_begin_event(
                    swipe_id, serial, time, surface_id, finger_count));
            }
        }
        for (&cid, &pinch_id) in client_gesture_pinch_ids.iter() {
            if let Some(client) = server.client_mut(cid) {
                client.send_message(zwp_pointer_gestures::pinch_begin_event(
                    pinch_id, serial, time, surface_id, finger_count));
            }
        }
    }

    // Send update events if gestures are active
    let dx = centroid_x - touch_state.prev_centroid_x;
    let dy = centroid_y - touch_state.prev_centroid_y;

    // Swipe: significant movement with fingers moving together
    if touch_state.swipe_active && (dx.abs() > 0.0001 || dy.abs() > 0.0001) {
        let dx_fixed = (dx * 65536.0 * 10.0) as i32 as u32; // scale up for visibility
        let dy_fixed = (dy * 65536.0 * 10.0) as i32 as u32;
        for (&cid, &swipe_id) in client_gesture_swipe_ids.iter() {
            if let Some(client) = server.client_mut(cid) {
                client.send_message(zwp_pointer_gestures::swipe_update_event(
                    swipe_id, time, dx_fixed, dy_fixed));
            }
        }
    }

    // Pinch: distance change between fingers
    if touch_state.pinch_active && touch_state.initial_pinch_distance > 0.0 {
        let scale = if touch_state.initial_pinch_distance > 0.0001 {
            (avg_dist / touch_state.initial_pinch_distance * 65536.0) as u32
        } else {
            65536 // 1.0 scale
        };
        // Rotation would require angle calculation — simplified to 0 for now
        let rotation = 0u32;

        let dx_fixed = (dx * 65536.0 * 10.0) as i32 as u32;
        let dy_fixed = (dy * 65536.0 * 10.0) as i32 as u32;

        for (&cid, &pinch_id) in client_gesture_pinch_ids.iter() {
            if let Some(client) = server.client_mut(cid) {
                client.send_message(zwp_pointer_gestures::pinch_update_event(
                    pinch_id, time, dx_fixed, dy_fixed, scale, rotation));
            }
        }
    }

    // Update state for next frame
    touch_state.prev_centroid_x = centroid_x;
    touch_state.prev_centroid_y = centroid_y;
    touch_state.active_fingers = finger_count;
}
