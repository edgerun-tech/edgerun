use super::{
    GpuClip, GpuScene, HitKind, UiAction, UiComponentPreviewState, UiEvent, UiFrameInput, UiIcon,
    UiPainter, UiRect, UiRuntimeState, spacing,
};
use std::string::String;
use std::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiTileAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiTileNode {
    Leaf {
        surface_id: u32,
    },
    Split {
        axis: UiTileAxis,
        ratio_percent: u8,
        first: Box<UiTileNode>,
        second: Box<UiTileNode>,
    },
    Tabs {
        surface_ids: Vec<u32>,
        selected: usize,
    },
}

#[derive(Clone, Debug)]
pub struct UiWorkspaceSurface {
    pub id: u32,
    pub title: String,
    pub kind: UiWorkspaceSurfaceKind,
    pub full_screen: bool,
    pub runtime: UiRuntimeState,
    pub style_preview: UiComponentPreviewState,
    pub(super) bounds: Option<UiRect>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiWorkspaceSurfaceKind {
    #[default]
    Generic,
}

impl UiWorkspaceSurface {
    pub fn new(id: u32, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            kind: UiWorkspaceSurfaceKind::Generic,
            full_screen: false,
            runtime: UiRuntimeState::default(),
            style_preview: UiComponentPreviewState::default(),
            bounds: None,
        }
    }

