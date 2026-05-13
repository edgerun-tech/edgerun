use super::app_registry::app_id_for_kind;
#[cfg(any(feature = "fontdue-text", test))]
use super::{
    app_launcher_item, column, header, UiIcon, UiPainter, UiRect, EDGERUN_APP_REGISTRY,
    SHELL_LAUNCHER_BUTTON_ID,
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
            UiAction::Activated(hit) if hit.kind == HitKind::ShellLauncher => {
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
    let theme = ui.theme();
    let colors = theme.colors;
    let radius = theme.radius;
    let bar = UiRect::new(10.0, 10.0, (bounds.w - 20.0).max(0.0), 42.0);
    ui.fill_rect(bar, radius.card, colors.topbar.with_alpha(0.92));
    ui.border_rect(bar, radius.card, colors.border.with_alpha(0.72));
    let launcher = UiRect::new(bar.x + 8.0, bar.y + 6.0, 112.0, 30.0);
    ui.hit(
        HitKind::ShellLauncher,
        SHELL_LAUNCHER_BUTTON_ID,
        launcher.x,
        launcher.y,
        launcher.w,
        launcher.h,
    );
    ui.fill_rect(
        launcher,
        radius.control,
        if shell.launcher_open {
            colors.active
        } else {
            colors.row
        },
    );
    ui.icon(
        UiRect::new(launcher.x + 9.0, launcher.y + 7.0, 16.0, 16.0),
        UiIcon::App,
        colors.accent,
    );
    ui.bounded_label(
        launcher.x + 32.0,
        launcher.y + 8.0,
        launcher.w - 42.0,
        "EdgeRun",
        2.0,
        colors.text,
    );
    ui.bounded_label(
        bar.x + 138.0,
        bar.y + 14.0,
        (bar.w - 280.0).max(0.0),
        "identity-routed local shell",
        2.0,
        colors.muted,
    );
    ui.badge(bar.x + bar.w - 108.0, bar.y + 10.0, "local", colors.success);

    if shell.launcher_open {
        let panel = UiRect::new(
            bar.x,
            bar.y + bar.h + 8.0,
            360.0_f32.min(bounds.w - 20.0),
            392.0,
        );
        let mut launcher = column("bg-panel border rounded-md p-3 gap-2")
            .child(header("Launcher").detail("system layer"));
        for spec in EDGERUN_APP_REGISTRY {
            launcher = launcher.child(app_launcher_item(
                spec.title,
                spec.detail,
                spec.icon,
                spec.launch_id,
            ));
        }
        launcher.render_with_state(ui, panel, Some(&shell.runtime));
    }
    ui.set_theme(previous_theme);
}

fn launcher_item_kind(hit: GpuHit) -> Option<UiAppKind> {
    if hit.kind != HitKind::AppLauncherItem {
        return None;
    }
    app_spec_for_launch_id(hit.id).map(|spec| spec.kind)
}
