use super::{GpuDragSource, GpuDropTarget, GpuScene, UiRect, UiTransitionSpec};
use std::string::String;
use std::vec::Vec;

/// Runtime hit classes. Hosts must route input by the `(HitKind, id)` pair
/// emitted by Rust scene construction and must not reinterpret ids globally.
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
    ScrollArea,
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
    /// Stable within its `HitKind` owner for the current scene contract.
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
    Delete,
    Enter,
    Escape,
    Tab,
    Home,
    End,
    PageUp,
    PageDown,
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
    DragStarted {
        source: GpuDragSource,
    },
    DragMoved {
        source: GpuDragSource,
        target: Option<GpuDropTarget>,
    },
    Dropped {
        source: GpuDragSource,
        target: Option<GpuDropTarget>,
    },
    Reordered {
        scope_id: u32,
        item_id: u32,
        from: usize,
        to: usize,
    },
    DragCancelled {
        source: GpuDragSource,
    },
    Toggled {
        id: u32,
        on: bool,
    },
    TabSelected {
        id: u32,
    },
    SliderChanged {
        id: u32,
        value: f32,
    },
    OpenChanged {
        id: u32,
        open: bool,
    },
    ScrollChanged {
        id: u32,
        offset: f32,
    },
    TextChanged {
        id: u32,
        value: String,
    },
    Submitted {
        id: u32,
    },
    Cancelled,
}

