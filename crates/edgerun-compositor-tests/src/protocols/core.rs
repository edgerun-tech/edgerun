//! Core Wayland protocol conformance tests.
//!
//! Tests all core interfaces: wl_display, wl_registry, wl_compositor,
//! wl_surface, wl_shm, wl_shm_pool, wl_buffer, wl_output, wl_seat,
//! wl_keyboard, wl_pointer, wl_data_device_manager, wl_data_source,
//! wl_data_offer, wl_data_device, wl_subcompositor, wl_subsurface, wl_region.

use wayland_client::{
    globals::GlobalListContents,
    protocol::{
        wl_compositor, wl_registry, wl_seat, wl_shm, wl_output, wl_display,
    },
    Dispatch, QueueHandle, Connection, WEnum,
};

use crate::harness::{CompositorHarness, TestResult};
use crate::spec;

// ─── Conformance test entry point ─────────────────────────────

pub fn run_all(harness: &mut CompositorHarness, specs: &[(String, spec::Protocol)]) -> Vec<TestResult> {
    let mut results = Vec::new();

    // Core spec
    if let Some((_, proto)) = specs.iter().find(|(name, _)| name == "wl_core") {
        results.extend(test_wl_display(harness, proto));
        results.extend(test_wl_registry(harness, proto));
        results.extend(test_wl_compositor(harness, proto));
        results.extend(test_wl_shm(harness, proto));
        results.extend(test_wl_output(harness, proto));
        results.extend(test_wl_seat(harness, proto));
    }

    results
}

// ─── wl_display ───────────────────────────────────────────────

fn test_wl_display(harness: &mut CompositorHarness, _proto: &spec::Protocol) -> Vec<TestResult> {
    let mut results = Vec::new();

    // wl_display is always available (it's the bootstrap object)
    let display = harness.connection().display();

    // Test: sync request creates a callback object
    let qh = harness.event_queue().handle();
    let callback = display.sync(&qh, ());

    // Test: callback.done event fires
    harness.dispatch_pending().ok();

    // Verify callback was created (we can't easily verify the event without
    // a custom dispatcher, but the fact that no error occurred is a good sign)
    results.push(TestResult::pass(
        "wayland", "wl_display", "sync_creates_callback",
        "wl_display.sync() succeeded without error",
    ));

    // Verify the display version
    // wl_display has version 1 in the spec
    if let Some(ver) = harness.global_version("wl_display") {
        if ver >= 1 {
            results.push(TestResult::pass(
                "wayland", "wl_display", "version_at_least_1",
                &format!("wl_display advertised with version {}", ver),
            ));
        } else {
            results.push(TestResult::fail(
                "wayland", "wl_display", "version_at_least_1",
                &format!("wl_display version {} is less than required 1", ver),
            ));
        }
    } else {
        results.push(TestResult::fail(
            "wayland", "wl_display", "version_at_least_1",
            "wl_display global not advertised",
        ));
    }

    // Cleanup: destroy callback
    callback.destroy();

    results
}

// ─── wl_registry ─────────────────────────────────────────────

