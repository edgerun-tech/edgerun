use super::app_registry::{
    CHAT_APP_ID, STORAGE_APP_ID, TRUST_MANAGER_APP_ID, app_id_for_kind, app_surface_for_kind,
};
use super::{
    GpuClip, GpuScene, HitKind, UiAction, UiEvent, UiIcon, UiPainter, UiRect, UiRuntimeState,
    palette,
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
        app_id: u32,
    },
    Split {
        axis: UiTileAxis,
        ratio_percent: u8,
        first: Box<UiTileNode>,
        second: Box<UiTileNode>,
    },
    Tabs {
        app_ids: Vec<u32>,
        selected: usize,
    },
}

#[derive(Clone, Debug)]
pub struct UiAppSurface {
    pub id: u32,
    pub title: String,
    pub kind: UiAppKind,
    pub full_screen: bool,
    pub runtime: UiRuntimeState,
    pub(super) bounds: Option<UiRect>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiAppKind {
    #[default]
    Generic,
    Chat,
    TrustManager,
    Storage,
    LockScreen,
    CapabilityRequest,
    ComponentGallery,
}

impl UiAppSurface {
    pub fn new(id: u32, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            kind: UiAppKind::Generic,
            full_screen: false,
            runtime: UiRuntimeState::default(),
            bounds: None,
        }
    }

    pub fn kind(mut self, kind: UiAppKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn full_screen(mut self, full_screen: bool) -> Self {
        self.full_screen = full_screen;
        self
    }

    pub fn chat(id: u32) -> Self {
        Self::new(id, "EdgeRun Chat").kind(UiAppKind::Chat)
    }

    pub fn trust_manager(id: u32) -> Self {
        Self::new(id, "Trust Manager").kind(UiAppKind::TrustManager)
    }

    pub fn storage(id: u32) -> Self {
        Self::new(id, "Storage").kind(UiAppKind::Storage)
    }

    pub fn lock_screen(id: u32) -> Self {
        Self::new(id, "Trust Container Locked")
            .kind(UiAppKind::LockScreen)
            .full_screen(true)
    }

    pub fn capability_request(id: u32) -> Self {
        Self::new(id, "Capability Request")
            .kind(UiAppKind::CapabilityRequest)
            .full_screen(true)
    }

    pub fn component_gallery(id: u32) -> Self {
        Self::new(id, "Component Gallery").kind(UiAppKind::ComponentGallery)
    }

    pub fn bounds(&self) -> Option<UiRect> {
        self.bounds
    }
}

#[derive(Clone, Debug)]
pub struct UiWorkspace {
    pub apps: Vec<UiAppSurface>,
    pub root: UiTileNode,
    pub focused_app: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiWorkspaceAction {
    None,
    FocusedApp(u32),
    ClosedApp(u32),
    SplitRequested { app_id: u32, axis: UiTileAxis },
    AppAction { app_id: u32, action: UiAction },
}

pub(super) const WORKSPACE_CHROME_H: f32 = 34.0;
const WORKSPACE_GAP: f32 = 6.0;

impl UiWorkspace {
    pub fn edgerun_default() -> Self {
        Self {
            apps: vec![
                app_surface_for_kind(UiAppKind::Chat),
                app_surface_for_kind(UiAppKind::TrustManager),
                app_surface_for_kind(UiAppKind::Storage),
            ],
            root: UiTileNode::Split {
                axis: UiTileAxis::Horizontal,
                ratio_percent: 56,
                first: Box::new(UiTileNode::Leaf {
                    app_id: CHAT_APP_ID,
                }),
                second: Box::new(UiTileNode::Split {
                    axis: UiTileAxis::Vertical,
                    ratio_percent: 52,
                    first: Box::new(UiTileNode::Leaf {
                        app_id: TRUST_MANAGER_APP_ID,
                    }),
                    second: Box::new(UiTileNode::Tabs {
                        app_ids: vec![STORAGE_APP_ID],
                        selected: 0,
                    }),
                }),
            },
            focused_app: Some(CHAT_APP_ID),
        }
    }

    pub fn single(app: UiAppSurface) -> Self {
        let focused_app = Some(app.id);
        Self {
            root: UiTileNode::Leaf { app_id: app.id },
            apps: vec![app],
            focused_app,
        }
    }

