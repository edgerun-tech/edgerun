use std::cell::RefCell;

use edgerun_ui_core::gpu::webgl2::{
    HIT_FLOAT_STRIDE, PackedGpuScene, RECT_FLOAT_STRIDE, TEXT_VERTEX_FLOAT_STRIDE, hit_kind_code,
    rect_mode_code,
};
use edgerun_ui_core::gpu::{
    FontAtlas, GpuHit, GpuScene, HitKind, UiAction, UiColorScheme, UiEvent, UiKey, UiShellAction,
    UiShellState, UiWorkProjection, UiWorkspace, UiWorkspaceAction, UnifiedChatState,
    build_edgerun_shell_overlay_with_font, build_edgerun_workspace_shell_with_font_and_work,
    palette,
};

thread_local! {
    static SCENE: RefCell<GpuScene> = RefCell::new(GpuScene::new(palette::BG));
    static SHELL_SCENE: RefCell<GpuScene> = RefCell::new(GpuScene::new(edgerun_ui_core::gpu::Color4::rgba(0.0, 0.0, 0.0, 0.0)));
    static WORKSPACE: RefCell<UiWorkspace> = RefCell::new(default_workspace());
    static SHELL: RefCell<UiShellState> = RefCell::new(UiShellState::default());
    static PACKED_SCENE: RefCell<PackedGpuScene> = RefCell::new(PackedGpuScene::default());
    static SHELL_PACKED_SCENE: RefCell<PackedGpuScene> = RefCell::new(PackedGpuScene::default());
    static INPUT_BYTES: RefCell<Vec<u8>> = RefCell::new(vec![0; 4096]);
    static SELECTED_CONTACT: RefCell<usize> = const { RefCell::new(0) };
    static COLOR_SCHEME: RefCell<UiColorScheme> = const { RefCell::new(UiColorScheme::Dark) };
    static WORK_PROJECTION: RefCell<UiWorkProjection> = RefCell::new(UiWorkProjection::preview());
    static FONT: FontAtlas = FontAtlas::from_font_bytes(include_bytes!(env!("CODEX_GL_INTER_FONT")), 18.0)
        .expect("embedded Inter font should parse");
}

fn default_workspace() -> UiWorkspace {
    UiWorkspace::edgerun_default()
}

fn lock_workspace() -> UiWorkspace {
    UiWorkspace::edgerun_lock_screen()
}

