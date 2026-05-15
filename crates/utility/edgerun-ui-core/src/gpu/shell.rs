use super::app_registry::{SHELL_LAUNCHER_BUTTON_ID, app_id_for_kind};
#[cfg(any(feature = "fontdue-text", test))]
use super::{
    EDGERUN_APP_REGISTRY, UiPainter, UiRect, UiShadcnBadgeVariant, UiShadcnButtonSize,
    UiShadcnButtonVariant, card, row, shadcn_badge, shadcn_button, shadcn_command, shadcn_item,
    text,
};
use super::{
    GpuHit, GpuScene, HitKind, UiAction, UiAppKind, UiComponentPreviewState, UiEvent,
    UiRuntimeState, UiWorkspace, UiWorkspaceAction, app_spec_for_launch_id,
};

#[derive(Clone, Debug, Default)]
pub struct UiShellState {
    pub launcher_open: bool,
    pub runtime: UiRuntimeState,
    pub user_style: UiComponentPreviewState,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiShellAction {
    None,
    ToggledLauncher(bool),
    OpenApp { app_id: u32, kind: UiAppKind },
    Runtime(UiAction),
}

impl UiShellAction {
    pub fn apply_to_workspace(&self, workspace: &mut UiWorkspace) -> bool {
        match self {
            Self::OpenApp { kind, .. } => {
                workspace.open_or_focus(*kind);
                true
            }
            Self::None => false,
            Self::ToggledLauncher(_) => true,
            Self::Runtime(action) => action.needs_redraw(),
        }
    }

    pub const fn needs_redraw(&self) -> bool {
        match self {
            Self::None => false,
            Self::ToggledLauncher(_) | Self::OpenApp { .. } => true,
            Self::Runtime(action) => action.needs_redraw(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiShellWorkspaceAction {
    None,
    Shell(UiShellAction),
    Workspace(UiWorkspaceAction),
}

impl UiShellWorkspaceAction {
    pub const fn needs_redraw(&self) -> bool {
        match self {
            Self::None => false,
            Self::Shell(action) => action.needs_redraw(),
            Self::Workspace(action) => action.needs_redraw(),
        }
    }
}

impl UiShellState {
    pub fn user_style(mut self, user_style: UiComponentPreviewState) -> Self {
        self.user_style = user_style;
        self
    }

    pub fn handle_event(&mut self, scene: &GpuScene, event: UiEvent) -> UiShellAction {
        let action = self.runtime.handle_event(scene, event);
        match action {
            UiAction::Activated(hit)
                if hit.kind == HitKind::ShellLauncher
                    || (hit.kind == HitKind::Button && hit.id == SHELL_LAUNCHER_BUTTON_ID) =>
            {
                self.launcher_open = !self.launcher_open;
                UiShellAction::ToggledLauncher(self.launcher_open)
            }
            UiAction::Activated(hit) => {
                if let Some(kind) = launcher_item_kind(hit) {
                    self.launcher_open = false;
                    UiShellAction::OpenApp {
                        app_id: app_id_for_kind(kind),
                        kind,
                    }
                } else {
                    UiShellAction::Runtime(UiAction::Activated(hit))
                }
            }
            UiAction::None => UiShellAction::None,
            other => UiShellAction::Runtime(other),
        }
    }

    pub fn owns_hit(hit: GpuHit) -> bool {
        matches!(hit.kind, HitKind::ShellLauncher | HitKind::AppLauncherItem)
    }

    pub fn handle_then_workspace(
        &mut self,
        workspace: &mut UiWorkspace,
        combined_scene: &GpuScene,
        workspace_scene: &GpuScene,
        event: UiEvent,
    ) -> UiShellWorkspaceAction {
        let shell_target = match event {
            UiEvent::PointerDown { x, y }
            | UiEvent::PointerMove { x, y }
            | UiEvent::PointerUp { x, y }
            | UiEvent::Wheel { x, y, .. } => {
                combined_scene.hit_test(x, y).is_some_and(Self::owns_hit)
            }
            UiEvent::KeyDown { .. } => self.runtime.focused().is_some(),
            UiEvent::TextInput(_) | UiEvent::Blur => false,
        };
        if shell_target {
            let action = self.handle_event(combined_scene, event);
            action.apply_to_workspace(workspace);
            UiShellWorkspaceAction::Shell(action)
        } else {
            let action = workspace.handle_event(workspace_scene, event);
            UiShellWorkspaceAction::Workspace(action)
        }
    }
}

#[cfg(any(feature = "fontdue-text", test))]
pub(super) fn render_edgerun_shell_overlay(
    ui: &mut UiPainter<'_, '_>,
    bounds: UiRect,
    shell: &mut UiShellState,
) {
    let previous_theme = ui.theme();
    ui.set_theme(shell.user_style.resolved_theme());
    let bar = UiRect::new(10.0, 10.0, (bounds.w - 20.0).max(0.0), 42.0);

    row("row bg-topbar border rounded-lg p-1.5 gap-3 items-center")
        .child(shadcn_button(
            "EdgeRun",
            SHELL_LAUNCHER_BUTTON_ID,
            if shell.launcher_open {
                UiShadcnButtonVariant::Secondary
            } else {
                UiShadcnButtonVariant::Ghost
            },
            UiShadcnButtonSize::Default,
        ))
        .child(text("identity-routed local shell").class("flex-1 text-muted truncate"))
        .child(shadcn_badge("local", UiShadcnBadgeVariant::Secondary))
        .render_with_state(ui, bar, Some(&shell.runtime));

    if shell.launcher_open {
        let panel = UiRect::new(
            bar.x,
            bar.y + bar.h + 8.0,
            360.0_f32.min(bounds.w - 20.0),
            392.0,
        );
        let mut launcher = card("bg-panel border rounded-lg p-3 gap-2").child(shadcn_command(
            "Search apps...",
            SHELL_LAUNCHER_BUTTON_ID + 1,
        ));
        for spec in EDGERUN_APP_REGISTRY {
            launcher = launcher.child(shadcn_item(
                spec.title,
                spec.detail,
                spec.launch_id,
                ui.theme().colors.accent,
            ));
        }
        launcher.render_with_state(ui, panel, Some(&shell.runtime));
    }
    ui.set_theme(previous_theme);
}

fn launcher_item_kind(hit: GpuHit) -> Option<UiAppKind> {
    app_spec_for_launch_id(hit.id).map(|spec| spec.kind)
}
