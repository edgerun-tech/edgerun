use super::app_registry::app_id_for_kind;
use super::{
    EDGERUN_APP_REGISTRY, GpuHit, GpuScene, HitKind, SHELL_LAUNCHER_BUTTON_ID, UiAction, UiAppKind,
    UiEvent, UiIcon, UiPainter, UiRect, UiRuntimeState, app_launcher_item, app_spec_for_launch_id,
    column, header, palette,
};

#[derive(Clone, Debug, Default)]
pub struct UiShellState {
    pub launcher_open: bool,
    pub runtime: UiRuntimeState,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiShellAction {
    None,
    ToggledLauncher(bool),
    OpenApp { app_id: u32, kind: UiAppKind },
    Runtime(UiAction),
}

impl UiShellState {
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

pub(super) fn render_edgerun_shell_overlay(
    ui: &mut UiPainter<'_, '_>,
    bounds: UiRect,
    shell: &mut UiShellState,
) {
    let bar = UiRect::new(10.0, 10.0, (bounds.w - 20.0).max(0.0), 42.0);
    ui.fill_rect(bar, 12.0, palette::TOPBAR.with_alpha(0.92));
    ui.border_rect(bar, 12.0, palette::BORDER.with_alpha(0.72));
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
        9.0,
        if shell.launcher_open {
            palette::ACTIVE_ROW
        } else {
            palette::ROW
        },
    );
    ui.icon(
        UiRect::new(launcher.x + 9.0, launcher.y + 7.0, 16.0, 16.0),
        UiIcon::App,
        palette::ACCENT,
    );
    ui.bounded_label(
        launcher.x + 32.0,
        launcher.y + 8.0,
        launcher.w - 42.0,
        "EdgeRun",
        2.0,
        palette::TEXT,
    );
    ui.bounded_label(
        bar.x + 138.0,
        bar.y + 14.0,
        (bar.w - 280.0).max(0.0),
        "identity-routed local shell",
        2.0,
        palette::MUTED,
    );
    ui.badge(bar.x + bar.w - 108.0, bar.y + 10.0, "local", palette::GREEN);

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
}

fn launcher_item_kind(hit: GpuHit) -> Option<UiAppKind> {
    if hit.kind != HitKind::AppLauncherItem {
        return None;
    }
    app_spec_for_launch_id(hit.id).map(|spec| spec.kind)
}
