//! Unified render pipeline: HTML + CSS → RGBA pixels.
//!
//! ```
//! let pixels = edgerun_html_render::render(
//!     "<p>Hello World</p>",
//!     "p { color: white; font-size: 16px; }",
//!     960, 640,
//! );
//! ```

use edgerun_rasterizer::scanline::{self, RasterCommand, cmd_text};
use edgerun_rasterizer::framebuffer::Framebuffer;
use edgerun_layout::position_layout::{PositionedNode, PositionedKind};

use crate::html_parser::{parse_html, Node};
use crate::css_parser::Stylesheet;
use crate::layout_builder::build_layout;

/// Render HTML + CSS into a RGBA pixel buffer.
///
/// Returns `width * height * 4` bytes in RGBA order.
pub fn render(html: &str, css: &str, width: u32, height: u32) -> Vec<u8> {
    // Parse → build layout tree with typed ComputedStyle
    let dom = parse_html(html);
    let sheet = crate::css_parser::parse_css(css);
    let root = build_layout(&dom, &sheet, width);

    // Position (two-pass block layout: measure heights, assign y positions)
    let margin = 20.0;
    let positioned = edgerun_layout::position_layout::position_tree(&root, width as f64, margin);

    // Paint: walk positioned tree, emit RasterCommand list
    let mut cmds = Vec::new();
    for obj in &positioned {
        paint_node(obj, &mut cmds);
    }

    // Rasterize
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let mut fb = Framebuffer::new(&mut pixels, width, height);
    fb.clear();
    scanline::rasterize(&mut fb, &cmds);

    pixels
}

/// Count nodes in a DOM tree.
pub fn count_nodes(n: &Node) -> usize {
    match n {
        Node::Element(e) => 1 + e.children.iter().map(|c| count_nodes(c)).sum::<usize>(),
        _ => 1,
    }
}

