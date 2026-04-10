use std::time::Instant;

use edgerun_rasterizer::scanline::{self, RasterCommand, cmd_fill};
use edgerun_rasterizer::gradient::GradientStop;
use edgerun_rasterizer::framebuffer::Framebuffer;

fn main() {
    let width: u32 = 3840;
    let height: u32 = 2160;
    let iterations = 120;

    let mut pixels = vec![0u8; (width * height * 4) as usize];

    // Build commands: mostly solid fills (realistic for UI)
    let mut cmds: Vec<RasterCommand> = Vec::with_capacity(50);
    cmds.push(RasterCommand::LinearGradient {
        x: 0, y: 0, w: width, h: height,
        angle: std::f64::consts::PI / 2.0,
        stops: vec![
            GradientStop { r: 0x1a, g: 0x1a, b: 0x2e, a: 255, position: 0.0 },
            GradientStop { r: 0x2d, g: 0x1b, b: 0x4e, a: 255, position: 1.0 },
        ],
    });
    // 40 solid color rectangles
    for i in 0..40 {
        let x = (i % 10) as u32 * 384;
        let y = (i / 10) as u32 * 540;
        cmds.push(cmd_fill(x, y, 380, 536, (i * 6) as u8, (i * 13) as u8, (i * 7) as u8));
    }

    // Benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        let mut fb = Framebuffer::new(&mut pixels, width, height);
        scanline::rasterize(&mut fb, &cmds);
    }
    let elapsed = start.elapsed();

    let total_pixels = (width as u64) * (height as u64) * (iterations as u64);
    let gbps = (total_pixels as f64) / elapsed.as_secs_f64() / 1e9;
    let fps = (iterations as f64) / elapsed.as_secs_f64();

    println!("4K frames: {} in {:?}", iterations, elapsed);
    println!("Throughput: {:.3} gigapixels/sec", gbps);
    println!("Effective FPS: {:.1} @ 4K", fps);
    println!("Per-frame: {:.2}ms", elapsed.as_millis() as f64 / iterations as f64);
}