    pub fn split(axis: UiTileAxis, first: UiAppSurface, second: UiAppSurface) -> Self {
        let focused_app = Some(first.id);
        Self {
            root: UiTileNode::Split {
                axis,
                ratio_percent: 50,
                first: Box::new(UiTileNode::Leaf { app_id: first.id }),
                second: Box::new(UiTileNode::Leaf { app_id: second.id }),
            },
            apps: vec![first, second],
            focused_app,
        }
    }

    pub fn tabs(apps: Vec<UiAppSurface>, selected: usize) -> Self {
        let selected = selected.min(apps.len().saturating_sub(1));
        let focused_app = apps.get(selected).map(|app| app.id);
        Self {
            root: UiTileNode::Tabs {
                app_ids: apps.iter().map(|app| app.id).collect(),
                selected,
            },
            apps,
            focused_app,
        }
    }

    pub fn full_screen(app: UiAppSurface) -> Self {
        let app = app.full_screen(true);
        let focused_app = Some(app.id);
        Self {
            root: UiTileNode::Leaf { app_id: app.id },
            apps: vec![app],
            focused_app,
        }
    }

    pub fn app(&self, id: u32) -> Option<&UiAppSurface> {
        self.apps.iter().find(|app| app.id == id)
    }

    pub fn app_mut(&mut self, id: u32) -> Option<&mut UiAppSurface> {
        self.apps.iter_mut().find(|app| app.id == id)
    }

    pub fn render(
        &mut self,
        ui: &mut UiPainter<'_, '_>,
        bounds: UiRect,
        mut render_app: impl FnMut(&mut UiPainter<'_, '_>, UiRect, &UiAppSurface),
    ) {
        for app in &mut self.apps {
            app.bounds = None;
        }
        let root = self.root.clone();
        self.render_tile(ui, bounds, &root, &mut render_app);
    }

    pub fn handle_event(&mut self, scene: &GpuScene, event: UiEvent) -> UiWorkspaceAction {
        match event {
            UiEvent::PointerDown { x, y } => {
                if let Some(hit) = scene.hit_test(x, y) {
                    match hit.kind {
                        HitKind::WorkspaceTab => {
                            self.focused_app = Some(hit.id);
                            return UiWorkspaceAction::FocusedApp(hit.id);
                        }
                        HitKind::WorkspaceClose => return UiWorkspaceAction::ClosedApp(hit.id),
                        HitKind::WorkspaceSplit => {
                            return UiWorkspaceAction::SplitRequested {
                                app_id: hit.id,
                                axis: UiTileAxis::Horizontal,
                            };
                        }
                        _ => {}
                    }
                }
                let Some(app_id) = self.app_id_at(x, y) else {
                    return UiWorkspaceAction::None;
                };
                self.focused_app = Some(app_id);
                self.route_to_app(scene, app_id, UiEvent::PointerDown { x, y })
            }
            UiEvent::PointerMove { x, y } => {
                let app_id = self.app_id_at(x, y).or(self.focused_app);
                app_id
                    .map(|app_id| self.route_to_app(scene, app_id, UiEvent::PointerMove { x, y }))
                    .unwrap_or(UiWorkspaceAction::None)
            }
            UiEvent::PointerUp { x, y } => self
                .focused_app
                .map(|app_id| self.route_to_app(scene, app_id, UiEvent::PointerUp { x, y }))
                .unwrap_or(UiWorkspaceAction::None),
            UiEvent::Wheel { x, y, delta_y } => {
                let app_id = self.app_id_at(x, y).or(self.focused_app);
                app_id
                    .map(|app_id| {
                        self.route_to_app(scene, app_id, UiEvent::Wheel { x, y, delta_y })
                    })
                    .unwrap_or(UiWorkspaceAction::None)
            }
            UiEvent::KeyDown { key } => self
                .focused_app
                .map(|app_id| self.route_to_app(scene, app_id, UiEvent::KeyDown { key }))
                .unwrap_or(UiWorkspaceAction::None),
            UiEvent::TextInput(value) => self
                .focused_app
                .map(|app_id| self.route_to_app(scene, app_id, UiEvent::TextInput(value)))
                .unwrap_or(UiWorkspaceAction::None),
            UiEvent::Blur => {
                let mut last = UiWorkspaceAction::None;
                for app in &mut self.apps {
                    let action = app.runtime.handle_event(scene, UiEvent::Blur);
                    if action != UiAction::None {
                        last = UiWorkspaceAction::AppAction {
                            app_id: app.id,
                            action,
                        };
                    }
                }
                last
            }
        }
    }

