//! Demo: real HTML+CSS → RenderObject → pixels via generated parsers.
use std::io::Write;

use edgerun_html_render::html_parser::parse_html;
use edgerun_html_render::css_parser::{parse_css, Stylesheet};
use edgerun_html_render::layout_builder::build_layout;

use edgerun_layout::paint_command::build_display_list;
use edgerun_rasterizer::scanline::{self, RasterCommand};
use edgerun_rasterizer::framebuffer::Framebuffer;
use edgerun_rasterizer::gradient::GradientStop;

fn main() {
    // ─── Real HTML ───
    let html = r#"
    <h1>EDGERUN DEMO</h1>
    <p>Software renderer <strong>generated from proto data</strong></p>
    <h2>Color Swatches</h2>
    <div class="colors">
        <span>White</span><span>Red</span><span>Green</span><span>Blue</span>
        <span>Yellow</span><span>Cyan</span><span>Magenta</span><span>Orange</span>
    </div>
    <h2>Border Styles</h2>
    <div class="borders">
        <span>solid</span><span>dashed</span><span>dotted</span><span>double</span>
        <span>groove</span><span>ridge</span><span>inset</span><span>outset</span>
    </div>
    <h2>Gradients</h2>
    <div class="gradients">
        <p>linear gradient</p>
        <p>radial gradient</p>
        <p>conic gradient</p>
    </div>
    <p>The quick brown fox jumps over the lazy dog</p>
    <p>ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789</p>
    <p>abcdefghijklmnopqrstuvwxyz</p>
    <!-- Generated from spec documents via proto → buf generate → Rust -->
    <footer>proto → buf generate → Rust → pixels</footer>
    "#;

    // ─── Real CSS ───
    let css = r#"
    h1 { color: white; background-color: #FF4D00; font-size: 32px; }
    h2 { color: #FFCC88; font-size: 24px; }
    p { color: #DDDDDD; font-size: 16px; }
    .colors span { color: #CCCCCC; }
    .borders span { color: #AA8855; }
    .gradients p { color: #AAAAAA; }
    footer { color: #666688; background-color: #0D0D1A; }
    "#;

    // Parse
    let dom = parse_html(html);
    let stylesheet = parse_css(css);

    println!("DOM parsed: {} nodes", count_nodes(&dom));
    println!("CSS parsed: {} rules", stylesheet.rules.len());

    // Build layout tree
    let root = build_layout(&dom, &stylesheet, 960);
    println!("Layout tree built");

    // Build display list
    let paint_cmds = build_display_list(&root);
    println!("Display list: {} commands", paint_cmds.len());

    // Rasterize
    let width: u32 = 960;
    let height: u32 = 640;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    // Clear + background gradient
    let mut fb = Framebuffer::new(&mut pixels, width, height);
    fb.clear();

    // Background gradient
    let mut cmds: Vec<RasterCommand> = vec![RasterCommand::LinearGradient {
        x: 0, y: 0, w: width, h: height,
        angle: std::f64::consts::PI / 2.0,
        stops: vec![
            GradientStop { r: 0x1a, g: 0x1a, b: 0x2e, a: 255, position: 0.0 },
            GradientStop { r: 0x2d, g: 0x1b, b: 0x4e, a: 255, position: 1.0 },
        ],
    }];

    // Merge layout paint commands
    // (Convert PaintCommand → RasterCommand)
    for cmd in paint_cmds {
        cmds.push(convert_paint_command(cmd));
    }

    scanline::rasterize(&mut fb, &cmds);

    // Write PPM
    let mut f = std::fs::File::create("edgerun_html.png").unwrap();
    // Actually write PPM first then convert
    let mut p = std::fs::File::create("edgerun_html.ppm").unwrap();
    writeln!(p, "P6").unwrap();
    writeln!(p, "{} {}", width, height).unwrap();
    writeln!(p, "255").unwrap();
    for y in 0..height {
        for x in 0..width {
            let i = (y * width * 4 + x * 4) as usize;
            let b = pixels[i]; let g = pixels[i+1]; let r = pixels[i+2];
            p.write_all(&[r, g, b]).unwrap();
        }
    }
    println!("Wrote edgerun_html.ppm ({}x{} pixels)", width, height);
}

fn convert_paint_command(cmd: edgerun_layout::paint_command::PaintCommand) -> RasterCommand {
    match cmd {
        edgerun_layout::paint_command::PaintCommand::FillRect { rect, color } => {
            RasterCommand::FillRect {
                x: rect.x as u32, y: rect.y as u32,
                w: rect.width.max(1.0) as u32, h: rect.height.max(1.0) as u32,
                r: (color.r * 255.0) as u8, g: (color.g * 255.0) as u8,
                b: (color.b * 255.0) as u8, a: (color.a * 255.0) as u8,
            }
        }
        edgerun_layout::paint_command::PaintCommand::StrokeRect { rect, color, width } => {
            RasterCommand::StrokeRect {
                x: rect.x as u32, y: rect.y as u32,
                w: rect.width.max(1.0) as u32, h: rect.height.max(1.0) as u32,
                r: (color.r * 255.0) as u8, g: (color.g * 255.0) as u8,
                b: (color.b * 255.0) as u8, style: 1, thickness: width as u32,
            }
        }
        edgerun_layout::paint_command::PaintCommand::DrawText { rect, text, color, .. } => {
            RasterCommand::Text {
                x: rect.x as u32, y: rect.y as u32, text,
                r: (color.r * 255.0) as u8, g: (color.g * 255.0) as u8,
                b: (color.b * 255.0) as u8,
            }
        }
        _ => RasterCommand::FillRect { x: 0, y: 0, w: 1, h: 1, r: 0, g: 0, b: 0, a: 0 },
    }
}

fn count_nodes(node: &edgerun_html_render::html_parser::Node) -> usize {
    match node {
        edgerun_html_render::html_parser::Node::Element(elem) => {
            1 + elem.children.iter().map(|c| count_nodes(c)).sum::<usize>()
        }
        _ => 1,
    }
}
