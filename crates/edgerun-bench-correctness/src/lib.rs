//! Benchmark: CPU vs GPU rasterization correctness + performance.
//!
//! Renders identical scenes with both the CPU scanline rasterizer
//! and the GPU fragment shader, then compares pixel-by-pixel.
//!
//! Outputs:
//! - edgerun_bench_cpu.png   (CPU output)
//! - edgerun_bench_gpu.png   (GPU output)
//! - edgerun_bench_diff.png  (diff visualization: red = mismatch)
//! - edgerun_bench_report.json (timing + diff stats)

pub mod diff_engine;

use edgerun_rasterizer::framebuffer::Framebuffer;
use edgerun_rasterizer::scanline::{self, RasterCommand};
use edgerun_rasterizer::gradient::GradientStop;
use edgerun_wgpu::uniforms::{GpuRectStyle, GpuTextCommand};

pub const WIDTH: u32 = 960;
pub const HEIGHT: u32 = 640;

// ─── Test Scene ───

pub struct TestScene {
    pub rects: Vec<TestRect>,
    pub texts: Vec<TestText>,
}

pub struct TestRect {
    x: u32, y: u32, w: u32, h: u32,
    r: u8, g: u8, b: u8, a: u8,
    kind: RectKind,
}

pub enum RectKind {
    Solid,
    LinearGradient { angle: f64, stops: Vec<(u8, u8, u8, u8, f64)> },
    RadialGradient { cx: f64, cy: f64, stops: Vec<(u8, u8, u8, u8, f64)> },
    ConicGradient { from_angle: f64, cx: f64, cy: f64, stops: Vec<(u8, u8, u8, u8, f64)> },
    Border { width: u8, style: u8, br: u8, bg: u8, bb: u8 },
    DropShadow { blur: f32, spread: f32, ox: f32, oy: f32, sr: u8, sg: u8, sb: u8, sa: u8 },
    InsetShadow { blur: f32, spread: f32, ox: f32, oy: f32, sr: u8, sg: u8, sb: u8, sa: u8 },
}

pub struct TestText {
    x: u32, y: u32,
    text: String,
    r: u8, g: u8, b: u8,
}

pub fn build_test_scene() -> TestScene {
    let mut rects = Vec::new();
    let mut texts = Vec::new();

    // Background: solid dark
    rects.push(TestRect {
        x: 0, y: 0, w: WIDTH, h: HEIGHT,
        r: 0x1a, g: 0x1a, b: 0x2e, a: 255,
        kind: RectKind::Solid,
    });

    // Solid color swatches
    let colors = [
        (0xFF, 0x00, 0x00, "RED"),
        (0x00, 0xFF, 0x00, "GRN"),
        (0x00, 0x00, 0xFF, "BLU"),
        (0xFF, 0xFF, 0x00, "YLW"),
        (0x00, 0xFF, 0xFF, "CYN"),
        (0xFF, 0x00, 0xFF, "MAG"),
    ];
    for (i, &(r, g, b, label)) in colors.iter().enumerate() {
        let x = 30 + (i as u32) * 150;
        rects.push(TestRect {
            x, y: 30, w: 120, h: 80,
            r, g, b, a: 255,
            kind: RectKind::Solid,
        });
        texts.push(TestText {
            x: x + 40, y: 115,
            text: label.to_string(),
            r, g, b,
        });
    }

    // Linear gradient
    rects.push(TestRect {
        x: 30, y: 160, w: 200, h: 80,
        r: 0, g: 0, b: 0, a: 255,
        kind: RectKind::LinearGradient {
            angle: 0.0,
            stops: vec![
                (0xFF, 0x00, 0x00, 255, 0.0),
                (0x00, 0xFF, 0x00, 255, 0.5),
                (0x00, 0x00, 0xFF, 255, 1.0),
            ],
        },
    });
    texts.push(TestText { x: 30, y: 245, text: "Linear".to_string(), r: 0xAA, g: 0xAA, b: 0xAA });

    // Radial gradient
    rects.push(TestRect {
        x: 260, y: 160, w: 200, h: 80,
        r: 0, g: 0, b: 0, a: 255,
        kind: RectKind::RadialGradient {
            cx: 0.5, cy: 0.5,
            stops: vec![
                (0xFF, 0xFF, 0xFF, 255, 0.0),
                (0x44, 0x44, 0xFF, 255, 1.0),
            ],
        },
    });
    texts.push(TestText { x: 260, y: 245, text: "Radial".to_string(), r: 0xAA, g: 0xAA, b: 0xAA });

    // Conic gradient
    rects.push(TestRect {
        x: 490, y: 160, w: 200, h: 80,
        r: 0, g: 0, b: 0, a: 255,
        kind: RectKind::ConicGradient {
            from_angle: 0.0,
            cx: 0.5, cy: 0.5,
            stops: vec![
                (0xFF, 0x00, 0x80, 255, 0.0),
                (0x00, 0xFF, 0x80, 255, 0.33),
                (0x80, 0x00, 0xFF, 255, 0.66),
                (0xFF, 0x00, 0x80, 255, 1.0),
            ],
        },
    });
    texts.push(TestText { x: 490, y: 245, text: "Conic".to_string(), r: 0xAA, g: 0xAA, b: 0xAA });

    // Solid with alpha
    rects.push(TestRect {
        x: 30, y: 280, w: 300, h: 60,
        r: 0x00, g: 0x80, b: 0xFF, a: 128,
        kind: RectKind::Solid,
    });
    texts.push(TestText { x: 30, y: 345, text: "Alpha=128".to_string(), r: 0x00, g: 0x80, b: 0xFF });

    // Full-width alpha overlay
    rects.push(TestRect {
        x: 0, y: 380, w: WIDTH, h: 40,
        r: 0xFF, g: 0xFF, b: 0x00, a: 64,
        kind: RectKind::Solid,
    });
    texts.push(TestText { x: 30, y: 425, text: "Yellow stripe alpha=64".to_string(), r: 0xFF, g: 0xFF, b: 0x00 });

    // More solids
    rects.push(TestRect {
        x: 30, y: 460, w: 400, h: 60,
        r: 0x22, g: 0x44, b: 0x66, a: 255,
        kind: RectKind::Solid,
    });

    // Shadow test cards
    // Drop shadow with blur
    rects.push(TestRect {
        x: 30, y: 540, w: 150, h: 80,
        r: 0x44, g: 0x66, b: 0x88, a: 255,
        kind: RectKind::DropShadow { blur: 10.0, spread: 0.0, ox: 5.0, oy: 5.0, sr: 0, sg: 0, sb: 0, sa: 128 },
    });

    // Drop shadow with spread
    rects.push(TestRect {
        x: 200, y: 540, w: 150, h: 80,
        r: 0x66, g: 0x88, b: 0x44, a: 255,
        kind: RectKind::DropShadow { blur: 8.0, spread: 3.0, ox: -3.0, oy: 3.0, sr: 0, sg: 0, sb: 0, sa: 180 },
    });

    // Inset shadow
    rects.push(TestRect {
        x: 370, y: 540, w: 150, h: 80,
        r: 0x88, g: 0x44, b: 0x66, a: 255,
        kind: RectKind::InsetShadow { blur: 8.0, spread: 0.0, ox: 3.0, oy: 3.0, sr: 0, sg: 0, sb: 0, sa: 180 },
    });

    texts.push(TestText {
        x: 30, y: 530,
        text: "GPU vs CPU rasterizer comparison".to_string(),
        r: 0xDD, g: 0xDD, b: 0xDD,
    });

    TestScene { rects, texts }
}

