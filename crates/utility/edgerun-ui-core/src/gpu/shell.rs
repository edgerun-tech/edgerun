use super::{
    GpuHit, GpuScene, HitKind, UiAction, UiComponentPreviewState, UiEvent, UiFrameInput,
    UiRuntimeState, UiWorkspace, UiWorkspaceAction,
};
use std::vec::Vec;

pub const SHELL_LAUNCHER_BUTTON_ID: u32 = 880;

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
    Runtime(UiAction),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiShellFrameOutput {
    pub actions: Vec<UiShellAction>,
    pub needs_redraw: bool,
    pub transitions_active: bool,
}

impl UiShellAction {
    pub fn apply_to_workspace(&self, _workspace: &mut UiWorkspace) -> bool {
        match self {
            Self::None => false,
            Self::ToggledLauncher(_) => true,
            Self::Runtime(action) => action.needs_redraw(),
        }
    }

    pub const fn needs_redraw(&self) -> bool {
        match self {
            Self::None => false,
            Self::ToggledLauncher(_) => true,
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
        self.action_from_runtime_action(action)
    }

    pub fn handle_frame(&mut self, scene: &GpuScene, input: UiFrameInput) -> UiShellFrameOutput {
        let frame = self.runtime.handle_frame(scene, input);
        let mut output = UiShellFrameOutput {
            needs_redraw: frame.needs_redraw,
            transitions_active: frame.transitions_active,
            actions: Vec::new(),
        };
        for action in frame.actions {
            let action = self.action_from_runtime_action(action);
            output.needs_redraw |= action.needs_redraw();
            if action != UiShellAction::None {
                output.actions.push(action);
            }
        }
        output
    }

    fn action_from_runtime_action(&mut self, action: UiAction) -> UiShellAction {
        match action {
            UiAction::Activated(hit)
                if hit.kind == HitKind::ShellLauncher
                    || (hit.kind == HitKind::Button && hit.id == SHELL_LAUNCHER_BUTTON_ID) =>
            {
                self.launcher_open = !self.launcher_open;
                UiShellAction::ToggledLauncher(self.launcher_open)
            }
            UiAction::Activated(hit) => UiShellAction::Runtime(UiAction::Activated(hit)),
            UiAction::None => UiShellAction::None,
            other => UiShellAction::Runtime(other),
        }
    }

    pub fn owns_hit(hit: GpuHit) -> bool {
        hit.kind == HitKind::ShellLauncher
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::{Color4, GpuScene, UiFrameInput, UiTransitionSpec, palette};

    #[test]
    fn shell_frame_batches_actions_and_transition_redraw_state() {
        let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 0.0));
        scene.push_hit(GpuHit::new(
            HitKind::ShellLauncher,
            SHELL_LAUNCHER_BUTTON_ID,
            0.0,
            0.0,
            120.0,
            40.0,
        ));
        scene.push_transition(UiTransitionSpec::opacity(77, 0.0, 1.0, 100));
        let mut shell = UiShellState::default();

        let output = shell.handle_frame(
            &scene,
            UiFrameInput::new(16).with_event(UiEvent::PointerDown { x: 8.0, y: 8.0 }),
        );

        assert!(output.needs_redraw);
        assert!(output.transitions_active);
        assert_eq!(output.actions, vec![UiShellAction::ToggledLauncher(true)]);
        assert!(shell.launcher_open);
        assert!(
            shell
                .runtime
                .transition_value(UiTransitionSpec::opacity(77, 0.0, 1.0, 100))
                > 0.0
        );

        let _ = palette::BG;
    }
}
