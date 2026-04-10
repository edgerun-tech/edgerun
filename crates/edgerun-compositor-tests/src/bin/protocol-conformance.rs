//! CLI binary for running conformance tests with visual overlay output.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

const OVERLAY_FILE: &str = "/tmp/edgerun-test-results.txt";

fn clear_overlay() {
    let _ = std::fs::write(OVERLAY_FILE, "");
}

fn write_result(line: &str) {
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(OVERLAY_FILE) {
        let _ = writeln!(f, "{}", line);
    }
}

fn run_tests() -> Vec<(String, bool, String)> {
    let mut results = Vec::new();

    clear_overlay();
    write_result("[conformance] Starting runtime conformance tests...");
    write_result("---");

    // Core protocol tests
    write_result("[conformance] Core protocol tests...");

    macro_rules! run_test {
        ($name:expr, $test_fn:expr) => {{
            write_result(&format!("[test] Running {}...", $name));
            match $test_fn {
                Ok(()) => {
                    write_result(&format!("[PASS] {}", $name));
                    results.push(($name.into(), true, String::new()));
                }
                Err(msg) => {
                    write_result(&format!("[FAIL] {}: {}", $name, msg));
                    results.push(($name.into(), false, msg));
                }
            }
        }};
    }

    // Core protocol tests
    run_test!("display_sync_done", edgerun_compositor_tests::protocols::core::test_display_sync_done());
    run_test!("raw_sync_response", edgerun_compositor_tests::protocols::core::test_raw_sync_response());
    run_test!("registry_globals", edgerun_compositor_tests::protocols::core::test_registry_globals());
    run_test!("shm_version", edgerun_compositor_tests::protocols::core::test_shm_version());
    run_test!("xdg_version", edgerun_compositor_tests::protocols::core::test_xdg_version());
    run_test!("compositor_version", edgerun_compositor_tests::protocols::core::test_compositor_version());
    run_test!("create_surface", edgerun_compositor_tests::protocols::core::test_create_surface());
    run_test!("shm_buffer_lifecycle", edgerun_compositor_tests::protocols::core::test_shm_buffer_lifecycle());
    run_test!("frame_callback", edgerun_compositor_tests::protocols::core::test_frame_callback());
    run_test!("seat_keyboard", edgerun_compositor_tests::protocols::core::test_seat_keyboard());
    run_test!("seat_pointer", edgerun_compositor_tests::protocols::core::test_seat_pointer());
    run_test!("invalid_object_request", edgerun_compositor_tests::protocols::core::test_invalid_object_request());
    run_test!("invalid_opcode", edgerun_compositor_tests::protocols::core::test_invalid_opcode());
    run_test!("multiple_clients", edgerun_compositor_tests::protocols::core::test_multiple_clients());

    write_result("---");
    write_result("[conformance] XDG shell tests...");

    // XDG shell tests
    run_test!("xdg_toplevel_lifecycle", edgerun_compositor_tests::protocols::xdg_shell::test_xdg_toplevel_lifecycle());
    run_test!("xdg_ping_pong", edgerun_compositor_tests::protocols::xdg_shell::test_xdg_ping_pong());
    run_test!("wm_capabilities", edgerun_compositor_tests::protocols::xdg_shell::test_wm_capabilities());
    run_test!("toplevel_maximize", edgerun_compositor_tests::protocols::xdg_shell::test_toplevel_set_maximized());
    run_test!("toplevel_minimize", edgerun_compositor_tests::protocols::xdg_shell::test_toplevel_minimize());

    write_result("---");

    let total = results.len();
    let passed = results.iter().filter(|(_, p, _)| *p).count();
    let failed = total - passed;

    write_result(&format!("[conformance] {} total, {} passed, {} failed", total, passed, failed));

    if failed > 0 {
        write_result("[conformance] FAILURES:");
        for (name, _, msg) in &results {
            if !results.iter().find(|(n, p, _)| n == name && *p).map(|_| true).unwrap_or(false) {
                // This is a failure
            }
        }
        for (name, passed, msg) in &results {
            if !passed {
                write_result(&format!("  [FAIL] {}: {}", name, msg));
            }
        }
    } else {
        write_result("[conformance] ALL TESTS PASSED");
    }

    results
}

pub fn main() {
    let results = run_tests();
    let failed = results.iter().filter(|(_, p, _)| !p).count();

    eprintln!("[conformance] {} tests, {} passed, {} failed",
        results.len(),
        results.iter().filter(|(_, p, _)| *p).count(),
        failed);

    if failed > 0 {
        eprintln!("[conformance] FAILURES:");
        for (name, _, msg) in &results {
            if !results.iter().find(|(n, p, _)| n == name && *p).map(|_| true).unwrap_or(false) {
                // skip
            }
        }
        for (name, passed, msg) in &results {
            if !passed {
                eprintln!("  [FAIL] {}: {}", name, msg);
            }
        }
    }

    eprintln!("[conformance] Holding for 5 seconds — read the screen...");
    std::thread::sleep(std::time::Duration::from_secs(5));

    std::process::exit(if failed > 0 { 1 } else { 0 });
}
