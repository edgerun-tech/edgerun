use super::{
    Color4, GpuScene, PackedGpuScene, UiAction, UiEvent, UiNode, UiPainter, UiRect, UiRuntimeState,
};

/// Result returned by a `UiSurfaceApp` after it handles a semantic action or
/// frame tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiAppControl {
    /// The app mutated state and the host should rebuild and redraw the scene.
    pub dirty: bool,
    /// The app requested that the current host session should close.
    pub quit: bool,
}

impl UiAppControl {
    pub const fn clean() -> Self {
        Self {
            dirty: false,
            quit: false,
        }
    }

    pub const fn dirty() -> Self {
        Self {
            dirty: true,
            quit: false,
        }
    }

    pub const fn quit() -> Self {
        Self {
            dirty: false,
            quit: true,
        }
    }
}

/// Canonical app boundary for EdgeRun UI surfaces.
///
/// App crates implement this trait and return one `UiNode` tree for the
/// assigned viewport. Input capture, hit testing, drag/drop, focus, text input,
/// scroll, transition ticks, scene construction, and buffer packing stay in
/// `edgerun-ui-core` hosts.
pub trait UiSurfaceApp {
    /// Build the current app surface for the provided viewport.
    fn surface(&mut self, viewport: UiRect) -> UiNode;

    /// Handle a normalized host event before or alongside semantic action
    /// dispatch. Apps may use this for app-level shortcuts or projected scroll
    /// state, but pointer capture and hit testing remain runtime-owned.
    fn handle_event(&mut self, _event: UiEvent) -> UiAppControl {
        UiAppControl::clean()
    }

    /// Handle a semantic action emitted by `UiRuntimeState`.
    fn handle_action(&mut self, _action: UiAction) -> UiAppControl {
        UiAppControl::clean()
    }

    /// Advance app-owned state for one frame. Runtime transitions are advanced
    /// by `UiSurfaceHost`; apps should only update their own model here.
    fn tick(&mut self, _delta_ms: u32) -> UiAppControl {
        UiAppControl::clean()
    }
}

/// Canonical host-side state for an app surface.
///
/// This owns the runtime state and scene buffers so SDL, WebGL, tests, and
/// future hosts route input through the same implementation instead of
/// recreating per-app event loops.
pub struct UiSurfaceHost<A> {
    app: A,
    runtime: UiRuntimeState,
    scene: GpuScene,
    packed: PackedGpuScene,
}

impl<A: UiSurfaceApp> UiSurfaceHost<A> {
    pub fn new(app: A, clear: Color4) -> Self {
        Self {
            app,
            runtime: UiRuntimeState::default(),
            scene: GpuScene::new(clear),
            packed: PackedGpuScene::default(),
        }
    }

    pub fn app(&self) -> &A {
        &self.app
    }

    pub fn app_mut(&mut self) -> &mut A {
        &mut self.app
    }

    pub fn runtime(&self) -> &UiRuntimeState {
        &self.runtime
    }

    pub fn scene(&self) -> &GpuScene {
        &self.scene
    }

    pub fn packed_scene(&self) -> &PackedGpuScene {
        &self.packed
    }

    pub fn render_without_font(&mut self, width: f32, height: f32) {
        self.scene.clear_rects();
        let viewport = UiRect::new(0.0, 0.0, width.max(0.0), height.max(0.0));
        let surface = self.app.surface(viewport);
        let mut ui = UiPainter::new(&mut self.scene);
        surface.render_with_state(&mut ui, viewport, Some(&self.runtime));
        self.packed.pack(&self.scene);
    }

    #[cfg(feature = "fontdue-text")]
    pub fn render_with_font(&mut self, atlas: &super::FontAtlas, width: f32, height: f32) {
        self.scene.clear_rects();
        let viewport = UiRect::new(0.0, 0.0, width.max(0.0), height.max(0.0));
        let surface = self.app.surface(viewport);
        let mut ui = UiPainter::with_font(&mut self.scene, atlas);
        surface.render_with_state(&mut ui, viewport, Some(&self.runtime));
        self.packed.pack(&self.scene);
    }

    pub fn handle_event(&mut self, event: UiEvent) -> UiAppControl {
        let action = self.runtime.handle_event(&self.scene, event.clone());
        let mut control = self.app.handle_event(event);
        let action_control = self.handle_action(action);
        control.dirty |= action_control.dirty;
        control.quit |= action_control.quit;
        control
    }

    pub fn handle_action(&mut self, action: UiAction) -> UiAppControl {
        if action == UiAction::None {
            UiAppControl::clean()
        } else {
            self.app.handle_action(action)
        }
    }

    pub fn frame(&mut self, delta_ms: u32) -> UiAppControl {
        let frame = self
            .runtime
            .handle_frame(&self.scene, super::UiFrameInput::new(delta_ms));
        let mut control = UiAppControl {
            dirty: frame.needs_redraw || frame.transitions_active,
            quit: false,
        };
        for action in frame.actions {
            let next = self.handle_action(action);
            control.dirty |= next.dirty;
            control.quit |= next.quit;
        }
        let tick = self.app.tick(delta_ms);
        control.dirty |= tick.dirty;
        control.quit |= tick.quit;
        control
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::{ButtonStyle, HitKind, UiEvent, button};

    #[derive(Default)]
    struct CounterApp {
        count: u32,
    }

    impl UiSurfaceApp for CounterApp {
        fn surface(&mut self, _viewport: UiRect) -> UiNode {
            button("Increment", 7, ButtonStyle::Primary).class("h-9")
        }

        fn handle_action(&mut self, action: UiAction) -> UiAppControl {
            if matches!(action, UiAction::Activated(hit) if hit.kind == HitKind::Button && hit.id == 7)
            {
                self.count += 1;
                UiAppControl::dirty()
            } else {
                UiAppControl::clean()
            }
        }
    }

    #[test]
    fn surface_host_owns_runtime_and_forwards_actions() {
        let mut host = UiSurfaceHost::new(CounterApp::default(), Color4::rgb_u8(0, 0, 0));
        host.render_without_font(240.0, 80.0);

        let hit = host
            .scene()
            .hits()
            .iter()
            .find(|hit| hit.kind == HitKind::Button && hit.id == 7)
            .copied()
            .expect("button hit");

        let result = host.handle_event(UiEvent::PointerDown {
            x: hit.x + 2.0,
            y: hit.y + 2.0,
        });
        assert!(result.dirty);
        assert_eq!(host.app().count, 1);
    }
}
