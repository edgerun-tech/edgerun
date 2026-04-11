//! Two-pass block layout — positions RenderObject nodes with computed x/y/width/height.
//!
//! Pass 1 (measure): compute height of every block node.
//! Pass 2 (position): assign y position using measured heights, x = margin.
//!
//! This mirrors the algorithm proven in `edgerun-demo/src/main.rs` (measure_node + paint_node)
//! but operates on the typed `RenderObject` tree instead of raw DOM nodes.
#![cfg_attr(not(test), no_std)]
extern crate alloc;
use alloc::vec::Vec;

use crate::render_object::{RenderObject, ComputedStyle, FontSelection};

/// A positioned render object with computed geometry.
#[derive(Debug, Clone)]
pub struct PositionedNode {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub kind: PositionedKind,
}

/// What kind of content a positioned node contains.
#[derive(Debug, Clone)]
pub enum PositionedKind {
    /// Container with child nodes (block, flex, grid, inline).
    Container { children: Vec<PositionedNode>, style: ComputedStyle },
    /// A text run with resolved font metrics.
    TextRun { text: alloc::string::String, font: FontSelection, style: ComputedStyle },
    /// An image element.
    Image { src: alloc::string::String, style: ComputedStyle },
}

/// Compute positions for an entire RenderObject tree.
///
/// Returns a flat list of PositionedNode in paint order.
/// `viewport_width`: total available horizontal space.
/// `margin`: left/right margin in pixels.
pub fn position_tree(
    root: &RenderObject,
    viewport_width: f64,
    margin: f64,
) -> Vec<PositionedNode> {
    let avail = viewport_width - 2.0 * margin;
    let mut heights = Vec::new();

    // Pass 1: measure heights of every node
    measure_node(root, avail, &mut heights);

    // Pass 2: assign y positions using measured heights
    let mut ctx = PositionCtx {
        heights: &heights,
        hidx: 0,
        y: 0.0,
        avail,
        margin,
    };
    position_node(root, &mut ctx)
}

struct PositionCtx<'a> {
    heights: &'a [f64],
    hidx: usize,
    y: f64,
    avail: f64,
    margin: f64,
}

// ---------------------------------------------------------------------------
// Pass 1: Measure heights
// ---------------------------------------------------------------------------

fn measure_node(node: &RenderObject, avail: f64, out: &mut Vec<f64>) {
    match node {
        RenderObject::BlockContainer { children, style }
        | RenderObject::FlexContainer { children, style }
        | RenderObject::GridContainer { children, style }
        | RenderObject::InlineContainer { children, style } => {
            // Measure children first
            let start = out.len();
            for c in children {
                measure_node(c, avail, out);
            }
            let child_heights = &out[start..];

            // Sum up children's heights
            let mut total = 0.0;
            let mut ci = 0;
            for c in children {
                match c {
                    RenderObject::TextRun { text, font, .. } => {
                        if text.trim().is_empty() {
                            total += font.size * 1.25;
                        } else {
                            let char_w = font.size * 0.5;
                            let cpl = (avail / char_w).max(10.0) as usize;
                            let lines = libm::ceil(text.chars().count() as f64 / cpl as f64).max(1.0);
                            total += font.size * 1.25 * lines;
                        }
                    }
                    _ => {
                        total += child_heights.get(ci).copied().unwrap_or(style.font.size * 1.5);
                        ci += 1;
                    }
                }
            }

            // Add small padding
            total += 4.0;
            // Minimum height: 1.5x the font size
            total = total.max(style.font.size * 1.5);
            out.push(total);
        }
        RenderObject::TextRun { text, font, .. } => {
            if text.trim().is_empty() {
                out.push(font.size * 1.25);
            } else {
                let char_w = font.size * 0.5;
                let cpl = (avail / char_w).max(10.0) as usize;
                let lines = libm::ceil(text.chars().count() as f64 / cpl as f64).max(1.0);
                out.push(font.size * 1.25 * lines);
            }
        }
        RenderObject::Image { intrinsic_size, style: _style, .. } => {
            let h = intrinsic_size.1.unwrap_or(100.0) + 4.0;
            out.push(h);
        }
    }
}

// ---------------------------------------------------------------------------
// Pass 2: Assign positions
// ---------------------------------------------------------------------------

