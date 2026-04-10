use std::time::Instant;
use edgerun_rasterizer::scanline::{self, RasterCommand, cmd_fill};
use edgerun_rasterizer::framebuffer::Framebuffer;

fn main() {
    let width: u32 = 3840;
    let height: u32 = 2160;
    let iterations: usize = 120;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    let mut cmds: Vec<RasterCommand> = Vec::with_capacity(200);
    cmds.push(cmd_fill(0, 0, width, height, 0x1a, 0x1a, 0x2e));
    for i in 0..195 {
        let x = (i % 15) as u32 * 256;
        let y = (i / 15) as u32 * 144;
        cmds.push(cmd_fill(x, y, 250, 140, (i * 3) as u8, (i * 7) as u8, (i * 11) as u8));
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let mut fb = Framebuffer::new(&mut pixels, width, height);
        scanline::rasterize(&mut fb, &cmds);
    }
    let elapsed = start.elapsed();
    let total_pixels = (width as u64) * (height as u64) * (iterations as u64);
    let gpix = (total_pixels as f64) / elapsed.as_secs_f64() / 1e9;
    let fps = (iterations as f64) / elapsed.as_secs_f64();
    println!("4K fill-only × {} in {:?}", iterations, elapsed);
    println!("Throughput: {:.3} Gpix/sec", gpix);
    println!("Effective FPS @ 4K: {:.1}", fps);
}
