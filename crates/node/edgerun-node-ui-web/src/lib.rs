use std::cell::RefCell;

use edgerun_ui_core::gpu::webgl2::{
    hit_kind_code, rect_mode_code, HIT_FLOAT_STRIDE, ICON_VERTEX_FLOAT_STRIDE,
    RECT_FLOAT_STRIDE, TEXT_VERTEX_FLOAT_STRIDE,
};
use edgerun_ui_core::gpu::{
    Color4, FontAtlas, UiAction, UiAppControl, UiColorScheme, UiEvent, UiKey, UiNode, UiRect,
    UiSurfaceApp, UiSurfaceHost, column, text,
};

thread_local! {
    static HOST: RefCell<UiSurfaceHost<FrontendApp>> = RefCell::new(UiSurfaceHost::new(
        FrontendApp,
        Color4::rgb_u8(18, 18, 22),
    ));
    static INPUT_BYTES: RefCell<Vec<u8>> = RefCell::new(vec![0; 4096]);
    static FONT: FontAtlas = FontAtlas::from_font_bytes(
        include_bytes!("../../../utility/edgerun-ui-core/assets/Inter.ttc"),
        18.0,
    )
    .expect("embedded Inter font should parse");
}

struct FrontendApp;

impl UiSurfaceApp for FrontendApp {
    fn surface(&mut self, _viewport: UiRect) -> UiNode {
        column("bg-bg h-full w-full").child(text("EdgeRun"))
    }

    fn handle_action(&mut self, action: UiAction) -> UiAppControl {
        match action {
            UiAction::Hovered(_) | UiAction::Focused(_) | UiAction::Activated(_) => {
                UiAppControl::dirty()
            }
            _ => UiAppControl::clean(),
        }
    }

