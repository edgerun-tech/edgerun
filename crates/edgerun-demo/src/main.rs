//! Demo: real HTML+CSS → cascade resolution → two-pass block layout → pixels.
use std::io::Write;
use std::collections::BTreeMap;

use edgerun_render::html::{parse_html, Node, BLOCK_ELEMENTS};
use edgerun_css_cascade::{
    CascadeStylesheet, Origin,
    parse_stylesheet as parse_css_cascade,
    DomElement,
};

use edgerun_rasterizer::scanline::{self, RasterCommand, cmd_fill, cmd_text};
use edgerun_rasterizer::gradient::GradientStop;
use edgerun_rasterizer::framebuffer::Framebuffer;

fn main() {
    let html = r#"
    <h1>EDGERUN DEMO</h1>
    <p>Software renderer generated from proto data</p>
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
    <!-- Generated from spec documents via proto -->
    <footer>proto -> buf generate -> Rust -> pixels</footer>
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
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let mut fb = Framebuffer::new(&mut pixels, width, height);
    fb.clear();

    let mut cmds: Vec<RasterCommand> = vec![RasterCommand::LinearGradient {
        x: 0, y: 0, w: width, h: height,
        angle: std::f64::consts::PI / 2.0,
        stops: vec![
            GradientStop { r: 0x1a, g: 0x1a, b: 0x2e, a: 255, position: 0.0 },
            GradientStop { r: 0x2d, g: 0x1b, b: 0x4e, a: 255, position: 1.0 },
        ],
    }];

    let avail = (width - 40) as f64;
    let mut measures = Vec::new();
    measure_node(&dom, &sheet, avail, &mut measures);

    let mut ctx = Ctx { cmds: &mut cmds, x: 20.0, y: 20.0, avail, measures: &measures, midx: 0, sheet: &sheet };
    paint_node(&dom, &mut ctx);

    println!("Generated {} paint commands", cmds.len());
    scanline::rasterize(&mut fb, &cmds);

    let mut f = std::fs::File::create("edgerun_html.ppm").unwrap();
    writeln!(f, "P6").unwrap(); writeln!(f, "{} {}", width, height).unwrap(); writeln!(f, "255").unwrap();
    for y in 0..height { for x in 0..width {
        let i = (y * width * 4 + x * 4) as usize;
        f.write_all(&[pixels[i+2], pixels[i+1], pixels[i]]).unwrap();
    }}
    println!("Wrote edgerun_html.ppm ({}x{})", width, height);
}

struct Ctx<'a> {
    cmds: &'a mut Vec<RasterCommand>,
    x: f64, y: f64, avail: f64,
    measures: &'a [f64], midx: usize,
    sheet: &'a CascadeStylesheet,
}

fn resolve_styles(tag: &str, class: Option<&str>, id: Option<&str>, sheet: &CascadeStylesheet) -> BTreeMap<String, String> {
    let classes_vec: Vec<&str> = class.into_iter().collect();
    let el = DomElement {
        tag_name: tag, id, classes: &classes_vec, attributes: &[],
        parent: None, prev_siblings: &[], child_index: 0, type_index: 0,
    };
    sheet.resolve(&el)
}

fn measure_node(node: &Node, sheet: &CascadeStylesheet, avail: f64, out: &mut Vec<f64>) {
    match node {
        Node::Element(elem) => {
            let is_block = BLOCK_ELEMENTS.contains(&elem.tag.as_str());
            let styles = resolve_styles(&elem.tag, elem.class(), elem.id(), sheet);
            let font_size = parse_font_size(&styles);
            if is_block {
                if let Some(h) = styles.get("height").and_then(|v| v.strip_suffix("px")).and_then(|n| n.parse::<f64>().ok()) {
                    for c in &elem.children { measure_node(c, sheet, avail, out); }
                    out.push(h + 10.0); return;
                }
                let start = out.len();
                for c in &elem.children { measure_node(c, sheet, avail, out); }
                let ch = &out[start..];
                let mut h = 0.0; let mut ci = 0;
                for c in &elem.children {
                    match c {
                        Node::Text(t) => {
                            let char_w = font_size * 0.5;
                            let cpl = (avail / char_w).max(10.0) as usize;
                            let lines = (t.chars().count() as f64 / cpl as f64).ceil().max(1.0);
                            h += font_size * 1.25 * lines;
                        }
                        _ => { h += ch.get(ci).copied().unwrap_or(20.0); ci += 1; }
                    }
                }
                out.push((h + 10.0).max(font_size * 1.5));
            } else { for c in &elem.children { measure_node(c, sheet, avail, out); } }
        }
        Node::Text(t) => {
            let char_w = 16.0 * 0.5;
            let cpl = (avail / char_w).max(10.0) as usize;
            let lines = (t.chars().count() as f64 / cpl as f64).ceil().max(1.0);
            out.push(16.0 * 1.25 * lines);
        }
        _ => {}
    }
}