// ─── CPU Rasterization ───

pub fn rasterize_cpu(scene: &TestScene, width: u32, height: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let mut fb = Framebuffer::new(&mut pixels, width, height);

    // Clear with background
    fb.clear();

    // Build CPU commands
    let mut cmds = Vec::new();
    for r in &scene.rects {
        match &r.kind {
            RectKind::Solid => {
                cmds.push(RasterCommand::FillRect {
                    x: r.x, y: r.y, w: r.w, h: r.h,
                    r: r.r, g: r.g, b: r.b, a: r.a,
                });
            }
            RectKind::LinearGradient { angle, stops } => {
                let gpu_stops: Vec<GradientStop> = stops.iter().map(|&(r, g, b, a, pos)| {
                    GradientStop { r, g, b, a, position: pos }
                }).collect();
                cmds.push(RasterCommand::LinearGradient {
                    x: r.x, y: r.y, w: r.w, h: r.h,
                    angle: *angle,
                    stops: gpu_stops,
                });
            }
            RectKind::RadialGradient { cx, cy, stops } => {
                let gpu_stops: Vec<GradientStop> = stops.iter().map(|&(r, g, b, a, pos)| {
                    GradientStop { r, g, b, a, position: pos }
                }).collect();
                cmds.push(RasterCommand::RadialGradient {
                    x: r.x, y: r.y, w: r.w, h: r.h,
                    cx: *cx, cy: *cy,
                    stops: gpu_stops,
                });
            }
            RectKind::ConicGradient { from_angle, cx, cy, stops } => {
                let gpu_stops: Vec<GradientStop> = stops.iter().map(|&(r, g, b, a, pos)| {
                    GradientStop { r, g, b, a, position: pos }
                }).collect();
                cmds.push(RasterCommand::ConicGradient {
                    x: r.x, y: r.y, w: r.w, h: r.h,
                    from_angle: *from_angle, cx: *cx, cy: *cy,
                    stops: gpu_stops,
                });
            }
            RectKind::Border { width: bw, style, br, bg, bb } => {
                cmds.push(RasterCommand::StrokeRect {
                    x: r.x, y: r.y, w: r.w, h: r.h,
                    r: *br, g: *bg, b: *bb,
                    style: *style,
                    thickness: *bw as u32,
                });
            }
            // Shadows are GPU-only - CPU renders just the rect content
            RectKind::DropShadow { .. } | RectKind::InsetShadow { .. } => {
                cmds.push(RasterCommand::FillRect {
                    x: r.x, y: r.y, w: r.w, h: r.h,
                    r: r.r, g: r.g, b: r.b, a: r.a,
                });
            }
        }
    }

    for t in &scene.texts {
        cmds.push(RasterCommand::Text {
            x: t.x, y: t.y,
            text: t.text.clone(),
            r: t.r, g: t.g, b: t.b,
        });
    }

    scanline::rasterize(&mut fb, &cmds);

    // Convert XRGB8888 to RGBA (the GPU outputs RGBA)
    // CPU framebuffer is XRGB8888 little-endian: [B, G, R, 0xFF]
    let mut rgba = vec![0u8; pixels.len()];
    for i in (0..pixels.len()).step_by(4) {
        let b = pixels[i];
        let g = pixels[i + 1];
        let r = pixels[i + 2];
        let a = pixels[i + 3];
        // For alpha handling, the CPU rasterizer blends into the framebuffer
        // but the framebuffer stores pre-multiplied alpha. We just return raw.
        rgba[i] = r;
        rgba[i + 1] = g;
        rgba[i + 2] = b;
        rgba[i + 3] = a;
    }
    rgba
}