fn position_node(node: &RenderObject, ctx: &mut PositionCtx) -> Vec<PositionedNode> {
    match node {
        RenderObject::BlockContainer { children, style }
        | RenderObject::FlexContainer { children, style }
        | RenderObject::GridContainer { children, style }
        | RenderObject::InlineContainer { children, style } => {
            let my_idx = ctx.hidx;
            ctx.hidx += 1;
            let my_y = ctx.y;
            let my_h = ctx.heights.get(my_idx).copied().unwrap_or(style.font.size * 1.5);
            ctx.y += my_h;

            // Position children
            let mut positioned_children = Vec::new();
            for c in children {
                let child_positions = position_node(c, ctx);
                positioned_children.extend(child_positions);
            }

            alloc::vec![PositionedNode {
                x: ctx.margin,
                y: my_y,
                width: ctx.avail,
                height: my_h,
                kind: PositionedKind::Container {
                    children: positioned_children,
                    style: style.clone(),
                },
            }]
        }
        RenderObject::TextRun { text, font, style } => {
            let my_y = ctx.y;
            let char_w = font.size * 0.5;
            let cpl = (ctx.avail / char_w).max(10.0) as usize;
            let lines = libm::ceil(text.chars().count() as f64 / cpl as f64).max(1.0);
            let h = font.size * 1.25 * lines;
            ctx.y += h;

            alloc::vec![PositionedNode {
                x: ctx.margin,
                y: my_y,
                width: ctx.avail,
                height: h,
                kind: PositionedKind::TextRun {
                    text: text.clone(),
                    font: font.clone(),
                    style: style.clone(),
                },
            }]
        }
        RenderObject::Image { src, intrinsic_size, style } => {
            let my_y = ctx.y;
            let h = intrinsic_size.1.unwrap_or(100.0) + 4.0;
            ctx.y += h;

            alloc::vec![PositionedNode {
                x: ctx.margin,
                y: my_y,
                width: ctx.avail,
                height: h,
                kind: PositionedKind::Image {
                    src: src.clone(),
                    style: style.clone(),
                },
            }]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_object::{Color, FormattingContext, PositionType};

    fn default_style() -> ComputedStyle {
        ComputedStyle {
            formatting_context: FormattingContext::Block,
            position: PositionType::Static,
            opacity: 1.0,
            z_index: None,
            background_color: None,
            border_color: Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            border_width: 0.0,
            color: Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
            font: FontSelection { family: "sans-serif".into(), weight: 400, size: 16.0, line_height: 20.0 },
            has_transform: false,
            will_change: Vec::new(),
        }
    }

    #[test]
    fn test_measure_text_node() {
        let text = RenderObject::TextRun {
            text: "Hello World".into(),
            font: FontSelection { family: "sans-serif".into(), weight: 400, size: 16.0, line_height: 20.0 },
            style: default_style(),
        };
        let mut heights = Vec::new();
        measure_node(&text, 920.0, &mut heights);
        assert_eq!(heights.len(), 1);
        assert!(heights[0] > 0.0);
    }

    #[test]
    fn test_position_single_text() {
        let text = RenderObject::TextRun {
            text: "Hello".into(),
            font: FontSelection { family: "sans-serif".into(), weight: 400, size: 16.0, line_height: 20.0 },
            style: default_style(),
        };
        let positioned = position_tree(&text, 960.0, 20.0);
        assert_eq!(positioned.len(), 1);
        assert_eq!(positioned[0].x, 20.0);
        assert_eq!(positioned[0].y, 0.0);
        assert!(positioned[0].height > 0.0);
    }

    #[test]
    fn test_position_container_with_two_texts() {
        let t1 = RenderObject::TextRun {
            text: "First line".into(),
            font: FontSelection { family: "sans-serif".into(), weight: 400, size: 16.0, line_height: 20.0 },
            style: default_style(),
        };
        let t2 = RenderObject::TextRun {
            text: "Second line".into(),
            font: FontSelection { family: "sans-serif".into(), weight: 400, size: 16.0, line_height: 20.0 },
            style: default_style(),
        };
        let container = RenderObject::BlockContainer {
            children: alloc::vec![t1, t2],
            style: default_style(),
        };
        let positioned = position_tree(&container, 960.0, 20.0);
        assert_eq!(positioned.len(), 1);
        // Container height should be > 0 and contain both text children
        assert!(positioned[0].height > 40.0); // at least 2 * 20px
    }

    #[test]
    fn test_position_nested_containers() {
        let text = RenderObject::TextRun {
            text: "Deep text".into(),
            font: FontSelection { family: "sans-serif".into(), weight: 400, size: 14.0, line_height: 17.5 },
            style: default_style(),
        };
        let inner = RenderObject::BlockContainer {
            children: alloc::vec![text],
            style: default_style(),
        };
        let outer = RenderObject::BlockContainer {
            children: alloc::vec![inner],
            style: default_style(),
        };
        let positioned = position_tree(&outer, 960.0, 20.0);
        assert!(!positioned.is_empty());
        assert!(positioned[0].height > 0.0);
    }

    #[test]
    fn test_position_empty_text_still_has_height() {
        let text = RenderObject::TextRun {
            text: "   ".into(),
            font: FontSelection { family: "sans-serif".into(), weight: 400, size: 16.0, line_height: 20.0 },
            style: default_style(),
        };
        let positioned = position_tree(&text, 960.0, 20.0);
        assert_eq!(positioned.len(), 1);
        // Empty text still gets a position with minimal height
        assert!(positioned[0].height >= 20.0);
    }
}