fn paint_node(node: &Node, ctx: &mut Ctx) {
    match node {
        Node::Element(elem) => {
            let is_block = BLOCK_ELEMENTS.contains(&elem.tag.as_str());
            let styles = resolve_styles(&elem.tag, elem.class(), elem.id(), ctx.sheet);
            let font_size = parse_font_size(&styles);
            let color = parse_color_prop(&styles, "color").unwrap_or((0xFF, 0xFF, 0xFF));
            let bg = parse_color_prop(&styles, "background-color");

            if is_block {
                let my_y = ctx.y; let my_i = ctx.midx; ctx.midx += 1;
                let h = styles.get("height")
                    .and_then(|v| v.strip_suffix("px").and_then(|n| n.parse::<f64>().ok()))
                    .map(|h| h + 10.0)
                    .unwrap_or_else(|| ctx.measures.get(my_i).copied().unwrap_or(font_size * 1.5));

                let gt = get_gradient_type(elem.class());
                if let Some(gt) = gt {
                    ctx.cmds.push(RasterCommand::FillRect {
                        x: (ctx.x + 10.0) as u32, y: my_y as u32, w: 200, h: 80, r: 0x11, g: 0x11, b: 0x22, a: 255,
                    });
                    match gt {
                        0 => ctx.cmds.push(RasterCommand::LinearGradient {
                            x: (ctx.x + 10.0) as u32, y: my_y as u32, w: 200, h: 80, angle: 0.0,
                            stops: vec![
                                GradientStop { r: 0xFF, g: 0x00, b: 0x00, a: 255, position: 0.0 },
                                GradientStop { r: 0x00, g: 0xFF, b: 0x00, a: 255, position: 0.5 },
                                GradientStop { r: 0x00, g: 0x00, b: 0xFF, a: 255, position: 1.0 },
                            ],
                        }),
                        1 => ctx.cmds.push(RasterCommand::RadialGradient {
                            x: (ctx.x + 10.0) as u32, y: my_y as u32, w: 200, h: 80, cx: 0.5, cy: 0.5,
                            stops: vec![
                                GradientStop { r: 0xFF, g: 0xFF, b: 0xFF, a: 255, position: 0.0 },
                                GradientStop { r: 0x44, g: 0x44, b: 0xFF, a: 255, position: 1.0 },
                            ],
                        }),
                        2 => ctx.cmds.push(RasterCommand::ConicGradient {
                            x: (ctx.x + 10.0) as u32, y: my_y as u32, w: 200, h: 80, from_angle: 0.0, cx: 0.5, cy: 0.5,
                            stops: vec![
                                GradientStop { r: 0xFF, g: 0x00, b: 0x80, a: 255, position: 0.0 },
                                GradientStop { r: 0x00, g: 0xFF, b: 0x80, a: 255, position: 0.33 },
                                GradientStop { r: 0x80, g: 0x00, b: 0xFF, a: 255, position: 0.66 },
                                GradientStop { r: 0xFF, g: 0x00, b: 0x80, a: 255, position: 1.0 },
                            ],
                        }),
                        _ => {}
                    }
                    let label = match gt { 0 => "linear", 1 => "radial", 2 => "conic", _ => "" };
                    ctx.cmds.push(cmd_text((ctx.x + 20.0) as u32, (my_y + 30.0) as u32, label, 0xFF, 0xFF, 0xFF));
                } else if let Some((r,g,b)) = bg {
                    ctx.cmds.push(cmd_fill(ctx.x as u32, my_y as u32, ctx.avail as u32, h as u32, r, g, b));
                }

                let saved_x = ctx.x; ctx.x += 10.0;
                for c in &elem.children {
                    match c {
                        Node::Text(t) => {
                            if t.trim().is_empty() { ctx.y += font_size * 1.25; continue; }
                            let char_w = font_size * 0.5;
                            let cpl = (ctx.avail / char_w).max(10.0) as usize;
                            let lines = (t.chars().count() as f64 / cpl as f64).ceil().max(1.0);
                            let text_w = t.chars().count() as f64 * char_w;
                            if ctx.x + text_w > 20.0 + ctx.avail && ctx.x > 30.0 { ctx.x = 30.0; ctx.y += font_size * 1.25; }
                            ctx.cmds.push(cmd_text(ctx.x as u32, ctx.y as u32, t, color.0, color.1, color.2));
                            ctx.y += font_size * 1.25 * lines;
                        }
                        _ => paint_node(c, ctx),
                    }
                }
                ctx.x = saved_x; ctx.y = my_y + h;
            } else { for c in &elem.children { paint_node(c, ctx); } }
        }
        Node::Text(t) => {
            if !t.trim().is_empty() { ctx.cmds.push(cmd_text(ctx.x as u32, ctx.y as u32, t, 0xDD, 0xDD, 0xDD)); }
            ctx.x += t.len() as f64 * 8.0 + 6.0;
        }
        Node::Comment(_) => {}
    }
}

fn get_gradient_type(class: Option<&str>) -> Option<u8> {
    match class {
        Some("gradient-linear") => Some(0), Some("gradient-radial") => Some(1), Some("gradient-conic") => Some(2), _ => None,
    }
}

fn parse_font_size(styles: &BTreeMap<String, String>) -> f64 {
    styles.get("font-size").and_then(|v| v.strip_suffix("px")).and_then(|n| n.parse().ok()).unwrap_or(16.0)
}

fn parse_color_prop(styles: &BTreeMap<String, String>, prop: &str) -> Option<(u8,u8,u8)> {
    styles.get(prop).and_then(|v| parse_color(v))
}

fn parse_color(s: &str) -> Option<(u8,u8,u8)> {
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            return Some((u8::from_str_radix(&hex[0..2], 16).ok()?, u8::from_str_radix(&hex[2..4], 16).ok()?, u8::from_str_radix(&hex[4..6], 16).ok()?));
        }
    }
    match s { "white" => Some((0xFF,0xFF,0xFF)), "black" => Some((0,0,0)), "orange" => Some((0xFF,0xA5,0)), "yellow" => Some((0xFF,0xFF,0)), _ => None }
}

fn count_nodes(n: &Node) -> usize {
    match n { Node::Element(e) => 1 + e.children.iter().map(|c| count_nodes(c)).sum::<usize>(), _ => 1 }
}