impl UiAction {
    pub const fn needs_redraw(&self) -> bool {
        !matches!(self, Self::None | Self::Hovered(_))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiFrameInput {
    pub viewport: Option<UiRect>,
    pub scale: f32,
    pub delta_ms: u32,
    pub events: Vec<UiEvent>,
}

impl UiFrameInput {
    pub fn new(delta_ms: u32) -> Self {
        Self {
            viewport: None,
            scale: 1.0,
            delta_ms,
            events: Vec::new(),
        }
    }

    pub fn with_viewport(mut self, viewport: UiRect, scale: f32) -> Self {
        self.viewport = Some(viewport);
        self.scale = scale.max(0.1);
        self
    }

    pub fn with_event(mut self, event: UiEvent) -> Self {
        self.events.push(event);
        self
    }

    pub fn with_events(mut self, events: impl IntoIterator<Item = UiEvent>) -> Self {
        self.events.extend(events);
        self
    }
}

impl Default for UiFrameInput {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiFrameOutput {
    pub actions: Vec<UiAction>,
    pub needs_redraw: bool,
    pub transitions_active: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiFrameReplay {
    pub outputs: Vec<UiFrameOutput>,
}

impl UiFrameReplay {
    pub fn actions(&self) -> Vec<UiAction> {
        self.outputs
            .iter()
            .flat_map(|output| output.actions.iter().cloned())
            .collect()
    }

    pub fn needs_redraw(&self) -> bool {
        self.outputs.iter().any(|output| output.needs_redraw)
    }

    pub fn transitions_active(&self) -> bool {
        self.outputs.iter().any(|output| output.transitions_active)
    }
}

const DRAG_START_THRESHOLD_PX: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiDragSession {
    pub source: GpuDragSource,
    pub target: Option<GpuDropTarget>,
    pub current_x: f32,
    pub current_y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct UiDragState {
    source: GpuDragSource,
    target: Option<GpuDropTarget>,
    start_x: f32,
    start_y: f32,
    current_x: f32,
    current_y: f32,
    started: bool,
}

impl UiDragState {
    fn session(self) -> UiDragSession {
        UiDragSession {
            source: self.source,
            target: self.target,
            current_x: self.current_x,
            current_y: self.current_y,
        }
    }

    fn moved_far_enough(&self, x: f32, y: f32) -> bool {
        let dx = x - self.start_x;
        let dy = y - self.start_y;
        dx * dx + dy * dy >= DRAG_START_THRESHOLD_PX * DRAG_START_THRESHOLD_PX
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct UiTransitionState {
    id: u32,
    elapsed_ms: u32,
    total_ms: u32,
}

impl UiTransitionState {
    fn new(spec: UiTransitionSpec) -> Self {
        Self {
            id: spec.id,
            elapsed_ms: 0,
            total_ms: spec.delay_ms.saturating_add(spec.duration_ms),
        }
    }

    fn value(self, spec: UiTransitionSpec) -> f32 {
        if self.elapsed_ms <= spec.delay_ms {
            return spec.from;
        }
        let active_ms = self.elapsed_ms - spec.delay_ms;
        let t = active_ms as f32 / spec.duration_ms.max(1) as f32;
        let eased = spec.easing.sample(t);
        spec.from + (spec.to - spec.from) * eased
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiTextBufferAction {
    None,
    Changed,
    Submit,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiKeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl UiKeyModifiers {
    pub const fn shift(shift: bool) -> Self {
        Self {
            shift,
            ctrl: false,
            alt: false,
            meta: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiTextBuffer {
    value: String,
    cursor: usize,
}

impl UiTextBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.value = text.into();
        self.cursor = self.value.chars().count();
    }

    pub fn insert(&mut self, text: &str) {
        let byte_index = self.cursor_byte_index();
        self.value.insert_str(byte_index, text);
        self.cursor += text.chars().count();
    }

    pub fn handle_text_input(&mut self, text: &str) -> UiTextBufferAction {
        let before = self.value.len();
        for ch in text.chars().filter(|ch| !ch.is_control()) {
            if self.value.len() >= 4096 {
                break;
            }
            self.insert(&ch.to_string());
        }
        if self.value.len() == before {
            UiTextBufferAction::None
        } else {
            UiTextBufferAction::Changed
        }
    }

    pub fn handle_key(&mut self, key: UiKey, shift: bool) -> UiTextBufferAction {
        self.handle_key_with_modifiers(key, UiKeyModifiers::shift(shift))
    }

    pub fn handle_key_with_modifiers(
        &mut self,
        key: UiKey,
        modifiers: UiKeyModifiers,
    ) -> UiTextBufferAction {
        match key {
            UiKey::Other(code)
                if modifiers.ctrl && (code == b'a' as u32 || code == b'A' as u32) =>
            {
                self.move_cursor_to_start();
                UiTextBufferAction::Changed
            }
            UiKey::Other(code)
                if modifiers.ctrl && (code == b'e' as u32 || code == b'E' as u32) =>
            {
                self.move_cursor_to_end();
                UiTextBufferAction::Changed
            }
            UiKey::Other(code)
                if modifiers.ctrl && (code == b'u' as u32 || code == b'U' as u32) =>
            {
                let before = self.value.len();
                self.delete_before_cursor_all();
                if self.value.len() == before {
                    UiTextBufferAction::None
                } else {
                    UiTextBufferAction::Changed
                }
            }
            UiKey::Other(code)
                if modifiers.ctrl && (code == b'k' as u32 || code == b'K' as u32) =>
            {
                let before = self.value.len();
                self.delete_after_cursor_all();
                if self.value.len() == before {
                    UiTextBufferAction::None
                } else {
                    UiTextBufferAction::Changed
                }
            }
            UiKey::Other(code)
                if modifiers.ctrl && (code == b'w' as u32 || code == b'W' as u32) =>
            {
                let before = self.value.len();
                self.delete_word_before_cursor();
                if self.value.len() == before {
                    UiTextBufferAction::None
                } else {
                    UiTextBufferAction::Changed
                }
            }
            UiKey::Backspace => {
                let before = self.value.len();
                self.delete_before_cursor();
                if self.value.len() == before {
                    UiTextBufferAction::None
                } else {
                    UiTextBufferAction::Changed
                }
            }
            UiKey::Delete => {
                let before = self.value.len();
                self.delete_after_cursor();
                if self.value.len() == before {
                    UiTextBufferAction::None
                } else {
                    UiTextBufferAction::Changed
                }
            }
            UiKey::Enter if modifiers.shift => {
                self.insert("\n");
                UiTextBufferAction::Changed
            }
            UiKey::Enter => UiTextBufferAction::Submit,
            UiKey::ArrowLeft => {
                self.move_cursor_left();
                UiTextBufferAction::Changed
            }
            UiKey::ArrowRight => {
                self.move_cursor_right();
                UiTextBufferAction::Changed
            }
            UiKey::Home => {
                self.move_cursor_to_start();
                UiTextBufferAction::Changed
            }
            UiKey::End => {
                self.move_cursor_to_end();
                UiTextBufferAction::Changed
            }
            _ => UiTextBufferAction::None,
        }
    }

    pub fn delete_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let end = self.cursor_byte_index();
        self.cursor -= 1;
        let start = self.cursor_byte_index();
        self.value.replace_range(start..end, "");
    }

    pub fn delete_after_cursor(&mut self) {
        let start = self.cursor_byte_index();
        if start == self.value.len() {
            return;
        }
        self.cursor += 1;
        let end = self.cursor_byte_index();
        self.cursor -= 1;
        self.value.replace_range(start..end, "");
    }

    pub fn delete_before_cursor_all(&mut self) {
        let end = self.cursor_byte_index();
        self.value.replace_range(..end, "");
        self.cursor = 0;
    }

    pub fn delete_after_cursor_all(&mut self) {
        let start = self.cursor_byte_index();
        self.value.truncate(start);
    }

    pub fn delete_word_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let chars = self.value.chars().collect::<Vec<_>>();
        let end = self.cursor_byte_index();
        let mut start_cursor = self.cursor.min(chars.len());
        while start_cursor > 0 && chars[start_cursor - 1].is_whitespace() {
            start_cursor -= 1;
        }
        while start_cursor > 0 && !chars[start_cursor - 1].is_whitespace() {
            start_cursor -= 1;
        }
        self.cursor = start_cursor;
        let start = self.cursor_byte_index();
        self.value.replace_range(start..end, "");
    }

    pub fn move_cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.value.chars().count());
    }

    pub fn move_cursor_to_start(&mut self) {
        self.cursor = 0;
    }

    pub fn move_cursor_to_end(&mut self) {
        self.cursor = self.value.chars().count();
    }

    pub fn value_with_cursor(&self, marker: char) -> String {
        let byte_index = self.cursor_byte_index();
        let mut text = String::with_capacity(self.value.len() + marker.len_utf8());
        text.push_str(&self.value[..byte_index]);
        text.push(marker);
        text.push_str(&self.value[byte_index..]);
        text
    }

    pub fn display_value(&self, placeholder: &str, cursor_marker: char) -> String {
        if self.value.is_empty() {
            placeholder.to_string()
        } else {
            self.value_with_cursor(cursor_marker)
        }
    }

    fn cursor_byte_index(&self) -> usize {
        self.value
            .char_indices()
            .nth(self.cursor)
            .map(|(index, _)| index)
            .unwrap_or(self.value.len())
    }
}

#[derive(Clone, Debug, Default)]
pub struct UiRuntimeState {
    hovered: Option<GpuHit>,
    active: Option<GpuHit>,
    focused: Option<GpuHit>,
    drag: Option<UiDragState>,
    transitions: Vec<UiTransitionState>,
    scroll_offsets: Vec<(u32, f32)>,
    toggle_values: Vec<(u32, bool)>,
    slider_values: Vec<(u32, f32)>,
    text_values: Vec<(u32, String)>,
    open_values: Vec<(u32, bool)>,
    focus_scopes: Vec<(u32, Vec<(HitKind, u32)>)>,
    selected_tab_ids: Vec<u32>,
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

    pub fn drag_session(&self) -> Option<UiDragSession> {
        self.drag.map(UiDragState::session)
    }

    pub fn transition_value(&self, spec: UiTransitionSpec) -> f32 {
        self.transitions
            .iter()
            .find(|state| state.id == spec.id)
            .map(|state| state.value(spec))
            .unwrap_or(spec.to)
    }

    pub fn sync_transitions(&mut self, scene: &GpuScene) -> bool {
        let before = self.transitions.len();
        let mut changed = false;
        for spec in scene.transitions() {
            if !self.transitions.iter().any(|state| state.id == spec.id) {
                self.transitions.push(UiTransitionState::new(*spec));
                changed = true;
            }
        }
        self.transitions
            .retain(|state| scene.transitions().iter().any(|spec| spec.id == state.id));
        changed || self.transitions.len() != before
    }

    pub fn advance_transitions(&mut self, delta_ms: u32) -> bool {
        let mut needs_redraw = false;
        for state in &mut self.transitions {
            if state.elapsed_ms < state.total_ms {
                state.elapsed_ms = state
                    .elapsed_ms
                    .saturating_add(delta_ms)
                    .min(state.total_ms);
                needs_redraw = true;
            }
        }
        needs_redraw
    }

    pub fn transitions_active(&self) -> bool {
        self.transitions
            .iter()
            .any(|state| state.elapsed_ms < state.total_ms)
    }

    pub fn handle_frame(&mut self, scene: &GpuScene, input: UiFrameInput) -> UiFrameOutput {
        let mut output = UiFrameOutput::default();
        output.needs_redraw |= self.sync_transitions(scene);
        output.needs_redraw |= self.advance_transitions(input.delta_ms);

        for event in input.events {
            let action = self.handle_event(scene, event);
            output.needs_redraw |= action.needs_redraw();
            if action != UiAction::None {
                output.actions.push(action);
            }
        }

        output.transitions_active = self.transitions_active();
        output.needs_redraw |= output.transitions_active;
        output
    }

    pub fn replay_frames(
        &mut self,
        scene: &GpuScene,
        frames: impl IntoIterator<Item = UiFrameInput>,
    ) -> UiFrameReplay {
        UiFrameReplay {
            outputs: frames
                .into_iter()
                .map(|input| self.handle_frame(scene, input))
                .collect(),
        }
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

    pub fn open_value(&self, id: u32, fallback: bool) -> bool {
        self.open_values
            .iter()
            .find_map(|(stored_id, value)| (*stored_id == id).then_some(*value))
            .unwrap_or(fallback)
    }

    pub fn selected_tab_index(&self, base_id: u32, len: usize, fallback: usize) -> usize {
        let Some(selected_id) = self
            .selected_tab_ids
            .iter()
            .rev()
            .copied()
            .find(|id| *id >= base_id && *id < base_id + len as u32)
        else {
            return fallback.min(len.saturating_sub(1));
        };
        (selected_id - base_id) as usize
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

    pub fn set_open(&mut self, id: u32, open: bool) {
        if let Some(index) = self
            .open_values
            .iter()
            .position(|(stored_id, _)| *stored_id == id)
        {
            self.open_values[index].1 = open;
            if open {
                let entry = self.open_values.remove(index);
                self.open_values.push(entry);
            }
        } else {
            self.open_values.push((id, open));
        }
        if !open {
            self.clear_focus_scope(id);
        }
    }

    pub fn set_focus_scope(&mut self, open_id: u32, hits: &[GpuHit]) {
        let scope = hits
            .iter()
            .copied()
            .filter(|hit| is_focusable_hit(*hit))
            .map(|hit| (hit.kind, hit.id))
            .collect::<Vec<_>>();
        if let Some((_, stored_scope)) = self
            .focus_scopes
            .iter_mut()
            .find(|(stored_id, _)| *stored_id == open_id)
        {
            *stored_scope = scope;
        } else {
            self.focus_scopes.push((open_id, scope));
        }
    }

    pub fn clear_focus_scope(&mut self, open_id: u32) {
        self.focus_scopes
            .retain(|(stored_id, _)| *stored_id != open_id);
    }

    pub fn active_focus_scope_id(&self) -> Option<u32> {
        self.focus_scopes
            .iter()
            .rev()
            .map(|(open_id, _)| *open_id)
            .find(|open_id| self.open_value(*open_id, false))
    }

    pub fn handle_event(&mut self, scene: &GpuScene, event: UiEvent) -> UiAction {
        match event {
            UiEvent::PointerDown { x, y } => {
                let raw_hit = scene.hit_test(x, y);
                if raw_hit.is_some_and(|hit| !self.hit_allowed_by_focus_scope(hit)) {
                    self.active = None;
                    self.drag = None;
                    return UiAction::None;
                }
                let hit = raw_hit;
                self.active = hit;
                self.drag = scene.drag_source_at(x, y).map(|source| UiDragState {
                    source,
                    target: None,
                    start_x: x,
                    start_y: y,
                    current_x: x,
                    current_y: y,
                    started: false,
                });
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
                if let Some(action) = self.drag_pointer_move(scene, x, y) {
                    return action;
                }
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
                if let Some(action) = self.drag_pointer_up(scene, x, y) {
                    return action;
                }
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
                    .hits()
                    .iter()
                    .rev()
                    .copied()
                    .find(|hit| {
                        matches!(hit.kind, HitKind::ScrollArea | HitKind::Scrollbar)
                            && hit.contains(x, y)
                    })
                    .or_else(|| {
                        self.hovered.filter(|hit| {
                            matches!(hit.kind, HitKind::ScrollArea | HitKind::Scrollbar)
                        })
                    })
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
                if let Some(drag) = self.drag.take().filter(|drag| drag.started) {
                    UiAction::DragCancelled {
                        source: drag.source,
                    }
                } else {
                    UiAction::Focused(None)
                }
            }
        }
    }

    fn drag_pointer_move(&mut self, scene: &GpuScene, x: f32, y: f32) -> Option<UiAction> {
        let drag = self.drag.as_mut()?;
        drag.current_x = x;
        drag.current_y = y;
        if !drag.started {
            if !drag.moved_far_enough(x, y) {
                return None;
            }
            drag.started = true;
            drag.target = scene.drop_target_at(x, y, drag.source.scope_id);
            return Some(UiAction::DragStarted {
                source: drag.source,
            });
        }

        let target = scene.drop_target_at(x, y, drag.source.scope_id);
        if target != drag.target {
            drag.target = target;
            return Some(UiAction::DragMoved {
                source: drag.source,
                target,
            });
        }
        None
    }

    fn drag_pointer_up(&mut self, scene: &GpuScene, x: f32, y: f32) -> Option<UiAction> {
        let mut drag = self.drag.take()?;
        if !drag.started {
            return None;
        }
        drag.current_x = x;
        drag.current_y = y;
        let target = scene
            .drop_target_at(x, y, drag.source.scope_id)
            .or(drag.target);
        self.active = None;
        if let Some(target) = target.filter(|target| target.index != drag.source.index) {
            Some(UiAction::Reordered {
                scope_id: drag.source.scope_id,
                item_id: drag.source.item_id,
                from: drag.source.index,
                to: target.index,
            })
        } else {
            Some(UiAction::Dropped {
                source: drag.source,
                target,
            })
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
            HitKind::Tab => {
                if !self.selected_tab_ids.contains(&hit.id) {
                    self.selected_tab_ids.push(hit.id);
                }
                UiAction::TabSelected { id: hit.id }
            }
            HitKind::Select => {
                let next = !self.open_value(hit.id, false);
                self.set_open(hit.id, next);
                UiAction::OpenChanged {
                    id: hit.id,
                    open: next,
                }
            }
            HitKind::Slider => self.set_slider_from_pointer(hit, x),
            HitKind::Input | HitKind::TextArea | HitKind::Composer => {
                self.focused = Some(hit);
                UiAction::Focused(Some(hit))
            }
            HitKind::ScrollArea => UiAction::None,
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
        if matches!(key, UiKey::Escape) {
            if let Some(drag) = self.drag.take().filter(|drag| drag.started) {
                self.active = None;
                return UiAction::DragCancelled {
                    source: drag.source,
                };
            }
        }
        if matches!(key, UiKey::Tab) {
            return self.focus_next_in_scene(scene, false);
        }
        if matches!(key, UiKey::Escape) {
            if let Some(open_id) = self.close_top_open_scope() {
                return UiAction::OpenChanged {
                    id: open_id,
                    open: false,
                };
            }
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
        let Some(hit) = self.focusable_hits(scene).into_iter().next() else {
            self.focused = None;
            return UiAction::Focused(None);
        };
        self.focused = Some(hit);
        UiAction::Focused(Some(hit))
    }

    pub fn focus_next_in_scene(&mut self, scene: &GpuScene, reverse: bool) -> UiAction {
        let focusable = self.focusable_hits(scene);
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

    fn focusable_hits(&self, scene: &GpuScene) -> Vec<GpuHit> {
        let Some((_, scope)) = self.active_focus_scope() else {
            return scene
                .hits()
                .iter()
                .copied()
                .filter(|hit| is_focusable_hit(*hit))
                .collect();
        };
        scene
            .hits()
            .iter()
            .copied()
            .filter(|hit| is_focusable_hit(*hit))
            .filter(|hit| {
                scope
                    .iter()
                    .any(|(kind, id)| *kind == hit.kind && *id == hit.id)
            })
            .collect()
    }

    fn active_focus_scope(&self) -> Option<(u32, &[(HitKind, u32)])> {
        self.focus_scopes
            .iter()
            .rev()
            .find(|(open_id, scope)| self.open_value(*open_id, false) && !scope.is_empty())
            .map(|(open_id, scope)| (*open_id, scope.as_slice()))
    }

    fn hit_allowed_by_focus_scope(&self, hit: GpuHit) -> bool {
        let Some((_, scope)) = self.active_focus_scope() else {
            return true;
        };
        scope
            .iter()
            .any(|(kind, id)| *kind == hit.kind && *id == hit.id)
    }

    fn close_top_open_scope(&mut self) -> Option<u32> {
        let open_id = self
            .open_values
            .iter()
            .rev()
            .find_map(|(open_id, open)| (*open).then_some(*open_id))?;
        self.set_open(open_id, false);
        self.focused = None;
        Some(open_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::palette;

    #[test]
    fn open_focus_scope_traps_tab_cycle_and_blocks_outside_pointer_focus() {
        let mut scene = GpuScene::new(palette::BG);
        let outside = GpuHit::new(HitKind::Button, 1, 0.0, 0.0, 80.0, 32.0);
        let first = GpuHit::new(HitKind::Button, 10, 100.0, 0.0, 80.0, 32.0);
        let second = GpuHit::new(HitKind::Button, 11, 190.0, 0.0, 80.0, 32.0);
        scene.push_hit(outside);
        scene.push_hit(first);
        scene.push_hit(second);

        let mut state = UiRuntimeState::default();
        state.set_open(99, true);
        state.set_focus_scope(99, &[first, second]);
        assert_eq!(state.active_focus_scope_id(), Some(99));
        assert_eq!(state.focus_first(&scene), UiAction::Focused(Some(first)));
        assert_eq!(
            state.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Tab }),
            UiAction::Focused(Some(second))
        );
        assert_eq!(
            state.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Tab }),
            UiAction::Focused(Some(first))
        );

        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerDown { x: 10.0, y: 10.0 }),
            UiAction::None
        );
        assert_eq!(state.focused(), Some(first));
    }

    #[test]
    fn escape_closes_top_open_scope_before_cancelling_focus() {
        let first = GpuHit::new(HitKind::Button, 10, 100.0, 0.0, 80.0, 32.0);
        let mut state = UiRuntimeState::default();
        state.set_open(1, true);
        state.set_open(2, true);
        state.set_focus_scope(2, &[first]);

        assert_eq!(
            state.handle_event(
                &GpuScene::new(palette::BG),
                UiEvent::KeyDown { key: UiKey::Escape }
            ),
            UiAction::OpenChanged { id: 2, open: false }
        );
        assert!(state.open_value(1, false));
        assert!(!state.open_value(2, true));
        assert_eq!(state.active_focus_scope_id(), None);
    }

    #[test]
    fn wheel_scrolls_scroll_area_content_not_only_scrollbar() {
        let mut scene = GpuScene::new(palette::BG);
        scene.push_hit(GpuHit::new(HitKind::ScrollArea, 77, 0.0, 0.0, 240.0, 160.0));
        scene.push_hit(GpuHit::new(HitKind::Button, 10, 16.0, 16.0, 80.0, 32.0));
        scene.push_hit(GpuHit::new(HitKind::Scrollbar, 77, 228.0, 0.0, 12.0, 160.0));

        let mut state = UiRuntimeState::default();
        assert_eq!(
            state.handle_event(
                &scene,
                UiEvent::Wheel {
                    x: 40.0,
                    y: 32.0,
                    delta_y: 90.0,
                },
            ),
            UiAction::ScrollChanged {
                id: 77,
                offset: 0.1,
            }
        );
    }

    #[test]
    fn drag_reorder_is_emitted_from_runtime_scene_metadata() {
        let mut scene = GpuScene::new(palette::BG);
        scene.push_hit(GpuHit::new(HitKind::ListRow, 77, 0.0, 0.0, 120.0, 40.0));
        scene.push_drag_source(GpuDragSource::new(9, 77, 0, 0.0, 0.0, 120.0, 40.0));
        scene.push_drop_target(GpuDropTarget::new(9, 0, 0.0, 0.0, 120.0, 40.0));
        scene.push_drop_target(GpuDropTarget::new(9, 1, 0.0, 48.0, 120.0, 40.0));

        let mut state = UiRuntimeState::default();
        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerDown { x: 20.0, y: 20.0 }),
            UiAction::Activated(GpuHit::new(HitKind::ListRow, 77, 0.0, 0.0, 120.0, 40.0))
        );
        assert!(matches!(
            state.handle_event(&scene, UiEvent::PointerMove { x: 22.0, y: 22.0 }),
            UiAction::Hovered(Some(_))
        ));
        assert_eq!(state.drag_session().map(|drag| drag.target), Some(None));
        assert!(matches!(
            state.handle_event(&scene, UiEvent::PointerMove { x: 24.0, y: 55.0 }),
            UiAction::DragStarted { source }
                if source.scope_id == 9 && source.item_id == 77 && source.index == 0
        ));
        assert!(matches!(
            state.drag_session(),
            Some(UiDragSession {
                source,
                target: Some(target),
                ..
            }) if source.item_id == 77 && target.index == 1
        ));
        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerUp { x: 24.0, y: 55.0 }),
            UiAction::Reordered {
                scope_id: 9,
                item_id: 77,
                from: 0,
                to: 1
            }
        );
    }

    #[test]
    fn escape_cancels_active_drag_before_other_keyboard_handling() {
        let mut scene = GpuScene::new(palette::BG);
        scene.push_drag_source(GpuDragSource::new(4, 44, 0, 0.0, 0.0, 100.0, 40.0));

        let mut state = UiRuntimeState::default();
        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerDown { x: 10.0, y: 10.0 }),
            UiAction::Focused(None)
        );
        assert!(matches!(
            state.handle_event(&scene, UiEvent::PointerMove { x: 40.0, y: 10.0 }),
            UiAction::DragStarted { .. }
        ));
        assert_eq!(
            state.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Escape }),
            UiAction::DragCancelled {
                source: GpuDragSource::new(4, 44, 0, 0.0, 0.0, 100.0, 40.0)
            }
        );
        assert_eq!(state.drag_session(), None);
    }

