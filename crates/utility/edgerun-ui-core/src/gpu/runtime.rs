use super::GpuScene;
use std::string::String;
use std::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitKind {
    Contact,
    Composer,
    Send,
    Button,
    Tab,
    Toggle,
    ListRow,
    Input,
    TextArea,
    Slider,
    Checkbox,
    Radio,
    Select,
    Breadcrumb,
    TreeItem,
    MenuItem,
    TransactionRow,
    Scrollbar,
    WorkspaceTab,
    WorkspaceClose,
    WorkspaceSplit,
    ShellLauncher,
    AppLauncherItem,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuHit {
    pub kind: HitKind,
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl GpuHit {
    pub const fn new(kind: HitKind, id: u32, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            kind,
            id,
            x,
            y,
            w,
            h,
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.w && y <= self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiKey {
    Backspace,
    Enter,
    Escape,
    Tab,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Other(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    PointerDown { x: f32, y: f32 },
    PointerMove { x: f32, y: f32 },
    PointerUp { x: f32, y: f32 },
    Wheel { x: f32, y: f32, delta_y: f32 },
    KeyDown { key: UiKey },
    TextInput(String),
    Blur,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiAction {
    None,
    Hovered(Option<GpuHit>),
    Focused(Option<GpuHit>),
    Activated(GpuHit),
    Toggled { id: u32, on: bool },
    TabSelected { id: u32 },
    SliderChanged { id: u32, value: f32 },
    ScrollChanged { id: u32, offset: f32 },
    TextChanged { id: u32, value: String },
    Submitted { id: u32 },
    Cancelled,
}

#[derive(Clone, Debug, Default)]
pub struct UiRuntimeState {
    hovered: Option<GpuHit>,
    active: Option<GpuHit>,
    focused: Option<GpuHit>,
    scroll_offsets: Vec<(u32, f32)>,
    toggle_values: Vec<(u32, bool)>,
    slider_values: Vec<(u32, f32)>,
    text_values: Vec<(u32, String)>,
}

impl UiRuntimeState {
    pub fn hovered(&self) -> Option<GpuHit> {
        self.hovered
    }

    pub fn active(&self) -> Option<GpuHit> {
        self.active
    }

    pub fn focused(&self) -> Option<GpuHit> {
        self.focused
    }

    pub fn scroll_offset(&self, id: u32) -> f32 {
        self.scroll_offsets
            .iter()
            .find_map(|(stored_id, value)| (*stored_id == id).then_some(*value))
            .unwrap_or(0.0)
    }

    pub fn toggle_value(&self, id: u32, fallback: bool) -> bool {
        self.toggle_values
            .iter()
            .find_map(|(stored_id, value)| (*stored_id == id).then_some(*value))
            .unwrap_or(fallback)
    }

    pub fn slider_value(&self, id: u32, fallback: f32) -> f32 {
        self.slider_values
            .iter()
            .find_map(|(stored_id, value)| (*stored_id == id).then_some(*value))
            .unwrap_or(fallback)
            .clamp(0.0, 1.0)
    }

    pub fn text_value<'a>(&'a self, id: u32, fallback: &'a str) -> &'a str {
        self.text_values
            .iter()
            .find_map(|(stored_id, value)| (*stored_id == id).then_some(value.as_str()))
            .unwrap_or(fallback)
    }

    pub fn set_scroll_offset(&mut self, id: u32, offset: f32) {
        set_pair_f32(&mut self.scroll_offsets, id, offset.clamp(0.0, 1.0));
    }

    pub fn handle_event(&mut self, scene: &GpuScene, event: UiEvent) -> UiAction {
        match event {
            UiEvent::PointerDown { x, y } => {
                let hit = scene.hit_test(x, y);
                self.active = hit;
                if let Some(hit) = hit.filter(|hit| is_focusable_hit(*hit)) {
                    self.focused = Some(hit);
                    if is_text_hit(hit) {
                        UiAction::Focused(Some(hit))
                    } else {
                        UiAction::Activated(hit)
                    }
                } else {
                    self.focused = None;
                    hit.map(UiAction::Activated)
                        .unwrap_or(UiAction::Focused(None))
                }
            }
            UiEvent::PointerMove { x, y } => {
                let hovered = scene.hit_test(x, y);
                if hovered != self.hovered {
                    self.hovered = hovered;
                    if let Some(action) = self.drag_action(x, y) {
                        return action;
                    }
                    UiAction::Hovered(hovered)
                } else {
                    self.drag_action(x, y).unwrap_or(UiAction::None)
                }
            }
            UiEvent::PointerUp { x, y } => {
                let active = self.active.take();
                let released = scene.hit_test(x, y);
                let Some(active) = active else {
                    return UiAction::None;
                };
                if !released.is_some_and(|hit| hit.kind == active.kind && hit.id == active.id) {
                    return UiAction::None;
                }
                self.activate_hit(active, x)
            }
            UiEvent::Wheel { x, y, delta_y } => {
                let Some(hit) = scene
                    .hit_test(x, y)
                    .or(self.hovered)
                    .filter(|hit| hit.kind == HitKind::Scrollbar)
                else {
                    return UiAction::None;
                };
                let current = self.scroll_offset(hit.id);
                let next = (current + delta_y / 900.0).clamp(0.0, 1.0);
                self.set_scroll_offset(hit.id, next);
                UiAction::ScrollChanged {
                    id: hit.id,
                    offset: next,
                }
            }
            UiEvent::KeyDown { key } => self.handle_key(scene, key),
            UiEvent::TextInput(value) => self.handle_text_input(&value),
            UiEvent::Blur => {
                self.hovered = None;
                self.active = None;
                self.focused = None;
                UiAction::Focused(None)
            }
        }
    }

    fn activate_hit(&mut self, hit: GpuHit, x: f32) -> UiAction {
        match hit.kind {
            HitKind::Toggle | HitKind::Checkbox => {
                let next = !self.toggle_value(hit.id, false);
                set_pair_bool(&mut self.toggle_values, hit.id, next);
                UiAction::Toggled {
                    id: hit.id,
                    on: next,
                }
            }
            HitKind::Radio => {
                set_pair_bool(&mut self.toggle_values, hit.id, true);
                UiAction::Toggled {
                    id: hit.id,
                    on: true,
                }
            }
            HitKind::Tab => UiAction::TabSelected { id: hit.id },
            HitKind::Slider => self.set_slider_from_pointer(hit, x),
            HitKind::Input | HitKind::TextArea | HitKind::Composer => {
                self.focused = Some(hit);
                UiAction::Focused(Some(hit))
            }
            _ => UiAction::Activated(hit),
        }
    }

    fn drag_action(&mut self, x: f32, y: f32) -> Option<UiAction> {
        match self.active {
            Some(hit) if hit.kind == HitKind::Slider => Some(self.set_slider_from_pointer(hit, x)),
            Some(hit) if hit.kind == HitKind::Scrollbar => {
                let value = ((y - hit.y) / hit.h.max(1.0)).clamp(0.0, 1.0);
                self.set_scroll_offset(hit.id, value);
                Some(UiAction::ScrollChanged {
                    id: hit.id,
                    offset: value,
                })
            }
            _ => None,
        }
    }

    fn set_slider_from_pointer(&mut self, hit: GpuHit, x: f32) -> UiAction {
        let value = ((x - hit.x) / hit.w.max(1.0)).clamp(0.0, 1.0);
        set_pair_f32(&mut self.slider_values, hit.id, value);
        UiAction::SliderChanged { id: hit.id, value }
    }

    fn handle_key(&mut self, scene: &GpuScene, key: UiKey) -> UiAction {
        if matches!(key, UiKey::Tab) {
            return self.focus_next_in_scene(scene, false);
        }
        let Some(hit) = self.focused else {
            return UiAction::None;
        };
        match key {
            UiKey::Backspace if is_text_hit(hit) => {
                let value = text_value_mut(&mut self.text_values, hit.id);
                value.pop();
                UiAction::TextChanged {
                    id: hit.id,
                    value: value.clone(),
                }
            }
            UiKey::Enter if hit.kind == HitKind::Composer || hit.kind == HitKind::TextArea => {
                UiAction::Submitted { id: hit.id }
            }
            UiKey::Enter if !is_text_hit(hit) => self.activate_hit(hit, hit.x + hit.w * 0.5),
            UiKey::Escape => {
                self.focused = None;
                UiAction::Cancelled
            }
            UiKey::ArrowDown | UiKey::ArrowRight if !is_text_hit(hit) => {
                self.focus_next_in_scene(scene, false)
            }
            UiKey::ArrowUp | UiKey::ArrowLeft if !is_text_hit(hit) => {
                self.focus_next_in_scene(scene, true)
            }
            _ => UiAction::None,
        }
    }

    pub fn focus_first(&mut self, scene: &GpuScene) -> UiAction {
        let Some(hit) = scene
            .hits()
            .iter()
            .copied()
            .find(|hit| is_focusable_hit(*hit))
        else {
            self.focused = None;
            return UiAction::Focused(None);
        };
        self.focused = Some(hit);
        UiAction::Focused(Some(hit))
    }

    pub fn focus_next_in_scene(&mut self, scene: &GpuScene, reverse: bool) -> UiAction {
        let focusable = scene
            .hits()
            .iter()
            .copied()
            .filter(|hit| is_focusable_hit(*hit))
            .collect::<Vec<_>>();
        if focusable.is_empty() {
            self.focused = None;
            return UiAction::Focused(None);
        }
        let index = self
            .focused
            .and_then(|focused| {
                focusable
                    .iter()
                    .position(|hit| hit.kind == focused.kind && hit.id == focused.id)
            })
            .unwrap_or(if reverse { 0 } else { focusable.len() - 1 });
        let next = if reverse {
            if index == 0 {
                focusable[focusable.len() - 1]
            } else {
                focusable[index - 1]
            }
        } else {
            focusable[(index + 1) % focusable.len()]
        };
        self.focused = Some(next);
        UiAction::Focused(Some(next))
    }

    fn handle_text_input(&mut self, input: &str) -> UiAction {
        let Some(hit) = self.focused.filter(|hit| is_text_hit(*hit)) else {
            return UiAction::None;
        };
        let value = text_value_mut(&mut self.text_values, hit.id);
        for ch in input.chars().filter(|ch| !ch.is_control()) {
            if value.len() >= 4096 {
                break;
            }
            value.push(ch);
        }
        UiAction::TextChanged {
            id: hit.id,
            value: value.clone(),
        }
    }
}

fn is_focusable_hit(hit: GpuHit) -> bool {
    matches!(
        hit.kind,
        HitKind::Button
            | HitKind::Tab
            | HitKind::Toggle
            | HitKind::ListRow
            | HitKind::Input
            | HitKind::TextArea
            | HitKind::Slider
            | HitKind::Checkbox
            | HitKind::Radio
            | HitKind::Select
            | HitKind::Breadcrumb
            | HitKind::TreeItem
            | HitKind::MenuItem
            | HitKind::TransactionRow
            | HitKind::Composer
            | HitKind::Send
            | HitKind::WorkspaceTab
            | HitKind::WorkspaceClose
            | HitKind::WorkspaceSplit
            | HitKind::ShellLauncher
            | HitKind::AppLauncherItem
    )
}

fn is_text_hit(hit: GpuHit) -> bool {
    matches!(
        hit.kind,
        HitKind::Input | HitKind::TextArea | HitKind::Composer
    )
}

fn set_pair_f32(values: &mut Vec<(u32, f32)>, id: u32, value: f32) {
    if let Some((_, stored)) = values.iter_mut().find(|(stored_id, _)| *stored_id == id) {
        *stored = value;
    } else {
        values.push((id, value));
    }
}

fn set_pair_bool(values: &mut Vec<(u32, bool)>, id: u32, value: bool) {
    if let Some((_, stored)) = values.iter_mut().find(|(stored_id, _)| *stored_id == id) {
        *stored = value;
    } else {
        values.push((id, value));
    }
}

fn text_value_mut(values: &mut Vec<(u32, String)>, id: u32) -> &mut String {
    if let Some(index) = values.iter().position(|(stored_id, _)| *stored_id == id) {
        &mut values[index].1
    } else {
        values.push((id, String::new()));
        &mut values.last_mut().expect("inserted text value").1
    }
}