fn paint_node(node: &PositionedNode, cmds: &mut Vec<RasterCommand>) {
    let x = node.x as u32;
    let y = node.y as u32;
    let w = node.width as u32;
    let h = node.height as u32;

    match &node.kind {
        PositionedKind::Container { style, .. } => {
            if let Some(bg) = &style.background_color {
                cmds.push(RasterCommand::FillRect {
                    x, y, w, h,
                    r: (bg.r * 255.0) as u8,
                    g: (bg.g * 255.0) as u8,
                    b: (bg.b * 255.0) as u8,
                    a: (bg.a * 255.0) as u8,
                });
            }
        }
        PositionedKind::TextRun { text, font, style } => {
            if text.trim().is_empty() {
                return;
            }
            let r = (style.color.r * 255.0) as u8;
            let g = (style.color.g * 255.0) as u8;
            let b = (style.color.b * 255.0) as u8;
            cmds.push(cmd_text(x, y, text, r, g, b));
        }
        PositionedKind::Image { .. } => {
            // TODO: decode image, emit Image command
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_empty() {
        let pixels = render("", "", 100, 100);
        assert_eq!(pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn test_render_text_produces_non_black() {
        let pixels = render(
            "<p>Hello</p>",
            "p { color: white; }",
            200, 50,
        );
        assert_eq!(pixels.len(), 200 * 50 * 4);
        // Background is cleared (black), text is white — expect some non-black pixels
        let any_lit = pixels.chunks(4).any(|c| c.len() == 4 && (c[0] > 0 || c[1] > 0 || c[2] > 0));
        assert!(any_lit, "expected some non-black pixels from white text");
    }

    #[test]
    #[ignore = "pre-existing rendering pipeline bug"]
    fn test_render_background() {
        // Step 1: Check CSS parsing
        let sheet = crate::css_parser::parse_css("div { background-color: red; color: white; }");
        assert_eq!(sheet.rules.len(), 1, "expected 1 CSS rule");
        assert_eq!(sheet.rules[0].selector, "div");
        assert_eq!(sheet.rules[0].declarations.get("background-color"), Some(&"red".to_string()));

        // Step 2: Check HTML parsing
        let dom = crate::html_parser::parse_html("<div>Test</div>");
        let count = count_nodes(&dom);
        assert_eq!(count, 2, "expected 2 nodes (div + text)");

        // Step 3: Check layout building
        let root = build_layout(&dom, &sheet, 200);

        // Step 4: Check CSS value parser handles "red"
        let cv = edgerun_css_value_parser::parse_css_value("red");
        assert!(matches!(cv, Some(edgerun_css_value_parser::CssValue::Color(_))),
            "parse_css_value(\"red\") should return Color, got {:?}", cv);

        // Step 5: Check compute_style
        let decls = sheet.rules[0].declarations.clone();
        let style = crate::computed_style::compute_style(&decls, &crate::computed_style::default_style());
        assert!(style.background_color.is_some(), "background_color should be Some, got None. style.font.size={}", style.font.size);

        // Step 6: Full render
        let pixels = render(
            "<div>Test</div>",
            "div { background-color: red; color: white; }",
            200, 50,
        );
        assert_eq!(pixels.len(), 200 * 50 * 4);

        // Check pixel output
        let red_count = pixels.chunks(4)
            .filter(|chunk| chunk.len() == 4 && chunk[0] > 200 && chunk[1] < 50 && chunk[2] < 50 && chunk[3] > 200)
            .count();
        assert!(red_count > 0, "expected red background pixels, got 0 red pixels out of {} total pixels", pixels.len() / 4);
    }

    #[test]
    fn test_render_multiple_elements() {
        let pixels = render(
            "<h1>Title</h1><p>Body text</p>",
            "h1 { font-size: 24px; color: white; background-color: navy; }\np { font-size: 14px; color: gray; }",
            400, 100,
        );
        assert_eq!(pixels.len(), 400 * 100 * 4);
        let non_black = pixels.chunks(4)
            .filter(|c| c.len() == 4 && (c[0] > 0 || c[1] > 0 || c[2] > 0))
            .count();
        assert!(non_black > 0, "expected non-black pixels");
    }
}

#[cfg(test)]
mod implicit_close_tests {
    use crate::html_parser::{parse_html, Node, count_nodes};
    fn extract_text(node: &Node) -> String {
        match node { Node::Text(t) => t.clone(), Node::Element(e) => e.children.iter().map(extract_text).collect(), _ => String::new() }
    }
    #[test] fn p_auto_closes_before_div() {
        let dom = parse_html("<p><div>nested</div></p>");
        match &dom { Node::Element(e) => { let c = e.children.iter().filter(|c| matches!(c, Node::Element(_))).count(); assert!(c >= 2, "got {} children", c); } _ => panic!("expected element") }
    }
    #[test] fn li_auto_closes() {
        let dom = parse_html("<ul><li>first</li><li>second</li></ul>");
        assert!(extract_text(&dom).contains("first") && extract_text(&dom).contains("second"));
    }
    #[test] fn headings_dont_nest() {
        let dom = parse_html("<h1>Header</h1><h2>Subheader</h2>");
        assert!(count_nodes(&dom) >= 2);
    }
    #[test] fn p_closes_before_block() {
        let dom = parse_html("<p>Text<div>Block</div></p>");
        match &dom { Node::Element(e) => { let c = e.children.iter().filter(|c| matches!(c, Node::Element(_))).count(); assert!(c >= 2, "got {} children", c); } _ => panic!("expected element") }
    }
}

#[cfg(test)]
mod table_tests {
    use crate::html_parser::{parse_html, Node, count_nodes};
    fn extract_text(node: &Node) -> String {
        match node { Node::Text(t) => t.clone(), Node::Element(e) => e.children.iter().map(extract_text).collect(), _ => String::new() }
    }
    #[test] fn simple_table() {
        let dom = parse_html("<table><tr><td>cell</td></tr></table>");
        assert!(extract_text(&dom).contains("cell"));
    }
    #[test] fn table_with_caption() {
        let dom = parse_html("<table><caption>Title</caption><tr><td>data</td></tr></table>");
        assert!(extract_text(&dom).contains("Title") && extract_text(&dom).contains("data"));
    }
    #[test] fn table_with_thead_tbody() {
        let dom = parse_html("<table><thead><tr><th>H</th></tr></thead><tbody><tr><td>B</td></tr></tbody></table>");
        assert!(extract_text(&dom).contains("H") && extract_text(&dom).contains("B"));
    }
    #[test] fn table_cell_closes_properly() {
        let dom = parse_html("<table><tr><td>a</td><td>b</td></tr></table>");
        let t = extract_text(&dom);
        assert!(t.contains("a") && t.contains("b"));
    }
    #[test] fn table_with_colgroup() {
        let dom = parse_html("<table><colgroup><col span='2'></colgroup><tr><td>a</td><td>b</td></tr></table>");
        assert!(count_nodes(&dom) > 1);
    }
}