    pub fn kind(mut self, kind: UiWorkspaceSurfaceKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn full_screen(mut self, full_screen: bool) -> Self {
        self.full_screen = full_screen;
        self
    }

    pub fn bounds(&self) -> Option<UiRect> {
        self.bounds
    }

    pub fn apply_action(&mut self, action: &UiAction, commit: bool) {
        let _ = (action, commit);
    }
}

#[derive(Clone, Debug)]
pub struct UiWorkspace {
    pub surfaces: Vec<UiWorkspaceSurface>,
    pub root: UiTileNode,
    pub focused_surface: Option<u32>,
    pub user_style: UiComponentPreviewState,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiWorkspaceAction {
    None,
    FocusedSurface(u32),
    ClosedSurface(u32),
    SplitRequested { surface_id: u32, axis: UiTileAxis },
    SurfaceAction { surface_id: u32, action: UiAction },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiWorkspaceFrameOutput {
    pub actions: Vec<UiWorkspaceAction>,
    pub needs_redraw: bool,
    pub transitions_active: bool,
}

impl UiWorkspaceAction {
    pub const fn needs_redraw(&self) -> bool {
        match self {
            Self::None => false,
            Self::SurfaceAction { action, .. } => action.needs_redraw(),
            Self::FocusedSurface(_) | Self::ClosedSurface(_) | Self::SplitRequested { .. } => true,
        }
    }
}

pub(super) const WORKSPACE_CHROME_H: f32 = spacing::WORKSPACE_CHROME_H;
const WORKSPACE_GAP: f32 = spacing::WORKSPACE_GAP;
const WORKSPACE_MIN_TILE_W: f32 = spacing::WORKSPACE_MIN_TILE_W;
const WORKSPACE_MIN_TILE_H: f32 = spacing::WORKSPACE_MIN_TILE_H;
const WORKSPACE_TAB_MIN_W: f32 = spacing::WORKSPACE_TAB_MIN_W;
const WORKSPACE_TAB_MAX_W: f32 = spacing::WORKSPACE_TAB_MAX_W;

impl UiWorkspace {
    pub fn single(surface: UiWorkspaceSurface) -> Self {
        let focused_surface = Some(surface.id);
        Self {
            root: UiTileNode::Leaf {
                surface_id: surface.id,
            },
            surfaces: vec![surface],
            focused_surface,
            user_style: UiComponentPreviewState::default(),
        }
    }

    pub fn split(axis: UiTileAxis, first: UiWorkspaceSurface, second: UiWorkspaceSurface) -> Self {
        let focused_surface = Some(first.id);
        Self {
            root: UiTileNode::Split {
                axis,
                ratio_percent: 50,
                first: Box::new(UiTileNode::Leaf {
                    surface_id: first.id,
                }),
                second: Box::new(UiTileNode::Leaf {
                    surface_id: second.id,
                }),
            },
            surfaces: vec![first, second],
            focused_surface,
            user_style: UiComponentPreviewState::default(),
        }
    }

    pub fn tabs(surfaces: Vec<UiWorkspaceSurface>, selected: usize) -> Self {
        let selected = selected.min(surfaces.len().saturating_sub(1));
        let focused_surface = surfaces.get(selected).map(|surface| surface.id);
        Self {
            root: UiTileNode::Tabs {
                surface_ids: surfaces.iter().map(|surface| surface.id).collect(),
                selected,
            },
            surfaces,
            focused_surface,
            user_style: UiComponentPreviewState::default(),
        }
    }

    pub fn full_screen(surface: UiWorkspaceSurface) -> Self {
        let surface = surface.full_screen(true);
        let focused_surface = Some(surface.id);
        Self {
            root: UiTileNode::Leaf {
                surface_id: surface.id,
            },
            surfaces: vec![surface],
            focused_surface,
            user_style: UiComponentPreviewState::default(),
        }
    }

    pub fn user_style(mut self, user_style: UiComponentPreviewState) -> Self {
        self.user_style = user_style;
        self
    }

    pub fn surface(&self, id: u32) -> Option<&UiWorkspaceSurface> {
        self.surfaces.iter().find(|surface| surface.id == id)
    }

    pub fn surface_mut(&mut self, id: u32) -> Option<&mut UiWorkspaceSurface> {
        self.surfaces.iter_mut().find(|surface| surface.id == id)
    }

    pub fn render(
        &mut self,
        ui: &mut UiPainter<'_, '_>,
        bounds: UiRect,
        mut render_surface: impl FnMut(&mut UiPainter<'_, '_>, UiRect, &UiWorkspaceSurface),
    ) {
        let previous_theme = ui.theme();
        ui.set_theme(self.user_style.resolved_theme());
        for surface in &mut self.surfaces {
            surface.bounds = None;
        }
        let root = self.root.clone();
        self.render_tile(ui, bounds, &root, &mut render_surface);
        ui.set_theme(previous_theme);
    }

    pub fn handle_event(&mut self, scene: &GpuScene, event: UiEvent) -> UiWorkspaceAction {
        match event {
            UiEvent::PointerDown { x, y } => {
                if let Some(hit) = scene.hit_test(x, y) {
                    match hit.kind {
                        HitKind::WorkspaceTab => {
                            self.focused_surface = Some(hit.id);
                            return UiWorkspaceAction::FocusedSurface(hit.id);
                        }
                        HitKind::WorkspaceClose => return UiWorkspaceAction::ClosedSurface(hit.id),
                        HitKind::WorkspaceSplit => {
                            return UiWorkspaceAction::SplitRequested {
                                surface_id: hit.id,
                                axis: UiTileAxis::Horizontal,
                            };
                        }
                        _ => {}
                    }
                }
                let Some(surface_id) = self.surface_id_at(x, y) else {
                    return UiWorkspaceAction::None;
                };
                self.focused_surface = Some(surface_id);
                self.route_to_surface(scene, surface_id, UiEvent::PointerDown { x, y })
            }
            UiEvent::PointerMove { x, y } => {
                let surface_id = self.surface_id_at(x, y).or(self.focused_surface);
                surface_id
                    .map(|surface_id| {
                        self.route_to_surface(scene, surface_id, UiEvent::PointerMove { x, y })
                    })
                    .unwrap_or(UiWorkspaceAction::None)
            }
            UiEvent::PointerUp { x, y } => self
                .focused_surface
                .map(|surface_id| {
                    self.route_to_surface(scene, surface_id, UiEvent::PointerUp { x, y })
                })
                .unwrap_or(UiWorkspaceAction::None),
            UiEvent::Wheel { x, y, delta_y } => {
                let surface_id = self.surface_id_at(x, y).or(self.focused_surface);
                surface_id
                    .map(|surface_id| {
                        self.route_to_surface(scene, surface_id, UiEvent::Wheel { x, y, delta_y })
                    })
                    .unwrap_or(UiWorkspaceAction::None)
            }
            UiEvent::KeyDown { key } => self
                .focused_surface
                .map(|surface_id| {
                    self.route_to_surface(scene, surface_id, UiEvent::KeyDown { key })
                })
                .unwrap_or(UiWorkspaceAction::None),
            UiEvent::TextInput(value) => self
                .focused_surface
                .map(|surface_id| {
                    self.route_to_surface(scene, surface_id, UiEvent::TextInput(value))
                })
                .unwrap_or(UiWorkspaceAction::None),
            UiEvent::Blur => {
                let mut last = UiWorkspaceAction::None;
                for surface in &mut self.surfaces {
                    let action = surface.runtime.handle_event(scene, UiEvent::Blur);
                    if action != UiAction::None {
                        last = UiWorkspaceAction::SurfaceAction {
                            surface_id: surface.id,
                            action,
                        };
                    }
                }
                last
            }
        }
    }

    pub fn handle_frame(
        &mut self,
        scene: &GpuScene,
        input: UiFrameInput,
    ) -> UiWorkspaceFrameOutput {
        let mut output = UiWorkspaceFrameOutput::default();
        for surface in &mut self.surfaces {
            let frame = surface.runtime.handle_frame(
                scene,
                UiFrameInput {
                    viewport: surface.bounds,
                    scale: input.scale,
                    delta_ms: input.delta_ms,
                    events: Vec::new(),
                },
            );
            output.needs_redraw |= frame.needs_redraw;
            output.transitions_active |= frame.transitions_active;
        }

        for event in input.events {
            let action = self.handle_event(scene, event);
            output.needs_redraw |= action.needs_redraw();
            if action != UiWorkspaceAction::None {
                output.actions.push(action);
            }
        }
        output.needs_redraw |= output.transitions_active;
        output
    }

    fn route_to_surface(
        &mut self,
        scene: &GpuScene,
        surface_id: u32,
        event: UiEvent,
    ) -> UiWorkspaceAction {
        let Some(surface) = self.surface_mut(surface_id) else {
            return UiWorkspaceAction::None;
        };
        let commit = matches!(
            event,
            UiEvent::PointerUp { .. } | UiEvent::KeyDown { .. } | UiEvent::TextInput(_)
        );
        let action = surface.runtime.handle_event(scene, event);
        if action == UiAction::None {
            UiWorkspaceAction::None
        } else {
            surface.apply_action(&action, commit);
            UiWorkspaceAction::SurfaceAction { surface_id, action }
        }
    }

    fn surface_id_at(&self, x: f32, y: f32) -> Option<u32> {
        self.surfaces
            .iter()
            .find(|surface| surface.bounds.is_some_and(|bounds| bounds.contains(x, y)))
            .map(|surface| surface.id)
    }

    pub fn select_or_insert_tab(&mut self, surface_id: u32) {
        if select_or_insert_tab_node(&mut self.root, surface_id) {
            return;
        }
        self.root = UiTileNode::Tabs {
            surface_ids: vec![surface_id],
            selected: 0,
        };
    }

    fn render_tile(
        &mut self,
        ui: &mut UiPainter<'_, '_>,
        rect: UiRect,
        tile: &UiTileNode,
        render_surface: &mut impl FnMut(&mut UiPainter<'_, '_>, UiRect, &UiWorkspaceSurface),
    ) {
        match tile {
            UiTileNode::Leaf { surface_id } => {
                self.render_leaf(ui, rect, *surface_id, render_surface)
            }
            UiTileNode::Split {
                axis,
                ratio_percent,
                first,
                second,
            } => {
                let ratio = (*ratio_percent as f32 / 100.0).clamp(0.18, 0.82);
                match axis {
                    UiTileAxis::Horizontal => {
                        let available = (rect.w - WORKSPACE_GAP).max(0.0);
                        let (first_w, second_w) =
                            split_lengths(available, ratio, WORKSPACE_MIN_TILE_W);
                        self.render_tile(
                            ui,
                            UiRect::new(rect.x, rect.y, first_w, rect.h),
                            first,
                            render_surface,
                        );
                        self.render_tile(
                            ui,
                            UiRect::new(rect.x + first_w + WORKSPACE_GAP, rect.y, second_w, rect.h),
                            second,
                            render_surface,
                        );
                    }
                    UiTileAxis::Vertical => {
                        let available = (rect.h - WORKSPACE_GAP).max(0.0);
                        let (first_h, second_h) =
                            split_lengths(available, ratio, WORKSPACE_MIN_TILE_H);
                        self.render_tile(
                            ui,
                            UiRect::new(rect.x, rect.y, rect.w, first_h),
                            first,
                            render_surface,
                        );
                        self.render_tile(
                            ui,
                            UiRect::new(rect.x, rect.y + first_h + WORKSPACE_GAP, rect.w, second_h),
                            second,
                            render_surface,
                        );
                    }
                }
            }
            UiTileNode::Tabs {
                surface_ids,
                selected,
            } => {
                self.render_tab_strip(ui, rect, surface_ids, *selected);
                if let Some(surface_id) = surface_ids.get(*selected) {
                    self.render_surface_body(
                        ui,
                        UiRect::new(
                            rect.x,
                            rect.y + WORKSPACE_CHROME_H,
                            rect.w,
                            (rect.h - WORKSPACE_CHROME_H).max(0.0),
                        ),
                        *surface_id,
                        render_surface,
                    );
                }
            }
        }
    }

    fn render_leaf(
        &mut self,
        ui: &mut UiPainter<'_, '_>,
        rect: UiRect,
        surface_id: u32,
        render_surface: &mut impl FnMut(&mut UiPainter<'_, '_>, UiRect, &UiWorkspaceSurface),
    ) {
        if self
            .surface(surface_id)
            .is_some_and(|surface| surface.full_screen)
        {
            self.render_surface_body(ui, rect, surface_id, render_surface);
            return;
        }
        self.render_surface_chrome(ui, rect, surface_id);
        self.render_surface_body(
            ui,
            UiRect::new(
                rect.x,
                rect.y + WORKSPACE_CHROME_H,
                rect.w,
                (rect.h - WORKSPACE_CHROME_H).max(0.0),
            ),
            surface_id,
            render_surface,
        );
    }

    fn render_tab_strip(
        &self,
        ui: &mut UiPainter<'_, '_>,
        rect: UiRect,
        surface_ids: &[u32],
        selected: usize,
    ) {
        let colors = ui.theme().colors;
        ui.fill_rect(
            UiRect::new(rect.x, rect.y, rect.w, WORKSPACE_CHROME_H),
            ui.theme().radius.card,
            colors.topbar,
        );
        let mut x = rect.x + 8.0;
        for (index, surface_id) in surface_ids.iter().copied().enumerate() {
            let title = self
                .surface(surface_id)
                .map(|surface| surface.title.as_str())
                .unwrap_or("Surface");
            let w = (title.chars().count() as f32 * 8.0 + 42.0)
                .clamp(WORKSPACE_TAB_MIN_W, WORKSPACE_TAB_MAX_W);
            let tab = UiRect::new(x, rect.y + 5.0, w, 24.0);
            ui.hit(
                HitKind::WorkspaceTab,
                surface_id,
                tab.x,
                tab.y,
                tab.w,
                tab.h,
            );
            ui.fill_rect(
                tab,
                ui.theme().radius.card,
                if index == selected {
                    colors.active
                } else {
                    colors.row
                },
            );
            ui.bounded_label(
                tab.x + 12.0,
                tab.y + 6.0,
                tab.w - 24.0,
                title,
                2.0,
                if index == selected {
                    colors.text
                } else {
                    colors.muted
                },
            );
            x += w + 6.0;
        }
    }

    fn render_surface_chrome(&self, ui: &mut UiPainter<'_, '_>, rect: UiRect, surface_id: u32) {
        let colors = ui.theme().colors;
        let chrome = UiRect::new(rect.x, rect.y, rect.w, WORKSPACE_CHROME_H);
        let focused = self.focused_surface == Some(surface_id);
        ui.fill_rect(chrome, ui.theme().radius.card, colors.topbar);
        ui.border_rect(
            rect,
            ui.theme().radius.card,
            if focused {
                colors.accent.with_alpha(0.68)
            } else {
                colors.border
            },
        );
        let title = self
            .surface(surface_id)
            .map(|surface| surface.title.as_str())
            .unwrap_or("Surface");
        ui.hit(
            HitKind::WorkspaceTab,
            surface_id,
            chrome.x,
            chrome.y,
            chrome.w,
            chrome.h,
        );
        ui.bounded_label(
            chrome.x + 12.0,
            chrome.y + 10.0,
            (chrome.w - 98.0).max(0.0),
            title,
            2.0,
            if focused { colors.text } else { colors.muted },
        );
        if chrome.w > 110.0 {
            let split_rect = UiRect::new(
                chrome.x + chrome.w - 60.0,
                chrome.y + 1.0,
                spacing::MIN_TOUCH_TARGET,
                spacing::MIN_TOUCH_TARGET,
            );
            ui.hit(
                HitKind::WorkspaceSplit,
                surface_id,
                split_rect.x,
                split_rect.y,
                split_rect.w,
                split_rect.h,
            );
            ui.icon(split_rect.inset(3.0, 3.0), UiIcon::Route, colors.muted);
            let close_rect = UiRect::new(
                chrome.x + chrome.w - 31.0,
                chrome.y + 1.0,
                spacing::MIN_TOUCH_TARGET,
                spacing::MIN_TOUCH_TARGET,
            );
            ui.hit(
                HitKind::WorkspaceClose,
                surface_id,
                close_rect.x,
                close_rect.y,
                close_rect.w,
                close_rect.h,
            );
            ui.icon(close_rect.inset(3.0, 3.0), UiIcon::X, colors.muted);
        }
    }

    fn render_surface_body(
        &mut self,
        ui: &mut UiPainter<'_, '_>,
        rect: UiRect,
        surface_id: u32,
        render_surface: &mut impl FnMut(&mut UiPainter<'_, '_>, UiRect, &UiWorkspaceSurface),
    ) {
        if rect.w <= 0.0 || rect.h <= 0.0 {
            return;
        }
        if let Some(surface) = self.surface_mut(surface_id) {
            surface.bounds = Some(rect);
        }
        let clipped = ui
            .scene
            .push_clip(GpuClip::new(rect.x, rect.y, rect.w, rect.h));
        if clipped {
            if let Some(surface) = self.surface(surface_id) {
                render_surface(ui, rect, surface);
            }
            ui.scene.pop_clip();
        }
    }
}

fn split_lengths(available: f32, ratio: f32, min: f32) -> (f32, f32) {
    if available <= 0.0 {
        return (0.0, 0.0);
    }
    if available < min * 2.0 {
        let first = (available * ratio).max(0.0);
        return (first, (available - first).max(0.0));
    }
    let first = (available * ratio).clamp(min, available - min);
    (first, available - first)
}

fn select_or_insert_tab_node(tile: &mut UiTileNode, surface_id: u32) -> bool {
    match tile {
        UiTileNode::Tabs {
            surface_ids,
            selected,
        } => {
            if let Some(index) = surface_ids.iter().position(|id| *id == surface_id) {
                *selected = index;
            } else {
                surface_ids.push(surface_id);
                *selected = surface_ids.len() - 1;
            }
            true
        }
        UiTileNode::Split { first, second, .. } => {
            select_or_insert_tab_node(second, surface_id)
                || select_or_insert_tab_node(first, surface_id)
        }
        UiTileNode::Leaf { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::{Color4, GpuScene, UiFrameInput, UiPainter, UiTransitionSpec, palette};

    fn render_workspace(workspace: &mut UiWorkspace, width: f32, height: f32) -> GpuScene {
        let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        {
            let mut ui = UiPainter::new(&mut scene);
            workspace.render(
                &mut ui,
                UiRect::new(0.0, 0.0, width, height),
                |ui, bounds, surface| {
                    ui.fill_rect(bounds, 0.0, palette::PANEL);
                    ui.hit(
                        HitKind::Button,
                        surface.id,
                        bounds.x,
                        bounds.y,
                        bounds.w,
                        bounds.h,
                    );
                    ui.bounded_label(
                        bounds.x + 8.0,
                        bounds.y + 8.0,
                        bounds.w - 16.0,
                        &surface.title,
                        2.0,
                        palette::TEXT,
                    );
                },
            );
        }
        scene
    }

    #[test]
    fn split_lengths_preserve_minimum_when_space_allows() {
        assert_eq!(split_lengths(600.0, 0.18, 220.0), (220.0, 380.0));
        assert_eq!(split_lengths(600.0, 0.82, 220.0), (380.0, 220.0));
        let narrow = split_lengths(300.0, 0.18, 220.0);
        assert!((narrow.0 - 54.0).abs() < 0.001);
        assert!((narrow.1 - 246.0).abs() < 0.001);
        assert_eq!(split_lengths(0.0, 0.5, 220.0), (0.0, 0.0));
    }

    #[test]
    fn workspace_layout_viewports_keep_surfaces_inside_bounds() {
        let viewports = [
            UiRect::new(0.0, 0.0, 360.0, 640.0),
            UiRect::new(0.0, 0.0, 768.0, 1024.0),
            UiRect::new(0.0, 0.0, 1280.0, 720.0),
            UiRect::new(0.0, 0.0, 1920.0, 1080.0),
        ];

        for viewport in viewports {
            let mut workspace = UiWorkspace::split(
                UiTileAxis::Horizontal,
                UiWorkspaceSurface::new(1, "Left"),
                UiWorkspaceSurface::new(2, "Right"),
            );
            let _scene = render_workspace(&mut workspace, viewport.w, viewport.h);
            for surface in &workspace.surfaces {
                let bounds = surface.bounds().expect("rendered surface bounds");
                assert!(bounds.x >= viewport.x);
                assert!(bounds.y >= viewport.y + WORKSPACE_CHROME_H);
                assert!(bounds.x + bounds.w <= viewport.x + viewport.w + f32::EPSILON);
                assert!(bounds.y + bounds.h <= viewport.y + viewport.h + f32::EPSILON);
            }
        }
    }

    #[test]
    fn split_ratios_do_not_collapse_tiles_on_desktop_widths() {
        let mut workspace = UiWorkspace {
            surfaces: vec![
                UiWorkspaceSurface::new(1, "Very long left surface"),
                UiWorkspaceSurface::new(2, "Very long right surface"),
            ],
            root: UiTileNode::Split {
                axis: UiTileAxis::Horizontal,
                ratio_percent: 5,
                first: Box::new(UiTileNode::Leaf { surface_id: 1 }),
                second: Box::new(UiTileNode::Leaf { surface_id: 2 }),
            },
            focused_surface: Some(1),
            user_style: UiComponentPreviewState::default(),
        };
        let _scene = render_workspace(&mut workspace, 960.0, 540.0);

        for id in [1, 2] {
            let bounds = workspace
                .surface(id)
                .and_then(UiWorkspaceSurface::bounds)
                .unwrap();
            assert!(bounds.w >= WORKSPACE_MIN_TILE_W);
        }
    }

    #[test]
    fn workspace_controls_keep_stable_hit_sizes_for_long_labels() {
        let long_title = "network-surface-with-a-very-long-policy-hash-7f2c35aa01b9d0f7";
        let mut workspace = UiWorkspace::single(UiWorkspaceSurface::new(1, long_title));
        let scene = render_workspace(&mut workspace, 480.0, 320.0);

        let controls: Vec<_> = scene
            .hits()
            .iter()
            .filter(|hit| matches!(hit.kind, HitKind::WorkspaceClose | HitKind::WorkspaceSplit))
            .collect();
        assert_eq!(controls.len(), 2);
        for hit in controls {
            assert!(hit.w >= spacing::MIN_TOUCH_TARGET);
            assert!(hit.h >= spacing::MIN_TOUCH_TARGET);
        }
    }

    #[test]
    fn tabs_use_exported_width_contract_for_long_labels() {
        let mut workspace = UiWorkspace::tabs(
            vec![
                UiWorkspaceSurface::new(1, "short"),
                UiWorkspaceSurface::new(2, "policy-hash-7f2c35aa01b9d0f7-route-budget-window"),
            ],
            1,
        );
        let scene = render_workspace(&mut workspace, 620.0, 360.0);
        let tab_hits: Vec<_> = scene
            .hits()
            .iter()
            .filter(|hit| hit.kind == HitKind::WorkspaceTab)
            .collect();
        assert_eq!(tab_hits.len(), 2);
        assert!(tab_hits.iter().all(|hit| hit.w >= WORKSPACE_TAB_MIN_W));
        assert!(tab_hits.iter().all(|hit| hit.w <= WORKSPACE_TAB_MAX_W));
        assert_eq!(tab_hits[0].h, WORKSPACE_CHROME_H - 10.0);
        assert_eq!(tab_hits[1].h, WORKSPACE_CHROME_H - 10.0);
    }

    #[test]
    fn workspace_frame_advances_surface_transitions_and_batches_actions() {
        let mut workspace = UiWorkspace::single(UiWorkspaceSurface::new(1, "Run surface"));
        let scene = render_workspace(&mut workspace, 420.0, 260.0);
        let surface_bounds = workspace
            .surface(1)
            .and_then(UiWorkspaceSurface::bounds)
            .unwrap();
        let mut scene = scene;
        scene.push_transition(UiTransitionSpec::opacity(42, 0.0, 1.0, 100));

        let output = workspace.handle_frame(
            &scene,
            UiFrameInput::new(20).with_event(UiEvent::PointerDown {
                x: surface_bounds.x + 8.0,
                y: surface_bounds.y + 8.0,
            }),
        );

        assert!(output.needs_redraw);
        assert!(output.transitions_active);
        assert!(matches!(
            output.actions.as_slice(),
            [UiWorkspaceAction::SurfaceAction {
                surface_id: 1,
                action: UiAction::Activated(_)
            }]
        ));
        let surface = workspace.surface(1).unwrap();
        assert!(
            surface
                .runtime
                .transition_value(UiTransitionSpec::opacity(42, 0.0, 1.0, 100))
                > 0.0
        );
    }
}
