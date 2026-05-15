use super::webgl2::PackedGpuScene;
use super::{
    build_edgerun_shell_overlay_with_font, build_edgerun_workspace_shell_with_font_and_work,
    build_edgerun_workspace_with_shell_with_font, palette, Color4, FontAtlas, GpuHit, GpuScene,
    HitKind, UiAction, UiColorScheme, UiEvent, UiKey, UiShellAction, UiShellState,
    UiWorkProjection, UiWorkspace, UiWorkspaceAction, UnifiedChatState,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiShellSurfacePreset {
    Workspace,
    #[default]
    Codex,
    Lock,
    Capability,
    Gallery,
}

impl UiShellSurfacePreset {
    pub fn workspace(self) -> UiWorkspace {
        match self {
            Self::Workspace | Self::Codex => UiWorkspace::edgerun_default(),
            Self::Lock => UiWorkspace::edgerun_lock_screen(),
            Self::Capability => UiWorkspace::edgerun_capability_request(),
            Self::Gallery => UiWorkspace::edgerun_component_gallery(),
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Workspace => "EdgeRun Unified Chat",
            Self::Codex => "EdgeRun Codex",
            Self::Lock => "EdgeRun Lock",
            Self::Capability => "EdgeRun Capability",
            Self::Gallery => "EdgeRun UI Gallery",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "codex" => Some(Self::Codex),
            "workspace" => Some(Self::Workspace),
            "lock" => Some(Self::Lock),
            "capability" => Some(Self::Capability),
            "gallery" => Some(Self::Gallery),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UiHostSession {
    pub scene: GpuScene,
    pub shell_scene: GpuScene,
    pub packed_scene: PackedGpuScene,
    pub shell_packed_scene: PackedGpuScene,
    pub workspace: UiWorkspace,
    pub shell: UiShellState,
    pub color_scheme: UiColorScheme,
    pub work_projection: UiWorkProjection,
    pub selected_contact: usize,
}

impl Default for UiHostSession {
    fn default() -> Self {
        Self::new(UiShellSurfacePreset::default())
    }
}

impl UiHostSession {
    pub fn new(preset: UiShellSurfacePreset) -> Self {
        Self {
            scene: GpuScene::new(palette::BG),
            shell_scene: GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 0.0)),
            packed_scene: PackedGpuScene::default(),
            shell_packed_scene: PackedGpuScene::default(),
            workspace: preset.workspace(),
            shell: UiShellState::default(),
            color_scheme: UiColorScheme::Dark,
            work_projection: UiWorkProjection::preview(),
            selected_contact: 0,
        }
    }

    pub fn set_preset(&mut self, preset: UiShellSurfacePreset) {
        self.workspace = preset.workspace();
    }

    pub fn set_color_scheme(&mut self, scheme: UiColorScheme) {
        self.color_scheme = scheme;
    }

    pub fn clear_work_projection(&mut self) {
        self.work_projection = UiWorkProjection::preview();
    }

    pub fn set_work_projection_from_key_values(&mut self, input: &str) -> bool {
        if let Some(projection) = UiWorkProjection::parse_key_values(input) {
            self.work_projection = projection;
            true
        } else {
            false
        }
    }

    pub fn build_workspace_frame(
        &mut self,
        atlas: &FontAtlas,
        width: f32,
        height: f32,
        connected: bool,
    ) -> u32 {
        let mut state = UnifiedChatState::empty();
        state.connected = connected;
        state.selected_contact = self.selected_contact;
        build_edgerun_workspace_shell_with_font_and_work(
            &mut self.scene,
            atlas,
            width,
            height,
            &mut self.workspace,
            &state,
            Some(&self.work_projection),
        );
        self.scene.apply_color_scheme(self.color_scheme);
        self.packed_scene.pack(&self.scene);
        self.scene.rects().len() as u32
    }

    pub fn build_combined_frame(&mut self, atlas: &FontAtlas, width: f32, height: f32) -> u32 {
        let mut state = UnifiedChatState::empty();
        state.selected_contact = self.selected_contact;
        build_edgerun_workspace_with_shell_with_font(
            &mut self.scene,
            atlas,
            width,
            height,
            &mut self.workspace,
            &mut self.shell,
            &state,
        );
        self.scene.apply_color_scheme(self.color_scheme);
        self.packed_scene.pack(&self.scene);
        self.scene.rects().len() as u32
    }

    pub fn build_shell_frame(&mut self, atlas: &FontAtlas, width: f32, height: f32) -> u32 {
        build_edgerun_shell_overlay_with_font(
            &mut self.shell_scene,
            atlas,
            width,
            height,
            &mut self.shell,
        );
        self.shell_packed_scene.pack(&self.shell_scene);
        self.shell_scene.rects().len() as u32
    }

    pub fn handle_workspace_event(&mut self, event: UiEvent) -> bool {
        let action = self.workspace.handle_event(&self.scene, event);
        self.apply_workspace_action(action)
    }

    pub fn handle_shell_event(&mut self, event: UiEvent) -> bool {
        let action = self.shell.handle_event(&self.shell_scene, event);
        self.apply_shell_action(action)
    }

    pub fn handle_combined_event(&mut self, event: UiEvent) -> bool {
        let action =
            self.shell
                .handle_then_workspace(&mut self.workspace, &self.scene, &self.scene, event);
        match action {
            super::UiShellWorkspaceAction::None => false,
            super::UiShellWorkspaceAction::Shell(action) => action.needs_redraw(),
            super::UiShellWorkspaceAction::Workspace(action) => self.apply_workspace_action(action),
        }
    }

    pub fn apply_shell_action(&mut self, action: UiShellAction) -> bool {
        action.apply_to_workspace(&mut self.workspace)
    }

    pub fn apply_workspace_action(&mut self, action: UiWorkspaceAction) -> bool {
        match action {
            UiWorkspaceAction::AppAction {
                action: UiAction::Activated(hit),
                ..
            } if matches!(hit.kind, HitKind::Contact) => self.activate_contact(hit),
            other => other.needs_redraw(),
        }
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<GpuHit> {
        self.scene.hit_test(x, y)
    }

    pub fn shell_hit_test(&self, x: f32, y: f32) -> Option<GpuHit> {
        self.shell_scene.hit_test(x, y)
    }

    fn activate_contact(&mut self, hit: GpuHit) -> bool {
        let next = hit.id as usize;
        if self.selected_contact == next {
            false
        } else {
            self.selected_contact = next;
            true
        }
    }
}

pub const fn ui_key_from_web_key_code(code: u32) -> UiKey {
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

pub const fn ui_key_from_sdl_key_sym(sym: i32) -> UiKey {
    match sym {
        8 => UiKey::Backspace,
        9 => UiKey::Tab,
        13 => UiKey::Enter,
        27 => UiKey::Escape,
        127 | 0x4000_004c => UiKey::Delete,
        0x4000_004a => UiKey::Home,
        0x4000_004d => UiKey::End,
        0x4000_004b => UiKey::PageUp,
        0x4000_004e => UiKey::PageDown,
        0x4000_0050 => UiKey::ArrowLeft,
        0x4000_004f => UiKey::ArrowRight,
        0x4000_0052 => UiKey::ArrowUp,
        0x4000_0051 => UiKey::ArrowDown,
        other => UiKey::Other(other as u32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_host_supplied_work_projection() {
        let projection = UiWorkProjection::parse_key_values(
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
        assert!(UiWorkProjection::parse_key_values("made_up=true").is_none());
    }

    #[test]
    fn maps_backend_key_codes() {
        assert_eq!(ui_key_from_web_key_code(13), UiKey::Enter);
        assert_eq!(ui_key_from_sdl_key_sym(0x4000_0050), UiKey::ArrowLeft);
    }
}