    #[test]
    fn transitions_sync_advance_and_ease_in_runtime() {
        let spec = UiTransitionSpec::new(
            88,
            crate::gpu::UiTransitionProperty::Opacity,
            0.0,
            1.0,
            100,
            20,
            crate::gpu::UiTransitionEasing::Linear,
        );
        let mut scene = GpuScene::new(palette::BG);
        scene.push_transition(spec);

        let mut state = UiRuntimeState::default();
        assert_eq!(state.transition_value(spec), 1.0);
        state.sync_transitions(&scene);
        assert_eq!(state.transition_value(spec), 0.0);
        assert!(state.advance_transitions(20));
        assert_eq!(state.transition_value(spec), 0.0);
        assert!(state.advance_transitions(50));
        assert_eq!(state.transition_value(spec), 0.5);
        assert!(state.advance_transitions(50));
        assert_eq!(state.transition_value(spec), 1.0);
        assert!(!state.advance_transitions(1));
    }

    #[test]
    fn frame_runtime_syncs_transitions_advances_time_and_batches_actions() {
        let spec = UiTransitionSpec::new(
            88,
            crate::gpu::UiTransitionProperty::Opacity,
            0.0,
            1.0,
            100,
            0,
            crate::gpu::UiTransitionEasing::Linear,
        );
        let mut scene = GpuScene::new(palette::BG);
        let button = GpuHit::new(HitKind::Button, 7, 0.0, 0.0, 100.0, 40.0);
        scene.push_hit(button);
        scene.push_transition(spec);

        let mut state = UiRuntimeState::default();
        let first = state.handle_frame(
            &scene,
            UiFrameInput::new(50)
                .with_viewport(UiRect::new(0.0, 0.0, 320.0, 240.0), 2.0)
                .with_events([
                    UiEvent::PointerDown { x: 10.0, y: 10.0 },
                    UiEvent::PointerUp { x: 10.0, y: 10.0 },
                ]),
        );

        assert!(first.needs_redraw);
        assert!(first.transitions_active);
        assert_eq!(
            first.actions,
            vec![UiAction::Activated(button), UiAction::Activated(button)]
        );
        assert_eq!(state.transition_value(spec), 0.5);

        let second = state.handle_frame(&scene, UiFrameInput::new(50));
        assert!(second.needs_redraw);
        assert!(!second.transitions_active);
        assert!(second.actions.is_empty());
        assert_eq!(state.transition_value(spec), 1.0);

        let idle = state.handle_frame(&scene, UiFrameInput::default());
        assert!(!idle.needs_redraw);
        assert!(!idle.transitions_active);
        assert!(idle.actions.is_empty());
    }

