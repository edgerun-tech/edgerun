//! edgerun-compositor — Wayland compositor binary.
//!
//! Usage: `edgerun-compositor [socket_path]`
//!
//! Default socket path: `/tmp/edgerun-wayland-0`

use std::collections::HashMap;
use std::io;
use std::os::fd::RawFd;
use std::time::Instant;

use edgerun_compositor::client::Client;
use edgerun_compositor::compositor::dmabuf::DmabufParams;
use edgerun_compositor::compositor::output::Output;
use edgerun_compositor::compositor::seat::Seat;
use edgerun_compositor::compositor::shell::Shell;
use edgerun_compositor::compositor::surface::{BufferRegistry, ShmBufferInfo, SurfaceBuffer, SurfaceTree};
use edgerun_compositor::drm;
use edgerun_compositor::drm::device::DrmDevice;
use edgerun_compositor::drm::dumb::DumbBuffer;
use edgerun_compositor::drm::kms;
use edgerun_compositor::input::evdev::EvdevManager;
use edgerun_compositor::input::keymap::{self, Keymap, Modifiers};
use edgerun_compositor::r#loop::{EventLoop, EventSource};
use edgerun_compositor::protocol;
use edgerun_compositor::protocol::linux_dmabuf;
use edgerun_compositor::protocol::wl_compositor;
use edgerun_compositor::protocol::wl_core;
use edgerun_compositor::protocol::wl_data_device;
use edgerun_compositor::protocol::wl_output;
use edgerun_compositor::protocol::wl_seat;
use edgerun_compositor::protocol::wl_shm;
use edgerun_compositor::protocol::wl_subcompositor;
use edgerun_compositor::protocol::wp_cursor_shape;
use edgerun_compositor::protocol::wp_presentation_time;
use edgerun_compositor::protocol::wp_viewporter;
use edgerun_compositor::protocol::xdg_activation;
use edgerun_compositor::protocol::xdg_decoration;
use edgerun_compositor::protocol::xdg_shell;
use edgerun_compositor::protocol::zwp_pointer_gestures;
use edgerun_compositor::protocol::zwp_relative_pointer;
use edgerun_compositor::protocol::zwp_text_input;
use edgerun_compositor::protocol::zxdg_idle_inhibit;
use edgerun_compositor::render::cursor::{Cursor, CursorShape};
use edgerun_compositor::render::shm::ShmManager;
use edgerun_compositor::gpu::compositor::GlCompositor;
use edgerun_compositor::resource::Registry;
use edgerun_compositor::server::WaylandServer;
use edgerun_compositor::wire;
use edgerun_compositor::wire::decode::ArgCursor;
use edgerun_compositor::wire::encode::*;

/// Clipboard data source state.
#[derive(Debug)]
struct DataSource {
    id: u32,
    mime_types: Vec<String>,
}

