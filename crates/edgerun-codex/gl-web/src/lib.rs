use std::cell::RefCell;

use edgerun_ui_core::gpu::{
    FontAtlas, GpuRect, GpuScene, HitKind, RectMode, TextQuad, UiColorScheme, UnifiedChatState,
    build_unified_chat_shell_with_font, palette,
};

thread_local! {
    static SCENE: RefCell<GpuScene> = RefCell::new(GpuScene::new(palette::BG));
    static PACKED_RECTS: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static PACKED_TEXT_VERTICES: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static SELECTED_CONTACT: RefCell<usize> = const { RefCell::new(0) };
    static COLOR_SCHEME: RefCell<UiColorScheme> = const { RefCell::new(UiColorScheme::Dark) };
    static FONT: FontAtlas = FontAtlas::from_font_bytes(include_bytes!(env!("CODEX_GL_INTER_FONT")), 18.0)
        .expect("embedded Inter font should parse");
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_build_scene(width: f32, height: f32, thinking: u32) -> u32 {
    build_scene(width, height, thinking != 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_build_frame(width: f32, height: f32, time_ms: f64) -> u32 {
    build_scene(width, height, frame_active(time_ms))
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_set_color_scheme(code: u32) {
    COLOR_SCHEME.with_borrow_mut(|scheme| *scheme = UiColorScheme::from_code(code));
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_color_scheme() -> u32 {
    COLOR_SCHEME.with_borrow(|scheme| scheme.code())
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_build_codex_scene(width: f32, height: f32, thinking: u32) -> u32 {
    build_scene(width, height, thinking != 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_build_unified_chat_scene(
    width: f32,
    height: f32,
    connected: u32,
) -> u32 {
    build_scene(width, height, connected != 0)
}

fn build_scene(width: f32, height: f32, active: bool) -> u32 {
    FONT.with(|font| {
        SCENE.with_borrow_mut(|scene| {
            let mut state = UnifiedChatState::empty();
            state.connected = active;
            build_unified_chat_shell_with_font(scene, font, width, height, &state);
            let scheme = COLOR_SCHEME.with_borrow(|scheme| *scheme);
            scene.apply_color_scheme(scheme);
            pack_scene(scene);
            scene.rects().len() as u32
        })
    })
}

fn pack_scene(scene: &GpuScene) {
    PACKED_RECTS.with_borrow_mut(|packed| {
        packed.clear();
        packed.reserve(scene.rects().len() * 11);
        for rect in scene.rects() {
            push_packed_rect(packed, rect);
        }
    });

    PACKED_TEXT_VERTICES.with_borrow_mut(|packed| {
        packed.clear();
        packed.reserve(scene.text_quads().len() * 48);
        for quad in scene.text_quads() {
            push_packed_text_quad(packed, quad);
        }
    });
}

fn push_packed_rect(packed: &mut Vec<f32>, rect: &GpuRect) {
    packed.extend_from_slice(&[
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        rect.radius,
        rect.shadow,
        rect.color.r,
        rect.color.g,
        rect.color.b,
        rect.color.a,
        rect_mode_code(rect.mode) as f32,
    ]);
}

fn push_packed_text_quad(packed: &mut Vec<f32>, quad: &TextQuad) {
    push_text_vertex(packed, quad.x, quad.y, quad.u0, quad.v0, quad);
    push_text_vertex(packed, quad.x + quad.w, quad.y, quad.u1, quad.v0, quad);
    push_text_vertex(
        packed,
        quad.x + quad.w,
        quad.y + quad.h,
        quad.u1,
        quad.v1,
        quad,
    );
    push_text_vertex(packed, quad.x, quad.y, quad.u0, quad.v0, quad);
    push_text_vertex(
        packed,
        quad.x + quad.w,
        quad.y + quad.h,
        quad.u1,
        quad.v1,
        quad,
    );
    push_text_vertex(packed, quad.x, quad.y + quad.h, quad.u0, quad.v1, quad);
}

fn push_text_vertex(packed: &mut Vec<f32>, x: f32, y: f32, u: f32, v: f32, quad: &TextQuad) {
    packed.extend_from_slice(&[
        x,
        y,
        u,
        v,
        quad.color.r,
        quad.color.g,
        quad.color.b,
        quad.color.a,
    ]);
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_selected_contact() -> u32 {
    SELECTED_CONTACT.with_borrow(|selected| *selected as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_set_selected_contact(index: u32) {
    SELECTED_CONTACT.with_borrow_mut(|selected| *selected = index as usize);
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_handle_pointer(x: f32, y: f32) -> u32 {
    SCENE.with_borrow(|scene| {
        let Some(hit) = scene.hit_test(x, y) else {
            return 0;
        };
        if !matches!(hit.kind, HitKind::Contact) {
            return 0;
        }
        SELECTED_CONTACT.with_borrow_mut(|selected| {
            let next = hit.id as usize;
            if *selected == next {
                0
            } else {
                *selected = next;
                1
            }
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_hit_test(x: f32, y: f32) -> u32 {
    SCENE.with_borrow(|scene| {
        scene
            .hit_test(x, y)
            .map(|hit| (hit_kind_code(hit.kind) << 24) | (hit.id & 0x00ff_ffff))
            .unwrap_or(u32::MAX)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_hit_count() -> u32 {
    SCENE.with_borrow(|scene| scene.hits().len() as u32)
}

fn hit_kind_code(kind: HitKind) -> u32 {
    match kind {
        HitKind::Contact => 1,
        HitKind::Composer => 2,
        HitKind::Send => 3,
        HitKind::Button => 4,
        HitKind::Tab => 5,
        HitKind::Toggle => 6,
        HitKind::ListRow => 7,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_quad_count() -> u32 {
    SCENE.with_borrow(|scene| scene.text_quads().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_float_stride() -> u32 {
    11
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_buffer_len() -> u32 {
    PACKED_RECTS.with_borrow(|packed| packed.len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_rect_buffer_ptr() -> *const f32 {
    PACKED_RECTS.with_borrow(|packed| packed.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_vertex_float_stride() -> u32 {
    8
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_vertex_buffer_len() -> u32 {
    PACKED_TEXT_VERTICES.with_borrow(|packed| packed.len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_text_vertex_buffer_ptr() -> *const f32 {
    PACKED_TEXT_VERTICES.with_borrow(|packed| packed.as_ptr())
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
            .map(|rect| rect_mode_code(rect.mode))
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

fn rect_mode_code(mode: RectMode) -> u32 {
    match mode {
        RectMode::Fill => 0,
        RectMode::Shadow => 1,
        RectMode::Border => 2,
    }
}

fn frame_active(time_ms: f64) -> bool {
    ((time_ms.max(0.0) as u64) / 800).is_multiple_of(2)
}
