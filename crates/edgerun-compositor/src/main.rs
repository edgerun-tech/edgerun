//! edgerun-compositor — Wayland compositor binary.
//!
//! Usage: `edgerun-compositor [socket_path]`
//!
//! Default socket path: `/tmp/edgerun-wayland-0`

use std::collections::HashMap;
use std::io;
use std::time::Instant;

use edgerun_compositor::compositor::dmabuf::DmabufParams;
use edgerun_compositor::compositor::output::Output;
use edgerun_compositor::compositor::seat::Seat;
use edgerun_compositor::compositor::shell::Shell;
use edgerun_compositor::compositor::surface::{BufferRegistry, DamageRect, SurfaceTree};
use edgerun_compositor::drm;
use edgerun_compositor::drm::device::DrmDevice;
use edgerun_compositor::drm::dumb::DumbBuffer;
use edgerun_compositor::drm::kms;
use edgerun_compositor::input::evdev::EvdevManager;
use edgerun_compositor::input::keymap::{Keymap, Modifiers};
use edgerun_compositor::r#loop::{EventLoop, EventSource};
use edgerun_compositor::vt::{VtEvent, VtManager};
use edgerun_compositor::protocol::wl_core;
use edgerun_compositor::protocol::text_input_v3;
use edgerun_compositor::protocol::input_method_v2;
use edgerun_compositor::protocol::wp_presentation_time;
use edgerun_compositor::protocol::wp_presentation_time::PresentationFeedbackTracker;
use edgerun_compositor::protocol::dispatch::{self, DataSource, process_input_for_device, PointerConstraint, ConstraintType, RegionRegistry};
use edgerun_compositor::render::cursor::Cursor;
use edgerun_compositor::render::shm::ShmManager;
use edgerun_compositor::render::server::{render_and_flip, DamageAccumulator};
use edgerun_compositor::gpu::compositor::GlCompositor;
use edgerun_compositor::resource::Registry;
use edgerun_compositor::server::WaylandServer;
use edgerun_compositor::wire;
// wire imports used in dispatch
// (wire types accessed through dispatch module)

