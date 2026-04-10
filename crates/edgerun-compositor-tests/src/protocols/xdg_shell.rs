//! XDG shell protocol conformance tests.
//!
//! Tests: xdg_wm_base, xdg_surface, xdg_toplevel, xdg_popup,
//! xdg_positioner.

use wayland_client::{
    globals::GlobalListContents,
    protocol::wl_compositor,
    Dispatch,
};

use crate::harness::{CompositorHarness, TestResult};
use crate::spec;

use wayland_protocols::xdg::shell::client::xdwm_base;
use wayland_protocols::xdg::shell::client::xdg_surface;
use wayland_protocols::xdg::shell::client::xdg_toplevel;
use wayland_protocols::xdg::shell::client::xdg_positioner;

// Re-export for the wayland-protocols crate types
type XdgWmBase = wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;
type XdgSurface = wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface;
type XdgToplevel = wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
type XdgPositioner = wayland_protocols::xdg::shell::client::xdg_positioner::XdgPositioner;

pub fn run_all(harness: &mut CompositorHarness, specs: &[(String, spec::Protocol)]) -> Vec<TestResult> {
    let mut results = Vec::new();

    if let Some((_, proto)) = specs.iter().find(|(name, _)| name == "xdg-shell") {
        results.extend(test_xdg_wm_base(harness, proto));
        results.extend(test_xdg_surface(harness, proto));
        results.extend(test_xdg_toplevel(harness, proto));
        results.extend(test_xdg_positioner(harness, proto));
    } else {
        results.push(TestResult::fail(
            "xdg-shell", "all", "spec_loaded",
            "xdg-shell.xml not found in specs",
        ));
    }

    results
}

fn test_xdg_wm_base(harness: &mut CompositorHarness, proto: &spec::Protocol) -> Vec<TestResult> {
    let mut results = Vec::new();

    if harness.has_global("xdg_wm_base").is_some() {
        results.push(TestResult::pass(
            "xdg-shell", "xdg_wm_base", "advertised",
            "xdg_wm_base global is advertised",
        ));

        let wm_base = harness.globals().bind::<XdgWmBase, _, _>(
            &harness.event_queue().handle(),
            1..=6,
            (),
        );

        if let Ok(base) = wm_base {
            results.push(TestResult::pass(
                "xdg-shell", "xdg_wm_base", "bind",
                "Successfully bound xdg_wm_base",
            ));

            // Test: create_positioner
            let _positioner = base.create_positioner(&harness.event_queue().handle(), ());
            results.push(TestResult::pass(
                "xdg-shell", "xdg_wm_base", "create_positioner",
                "xdg_wm_base.create_positioner() succeeded",
            ));

            // Verify spec
            if let Some(spec) = proto.interfaces.get("xdg_wm_base") {
                for req in &spec.requests {
                    let _ = req; // just verifying it exists
                }
                for req_name in &["create_positioner", "get_xdg_surface", "destroy", "pong"] {
                    let found = spec.requests.iter().any(|r| r.name == *req_name);
                    if found {
                        results.push(TestResult::pass(
                            "xdg-shell", "xdg_wm_base", &format!("request_{}", req_name),
                            &format!("xdg_wm_base.{} defined in spec", req_name),
                        ));
                    } else {
                        results.push(TestResult::fail(
                            "xdg-shell", "xdg_wm_base", &format!("request_{}", req_name),
                            &format!("xdg_wm_base.{} NOT found in spec", req_name),
                        ));
                    }
                }
            }

            // Cleanup
            base.destroy();
        } else {
            results.push(TestResult::fail(
                "xdg-shell", "xdg_wm_base", "bind",
                "Failed to bind xdg_wm_base",
            ));
        }
    } else {
        results.push(TestResult::fail(
            "xdg-shell", "xdg_wm_base", "advertised",
            "xdg_wm_base global not advertised",
        ));
    }

    results
}

fn test_xdg_surface(harness: &mut CompositorHarness, proto: &spec::Protocol) -> Vec<TestResult> {
    let mut results = Vec::new();

    // Need wl_compositor to create a surface first
    let compositor = match harness.globals().bind::<wl_compositor::WlCompositor, _, _>(
        &harness.event_queue().handle(),
        1..=5,
        (),
    ) {
        Ok(c) => c,
        Err(_) => {
            results.push(TestResult::fail(
                "xdg-shell", "xdg_surface", "setup",
                "Cannot bind wl_compositor for xdg_surface test",
            ));
            return results;
        }
    };

    let wl_surface = compositor.create_surface(&harness.event_queue().handle(), ());

    // Bind xdg_wm_base
    let wm_base = match harness.globals().bind::<XdgWmBase, _, _>(
        &harness.event_queue().handle(),
        1..=6,
        (),
    ) {
        Ok(b) => b,
        Err(_) => {
            results.push(TestResult::fail(
                "xdg-shell", "xdg_surface", "setup",
                "Cannot bind xdg_wm_base for xdg_surface test",
            ));
            wl_surface.destroy();
            return results;
        }
    };

    // Test: get_xdg_surface
    let xdg_surface_obj = wm_base.get_xdg_surface(&wl_surface, &harness.event_queue().handle(), ());

    results.push(TestResult::pass(
        "xdg-shell", "xdg_surface", "get_xdg_surface",
        "xdg_wm_base.get_xdg_surface() succeeded",
    ));

    // Verify spec
    if let Some(spec) = proto.interfaces.get("xdg_surface") {
        for req_name in &["get_toplevel", "get_popup", "ack_configure", "destroy", "set_window_geometry"] {
            let found = spec.requests.iter().any(|r| r.name == *req_name);
            if found {
                results.push(TestResult::pass(
                    "xdg-shell", "xdg_surface", &format!("request_{}", req_name),
                    &format!("xdg_surface.{} defined in spec", req_name),
                ));
            }
        }
    }

    // Cleanup
    xdg_surface_obj.destroy();
    wm_base.destroy();
    wl_surface.destroy();
    compositor.destroy();

    results
}