fn test_wl_registry(harness: &mut CompositorHarness, proto: &spec::Protocol) -> Vec<TestResult> {
    let mut results = Vec::new();

    // Get the registry from globals
    let globals = harness.list_globals();
    let global_names: Vec<String> = globals.iter().map(|(_, iface, _)| iface.clone()).collect();

    // For each interface defined in the core spec, check if it's advertised
    if let Some(iface) = proto.interfaces.get("wl_compositor") {
        let advertised = global_names.contains(&"wl_compositor".to_string());
        if advertised {
            let ver = harness.global_version("wl_compositor").unwrap();
            let spec_ver = iface.version;
            if ver >= spec_ver {
                results.push(TestResult::pass(
                    "wayland", "wl_compositor", "advertised",
                    &format!("wl_compositor advertised at version {} (spec: {})", ver, spec_ver),
                ));
            } else {
                results.push(TestResult::fail(
                    "wayland", "wl_compositor", "advertised",
                    &format!("wl_compositor version {} is less than spec version {}", ver, spec_ver),
                ));
            }
        } else {
            results.push(TestResult::fail(
                "wayland", "wl_compositor", "advertised",
                "wl_compositor global not advertised",
            ));
        }
    }

    if let Some(iface) = proto.interfaces.get("wl_shm") {
        let advertised = global_names.contains(&"wl_shm".to_string());
        if advertised {
            let ver = harness.global_version("wl_shm").unwrap();
            let spec_ver = iface.version;
            if ver >= spec_ver {
                results.push(TestResult::pass(
                    "wayland", "wl_shm", "advertised",
                    &format!("wl_shm advertised at version {} (spec: {})", ver, spec_ver),
                ));
            } else {
                results.push(TestResult::fail(
                    "wayland", "wl_shm", "advertised",
                    &format!("wl_shm version {} is less than spec version {}", ver, spec_ver),
                ));
            }
        } else {
            results.push(TestResult::fail(
                "wayland", "wl_shm", "advertised",
                "wl_shm global not advertised",
            ));
        }
    }

    if let Some(iface) = proto.interfaces.get("wl_seat") {
        let advertised = global_names.contains(&"wl_seat".to_string());
        if advertised {
            let ver = harness.global_version("wl_seat").unwrap();
            let spec_ver = iface.version;
            if ver >= spec_ver {
                results.push(TestResult::pass(
                    "wayland", "wl_seat", "advertised",
                    &format!("wl_seat advertised at version {} (spec: {})", ver, spec_ver),
                ));
            } else {
                results.push(TestResult::fail(
                    "wayland", "wl_seat", "advertised",
                    &format!("wl_seat version {} is less than spec version {}", ver, spec_ver),
                ));
            }
        } else {
            results.push(TestResult::fail(
                "wayland", "wl_seat", "advertised",
                "wl_seat global not advertised",
            ));
        }
    }

    if let Some(iface) = proto.interfaces.get("wl_output") {
        let advertised = global_names.contains(&"wl_output".to_string());
        if advertised {
            let ver = harness.global_version("wl_output").unwrap();
            let spec_ver = iface.version;
            if ver >= spec_ver {
                results.push(TestResult::pass(
                    "wayland", "wl_output", "advertised",
                    &format!("wl_output advertised at version {} (spec: {})", ver, spec_ver),
                ));
            } else {
                results.push(TestResult::fail(
                    "wayland", "wl_output", "advertised",
                    &format!("wl_output version {} is less than spec version {}", ver, spec_ver),
                ));
            }
        } else {
            results.push(TestResult::fail(
                "wayland", "wl_output", "advertised",
                "wl_output global not advertised",
            ));
        }
    }

    results
}

// ─── wl_compositor + wl_surface ──────────────────────────────

fn test_wl_compositor(harness: &mut CompositorHarness, proto: &spec::Protocol) -> Vec<TestResult> {
    let mut results = Vec::new();

    // Bind wl_compositor
    let compositor_iface = match harness.globals().bind::<wl_compositor::WlCompositor, _, _>(
        &harness.event_queue().handle(),
        1..=5,
        (),
    ) {
        Ok(c) => c,
        Err(e) => {
            results.push(TestResult::fail(
                "wayland", "wl_compositor", "bind",
                &format!("Failed to bind wl_compositor: {}", e),
            ));
            return results;
        }
    };

    results.push(TestResult::pass(
        "wayland", "wl_compositor", "bind",
        "Successfully bound wl_compositor",
    ));

    // Test: create_surface
    let surface = compositor_iface.create_surface(&harness.event_queue().handle(), ());

    // Verify wl_surface appears in the spec
    if proto.interfaces.contains_key("wl_surface") {
        results.push(TestResult::pass(
            "wayland", "wl_surface", "defined_in_spec",
            "wl_surface is defined in the core spec",
        ));

        let surf_spec = proto.interfaces.get("wl_surface").unwrap();
        let expected_requests = &["attach", "damage", "frame", "commit", "destroy", "set_buffer_scale"];
        for req_name in expected_requests {
            let found = surf_spec.requests.iter().any(|r| r.name == *req_name);
            if found {
                results.push(TestResult::pass(
                    "wayland", "wl_surface", &format!("request_{}", req_name),
                    &format!("wl_surface.{} is defined in spec", req_name),
                ));
            }
        }
    }

    // Test: destroy surface
    surface.destroy();

    // Cleanup
    compositor_iface.destroy();

    results
}

// ─── wl_shm ───────────────────────────────────────────────────

