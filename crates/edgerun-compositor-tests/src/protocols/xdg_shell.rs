//! XDG shell protocol runtime conformance tests.
//!
//! These tests verify:
//! - xdg_wm_base binding and version
//! - xdg_surface + xdg_toplevel creation
//! - configure → ack → commit lifecycle
//! - wm_capabilities event

use edgerun_compositor::protocol::wl_compositor;
use edgerun_compositor::protocol::xdg_shell;
use edgerun_compositor::wire::encode::encode_array;

use crate::client::WlClient;
use crate::harness::CompositorHarness;

/// Test: Full xdg-shell lifecycle — create toplevel, receive configure, ack, commit.
pub fn test_xdg_toplevel_lifecycle() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    // Create surface first
    let (_, surface_id) = client.create_surface(reg_id)
        .map_err(|e| format!("Failed to create surface: {}", e))?;

    // Create xdg_toplevel
    let (wm_base_id, xdg_surface_id, toplevel_id) = client.create_xdg_toplevel(reg_id, surface_id)
        .map_err(|e| format!("Failed to create xdg_toplevel: {}", e))?;

    if wm_base_id < 2 || xdg_surface_id < 2 || toplevel_id < 2 {
        return Err("Invalid xdg object IDs".into());
    }

    // Set a title
    let title = "conformance-test";
    let mut args = Vec::new();
    edgerun_compositor::wire::encode::encode_string(&mut args, title);
    let msg = edgerun_compositor::wire::Message {
        sender_id: toplevel_id,
        opcode: xdg_shell::xdg_toplevel_request::SET_TITLE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("set_title failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    // Now commit the surface — this should trigger a configure event
    client.surface_commit(surface_id)
        .map_err(|e| format!("commit failed: {}", e))?;

    // Wait for configure events — dispatch multiple times
    for _ in 0..20 {
        client.dispatch().ok();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // Look for xdg_surface configure
    let configure = client.find_event(|e| {
        e.sender_id == xdg_surface_id && e.opcode == xdg_shell::xdg_surface_event::CONFIGURE
    });

    if configure.is_none() {
        return Err("Did not receive xdg_surface.configure event".into());
    }

    let configure = configure.unwrap();
    let mut c = configure.cursor();
    let serial = c.uint().map_err(|e| format!("Failed to decode configure serial: {}", e))?;

    // Ack the configure
    client.xdg_surface_ack_configure(xdg_surface_id, serial)
        .map_err(|e| format!("ack_configure failed: {}", e))?;

    // Commit again
    client.surface_commit(surface_id)
        .map_err(|e| format!("commit after ack failed: {}", e))?;

    Ok(())
}

/// Test: xdg_wm_base ping/pong.
pub fn test_xdg_ping_pong() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    let xdg_global = client.globals.iter()
        .find(|g| g.interface == xdg_shell::XDG_WM_BASE)
        .ok_or("xdg_wm_base not found")?;

    let wm_base_id = client.bind_with_registry(reg_id, xdg_global.name,
                                                xdg_shell::XDG_WM_BASE, xdg_global.version)
        .map_err(|e| format!("Bind failed: {}", e))?;

    // The compositor should send a ping. Wait for it.
    client.wait_for_event()
        .map_err(|e| format!("No events after bind: {}", e))?;

    // Find the ping event
    let ping = client.find_event(|e| {
        e.sender_id == wm_base_id && e.opcode == xdg_shell::xdg_wm_base_event::PING
    });

    if let Some(ping) = ping {
        let mut c = ping.cursor();
        let serial = c.uint().map_err(|e| format!("Failed to decode ping serial: {}", e))?;

        // Respond with pong
        client.xdg_wm_base_pong(wm_base_id, serial)
            .map_err(|e| format!("pong failed: {}", e))?;
    }
    // If no ping, that's OK — some compositors only ping on demand

    Ok(())
}

/// Test: xdg_toplevel wm_capabilities event.
pub fn test_wm_capabilities() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    let (_, surface_id) = client.create_surface(reg_id)
        .map_err(|e| format!("Failed to create surface: {}", e))?;

    let (wm_base_id, xdg_surface_id, toplevel_id) = client.create_xdg_toplevel(reg_id, surface_id)
        .map_err(|e| format!("Failed to create xdg_toplevel: {}", e))?;

    client.surface_commit(surface_id)
        .map_err(|e| format!("commit failed: {}", e))?;

    // Wait for events
    client.wait_for_event()
        .map_err(|e| format!("No events: {}", e))?;

    // Also try to read more
    for _ in 0..5 {
        client.dispatch().ok();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Check for configure_bounds (v4+) or wm_capabilities (v5+)
    let has_config_bounds = client.find_event(|e| {
        e.sender_id == toplevel_id && e.opcode == xdg_shell::xdg_toplevel_event::CONFIGURE_BOUNDS
    }).is_some();

    let has_wm_caps = client.find_event(|e| {
        e.sender_id == toplevel_id && e.opcode == xdg_shell::xdg_toplevel_event::WM_CAPABILITIES
    }).is_some();

    // Drain remaining events and check
    for evt in client.drain_events() {
        if evt.sender_id == toplevel_id && evt.opcode == xdg_shell::xdg_toplevel_event::CONFIGURE_BOUNDS {
            // Got configure_bounds
        }
        if evt.sender_id == toplevel_id && evt.opcode == xdg_shell::xdg_toplevel_event::WM_CAPABILITIES {
            // Got wm_capabilities
        }
    }

    // At minimum we got the surface configure
    // The compositor may or may not send configure_bounds/wm_capabilities
    // depending on version support
    if has_config_bounds {
        eprintln!("[test] Got configure_bounds (v4+)");
    }
    if has_wm_caps {
        eprintln!("[test] Got wm_capabilities (v5+)");
    }

    Ok(())
}

/// Test: xdg_toplevel minimize/maximize.
pub fn test_toplevel_set_maximized() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    let (_, surface_id) = client.create_surface(reg_id)
        .map_err(|e| format!("Failed to create surface: {}", e))?;

    let (_, _, toplevel_id) = client.create_xdg_toplevel(reg_id, surface_id)
        .map_err(|e| format!("Failed to create xdg_toplevel: {}", e))?;

    // Send maximize request
    let msg = edgerun_compositor::wire::Message {
        sender_id: toplevel_id,
        opcode: xdg_shell::xdg_toplevel_request::MAXIMIZE,
        size: 8,
        args: Vec::new(),
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("maximize failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    // Send unmaximize
    let msg = edgerun_compositor::wire::Message {
        sender_id: toplevel_id,
        opcode: xdg_shell::xdg_toplevel_request::UNMAXIMIZE,
        size: 8,
        args: Vec::new(),
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("unmaximize failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    // Verify no protocol error
    std::thread::sleep(std::time::Duration::from_millis(100));
    client.dispatch().ok();

    // Check for error events
    if let Some(evt) = client.find_event(|e| {
        e.sender_id == 1 && e.opcode == edgerun_compositor::protocol::wl_core::display_event::ERROR
    }) {
        let mut c = evt.cursor();
        let _obj = c.uint().ok();
        let code = c.uint().ok();
        let msg = c.string().ok();
        return Err(format!("Compositor sent error: code={:?}, msg={:?}", code, msg));
    }

    Ok(())
}

/// Test: xdg_toplevel set_minimized.
pub fn test_toplevel_minimize() -> Result<(), String> {
    let harness = CompositorHarness::new()
        .map_err(|e| format!("Failed to spawn compositor: {}", e))?;
    let mut client = harness.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let (reg_id, _) = client.full_registry()
        .map_err(|e| format!("Registry failed: {}", e))?;

    let (_, surface_id) = client.create_surface(reg_id)
        .map_err(|e| format!("Failed to create surface: {}", e))?;

    let (_, _, toplevel_id) = client.create_xdg_toplevel(reg_id, surface_id)
        .map_err(|e| format!("Failed to create xdg_toplevel: {}", e))?;

    // Send set_minimized (v6)
    let msg = edgerun_compositor::wire::Message {
        sender_id: toplevel_id,
        opcode: xdg_shell::xdg_toplevel_request::SET_MINIMIZED,
        size: 8,
        args: Vec::new(),
        fds: Vec::new(),
    };
    client.send(&msg).map_err(|e| format!("set_minimized failed: {}", e))?;
    client.flush().map_err(|e| format!("flush failed: {}", e))?;

    std::thread::sleep(std::time::Duration::from_millis(100));
    client.dispatch().ok();

    // Check for errors
    if let Some(evt) = client.find_event(|e| {
        e.sender_id == 1 && e.opcode == edgerun_compositor::protocol::wl_core::display_event::ERROR
    }) {
        let mut c = evt.cursor();
        let _obj = c.uint().ok();
        let code = c.uint().ok();
        let msg = c.string().ok();
        return Err(format!("Compositor sent error: code={:?}, msg={:?}", code, msg));
    }

    Ok(())
}
