//! Integration tests for edgerun-compositor protocol conformance.
//!
//! These are proper `#[test]` functions that can be run via `cargo test`.

// ─── Core Protocol Tests ─────────────────────────────────────

#[test]
fn display_sync_done() {
    let result = edgerun_compositor_tests::protocols::core::test_display_sync_done();
    assert!(result.is_ok(), "display_sync_done: {:?}", result.err());
}

#[test]
fn raw_sync_response() {
    let result = edgerun_compositor_tests::protocols::core::test_raw_sync_response();
    assert!(result.is_ok(), "raw_sync_response: {:?}", result.err());
}

#[test]
fn registry_globals() {
    let result = edgerun_compositor_tests::protocols::core::test_registry_globals();
    assert!(result.is_ok(), "registry_globals: {:?}", result.err());
}

#[test]
fn shm_version() {
    let result = edgerun_compositor_tests::protocols::core::test_shm_version();
    assert!(result.is_ok(), "shm_version: {:?}", result.err());
}

#[test]
fn xdg_version() {
    let result = edgerun_compositor_tests::protocols::core::test_xdg_version();
    assert!(result.is_ok(), "xdg_version: {:?}", result.err());
}

#[test]
fn compositor_version() {
    let result = edgerun_compositor_tests::protocols::core::test_compositor_version();
    assert!(result.is_ok(), "compositor_version: {:?}", result.err());
}

#[test]
fn create_surface() {
    let result = edgerun_compositor_tests::protocols::core::test_create_surface();
    assert!(result.is_ok(), "create_surface: {:?}", result.err());
}

#[test]
fn shm_buffer_lifecycle() {
    let result = edgerun_compositor_tests::protocols::core::test_shm_buffer_lifecycle();
    assert!(result.is_ok(), "shm_buffer_lifecycle: {:?}", result.err());
}

#[test]
fn frame_callback() {
    let result = edgerun_compositor_tests::protocols::core::test_frame_callback();
    assert!(result.is_ok(), "frame_callback: {:?}", result.err());
}

#[test]
fn seat_keyboard() {
    let result = edgerun_compositor_tests::protocols::core::test_seat_keyboard();
    assert!(result.is_ok(), "seat_keyboard: {:?}", result.err());
}

#[test]
fn seat_pointer() {
    let result = edgerun_compositor_tests::protocols::core::test_seat_pointer();
    assert!(result.is_ok(), "seat_pointer: {:?}", result.err());
}

#[test]
fn invalid_object_request() {
    let result = edgerun_compositor_tests::protocols::core::test_invalid_object_request();
    assert!(result.is_ok(), "invalid_object_request: {:?}", result.err());
}

#[test]
fn invalid_opcode() {
    let result = edgerun_compositor_tests::protocols::core::test_invalid_opcode();
    assert!(result.is_ok(), "invalid_opcode: {:?}", result.err());
}

#[test]
fn multiple_clients() {
    let result = edgerun_compositor_tests::protocols::core::test_multiple_clients();
    assert!(result.is_ok(), "multiple_clients: {:?}", result.err());
}

// ─── XDG Shell Tests ─────────────────────────────────────────

#[test]
fn xdg_toplevel_lifecycle() {
    let result = edgerun_compositor_tests::protocols::xdg_shell::test_xdg_toplevel_lifecycle();
    assert!(result.is_ok(), "xdg_toplevel_lifecycle: {:?}", result.err());
}

#[test]
fn xdg_ping_pong() {
    let result = edgerun_compositor_tests::protocols::xdg_shell::test_xdg_ping_pong();
    assert!(result.is_ok(), "xdg_ping_pong: {:?}", result.err());
}

#[test]
fn wm_capabilities() {
    let result = edgerun_compositor_tests::protocols::xdg_shell::test_wm_capabilities();
    assert!(result.is_ok(), "wm_capabilities: {:?}", result.err());
}

#[test]
fn toplevel_maximize() {
    let result = edgerun_compositor_tests::protocols::xdg_shell::test_toplevel_set_maximized();
    assert!(result.is_ok(), "toplevel_maximize: {:?}", result.err());
}

#[test]
fn toplevel_minimize() {
    let result = edgerun_compositor_tests::protocols::xdg_shell::test_toplevel_minimize();
    assert!(result.is_ok(), "toplevel_minimize: {:?}", result.err());
}

// ─── Extended Protocol Tests ─────────────────────────────────

#[test]
fn seat_touch() {
    let result = edgerun_compositor_tests::protocols::extended::test_seat_touch();
    assert!(result.is_ok(), "seat_touch: {:?}", result.err());
}

#[test]
fn pointer_lock() {
    let result = edgerun_compositor_tests::protocols::extended::test_pointer_lock();
    assert!(result.is_ok(), "pointer_lock: {:?}", result.err());
}

#[test]
fn pointer_confine() {
    let result = edgerun_compositor_tests::protocols::extended::test_pointer_confine();
    assert!(result.is_ok(), "pointer_confine: {:?}", result.err());
}

#[test]
fn idle_inhibit() {
    let result = edgerun_compositor_tests::protocols::extended::test_idle_inhibit();
    assert!(result.is_ok(), "idle_inhibit: {:?}", result.err());
}

#[test]
fn primary_selection() {
    let result = edgerun_compositor_tests::protocols::extended::test_primary_selection();
    assert!(result.is_ok(), "primary_selection: {:?}", result.err());
}

#[test]
fn screencopy_capture() {
    let result = edgerun_compositor_tests::protocols::extended::test_screencopy_capture();
    assert!(result.is_ok(), "screencopy_capture: {:?}", result.err());
}

#[test]
fn fractional_scale() {
    let result = edgerun_compositor_tests::protocols::extended::test_fractional_scale();
    assert!(result.is_ok(), "fractional_scale: {:?}", result.err());
}

#[test]
fn tearing_control() {
    let result = edgerun_compositor_tests::protocols::extended::test_tearing_control();
    assert!(result.is_ok(), "tearing_control: {:?}", result.err());
}

#[test]
fn single_pixel_buffer() {
    let result = edgerun_compositor_tests::protocols::extended::test_single_pixel_buffer();
    assert!(result.is_ok(), "single_pixel_buffer: {:?}", result.err());
}
