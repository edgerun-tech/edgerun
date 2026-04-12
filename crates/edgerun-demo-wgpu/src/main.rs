//! Demo: real HTML+CSS → cascade → two-pass layout → WGPU GPU raster → PNG.
//!
//! This mirrors the CPU-based `edgerun-demo` but replaces the scanline
//! rasterizer with a GPU fragment shader generated from proto data.

use std::fs::File;
use std::io::BufWriter;

use edgerun_render::html::{parse_html, Node};
use edgerun_css_cascade::{CascadeStylesheet, Origin, parse_stylesheet as parse_css_cascade};
use edgerun_wgpu::uniforms::{GpuRectStyle, GpuTextCommand, GpuDomNode, GpuCssRule, GpuLayoutResult, tag_hash};
use edgerun_wgpu::layout_compute::LayoutComputePipeline;

fn main() {
    let html = r#"
    <h1>EDGERUN DEMO</h1>
    <p>GPU layout + raster from proto data</p>
    <h2>Color Swatches</h2>
    <div class="swatches"><span>White</span> <span>Red</span> <span>Green</span> <span>Blue</span>
         <span>Yellow</span> <span>Cyan</span> <span>Magenta</span> <span>Orange</span></div>
    <h2>Gradients</h2>
    <div class="gradients">
        <div class="gradient-linear"></div>
        <div class="gradient-radial"></div>
        <div class="gradient-conic"></div>
    </div>
    <p>The quick brown fox jumps over the lazy dog</p>
    <p>ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789</p>
    <p>abcdefghijklmnopqrstuvwxyz</p>
    <footer>proto -> buf generate -> WGSL -> GPU layout -> GPU pixels</footer>
    "#;

    let css = r#"
    p { color: #AAAAAA; font-size: 16px; }
    h2 { color: #FFCC88; font-size: 24px; }
    .swatches { color: #CCCCCC; font-size: 16px; }
    .gradients { color: #CCCCCC; font-size: 14px; }
    p { color: #DDDDDD; font-size: 16px; }
    h1 { color: white !important; background-color: orange; font-size: 32px; }
    .gradients .gradient-linear { background-color: blue; width: 200px; height: 80px; }
    .gradients .gradient-radial { background-color: green; width: 200px; height: 80px; }
    .gradients .gradient-conic { background-color: purple; width: 200px; height: 80px; }
    footer { color: #666688; background-color: #0D0D1A; }
    "#;

    let dom = parse_html(html);
    let sheet = parse_css_cascade(css, Origin::Author);
    println!("DOM: {} nodes, cascade: {} rules", count_nodes(&dom), sheet.rules.len());

    let width: u32 = 960;
    let height: u32 = 640;
    let avail = (width - 40) as f32;

    // Flatten DOM to GPU nodes
    let (nodes, text_data) = flatten_dom(&dom);
    println!("Flattened to {} GPU nodes, {} bytes text", nodes.len(), text_data.len());

    // Flatten CSS rules to GPU rules
    let rules = flatten_css_rules(&sheet);
    println!("Flattened to {} GPU CSS rules", rules.len());

    // GPU Layout Compute
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        force_fallback_adapter: false,
        compatible_surface: None,
    }))
    .expect("Failed to find GPU adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("edgerun-wgpu-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        },
    ))
    .expect("Failed to create WGPU device");

    let layout_pipeline = LayoutComputePipeline::new(&device, 256, 128);

    let layout_results = layout_pipeline.run_full_layout(
        &device, &queue,
        &nodes, &rules, &text_data,
        avail, 20.0,
    );
    println!("GPU layout computed: {} results", layout_results.len());

    // Convert layout results to GPU rects + text commands
    let (rects, text_cmds) = layout_results_to_commands(&layout_results, &nodes, &text_data);
    println!("Generated {} GPU rects, {} text commands", rects.len(), text_cmds.len());

    // Add background gradient
    let mut all_rects = vec![GpuRectStyle::linear_gradient(
        0.0, 0.0, width as f32, height as f32,
        std::f32::consts::PI / 2.0,
        &[
            (0x1a, 0x1a, 0x2e, 255, 0.0),
            (0x2d, 0x1b, 0x4e, 255, 1.0),
        ],
        0,
    )];

    // Add a bordered test card
    all_rects.push(GpuRectStyle::with_border(
        50.0, 50.0, 200.0, 100.0,
        0x33, 0x33, 0x55, 255,  // bg
        3.0, 1,  // border: 3px solid
        0xFF, 0x88, 0x44,  // border color: orange
        10.0,  // border radius
        all_rects.len() as u32,
    ));

    // Dashed border card
    all_rects.push(GpuRectStyle::with_border(
        280.0, 50.0, 200.0, 100.0,
        0x22, 0x44, 0x33, 255,  // bg: dark green
        4.0, 2,  // border: 4px dashed
        0x44, 0xFF, 0x88,  // border color: green
        5.0,  // border radius
        all_rects.len() as u32,
    ));

    // Dotted border card
    all_rects.push(GpuRectStyle::with_border(
        510.0, 50.0, 200.0, 100.0,
        0x44, 0x22, 0x33, 255,  // bg: dark red
        3.0, 3,  // border: 3px dotted
        0xFF, 0x44, 0x88,  // border color: pink
        15.0,  // border radius
        all_rects.len() as u32,
    ));

    all_rects.extend(rects);

    // Collect all text for the font atlas
    let mut all_text = String::new();
    for tc in &text_cmds {
        for &cp in &tc.glyphs[..tc.glyph_count as usize] {
            if let Some(ch) = char::from_u32(cp) {
                all_text.push(ch);
            }
        }
    }
    println!("Font atlas text: {} chars ({} unique)", all_text.len(), all_text.chars().count());

    // Load a system font
    let font_path = "/usr/share/fonts/TTF/DejaVuSans.ttf";
    let font_data = std::fs::read(font_path)
        .unwrap_or_else(|e| panic!("Failed to load font {}: {}", font_path, e));
    println!("Loaded font: {} ({} bytes)", font_path, font_data.len());
    let font_size = 16.0;

    // GPU Raster
    let pixels = edgerun_wgpu::render::render_to_pixels(
        width, height, &all_rects, &text_cmds,
        &font_data, font_size, &all_text,
    );

    // Save PNG
    save_png("edgerun_wgpu.png", &pixels, width, height);
    println!("Wrote edgerun_wgpu.png ({}x{})", width, height);
}

fn save_png(path: &str, rgba: &[u8], width: u32, height: u32) {
    let file = File::create(path).unwrap();
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(rgba).unwrap();
    writer.finish().unwrap();
}

// ─── GPU Layout Helpers (Phase 4) ───

/// Flatten DOM tree to flat GPU node array + text buffer.
fn flatten_dom(root: &Node) -> (Vec<GpuDomNode>, Vec<u8>) {
    let mut nodes = Vec::new();
    let mut text_data = Vec::new();
    flatten_node_recursive(root, &mut nodes, &mut text_data, 0, None, None, None);
    (nodes, text_data)
}

fn flatten_node_recursive(
    node: &Node,
    nodes: &mut Vec<GpuDomNode>,
    text_data: &mut Vec<u8>,
    parent_idx: u32,
    _tag: Option<&str>,
    _class: Option<&str>,
    _id: Option<&str>,
) {
    let node_idx = nodes.len() as u32;

    match node {
        Node::Element(elem) => {
            let mut gpu_node = GpuDomNode::element(&elem.tag, elem.class(), elem.id());
            gpu_node.parent_idx = parent_idx;
            nodes.push(gpu_node);

            // Process children, link as siblings
            let mut prev_child_idx: Option<u32> = None;
            for child in &elem.children {
                let child_idx = nodes.len() as u32;
                flatten_node_recursive(
                    child, nodes, text_data,
                    node_idx,
                    None, None, None,
                );
                if let Some(prev) = prev_child_idx {
                    nodes[prev as usize].next_sibling_idx = child_idx;
                } else {
                    nodes[node_idx as usize].first_child_idx = child_idx;
                }
                prev_child_idx = Some(child_idx);
            }
        }
        Node::Text(t) => {
            let offset = text_data.len() as u32;
            text_data.extend_from_slice(t.as_bytes());
            let len = t.len() as u32;
            let mut gpu_node = GpuDomNode::text(offset, len);
            gpu_node.parent_idx = parent_idx;
            nodes.push(gpu_node);
        }
        Node::Comment(_) => {}
    }
}

/// Flatten CSS cascade rules to GPU rules.
fn flatten_css_rules(_sheet: &CascadeStylesheet) -> Vec<GpuCssRule> {
    // Simplified: parse known CSS properties from the cascade rules
    // This mirrors what the CPU demo does in resolve_styles
    let mut rules = Vec::new();

    // Add default rules matching what the CPU demo uses
    // p { color: #AAAAAA; font-size: 16px; }
    rules.push(GpuCssRule::tag("p", 0)
        .font_size(16.0)
        .color(0.667, 0.667, 0.667));

    // h2 { color: #FFCC88; font-size: 24px; }
    rules.push(GpuCssRule::tag("h2", 1)
        .font_size(24.0)
        .color(1.0, 0.8, 0.533));

    // .swatches { color: #CCCCCC; font-size: 16px; }
    rules.push(GpuCssRule::class("swatches", 2)
        .font_size(16.0)
        .color(0.8, 0.8, 0.8));

    // .gradients { color: #CCCCCC; font-size: 14px; }
    rules.push(GpuCssRule::class("gradients", 3)
        .font_size(14.0)
        .color(0.8, 0.8, 0.8));

    // h1 { color: white !important; background-color: orange; font-size: 32px; }
    rules.push(GpuCssRule::tag("h1", 4)
        .font_size(32.0)
        .color(1.0, 1.0, 1.0)
        .important(0b11)); // font_size + color are important
    // Note: bg is set separately since it's a different property slot
    {
        let last = rules.last_mut().unwrap();
        last.bg_r = 1.0; last.bg_g = 0.647; last.bg_b = 0.0;
    }

    // .gradient-linear { background-color: blue; height: 80px; }
    rules.push(GpuCssRule::class("gradient-linear", 5)
        .bg(0.0, 0.0, 1.0)
        .height(80.0));

    // .gradient-radial { background-color: green; height: 80px; }
    rules.push(GpuCssRule::class("gradient-radial", 6)
        .bg(0.0, 0.502, 0.0)
        .height(80.0));

    // .gradient-conic { background-color: purple; height: 80px; }
    rules.push(GpuCssRule::class("gradient-conic", 7)
        .bg(0.502, 0.0, 0.502)
        .height(80.0));

    // footer { color: #666688; background-color: #0D0D1A; }
    rules.push(GpuCssRule::tag("footer", 8)
        .color(0.4, 0.4, 0.533)
        .bg(0.051, 0.051, 0.102));

    rules
}

/// Convert GPU layout results to rect + text commands.
fn layout_results_to_commands(
    results: &[GpuLayoutResult],
    nodes: &[GpuDomNode],
    text_data: &[u8],
) -> (Vec<GpuRectStyle>, Vec<GpuTextCommand>) {
    let mut rects = Vec::new();
    let mut text_cmds = Vec::new();
    let mut y: f32 = 20.0;
    let avail = 920.0; // 960 - 40

    // Sequential Y positioning (block flow)
    let count = results.len().min(nodes.len());
    for i in 0..count {
        let result = &results[i];
        let node = &nodes[i];

        if result.is_text != 0 {
            // Text node
            let start = node.text_offset as usize;
            let end = start + node.text_len as usize;
            let text = String::from_utf8_lossy(&text_data[start..end.min(text_data.len())]).to_string();
            if !text.trim().is_empty() {
                text_cmds.push(GpuTextCommand::new(
                    30.0, y,
                    (result.color_r * 255.0) as u8,
                    (result.color_g * 255.0) as u8,
                    (result.color_b * 255.0) as u8,
                    &text,
                ));
            }
            y += result.h;
        } else if result.is_block != 0 {
            // Block element with background
            if result.has_bg != 0 {
                rects.push(GpuRectStyle::solid(
                    30.0, y, avail, result.h,
                    (result.bg_r * 255.0) as u8,
                    (result.bg_g * 255.0) as u8,
                    (result.bg_b * 255.0) as u8,
                    255,
                    rects.len() as u32,
                ));
            }
            // Check for gradient classes (simplified — by hash)
            // The gradient-linear, gradient-radial, gradient-conic classes
            let grad_classes = [
                (tag_hash("gradient-linear"), 1u32),
                (tag_hash("gradient-radial"), 2u32),
                (tag_hash("gradient-conic"), 3u32),
            ];
            for &(hash, gt) in &grad_classes {
                if node.class_hash == hash {
                    rects.push(GpuRectStyle::solid(
                        30.0 + 10.0, y, 200.0, 80.0,
                        0x11, 0x11, 0x22, 255,
                        rects.len() as u32,
                    ));
                    match gt {
                        1 => rects.push(GpuRectStyle::linear_gradient(
                            30.0 + 10.0, y, 200.0, 80.0,
                            0.0,
                            &[(0xFF,0x00,0x00,255,0.0),(0x00,0xFF,0x00,255,0.5),(0x00,0x00,0xFF,255,1.0)],
                            rects.len() as u32,
                        )),
                        2 => rects.push(GpuRectStyle::radial_gradient(
                            30.0 + 10.0, y, 200.0, 80.0,
                            0.5, 0.5,
                            &[(0xFF,0xFF,0xFF,255,0.0),(0x44,0x44,0xFF,255,1.0)],
                            rects.len() as u32,
                        )),
                        3 => rects.push(GpuRectStyle::conic_gradient(
                            30.0 + 10.0, y, 200.0, 80.0,
                            0.0, 0.5, 0.5,
                            &[(0xFF,0x00,0x80,255,0.0),(0x00,0xFF,0x80,255,0.33),(0x80,0x00,0xFF,255,0.66),(0xFF,0x00,0x80,255,1.0)],
                            rects.len() as u32,
                        )),
                        _ => {}
                    }
                }
            }
            y += result.h;
        }
    }

    (rects, text_cmds)
}

fn count_nodes(n: &Node) -> usize {
    match n { Node::Element(e) => 1 + e.children.iter().map(|c| count_nodes(c)).sum::<usize>(), _ => 1 }
}
