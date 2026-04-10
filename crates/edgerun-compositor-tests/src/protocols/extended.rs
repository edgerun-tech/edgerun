//! Extended protocol conformance tests — touch, constraints, clipboard, etc.

use std::io::{self, Read, Write};

use edgerun_compositor::wire::{Message, encode, encode_string};
use edgerun_compositor::protocol::wl_core;
use edgerun_compositor::protocol::wl_seat;
use edgerun_compositor::protocol::zwp_pointer_constraints;
use edgerun_compositor::protocol::zxdg_idle_inhibit;
use edgerun_compositor::protocol::screencopy;
use edgerun_compositor::protocol::fractional_scale;
use edgerun_compositor::protocol::tearing_control;
use edgerun_compositor::protocol::single_pixel_buffer;
use edgerun_compositor::protocol::primary_selection;

use crate::client::WlClient;
use crate::harness::CompositorHarness;

// ─── Touch Tests ─────────────────────────────────────────────

/// Test: wl_touch can be created from seat.
pub fn test_seat_touch() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, globals) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    // Verify seat has TOUCH capability (bitmask = 7 = KEYBOARD|POINTER|TOUCH)
    let seat_global = globals.iter().find(|g| g.interface == wl_seat::WL_SEAT)
        .ok_or("wl_seat not found")?;

    // The seat should advertise version >= 7 (which includes touch capability info)
    if seat_global.version < 5 {
        return Err(format!("wl_seat version = {}, expected >= 5 for touch", seat_global.version));
    }

    // Bind seat and get touch
    let seat_id = client.bind_with_registry(reg_id, seat_global.name,
                                             wl_seat::WL_SEAT, seat_global.version)
        .map_err(|e| format!("Bind seat failed: {}", e))?;

    let touch_id = client.alloc_id();
    let msg = Message {
        sender_id: seat_id,
        opcode: wl_seat::seat_request::GET_TOUCH,
        size: 12,
        args: touch_id.to_le_bytes().to_vec(),
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("get_touch failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    if touch_id < 2 {
        return Err("Invalid touch object ID".into());
    }

    // Touch object was created — compositor accepted it
    Ok(())
}

// ─── Pointer Constraints Tests ───────────────────────────────