// ─── GPU Rasterization ───

pub fn rasterize_gpu(scene: &TestScene, width: u32, height: u32) -> Vec<u8> {
    let mut rects = Vec::new();
    let mut text_cmds = Vec::new();

    for (i, r) in scene.rects.iter().enumerate() {
        match &r.kind {
            RectKind::Solid => {
                rects.push(GpuRectStyle::solid(
                    r.x as f32, r.y as f32, r.w as f32, r.h as f32,
                    r.r, r.g, r.b, r.a,
                    i as u32,
                ));
            }
            RectKind::LinearGradient { angle, stops } => {
                let gpu_stops: Vec<(u8, u8, u8, u8, f32)> = stops.iter()
                    .map(|&(r, g, b, a, pos)| (r, g, b, a, pos as f32)).collect();
                rects.push(GpuRectStyle::linear_gradient(
                    r.x as f32, r.y as f32, r.w as f32, r.h as f32,
                    *angle as f32,
                    &gpu_stops,
                    i as u32,
                ));
            }
            RectKind::RadialGradient { cx, cy, stops } => {
                let gpu_stops: Vec<(u8, u8, u8, u8, f32)> = stops.iter()
                    .map(|&(r, g, b, a, pos)| (r, g, b, a, pos as f32)).collect();
                rects.push(GpuRectStyle::radial_gradient(
                    r.x as f32, r.y as f32, r.w as f32, r.h as f32,
                    *cx as f32, *cy as f32,
                    &gpu_stops,
                    i as u32,
                ));
            }
            RectKind::ConicGradient { from_angle, cx, cy, stops } => {
                let gpu_stops: Vec<(u8, u8, u8, u8, f32)> = stops.iter()
                    .map(|&(r, g, b, a, pos)| (r, g, b, a, pos as f32)).collect();
                rects.push(GpuRectStyle::conic_gradient(
                    r.x as f32, r.y as f32, r.w as f32, r.h as f32,
                    *from_angle as f32,
                    *cx as f32, *cy as f32,
                    &gpu_stops,
                    i as u32,
                ));
            }
            RectKind::Border { width: bw, style, br, bg, bb } => {
                rects.push(GpuRectStyle::with_border(
                    r.x as f32, r.y as f32, r.w as f32, r.h as f32,
                    r.r, r.g, r.b, r.a,
                    *bw as f32, *style as u32,
                    *br, *bg, *bb,
                    0.0,
                    i as u32,
                ));
            }
            RectKind::DropShadow { blur, spread, ox, oy, sr, sg, sb, sa } => {
                rects.push(GpuRectStyle::with_shadow(
                    r.x as f32, r.y as f32, r.w as f32, r.h as f32,
                    r.r, r.g, r.b, r.a,
                    *sr, *sg, *sb, *sa,
                    *blur, *spread,
                    *ox, *oy,
                    0, // drop shadow
                    i as u32,
                ));
            }
            RectKind::InsetShadow { blur, spread, ox, oy, sr, sg, sb, sa } => {
                rects.push(GpuRectStyle::with_shadow(
                    r.x as f32, r.y as f32, r.w as f32, r.h as f32,
                    r.r, r.g, r.b, r.a,
                    *sr, *sg, *sb, *sa,
                    *blur, *spread,
                    *ox, *oy,
                    1, // inset shadow
                    i as u32,
                ));
            }
        }
    }

    for t in &scene.texts {
        text_cmds.push(GpuTextCommand::new(
            t.x as f32, t.y as f32,
            t.r, t.g, t.b,
            &t.text,
        ));
    }

    edgerun_wgpu::render::render_to_pixels(width, height, &rects, &text_cmds)
}