    fn route_to_app(&mut self, scene: &GpuScene, app_id: u32, event: UiEvent) -> UiWorkspaceAction {
        let Some(app) = self.app_mut(app_id) else {
            return UiWorkspaceAction::None;
        };
        let action = app.runtime.handle_event(scene, event);
        if action == UiAction::None {
            UiWorkspaceAction::None
        } else {
            UiWorkspaceAction::AppAction { app_id, action }
        }
    }

    fn app_id_at(&self, x: f32, y: f32) -> Option<u32> {
        self.apps
            .iter()
            .find(|app| app.bounds.is_some_and(|bounds| bounds.contains(x, y)))
            .map(|app| app.id)
    }

    pub fn open_or_focus(&mut self, kind: UiAppKind) -> u32 {
        let app_id = app_id_for_kind(kind);
        if self.app(app_id).is_none() {
            self.apps.push(app_surface_for_kind(kind));
        }
        self.focused_app = Some(app_id);
        if self.app(app_id).is_some_and(|app| app.full_screen) {
            self.root = UiTileNode::Leaf { app_id };
        } else {
            self.select_or_insert_tab(app_id);
        }
        app_id
    }

    fn select_or_insert_tab(&mut self, app_id: u32) {
        if select_or_insert_tab_node(&mut self.root, app_id) {
            return;
        }
        self.root = UiTileNode::Tabs {
            app_ids: vec![app_id],
            selected: 0,
        };
    }

    fn render_tile(
        &mut self,
        ui: &mut UiPainter<'_, '_>,
        rect: UiRect,
        tile: &UiTileNode,
        render_app: &mut impl FnMut(&mut UiPainter<'_, '_>, UiRect, &UiAppSurface),
    ) {
        match tile {
            UiTileNode::Leaf { app_id } => self.render_leaf(ui, rect, *app_id, render_app),
            UiTileNode::Split {
                axis,
                ratio_percent,
                first,
                second,
            } => {
                let ratio = (*ratio_percent as f32 / 100.0).clamp(0.18, 0.82);
                match axis {
                    UiTileAxis::Horizontal => {
                        let first_w = ((rect.w - WORKSPACE_GAP) * ratio).max(0.0);
                        let second_w = (rect.w - WORKSPACE_GAP - first_w).max(0.0);
                        self.render_tile(
                            ui,
                            UiRect::new(rect.x, rect.y, first_w, rect.h),
                            first,
                            render_app,
                        );
                        self.render_tile(
                            ui,
                            UiRect::new(rect.x + first_w + WORKSPACE_GAP, rect.y, second_w, rect.h),
                            second,
                            render_app,
                        );
                    }
                    UiTileAxis::Vertical => {
                        let first_h = ((rect.h - WORKSPACE_GAP) * ratio).max(0.0);
                        let second_h = (rect.h - WORKSPACE_GAP - first_h).max(0.0);
                        self.render_tile(
                            ui,
                            UiRect::new(rect.x, rect.y, rect.w, first_h),
                            first,
                            render_app,
                        );
                        self.render_tile(
                            ui,
                            UiRect::new(rect.x, rect.y + first_h + WORKSPACE_GAP, rect.w, second_h),
                            second,
                            render_app,
                        );
                    }
                }
            }
            UiTileNode::Tabs { app_ids, selected } => {
                self.render_tab_strip(ui, rect, app_ids, *selected);
                if let Some(app_id) = app_ids.get(*selected) {
                    self.render_app_body(
                        ui,
                        UiRect::new(
                            rect.x,
                            rect.y + WORKSPACE_CHROME_H,
                            rect.w,
                            (rect.h - WORKSPACE_CHROME_H).max(0.0),
                        ),
                        *app_id,
                        render_app,
                    );
                }
            }
        }
    }

    fn render_leaf(
        &mut self,
        ui: &mut UiPainter<'_, '_>,
        rect: UiRect,
        app_id: u32,
        render_app: &mut impl FnMut(&mut UiPainter<'_, '_>, UiRect, &UiAppSurface),
    ) {
        if self.app(app_id).is_some_and(|app| app.full_screen) {
            self.render_app_body(ui, rect, app_id, render_app);
            return;
        }
        self.render_app_chrome(ui, rect, app_id);
        self.render_app_body(
            ui,
            UiRect::new(
                rect.x,
                rect.y + WORKSPACE_CHROME_H,
                rect.w,
                (rect.h - WORKSPACE_CHROME_H).max(0.0),
            ),
            app_id,
            render_app,
        );
    }