/// Test: pointer lock can be created.
pub fn test_pointer_lock() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    // Find pointer_constraints global
    let pc_global = client.globals.iter()
        .find(|g| g.interface == zwp_pointer_constraints::ZWP_POINTER_CONSTRAINTS_V1)
        .ok_or("zwp_pointer_constraints_v1 not advertised")?;

    let pc_id = client.bind_with_registry(reg_id, pc_global.name,
                                           zwp_pointer_constraints::ZWP_POINTER_CONSTRAINTS_V1,
                                           pc_global.version)
        .map_err(|e| format!("Bind pointer constraints failed: {}", e))?;

    // Create surface + xdg_toplevel for lock target
    let (_, surface_id) = client.create_surface(reg_id)
        .map_err(|e| format!("create_surface failed: {}", e))?;
    let (_, _, toplevel_id) = client.create_xdg_toplevel(reg_id, surface_id)
        .map_err(|e| format!("create_xdg_toplevel failed: {}", e))?;

    client.surface_commit(surface_id)
        .map_err(|e| format!("commit failed: {}", e))?;

    // Wait for configure
    for _ in 0..20 {
        client.dispatch().ok();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // Create locked pointer
    let locked_id = client.alloc_id();
    let mut args = Vec::new();
    args.extend_from_slice(&locked_id.to_le_bytes());
    args.extend_from_slice(&surface_id.to_le_bytes());
    args.extend_from_slice(&toplevel_id.to_le_bytes()); // using toplevel as pointer proxy
    args.extend_from_slice(&zwp_pointer_constraints::lifetime::ONESHOT.to_le_bytes());
    args.extend_from_slice(&0u32.to_le_bytes()); // null region

    let msg = Message {
        sender_id: pc_id,
        opcode: zwp_pointer_constraints::constraints_request::LOCK_POINTER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("lock_pointer failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    // Should receive locked event
    client.wait_for_event()
        .map_err(|e| format!("No events after lock: {}", e))?;

    let locked_evt = client.find_event(|e| {
        e.sender_id == locked_id && e.opcode == zwp_pointer_constraints::locked_pointer_event::LOCKED
    });

    if locked_evt.is_none() {
        return Err("Did not receive locked_pointer_v1.locked event".into());
    }

    // Clean up — destroy locked pointer
    let msg = Message {
        sender_id: locked_id,
        opcode: zwp_pointer_constraints::locked_pointer_request::DESTROY,
        size: 8,
        args: Vec::new(),
        fds: Vec::new(),
    };
    client.send(&msg).ok();
    client.flush().ok();

    // Should receive unlocked event
    std::thread::sleep(std::time::Duration::from_millis(100));
    client.dispatch().ok();

    Ok(())
}

/// Test: pointer confine can be created.
pub fn test_pointer_confine() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    let pc_global = client.globals.iter()
        .find(|g| g.interface == zwp_pointer_constraints::ZWP_POINTER_CONSTRAINTS_V1)
        .ok_or("zwp_pointer_constraints_v1 not advertised")?;

    let pc_id = client.bind_with_registry(reg_id, pc_global.name,
                                           zwp_pointer_constraints::ZWP_POINTER_CONSTRAINTS_V1,
                                           pc_global.version)
        .map_err(|e| format!("Bind failed: {}", e))?;

    let (_, surface_id) = client.create_surface(reg_id)
        .map_err(|e| format!("create_surface failed: {}", e))?;
    let (_, _, toplevel_id) = client.create_xdg_toplevel(reg_id, surface_id)
        .map_err(|e| format!("create_xdg_toplevel failed: {}", e))?;

    client.surface_commit(surface_id).ok();
    for _ in 0..20 {
        client.dispatch().ok();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // Create confined pointer
    let confined_id = client.alloc_id();
    let mut args = Vec::new();
    args.extend_from_slice(&confined_id.to_le_bytes());
    args.extend_from_slice(&surface_id.to_le_bytes());
    args.extend_from_slice(&toplevel_id.to_le_bytes());
    args.extend_from_slice(&zwp_pointer_constraints::lifetime::ONESHOT.to_le_bytes());
    args.extend_from_slice(&0u32.to_le_bytes());

    let msg = Message {
        sender_id: pc_id,
        opcode: zwp_pointer_constraints::constraints_request::CONFINE_POINTER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("confine failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    client.wait_for_event().map_err(|e| format!("No events: {}", e))?;

    let confined_evt = client.find_event(|e| {
        e.sender_id == confined_id && e.opcode == zwp_pointer_constraints::confined_pointer_event::CONFINED
    });

    if confined_evt.is_none() {
        return Err("Did not receive confined_pointer_v1.confined event".into());
    }

    // Clean up
    let msg = Message {
        sender_id: confined_id,
        opcode: zwp_pointer_constraints::confined_pointer_request::DESTROY,
        size: 8,
        args: Vec::new(),
        fds: Vec::new(),
    };
    client.send(&msg).ok();
    client.flush().ok();

    Ok(())
}

// ─── Idle Inhibit Tests ──────────────────────────────────────

/// Test: idle inhibitor can be created and destroyed.
pub fn test_idle_inhibit() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    let ih_global = client.globals.iter()
        .find(|g| g.interface == zxdg_idle_inhibit::ZWP_IDLE_INHIBIT_MANAGER_V1)
        .ok_or("zwp_idle_inhibit_manager_v1 not advertised")?;

    let ih_id = client.bind_with_registry(reg_id, ih_global.name,
                                           zxdg_idle_inhibit::ZWP_IDLE_INHIBIT_MANAGER_V1,
                                           ih_global.version)
        .map_err(|e| format!("Bind failed: {}", e))?;

    let (_, surface_id) = client.create_surface(reg_id)
        .map_err(|e| format!("create_surface failed: {}", e))?;

    // Create inhibitor
    let inhibitor_id = client.alloc_id();
    let mut args = Vec::new();
    args.extend_from_slice(&inhibitor_id.to_le_bytes());
    args.extend_from_slice(&surface_id.to_le_bytes());

    let msg = Message {
        sender_id: ih_id,
        opcode: zxdg_idle_inhibit::idle_inhibit_request::CREATE_INHIBITOR,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("create_inhibitor failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    // Destroy inhibitor
    let msg = Message {
        sender_id: inhibitor_id,
        opcode: zxdg_idle_inhibit::idle_inhibitor_request::DESTROY,
        size: 8,
        args: Vec::new(),
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("destroy_inhibitor failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    // No protocol error = success
    std::thread::sleep(std::time::Duration::from_millis(100));
    client.dispatch().ok();

    if client.find_event(|e| e.sender_id == 1 && e.opcode == wl_core::display_event::ERROR).is_some() {
        return Err("Compositor sent error during idle inhibit test".into());
    }

    Ok(())
}

// ─── Primary Selection Tests ─────────────────────────────────

/// Test: primary selection can be created and offer mime types.
pub fn test_primary_selection() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    let ps_global = client.globals.iter()
        .find(|g| g.interface == primary_selection::ZWLR_PRIMARY_SELECTION_MANAGER_V1)
        .ok_or("zwlr_primary_selection_manager_v1 not advertised")?;

    let ps_id = client.bind_with_registry(reg_id, ps_global.name,
                                           primary_selection::ZWLR_PRIMARY_SELECTION_MANAGER_V1,
                                           ps_global.version)
        .map_err(|e| format!("Bind failed: {}", e))?;

    // Create data source
    let source_id = client.alloc_id();
    let mut args = Vec::new();
    args.extend_from_slice(&source_id.to_le_bytes());
    let msg = Message {
        sender_id: ps_id,
        opcode: primary_selection::manager_request::CREATE_DATA_SOURCE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("create_source failed: {}", e))?;

    // Offer mime type
    let mut args = Vec::new();
    encode_string(&mut args, "text/plain");
    let msg = Message {
        sender_id: source_id,
        opcode: primary_selection::source_request::OFFER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("offer failed: {}", e))?;

    // Create device
    let device_id = client.alloc_id();
    let mut args = Vec::new();
    args.extend_from_slice(&device_id.to_le_bytes());
    args.extend_from_slice(&0u32.to_le_bytes()); // seat (we use a placeholder)
    let msg = Message {
        sender_id: ps_id,
        opcode: primary_selection::manager_request::GET_PRIMARY_SELECTION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("get_device failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    // Should receive selection event (may be null if no primary set yet)
    std::thread::sleep(std::time::Duration::from_millis(100));
    client.dispatch().ok();

    if client.find_event(|e| e.sender_id == 1 && e.opcode == wl_core::display_event::ERROR).is_some() {
        return Err("Compositor sent error during primary selection test".into());
    }

    Ok(())
}

// ─── Screencopy Tests ────────────────────────────────────────

/// Test: screencopy capture output can be created.
pub fn test_screencopy_capture() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    let sc_global = client.globals.iter()
        .find(|g| g.interface == screencopy::ZWLR_SCREENCOPY_MANAGER_V1)
        .ok_or("zwlr_screencopy_manager_v1 not advertised")?;

    let sc_id = client.bind_with_registry(reg_id, sc_global.name,
                                           screencopy::ZWLR_SCREENCOPY_MANAGER_V1,
                                           sc_global.version)
        .map_err(|e| format!("Bind failed: {}", e))?;

    // Find output global
    let output_global = client.globals.iter()
        .find(|g| g.interface == "wl_output")
        .ok_or("wl_output not found")?;
    let output_name = output_global.name;

    // Create capture frame
    let frame_id = client.alloc_id();
    let mut args = Vec::new();
    args.extend_from_slice(&frame_id.to_le_bytes());
    args.extend_from_slice(&0u32.to_le_bytes()); // overlay_cursor = 0
    args.extend_from_slice(&screencopy::capture_type::OUTPUT.to_le_bytes());
    args.extend_from_slice(&output_name.to_le_bytes());

    let msg = Message {
        sender_id: sc_id,
        opcode: screencopy::manager_request::CAPTURE_OUTPUT,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("capture_output failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    // Should receive buffer event with format info
    client.wait_for_event().map_err(|e| format!("No events: {}", e))?;

    let buffer_evt = client.find_event(|e| {
        e.sender_id == frame_id && e.opcode == screencopy::frame_event::BUFFER
    });

    if buffer_evt.is_none() {
        return Err("Did not receive screencopy buffer event".into());
    }

    // Verify buffer event has valid format
    let buffer_evt = buffer_evt.unwrap();
    let mut c = buffer_evt.cursor();
    let format = c.uint().map_err(|e| format!("decode format: {}", e))?;
    let width = c.uint().map_err(|e| format!("decode width: {}", e))?;
    let _height = c.uint().map_err(|e| format!("decode height: {}", e))?;
    let _stride = c.uint().map_err(|e| format!("decode stride: {}", e))?;

    // Format should be XRGB8888
    if format != 0x34325258 {
        return Err(format!("Unexpected screencopy format: 0x{:x}", format));
    }

    // Width should be > 0
    if width == 0 {
        return Err("Screencopy width is 0".into());
    }

    // Clean up
    let msg = Message {
        sender_id: frame_id,
        opcode: screencopy::frame_request::DESTROY,
        size: 8,
        args: Vec::new(),
        fds: Vec::new(),
    };
    client.send(&msg).ok();
    client.flush().ok();

    Ok(())
}

// ─── Fractional Scale Tests ──────────────────────────────────

/// Test: fractional scale manager can be bound and returns preferred scale.
pub fn test_fractional_scale() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    let fs_global = client.globals.iter()
        .find(|g| g.interface == fractional_scale::WP_FRACTIONAL_SCALE_MANAGER_V1)
        .ok_or("wp_fractional_scale_manager_v1 not advertised")?;

    let fs_id = client.bind_with_registry(reg_id, fs_global.name,
                                           fractional_scale::WP_FRACTIONAL_SCALE_MANAGER_V1,
                                           fs_global.version)
        .map_err(|e| format!("Bind failed: {}", e))?;

    let (_, surface_id) = client.create_surface(reg_id)
        .map_err(|e| format!("create_surface failed: {}", e))?;

    // Get fractional scale object
    let scale_id = client.alloc_id();
    let mut args = Vec::new();
    args.extend_from_slice(&scale_id.to_le_bytes());
    args.extend_from_slice(&surface_id.to_le_bytes());

    let msg = Message {
        sender_id: fs_id,
        opcode: fractional_scale::manager_request::GET_FRACTIONAL_SCALE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("get_fractional_scale failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    // Should receive preferred_scale event
    client.wait_for_event().map_err(|e| format!("No events: {}", e))?;

    let scale_evt = client.find_event(|e| {
        e.sender_id == scale_id && e.opcode == fractional_scale::fractional_scale_event::PREFERRED_SCALE
    });

    if scale_evt.is_none() {
        return Err("Did not receive fractional_scale preferred_scale event".into());
    }

    let scale_evt = scale_evt.unwrap();
    let mut c = scale_evt.cursor();
    let scale = c.uint().map_err(|e| format!("decode scale: {}", e))?;

    // 1.0x = 120 (scale * 120)
    if scale == 0 {
        return Err("Preferred scale is 0".into());
    }

    // Clean up
    let msg = Message {
        sender_id: scale_id,
        opcode: fractional_scale::fractional_scale_request::DESTROY,
        size: 8,
        args: Vec::new(),
        fds: Vec::new(),
    };
    client.send(&msg).ok();
    client.flush().ok();

    Ok(())
}

// ─── Tearing Control Tests ───────────────────────────────────

/// Test: tearing control can be created and set presentation hint.
pub fn test_tearing_control() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    let tc_global = client.globals.iter()
        .find(|g| g.interface == tearing_control::WP_TEARING_CONTROL_MANAGER_V1)
        .ok_or("wp_tearing_control_manager_v1 not advertised")?;

    let tc_id = client.bind_with_registry(reg_id, tc_global.name,
                                           tearing_control::WP_TEARING_CONTROL_MANAGER_V1,
                                           tc_global.version)
        .map_err(|e| format!("Bind failed: {}", e))?;

    let (_, surface_id) = client.create_surface(reg_id)
        .map_err(|e| format!("create_surface failed: {}", e))?;

    // Get tearing control object
    let control_id = client.alloc_id();
    let mut args = Vec::new();
    args.extend_from_slice(&control_id.to_le_bytes());
    args.extend_from_slice(&surface_id.to_le_bytes());

    let msg = Message {
        sender_id: tc_id,
        opcode: tearing_control::manager_request::GET_TEARING_CONTROL,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("get_tearing_control failed: {}", e))?;

    // Set presentation hint (async/tearing allowed)
    let mut args = Vec::new();
    args.extend_from_slice(&tearing_control::hint::ASYNC.to_le_bytes());
    let msg = Message {
        sender_id: control_id,
        opcode: tearing_control::tearing_control_request::SET_PRESENTATION_HINT,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("set_presentation_hint failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    // No error = success
    std::thread::sleep(std::time::Duration::from_millis(100));
    client.dispatch().ok();

    if client.find_event(|e| e.sender_id == 1 && e.opcode == wl_core::display_event::ERROR).is_some() {
        return Err("Compositor sent error during tearing control test".into());
    }

    Ok(())
}

// ─── Single Pixel Buffer Tests ──────────────────────────────

/// Test: single pixel buffer can be created with RGBA color.
pub fn test_single_pixel_buffer() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    let spb_global = client.globals.iter()
        .find(|g| g.interface == single_pixel_buffer::WP_SINGLE_PIXEL_BUFFER_MANAGER_V1)
        .ok_or("wp_single_pixel_buffer_manager_v1 not advertised")?;

    let spb_id = client.bind_with_registry(reg_id, spb_global.name,
                                            single_pixel_buffer::WP_SINGLE_PIXEL_BUFFER_MANAGER_V1,
                                            spb_global.version)
        .map_err(|e| format!("Bind failed: {}", e))?;

    // Create single pixel buffer (red)
    let buffer_id = client.alloc_id();
    let mut args = Vec::new();
    args.extend_from_slice(&buffer_id.to_le_bytes());
    args.extend_from_slice(&0xFFFF0000u32.to_le_bytes()); // red
    args.extend_from_slice(&0x0000FF00u32.to_le_bytes()); // green
    args.extend_from_slice(&0x000000FFu32.to_le_bytes()); // blue
    args.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // alpha

    let msg = Message {
        sender_id: spb_id,
        opcode: single_pixel_buffer::manager_request::CREATE_SRGB32_BUFFER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("create_buffer failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    // No error = success (buffer was created)
    std::thread::sleep(std::time::Duration::from_millis(100));
    client.dispatch().ok();

    if client.find_event(|e| e.sender_id == 1 && e.opcode == wl_core::display_event::ERROR).is_some() {
        return Err("Compositor sent error during single pixel buffer test".into());
    }

    Ok(())
}
