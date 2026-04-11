//! Layout builder — converts DOM + CSS → edgerun-layout RenderObject tree.
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::html_parser::{Node, Element, BLOCK_ELEMENTS, VOID_ELEMENTS};
use crate::css_parser::Stylesheet;
use crate::computed_style::{compute_style, default_style};
use edgerun_layout::render_object::{RenderObject, ComputedStyle, LayoutAlgorithm};
use edgerun_layout::layout_context::{determine_formatting_context, determine_layout_algorithm};

pub fn build_layout(node: &Node, stylesheet: &Stylesheet, _viewport_width: u32) -> RenderObject {
    build_node(node, stylesheet, &default_style())
}

fn build_node(node: &Node, ss: &Stylesheet, parent_style: &ComputedStyle) -> RenderObject {
    match node {
        Node::Element(elem) => build_element(elem, ss, parent_style),
        Node::Text(text) => RenderObject::TextRun {
            text: text.clone(),
            font: parent_style.font.clone(),
            style: parent_style.clone(),
        },
        Node::Comment(_) => RenderObject::TextRun {
            text: String::new(),
            font: parent_style.font.clone(),
            style: parent_style.clone(),
        },
    }
}

fn build_element(elem: &Element, ss: &Stylesheet, parent_style: &ComputedStyle) -> RenderObject {
    let class = elem.class();
    let id = elem.id();
    let decls = ss.compute(&elem.tag, class, id);

    // Compute styled values from CSS declarations (with parent inheritance)
    let style = compute_style(&decls, parent_style);

    let is_block = BLOCK_ELEMENTS.contains(&elem.tag.as_str());
    let _is_void = VOID_ELEMENTS.contains(&elem.tag.as_str());

    let fc = determine_formatting_context(
        if is_block { 1 } else { 2 },
        if is_block { 1 } else { 2 },
    );
    let algo = determine_layout_algorithm(fc);

    let children: Vec<RenderObject> = elem.children.iter()
        .map(|c| build_node(c, ss, &style))
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
                                style: default_style(),
                            });
                        }
                        grouped.push(child);
                    }
                }
            }
            if !buf.is_empty() {
                grouped.push(RenderObject::BlockContainer { children: buf, style: default_style() });
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