    fn render_tab_strip(
        &self,
        ui: &mut UiPainter<'_, '_>,
        rect: UiRect,
        app_ids: &[u32],
        selected: usize,
    ) {
        ui.fill_rect(
            UiRect::new(rect.x, rect.y, rect.w, WORKSPACE_CHROME_H),
            8.0,
            palette::TOPBAR,
        );
        let mut x = rect.x + 8.0;
        for (index, app_id) in app_ids.iter().copied().enumerate() {
            let title = self
                .app(app_id)
                .map(|app| app.title.as_str())
                .unwrap_or("App");
            let w = (title.chars().count() as f32 * 8.0 + 42.0).clamp(82.0, 180.0);
            let tab = UiRect::new(x, rect.y + 5.0, w, 24.0);
            ui.hit(HitKind::WorkspaceTab, app_id, tab.x, tab.y, tab.w, tab.h);
            ui.fill_rect(
                tab,
                8.0,
                if index == selected {
                    palette::ACTIVE_ROW
                } else {
                    palette::ROW
                },
            );
            ui.bounded_label(
                tab.x + 12.0,
                tab.y + 6.0,
                tab.w - 24.0,
                title,
                2.0,
                if index == selected {
                    palette::TEXT
                } else {
                    palette::MUTED
                },
            );
            x += w + 6.0;
        }
    }

    fn render_app_chrome(&self, ui: &mut UiPainter<'_, '_>, rect: UiRect, app_id: u32) {
        let chrome = UiRect::new(rect.x, rect.y, rect.w, WORKSPACE_CHROME_H);
        let focused = self.focused_app == Some(app_id);
        ui.fill_rect(chrome, 8.0, palette::TOPBAR);
        ui.border_rect(
            rect,
            8.0,
            if focused {
                palette::ACCENT.with_alpha(0.68)
            } else {
                palette::BORDER
            },
        );
        let title = self
            .app(app_id)
            .map(|app| app.title.as_str())
            .unwrap_or("App");
        ui.hit(
            HitKind::WorkspaceTab,
            app_id,
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
            if focused {
                palette::TEXT
            } else {
                palette::MUTED
            },
        );
        if chrome.w > 110.0 {
            let split_rect = UiRect::new(chrome.x + chrome.w - 60.0, chrome.y + 6.0, 22.0, 22.0);
            ui.hit(
                HitKind::WorkspaceSplit,
                app_id,
                split_rect.x,
                split_rect.y,
                split_rect.w,
                split_rect.h,
            );
            ui.icon(split_rect.inset(3.0, 3.0), UiIcon::Route, palette::MUTED);
            let close_rect = UiRect::new(chrome.x + chrome.w - 31.0, chrome.y + 6.0, 22.0, 22.0);
            ui.hit(
                HitKind::WorkspaceClose,
                app_id,
                close_rect.x,
                close_rect.y,
                close_rect.w,
                close_rect.h,
            );
            ui.icon(close_rect.inset(3.0, 3.0), UiIcon::X, palette::MUTED);
        }
    }

    fn render_app_body(
        &mut self,
        ui: &mut UiPainter<'_, '_>,
        rect: UiRect,
        app_id: u32,
        render_app: &mut impl FnMut(&mut UiPainter<'_, '_>, UiRect, &UiAppSurface),
    ) {
        if rect.w <= 0.0 || rect.h <= 0.0 {
            return;
        }
        if let Some(app) = self.app_mut(app_id) {
            app.bounds = Some(rect);
        }
        let clipped = ui
            .scene
            .push_clip(GpuClip::new(rect.x, rect.y, rect.w, rect.h));
        if clipped {
            if let Some(app) = self.app(app_id) {
                render_app(ui, rect, app);
            }
            ui.scene.pop_clip();
        }
    }
}

fn select_or_insert_tab_node(tile: &mut UiTileNode, app_id: u32) -> bool {
    match tile {
        UiTileNode::Tabs { app_ids, selected } => {
            if let Some(index) = app_ids.iter().position(|id| *id == app_id) {
                *selected = index;
            } else {
                app_ids.push(app_id);
                *selected = app_ids.len() - 1;
            }
            true
        }
        UiTileNode::Split { first, second, .. } => {
            select_or_insert_tab_node(second, app_id) || select_or_insert_tab_node(first, app_id)
        }
        UiTileNode::Leaf { .. } => false,
    }
}
