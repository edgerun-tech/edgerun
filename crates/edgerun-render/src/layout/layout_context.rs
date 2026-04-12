//! Layout context dispatch — generated from CSS Display proto.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py
use super::render_object::{FormattingContext, LayoutAlgorithm};

/// DisplayOuter × DisplayInner → FormattingContext
pub fn determine_formatting_context(outer: u8, inner: u8) -> FormattingContext {
    // outer: 1=flow, 2=flow-root, 3=table, 4=flex, 5=grid, 6=ruby, 7=none, 8=contents
    // inner: 1=block, 2=inline, 3+=table-*, 11=flex, 12=grid, 17=list-item, 18=math
    if outer >= 4 && outer <= 5 { return if outer == 4 { FormattingContext::Flex } else { FormattingContext::Grid }; }
    if outer == 3 { return FormattingContext::Table; }
    if outer == 6 { return FormattingContext::Ruby; }
    if outer == 7 { return FormattingContext::None; }
    if outer == 8 { return FormattingContext::Contents; }
    match inner {
        1 => FormattingContext::Block,
        2 => FormattingContext::Inline,
        17 => FormattingContext::List,
        18 => FormattingContext::Math,
        3..=10 => FormattingContext::Table,
        _ => FormattingContext::Block,
    }
}

pub fn determine_layout_algorithm(ctx: FormattingContext) -> LayoutAlgorithm {
    match ctx {
        FormattingContext::Block => LayoutAlgorithm::BlockFlow,
        FormattingContext::Inline => LayoutAlgorithm::InlineFlow,
        FormattingContext::Flex => LayoutAlgorithm::FlexMainAxis,
        FormattingContext::Grid => LayoutAlgorithm::GridTracks,
        FormattingContext::Table => LayoutAlgorithm::TableGrid,
        FormattingContext::Ruby => LayoutAlgorithm::RubyBase,
        _ => LayoutAlgorithm::InlineFlow,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IntrinsicSizes { pub min_content: f64, pub max_content: f64, pub fit_content: f64, pub aspect_ratio: Option<f64> }

/// Resolve sizing keyword to computed length.
pub fn resolve_intrinsic_size(keyword: u8, intrinsic: IntrinsicSizes, available: f64) -> f64 {
    match keyword { 1 => intrinsic.min_content, 2 => intrinsic.max_content, 3 => intrinsic.fit_content.min(available), _ => available }
}

/// Resolve box-sizing → (content_w, content_h, used_w, used_h).
pub fn resolve_box_dimensions(sizing: u8, sw: f64, sh: f64, pb_w: f64, pb_h: f64) -> (f64, f64, f64, f64) {
    if sizing == 1 { (sw, sh, sw + pb_w, sh + pb_h) }
    else { ((sw - pb_w).max(0.0), (sh - pb_h).max(0.0), sw, sh) }
}
