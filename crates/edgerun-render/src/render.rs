//! Unified render pipeline: HTML + CSS → RGBA pixels.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use edgerun_rasterizer::scanline::{self, RasterCommand, cmd_text};
use edgerun_rasterizer::framebuffer::Framebuffer;

use crate::html::parse_html;
use crate::html::Node;
use crate::css::parse_css;
use crate::layout::layout_builder::build_layout;
use crate::layout::position_layout::{PositionedNode, PositionedKind};

/// Render HTML + CSS into a RGBA pixel buffer.
///
/// Returns `width * height * 4` bytes in RGBA order.
///
/// ## Example
///
/// ```ignore
/// let pixels = edgerun_render::render(
///     "<p>Hello World</p>",
///     "p { color: white; font-size: 16px; }",
///     960, 640,
/// );
/// ```
pub fn render(html: &str, css: &str, width: u32, height: u32) -> Vec<u8> {
    // Parse → build layout tree with typed ComputedStyle
    let dom = parse_html(html);
    let sheet = parse_css(css);
    let root = build_layout(&dom, &sheet, width);

    // Position (two-pass block layout: measure heights, assign y positions)
    let margin = 20.0;
    let positioned = crate::layout::position_layout::position_tree(
        &root, width as f64, margin
    );

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
        PositionedKind::TextRun { text, font: _, style } => {
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
    use alloc::vec::Vec;

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
        let any_lit = pixels.chunks(4).any(|c| c.len() == 4 && (c[0] > 0 || c[1] > 0 || c[2] > 0));
        assert!(any_lit, "expected some non-black pixels from white text");
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
