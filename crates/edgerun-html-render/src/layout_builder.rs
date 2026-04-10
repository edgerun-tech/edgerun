//! Layout builder — converts DOM + CSS → edgerun-layout RenderObject tree.
//! DO NOT EDIT. Regenerate with: scripts/generate_html_parser.py
#![no_std]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;

use crate::html_parser::{Node, Element, BLOCK_ELEMENTS, INLINE_ELEMENTS, VOID_ELEMENTS};
use crate::css_parser::Stylesheet;
use edgerun_layout::render_object::{RenderObject, ComputedStyle, FormattingContext, LayoutAlgorithm, PositionType, FontSelection, Color};
use edgerun_layout::layout_context::{determine_formatting_context, determine_layout_algorithm};

pub fn build_layout(node: &Node, stylesheet: &Stylesheet, _viewport_width: u32) -> RenderObject {
    build_node(node, stylesheet)
}

fn build_node(node: &Node, ss: &Stylesheet) -> RenderObject {
    match node {
        Node::Element(elem) => build_element(elem, ss),
        Node::Text(text) => RenderObject::TextRun {
            text: text.clone(),
            font: FontSelection { family: "sans-serif".into(), weight: 400, size: 16.0, line_height: 20.0 },
            style: default_style(FormattingContext::Inline),
        },
        Node::Comment(_) => RenderObject::TextRun {
            text: String::new(),
            font: FontSelection { family: "sans-serif".into(), weight: 400, size: 16.0, line_height: 20.0 },
            style: default_style(FormattingContext::Inline),
        },
    }
}

fn build_element(elem: &Element, ss: &Stylesheet) -> RenderObject {
    let class = elem.class();
    let id = elem.id();
    let _decls = ss.compute(&elem.tag, class, id);
    let is_block = BLOCK_ELEMENTS.contains(&elem.tag.as_str());
    let _is_void = VOID_ELEMENTS.contains(&elem.tag.as_str());

    let fc = determine_formatting_context(
        if is_block { 1 } else { 2 },
        if is_block { 1 } else { 2 },
    );
    let algo = determine_layout_algorithm(fc);

    let font_size = if elem.tag == "h1" { 32.0 } else if elem.tag == "h2" { 24.0 } else { 16.0 };
    let line_height = font_size * 1.25;
    let weight = if elem.tag.starts_with('h') { 700 } else { 400 };

    let bg = if elem.tag == "h1" {
        Some(Color { r: 1.0, g: 0.3, b: 0.0, a: 1.0 })
    } else { None };

    let style = ComputedStyle {
        formatting_context: fc, position: PositionType::Static, opacity: 1.0, z_index: None,
        background_color: bg,
        border_color: Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }, border_width: 0.0,
        color: Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
        font: FontSelection {
            family: "sans-serif".into(), weight, size: font_size, line_height,
        },
        has_transform: false, will_change: Vec::new(),
    };

    let children: Vec<RenderObject> = elem.children.iter()
        .map(|c| build_node(c, ss))
        .collect();

    if is_block && !children.is_empty() {
        let has_inlines = children.iter().any(|c| matches!(c, RenderObject::TextRun { .. }));
        if has_inlines {
            let mut grouped = Vec::new();
            let mut buf = Vec::new();
            for child in children {
                match &child {
                    RenderObject::TextRun { .. } | RenderObject::InlineContainer { .. } => buf.push(child),
                    _ => {
                        if !buf.is_empty() {
                            grouped.push(RenderObject::BlockContainer {
                                children: core::mem::take(&mut buf),
                                style: default_style(FormattingContext::Inline),
                            });
                        }
                        grouped.push(child);
                    }
                }
            }
            if !buf.is_empty() {
                grouped.push(RenderObject::BlockContainer { children: buf, style: default_style(FormattingContext::Inline) });
            }
            return match algo {
                LayoutAlgorithm::FlexMainAxis | LayoutAlgorithm::FlexCrossAxis => RenderObject::FlexContainer { children: grouped, style },
                LayoutAlgorithm::GridTracks => RenderObject::GridContainer { children: grouped, style },
                _ => RenderObject::BlockContainer { children: grouped, style },
            };
        }
    }

    match algo {
        LayoutAlgorithm::FlexMainAxis | LayoutAlgorithm::FlexCrossAxis => RenderObject::FlexContainer { children, style },
        LayoutAlgorithm::GridTracks => RenderObject::GridContainer { children, style },
        _ => RenderObject::BlockContainer { children, style },
    }
}

fn default_style(fc: FormattingContext) -> ComputedStyle {
    ComputedStyle {
        formatting_context: fc, position: PositionType::Static, opacity: 1.0, z_index: None,
        background_color: None,
        border_color: Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }, border_width: 0.0,
        color: Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
        font: FontSelection { family: "sans-serif".into(), weight: 400, size: 16.0, line_height: 20.0 },
        has_transform: false, will_change: Vec::new(),
    }
}
