use std::cell::RefCell;

use edgerun_crypto::Ed25519SigningKey;
use edgerun_ui_core::gpu::{
    FontAtlas, GpuHit, GpuRect, GpuScene, HitKind, RectMode, TextQuad, UiAction, UiAppSurface,
    UiColorScheme, UiEvent, UiKey, UiShellAction, UiShellState, UiWorkProjection, UiWorkspace,
    UiWorkspaceAction, UnifiedChatState, build_edgerun_shell_overlay_with_font,
    build_edgerun_workspace_shell_with_font_and_work, palette,
};
use edgerun_work::{
    CHANNEL_KIND_WASM_HOST, ChannelEndpoint, DEPARTMENT_STORAGE, NODE_ROLE_ADMISSION,
    NODE_ROLE_RELAY, NODE_ROLE_STORAGE, ObjectStoreRequest, StoragePayload, WORK_TYPE_OBJECT_STORE,
    WORK_WIRE_ABI_VERSION, WorkAdmission, WorkPacket, WorkRequest, blake3_hash, empty_signature,
    encode_xor_2_1, node_identity_from_key, packet_hash, sign_work_admission, sign_work_request,
    storage_payload_bytes, verify_store_request, verify_work_admission, verify_work_request,
    work_request_preimage,
};

thread_local! {
    static SCENE: RefCell<GpuScene> = RefCell::new(GpuScene::new(palette::BG));
    static SHELL_SCENE: RefCell<GpuScene> = RefCell::new(GpuScene::new(edgerun_ui_core::gpu::Color4::rgba(0.0, 0.0, 0.0, 0.0)));
    static WORKSPACE: RefCell<UiWorkspace> = RefCell::new(default_workspace());
    static SHELL: RefCell<UiShellState> = RefCell::new(UiShellState::default());
    static PACKED_RECTS: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static PACKED_TEXT_VERTICES: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static PACKED_HITS: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static SHELL_PACKED_RECTS: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static SHELL_PACKED_TEXT_VERTICES: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static SHELL_PACKED_HITS: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static INPUT_BYTES: RefCell<Vec<u8>> = RefCell::new(vec![0; 4096]);
    static SELECTED_CONTACT: RefCell<usize> = const { RefCell::new(0) };
    static COLOR_SCHEME: RefCell<UiColorScheme> = const { RefCell::new(UiColorScheme::Dark) };
    static WORK_PROJECTION: RefCell<UiWorkProjection> = RefCell::new(build_protocol_projection());
    static FONT: FontAtlas = FontAtlas::from_font_bytes(include_bytes!(env!("CODEX_GL_INTER_FONT")), 18.0)
        .expect("embedded Inter font should parse");
}

fn default_workspace() -> UiWorkspace {
    UiWorkspace::edgerun_default()
}

fn lock_workspace() -> UiWorkspace {
    UiWorkspace::full_screen(UiAppSurface::lock_screen(10))
}