fn main() {
    let socket_path = std::env::args().nth(1).unwrap_or_else(|| "/tmp/edgerun-wayland-0".to_string());

    println!("[edgerun-compositor] Starting...");

    // ─── Open DRM device ──────────────────────────────────────
    let devices = drm::find_card_devices();
    if devices.is_empty() {
        eprintln!("No DRM devices found!");
        std::process::exit(1);
    }

    let drm_device = match DrmDevice::open(&devices[0]) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to open DRM device {}: {}", devices[0], e);
            std::process::exit(1);
        }
    };

    println!("[edgerun-compositor] DRM device: {}", devices[0]);

    let _ = drm_device.set_client_cap(drm::ioctl::client_cap::ATOMIC, 1);
    let _ = drm_device.set_client_cap(drm::ioctl::client_cap::UNIVERSAL_PLANES, 1);

    let resources = match drm_device.get_resources() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to get DRM resources: {}", e);
            std::process::exit(1);
        }
    };

    println!("[edgerun-compositor] {} connectors, {} CRTCs", resources.count_connectors, resources.count_crtcs);

    let mut selected_connector = None;
    let mut selected_crtc = None;
    let mut selected_mode = None;

    for &conn_id in &resources.connectors {
        let conn = match drm_device.get_connector(conn_id) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if !conn.is_connected() || conn.modes.is_empty() {
            continue;
        }

        for &crtc_id in &resources.crtcs {
            let crtc = match drm_device.get_crtc(crtc_id) {
                Ok(c) => c,
                Err(_) => continue,
            };

            if crtc.mode_valid != 0 && crtc.mode.hdisplay == 0 {
                continue;
            }

            selected_connector = Some((conn_id, conn.clone()));
            selected_crtc = Some(crtc_id);
            selected_mode = Some(conn.modes[0]);
            break;
        }

        if selected_connector.is_some() {
            break;
        }
    }

    let (conn_id, conn) = match selected_connector {
        Some(c) => c,
        None => {
            eprintln!("No connected display found!");
            std::process::exit(1);
        }
    };

    let crtc_id = match selected_crtc {
        Some(id) => id,
        None => {
            eprintln!("Failed to select a CRTC!");
            std::process::exit(1);
        }
    };
    let mode = match selected_mode {
        Some(m) => m,
        None => {
            eprintln!("Failed to select a display mode!");
            std::process::exit(1);
        }
    };

    println!(
        "[edgerun-compositor] Output: {} ({}) at {}x{}@{}Hz",
        conn.type_name(),
        conn.connector_type_id,
        mode.hdisplay,
        mode.vdisplay,
        mode.vrefresh
    );

    // ─── Set mode with dumb buffer ────────────────────────────
    let mut dumb = match DumbBuffer::create(drm_device.as_raw_fd(), mode.hdisplay as u32, mode.vdisplay as u32, 32) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to create dumb buffer: {}", e);
            std::process::exit(1);
        }
    };

    let fb_id = match dumb.add_fb() {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Failed to create framebuffer: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = kms::set_crtc(drm_device.as_raw_fd(), crtc_id, fb_id, conn_id, &mode) {
        eprintln!("Failed to set mode: {}", e);
        std::process::exit(1);
    }

    println!("[edgerun-compositor] Mode set: fb={fb_id}");

    // Initial clear — fill with dark blue-gray
    // DRM framebuffer uses XRGB8888 format, which in little-endian memory is [B, G, R, X]
    let fb_pixels = match dumb.map() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to map dumb buffer: {}", e);
            std::process::exit(1);
        }
    };

    // Clear with dark blue-gray: R=0x1a, G=0x1a, B=0x2e
    // In XRGB8888 little-endian: [B=0x2e, G=0x1a, R=0x1a, X=0xff]
    for i in (0..fb_pixels.len()).step_by(4) {
        if i + 3 < fb_pixels.len() {
            fb_pixels[i] = 0x2e;     // B
            fb_pixels[i + 1] = 0x1a; // G
            fb_pixels[i + 2] = 0x1a; // R
            fb_pixels[i + 3] = 0xff; // X (unused)
        }
    }

    // ─── GPU Compositor (EGL + GL) ──────────────────────────
    let mut gl_compositor = GlCompositor::new(drm_device.as_raw_fd(), mode.hdisplay as u32, mode.vdisplay as u32);
    if gl_compositor.is_some() {
        println!("[edgerun-compositor] GPU compositor active — GL-accelerated compositing enabled");
    } else {
        eprintln!("[edgerun-compositor] GPU compositor failed to initialize — falling back to software rendering");
    }

    // ─── Wayland server ──────────────────────────────────────
    let mut server = match WaylandServer::new(&socket_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create Wayland server: {}", e);
            std::process::exit(1);
        }
    };

    println!("[edgerun-compositor] Wayland socket: {socket_path}");

    // ─── Compositor state ────────────────────────────────────
    let mut surfaces = SurfaceTree::new();
    let mut buffers = BufferRegistry::new();
    let mut shm = ShmManager::new();
    let mut shell = Shell::new(0);
    shell.set_output_size(mode.hdisplay as i32, mode.vdisplay as i32);
    let mut seat_obj = Seat::new(0);
    let mut keymap_obj = Keymap::estonian_nodeadkeys();
    let mut modifiers = Modifiers::default();
    let mut cursor = Cursor::new();

    // Pending dmabuf params per client
    let mut dmabuf_pending: HashMap<u32, DmabufParams> = HashMap::new();

    // Clipboard: currently active data source
    let mut current_data_source: Option<DataSource> = None;
    let mut selection_offer_counter: u32 = 0;

    // Registry (globals advertised to clients)
    let mut global_name: u32 = 1;

    let compositor_global = { global_name += 1; global_name };
    let shm_global = { global_name += 1; global_name };
    let seat_global = { global_name += 1; global_name };
    let xdg_wm_base_global = { global_name += 1; global_name };
    let output_global = { global_name += 1; global_name };
    let dmabuf_global = { global_name += 1; global_name };
    let data_device_manager_global = { global_name += 1; global_name };
    let subcompositor_global = { global_name += 1; global_name };
    let decoration_manager_global = { global_name += 1; global_name };
    let viewporter_global = { global_name += 1; global_name };
    let cursor_shape_manager_global = { global_name += 1; global_name };
    let activation_global = { global_name += 1; global_name };
    let presentation_global = { global_name += 1; global_name };
    let relative_pointer_manager_global = { global_name += 1; global_name };
    let pointer_gestures_global = { global_name += 1; global_name };
    let text_input_manager_global = { global_name += 1; global_name };
    let idle_inhibit_manager_global = { global_name += 1; global_name };

    let _output = Output::from_drm(
        0,
        conn.type_name(),
        mode.hdisplay as u32,
        mode.vdisplay as u32,
        mode.vrefresh * 1000,
        conn.mm_width,
        conn.mm_height,
        conn.type_name(),
    );

    // Input
    let mut input_mgr = EvdevManager::new();
    match input_mgr.discover() {
        Ok(devs) => {
            for dev in &devs {
                println!("[edgerun-compositor] Input: {} ({:?})", dev.name, dev.kind);
            }
        }
        Err(e) => eprintln!("Input discovery failed: {e}"),
    }

    // ─── Launch Firefox browser ────────────────────
    let socket_path_clone = socket_path.to_string();
    let _browser_child = std::process::Command::new("firefox")
        .env("WAYLAND_DISPLAY", &socket_path_clone)
        .env("MOZ_ENABLE_WAYLAND", "1")
        .env("MOZ_DISABLE_RDD_SANDBOX", "1")
        .env("MOZ_WEBRENDER", "0")
        .env("GDK_BACKEND", "wayland")
        .arg("https://www.google.com")
        .spawn();
    match &_browser_child {
        Ok(child) => println!("[edgerun-compositor] Launched Firefox (PID {})", child.id()),
        Err(e) => eprintln!("[edgerun-compositor] Failed to launch Firefox: {e}"),
    }

    // ─── Event loop ──────────────────────────────────────────
    let mut event_loop = EventLoop::new().expect("Failed to create event loop");

    event_loop.add_read(server.listen_fd(), EventSource::WaylandListen)
        .expect("Failed to register Wayland socket");

    // Register input device FDs with epoll — evdev FDs are readable only when
    // new events are available. When idle, read() returns EAGAIN (not 0 bytes).
    for dev_id in input_mgr.device_ids().collect::<Vec<_>>() {
        if let Some(fd) = input_mgr.device_fd(dev_id) {
            if let Err(e) = event_loop.add_read(fd, EventSource::EvdevDevice(dev_id)) {
                eprintln!("[edgerun-compositor] Failed to register evdev fd for device {dev_id}: {e}");
            }
        }
    }

    // Register DRM fd with edge-triggered epoll for page flip completion events.
    // We use EPOLLET to avoid spin: the fd is always "ready" after a flip event
    // until the event is consumed via read().
    if let Err(e) = event_loop.add_read_edge_triggered(drm_device.as_raw_fd(), EventSource::DrmDevice) {
        eprintln!("[edgerun-compositor] Failed to register DRM fd: {e}");
    }

    // Per-client registries and object id mappings
    let mut client_registries: HashMap<u32, Registry> = HashMap::new();
    let mut client_registry_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_compositor_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_shm_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_seat_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_xdg_base_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_output_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_keyboard_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_pointer_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_dmabuf_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_data_device_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_data_source_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_subcompositor_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_decoration_manager_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_decoration_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_viewporter_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_cursor_shape_manager_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_cursor_shape_device_ids: HashMap<u32, u32> = HashMap::new();

    // Track which client-side pool id maps to which internal pool
    let mut client_pool_map: HashMap<u32, HashMap<u32, u32>> = HashMap::new();

    // Per-client cursor surface tracking
    let mut client_cursor_surfaces: HashMap<u32, u32> = HashMap::new(); // client_id -> surface_id

    // Map from xdg_surface id to wl_surface id (needed for keyboard/pointer enter)
    let mut xdg_surface_to_wl_surface: HashMap<u32, u32> = HashMap::new();

    let mut config_serial: u32 = 1;
    let mut frame_count: u64 = 0;
    let _start_time = Instant::now();
    let mut old_fb_ids: Vec<u32> = Vec::new(); // Track old FB IDs for cleanup

    println!("[edgerun-compositor] Running event loop...");

    'main_loop: loop {
        let events = match event_loop.wait(16) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Event loop error: {e}");
                break;
            }
        };

        // On timeout (no events), render and fire frame callbacks — our VBLANK ticker
        if events.is_empty() {
            frame_count += 1;

            // Fire frame callbacks
            let mut frame_callbacks: Vec<(u32, u32)> = Vec::new();
            for (&cid, _reg) in &client_registry_ids {
                if let Some(&compositor_id) = client_compositor_ids.get(&cid) {
                    for surface in surfaces.surfaces_mut() {
                        for &cb_id in &surface.frame_callbacks {
                            frame_callbacks.push((compositor_id, cb_id));
                        }
                        surface.frame_callbacks.clear();
                    }
                }
            }

            let serial = frame_count as u32;
            for (compositor_id, cb_id) in frame_callbacks {
                for &cid in client_registry_ids.keys() {
                    if let Some(client) = server.client_mut(cid) {
                        client.send_message(
                            wl_core::callback_done_event(cb_id, serial)
                        );
                    }
                }
            }

            // Render all surfaces and page flip
            if let Some(gl) = gl_compositor.as_mut() {
                // GPU-accelerated path
                gl.composite(&surfaces, &shell, &cursor, &mut dumb);
            } else {
                // Software fallback
                render_and_flip(
                    &mut dumb, fb_id, crtc_id, &drm_device,
                    &surfaces, &shell, &cursor, &shm,
                    &mut old_fb_ids,
                );
            }

            // Clean up old FB IDs periodically
            if old_fb_ids.len() > 4 {
                // Keep last 4 to be safe (triple buffering + margin)
                let to_remove = old_fb_ids.drain(..old_fb_ids.len() - 4);
                for old_id in to_remove {
                    let _ = kms::rmfb(drm_device.as_raw_fd(), old_id);
                }
            }
        }

        for (source, evmask) in events {
            match source {
                EventSource::WaylandListen => {
                    if evmask & (libc::EPOLLIN as u32) != 0 {
                        if let Ok(Some(client_id)) = server.accept_client() {
                            println!("[edgerun-compositor] Client {client_id} connected");

                            // Register the new client's socket fd with epoll so we can read its messages
                            let client_fd = server.client_mut(client_id).unwrap().fd();
                            event_loop.add_read(client_fd, EventSource::WaylandClient(client_id))
                                .expect("Failed to register client socket");

                            let mut cr = Registry::new();
                            let registry_id = cr.alloc_id();
                            cr.register(registry_id, "wl_registry", 1, client_id);

                            client_registries.insert(client_id, cr);
                            client_registry_ids.insert(client_id, registry_id);
                            client_pool_map.insert(client_id, HashMap::new());
                            dmabuf_pending.insert(client_id, DmabufParams::new());

                            send_globals_to_client(
                                &mut server, client_id, registry_id,
                                compositor_global, shm_global, seat_global,
                                xdg_wm_base_global, output_global, dmabuf_global,
                                data_device_manager_global, subcompositor_global,
                                decoration_manager_global, viewporter_global,
                                cursor_shape_manager_global, activation_global,
                                presentation_global, relative_pointer_manager_global,
                                pointer_gestures_global, text_input_manager_global,
                                idle_inhibit_manager_global,
                            );

                            // Flush immediately — the client socket is read-only in epoll,
                            // so EPOLLOUT never fires to drain the send queue.
                            if let Some(client) = server.client_mut(client_id) {
                                match client.flush() {
                                    Ok(()) => println!("[edgerun-compositor] globals flushed to client {}", client_id),
                                    Err(e) => eprintln!("[edgerun-compositor] flush error: {}", e),
                                }
                            }
                        }
                    }
                }

                EventSource::WaylandClient(client_id) => {
                    if evmask & (libc::EPOLLIN as u32) != 0 {
                        if let Some(client) = server.client_mut(client_id) {
                            if let Err(e) = client.recv() {
                                if e.kind() != io::ErrorKind::WouldBlock {
                                    println!("[edgerun-compositor] Client {client_id} read error: {e}");
                                    client.disconnected = true;
                                }
                            }
                        }

                        let (messages, disconnected) = {
                            if let Some(client) = server.client_mut(client_id) {
                                (client.drain_messages(), client.disconnected)
                            } else {
                                (Vec::new(), false)
                            }
                        };

                        for msg in messages {
                            process_message(
                                &mut server, client_id, msg,
                                &mut surfaces, &mut buffers, &mut shm, &mut shell,
                                &mut seat_obj, &mut keymap_obj, &mut modifiers,
                                &mut cursor,
                                &mut client_registries, &mut client_registry_ids,
                                &mut client_compositor_ids, &mut client_shm_ids,
                                &mut client_seat_ids, &mut client_xdg_base_ids,
                                &mut client_output_ids, &mut client_keyboard_ids,
                                &mut client_pointer_ids, &mut client_dmabuf_ids,
                                &mut client_pool_map,
                                &mut client_data_device_ids, &mut client_data_source_ids,
                                &mut client_subcompositor_ids,
                                &mut client_decoration_manager_ids, &mut client_decoration_ids,
                                &mut client_viewporter_ids,
                                &mut client_cursor_shape_manager_ids, &mut client_cursor_shape_device_ids,
                                &mut client_cursor_surfaces,
                                &mut xdg_surface_to_wl_surface,
                                &mut dmabuf_pending,
                                &mut current_data_source, &mut selection_offer_counter,
                                &mut config_serial,
                            );
                        }

                        // Flush any responses (sync_done, configure, etc.)
                        if let Some(client) = server.client_mut(client_id) {
                            let _ = client.flush();
                        }

                        if disconnected {
                            // Remove client socket from epoll before removing the client
                            if let Some(client) = server.client_mut(client_id) {
                                let _ = event_loop.remove(client.fd());
                            }
                            println!("[edgerun-compositor] Client {client_id} disconnected");
                            client_registries.remove(&client_id);
                            client_registry_ids.remove(&client_id);
                            client_compositor_ids.remove(&client_id);
                            client_shm_ids.remove(&client_id);
                            client_seat_ids.remove(&client_id);
                            client_xdg_base_ids.remove(&client_id);
                            client_output_ids.remove(&client_id);
                            client_keyboard_ids.remove(&client_id);
                            client_pointer_ids.remove(&client_id);
                            client_dmabuf_ids.remove(&client_id);
                            client_data_device_ids.remove(&client_id);
                            client_data_source_ids.remove(&client_id);
                            client_subcompositor_ids.remove(&client_id);
                            client_decoration_manager_ids.remove(&client_id);
                            client_decoration_ids.remove(&client_id);
                            client_viewporter_ids.remove(&client_id);
                            client_cursor_shape_manager_ids.remove(&client_id);
                            client_cursor_shape_device_ids.remove(&client_id);
                            client_pool_map.remove(&client_id);
                            client_cursor_surfaces.remove(&client_id);
                            dmabuf_pending.remove(&client_id);
                            server.remove_client(client_id);
                        }
                    }

                    if evmask & (libc::EPOLLOUT as u32) != 0 {
                        if let Some(client) = server.client_mut(client_id) {
                            let _ = client.flush();
                        }
                    }
                }

                EventSource::DrmDevice => {
                    // DRM page flip completion event — read to clear the edge-triggered epoll
                    let _ = drm::kms::read_events(drm_device.as_raw_fd());
                }

                EventSource::EvdevDevice(dev_id) => {
                    // Input event available — process it immediately (event-driven, no polling)
                    if evmask & (libc::EPOLLIN as u32) != 0 {
                        process_input_for_device(
                            &mut input_mgr, dev_id, &mut seat_obj, &mut keymap_obj,
                            &mut modifiers, &mut server, &surfaces, &mut shell,
                            &client_keyboard_ids, &client_pointer_ids,
                            &client_compositor_ids, &mut cursor,
                        );
                    }
                }
            }
        }
    }

    println!("[edgerun-compositor] Shutting down...");
    let _ = kms::rmfb(drm_device.as_raw_fd(), fb_id);
    let _ = dumb.destroy();
    let _ = std::fs::remove_file(&socket_path);
}