fn capability_request_workspace() -> UiWorkspace {
    UiWorkspace::edgerun_capability_request()
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
pub extern "C" fn edgerun_frontend_set_color_scheme(code: u32) {
    COLOR_SCHEME.with_borrow_mut(|scheme| *scheme = UiColorScheme::from_code(code));
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_color_scheme() -> u32 {
    COLOR_SCHEME.with_borrow(|scheme| scheme.code())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_work_projection_schema_version() -> u32 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_clear_work_projection() {
    WORK_PROJECTION.with_borrow_mut(|projection| *projection = UiWorkProjection::preview());
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_set_work_projection_from_input(len: u32) -> u32 {
    let input = match input_buffer_string(len) {
        Some(input) => input,
        None => return 0,
    };
    match parse_work_projection(&input) {
        Some(projection) => {
            WORK_PROJECTION.with_borrow_mut(|stored| *stored = projection);
            1
        }
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_show_workspace() {
    WORKSPACE.with_borrow_mut(|workspace| *workspace = default_workspace());
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_show_lock_screen() {
    WORKSPACE.with_borrow_mut(|workspace| *workspace = lock_workspace());
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_show_capability_request() {
    WORKSPACE.with_borrow_mut(|workspace| *workspace = capability_request_workspace());
}

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

fn build_scene(width: f32, height: f32, active: bool) -> u32 {
    FONT.with(|font| {
        SCENE.with_borrow_mut(|scene| {
            let mut state = UnifiedChatState::empty();
            state.connected = active;
            WORKSPACE.with_borrow_mut(|workspace| {
                WORK_PROJECTION.with_borrow(|work| {
                    build_edgerun_workspace_shell_with_font_and_work(
                        scene,
                        font,
                        width,
                        height,
                        workspace,
                        &state,
                        Some(&work),
                    );
                });
            });
            let scheme = COLOR_SCHEME.with_borrow(|scheme| *scheme);
            scene.apply_color_scheme(scheme);
            pack_scene(scene);
            scene.rects().len() as u32
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
        SHELL_SCENE.with_borrow_mut(|scene| {
            SHELL.with_borrow_mut(|shell| {
                build_edgerun_shell_overlay_with_font(scene, font, width, height, shell);
            });
            pack_shell_scene(scene);
            scene.rects().len() as u32
        })
    })
}

fn pack_scene(scene: &GpuScene) {
    PACKED_SCENE.with_borrow_mut(|packed| packed.pack(scene));
}

fn pack_shell_scene(scene: &GpuScene) {
    SHELL_PACKED_SCENE.with_borrow_mut(|packed| packed.pack(scene));
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_selected_contact() -> u32 {
    SELECTED_CONTACT.with_borrow(|selected| *selected as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_set_selected_contact(index: u32) {
    SELECTED_CONTACT.with_borrow_mut(|selected| *selected = index as usize);
}

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
        key: key_from_code(code),
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

fn parse_work_projection(input: &str) -> Option<UiWorkProjection> {
    let mut projection = UiWorkProjection::preview();
    let mut saw_field = false;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('=')?;
        let key = key.trim();
        let value = value.trim();
        saw_field = true;

        match key {
            "local_node" => projection.local_node = value.into(),
            "admission_node" => projection.admission_node = value.into(),
            "relay_node" => projection.relay_node = value.into(),
            "channel" => projection.channel = value.into(),
            "policy_hash" => projection.policy_hash = value.into(),
            "request_hash" => projection.request_hash = value.into(),
            "admission_hash" => projection.admission_hash = value.into(),
            "route_commitment" => projection.route_commitment = value.into(),
            "storage_payload_hash" => projection.storage_payload_hash = value.into(),
            "manifest_hash" => projection.manifest_hash = value.into(),
            "admitted_budget" => projection.admitted_budget = value.parse().ok()?,
            "retrieval_cost" => projection.retrieval_cost = value.parse().ok()?,
            "request_verified" => projection.request_verified = parse_bool_field(value)?,
            "admission_verified" => projection.admission_verified = parse_bool_field(value)?,
            "storage_payload_verified" => {
                projection.storage_payload_verified = parse_bool_field(value)?;
            }
            _ => return None,
        }
    }

    saw_field.then_some(projection)
}

fn parse_bool_field(value: &str) -> Option<bool> {
    match value {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

fn handle_ui_event(event: UiEvent) -> u32 {
    SCENE.with_borrow(|scene| {
        WORKSPACE.with_borrow_mut(|workspace| {
            let action = workspace.handle_event(scene, event);
            workspace_action_dirty(action)
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_shell_pointer_down(x: f32, y: f32) -> u32 {
    handle_shell_event(UiEvent::PointerDown { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_shell_pointer_move(x: f32, y: f32) -> u32 {
    handle_shell_event(UiEvent::PointerMove { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_shell_pointer_up(x: f32, y: f32) -> u32 {
    handle_shell_event(UiEvent::PointerUp { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_handle_shell_key(code: u32) -> u32 {
    handle_shell_event(UiEvent::KeyDown {
        key: key_from_code(code),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_hit_test(x: f32, y: f32) -> u32 {
    SHELL_SCENE.with_borrow(|scene| {
        scene
            .hit_test(x, y)
            .map(|hit| (hit_kind_code(hit.kind) << 24) | (hit.id & 0x00ff_ffff))
            .unwrap_or(u32::MAX)
    })
}

fn handle_shell_event(event: UiEvent) -> u32 {
    SHELL_SCENE.with_borrow(|scene| {
        SHELL.with_borrow_mut(|shell| {
            let action = shell.handle_event(scene, event);
            shell_action_dirty(action)
        })
    })
}

fn shell_action_dirty(action: UiShellAction) -> u32 {
    WORKSPACE.with_borrow_mut(|workspace| action.apply_to_workspace(workspace) as u32)
}

fn workspace_action_dirty(action: UiWorkspaceAction) -> u32 {
    match action {
        UiWorkspaceAction::AppAction {
            action: UiAction::Activated(hit),
            ..
        } if matches!(hit.kind, HitKind::Contact) => activate_hit(hit),
        other => other.needs_redraw() as u32,
    }
}

fn activate_hit(hit: GpuHit) -> u32 {
    if !matches!(hit.kind, HitKind::Contact) {
        return 1;
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
}

fn key_from_code(code: u32) -> UiKey {
    match code {
        8 => UiKey::Backspace,
        9 => UiKey::Tab,
        13 => UiKey::Enter,
        27 => UiKey::Escape,
        37 => UiKey::ArrowLeft,
        38 => UiKey::ArrowUp,
        39 => UiKey::ArrowRight,
        40 => UiKey::ArrowDown,
        other => UiKey::Other(other),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_hit_test(x: f32, y: f32) -> u32 {
    SCENE.with_borrow(|scene| {
        scene
            .hit_test(x, y)
            .map(|hit| (hit_kind_code(hit.kind) << 24) | (hit.id & 0x00ff_ffff))
            .unwrap_or(u32::MAX)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_hit_count() -> u32 {
    SCENE.with_borrow(|scene| scene.hits().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_hit_float_stride() -> u32 {
    HIT_FLOAT_STRIDE
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_hit_buffer_len() -> u32 {
    PACKED_SCENE.with_borrow(|packed| packed.hit_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_hit_buffer_ptr() -> *const f32 {
    PACKED_SCENE.with_borrow(|packed| packed.hit_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_rect_buffer_len() -> u32 {
    SHELL_PACKED_SCENE.with_borrow(|packed| packed.rect_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_rect_buffer_ptr() -> *const f32 {
    SHELL_PACKED_SCENE.with_borrow(|packed| packed.rect_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_text_vertex_buffer_len() -> u32 {
    SHELL_PACKED_SCENE.with_borrow(|packed| packed.text_vertex_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_text_vertex_buffer_ptr() -> *const f32 {
    SHELL_PACKED_SCENE.with_borrow(|packed| packed.text_vertex_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_hit_buffer_len() -> u32 {
    SHELL_PACKED_SCENE.with_borrow(|packed| packed.hit_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_shell_hit_buffer_ptr() -> *const f32 {
    SHELL_PACKED_SCENE.with_borrow(|packed| packed.hit_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_quad_count() -> u32 {
    SCENE.with_borrow(|scene| scene.text_quads().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_float_stride() -> u32 {
    RECT_FLOAT_STRIDE
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_buffer_len() -> u32 {
    PACKED_SCENE.with_borrow(|packed| packed.rect_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_rect_buffer_ptr() -> *const f32 {
    PACKED_SCENE.with_borrow(|packed| packed.rect_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_vertex_float_stride() -> u32 {
    TEXT_VERTEX_FLOAT_STRIDE
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_vertex_buffer_len() -> u32 {
    PACKED_SCENE.with_borrow(|packed| packed.text_vertex_buffer().len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_text_vertex_buffer_ptr() -> *const f32 {
    PACKED_SCENE.with_borrow(|packed| packed.text_vertex_buffer().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_clear_r() -> f32 {
    SCENE.with_borrow(|scene| scene.clear.r)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_clear_g() -> f32 {
    SCENE.with_borrow(|scene| scene.clear.g)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_clear_b() -> f32 {
    SCENE.with_borrow(|scene| scene.clear.b)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_frontend_clear_a() -> f32 {
    SCENE.with_borrow(|scene| scene.clear.a)
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
    SCENE.with_borrow(|scene| {
        scene
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

fn frame_active(time_ms: f64) -> bool {
    ((time_ms.max(0.0) as u64) / 800).is_multiple_of(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_host_supplied_work_projection() {
        let projection = parse_work_projection(
            "\
local_node=wasm:alice:storage
admission_node=admission:dao
relay_node=relay:public
channel=channel:7
policy_hash=policy_abc
request_hash=request_abc
admission_hash=admission_abc
route_commitment=route_abc
storage_payload_hash=payload_abc
manifest_hash=manifest_abc
admitted_budget=1200
retrieval_cost=15
request_verified=true
admission_verified=1
storage_payload_verified=yes
",
        )
        .expect("valid projection should parse");

        assert_eq!(projection.local_node, "wasm:alice:storage");
        assert_eq!(projection.admitted_budget, 1200);
        assert_eq!(projection.retrieval_cost, 15);
        assert!(projection.request_verified);
        assert!(projection.admission_verified);
        assert!(projection.storage_payload_verified);
    }

    #[test]
    fn rejects_unknown_projection_fields() {
        assert!(parse_work_projection("made_up=true").is_none());
    }
}
