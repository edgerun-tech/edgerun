//! Pure Rust Wayland compositor.
//!
//! No external C libraries — direct Linux kernel via ioctls and raw sockets.

// Protocol and FFI modules generate many doc/name warnings — suppress them
#![allow(missing_docs)]
#![allow(non_snake_case)]
#![allow(unused_comparisons)]

pub mod client;
pub mod compositor;
pub mod drm;
pub mod font;
pub mod gpu;
pub mod input;
pub mod libc;
pub mod logind;
pub mod r#loop;
pub mod protocol;
pub mod render;
pub mod resource;
pub mod server;
pub mod vt;
pub mod wire;

pub struct CompositorCapabilityReport {
    pub globals: usize,
    pub wl_compositor_version: u32,
    pub registry_probe_ok: bool,
    pub wire_probe_bytes: usize,
}

pub fn capability_report() -> CompositorCapabilityReport {
    let (_, globals) = protocol::dispatch::assign_globals(1);
    use edgerun_protocols::wayland::wl_compositor;

    let mut registry = resource::Registry::new();
    registry.register(
        2,
        wl_compositor::WL_COMPOSITOR,
        wl_compositor::WL_COMPOSITOR_VERSION,
        1,
    );
    let registry_probe_ok = registry.interface(2) == Some(wl_compositor::WL_COMPOSITOR);

    let wire_probe = wl_compositor::surface_enter_event(3, 4);

    CompositorCapabilityReport {
        globals: globals.len(),
        wl_compositor_version: wl_compositor::WL_COMPOSITOR_VERSION,
        registry_probe_ok,
        wire_probe_bytes: usize::from(wire_probe.size),
    }
}