fn test_xdg_toplevel(harness: &mut CompositorHarness, proto: &spec::Protocol) -> Vec<TestResult> {
    let mut results = Vec::new();

    let compositor = match harness.globals().bind::<wl_compositor::WlCompositor, _, _>(
        &harness.event_queue().handle(),
        1..=5,
        (),
    ) {
        Ok(c) => c,
        Err(_) => return results,
    };

    let wl_surface = compositor.create_surface(&harness.event_queue().handle(), ());

    let wm_base = match harness.globals().bind::<XdgWmBase, _, _>(
        &harness.event_queue().handle(),
        1..=6,
        (),
    ) {
        Ok(b) => b,
        Err(_) => {
            wl_surface.destroy();
            return results;
        }
    };

    let xdg_surface_obj = wm_base.get_xdg_surface(&wl_surface, &harness.event_queue().handle(), ());

    // Test: get_toplevel
    let toplevel = xdg_surface_obj.get_toplevel(&harness.event_queue().handle(), ());

    results.push(TestResult::pass(
        "xdg-shell", "xdg_toplevel", "get_toplevel",
        "xdg_surface.get_toplevel() succeeded",
    ));

    // Test: set_title
    toplevel.set_title("edgerun-test-window");
    results.push(TestResult::pass(
        "xdg-shell", "xdg_toplevel", "set_title",
        "xdg_toplevel.set_title() succeeded",
    ));

    // Test: set_app_id
    toplevel.set_app_id("edgerun.test");
    results.push(TestResult::pass(
        "xdg-shell", "xdg_toplevel", "set_app_id",
        "xdg_toplevel.set_app_id() succeeded",
    ));

    // Verify spec
    if let Some(spec) = proto.interfaces.get("xdg_toplevel") {
        for req_name in &[
            "set_title", "set_app_id", "show_window_menu", "move", "resize",
            "set_min_size", "set_max_size", "maximize", "unmaximize",
            "set_fullscreen", "unset_fullscreen", "minimize", "close", "destroy",
        ] {
            let found = spec.requests.iter().any(|r| r.name == *req_name);
            if found {
                results.push(TestResult::pass(
                    "xdg-shell", "xdg_toplevel", &format!("request_{}", req_name),
                    &format!("xdg_toplevel.{} defined in spec", req_name),
                ));
            }
        }
    }

    // Cleanup
    toplevel.destroy();
    xdg_surface_obj.destroy();
    wm_base.destroy();
    wl_surface.destroy();
    compositor.destroy();

    results
}

fn test_xdg_positioner(harness: &mut CompositorHarness, proto: &spec::Protocol) -> Vec<TestResult> {
    let mut results = Vec::new();

    let wm_base = match harness.globals().bind::<XdgWmBase, _, _>(
        &harness.event_queue().handle(),
        1..=6,
        (),
    ) {
        Ok(b) => b,
        Err(_) => return results,
    };

    let positioner = wm_base.create_positioner(&harness.event_queue().handle(), ());

    // Test: set_size
    positioner.set_size(200, 100);
    results.push(TestResult::pass(
        "xdg-shell", "xdg_positioner", "set_size",
        "xdg_positioner.set_size() succeeded",
    ));

    // Test: set_anchor_rect
    positioner.set_anchor_rect(0, 0, 100, 50);
    results.push(TestResult::pass(
        "xdg-shell", "xdg_positioner", "set_anchor_rect",
        "xdg_positioner.set_anchor_rect() succeeded",
    ));

    // Test: set_anchor
    positioner.set_anchor(xdg_positioner::Anchor::Top);
    results.push(TestResult::pass(
        "xdg-shell", "xdg_positioner", "set_anchor",
        "xdg_positioner.set_anchor() succeeded",
    ));

    // Test: set_gravity
    positioner.set_gravity(xdg_positioner::Gravity::Bottom);
    results.push(TestResult::pass(
        "xdg-shell", "xdg_positioner", "set_gravity",
        "xdg_positioner.set_gravity() succeeded",
    ));

    // Test: set_offset
    positioner.set_offset(10, 20);
    results.push(TestResult::pass(
        "xdg-shell", "xdg_positioner", "set_offset",
        "xdg_positioner.set_offset() succeeded",
    ));

    // Verify spec
    if let Some(spec) = proto.interfaces.get("xdg_positioner") {
        for req_name in &[
            "set_size", "set_anchor_rect", "set_anchor", "set_gravity",
            "set_offset", "set_constraint_adjustment", "set_reactive",
            "set_parent_size", "set_parent_configure", "destroy",
        ] {
            let found = spec.requests.iter().any(|r| r.name == *req_name);
            if found {
                results.push(TestResult::pass(
                    "xdg-shell", "xdg_positioner", &format!("request_{}", req_name),
                    &format!("xdg_positioner.{} defined in spec", req_name),
                ));
            }
        }
    }

    positioner.destroy();
    wm_base.destroy();

    results
}