// ─── Helper functions ────────────────────────────────────────

fn send_globals_to_client(
    server: &mut WaylandServer,
    client_id: u32,
    registry_id: u32,
    compositor_global: u32,
    shm_global: u32,
    seat_global: u32,
    xdg_wm_base_global: u32,
    output_global: u32,
    dmabuf_global: u32,
    data_device_manager_global: u32,
    subcompositor_global: u32,
    decoration_manager_global: u32,
    viewporter_global: u32,
    cursor_shape_manager_global: u32,
    activation_global: u32,
    presentation_global: u32,
    relative_pointer_manager_global: u32,
    pointer_gestures_global: u32,
    text_input_manager_global: u32,
    idle_inhibit_manager_global: u32,
) {
    let globals = [
        (compositor_global, wl_compositor::WL_COMPOSITOR, wl_compositor::WL_COMPOSITOR_VERSION),
        (shm_global, wl_shm::WL_SHM, wl_shm::WL_SHM_VERSION),
        (seat_global, wl_seat::WL_SEAT, wl_seat::WL_SEAT_VERSION),
        (xdg_wm_base_global, xdg_shell::XDG_WM_BASE, xdg_shell::XDG_WM_BASE_VERSION),
        (output_global, wl_output::WL_OUTPUT, wl_output::WL_OUTPUT_VERSION),
        (dmabuf_global, linux_dmabuf::ZWP_LINUX_DMABUF_V1, linux_dmabuf::ZWP_LINUX_DMABUF_V1_VERSION),
        (data_device_manager_global, wl_data_device::WL_DATA_DEVICE_MANAGER, wl_data_device::WL_DATA_DEVICE_MANAGER_VERSION),
        (subcompositor_global, wl_subcompositor::WL_SUBCOMPOSITOR, wl_subcompositor::WL_SUBCOMPOSITOR_VERSION),
        (decoration_manager_global, xdg_decoration::ZXDG_DECORATION_MANAGER_V1, xdg_decoration::ZXDG_DECORATION_MANAGER_V1_VERSION),
        (viewporter_global, wp_viewporter::WP_VIEWPORTER, wp_viewporter::WP_VIEWPORTER_VERSION),
        (cursor_shape_manager_global, wp_cursor_shape::WP_CURSOR_SHAPE_MANAGER_V1, wp_cursor_shape::WP_CURSOR_SHAPE_MANAGER_V1_VERSION),
        (activation_global, xdg_activation::ZXDG_ACTIVATION_V1, xdg_activation::ZXDG_ACTIVATION_V1_VERSION),
        (presentation_global, wp_presentation_time::WP_PRESENTATION, wp_presentation_time::WP_PRESENTATION_VERSION),
        (relative_pointer_manager_global, zwp_relative_pointer::ZWP_RELATIVE_POINTER_MANAGER_V1, zwp_relative_pointer::ZWP_RELATIVE_POINTER_MANAGER_V1_VERSION),
        (pointer_gestures_global, zwp_pointer_gestures::ZWP_POINTER_GESTURES_V1, zwp_pointer_gestures::ZWP_POINTER_GESTURES_V1_VERSION),
        (text_input_manager_global, zwp_text_input::ZWP_TEXT_INPUT_MANAGER_V1, zwp_text_input::ZWP_TEXT_INPUT_MANAGER_V1_VERSION),
        (idle_inhibit_manager_global, zxdg_idle_inhibit::ZWP_IDLE_INHIBIT_MANAGER_V1, zxdg_idle_inhibit::ZWP_IDLE_INHIBIT_MANAGER_V1_VERSION),
    ];

    for &(name, interface, version) in &globals {
        let mut args = Vec::new();
        args.extend_from_slice(&name.to_le_bytes());
        crate::wire::encode::encode_string(&mut args, interface);
        args.extend_from_slice(&version.to_le_bytes());

        if let Some(client) = server.client_mut(client_id) {
            client.send_message(wire::Message {
                sender_id: registry_id,
                opcode: wl_core::registry_event::GLOBAL,
                size: (8 + args.len()) as u16,
                args,
                fds: Vec::new(),
            });
        }
    }
}