fn main() {
    let socket_path = std::env::args().nth(1).unwrap_or_else(|| "/tmp/edgerun-wayland-0".to_string());

    println!("[edgerun-compositor] Starting...");

    // ─── Open DRM device ──────────────────────────────────────
    let devices = drm::find_card_devices();
    if devices.is_empty() {
        eprintln!("[edgerun-compositor] No DRM devices found!");
        std::process::exit(1);
    }

    let drm_device = match DrmDevice::open(&devices[0]) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[edgerun-compositor] Failed to open DRM device {}: {}", devices[0], e);
            std::process::exit(1);
        }
    };

    println!("[edgerun-compositor] DRM device: {}", devices[0]);

    let _ = drm_device.set_client_cap(drm::ioctl::client_cap::ATOMIC, 1);
    let _ = drm_device.set_client_cap(drm::ioctl::client_cap::UNIVERSAL_PLANES, 1);

    // Acquire DRM master — required for KMS (mode setting, page flips).
    if let Err(e) = drm_device.set_master() {
        eprintln!("[edgerun-compositor] Failed to acquire DRM master: {}", e);
        eprintln!("[edgerun-compositor] Switch to a TTY (Ctrl+Alt+F3) and run again.");
        std::process::exit(1);
    }
    println!("[edgerun-compositor] DRM master acquired");

    // ─── VT management ────────────────────────────────────────
    // Try to set up VT management. If we're not on a VT (e.g., running
    // under an existing Wayland/X11 session), this will fail gracefully
    // and we fall back to direct DRM access.
    let mut vt_manager = match VtManager::open(0) {
        Ok(vt) => {
            println!("[edgerun-compositor] VT{} acquired — running as standalone compositor", vt.vt_num());
            Some(vt)
        }
        Err(e) => {
            eprintln!("[edgerun-compositor] VT management unavailable: {}", e);
            eprintln!("[edgerun-compositor] Running in embedded mode — no VT switching");
            None
        }
    };

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

    let mut fb_id = match dumb.add_fb() {
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
    let mut region_registry = RegionRegistry::new();

    // Create output to compute scale factor
    let output = Output::from_drm(
        0,
        conn.type_name(),
        mode.hdisplay as u32,
        mode.vdisplay as u32,
        mode.vrefresh * 1000,
        conn.mm_width,
        conn.mm_height,
        conn.type_name(),
    );
    let output_scale = output.scale;

    let mut shell = Shell::new(0);
    shell.set_output_size(mode.hdisplay as i32, mode.vdisplay as i32, (mode.vrefresh * 1000) as i32, conn.mm_width as i32, conn.mm_height as i32, output_scale);
    let mut seat_obj = Seat::new(0);
    let mut keymap_obj = Keymap::estonian_nodeadkeys();
    let mut modifiers = Modifiers::default();
    let mut cursor = Cursor::new();

    // Pending dmabuf params per client
    let mut dmabuf_pending: HashMap<u32, DmabufParams> = HashMap::new();

    // Presentation feedback tracking
    let mut presentation_tracker = PresentationFeedbackTracker::new();

    // Clipboard: currently active data source
    let mut current_data_source: Option<DataSource> = None;
    let mut selection_offer_counter: u32 = 0;
    let mut current_primary_selection: Option<dispatch::PrimarySelectionSource> = None;
    let mut primary_selection_offer_counter: u32 = 0;

    // Registry (globals advertised to clients) — single source of truth
    // via dispatch::assign_globals (matches dispatch::GLOBALS).
    let mut global_name: u32 = 1;
    let (global_name_after, globals) = dispatch::assign_globals(global_name);
    global_name = global_name_after;

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

    // ─── Browser launch (disabled - launch clients manually) ───
    // Uncomment to auto-launch:
    // let _browser_child = std::process::Command::new("firefox")
    //     .env("WAYLAND_DISPLAY", &socket_path)
    //     .env("MOZ_ENABLE_WAYLAND", "1")
    //     .spawn();

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
    let mut client_touch_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_dmabuf_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_data_device_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_data_source_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_subcompositor_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_decoration_manager_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_decoration_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_viewporter_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_cursor_shape_manager_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_cursor_shape_device_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_relative_pointer_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_gesture_swipe_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_gesture_pinch_ids: HashMap<u32, u32> = HashMap::new();
    let mut client_tearing_control_ids: HashMap<u32, u32> = HashMap::new();

    // Track which client-side pool id maps to which internal pool
    let mut client_pool_map: HashMap<u32, HashMap<u32, u32>> = HashMap::new();

    // Per-client cursor surface tracking
    let mut client_cursor_surfaces: HashMap<u32, u32> = HashMap::new(); // client_id -> surface_id

    // Map from xdg_surface id to wl_surface id (needed for keyboard/pointer enter)
    let mut xdg_surface_to_wl_surface: HashMap<u32, u32> = HashMap::new();

    let mut config_serial: u32 = 1;
    let mut touch_state = dispatch::TouchState::new();
    let mut screencopy_state = dispatch::ScreencopyState::new();
    let mut pointer_constraints: HashMap<u32, dispatch::PointerConstraint> = HashMap::new();
    let mut constraint_type_map: HashMap<u32, dispatch::ConstraintType> = HashMap::new();
    let mut text_input_state: Option<text_input_v3::TextInputState> = None;
    let mut ime_state: Option<input_method_v2::IMEState> = None;
    let mut text_input_serial: u32 = 1;
    let mut frame_count: u64 = 0;
    let _start_time = Instant::now();
    let mut old_fb_ids: Vec<u32> = Vec::new(); // Track old FB IDs for cleanup

    println!("[edgerun-compositor] Running event loop...");

    // Frame pacing — derived from the detected display mode.
    // Use the actual mode's refresh rate (not hardcoded 60Hz).
    let refresh_hz = mode.vrefresh.max(1) as u64;
    let frame_interval_ms = (1000.0 / refresh_hz as f64).max(1.0) as i32;
    eprintln!("[edgerun-compositor] Display refresh: {refresh_hz}Hz, frame interval: {frame_interval_ms}ms");

    // Reusable buffer for frame callbacks — avoids per-frame allocation.
    let mut frame_callbacks: Vec<(u32, u32)> = Vec::with_capacity(16);

    // Page flip tracking — only render when the previous flip has completed
    // to avoid tearing and respect the display's refresh rate.
    let mut flip_pending = false;
    let mut pending_render = true; // Render immediately on first frame

    // Damage tracking — accumulate surface damage for incremental rendering
    let mut damage = DamageAccumulator::new();

    loop {
        // Wait for events with a timeout matching the display refresh rate.
        // When flip_pending is true, the timeout is our fallback in case
        // the DRM event is delayed. When flip_pending is false and we have
        // a pending render, we use a short timeout to render quickly.
        let wait_ms = if pending_render && !flip_pending {
            1 // Short timeout so we render the first frame quickly
        } else {
            frame_interval_ms
        };

        let events = match event_loop.wait(wait_ms) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Event loop error: {e}");
                break;
            }
        };

        // On timeout with pending render — render and page flip.
        // Frame callbacks are fired AFTER the VBLANK event completes (see DrmDevice handler).
        if events.is_empty() && pending_render && !flip_pending {
            frame_count += 1;
            pending_render = false;
            flip_pending = true;

            // Wait for acquire fences before compositing
            for surface in surfaces.surfaces() {
                shell.syncobj_state.wait_acquire(drm_device.as_raw_fd(), surface.id);
            }

            // Render all surfaces and page flip
            if let Some(gl) = gl_compositor.as_mut() {
                // GPU-accelerated path
                gl.composite(&shm, &surfaces, &shell, &cursor, &mut dumb);
                // Read back to dumb, then page flip
                if let Ok(mapped) = dumb.map() {
                    let pixels = unsafe { std::slice::from_raw_parts(mapped.as_ptr(), mapped.len()) };
                    screencopy_state.update(dumb.width, dumb.height, dumb.pitch, pixels);
                }
                let new_fb_id = match dumb.add_fb() {
                    Ok(id) => id,
                    Err(_) => { flip_pending = false; pending_render = true; continue; }
                };
                if fb_id != 0 {
                    old_fb_ids.push(fb_id);
                }
                let mut flip_flags = crate::drm::ioctl::page_flip::PAGE_FLIP_EVENT;
                if let Some(front) = shell.stack.last() {
                    if let Some(tl) = shell.toplevels.get(front) {
                        if let Some(surface) = surfaces.get(tl.surface_id) {
                            if surface.buffer.is_some() && surface.tearing_hint == 2 {
                                flip_flags |= crate::drm::ioctl::page_flip::PAGE_FLIP_ASYNC;
                            }
                        }
                    }
                }
                let user_data = kms::next_flip_serial();
                let _ = kms::page_flip(drm_device.as_raw_fd(), crtc_id, new_fb_id, flip_flags, user_data);
                fb_id = new_fb_id;
            } else {
                // Software fallback — use damage tracking
                render_and_flip(
                    &mut dumb, fb_id, crtc_id, &drm_device,
                    &surfaces, &shell, &cursor, &shm,
                    &mut damage,
                    &mut old_fb_ids,
                );
            }

            // Collect damage from surface commits for the next frame
            for surface in surfaces.surfaces() {
                if !surface.pending_damage.is_empty() {
                    for rect in &surface.pending_damage {
                        damage.add(DamageRect {
                            x: surface.x + rect.x,
                            y: surface.y + rect.y,
                            width: rect.width,
                            height: rect.height,
                        });
                    }
                }
            }
            damage.add(DamageRect {
                x: cursor.x.saturating_sub(32),
                y: cursor.y.saturating_sub(32),
                width: 64,
                height: 64,
            });

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
                            let registry_id = 2; // First allocatable id after wl_display (1)
                            cr.register(registry_id, "wl_registry", 1, client_id);

                            client_registries.insert(client_id, cr);
                            client_registry_ids.insert(client_id, registry_id);
                            client_pool_map.insert(client_id, HashMap::new());
                            dmabuf_pending.insert(client_id, DmabufParams::new());

                            send_globals_to_client(&mut server, client_id, registry_id, &globals);

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
                            dispatch::process_message(
                                &mut server, client_id, msg,
                                drm_device.as_raw_fd(),
                                &mut surfaces, &mut buffers, &mut shm, &mut shell,
                                &mut seat_obj, &mut keymap_obj, &mut modifiers,
                                &mut cursor,
                                &mut presentation_tracker,
                                &mut region_registry,
                                &mut client_registries, &mut client_registry_ids,
                                &mut client_compositor_ids, &mut client_shm_ids,
                                &mut client_seat_ids, &mut client_xdg_base_ids,
                                &mut client_output_ids, &mut client_keyboard_ids,
                                &mut client_pointer_ids, &mut client_touch_ids,
                                &mut client_dmabuf_ids,
                                &mut client_pool_map,
                                &mut client_data_device_ids, &mut client_data_source_ids,
                                &mut client_subcompositor_ids,
                                &mut client_decoration_manager_ids, &mut client_decoration_ids,
                                &mut client_viewporter_ids,
                                &mut client_cursor_shape_manager_ids, &mut client_cursor_shape_device_ids,
                                &mut client_cursor_surfaces,
                                &mut client_relative_pointer_ids,
                                &mut xdg_surface_to_wl_surface,
                                &mut dmabuf_pending,
                                &mut current_data_source, &mut selection_offer_counter,
                                &mut config_serial,
                                &mut pointer_constraints,
                                &mut constraint_type_map,
                                &mut current_primary_selection,
                                &mut primary_selection_offer_counter,
                                &mut screencopy_state,
                                &mut text_input_state,
                                &mut ime_state,
                                &mut text_input_serial,
                                &mut client_tearing_control_ids,
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
                            client_touch_ids.remove(&client_id);
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
                            client_relative_pointer_ids.remove(&client_id);
                            client_gesture_swipe_ids.remove(&client_id);
                            client_gesture_pinch_ids.remove(&client_id);
                            client_tearing_control_ids.remove(&client_id);
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
                    // DRM page flip completion — the previous frame was displayed.
                    // Send `presented` events to all pending feedback objects with real VBLANK timing.
                    // Also fire frame callbacks NOW (after presentation, per Wayland spec).
                    let _ = drm::kms::read_events(drm_device.as_raw_fd());

                    // Fire frame callbacks AFTER the frame has been presented
                    frame_callbacks.clear();
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
                    for &(_compositor_id, cb_id) in &frame_callbacks {
                        for &cid in client_registry_ids.keys() {
                            if let Some(client) = server.client_mut(cid) {
                                client.send_message(wl_core::callback_done_event(cb_id, serial));
                            }
                        }
                    }

                    // Send `presented` events with real timestamp
                    let (sec_hi, sec_lo, nsec) = presentation_tracker.clock_timestamp();
                    let refresh_ns = (1_000_000_000u64 / refresh_hz) as u32;
                    let seq_hi = presentation_tracker.seq_hi();
                    let seq_lo = presentation_tracker.seq_lo();
                    for (feedback_id, client_id) in presentation_tracker.take_presented() {
                        if let Some(client) = server.client_mut(client_id) {
                            client.send_message(wp_presentation_time::feedback_presented_event(
                                feedback_id, sec_hi, sec_lo, nsec, refresh_ns, seq_hi, seq_lo,
                                wp_presentation_time::presentation_kind::VSYNC
                                    | wp_presentation_time::presentation_kind::HW_COMPLETION,
                            ));
                        }
                    }

                    flip_pending = false;
                    pending_render = true;

                    // Signal release fences for all surfaces after page flip completes
                    for surface in surfaces.surfaces() {
                        shell.syncobj_state.signal_release(drm_device.as_raw_fd(), surface.id);
                    }
                }

                EventSource::EvdevDevice(dev_id) => {
                    // Input event available — process it immediately (event-driven, no polling)
                    if evmask & (libc::EPOLLIN as u32) != 0 {
                        process_input_for_device(
                            &mut input_mgr, dev_id, &mut seat_obj, &mut keymap_obj,
                            &mut modifiers, &mut server, &surfaces, &mut shell,
                            &client_keyboard_ids, &client_pointer_ids,
                            &client_touch_ids,
                            &client_relative_pointer_ids,
                            &client_gesture_swipe_ids,
                            &client_gesture_pinch_ids,
                            &client_compositor_ids, &mut cursor,
                            &mut touch_state,
                            &mut pointer_constraints,
                            &mut constraint_type_map,
                        );
                    }
                }
            }
        }

        // ─── VT event handling ──────────────────────────────────
        if let Some(ref mut vt) = vt_manager {
            vt.poll_events(&mut |event| {
                match event {
                    VtEvent::Release => {
                        // Pause rendering, release DRM master
                        eprintln!("[vt] VT release — pausing compositor");
                        flip_pending = false;
                        pending_render = false;
                        // In a full implementation, we'd drop DRM master here
                        // and release the framebuffer.
                    }
                    VtEvent::Acquire => {
                        // Resume rendering, re-acquire DRM master
                        eprintln!("[vt] VT re-acquired — resuming compositor");
                        pending_render = true;
                        // In a full implementation, we'd re-set DRM master
                        // and restore the framebuffer.
                    }
                }
            });
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
    globals: &[dispatch::GlobalDescriptor],
) {
    for g in globals {
        let mut args = Vec::new();
        args.extend_from_slice(&g.global_name.to_le_bytes());
        crate::wire::encode::encode_string(&mut args, g.interface);
        args.extend_from_slice(&g.version.to_le_bytes());

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