fn capability_request_workspace() -> UiWorkspace {
    UiWorkspace::full_screen(UiAppSurface::capability_request(11))
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
pub extern "C" fn codex_gl_show_workspace() {
    WORKSPACE.with_borrow_mut(|workspace| *workspace = default_workspace());
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_show_lock_screen() {
    WORKSPACE.with_borrow_mut(|workspace| *workspace = lock_workspace());
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_show_capability_request() {
    WORKSPACE.with_borrow_mut(|workspace| *workspace = capability_request_workspace());
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

fn build_protocol_projection() -> UiWorkProjection {
    let user_key = Ed25519SigningKey::from_bytes(&[7u8; 32]);
    let admission_key = Ed25519SigningKey::from_bytes(&[11u8; 32]);
    let relay_key = Ed25519SigningKey::from_bytes(&[13u8; 32]);
    let storage_key = Ed25519SigningKey::from_bytes(&[17u8; 32]);

    let admission_node = node_identity_from_key(&admission_key, NODE_ROLE_ADMISSION);
    let relay_node = node_identity_from_key(&relay_key, NODE_ROLE_RELAY);
    let storage_node = node_identity_from_key(&storage_key, NODE_ROLE_STORAGE);
    let user_public = public_key(&user_key);

    let (manifest, shards) = encode_xor_2_1(
        b"EdgeRun network app package bytes for UI protocol projection",
        [
            storage_node.node_id,
            relay_node.node_id,
            admission_node.node_id,
        ],
    )
    .expect("deterministic erasure fixture should encode");
    let store_request: ObjectStoreRequest =
        edgerun_work::store_request_from_shard(&manifest, &shards[0]);
    let storage_payload = StoragePayload::StoreRequest(store_request.clone());
    let payload_bytes =
        storage_payload_bytes(&storage_payload).expect("storage payload should encode");
    let payload_hash = blake3_hash(&payload_bytes);
    let input_root = edgerun_work::manifest_hash(&manifest);

    let request = sign_work_request(
        &user_key,
        WorkRequest {
            abi_version: WORK_WIRE_ABI_VERSION,
            request_id: blake3_hash(b"edgerun-ui-web:storage-request:v1"),
            user: user_public,
            user_sequence: 1,
            recipient: storage_node.node_id,
            work_type: WORK_TYPE_OBJECT_STORE,
            department: DEPARTMENT_STORAGE,
            payload_hash,
            input_root,
            max_total_cost: 48,
            valid_until_unix_ms: 1_893_456_000_000,
            signature: empty_signature(),
        },
    );
    let request_hash =
        packet_hash(&WorkPacket::WorkRequest(request.clone())).expect("request should hash");

    let route_commitment = blake3_hash(
        &[
            relay_node.node_id.as_slice(),
            storage_node.node_id.as_slice(),
            input_root.as_slice(),
        ]
        .concat(),
    );
    let assigned_channel = ChannelEndpoint::new(
        blake3_hash(b"edgerun-ui-web:wasm-host-channel:v1"),
        CHANNEL_KIND_WASM_HOST,
        b"browser://edgerun-work/ui-web".to_vec(),
        "browser wasm host".into(),
    );
    let policy_hash = blake3_hash(b"edgerun-ui-web:run-from-network-storage-policy:v1");
    let admission = sign_work_admission(
        &admission_key,
        WorkAdmission {
            abi_version: WORK_WIRE_ABI_VERSION,
            admission_id: blake3_hash(
                &[
                    admission_node.node_id.as_slice(),
                    request_hash.as_slice(),
                    &[1],
                ]
                .concat(),
            ),
            dao_id: admission_node.public_key,
            user: user_public,
            admission_node: admission_node.clone(),
            request_hash,
            assigned_route_commitment: route_commitment,
            assigned_channel: assigned_channel.clone(),
            assigned_relay_path: vec![relay_node.node_id],
            admitted_budget: 48,
            policy_hash,
            sequence: 1,
            valid_until_unix_ms: 1_893_456_000_000,
            signature: empty_signature(),
        },
    );
    let admission_hash =
        packet_hash(&WorkPacket::WorkAdmission(admission.clone())).expect("admission should hash");

    UiWorkProjection {
        browser_node: short_hash(&user_public),
        admission_node: short_hash(&admission_node.node_id),
        relay_node: short_hash(&relay_node.node_id),
        channel: short_hash(&assigned_channel.channel_id),
        policy_hash: short_hash(&policy_hash),
        request_hash: short_hash(&request_hash),
        admission_hash: short_hash(&admission_hash),
        route_commitment: short_hash(&route_commitment),
        storage_payload_hash: short_hash(&payload_hash),
        manifest_hash: short_hash(&input_root),
        admitted_budget: admission.admitted_budget,
        retrieval_cost: payload_bytes.len() as u64,
        request_verified: verify_work_request(&request)
            && blake3_hash(&work_request_preimage(&request)) != [0u8; 32],
        admission_verified: verify_work_admission(&admission),
        storage_payload_verified: verify_store_request(&store_request),
    }
}

fn public_key(key: &Ed25519SigningKey) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(key.verifying_key().as_bytes());
    out
}

fn short_hash(hash: &[u8; 32]) -> String {
    let hex = hash_hex(hash);
    format!("{}...{}", &hex[..10], &hex[hex.len() - 8..])
}

fn hash_hex(hash: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in hash {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_build_shell_frame(width: f32, height: f32, _time_ms: f64) -> u32 {
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

    PACKED_HITS.with_borrow_mut(|packed| {
        packed.clear();
        packed.reserve(scene.hits().len() * 6);
        for hit in scene.hits() {
            packed.extend_from_slice(&[
                hit_kind_code(hit.kind) as f32,
                (hit.id & 0x00ff_ffff) as f32,
                hit.x,
                hit.y,
                hit.w,
                hit.h,
            ]);
        }
    });
}

fn pack_shell_scene(scene: &GpuScene) {
    SHELL_PACKED_RECTS.with_borrow_mut(|packed| {
        packed.clear();
        packed.reserve(scene.rects().len() * 11);
        for rect in scene.rects() {
            push_packed_rect(packed, rect);
        }
    });

    SHELL_PACKED_TEXT_VERTICES.with_borrow_mut(|packed| {
        packed.clear();
        packed.reserve(scene.text_quads().len() * 48);
        for quad in scene.text_quads() {
            push_packed_text_quad(packed, quad);
        }
    });

    SHELL_PACKED_HITS.with_borrow_mut(|packed| {
        packed.clear();
        packed.reserve(scene.hits().len() * 6);
        for hit in scene.hits() {
            packed.extend_from_slice(&[
                hit_kind_code(hit.kind) as f32,
                (hit.id & 0x00ff_ffff) as f32,
                hit.x,
                hit.y,
                hit.w,
                hit.h,
            ]);
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
    let changed = handle_ui_event(UiEvent::PointerDown { x, y }) != 0;
    let changed = handle_ui_event(UiEvent::PointerUp { x, y }) != 0 || changed;
    changed as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_handle_pointer_down(x: f32, y: f32) -> u32 {
    handle_ui_event(UiEvent::PointerDown { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_handle_pointer_move(x: f32, y: f32) -> u32 {
    handle_ui_event(UiEvent::PointerMove { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_handle_pointer_up(x: f32, y: f32) -> u32 {
    handle_ui_event(UiEvent::PointerUp { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_handle_wheel(x: f32, y: f32, delta_y: f32) -> u32 {
    handle_ui_event(UiEvent::Wheel { x, y, delta_y })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_handle_key(code: u32) -> u32 {
    handle_ui_event(UiEvent::KeyDown {
        key: key_from_code(code),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_handle_blur() -> u32 {
    handle_ui_event(UiEvent::Blur)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_input_buffer_ptr() -> *mut u8 {
    INPUT_BYTES.with_borrow_mut(|bytes| bytes.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_input_buffer_capacity() -> u32 {
    INPUT_BYTES.with_borrow(|bytes| bytes.len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_handle_text_input(len: u32) -> u32 {
    let value = INPUT_BYTES.with_borrow(|bytes| {
        let len = (len as usize).min(bytes.len());
        core::str::from_utf8(&bytes[..len])
            .unwrap_or("")
            .to_string()
    });
    handle_ui_event(UiEvent::TextInput(value))
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
pub extern "C" fn codex_gl_handle_shell_pointer_down(x: f32, y: f32) -> u32 {
    handle_shell_event(UiEvent::PointerDown { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_handle_shell_pointer_move(x: f32, y: f32) -> u32 {
    handle_shell_event(UiEvent::PointerMove { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_handle_shell_pointer_up(x: f32, y: f32) -> u32 {
    handle_shell_event(UiEvent::PointerUp { x, y })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_handle_shell_key(code: u32) -> u32 {
    handle_shell_event(UiEvent::KeyDown {
        key: key_from_code(code),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_shell_hit_test(x: f32, y: f32) -> u32 {
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
    match action {
        UiShellAction::None => 0,
        UiShellAction::ToggledLauncher(_) => 1,
        UiShellAction::Runtime(action) => ui_action_dirty(action),
        UiShellAction::OpenApp { kind, .. } => {
            WORKSPACE.with_borrow_mut(|workspace| {
                workspace.open_or_focus(kind);
            });
            1
        }
    }
}

fn workspace_action_dirty(action: UiWorkspaceAction) -> u32 {
    match action {
        UiWorkspaceAction::None => 0,
        UiWorkspaceAction::AppAction { action, .. } => ui_action_dirty(action),
        UiWorkspaceAction::FocusedApp(_)
        | UiWorkspaceAction::ClosedApp(_)
        | UiWorkspaceAction::SplitRequested { .. } => 1,
    }
}

fn ui_action_dirty(action: UiAction) -> u32 {
    match action {
        UiAction::None | UiAction::Hovered(_) => 0,
        UiAction::Activated(hit) => activate_hit(hit),
        UiAction::TabSelected { .. }
        | UiAction::Toggled { .. }
        | UiAction::SliderChanged { .. }
        | UiAction::OpenChanged { .. }
        | UiAction::ScrollChanged { .. }
        | UiAction::TextChanged { .. }
        | UiAction::Focused(_)
        | UiAction::Submitted { .. }
        | UiAction::Cancelled => 1,
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

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_hit_float_stride() -> u32 {
    6
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_hit_buffer_len() -> u32 {
    PACKED_HITS.with_borrow(|packed| packed.len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_hit_buffer_ptr() -> *const f32 {
    PACKED_HITS.with_borrow(|packed| packed.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_shell_rect_buffer_len() -> u32 {
    SHELL_PACKED_RECTS.with_borrow(|packed| packed.len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_shell_rect_buffer_ptr() -> *const f32 {
    SHELL_PACKED_RECTS.with_borrow(|packed| packed.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_shell_text_vertex_buffer_len() -> u32 {
    SHELL_PACKED_TEXT_VERTICES.with_borrow(|packed| packed.len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_shell_text_vertex_buffer_ptr() -> *const f32 {
    SHELL_PACKED_TEXT_VERTICES.with_borrow(|packed| packed.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_shell_hit_buffer_len() -> u32 {
    SHELL_PACKED_HITS.with_borrow(|packed| packed.len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn codex_gl_shell_hit_buffer_ptr() -> *const f32 {
    SHELL_PACKED_HITS.with_borrow(|packed| packed.as_ptr())
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
        HitKind::Input => 8,
        HitKind::TextArea => 9,
        HitKind::Slider => 10,
        HitKind::MenuItem => 11,
        HitKind::TransactionRow => 12,
        HitKind::Scrollbar => 13,
        HitKind::WorkspaceTab => 14,
        HitKind::WorkspaceClose => 15,
        HitKind::WorkspaceSplit => 16,
        HitKind::Checkbox => 17,
        HitKind::Radio => 18,
        HitKind::Select => 19,
        HitKind::Breadcrumb => 20,
        HitKind::TreeItem => 21,
        HitKind::AppLauncherItem => 22,
        HitKind::ShellLauncher => 23,
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