    #[test]
    fn frame_runtime_drops_stale_transition_state_when_scene_no_longer_declares_it() {
        let spec = UiTransitionSpec::opacity(9, 0.0, 1.0, 100);
        let mut scene = GpuScene::new(palette::BG);
        scene.push_transition(spec);
        let mut state = UiRuntimeState::default();

        assert!(
            state
                .handle_frame(&scene, UiFrameInput::new(10))
                .transitions_active
        );
        assert!(state.transitions_active());

        let empty_scene = GpuScene::new(palette::BG);
        let output = state.handle_frame(&empty_scene, UiFrameInput::default());
        assert!(output.needs_redraw);
        assert!(!output.transitions_active);
        assert!(!state.transitions_active());
        assert_eq!(state.transition_value(spec), 1.0);
    }

    #[test]
    fn frame_replay_produces_deterministic_actions_and_redraw_state() {
        let spec = UiTransitionSpec::new(
            12,
            crate::gpu::UiTransitionProperty::Opacity,
            0.0,
            1.0,
            100,
            0,
            crate::gpu::UiTransitionEasing::Linear,
        );
        let button = GpuHit::new(HitKind::Button, 3, 0.0, 0.0, 64.0, 32.0);
        let mut scene = GpuScene::new(palette::BG);
        scene.push_hit(button);
        scene.push_transition(spec);
        let frames = vec![
            UiFrameInput::new(16).with_event(UiEvent::PointerMove { x: 4.0, y: 4.0 }),
            UiFrameInput::new(34).with_events([
                UiEvent::PointerDown { x: 4.0, y: 4.0 },
                UiEvent::PointerUp { x: 4.0, y: 4.0 },
            ]),
            UiFrameInput::new(50),
        ];

        let mut first_state = UiRuntimeState::default();
        let first = first_state.replay_frames(&scene, frames.clone());
        let mut second_state = UiRuntimeState::default();
        let second = second_state.replay_frames(&scene, frames);

        assert_eq!(first, second);
        assert!(first.needs_redraw());
        assert!(first.transitions_active());
        assert_eq!(
            first.actions(),
            vec![
                UiAction::Hovered(Some(button)),
                UiAction::Activated(button),
                UiAction::Activated(button)
            ]
        );
        assert_eq!(first_state.transition_value(spec), 1.0);
        assert_eq!(second_state.transition_value(spec), 1.0);
    }