    fn tick(&mut self, _delta_ms: u32) -> UiAppControl {
        UiAppControl::clean()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_build_scene(width: f32, height: f32, thinking: u32) -> u32 {
    build_scene(width, height, thinking != 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_build_frame(width: f32, height: f32, time_ms: f64) -> u32 {
    build_scene(width, height, frame_active(time_ms))
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_set_color_scheme(_code: u32) {
    // Color scheme is managed through UiSurfaceHost clear color on init.
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_color_scheme() -> u32 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_work_projection_schema_version() -> u32 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_clear_work_projection() {}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_set_work_projection_from_input(_len: u32) -> u32 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_show_workspace() {}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_show_lock_screen() {}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_show_capability_request() {}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_build_agent_scene(
    width: f32,
    height: f32,
    thinking: u32,
) -> u32 {
    build_scene(width, height, thinking != 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_build_unified_chat_scene(
    width: f32,
    height: f32,
    connected: u32,
) -> u32 {
    build_scene(width, height, connected != 0)
}

fn build_scene(width: f32, height: f32, _active: bool) -> u32 {
    FONT.with(|font| {
        HOST.with_borrow_mut(|host| {
            let control = host.frame(0);
            host.render_with_font(font, width, height);
            control.dirty as u32
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_build_shell_frame(
    width: f32,
    height: f32,
    _time_ms: f64,
) -> u32 {
    FONT.with(|font| {
        HOST.with_borrow_mut(|host| {
            host.render_with_font(font, width, height);
            0
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_selected_contact() -> u32 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_set_selected_contact(_index: u32) {}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_pointer(x: f32, y: f32) -> u32 {
    let changed = handle_ui_event(UiEvent::PointerDown { x, y }) != 0;
    let changed = handle_ui_event(UiEvent::PointerUp { x, y }) != 0 || changed;
    changed as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_pointer_down(x: f32, y: f32) -> u32 {
    handle_ui_event(UiEvent::PointerDown { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_pointer_move(x: f32, y: f32) -> u32 {
    handle_ui_event(UiEvent::PointerMove { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_pointer_up(x: f32, y: f32) -> u32 {
    handle_ui_event(UiEvent::PointerUp { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_wheel(x: f32, y: f32, delta_y: f32) -> u32 {
    handle_ui_event(UiEvent::Wheel { x, y, delta_y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_key(code: u32) -> u32 {
    handle_ui_event(UiEvent::KeyDown {
        key: ui_key_from_web_key_code(code),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_blur() -> u32 {
    handle_ui_event(UiEvent::Blur)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_input_buffer_ptr() -> *mut u8 {
    INPUT_BYTES.with_borrow_mut(|bytes| bytes.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_input_buffer_capacity() -> u32 {
    INPUT_BYTES.with_borrow(|bytes| bytes.len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_text_input(len: u32) -> u32 {
    let value = input_buffer_string(len).unwrap_or_default();
    handle_ui_event(UiEvent::TextInput(value))
}

fn input_buffer_string(len: u32) -> Option<String> {
    INPUT_BYTES.with_borrow(|bytes| {
        let len = (len as usize).min(bytes.len());
        core::str::from_utf8(&bytes[..len])
            .ok()
            .map(ToString::to_string)
    })
}

fn handle_ui_event(event: UiEvent) -> u32 {
    HOST.with_borrow_mut(|host| host.handle_event(event).dirty as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_shell_pointer_down(x: f32, y: f32) -> u32 {
    handle_ui_event(UiEvent::PointerDown { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_shell_pointer_move(x: f32, y: f32) -> u32 {
    handle_ui_event(UiEvent::PointerMove { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_shell_pointer_up(x: f32, y: f32) -> u32 {
    handle_ui_event(UiEvent::PointerUp { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_shell_key(code: u32) -> u32 {
    handle_ui_event(UiEvent::KeyDown {
        key: ui_key_from_web_key_code(code),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_hit_test(x: f32, y: f32) -> u32 {
    HOST.with_borrow(|host| {
        host.scene()
            .hit_test(x, y)
            .map(|hit| (hit_kind_code(hit.kind) << 24) | (hit.id & 0x00ff_ffff))
            .unwrap_or(u32::MAX)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_hit_test(x: f32, y: f32) -> u32 {
    HOST.with_borrow(|host| {
        host.scene()
            .hit_test(x, y)
            .map(|hit| (hit_kind_code(hit.kind) << 24) | (hit.id & 0x00ff_ffff))
            .unwrap_or(u32::MAX)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_hit_count() -> u32 {
    HOST.with_borrow(|host| host.scene().hits().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_hit_float_stride() -> u32 {
    HIT_FLOAT_STRIDE
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_hit_buffer_len() -> u32 {
    HOST.with_borrow(|host| host.packed_scene().hit_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_hit_buffer_ptr() -> *const f32 {
    HOST.with_borrow(|host| host.packed_scene().hit_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_rect_buffer_len() -> u32 {
    HOST.with_borrow(|host| host.packed_scene().rect_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_rect_buffer_ptr() -> *const f32 {
    HOST.with_borrow(|host| host.packed_scene().rect_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_text_vertex_buffer_len() -> u32 {
    HOST.with_borrow(|host| host.packed_scene().text_vertex_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_text_vertex_buffer_ptr() -> *const f32 {
    HOST.with_borrow(|host| host.packed_scene().text_vertex_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_hit_buffer_len() -> u32 {
    HOST.with_borrow(|host| host.packed_scene().hit_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_hit_buffer_ptr() -> *const f32 {
    HOST.with_borrow(|host| host.packed_scene().hit_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_quad_count() -> u32 {
    HOST.with_borrow(|host| host.scene().text_quads().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_float_stride() -> u32 {
    RECT_FLOAT_STRIDE
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_buffer_len() -> u32 {
    HOST.with_borrow(|host| host.packed_scene().rect_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_buffer_ptr() -> *const f32 {
    HOST.with_borrow(|host| host.packed_scene().rect_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_vertex_float_stride() -> u32 {
    TEXT_VERTEX_FLOAT_STRIDE
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_vertex_buffer_len() -> u32 {
    HOST.with_borrow(|host| host.packed_scene().text_vertex_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_vertex_buffer_ptr() -> *const f32 {
    HOST.with_borrow(|host| host.packed_scene().text_vertex_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_clear_r() -> f32 {
    HOST.with_borrow(|host| host.scene().clear.r)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_clear_g() -> f32 {
    HOST.with_borrow(|host| host.scene().clear.g)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_clear_b() -> f32 {
    HOST.with_borrow(|host| host.scene().clear.b)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_clear_a() -> f32 {
    HOST.with_borrow(|host| host.scene().clear.a)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_x(index: u32) -> f32 {
    rect_field(index, |rect| rect.x)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_y(index: u32) -> f32 {
    rect_field(index, |rect| rect.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_w(index: u32) -> f32 {
    rect_field(index, |rect| rect.w)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_h(index: u32) -> f32 {
    rect_field(index, |rect| rect.h)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_radius(index: u32) -> f32 {
    rect_field(index, |rect| rect.radius)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_shadow(index: u32) -> f32 {
    rect_field(index, |rect| rect.shadow)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_mode(index: u32) -> u32 {
    HOST.with_borrow(|host| {
        host.scene()
            .rects()
            .get(index as usize)
            .map(|rect| rect_mode_code(rect.mode))
            .unwrap_or(0)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_r(index: u32) -> f32 {
    rect_field(index, |rect| rect.color.r)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_g(index: u32) -> f32 {
    rect_field(index, |rect| rect.color.g)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_b(index: u32) -> f32 {
    rect_field(index, |rect| rect.color.b)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_a(index: u32) -> f32 {
    rect_field(index, |rect| rect.color.a)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_font_atlas_width() -> u32 {
    FONT.with(|font| font.width)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_font_atlas_height() -> u32 {
    FONT.with(|font| font.height)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_font_atlas_ptr() -> *const u8 {
    FONT.with(|font| font.alpha.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_x(index: u32) -> f32 {
    text_field(index, |quad| quad.x)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_y(index: u32) -> f32 {
    text_field(index, |quad| quad.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_w(index: u32) -> f32 {
    text_field(index, |quad| quad.w)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_h(index: u32) -> f32 {
    text_field(index, |quad| quad.h)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_u0(index: u32) -> f32 {
    text_field(index, |quad| quad.u0)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_v0(index: u32) -> f32 {
    text_field(index, |quad| quad.v0)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_u1(index: u32) -> f32 {
    text_field(index, |quad| quad.u1)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_v1(index: u32) -> f32 {
    text_field(index, |quad| quad.v1)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_r(index: u32) -> f32 {
    text_field(index, |quad| quad.color.r)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_g(index: u32) -> f32 {
    text_field(index, |quad| quad.color.g)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_b(index: u32) -> f32 {
    text_field(index, |quad| quad.color.b)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_a(index: u32) -> f32 {
    text_field(index, |quad| quad.color.a)
}

fn rect_field(index: u32, field: impl FnOnce(&edgerun_ui_core::gpu::GpuRect) -> f32) -> f32 {
    HOST.with_borrow(|host| {
        host.scene()
            .rects()
            .get(index as usize)
            .map(field)
            .unwrap_or(0.0)
    })
}

fn text_field(index: u32, field: impl FnOnce(&edgerun_ui_core::gpu::TextQuad) -> f32) -> f32 {
    HOST.with_borrow(|host| {
        host.scene()
            .text_quads()
            .get(index as usize)
            .map(field)
            .unwrap_or(0.0)
    })
}

fn frame_active(time_ms: f64) -> bool {
    ((time_ms.max(0.0) as u64) / 800).is_multiple_of(2)
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
