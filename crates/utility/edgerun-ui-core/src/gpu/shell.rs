use super::app_registry::app_id_for_kind;
#[cfg(any(feature = "fontdue-text", test))]
use super::{
    card, row, shadcn_badge, shadcn_button, shadcn_command, shadcn_item, text,
    UiShadcnBadgeVariant, UiShadcnButtonSize, UiShadcnButtonVariant, UiPainter, UiRect,
    EDGERUN_APP_REGISTRY, SHELL_LAUNCHER_BUTTON_ID,
};
use super::{
    app_spec_for_launch_id, GpuHit, GpuScene, HitKind, UiAction, UiAppKind,
    UiComponentPreviewState, UiEvent, UiRuntimeState,
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
        let mut launcher = card("bg-panel border rounded-lg p-3 gap-2")
            .child(shadcn_command("Search apps...", SHELL_LAUNCHER_BUTTON_ID + 1));
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
