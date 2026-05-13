use std::cell::RefCell;

use edgerun_ui_core::gpu::{
    FontAtlas, GpuScene, RectMode, UnifiedChatState, build_codex_chat_shell_with_font,
    build_unified_chat_shell_with_font, palette,
};

thread_local! {
    static SCENE: RefCell<GpuScene> = RefCell::new(GpuScene::new(palette::BG));
    static FONT: FontAtlas = FontAtlas::from_font_bytes(include_bytes!(env!("CODEX_GL_INTER_FONT")), 18.0)
        .expect("embedded Inter font should parse");
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_build_scene(width: f32, height: f32, thinking: u32) -> u32 {
    FONT.with(|font| {
        SCENE.with_borrow_mut(|scene| {
            build_scene(
                scene,
                font,
                width,
                height,
                thinking != 0,
                Surface::UnifiedChat,
            );
            scene.rects().len() as u32
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_build_codex_scene(width: f32, height: f32, thinking: u32) -> u32 {
    FONT.with(|font| {
        SCENE.with_borrow_mut(|scene| {
            build_scene(scene, font, width, height, thinking != 0, Surface::Codex);
            scene.rects().len() as u32
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_build_unified_chat_scene(
    width: f32,
    height: f32,
    connected: u32,
) -> u32 {
    FONT.with(|font| {
        SCENE.with_borrow_mut(|scene| {
            build_scene(
                scene,
                font,
                width,
                height,
                connected != 0,
                Surface::UnifiedChat,
            );
            scene.rects().len() as u32
        })
    })
}

#[derive(Clone, Copy)]
enum Surface {
    Codex,
    UnifiedChat,
}

fn build_scene(
    scene: &mut GpuScene,
    font: &FontAtlas,
    width: f32,
    height: f32,
    active: bool,
    surface: Surface,
) {
    match surface {
        Surface::Codex => build_codex_chat_shell_with_font(scene, font, width, height, active),
        Surface::UnifiedChat => {
            let state = UnifiedChatState::demo(active);
            build_unified_chat_shell_with_font(scene, font, width, height, &state);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_quad_count() -> u32 {
    SCENE.with_borrow(|scene| scene.text_quads().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_clear_r() -> f32 {
    SCENE.with_borrow(|scene| scene.clear.r)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_clear_g() -> f32 {
    SCENE.with_borrow(|scene| scene.clear.g)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_clear_b() -> f32 {
    SCENE.with_borrow(|scene| scene.clear.b)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_clear_a() -> f32 {
    SCENE.with_borrow(|scene| scene.clear.a)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_x(index: u32) -> f32 {
    rect_field(index, |rect| rect.x)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_y(index: u32) -> f32 {
    rect_field(index, |rect| rect.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_w(index: u32) -> f32 {
    rect_field(index, |rect| rect.w)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_h(index: u32) -> f32 {
    rect_field(index, |rect| rect.h)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_radius(index: u32) -> f32 {
    rect_field(index, |rect| rect.radius)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_shadow(index: u32) -> f32 {
    rect_field(index, |rect| rect.shadow)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_mode(index: u32) -> u32 {
    SCENE.with_borrow(|scene| {
        scene
            .rects()
            .get(index as usize)
            .map(|rect| match rect.mode {
                RectMode::Fill => 0,
                RectMode::Shadow => 1,
                RectMode::Border => 2,
            })
            .unwrap_or(0)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_r(index: u32) -> f32 {
    rect_field(index, |rect| rect.color.r)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_g(index: u32) -> f32 {
    rect_field(index, |rect| rect.color.g)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_b(index: u32) -> f32 {
    rect_field(index, |rect| rect.color.b)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_a(index: u32) -> f32 {
    rect_field(index, |rect| rect.color.a)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_font_atlas_width() -> u32 {
    FONT.with(|font| font.width)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_font_atlas_height() -> u32 {
    FONT.with(|font| font.height)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_font_atlas_ptr() -> *const u8 {
    FONT.with(|font| font.alpha.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_x(index: u32) -> f32 {
    text_field(index, |quad| quad.x)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_y(index: u32) -> f32 {
    text_field(index, |quad| quad.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_w(index: u32) -> f32 {
    text_field(index, |quad| quad.w)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_h(index: u32) -> f32 {
    text_field(index, |quad| quad.h)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_u0(index: u32) -> f32 {
    text_field(index, |quad| quad.u0)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_v0(index: u32) -> f32 {
    text_field(index, |quad| quad.v0)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_u1(index: u32) -> f32 {
    text_field(index, |quad| quad.u1)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_v1(index: u32) -> f32 {
    text_field(index, |quad| quad.v1)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_r(index: u32) -> f32 {
    text_field(index, |quad| quad.color.r)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_g(index: u32) -> f32 {
    text_field(index, |quad| quad.color.g)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_b(index: u32) -> f32 {
    text_field(index, |quad| quad.color.b)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_a(index: u32) -> f32 {
    text_field(index, |quad| quad.color.a)
}

fn rect_field(index: u32, field: impl FnOnce(&edgerun_ui_core::gpu::GpuRect) -> f32) -> f32 {
    SCENE.with_borrow(|scene| scene.rects().get(index as usize).map(field).unwrap_or(0.0))
}

fn text_field(index: u32, field: impl FnOnce(&edgerun_ui_core::gpu::TextQuad) -> f32) -> f32 {
    SCENE.with_borrow(|scene| {
        scene
            .text_quads()
            .get(index as usize)
            .map(field)
            .unwrap_or(0.0)
    })
}
