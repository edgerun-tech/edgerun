//! EdgeRun system UI overlay for the compositor CPU scanout path.
//!
//! This deliberately stays above Wayland surface composition and below the
//! cursor. It is the first bridge from the compositor into `edgerun-ui-core`.

use edgerun_ui_core::{demo_dashboard, DashboardState, Painter, EDGERUN_DARK};

/// Draw the EdgeRun system dashboard into the caller-owned scanout buffer.
///
/// The CPU renderer already composites Wayland surfaces into an XRGB8888 dumb
/// buffer. This function paints a small native system UI pass directly into the
/// same buffer, before the cursor is drawn.
pub fn draw_system_ui_overlay(pixels: &mut [u8], width: u32, height: u32, pitch: u32) {
    if pixels.is_empty() || width == 0 || height == 0 || pitch < width.saturating_mul(4) {
        return;
    }

    let mut painter = Painter {
        pixels,
        width,
        height,
        pitch,
    };

    demo_dashboard(
        &mut painter,
        DashboardState {
            title: "EdgeRun",
            subtitle: "semantic UI / compositor native / no browser chrome",
            node_status: "native",
            cpu_permille: 170,
            memory_mb: 42,
            network_status: "mesh ready",
        },
        EDGERUN_DARK,
    );
}