fn process_message(
    server: &mut WaylandServer,
    client_id: u32,
    msg: wire::Message,
    surfaces: &mut SurfaceTree,
    buffers: &mut BufferRegistry,
    shm: &mut ShmManager,
    shell: &mut Shell,
    seat: &mut Seat,
    _keymap: &mut Keymap,
    _modifiers: &mut Modifiers,
    cursor: &mut Cursor,
    client_registries: &mut HashMap<u32, Registry>,
    client_registry_ids: &mut HashMap<u32, u32>,
    client_compositor_ids: &mut HashMap<u32, u32>,
    client_shm_ids: &mut HashMap<u32, u32>,
    client_seat_ids: &mut HashMap<u32, u32>,
    client_xdg_base_ids: &mut HashMap<u32, u32>,
    client_output_ids: &mut HashMap<u32, u32>,
    client_keyboard_ids: &mut HashMap<u32, u32>,
    client_pointer_ids: &mut HashMap<u32, u32>,
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
    client_cursor_surfaces: &mut HashMap<u32, u32>,
    xdg_surface_to_wl_surface: &mut HashMap<u32, u32>,
    dmabuf_pending: &mut HashMap<u32, DmabufParams>,
    current_data_source: &mut Option<DataSource>,
    selection_offer_counter: &mut u32,
    config_serial: &mut u32,
) {
    let interface = {
        let reg = match client_registries.get(&client_id) {
            Some(r) => r,
            None => return,
        };
        reg.interface(msg.sender_id).unwrap_or("").to_string()
    };

    match interface.as_str() {
        "wl_display" => {
            match msg.opcode {
                wl_core::display_request::SYNC => {
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let callback_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(callback_id, "wl_callback", 1, client_id);
                    }
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(wl_core::callback_done_event(callback_id, 0));
                    }
                }
                wl_core::display_request::GET_REGISTRY => {
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let registry_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(registry_id, "wl_registry", 1, client_id);
                    }
                    client_registry_ids.insert(client_id, registry_id);
                }
                _ => {}
            }
        }

        "wl_registry" => {
            match msg.opcode {
                wl_core::registry_request::BIND => {
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let name = cursor_obj.uint().unwrap_or(0);
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
                            seat.capabilities = wl_seat::capability::KEYBOARD | wl_seat::capability::POINTER;
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
                            // Send output events — only those supported by the client's version
                            if let Some(client) = server.client_mut(client_id) {
                                // geometry (v1+)
                                let mut geo_args = Vec::new();
                                geo_args.extend_from_slice(&0i32.to_le_bytes()); // x
                                geo_args.extend_from_slice(&0i32.to_le_bytes()); // y
                                geo_args.extend_from_slice(&1920i32.to_le_bytes()); // physical_width
                                geo_args.extend_from_slice(&1080i32.to_le_bytes()); // physical_height
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
                                mode_args.extend_from_slice(&1920i32.to_le_bytes()); // width
                                mode_args.extend_from_slice(&1080i32.to_le_bytes()); // height
                                mode_args.extend_from_slice(&60000i32.to_le_bytes()); // refresh (mHz)
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
                                    scale_args.extend_from_slice(&1i32.to_le_bytes());
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
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        "wl_compositor" => {
            match msg.opcode {
                wl_compositor::compositor_request::CREATE_SURFACE => {
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let surface_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(surface_id, "wl_surface", 4, client_id);
                    }
                    surfaces.create(surface_id);
                }
                _ => {}
            }
        }

        "wl_surface" => {
            match msg.opcode {
                wl_compositor::surface_request::ATTACH => {
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let x = cursor_obj.int().unwrap_or(0);
                    let y = cursor_obj.int().unwrap_or(0);
                    let w = cursor_obj.int().unwrap_or(0);
                    let h = cursor_obj.int().unwrap_or(0);
                    surfaces.damage(msg.sender_id, x, y, w, h);
                }
                wl_compositor::surface_request::FRAME => {
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let callback_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(callback_id, "wl_callback", 1, client_id);
                    }
                    surfaces.add_frame_callback(msg.sender_id, callback_id);
                }
                wl_compositor::surface_request::COMMIT => {
                    surfaces.commit(msg.sender_id);
                }
                wl_compositor::surface_request::SET_BUFFER_SCALE => {
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let scale = cursor_obj.int().unwrap_or(1);
                    surfaces.set_buffer_scale(msg.sender_id, scale);
                }
                wl_compositor::surface_request::DESTROY => {
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
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
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

                    // Create a memfd for the keymap
                    let keymap_fd = unsafe {
                        libc::memfd_create(b"edgerun-keymap\0".as_ptr() as *const libc::c_char, 0)
                    };
                    if keymap_fd >= 0 {
                        use std::io::Write;
                        use std::os::fd::{FromRawFd, IntoRawFd};
                        let mut file = unsafe { std::fs::File::from_raw_fd(keymap_fd) };
                        file.write_all(keymap_str.as_bytes()).expect("Failed to write keymap");
                        let keymap_fd = file.into_raw_fd();

                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(wl_seat::keyboard_keymap_event(
                                kb_id,
                                1, // Keymap format: XKB_V1
                                keymap_fd,
                                keymap_size,
                            ));
                        }
                    }
                }
                wl_seat::seat_request::GET_POINTER => {
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                wl_seat::seat_request::GET_TOUCH => {}
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
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let _serial = cursor_obj.uint().unwrap_or(0);
                    let surface_id = cursor_obj.object().unwrap_or(0);
                    let hotspot_x = cursor_obj.int().unwrap_or(0);
                    let hotspot_y = cursor_obj.int().unwrap_or(0);

                    cursor.hotspot_x = hotspot_x;
                    cursor.hotspot_y = hotspot_y;

                    if surface_id != 0 {
                        if let Some(surface) = surfaces.get(surface_id) {
                            cursor.surface_id = Some(surface_id);
                            cursor.surface_width = surface.width as i32;
                            cursor.surface_height = surface.height as i32;
                            client_cursor_surfaces.insert(client_id, surface_id);
                        }
                    } else {
                        cursor.surface_id = None;
                        client_cursor_surfaces.remove(&client_id);
                    }
                }
                wl_seat::pointer_request::RELEASE => {}
                _ => {}
            }
        }

        "wl_data_device_manager" => {
            match msg.opcode {
                wl_data_device::dnd_manager_request::CREATE_DATA_SOURCE => {
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let source_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(source_id, "wl_data_source", 3, client_id);
                    }
                    client_data_source_ids.insert(client_id, source_id);
                }
                wl_data_device::dnd_manager_request::GET_DATA_DEVICE => {
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
                    if let Ok(Some(mime_type)) = cursor_obj.string() {
                        client_data_source_ids.entry(client_id).and_modify(|_| {});
                        // Store mime type for this source — we'll track via the current_data_source
                        // For now, just note it was offered
                        let _ = (&mime_type);
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

        "wl_data_device" => {
            match msg.opcode {
                wl_data_device::data_device_request::SET_SELECTION => {
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let source_id = cursor_obj.object().unwrap_or(0);
                    let _serial = cursor_obj.uint().unwrap_or(0);

                    if source_id != 0 {
                        *selection_offer_counter += 1;
                        let offer_id = *selection_offer_counter;

                        if let Some(reg) = client_registries.get_mut(&client_id) {
                            reg.register(offer_id, "wl_data_offer", 3, client_id);
                        }

                        // Send data_offer event first, then selection
                        if let Some(client) = server.client_mut(client_id) {
                            if let Some(&device_id) = client_data_device_ids.get(&client_id) {
                                client.send_message(wl_data_device::data_device_data_offer_event(device_id, offer_id));
                                client.send_message(wl_data_device::data_device_selection_event(device_id, offer_id));
                            }
                        }

                        *current_data_source = Some(DataSource {
                            id: source_id,
                            mime_types: vec!["text/plain".to_string(), "text/plain;charset=utf-8".to_string()],
                        });
                    } else {
                        // Clear selection
                        if let Some(client) = server.client_mut(client_id) {
                            if let Some(&device_id) = client_data_device_ids.get(&client_id) {
                                client.send_message(wl_data_device::data_device_selection_event(device_id, 0));
                            }
                        }
                        *current_data_source = None;
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
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                wp_viewporter::viewport_request::SET_SOURCE => {}
                wp_viewporter::viewport_request::SET_DESTINATION => {
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let width = cursor_obj.int().unwrap_or(-1);
                    let height = cursor_obj.int().unwrap_or(-1);
                    // Update surface size if needed
                    if width > 0 && height > 0 {
                        if let Some(surface) = surfaces.get_mut(msg.sender_id) {
                            surface.width = width as u32;
                            surface.height = height as u32;
                        }
                    }
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
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let _serial = cursor_obj.uint().unwrap_or(0);
                    let shape_id = cursor_obj.uint().unwrap_or(0);

                    let shape = match shape_id {
                        1 => CursorShape::Default,
                        4 => CursorShape::Pointer,
                        9 => CursorShape::Text,
                        16 => CursorShape::Grab,
                        17 => CursorShape::Grabbing,
                        13 => CursorShape::Move,
                        19 => CursorShape::ResizeN,
                        22 => CursorShape::ResizeS,
                        18 => CursorShape::ResizeE,
                        25 => CursorShape::ResizeW,
                        20 => CursorShape::ResizeNE,
                        21 => CursorShape::ResizeNW,
                        23 => CursorShape::ResizeSE,
                        24 => CursorShape::ResizeSW,
                        26 => CursorShape::ResizeEW,
                        27 => CursorShape::ResizeNS,
                        28 => CursorShape::ResizeNESW,
                        29 => CursorShape::ResizeNWSE,
                        6 => CursorShape::Wait,
                        5 => CursorShape::Progress,
                        3 => CursorShape::Help,
                        15 => CursorShape::NotAllowed,
                        12 => CursorShape::Copy,
                        11 => CursorShape::Alias,
                        7 => CursorShape::Cell,
                        8 => CursorShape::Crosshair,
                        30 => CursorShape::ZoomIn,
                        31 => CursorShape::ZoomOut,
                        _ => CursorShape::Default,
                    };
                    cursor.shape = shape;
                    cursor.surface_id = None; // Clear client surface, use builtin
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
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let token_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(token_id, "xdg_activation_token_v1", 1, client_id);
                    }
                    // Send a done event with a simple token
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(xdg_activation::activation_done_event(
                            token_id,
                            &format!("edgerun-token-{}", token_id),
                        ));
                    }
                }
                xdg_activation::xdg_activation_request::ACTIVATE => {
                    // Client wants to activate a window - just ack it
                    // In a full implementation, you'd focus the requested surface
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
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let _surface_id = cursor_obj.object().unwrap_or(0);
                    let feedback_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(feedback_id, "wp_presentation_feedback", 1, client_id);
                    }
                    // In a full implementation, you'd track this and send presented/discarded
                    // events after each frame. For now, we'll send discarded immediately
                    // since we don't have precise timing yet.
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(wp_presentation_time::feedback_discarded_event(
                            feedback_id,
                            0, // reason: 0 = default
                        ));
                    }
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
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let _pointer_id = cursor_obj.object().unwrap_or(0);
                    let relative_pointer_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(relative_pointer_id, "zwp_relative_pointer_v1", 1, client_id);
                    }
                    // Client will now receive relative motion events
                    // These would be sent alongside regular pointer motion events
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
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let _pointer_id = cursor_obj.object().unwrap_or(0);
                    let gesture_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(gesture_id, "zwp_gesture_swipe_v1", 1, client_id);
                    }
                    // Client can now receive swipe gesture events
                }
                zwp_pointer_gestures::pointer_gestures_request::GET_PINCH_GESTURE => {
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                zwp_pointer_gestures::gesture_swipe_request::DESTROY |
                zwp_pointer_gestures::gesture_pinch_request::DESTROY => {
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
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let _seat_id = cursor_obj.object().unwrap_or(0);
                    let surface_id = cursor_obj.object().unwrap_or(0);
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(zwp_text_input::enter_event(msg.sender_id, surface_id));
                    }
                }
                zwp_text_input::text_input_request::DEACTIVATE => {
                    let mut cursor_obj = ArgCursor::new(&msg);
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

        "zwp_idle_inhibit_manager_v1" => {
            match msg.opcode {
                zxdg_idle_inhibit::idle_inhibit_request::CREATE_INHIBITOR => {
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let _surface_id = cursor_obj.object().unwrap_or(0);
                    let inhibitor_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(inhibitor_id, "zwp_idle_inhibitor_v1", 1, client_id);
                    }
                    // Inhibitor created - in full implementation, track which surfaces are inhibiting idle
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
                }
                _ => {}
            }
        }

        "xdg_wm_base" => {
            match msg.opcode {
                xdg_shell::xdg_wm_base_request::GET_XDG_SURFACE => {
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let pos_id = cursor_obj.new_id().unwrap_or(0);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(pos_id, "xdg_positioner", 6, client_id);
                    }
                }
                xdg_shell::xdg_wm_base_request::PONG => {}
                _ => {}
            }
        }

        "xdg_surface" => {
            match msg.opcode {
                xdg_shell::xdg_surface_request::GET_TOPLEVEL => {
                    let mut cursor_obj = ArgCursor::new(&msg);
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
                        client.send_message(xdg_shell::xdg_toplevel_configure_event(
                            toplevel_id,
                            shell.output_width,
                            shell.output_height,
                            &state,
                        ));
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
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let popup_id = cursor_obj.new_id().unwrap_or(0);
                    let _parent = cursor_obj.object().ok();
                    let _positioner = cursor_obj.object().ok();
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.register(popup_id, "xdg_popup", 6, client_id);
                    }
                    shell.create_popup(popup_id, msg.sender_id, None);
                }
                xdg_shell::xdg_surface_request::ACK_CONFIGURE => {
                    let mut cursor_obj = ArgCursor::new(&msg);
                    let serial = cursor_obj.uint().unwrap_or(0);
                    if let Some(tl) = shell.toplevels.values_mut().find(|tl| tl.surface_id == msg.sender_id) {
                        if tl.configure_serial == Some(serial) {
                            tl.configured = true;
                        }
                    }
                }
                xdg_shell::xdg_surface_request::SET_WINDOW_GEOMETRY => {
                    let _ = (&msg);
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
                    let mut cursor_obj = ArgCursor::new(&msg);
                    if let Ok(Some(title)) = cursor_obj.string() {
                        if let Some(tl) = shell.toplevels.values_mut().find(|tl| tl.id == msg.sender_id) {
                            tl.title = Some(title.0.clone());
                            println!("[edgerun-compositor] Title: {}", title.0);
                        }
                    }
                }
                xdg_shell::xdg_toplevel_request::SET_APP_ID => {
                    let mut cursor_obj = ArgCursor::new(&msg);
                    if let Ok(Some(app_id)) = cursor_obj.string() {
                        if let Some(tl) = shell.toplevels.values_mut().find(|tl| tl.id == msg.sender_id) {
                            tl.app_id = Some(app_id.0.clone());
                            println!("[edgerun-compositor] App ID: {}", app_id.0);
                        }
                    }
                }
                xdg_shell::xdg_toplevel_request::SET_MIN_SIZE => {
                    let _ = (&msg);
                }
                xdg_shell::xdg_toplevel_request::SET_MAX_SIZE => {
                    let _ = (&msg);
                }
                xdg_shell::xdg_toplevel_request::MAXIMIZE | xdg_shell::xdg_toplevel_request::SET_MAXIMIZED => {
                    let toplevel_id = shell.toplevels.values().find(|tl| tl.id == msg.sender_id).map(|tl| tl.id);
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
                    let toplevel_id = shell.toplevels.values().find(|tl| tl.id == msg.sender_id).map(|tl| tl.id);
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
                    let toplevel_id = shell.toplevels.values().find(|tl| tl.id == msg.sender_id).map(|tl| tl.id);
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
                    let toplevel_id = shell.toplevels.values().find(|tl| tl.id == msg.sender_id).map(|tl| tl.id);
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
                    let _ = (&msg);
                }
                xdg_shell::xdg_toplevel_request::RESIZE => {
                    let _ = (&msg);
                }
                xdg_shell::xdg_toplevel_request::SET_PARENT => {
                    let _ = (&msg);
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

        "xdg_popup" => {
            match msg.opcode {
                xdg_shell::xdg_popup_request::DESTROY => {
                    shell.destroy_popup(msg.sender_id);
                    if let Some(reg) = client_registries.get_mut(&client_id) {
                        reg.destroy(msg.sender_id);
                    }
                }
                xdg_shell::xdg_popup_request::GRAB => {}
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
                    let mut cursor_obj = ArgCursor::new(&msg);
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

        _ => {}
    }
}

/// Process input events for a single device (called from epoll event handler).
fn process_input_for_device(
    input_mgr: &mut EvdevManager,
    dev_id: u32,
    seat: &mut Seat,
    keymap: &mut Keymap,
    modifiers: &mut Modifiers,
    server: &mut WaylandServer,
    surfaces: &SurfaceTree,
    shell: &mut Shell,
    client_keyboard_ids: &HashMap<u32, u32>,
    client_pointer_ids: &HashMap<u32, u32>,
    _client_compositor_ids: &HashMap<u32, u32>,
    cursor: &mut Cursor,
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

                edgerun_compositor::input::keymap::process_key_event(scancode, pressed, modifiers);
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
                let dx = event.value as f64;
                let dy = 0f64;
                seat.pointer_x += dx;
                seat.pointer_y += dy;

                cursor.x = seat.pointer_x as i32;
                cursor.y = seat.pointer_y as i32;

                let _serial = seat.next_serial();
                for (&client_id, &ptr_id) in client_pointer_ids {
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(wl_seat::pointer_motion_event(
                            ptr_id, event.timestamp_sec as u32,
                            seat.pointer_x, seat.pointer_y,
                        ));
                        client.send_message(wl_seat::pointer_frame_event(ptr_id));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Render all surfaces to the scanout buffer and issue a page flip.
fn render_and_flip(
    dumb: &mut DumbBuffer,
    fb_id: u32,
    crtc_id: u32,
    drm_device: &DrmDevice,
    surfaces: &SurfaceTree,
    shell: &Shell,
    cursor: &Cursor,
    shm: &ShmManager,
    old_fb_ids: &mut Vec<u32>,
) {
    // Map the dumb buffer once — DumbBuffer::map() caches the mapping internally
    let fb_slice = match dumb.map() {
        Ok(p) => p,
        Err(_) => return,
    };
    let pixels = unsafe { std::slice::from_raw_parts_mut(fb_slice.as_mut_ptr(), fb_slice.len()) };

    let width = dumb.width;
    let height = dumb.height;
    let stride = dumb.pitch;

    // Always clear the framebuffer — surfaces with alpha (like Firefox)
    // need a clean background each frame to avoid ghosting artifacts.
    {
        // Clear to dark blue-gray
        // XRGB8888 little-endian: [B=0x2e, G=0x1a, R=0x1a, X=0xff] => 0xff1a1a2e
        let pixel_count = (width * height) as usize;
        let pixels_u32 = unsafe {
            std::slice::from_raw_parts_mut(pixels.as_mut_ptr() as *mut u32, pixel_count)
        };
        let bg_pixel = 0xff1a1a2eu32;
        for p in pixels_u32.iter_mut() {
            *p = bg_pixel;
        }
    }

    // Composite surfaces in z-order (toplevels + subsurfaces)
    // Always render surfaces with buffers — surfaces are composited
    // back-to-front onto the cleared framebuffer each frame.
    for toplevel in shell.toplevels_z_order() {
        let surface_id = toplevel.surface_id;
        if let Some(surface) = surfaces.get(surface_id) {
            if surface.buffer.is_none() {
                continue;
            }
            if let Some(ref buf) = surface.buffer {
                blit_surface_buffer(buf, pixels, surface.x, surface.y, width, height, stride, shm);
            }
        }

        // Composite subsurfaces
        for sub in shell.subsurfaces.for_parent(surface_id) {
            if let Some(sub_surface) = surfaces.get(sub.surface_id) {
                if sub_surface.buffer.is_some() {
                    if let Some(ref buf) = sub_surface.buffer {
                        blit_surface_buffer(buf, pixels, sub.x, sub.y, width, height, stride, shm);
                    }
                }
            }
        }
    }

    // Draw cursor
    cursor.draw(pixels, width, height, stride, None, 0);

    // Page flip to display the rendered buffer
    let new_fb_id = match dumb.add_fb() {
        Ok(id) => id,
        Err(_) => return,
    };

    // Track old FB IDs for cleanup (avoid resource leak)
    if fb_id != 0 {
        old_fb_ids.push(fb_id);
    }

    let user_data = kms::next_flip_serial();
    let _ = kms::page_flip(drm_device.as_raw_fd(), crtc_id, new_fb_id, user_data);
}

/// Blit a surface buffer onto the pixel array.
///
/// Uses cached SHM pool mappings instead of re-mmap/munmap per call.
/// Uses integer alpha blending for performance.
fn blit_surface_buffer(
    buf: &SurfaceBuffer,
    pixels: &mut [u8],
    origin_x: i32,
    origin_y: i32,
    output_width: u32,
    output_height: u32,
    output_stride: u32,
    shm: &ShmManager,
) {
    match buf {
        SurfaceBuffer::Shm { pool_fd, offset, width: buf_w, height: buf_h, stride: buf_stride, format } => {
            // Use cached SHM pool mapping instead of re-mmap
            let pool_size = match pool_size(*pool_fd) {
                Ok(s) => s,
                Err(_) => return,
            };

            // Try to find the pool in the shm manager (already mmap'd)
            let data: &[u8] = if let Some((_, pool)) = shm.pools().find(|(_, p)| p.fd == *pool_fd) {
                match pool.read(*offset as usize, (*buf_stride as usize) * (*buf_h as usize)) {
                    Some(d) => d,
                    None => return,
                }
            } else {
                // Fallback: mmap temporarily if pool not found in manager
                let mapping = unsafe {
                    libc::mmap(std::ptr::null_mut(), pool_size, libc::PROT_READ,
                               libc::MAP_SHARED, *pool_fd, 0)
                };
                if mapping == libc::MAP_FAILED { return; }
                let pool_data = unsafe { std::slice::from_raw_parts(mapping as *const u8, pool_size) };
                let off = *offset as usize;
                let len = (*buf_stride as usize) * (*buf_h as usize);
                if off + len > pool_data.len() {
                    unsafe { libc::munmap(mapping, pool_size) };
                    return;
                }
                // Leak the mapping — we'll unmap below in the scope guard
                let data = &pool_data[off..off + len];
                // For the fallback, we just proceed with the mmap'd data and unmap at the end
                blit_pixels(
                    data, *format, *buf_w as u32, *buf_h as u32, *buf_stride as u32,
                    origin_x, origin_y, output_width, output_height, output_stride, pixels,
                );
                unsafe { libc::munmap(mapping, pool_size) };
                return;
            };

            blit_pixels(
                data, *format, *buf_w as u32, *buf_h as u32, *buf_stride as u32,
                origin_x, origin_y, output_width, output_height, output_stride, pixels,
            );
        }
        SurfaceBuffer::DmaBuf { width: buf_w, height: buf_h, format, plane_fds, offsets, strides, num_planes: _ } => {
            if plane_fds.is_empty() || plane_fds[0] < 0 { return; }
            let fd = plane_fds[0];
            let buf_stride = strides.first().copied().unwrap_or(*buf_w as u32 * 4);
            let offset = offsets.first().copied().unwrap_or(0);

            let pool_size = match pool_size(fd) {
                Ok(s) => s,
                Err(_) => return,
            };
            let mapping = unsafe {
                libc::mmap(std::ptr::null_mut(), pool_size, libc::PROT_READ,
                           libc::MAP_SHARED, fd, 0)
            };
            if mapping == libc::MAP_FAILED { return; }

            let buf_data = unsafe { std::slice::from_raw_parts(mapping as *const u8, pool_size) };
            let off = offset as usize;
            let len = (buf_stride as usize) * (*buf_h as usize);
            if off + len > buf_data.len() {
                unsafe { libc::munmap(mapping, pool_size) };
                return;
            }
            let data = &buf_data[off..off + len];

            blit_pixels(
                data, *format, *buf_w as u32, *buf_h as u32, buf_stride,
                origin_x, origin_y, output_width, output_height, output_stride, pixels,
            );

            unsafe { libc::munmap(mapping, pool_size) };
        }
        SurfaceBuffer::Dumb { .. } => {}
        SurfaceBuffer::Null => {}
    }
}

/// Shared pixel blitting logic — handles format conversion and clipping.
/// Uses integer alpha blending for performance (no f32).
fn blit_pixels(
    src_data: &[u8],
    format: u32,
    buf_w: u32,
    buf_h: u32,
    buf_stride: u32,
    origin_x: i32,
    origin_y: i32,
    output_width: u32,
    output_height: u32,
    output_stride: u32,
    pixels: &mut [u8],
) {
    for sy in 0..buf_h {
        for sx in 0..buf_w {
            let dx = origin_x + sx as i32;
            let dy = origin_y + sy as i32;
            if dx < 0 || dy < 0 || dx as u32 >= output_width || dy as u32 >= output_height {
                continue;
            }
            let src_off = (sy as usize * buf_stride as usize + sx as usize * 4);
            if src_off + 4 > src_data.len() { continue; }
            let dst_off = (dy as u32 * output_stride + dx as u32 * 4) as usize;
            if dst_off + 4 > pixels.len() { continue; }

            // Target framebuffer is XRGB8888 (little-endian memory: [B, G, R, X])
            match format {
                0x34325258 /* XRGB8888 */ => {
                    // Same format as framebuffer — direct copy
                    // Source LE memory: [B, G, R, X], Dest LE memory: [B, G, R, X]
                    pixels[dst_off] = src_data[src_off];
                    pixels[dst_off + 1] = src_data[src_off + 1];
                    pixels[dst_off + 2] = src_data[src_off + 2];
                    pixels[dst_off + 3] = 0xff;
                }
                0x34325241 /* ARGB8888 */ => {
                    // ARGB8888 little-endian: [B, G, R, A]
                    // Same RGB order as framebuffer, needs alpha blending
                    let alpha = src_data[src_off + 3];
                    if alpha == 0 { continue; }
                    if alpha == 255 {
                        pixels[dst_off] = src_data[src_off];
                        pixels[dst_off + 1] = src_data[src_off + 1];
                        pixels[dst_off + 2] = src_data[src_off + 2];
                    } else {
                        // Integer alpha blending: dst = (src * alpha + dst * (255 - alpha)) / 255
                        let a_inv = 255u32 - alpha as u32;
                        pixels[dst_off] =     ((src_data[src_off] as u32 * alpha as u32 + pixels[dst_off] as u32 * a_inv) / 255) as u8;
                        pixels[dst_off + 1] = ((src_data[src_off + 1] as u32 * alpha as u32 + pixels[dst_off + 1] as u32 * a_inv) / 255) as u8;
                        pixels[dst_off + 2] = ((src_data[src_off + 2] as u32 * alpha as u32 + pixels[dst_off + 2] as u32 * a_inv) / 255) as u8;
                    }
                    pixels[dst_off + 3] = 0xff;
                }
                0x34324241 /* ABGR8888 */ => {
                    // ABGR8888 little-endian: [R, G, B, A]
                    // Need to swap R↔B to match XRGB8888 framebuffer [B, G, R, X]
                    let alpha = src_data[src_off + 3];
                    if alpha == 0 { continue; }
                    if alpha == 255 {
                        pixels[dst_off] =     src_data[src_off + 2]; // B <- source B (byte 2)
                        pixels[dst_off + 1] = src_data[src_off + 1]; // G <- source G (byte 1)
                        pixels[dst_off + 2] = src_data[src_off];     // R <- source R (byte 0)
                    } else {
                        let a_inv = 255u32 - alpha as u32;
                        // Source: [R, G, B, A] → we need [B, G, R] for dest
                        let src_b = src_data[src_off + 2] as u32;
                        let src_g = src_data[src_off + 1] as u32;
                        let src_r = src_data[src_off] as u32;
                        pixels[dst_off] =     ((src_b * alpha as u32 + pixels[dst_off] as u32 * a_inv) / 255) as u8;
                        pixels[dst_off + 1] = ((src_g * alpha as u32 + pixels[dst_off + 1] as u32 * a_inv) / 255) as u8;
                        pixels[dst_off + 2] = ((src_r * alpha as u32 + pixels[dst_off + 2] as u32 * a_inv) / 255) as u8;
                    }
                    pixels[dst_off + 3] = 0xff;
                }
                _ => {
                    // Default: treat as XRGB8888, direct copy
                    pixels[dst_off] = src_data[src_off];
                    pixels[dst_off + 1] = src_data[src_off + 1];
                    pixels[dst_off + 2] = src_data[src_off + 2];
                    pixels[dst_off + 3] = 0xff;
                }
            }
        }
    }
}

fn pool_size(fd: i32) -> io::Result<usize> {
    drm::fd_size(fd)
}