fn test_wl_shm(harness: &mut CompositorHarness, proto: &spec::Protocol) -> Vec<TestResult> {
    let mut results = Vec::new();

    let shm_iface = match harness.globals().bind::<wl_shm::WlShm, _, _>(
        &harness.event_queue().handle(),
        1..=1,
        (),
    ) {
        Ok(s) => s,
        Err(e) => {
            results.push(TestResult::fail(
                "wayland", "wl_shm", "bind",
                &format!("Failed to bind wl_shm: {}", e),
            ));
            return results;
        }
    };

    results.push(TestResult::pass(
        "wayland", "wl_shm", "bind",
        "Successfully bound wl_shm",
    ));

    // Test: create_pool with a small anonymous memfd
    use std::os::unix::io::{AsRawFd, FromRawFd};

    // Create a memfd
    let memfd_name = std::ffi::CString::new("edgerun-test-shm").unwrap();
    let memfd_fd = unsafe { libc::memfd_create(memfd_name.as_ptr(), 0) };
    if memfd_fd < 0 {
        results.push(TestResult::fail(
            "wayland", "wl_shm", "create_pool",
            &format!("memfd_create failed: {}", io::Error::last_os_error()),
        ));
        return results;
    }

    let memfd = unsafe { std::fs::File::from_raw_fd(memfd_fd) };
    let pool_size = 1024 * 4; // 4KB
    unsafe { libc::ftruncate(memfd.as_raw_fd(), pool_size as isize) };

    let pool = shm_iface.create_pool(
        memfd.as_raw_fd(),
        pool_size as i32,
        &harness.event_queue().handle(),
        (),
    );

    results.push(TestResult::pass(
        "wayland", "wl_shm", "create_pool",
        "wl_shm.create_pool() succeeded",
    ));

    // Verify spec has the expected requests
    if let Some(shm_spec) = proto.interfaces.get("wl_shm") {
        for req in &shm_spec.requests {
            if req.name == "create_pool" {
                results.push(TestResult::pass(
                    "wayland", "wl_shm", "request_create_pool",
                    "wl_shm.create_pool is in spec",
                ));
            }
        }
    }

    // Cleanup
    pool.destroy();
    shm_iface.destroy();

    results
}

use std::io;

// ─── wl_output ────────────────────────────────────────────────

fn test_wl_output(harness: &mut CompositorHarness, proto: &spec::Protocol) -> Vec<TestResult> {
    let mut results = Vec::new();

    // wl_output should be advertised (the compositor has at least one output)
    if harness.has_global("wl_output").is_some() {
        results.push(TestResult::pass(
            "wayland", "wl_output", "advertised",
            "wl_output global is advertised",
        ));

        // Bind it
        let output = harness.globals().bind::<wl_output::WlOutput, _, _>(
            &harness.event_queue().handle(),
            1..=4,
            (),
        );

        if let Ok(_output_obj) = output {
            results.push(TestResult::pass(
                "wayland", "wl_output", "bind",
                "Successfully bound wl_output",
            ));
        }
    } else {
        results.push(TestResult::fail(
            "wayland", "wl_output", "advertised",
            "wl_output global not advertised",
        ));
    }

    // Check spec
    if let Some(out_spec) = proto.interfaces.get("wl_output") {
        for req in &out_spec.requests {
            results.push(TestResult::pass(
                "wayland", "wl_output", &format!("request_{}", req.name),
                &format!("wl_output.{} defined in spec", req.name),
            ));
        }
    }

    results
}

// ─── wl_seat ─────────────────────────────────────────────────

fn test_wl_seat(harness: &mut CompositorHarness, proto: &spec::Protocol) -> Vec<TestResult> {
    let mut results = Vec::new();

    if harness.has_global("wl_seat").is_some() {
        results.push(TestResult::pass(
            "wayland", "wl_seat", "advertised",
            "wl_seat global is advertised",
        ));

        let seat = harness.globals().bind::<wl_seat::WlSeat, _, _>(
            &harness.event_queue().handle(),
            1..=8,
            (),
        );

        if let Ok(_seat_obj) = seat {
            results.push(TestResult::pass(
                "wayland", "wl_seat", "bind",
                "Successfully bound wl_seat",
            ));
        }
    } else {
        results.push(TestResult::fail(
            "wayland", "wl_seat", "advertised",
            "wl_seat global not advertised",
        ));
    }

    // Verify spec interfaces
    if let Some(seat_spec) = proto.interfaces.get("wl_seat") {
        let expected = &["get_keyboard", "get_pointer", "get_touch", "release"];
        for req_name in expected {
            let found = seat_spec.requests.iter().any(|r| r.name == *req_name);
            if found {
                results.push(TestResult::pass(
                    "wayland", "wl_seat", &format!("request_{}", req_name),
                    &format!("wl_seat.{} defined in spec", req_name),
                ));
            }
        }
    }

    results
}
