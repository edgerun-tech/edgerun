//! Paint command builder — generated from CSS proto data.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py
extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use crate::render_object::Color;

#[derive(Debug, Clone, Copy)]
pub struct Rect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }

#[derive(Debug, Clone)]
pub enum PaintCommand {
    FillRect { rect: Rect, color: Color },
    StrokeRect { rect: Rect, color: Color, width: f64 },
    DrawText { rect: Rect, text: String, color: Color },
    PushClip, PopClip,
    PushTransform { matrix: [f64; 16] }, PopTransform,
    PushOpacity { alpha: f32 }, PopOpacity,
}

pub fn build_display_list(root: &crate::render_object::RenderObject) -> Vec<PaintCommand> {
    let mut cmds = Vec::new();
    build_inner(root, &mut cmds);
    cmds
}

fn build_inner(obj: &crate::render_object::RenderObject, out: &mut Vec<PaintCommand>) {
    match obj {
        crate::render_object::RenderObject::BlockContainer { children, style }
        | crate::render_object::RenderObject::FlexContainer { children, style }
        | crate::render_object::RenderObject::GridContainer { children, style }
        | crate::render_object::RenderObject::InlineContainer { children, style } => {
            if let Some(c) = style.background_color {
                out.push(PaintCommand::FillRect { rect: Rect{x:0.0,y:0.0,width:0.0,height:0.0}, color: c });
            }
            if style.border_width > 0.0 {
                out.push(PaintCommand::StrokeRect { rect: Rect{x:0.0,y:0.0,width:0.0,height:0.0}, color: style.border_color, width: style.border_width });
            }
            for c in children { build_inner(c, out); }
        }
        crate::render_object::RenderObject::TextRun { text, style, .. } => {
            out.push(PaintCommand::DrawText { rect: Rect{x:0.0,y:0.0,width:0.0,height:0.0}, text: text.clone(), color: style.color });
        }
        crate::render_object::RenderObject::Image { style, .. } => {
            if let Some(c) = style.background_color {
                out.push(PaintCommand::FillRect { rect: Rect{x:0.0,y:0.0,width:0.0,height:0.0}, color: c });
            }
        }
    }
}
