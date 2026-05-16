use std::cell::RefCell;

use edgerun_ui_core::gpu::webgl2::{
    HIT_FLOAT_STRIDE, ICON_VERTEX_FLOAT_STRIDE, RECT_FLOAT_STRIDE, TEXT_VERTEX_FLOAT_STRIDE,
};
use edgerun_ui_core::gpu::{
    Color4, FontAtlas, UiAction, UiAppControl, UiEvent, UiKey, UiNode, UiRect,
    UiShadcnDemoGalleryState, UiSurfaceApp, UiSurfaceHost, build_shadcn_demo_gallery_with_state,
    lucide_svg_icon_atlas,
};

thread_local! {
    static HOST: RefCell<UiSurfaceHost<ShowcaseApp>> = RefCell::new(UiSurfaceHost::new(
        ShowcaseApp::default(),
        Color4::rgb_u8(248, 250, 252),
    ));
    static FONT: FontAtlas = FontAtlas::from_font_bytes(
        include_bytes!("../../../utility/edgerun-ui-core/assets/Inter.ttc"),
        18.0,
    )
    .expect("embedded Inter font should parse");
}

#[derive(Default)]
struct ShowcaseApp {
    shadcn: UiShadcnDemoGalleryState,
}

impl UiSurfaceApp for ShowcaseApp {
    fn surface(&mut self, _viewport: UiRect) -> UiNode {
        build_shadcn_demo_gallery_with_state(&self.shadcn)
    }

    fn handle_action(&mut self, action: UiAction) -> UiAppControl {
        if self.shadcn.apply_action(&action) {
            return UiAppControl::dirty();
        }

        match action {
            UiAction::Hovered(_)
            | UiAction::Focused(_)
            | UiAction::ScrollChanged { .. }
            | UiAction::OpenChanged { .. } => UiAppControl::dirty(),
            _ => UiAppControl::clean(),
        }
    }

    fn tick(&mut self, _delta_ms: u32) -> UiAppControl {
        UiAppControl::dirty()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_build_frame(width: f32, height: f32, delta_ms: u32) -> u32 {
    FONT.with(|font| {
        HOST.with_borrow_mut(|host| {
            let control = host.frame(delta_ms);
            host.render_with_font(font, width, height);
            control.dirty as u32
        })
    })
}

fn handle_event(event: UiEvent) -> u32 {
    HOST.with_borrow_mut(|host| host.handle_event(event).dirty as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_handle_pointer_down(x: f32, y: f32) -> u32 {
    handle_event(UiEvent::PointerDown { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_handle_pointer_move(x: f32, y: f32) -> u32 {
    handle_event(UiEvent::PointerMove { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_handle_pointer_up(x: f32, y: f32) -> u32 {
    handle_event(UiEvent::PointerUp { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_handle_wheel(x: f32, y: f32, delta_y: f32) -> u32 {
    handle_event(UiEvent::Wheel { x, y, delta_y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_handle_key(code: u32) -> u32 {
    handle_event(UiEvent::KeyDown {
        key: ui_key_from_web_key_code(code),
    })
}

fn ui_key_from_web_key_code(code: u32) -> UiKey {
    match code {
        8 => UiKey::Backspace,
        9 => UiKey::Tab,
        13 => UiKey::Enter,
        27 => UiKey::Escape,
        33 => UiKey::PageUp,
        34 => UiKey::PageDown,
        35 => UiKey::End,
        36 => UiKey::Home,
        37 => UiKey::ArrowLeft,
        38 => UiKey::ArrowUp,
        39 => UiKey::ArrowRight,
        40 => UiKey::ArrowDown,
        46 => UiKey::Delete,
        other => UiKey::Other(other),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_handle_blur() -> u32 {
    handle_event(UiEvent::Blur)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_rect_float_stride() -> u32 {
    RECT_FLOAT_STRIDE
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_text_vertex_float_stride() -> u32 {
    TEXT_VERTEX_FLOAT_STRIDE
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_icon_vertex_float_stride() -> u32 {
    ICON_VERTEX_FLOAT_STRIDE
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_hit_float_stride() -> u32 {
    HIT_FLOAT_STRIDE
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_rect_buffer_len() -> u32 {
    HOST.with_borrow(|host| host.packed_scene().rect_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_rect_buffer_ptr() -> *const f32 {
    HOST.with_borrow(|host| host.packed_scene().rect_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_text_vertex_buffer_len() -> u32 {
    HOST.with_borrow(|host| host.packed_scene().text_vertex_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_text_vertex_buffer_ptr() -> *const f32 {
    HOST.with_borrow(|host| host.packed_scene().text_vertex_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_icon_vertex_buffer_len() -> u32 {
    HOST.with_borrow(|host| host.packed_scene().icon_vertex_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_icon_vertex_buffer_ptr() -> *const f32 {
    HOST.with_borrow(|host| host.packed_scene().icon_vertex_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_hit_buffer_len() -> u32 {
    HOST.with_borrow(|host| host.packed_scene().hit_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_hit_buffer_ptr() -> *const f32 {
    HOST.with_borrow(|host| host.packed_scene().hit_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_font_atlas_width() -> u32 {
    FONT.with(|font| font.width)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_font_atlas_height() -> u32 {
    FONT.with(|font| font.height)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_font_atlas_ptr() -> *const u8 {
    FONT.with(|font| font.alpha.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_icon_atlas_width() -> u32 {
    lucide_svg_icon_atlas().width
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_icon_atlas_height() -> u32 {
    lucide_svg_icon_atlas().height
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_docs_icon_atlas_ptr() -> *const u8 {
    lucide_svg_icon_atlas().alpha.as_ptr()
}