    #[test]
    fn text_buffer_edits_at_cursor_with_utf8() {
        let mut buffer = UiTextBuffer::new();
        buffer.set_text("hello");

        assert_eq!(buffer.value_with_cursor('|'), "hello|");
        buffer.clear();
        buffer.insert("hé");
        buffer.move_cursor_left();
        buffer.insert("!");

        assert_eq!(buffer.as_str(), "h!é");
        assert_eq!(buffer.value_with_cursor('|'), "h!|é");

        buffer.delete_before_cursor();

        assert_eq!(buffer.as_str(), "hé");
        assert_eq!(buffer.value_with_cursor('|'), "h|é");

        assert_eq!(
            buffer.handle_key(UiKey::Home, false),
            UiTextBufferAction::Changed
        );
        assert_eq!(buffer.value_with_cursor('|'), "|hé");
        assert_eq!(
            buffer.handle_key(UiKey::Delete, false),
            UiTextBufferAction::Changed
        );
        assert_eq!(buffer.as_str(), "é");
        assert_eq!(
            buffer.handle_key(UiKey::End, false),
            UiTextBufferAction::Changed
        );
        assert_eq!(buffer.value_with_cursor('|'), "é|");
        assert_eq!(UiTextBuffer::new().display_value("Ask...", '|'), "Ask...");
        assert_eq!(buffer.display_value("Ask...", '|'), "é|");
    }

    #[test]
    fn text_buffer_reports_keyboard_actions() {
        let mut buffer = UiTextBuffer::new();

        assert_eq!(buffer.handle_text_input("a"), UiTextBufferAction::Changed);
        assert_eq!(
            buffer.handle_key(UiKey::Enter, true),
            UiTextBufferAction::Changed
        );
        assert_eq!(buffer.as_str(), "a\n");
        assert_eq!(
            buffer.handle_key(UiKey::Enter, false),
            UiTextBufferAction::Submit
        );
    }

    #[test]
    fn text_buffer_supports_terminal_editing_shortcuts() {
        let ctrl = UiKeyModifiers {
            ctrl: true,
            ..UiKeyModifiers::default()
        };
        let mut buffer = UiTextBuffer::new();
        buffer.insert("alpha beta gamma");

        assert_eq!(
            buffer.handle_key_with_modifiers(UiKey::Other(b'a' as u32), ctrl),
            UiTextBufferAction::Changed
        );
        assert_eq!(buffer.value_with_cursor('|'), "|alpha beta gamma");
        assert_eq!(
            buffer.handle_key_with_modifiers(UiKey::Other(b'e' as u32), ctrl),
            UiTextBufferAction::Changed
        );
        assert_eq!(buffer.value_with_cursor('|'), "alpha beta gamma|");

        buffer.move_cursor_left();
        buffer.move_cursor_left();
        assert_eq!(
            buffer.handle_key_with_modifiers(UiKey::Other(b'u' as u32), ctrl),
            UiTextBufferAction::Changed
        );
        assert_eq!(buffer.value_with_cursor('|'), "|ma");

        buffer.insert("delta ");
        buffer.move_cursor_left();
        assert_eq!(
            buffer.handle_key_with_modifiers(UiKey::Other(b'k' as u32), ctrl),
            UiTextBufferAction::Changed
        );
        assert_eq!(buffer.value_with_cursor('|'), "delta|");

        buffer.insert(" beta gamma");
        assert_eq!(
            buffer.handle_key_with_modifiers(UiKey::Other(b'w' as u32), ctrl),
            UiTextBufferAction::Changed
        );
        assert_eq!(buffer.value_with_cursor('|'), "delta beta |");
        assert_eq!(
            buffer.handle_key_with_modifiers(UiKey::Other(b'w' as u32), ctrl),
            UiTextBufferAction::Changed
        );
        assert_eq!(buffer.value_with_cursor('|'), "delta |");
    }
}
