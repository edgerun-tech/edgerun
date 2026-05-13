//! Shared GPU UI scene primitives and a small native OpenGL renderer.
//!
//! The scene types are platform neutral and are intended to be consumed by both
//! native OpenGL/EGL/SDL hosts and browser WebGL hosts. The native GL renderer
//! below is only one backend for the scene.

use std::string::String;
use std::vec::Vec;

#[cfg(not(feature = "fontdue-text"))]
use core::marker::PhantomData;

mod app_registry;
mod bitmap_font;
pub mod components;
#[cfg(all(feature = "gpu-gl", not(target_arch = "wasm32")))]
pub mod gl;
mod icons;
pub mod palette;
mod runtime;
mod scene;
mod shell;
pub mod style;
#[cfg(feature = "fontdue-text")]
mod text;
pub mod webgl2;
mod workspace;
pub use app_registry::{
    CAPABILITY_REQUEST_APP_ID, CHAT_APP_ID, COMPONENT_GALLERY_APP_ID, EDGERUN_APP_REGISTRY,
    LAUNCH_CAPABILITY_REQUEST_ITEM_ID, LAUNCH_CHAT_ITEM_ID, LAUNCH_COMPONENT_GALLERY_ITEM_ID,
    LAUNCH_LOCK_SCREEN_ITEM_ID, LAUNCH_STORAGE_ITEM_ID, LAUNCH_TRUST_MANAGER_ITEM_ID,
    LOCK_SCREEN_APP_ID, SHELL_LAUNCHER_BUTTON_ID, STORAGE_APP_ID, TRUST_MANAGER_APP_ID,
    UiAppPlacement, UiAppSpec, app_spec, app_spec_for_launch_id,
};
pub use components::{
    BarChart, ControlAccessory, ControlRow, Field, MenuItem, MetricCard, PanelHeader, Slider,
    TextArea, TransactionRow, UiGrid, UiStack, bar_chart, control_row, field, menu_item,
    metric_card, panel_header, slider, text_area, transaction_row,
};
pub use icons::{UiIcon, UiIconAtlasRect, UiIconSet};
#[cfg(feature = "tabler-svg-atlas")]
pub use icons::{UiIconAtlas, tabler_svg_icon_atlas};
use icons::{draw_canonical_icon, icon_circle, icon_line};
pub use runtime::{GpuHit, HitKind, UiAction, UiEvent, UiKey, UiRuntimeState};
pub use scene::{Color4, GpuClip, GpuRect, GpuScene, RectMode, UiColorScheme};
use shell::render_edgerun_shell_overlay;
pub use shell::{UiShellAction, UiShellState};
pub use style::{AlignItems, Axis, JustifyContent, UiStyle};
#[cfg(feature = "fontdue-text")]
pub use text::{FontAtlas, TextQuad};
#[cfg(test)]
use workspace::WORKSPACE_CHROME_H;
pub use workspace::{
    UiAppKind, UiAppSurface, UiTileAxis, UiTileNode, UiWorkspace, UiWorkspaceAction,
};

#[derive(Clone, Copy, Debug)]
pub struct ChatShellMetrics {
    pub sidebar_w: f32,
    pub topbar_h: f32,
    pub composer_h: f32,
    pub pad: f32,
}

impl Default for ChatShellMetrics {
    fn default() -> Self {
        Self {
            sidebar_w: 284.0,
            topbar_h: 58.0,
            composer_h: 86.0,
            pad: 18.0,
        }
    }
}

pub struct UiPainter<'a, 'font> {
    scene: &'a mut GpuScene,
    #[cfg(feature = "fontdue-text")]
    atlas: Option<&'font FontAtlas>,
    #[cfg(not(feature = "fontdue-text"))]
    _font: PhantomData<&'font ()>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UiRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn inset(self, dx: f32, dy: f32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            w: (self.w - dx * 2.0).max(0.0),
            h: (self.h - dy * 2.0).max(0.0),
        }
    }

    pub fn right(self, w: f32) -> Self {
        Self {
            x: self.x + self.w - w,
            y: self.y,
            w,
            h: self.h,
        }
    }

    pub fn bottom(self, h: f32) -> Self {
        Self {
            x: self.x,
            y: self.y + self.h - h,
            w: self.w,
            h,
        }
    }

    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.w && y <= self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Clone, Debug)]
pub enum UiControlAccessory {
    None,
    Value(String),
    Badge {
        label: String,
        color: Color4,
    },
    Toggle {
        on: bool,
        id: u32,
    },
    Button {
        label: String,
        id: u32,
        style: ButtonStyle,
    },
}

#[derive(Clone, Debug)]
pub enum UiNodeKind {
    Row,
    Column,
    Grid {
        columns: u16,
    },
    Card,
    ScrollArea {
        offset: f32,
        id: Option<u32>,
    },
    Text(String),
    Badge {
        label: String,
        color: Color4,
    },
    Button {
        label: String,
        id: u32,
        style: ButtonStyle,
    },
    IconButton {
        icon: UiIcon,
        id: u32,
        active: bool,
    },
    Icon {
        icon: UiIcon,
        color: Color4,
    },
    Checkbox {
        label: String,
        checked: bool,
        id: u32,
    },
    Radio {
        label: String,
        selected: bool,
        id: u32,
    },
    Select {
        label: String,
        value: String,
        id: u32,
    },
    Tooltip {
        text: String,
    },
    Dialog {
        title: String,
        body: String,
        icon: UiIcon,
    },
    Toast {
        message: String,
        icon: UiIcon,
        accent: Color4,
    },
    EmptyState {
        title: String,
        body: String,
        icon: UiIcon,
    },
    Skeleton,
    ProgressRing {
        value: f32,
        color: Color4,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        id_base: u32,
    },
    Breadcrumb {
        items: Vec<String>,
        selected: usize,
        base_id: u32,
    },
    CommandPalette {
        placeholder: String,
        id: u32,
    },
    TreeItem {
        label: String,
        detail: String,
        depth: u8,
        expanded: bool,
        id: u32,
    },
    Section {
        title: String,
        detail: String,
    },
    IdentityCard {
        name: String,
        node: String,
        policy: String,
        id: u32,
    },
    ContactCard {
        name: String,
        detail: String,
        id: u32,
    },
    ThreadRow {
        title: String,
        last_message: String,
        unread: bool,
        id: u32,
    },
    AttachmentPreview {
        name: String,
        kind: String,
        id: u32,
    },
    CapabilityGrantRow {
        app: String,
        capability: String,
        state: String,
        id: u32,
    },
    ProofEventRow {
        title: String,
        hash: String,
        status: String,
        id: u32,
    },
    RoutePath {
        label: String,
        hops: Vec<String>,
    },
    PackageCard {
        name: String,
        policy: String,
        hash: String,
        id: u32,
    },
    ReceiptRow {
        label: String,
        amount: String,
        status: String,
        id: u32,
    },
    AppLauncherItem {
        title: String,
        detail: String,
        icon: UiIcon,
        id: u32,
    },
    Toggle {
        on: bool,
        id: u32,
    },
    Avatar {
        label: String,
        color: Color4,
        online: bool,
    },
    ProgressBar {
        value: f32,
        color: Color4,
    },
    Tabs {
        labels: Vec<String>,
        selected: usize,
        base_id: u32,
    },
    PanelHeader {
        title: String,
        subtitle: String,
        action: Option<(String, u32)>,
    },
    MetricCard {
        title: String,
        value: String,
        detail: String,
        progress: Option<f32>,
        accent: Color4,
    },
    Field {
        label: String,
        value: String,
        helper: String,
        focused: bool,
        id: Option<u32>,
    },
    TextArea {
        label: String,
        value: String,
        focused: bool,
        id: Option<u32>,
    },
    Slider {
        label: String,
        value: f32,
        min_label: String,
        max_label: String,
        accent: Color4,
        id: u32,
    },
    BarChart {
        title: String,
        subtitle: String,
        labels: Vec<String>,
        values: Vec<f32>,
        accent: Color4,
    },
    TransactionRow {
        title: String,
        subtitle: String,
        date: String,
        amount: String,
        positive: bool,
        id: u32,
    },
    MenuItem {
        label: String,
        detail: String,
        badge: String,
        selected: bool,
        accent: Color4,
        id: u32,
    },
    ListRow {
        title: String,
        detail: String,
        accent: Color4,
        id: u32,
    },
    ControlRow {
        label: String,
        detail: String,
        accessory: UiControlAccessory,
        id: Option<u32>,
    },
    Divider,
    Spacer,
}

#[derive(Clone, Debug)]
pub struct UiNode {
    kind: UiNodeKind,
    style: UiStyle,
    children: Vec<UiNode>,
}

impl UiNode {
    pub fn row(classes: &str) -> Self {
        let mut style = UiStyle::parse(classes);
        style.direction = Axis::Horizontal;
        Self {
            kind: UiNodeKind::Row,
            style,
            children: Vec::new(),
        }
    }

    pub fn column(classes: &str) -> Self {
        let mut style = UiStyle::parse(classes);
        style.direction = Axis::Vertical;
        Self {
            kind: UiNodeKind::Column,
            style,
            children: Vec::new(),
        }
    }

    pub fn grid(classes: &str, columns: u16) -> Self {
        Self {
            kind: UiNodeKind::Grid {
                columns: columns.max(1),
            },
            style: UiStyle::parse(classes),
            children: Vec::new(),
        }
    }

    pub fn grid_auto(classes: &str) -> Self {
        let style = UiStyle::parse(classes);
        Self {
            kind: UiNodeKind::Grid {
                columns: style.grid_cols.unwrap_or(1),
            },
            style,
            children: Vec::new(),
        }
    }

    pub fn grid_auto_for_width(classes: &str, width: f32) -> Self {
        let style = UiStyle::parse_for_width(classes, width);
        Self {
            kind: UiNodeKind::Grid {
                columns: style.grid_cols.unwrap_or(1),
            },
            style,
            children: Vec::new(),
        }
    }

    pub fn card(classes: &str) -> Self {
        Self {
            kind: UiNodeKind::Card,
            style: UiStyle::parse(classes),
            children: Vec::new(),
        }
    }

    pub fn scroll_area(classes: &str, offset: f32) -> Self {
        let mut style = UiStyle::parse(classes);
        style.direction = Axis::Vertical;
        Self {
            kind: UiNodeKind::ScrollArea { offset, id: None },
            style,
            children: Vec::new(),
        }
    }

    pub fn text(value: &str) -> Self {
        Self {
            kind: UiNodeKind::Text(value.to_string()),
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn badge(label: &str, color: Color4) -> Self {
        Self {
            kind: UiNodeKind::Badge {
                label: label.to_string(),
                color,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn button(label: &str, id: u32, style: ButtonStyle) -> Self {
        Self {
            kind: UiNodeKind::Button {
                label: label.to_string(),
                id,
                style,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn icon_button(icon: UiIcon, id: u32) -> Self {
        Self {
            kind: UiNodeKind::IconButton {
                icon,
                id,
                active: true,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn icon(icon: UiIcon) -> Self {
        Self {
            kind: UiNodeKind::Icon {
                icon,
                color: palette::MUTED,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn checkbox(label: &str, checked: bool, id: u32) -> Self {
        Self {
            kind: UiNodeKind::Checkbox {
                label: label.to_string(),
                checked,
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn radio(label: &str, selected: bool, id: u32) -> Self {
        Self {
            kind: UiNodeKind::Radio {
                label: label.to_string(),
                selected,
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn select(label: &str, value: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::Select {
                label: label.to_string(),
                value: value.to_string(),
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn tooltip(text: &str) -> Self {
        Self {
            kind: UiNodeKind::Tooltip {
                text: text.to_string(),
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn dialog(title: &str, body: &str, icon: UiIcon) -> Self {
        Self {
            kind: UiNodeKind::Dialog {
                title: title.to_string(),
                body: body.to_string(),
                icon,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn toast(message: &str, icon: UiIcon, accent: Color4) -> Self {
        Self {
            kind: UiNodeKind::Toast {
                message: message.to_string(),
                icon,
                accent,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn empty_state(title: &str, body: &str, icon: UiIcon) -> Self {
        Self {
            kind: UiNodeKind::EmptyState {
                title: title.to_string(),
                body: body.to_string(),
                icon,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn skeleton() -> Self {
        Self {
            kind: UiNodeKind::Skeleton,
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn progress_ring(value: f32, color: Color4) -> Self {
        Self {
            kind: UiNodeKind::ProgressRing { value, color },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn table(
        headers: impl IntoIterator<Item = String>,
        rows: impl IntoIterator<Item = Vec<String>>,
        id_base: u32,
    ) -> Self {
        Self {
            kind: UiNodeKind::Table {
                headers: headers.into_iter().collect(),
                rows: rows.into_iter().collect(),
                id_base,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn table_labels(headers: &[&str], rows: &[&[&str]], id_base: u32) -> Self {
        Self::table(
            headers.iter().copied().map(str::to_string),
            rows.iter()
                .map(|row| row.iter().copied().map(str::to_string).collect()),
            id_base,
        )
    }

    pub fn breadcrumb(labels: &[&str], selected: usize, base_id: u32) -> Self {
        Self {
            kind: UiNodeKind::Breadcrumb {
                items: labels.iter().copied().map(str::to_string).collect(),
                selected,
                base_id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn command_palette(placeholder: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::CommandPalette {
                placeholder: placeholder.to_string(),
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn tree_item(label: &str, detail: &str, depth: u8, expanded: bool, id: u32) -> Self {
        Self {
            kind: UiNodeKind::TreeItem {
                label: label.to_string(),
                detail: detail.to_string(),
                depth,
                expanded,
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn section(title: &str, detail: &str) -> Self {
        Self {
            kind: UiNodeKind::Section {
                title: title.to_string(),
                detail: detail.to_string(),
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn identity_card(name: &str, node: &str, policy: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::IdentityCard {
                name: name.to_string(),
                node: node.to_string(),
                policy: policy.to_string(),
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn contact_card(name: &str, detail: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::ContactCard {
                name: name.to_string(),
                detail: detail.to_string(),
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn thread_row(title: &str, last_message: &str, unread: bool, id: u32) -> Self {
        Self {
            kind: UiNodeKind::ThreadRow {
                title: title.to_string(),
                last_message: last_message.to_string(),
                unread,
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn attachment_preview(name: &str, kind: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::AttachmentPreview {
                name: name.to_string(),
                kind: kind.to_string(),
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn capability_grant_row(app: &str, capability: &str, state: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::CapabilityGrantRow {
                app: app.to_string(),
                capability: capability.to_string(),
                state: state.to_string(),
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn proof_event_row(title: &str, hash: &str, status: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::ProofEventRow {
                title: title.to_string(),
                hash: hash.to_string(),
                status: status.to_string(),
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn route_path(label: &str, hops: &[&str]) -> Self {
        Self {
            kind: UiNodeKind::RoutePath {
                label: label.to_string(),
                hops: hops.iter().copied().map(str::to_string).collect(),
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn package_card(name: &str, policy: &str, hash: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::PackageCard {
                name: name.to_string(),
                policy: policy.to_string(),
                hash: hash.to_string(),
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn receipt_row(label: &str, amount: &str, status: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::ReceiptRow {
                label: label.to_string(),
                amount: amount.to_string(),
                status: status.to_string(),
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn app_launcher_item(title: &str, detail: &str, icon: UiIcon, id: u32) -> Self {
        Self {
            kind: UiNodeKind::AppLauncherItem {
                title: title.to_string(),
                detail: detail.to_string(),
                icon,
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn toggle(on: bool, id: u32) -> Self {
        Self {
            kind: UiNodeKind::Toggle { on, id },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn avatar(label: &str, color: Color4) -> Self {
        Self {
            kind: UiNodeKind::Avatar {
                label: label.to_string(),
                color,
                online: false,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn progress_bar(value: f32, color: Color4) -> Self {
        Self {
            kind: UiNodeKind::ProgressBar { value, color },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn tabs(labels: impl IntoIterator<Item = String>, selected: usize, base_id: u32) -> Self {
        Self {
            kind: UiNodeKind::Tabs {
                labels: labels.into_iter().collect(),
                selected,
                base_id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn tab_labels(labels: &[&str], selected: usize, base_id: u32) -> Self {
        Self::tabs(
            labels.iter().copied().map(str::to_string),
            selected,
            base_id,
        )
    }

    pub fn panel_header(title: &str) -> Self {
        Self {
            kind: UiNodeKind::PanelHeader {
                title: title.to_string(),
                subtitle: String::new(),
                action: None,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn metric_card(title: &str, value: &str) -> Self {
        Self {
            kind: UiNodeKind::MetricCard {
                title: title.to_string(),
                value: value.to_string(),
                detail: String::new(),
                progress: None,
                accent: palette::ACCENT,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn field(label: &str, value: &str) -> Self {
        Self {
            kind: UiNodeKind::Field {
                label: label.to_string(),
                value: value.to_string(),
                helper: String::new(),
                focused: false,
                id: None,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn text_area(label: &str, value: &str) -> Self {
        Self {
            kind: UiNodeKind::TextArea {
                label: label.to_string(),
                value: value.to_string(),
                focused: false,
                id: None,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn slider(label: &str, value: f32, id: u32) -> Self {
        Self {
            kind: UiNodeKind::Slider {
                label: label.to_string(),
                value,
                min_label: String::new(),
                max_label: String::new(),
                accent: palette::ACCENT,
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn bar_chart(
        title: &str,
        labels: impl IntoIterator<Item = String>,
        values: impl IntoIterator<Item = f32>,
    ) -> Self {
        Self {
            kind: UiNodeKind::BarChart {
                title: title.to_string(),
                subtitle: String::new(),
                labels: labels.into_iter().collect(),
                values: values.into_iter().collect(),
                accent: palette::ACCENT,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn bar_chart_labels(title: &str, labels: &[&str], values: &[f32]) -> Self {
        Self::bar_chart(
            title,
            labels.iter().copied().map(str::to_string),
            values.iter().copied(),
        )
    }

    pub fn transaction_row(title: &str, amount: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::TransactionRow {
                title: title.to_string(),
                subtitle: String::new(),
                date: String::new(),
                amount: amount.to_string(),
                positive: false,
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn menu_item(label: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::MenuItem {
                label: label.to_string(),
                detail: String::new(),
                badge: String::new(),
                selected: false,
                accent: palette::ACCENT,
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn list_row(title: &str, detail: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::ListRow {
                title: title.to_string(),
                detail: detail.to_string(),
                accent: palette::ACCENT,
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn control_row(label: &str) -> Self {
        Self {
            kind: UiNodeKind::ControlRow {
                label: label.to_string(),
                detail: String::new(),
                accessory: UiControlAccessory::None,
                id: None,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn divider(classes: &str) -> Self {
        Self {
            kind: UiNodeKind::Divider,
            style: UiStyle::parse(classes),
            children: Vec::new(),
        }
    }

    pub fn spacer(classes: &str) -> Self {
        Self {
            kind: UiNodeKind::Spacer,
            style: UiStyle::parse(classes),
            children: Vec::new(),
        }
    }

    pub fn class(mut self, classes: &str) -> Self {
        let mut parsed = UiStyle::parse(classes);
        self.apply_parsed_style(&mut parsed);
        self.style = parsed;
        self
    }

    pub fn class_for_width(mut self, classes: &str, width: f32) -> Self {
        let mut parsed = UiStyle::parse_for_width(classes, width);
        self.apply_parsed_style(&mut parsed);
        self.style = parsed;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.style.disabled = disabled;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.style.loading = loading;
        self
    }

    fn apply_parsed_style(&mut self, parsed: &mut UiStyle) {
        if matches!(self.kind, UiNodeKind::Row) {
            parsed.direction = Axis::Horizontal;
        }
        if let UiNodeKind::Grid { columns } = &mut self.kind {
            if let Some(grid_cols) = parsed.grid_cols {
                *columns = grid_cols;
            }
        }
        if self.style.col_span > 1 && parsed.col_span == 1 {
            parsed.col_span = self.style.col_span;
        }
    }

    pub fn span(mut self, span: u16) -> Self {
        self.style.col_span = span.max(1);
        self
    }

    pub fn scroll_offset(mut self, offset: f32) -> Self {
        if let UiNodeKind::ScrollArea {
            offset: node_offset,
            ..
        } = &mut self.kind
        {
            *node_offset = offset;
        }
        self
    }

    pub fn scroll_id(mut self, id: u32) -> Self {
        if let UiNodeKind::ScrollArea { id: node_id, .. } = &mut self.kind {
            *node_id = Some(id);
        }
        self
    }

    pub fn child(mut self, child: UiNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = UiNode>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn when(self, condition: bool, child: UiNode) -> Self {
        if condition { self.child(child) } else { self }
    }

    pub fn detail(mut self, value: &str) -> Self {
        match &mut self.kind {
            UiNodeKind::MetricCard { detail, .. } | UiNodeKind::MenuItem { detail, .. } => {
                *detail = value.to_string()
            }
            UiNodeKind::ListRow { detail, .. } | UiNodeKind::ControlRow { detail, .. } => {
                *detail = value.to_string()
            }
            UiNodeKind::Field { helper, .. } => *helper = value.to_string(),
            UiNodeKind::TextArea { value: text, .. } => *text = value.to_string(),
            UiNodeKind::TransactionRow { subtitle, .. }
            | UiNodeKind::BarChart { subtitle, .. }
            | UiNodeKind::PanelHeader { subtitle, .. } => *subtitle = value.to_string(),
            _ => {}
        }
        self
    }

    pub fn action(mut self, label: &str, id: u32) -> Self {
        if let UiNodeKind::PanelHeader { action, .. } = &mut self.kind {
            *action = Some((label.to_string(), id));
        }
        self
    }

    pub fn hit_id(mut self, id: u32) -> Self {
        match &mut self.kind {
            UiNodeKind::Field { id: field_id, .. } | UiNodeKind::TextArea { id: field_id, .. } => {
                *field_id = Some(id)
            }
            UiNodeKind::ControlRow { id: row_id, .. } => *row_id = Some(id),
            _ => {}
        }
        self
    }

    pub fn badge_text(mut self, value: &str) -> Self {
        match &mut self.kind {
            UiNodeKind::MenuItem { badge, .. } => *badge = value.to_string(),
            UiNodeKind::ControlRow { accessory, .. } => {
                *accessory = UiControlAccessory::Badge {
                    label: value.to_string(),
                    color: palette::ACCENT,
                }
            }
            _ => {}
        }
        self
    }

    pub fn value_text(mut self, value: &str) -> Self {
        if let UiNodeKind::ControlRow { accessory, .. } = &mut self.kind {
            *accessory = UiControlAccessory::Value(value.to_string());
        }
        self
    }

    pub fn progress(mut self, value: f32) -> Self {
        match &mut self.kind {
            UiNodeKind::MetricCard { progress, .. } => *progress = Some(value),
            UiNodeKind::ProgressBar {
                value: progress_value,
                ..
            } => *progress_value = value,
            _ => {}
        }
        self
    }

    pub fn accent(mut self, color: Color4) -> Self {
        match &mut self.kind {
            UiNodeKind::MetricCard { accent, .. }
            | UiNodeKind::Slider { accent, .. }
            | UiNodeKind::BarChart { accent, .. }
            | UiNodeKind::MenuItem { accent, .. } => *accent = color,
            UiNodeKind::ListRow {
                accent: row_accent, ..
            } => *row_accent = color,
            UiNodeKind::ControlRow { accessory, .. } => {
                if let UiControlAccessory::Badge {
                    color: badge_color, ..
                } = accessory
                {
                    *badge_color = color;
                }
            }
            UiNodeKind::Avatar {
                color: avatar_color,
                ..
            }
            | UiNodeKind::ProgressBar {
                color: avatar_color,
                ..
            } => *avatar_color = color,
            UiNodeKind::Badge {
                color: badge_color, ..
            }
            | UiNodeKind::Icon {
                color: badge_color, ..
            } => *badge_color = color,
            _ => {}
        }
        self
    }

    pub fn range_labels(mut self, min_label: &str, max_label: &str) -> Self {
        if let UiNodeKind::Slider {
            min_label: min_node_label,
            max_label: max_node_label,
            ..
        } = &mut self.kind
        {
            *min_node_label = min_label.to_string();
            *max_node_label = max_label.to_string();
        }
        self
    }

    pub fn date(mut self, date: &str) -> Self {
        if let UiNodeKind::TransactionRow {
            date: node_date, ..
        } = &mut self.kind
        {
            *node_date = date.to_string();
        }
        self
    }

    pub fn positive(mut self, positive: bool) -> Self {
        if let UiNodeKind::TransactionRow {
            positive: node_positive,
            ..
        } = &mut self.kind
        {
            *node_positive = positive;
        }
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        match &mut self.kind {
            UiNodeKind::Field {
                focused: node_focused,
                ..
            }
            | UiNodeKind::TextArea {
                focused: node_focused,
                ..
            } => *node_focused = focused,
            _ => {}
        }
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        if let UiNodeKind::IconButton {
            active: node_active,
            ..
        } = &mut self.kind
        {
            *node_active = active;
        }
        self
    }

    pub fn online(mut self, online: bool) -> Self {
        if let UiNodeKind::Avatar {
            online: node_online,
            ..
        } = &mut self.kind
        {
            *node_online = online;
        }
        self
    }

    pub fn on(mut self, on: bool) -> Self {
        if let UiNodeKind::Toggle { on: node_on, .. } = &mut self.kind {
            *node_on = on;
        } else if let UiNodeKind::ControlRow { accessory, .. } = &mut self.kind {
            if let UiControlAccessory::Toggle {
                on: accessory_on, ..
            } = accessory
            {
                *accessory_on = on;
            }
        }
        self
    }

    pub fn control_toggle(mut self, on: bool, id: u32) -> Self {
        if let UiNodeKind::ControlRow { accessory, .. } = &mut self.kind {
            *accessory = UiControlAccessory::Toggle { on, id };
        }
        self
    }

    pub fn control_button(mut self, label: &str, id: u32, style: ButtonStyle) -> Self {
        if let UiNodeKind::ControlRow { accessory, .. } = &mut self.kind {
            *accessory = UiControlAccessory::Button {
                label: label.to_string(),
                id,
                style,
            };
        }
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        if let UiNodeKind::MenuItem {
            selected: node_selected,
            ..
        } = &mut self.kind
        {
            *node_selected = selected;
        }
        self
    }

    pub fn render(&self, ui: &mut UiPainter<'_, '_>, bounds: UiRect) {
        self.render_with_state(ui, bounds, None);
    }

    pub fn render_with_state(
        &self,
        ui: &mut UiPainter<'_, '_>,
        bounds: UiRect,
        state: Option<&UiRuntimeState>,
    ) {
        let rect = self.style.layout_rect(bounds);
        let hit_start = ui.scene.hit_count();
        match &self.kind {
            UiNodeKind::Text(value) => {
                ui.bounded_label(rect.x, rect.y, rect.w, value, 2.0, self.style.text);
            }
            UiNodeKind::Badge { label, color } => {
                ui.badge(rect.x, rect.y, label, *color);
            }
            UiNodeKind::Button { label, id, style } => {
                let h = self.style.height.unwrap_or(34.0).min(rect.h);
                ui.button(
                    UiRect::new(rect.x, rect.y, rect.w, h),
                    label,
                    *style,
                    *id,
                    true,
                );
            }
            UiNodeKind::IconButton { icon, id, active } => {
                let size = self
                    .style
                    .width
                    .or(self.style.height)
                    .unwrap_or(34.0)
                    .min(rect.w)
                    .min(rect.h);
                ui.icon_button(UiRect::new(rect.x, rect.y, size, size), *icon, *id, *active);
            }
            UiNodeKind::Icon { icon, color } => {
                let size = self
                    .style
                    .width
                    .or(self.style.height)
                    .unwrap_or(24.0)
                    .min(rect.w)
                    .min(rect.h);
                ui.icon(UiRect::new(rect.x, rect.y, size, size), *icon, *color);
            }
            UiNodeKind::Checkbox { label, checked, id } => {
                let checked = state
                    .map(|state| state.toggle_value(*id, *checked))
                    .unwrap_or(*checked);
                ui.checkbox(rect, label, checked, *id);
            }
            UiNodeKind::Radio {
                label,
                selected,
                id,
            } => {
                let selected = state
                    .map(|state| state.toggle_value(*id, *selected))
                    .unwrap_or(*selected);
                ui.radio(rect, label, selected, *id);
            }
            UiNodeKind::Select { label, value, id } => {
                ui.select_trigger(rect, label, value, *id);
            }
            UiNodeKind::Tooltip { text } => {
                ui.tooltip(rect, text);
            }
            UiNodeKind::Dialog { title, body, icon } => {
                ui.dialog(rect, title, body, *icon);
                render_children(
                    ui,
                    rect.inset(18.0, 88.0),
                    &self.style,
                    &self.children,
                    state,
                );
            }
            UiNodeKind::Toast {
                message,
                icon,
                accent,
            } => {
                ui.toast(rect, message, *icon, *accent);
            }
            UiNodeKind::EmptyState { title, body, icon } => {
                ui.empty_state(rect, title, body, *icon);
            }
            UiNodeKind::Skeleton => {
                ui.skeleton(rect);
            }
            UiNodeKind::ProgressRing { value, color } => {
                ui.progress_ring(rect, *value, *color);
            }
            UiNodeKind::Table {
                headers,
                rows,
                id_base,
            } => {
                ui.table(rect, headers, rows, *id_base);
            }
            UiNodeKind::Breadcrumb {
                items,
                selected,
                base_id,
            } => {
                ui.breadcrumb(rect, items, *selected, *base_id);
            }
            UiNodeKind::CommandPalette { placeholder, id } => {
                ui.command_palette(rect, placeholder, *id);
            }
            UiNodeKind::TreeItem {
                label,
                detail,
                depth,
                expanded,
                id,
            } => {
                ui.tree_item(rect, label, detail, *depth, *expanded, *id);
            }
            UiNodeKind::Section { title, detail } => {
                ui.section_header(rect, title, detail);
            }
            UiNodeKind::IdentityCard {
                name,
                node,
                policy,
                id,
            } => ui.identity_card(rect, name, node, policy, *id),
            UiNodeKind::ContactCard { name, detail, id } => {
                ui.contact_card(rect, name, detail, *id)
            }
            UiNodeKind::ThreadRow {
                title,
                last_message,
                unread,
                id,
            } => ui.thread_row(rect, title, last_message, *unread, *id),
            UiNodeKind::AttachmentPreview { name, kind, id } => {
                ui.attachment_preview(rect, name, kind, *id)
            }
            UiNodeKind::CapabilityGrantRow {
                app,
                capability,
                state,
                id,
            } => ui.capability_grant_row(rect, app, capability, state, *id),
            UiNodeKind::ProofEventRow {
                title,
                hash,
                status,
                id,
            } => ui.proof_event_row(rect, title, hash, status, *id),
            UiNodeKind::RoutePath { label, hops } => ui.route_path(rect, label, hops),
            UiNodeKind::PackageCard {
                name,
                policy,
                hash,
                id,
            } => ui.package_card(rect, name, policy, hash, *id),
            UiNodeKind::ReceiptRow {
                label,
                amount,
                status,
                id,
            } => ui.receipt_row(rect, label, amount, status, *id),
            UiNodeKind::AppLauncherItem {
                title,
                detail,
                icon,
                id,
            } => ui.app_launcher_item(rect, title, detail, *icon, *id),
            UiNodeKind::Toggle { on, id } => {
                let y = rect.y + ((rect.h - 24.0).max(0.0) * 0.5);
                let on = state
                    .map(|state| state.toggle_value(*id, *on))
                    .unwrap_or(*on);
                ui.toggle(rect.x, y, on, *id);
            }
            UiNodeKind::Avatar {
                label,
                color,
                online,
            } => {
                let size = self
                    .style
                    .width
                    .or(self.style.height)
                    .unwrap_or(36.0)
                    .min(rect.w)
                    .min(rect.h);
                ui.avatar(rect.x, rect.y, size, label, *color, *online);
            }
            UiNodeKind::ProgressBar { value, color } => {
                let h = self.style.height.unwrap_or(8.0).min(rect.h);
                let y = rect.y + ((rect.h - h).max(0.0) * 0.5);
                ui.progress_bar(UiRect::new(rect.x, y, rect.w, h), *value, *color);
            }
            UiNodeKind::Tabs {
                labels,
                selected,
                base_id,
            } => {
                let label_refs = labels.iter().map(String::as_str).collect::<Vec<_>>();
                let h = self.style.height.unwrap_or(34.0).min(rect.h);
                ui.segmented_tabs(
                    UiRect::new(rect.x, rect.y, rect.w, h),
                    &label_refs,
                    *selected,
                    *base_id,
                );
            }
            UiNodeKind::PanelHeader {
                title,
                subtitle,
                action,
            } => {
                self::components::panel_header(
                    ui,
                    rect,
                    self::components::PanelHeader {
                        title,
                        subtitle,
                        action: action.as_ref().map(|(label, id)| (label.as_str(), *id)),
                    },
                );
            }
            UiNodeKind::MetricCard {
                title,
                value,
                detail,
                progress,
                accent,
            } => {
                self::components::metric_card(
                    ui,
                    rect,
                    self::components::MetricCard {
                        title,
                        value,
                        detail,
                        progress: *progress,
                        accent: *accent,
                    },
                );
            }
            UiNodeKind::Field {
                label,
                value,
                helper,
                focused,
                id,
            } => {
                let value = id
                    .and_then(|id| state.map(|state| state.text_value(id, value)))
                    .unwrap_or(value);
                let focused = *focused
                    || state.is_some_and(|state| {
                        state
                            .focused()
                            .is_some_and(|hit| hit.kind == HitKind::Input && Some(hit.id) == *id)
                    });
                self::components::field(
                    ui,
                    rect,
                    self::components::Field {
                        label,
                        value,
                        helper,
                        focused,
                        id: *id,
                    },
                );
            }
            UiNodeKind::TextArea {
                label,
                value,
                focused,
                id,
            } => {
                let value = id
                    .and_then(|id| state.map(|state| state.text_value(id, value)))
                    .unwrap_or(value);
                let focused = *focused
                    || state.is_some_and(|state| {
                        state
                            .focused()
                            .is_some_and(|hit| hit.kind == HitKind::TextArea && Some(hit.id) == *id)
                    });
                self::components::text_area(
                    ui,
                    rect,
                    self::components::TextArea {
                        label,
                        value,
                        focused,
                        id: *id,
                    },
                );
            }
            UiNodeKind::Slider {
                label,
                value,
                min_label,
                max_label,
                accent,
                id,
            } => {
                let value = state
                    .map(|state| state.slider_value(*id, *value))
                    .unwrap_or(*value);
                self::components::slider(
                    ui,
                    rect,
                    self::components::Slider {
                        label,
                        value,
                        min_label,
                        max_label,
                        accent: *accent,
                        id: *id,
                    },
                );
            }
            UiNodeKind::BarChart {
                title,
                subtitle,
                labels,
                values,
                accent,
            } => {
                let label_refs = labels.iter().map(String::as_str).collect::<Vec<_>>();
                self::components::bar_chart(
                    ui,
                    rect,
                    self::components::BarChart {
                        title,
                        subtitle,
                        labels: &label_refs,
                        values,
                        accent: *accent,
                    },
                );
            }
            UiNodeKind::TransactionRow {
                title,
                subtitle,
                date,
                amount,
                positive,
                id,
            } => {
                self::components::transaction_row(
                    ui,
                    rect,
                    self::components::TransactionRow {
                        title,
                        subtitle,
                        date,
                        amount,
                        positive: *positive,
                        id: *id,
                    },
                );
            }
            UiNodeKind::MenuItem {
                label,
                detail,
                badge,
                selected,
                accent,
                id,
            } => {
                self::components::menu_item(
                    ui,
                    rect,
                    self::components::MenuItem {
                        label,
                        detail,
                        badge,
                        selected: *selected,
                        accent: *accent,
                        id: *id,
                    },
                );
            }
            UiNodeKind::ListRow {
                title,
                detail,
                accent,
                id,
            } => {
                ui.list_row(rect, title, detail, *accent, *id);
            }
            UiNodeKind::ControlRow {
                label,
                detail,
                accessory,
                id,
            } => {
                let accessory = match accessory {
                    UiControlAccessory::None => self::components::ControlAccessory::None,
                    UiControlAccessory::Value(value) => {
                        self::components::ControlAccessory::Value(value)
                    }
                    UiControlAccessory::Badge { label, color } => {
                        self::components::ControlAccessory::Badge(label, *color)
                    }
                    UiControlAccessory::Toggle { on, id } => {
                        let on = state
                            .map(|state| state.toggle_value(*id, *on))
                            .unwrap_or(*on);
                        self::components::ControlAccessory::Toggle { on, id: *id }
                    }
                    UiControlAccessory::Button { label, id, style } => {
                        self::components::ControlAccessory::Button {
                            label,
                            id: *id,
                            style: *style,
                        }
                    }
                };
                self::components::control_row(
                    ui,
                    rect,
                    self::components::ControlRow {
                        label,
                        detail,
                        accessory,
                        id: *id,
                    },
                );
            }
            UiNodeKind::Divider => {
                let axis = if rect.w >= rect.h {
                    Axis::Horizontal
                } else {
                    Axis::Vertical
                };
                ui.divider(rect.x, rect.y, rect.w.max(rect.h), axis);
            }
            UiNodeKind::Spacer => {}
            UiNodeKind::Row
            | UiNodeKind::Column
            | UiNodeKind::Grid { .. }
            | UiNodeKind::Card
            | UiNodeKind::ScrollArea { .. } => {
                if let Some(bg) = self.style.bg {
                    if matches!(self.kind, UiNodeKind::Card) {
                        ui.card(rect.x, rect.y, rect.w, rect.h, self.style.radius, bg);
                    } else {
                        ui.scene.push_rect(GpuRect::fill(
                            rect.x,
                            rect.y,
                            rect.w,
                            rect.h,
                            self.style.radius,
                            bg,
                        ));
                        if self.style.border {
                            ui.scene.push_rect(GpuRect::border(
                                rect.x,
                                rect.y,
                                rect.w,
                                rect.h,
                                self.style.radius,
                                palette::BORDER,
                            ));
                        }
                    }
                }
                if let UiNodeKind::Grid { columns } = &self.kind {
                    render_grid_children(ui, rect, &self.style, *columns, &self.children, state);
                } else if let UiNodeKind::ScrollArea { offset, id } = &self.kind {
                    let offset = id
                        .and_then(|id| state.map(|state| state.scroll_offset(id)))
                        .unwrap_or(*offset);
                    render_scroll_children(
                        ui,
                        rect,
                        &self.style,
                        offset,
                        *id,
                        &self.children,
                        state,
                    );
                } else {
                    render_children(ui, rect, &self.style, &self.children, state);
                }
            }
        }
        let added_hits = &ui.scene.hits()[hit_start..];
        let focused_rect = state.and_then(|state| {
            let focused = state.focused()?;
            added_hits
                .iter()
                .copied()
                .find(|hit| hit.kind == focused.kind && hit.id == focused.id)
        });
        if let Some(focused) = focused_rect {
            ui.border_rect(
                UiRect::new(focused.x, focused.y, focused.w, focused.h),
                10.0,
                palette::ACCENT.with_alpha(0.86),
            );
        }
        if self.style.disabled || self.style.loading {
            ui.scene.truncate_hits(hit_start);
            ui.fill_rect(
                rect,
                self.style.radius.max(8.0),
                palette::BG.with_alpha(0.34),
            );
        }
        if self.style.loading {
            let size = rect.w.min(rect.h).min(24.0);
            ui.progress_ring(
                UiRect::new(
                    rect.x + (rect.w - size) * 0.5,
                    rect.y + (rect.h - size) * 0.5,
                    size,
                    size,
                ),
                0.72,
                palette::ACCENT,
            );
        }
    }
}

pub fn row(classes: &str) -> UiNode {
    UiNode::row(classes)
}

pub fn column(classes: &str) -> UiNode {
    UiNode::column(classes)
}

pub fn grid(classes: &str, columns: u16) -> UiNode {
    UiNode::grid(classes, columns)
}

pub fn grid_auto(classes: &str) -> UiNode {
    UiNode::grid_auto(classes)
}

pub fn grid_auto_for_width(classes: &str, width: f32) -> UiNode {
    UiNode::grid_auto_for_width(classes, width)
}

pub fn card(classes: &str) -> UiNode {
    UiNode::card(classes)
}

pub fn scroll_area(classes: &str, offset: f32) -> UiNode {
    UiNode::scroll_area(classes, offset)
}

pub fn text(value: &str) -> UiNode {
    UiNode::text(value)
}

pub fn badge(label: &str, color: Color4) -> UiNode {
    UiNode::badge(label, color)
}

pub fn button(label: &str, id: u32, style: ButtonStyle) -> UiNode {
    UiNode::button(label, id, style)
}

pub fn icon(icon: UiIcon) -> UiNode {
    UiNode::icon(icon)
}

pub fn icon_button(icon: UiIcon, id: u32) -> UiNode {
    UiNode::icon_button(icon, id)
}

pub fn checkbox(label: &str, checked: bool, id: u32) -> UiNode {
    UiNode::checkbox(label, checked, id)
}

pub fn radio(label: &str, selected: bool, id: u32) -> UiNode {
    UiNode::radio(label, selected, id)
}

pub fn select_node(label: &str, value: &str, id: u32) -> UiNode {
    UiNode::select(label, value, id)
}

pub fn tooltip(text: &str) -> UiNode {
    UiNode::tooltip(text)
}

pub fn dialog(title: &str, body: &str, icon: UiIcon) -> UiNode {
    UiNode::dialog(title, body, icon)
}

pub fn toast(message: &str, icon: UiIcon, accent: Color4) -> UiNode {
    UiNode::toast(message, icon, accent)
}

pub fn empty_state(title: &str, body: &str, icon: UiIcon) -> UiNode {
    UiNode::empty_state(title, body, icon)
}

pub fn skeleton() -> UiNode {
    UiNode::skeleton()
}

pub fn progress_ring(value: f32, color: Color4) -> UiNode {
    UiNode::progress_ring(value, color)
}

pub fn table_labels(headers: &[&str], rows: &[&[&str]], id_base: u32) -> UiNode {
    UiNode::table_labels(headers, rows, id_base)
}

pub fn breadcrumb(labels: &[&str], selected: usize, base_id: u32) -> UiNode {
    UiNode::breadcrumb(labels, selected, base_id)
}

pub fn command_palette(placeholder: &str, id: u32) -> UiNode {
    UiNode::command_palette(placeholder, id)
}

pub fn tree_item(label: &str, detail: &str, depth: u8, expanded: bool, id: u32) -> UiNode {
    UiNode::tree_item(label, detail, depth, expanded, id)
}

pub fn section(title: &str, detail: &str) -> UiNode {
    UiNode::section(title, detail)
}

pub fn identity_card(name: &str, node: &str, policy: &str, id: u32) -> UiNode {
    UiNode::identity_card(name, node, policy, id)
}

pub fn contact_card(name: &str, detail: &str, id: u32) -> UiNode {
    UiNode::contact_card(name, detail, id)
}

pub fn thread_row(title: &str, last_message: &str, unread: bool, id: u32) -> UiNode {
    UiNode::thread_row(title, last_message, unread, id)
}

pub fn attachment_preview(name: &str, kind: &str, id: u32) -> UiNode {
    UiNode::attachment_preview(name, kind, id)
}

pub fn capability_grant_row(app: &str, capability: &str, state: &str, id: u32) -> UiNode {
    UiNode::capability_grant_row(app, capability, state, id)
}

pub fn proof_event_row(title: &str, hash: &str, status: &str, id: u32) -> UiNode {
    UiNode::proof_event_row(title, hash, status, id)
}

pub fn route_path(label: &str, hops: &[&str]) -> UiNode {
    UiNode::route_path(label, hops)
}

pub fn package_card(name: &str, policy: &str, hash: &str, id: u32) -> UiNode {
    UiNode::package_card(name, policy, hash, id)
}

pub fn receipt_row(label: &str, amount: &str, status: &str, id: u32) -> UiNode {
    UiNode::receipt_row(label, amount, status, id)
}

pub fn app_launcher_item(title: &str, detail: &str, icon: UiIcon, id: u32) -> UiNode {
    UiNode::app_launcher_item(title, detail, icon, id)
}

pub fn toggle_node(on: bool, id: u32) -> UiNode {
    UiNode::toggle(on, id)
}

pub fn avatar_node(label: &str, color: Color4) -> UiNode {
    UiNode::avatar(label, color)
}

pub fn progress_bar_node(value: f32, color: Color4) -> UiNode {
    UiNode::progress_bar(value, color)
}

pub fn tabs_node(
    labels: impl IntoIterator<Item = String>,
    selected: usize,
    base_id: u32,
) -> UiNode {
    UiNode::tabs(labels, selected, base_id)
}

pub fn tab_labels(labels: &[&str], selected: usize, base_id: u32) -> UiNode {
    UiNode::tab_labels(labels, selected, base_id)
}

pub fn header(title: &str) -> UiNode {
    UiNode::panel_header(title)
}

pub fn metric(title: &str, value: &str) -> UiNode {
    UiNode::metric_card(title, value)
}

pub fn field_node(label: &str, value: &str) -> UiNode {
    UiNode::field(label, value)
}

pub fn text_area_node(label: &str, value: &str) -> UiNode {
    UiNode::text_area(label, value)
}

pub fn slider_node(label: &str, value: f32, id: u32) -> UiNode {
    UiNode::slider(label, value, id)
}

pub fn bar_chart_node(
    title: &str,
    labels: impl IntoIterator<Item = String>,
    values: impl IntoIterator<Item = f32>,
) -> UiNode {
    UiNode::bar_chart(title, labels, values)
}

pub fn bar_chart_labels(title: &str, labels: &[&str], values: &[f32]) -> UiNode {
    UiNode::bar_chart_labels(title, labels, values)
}

pub fn transaction_node(title: &str, amount: &str, id: u32) -> UiNode {
    UiNode::transaction_row(title, amount, id)
}

pub fn menu_item_node(label: &str, id: u32) -> UiNode {
    UiNode::menu_item(label, id)
}

pub fn list_row_node(title: &str, detail: &str, id: u32) -> UiNode {
    UiNode::list_row(title, detail, id)
}

pub fn control_row_node(label: &str) -> UiNode {
    UiNode::control_row(label)
}

pub fn divider(classes: &str) -> UiNode {
    UiNode::divider(classes)
}

pub fn spacer(classes: &str) -> UiNode {
    UiNode::spacer(classes)
}

#[macro_export]
macro_rules! gpu_ui {
    ($node:expr) => {
        $node
    };
    ($node:expr, [ $($child:expr),* $(,)? ]) => {{
        let mut node = $node;
        $(
            node = node.child($child);
        )*
        node
    }};
}

impl<'a, 'font> UiPainter<'a, 'font> {
    pub fn new(scene: &'a mut GpuScene) -> Self {
        Self {
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas: None,
            #[cfg(not(feature = "fontdue-text"))]
            _font: PhantomData,
        }
    }

    #[cfg(feature = "fontdue-text")]
    pub fn with_font(scene: &'a mut GpuScene, atlas: &'font FontAtlas) -> Self {
        Self {
            scene,
            atlas: Some(atlas),
        }
    }

    pub fn label(&mut self, x: f32, y: f32, text: &str, scale: f32, color: Color4) {
        push_label(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            text,
            scale,
            color,
        );
    }

    pub fn bounded_label(
        &mut self,
        x: f32,
        y: f32,
        max_w: f32,
        text: &str,
        scale: f32,
        color: Color4,
    ) {
        push_bounded_label(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            max_w,
            text,
            scale,
            color,
        );
    }

    pub fn panel(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
        panel(self.scene, x, y, w, h, radius, color);
    }

    pub fn card(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
        soft_card(self.scene, x, y, w, h, radius, color);
    }

    pub fn fill_rect(&mut self, rect: UiRect, radius: f32, color: Color4) {
        self.scene
            .push_rect(GpuRect::fill(rect.x, rect.y, rect.w, rect.h, radius, color));
    }

    pub fn border_rect(&mut self, rect: UiRect, radius: f32, color: Color4) {
        self.scene.push_rect(GpuRect::border(
            rect.x, rect.y, rect.w, rect.h, radius, color,
        ));
    }

    pub fn pill(&mut self, x: f32, y: f32, w: f32, label: &str, color: Color4) {
        draw_pill(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            w,
            label,
            color,
        );
    }

    pub fn hit(&mut self, kind: HitKind, id: u32, x: f32, y: f32, w: f32, h: f32) {
        self.scene.push_hit(GpuHit::new(kind, id, x, y, w, h));
    }

    pub fn divider(&mut self, x: f32, y: f32, w: f32, axis: Axis) {
        match axis {
            Axis::Horizontal => {
                self.scene
                    .push_rect(GpuRect::fill(x, y, w, 1.0, 0.0, palette::BORDER));
            }
            Axis::Vertical => {
                self.scene
                    .push_rect(GpuRect::fill(x, y, 1.0, w, 0.0, palette::BORDER));
            }
        }
    }

    pub fn badge(&mut self, x: f32, y: f32, label: &str, color: Color4) -> f32 {
        let w = component_label_width(
            label,
            2.0,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
        ) + 18.0;
        self.scene
            .push_rect(GpuRect::fill(x, y, w, 22.0, 11.0, color.with_alpha(0.16)));
        self.scene
            .push_rect(GpuRect::border(x, y, w, 22.0, 11.0, color.with_alpha(0.42)));
        self.bounded_label(x + 9.0, y + 5.0, w - 18.0, label, 2.0, color);
        w
    }

    pub fn avatar(&mut self, x: f32, y: f32, size: f32, label: &str, color: Color4, online: bool) {
        let radius = size * 0.5;
        self.scene.push_rect(GpuRect::fill(
            x,
            y,
            size,
            size,
            radius,
            color.with_alpha(0.22),
        ));
        self.scene.push_rect(GpuRect::border(
            x,
            y,
            size,
            size,
            radius,
            color.with_alpha(0.54),
        ));
        self.bounded_label(
            x + size * 0.34,
            y + size * 0.30,
            size * 0.42,
            contact_initial(label),
            2.0,
            color,
        );
        self.status_dot(x + size - 8.0, y + size - 8.0, online);
    }

    pub fn status_dot(&mut self, x: f32, y: f32, online: bool) {
        self.scene.push_rect(GpuRect::fill(
            x,
            y,
            8.0,
            8.0,
            4.0,
            if online {
                palette::GREEN
            } else {
                palette::MUTED
            },
        ));
    }

    pub fn button(&mut self, rect: UiRect, label: &str, style: ButtonStyle, id: u32, active: bool) {
        self.hit(HitKind::Button, id, rect.x, rect.y, rect.w, rect.h);
        let (fill, border, text) = match style {
            ButtonStyle::Primary => (palette::ACCENT, palette::ACCENT, palette::ACCENT_TEXT),
            ButtonStyle::Secondary => (palette::ROW, palette::BORDER, palette::TEXT),
            ButtonStyle::Ghost => (
                palette::PANEL.with_alpha(0.0),
                palette::BORDER,
                palette::MUTED,
            ),
            ButtonStyle::Danger => (
                palette::DANGER.with_alpha(0.18),
                palette::DANGER,
                palette::DANGER,
            ),
        };
        let fill = if active {
            fill
        } else {
            fill.with_alpha(fill.a * 0.74)
        };
        self.scene
            .push_rect(GpuRect::fill(rect.x, rect.y, rect.w, rect.h, 10.0, fill));
        self.scene.push_rect(GpuRect::border(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            10.0,
            border.with_alpha(if active { 0.72 } else { 0.42 }),
        ));
        let label_w = component_label_width(
            label,
            2.0,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
        );
        self.bounded_label(
            rect.x + ((rect.w - label_w) * 0.5).max(10.0),
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 20.0).max(0.0),
            label,
            2.0,
            text,
        );
    }

    pub fn icon_button(&mut self, rect: UiRect, icon: UiIcon, id: u32, active: bool) {
        self.hit(HitKind::Button, id, rect.x, rect.y, rect.w, rect.h);
        let fill = if active {
            palette::ROW
        } else {
            palette::ROW.with_alpha(0.58)
        };
        self.fill_rect(rect, 10.0, fill);
        self.border_rect(
            rect,
            10.0,
            palette::BORDER.with_alpha(if active { 0.72 } else { 0.42 }),
        );
        let size = rect.w.min(rect.h).min(22.0);
        self.icon(
            UiRect::new(
                rect.x + (rect.w - size) * 0.5,
                rect.y + (rect.h - size) * 0.5,
                size,
                size,
            ),
            icon,
            if active {
                palette::TEXT
            } else {
                palette::MUTED
            },
        );
    }

    pub fn icon(&mut self, rect: UiRect, icon: UiIcon, color: Color4) {
        draw_canonical_icon(self.scene, rect, icon, color);
    }

    pub fn checkbox(&mut self, rect: UiRect, label: &str, checked: bool, id: u32) {
        self.hit(HitKind::Checkbox, id, rect.x, rect.y, rect.w, rect.h);
        let box_rect = UiRect::new(rect.x, rect.y + (rect.h - 22.0) * 0.5, 22.0, 22.0);
        self.fill_rect(box_rect, 6.0, palette::ROW);
        self.border_rect(
            box_rect,
            6.0,
            if checked {
                palette::ACCENT
            } else {
                palette::BORDER
            },
        );
        if checked {
            self.icon(box_rect.inset(4.0, 4.0), UiIcon::Check, palette::ACCENT);
        }
        self.bounded_label(
            rect.x + 32.0,
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 32.0).max(0.0),
            label,
            2.0,
            palette::TEXT,
        );
    }

    pub fn radio(&mut self, rect: UiRect, label: &str, selected: bool, id: u32) {
        self.hit(HitKind::Radio, id, rect.x, rect.y, rect.w, rect.h);
        let dot_rect = UiRect::new(rect.x, rect.y + (rect.h - 22.0) * 0.5, 22.0, 22.0);
        self.fill_rect(dot_rect, 11.0, palette::ROW);
        self.border_rect(
            dot_rect,
            11.0,
            if selected {
                palette::ACCENT
            } else {
                palette::BORDER
            },
        );
        if selected {
            self.fill_rect(dot_rect.inset(6.0, 6.0), 5.0, palette::ACCENT);
        }
        self.bounded_label(
            rect.x + 32.0,
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 32.0).max(0.0),
            label,
            2.0,
            palette::TEXT,
        );
    }

    pub fn select_trigger(&mut self, rect: UiRect, label: &str, value: &str, id: u32) {
        self.hit(HitKind::Select, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 10.0, palette::COMPOSER);
        self.border_rect(rect, 10.0, palette::BORDER);
        self.bounded_label(
            rect.x + 14.0,
            rect.y + 8.0,
            (rect.w - 48.0).max(0.0),
            label,
            2.0,
            palette::MUTED,
        );
        self.bounded_label(
            rect.x + 14.0,
            rect.y + 29.0,
            (rect.w - 48.0).max(0.0),
            value,
            2.0,
            palette::TEXT,
        );
        self.icon(
            UiRect::new(
                rect.x + rect.w - 31.0,
                rect.y + (rect.h - 18.0) * 0.5,
                18.0,
                18.0,
            ),
            UiIcon::ChevronRight,
            palette::MUTED,
        );
    }

    pub fn tooltip(&mut self, rect: UiRect, text: &str) {
        self.fill_rect(rect, 8.0, palette::TOPBAR);
        self.border_rect(rect, 8.0, palette::BORDER.with_alpha(0.72));
        self.bounded_label(
            rect.x + 10.0,
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 20.0).max(0.0),
            text,
            2.0,
            palette::TEXT,
        );
    }

    pub fn dialog(&mut self, rect: UiRect, title: &str, body: &str, icon: UiIcon) {
        self.card(rect.x, rect.y, rect.w, rect.h, 12.0, palette::PANEL);
        self.icon(
            UiRect::new(rect.x + 18.0, rect.y + 18.0, 34.0, 34.0),
            icon,
            palette::ACCENT,
        );
        self.bounded_label(
            rect.x + 64.0,
            rect.y + 18.0,
            rect.w - 84.0,
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 64.0,
            rect.y + 42.0,
            rect.w - 84.0,
            body,
            2.0,
            palette::MUTED,
        );
        self.divider(
            rect.x + 18.0,
            rect.y + 74.0,
            rect.w - 36.0,
            Axis::Horizontal,
        );
    }

    pub fn toast(&mut self, rect: UiRect, message: &str, icon: UiIcon, accent: Color4) {
        self.fill_rect(rect, 10.0, palette::TOPBAR);
        self.border_rect(rect, 10.0, accent.with_alpha(0.54));
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + (rect.h - 22.0) * 0.5, 22.0, 22.0),
            icon,
            accent,
        );
        self.bounded_label(
            rect.x + 44.0,
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 56.0).max(0.0),
            message,
            2.0,
            palette::TEXT,
        );
    }

    pub fn empty_state(&mut self, rect: UiRect, title: &str, body: &str, icon: UiIcon) {
        self.fill_rect(rect, 10.0, palette::PANEL);
        self.border_rect(rect, 10.0, palette::BORDER.with_alpha(0.64));
        let icon_size = rect.h.min(rect.w).min(54.0);
        let icon_rect = UiRect::new(
            rect.x + (rect.w - icon_size) * 0.5,
            rect.y + 22.0,
            icon_size,
            icon_size,
        );
        self.icon(icon_rect, icon, palette::ACCENT);
        self.bounded_label(
            rect.x + 20.0,
            icon_rect.y + icon_rect.h + 16.0,
            rect.w - 40.0,
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 20.0,
            icon_rect.y + icon_rect.h + 40.0,
            rect.w - 40.0,
            body,
            2.0,
            palette::MUTED,
        );
    }

    pub fn skeleton(&mut self, rect: UiRect) {
        self.fill_rect(rect, 8.0, palette::ROW.with_alpha(0.74));
        let shine_w = (rect.w * 0.28).max(18.0).min(rect.w);
        self.fill_rect(
            UiRect::new(rect.x + rect.w * 0.18, rect.y, shine_w, rect.h),
            8.0,
            palette::BORDER.with_alpha(0.34),
        );
    }

    pub fn progress_ring(&mut self, rect: UiRect, value: f32, color: Color4) {
        let size = rect.w.min(rect.h);
        let center = (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5);
        let radius = size * 0.38;
        icon_circle(self.scene, center, radius, 2.0, palette::BORDER);
        let steps = (32.0 * value.clamp(0.0, 1.0)).ceil().max(1.0) as u32;
        let mut prev = None;
        for i in 0..=steps {
            let angle = -core::f32::consts::FRAC_PI_2 + (i as f32 / 32.0) * core::f32::consts::TAU;
            let pt = (
                center.0 + angle.cos() * radius,
                center.1 + angle.sin() * radius,
            );
            if let Some(prev) = prev {
                icon_line(self.scene, prev, pt, 3.0, color);
            }
            prev = Some(pt);
        }
    }

    pub fn table(&mut self, rect: UiRect, headers: &[String], rows: &[Vec<String>], id_base: u32) {
        self.fill_rect(rect, 8.0, palette::PANEL);
        self.border_rect(rect, 8.0, palette::BORDER);
        let cols = headers.len().max(1);
        let col_w = (rect.w - 24.0).max(0.0) / cols as f32;
        let mut y = rect.y + 12.0;
        for (index, header) in headers.iter().enumerate() {
            self.bounded_label(
                rect.x + 12.0 + index as f32 * col_w,
                y,
                col_w - 10.0,
                header,
                2.0,
                palette::MUTED,
            );
        }
        y += 28.0;
        self.divider(rect.x + 12.0, y - 8.0, rect.w - 24.0, Axis::Horizontal);
        for (row_index, row) in rows.iter().enumerate() {
            let row_rect = UiRect::new(rect.x + 6.0, y - 7.0, rect.w - 12.0, 34.0);
            self.hit(
                HitKind::ListRow,
                id_base + row_index as u32,
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
            );
            if row_index % 2 == 1 {
                self.fill_rect(row_rect, 6.0, palette::ROW.with_alpha(0.48));
            }
            for col in 0..cols {
                let value = row.get(col).map(String::as_str).unwrap_or("");
                self.bounded_label(
                    rect.x + 12.0 + col as f32 * col_w,
                    y,
                    col_w - 10.0,
                    value,
                    2.0,
                    palette::TEXT,
                );
            }
            y += 34.0;
            if y > rect.y + rect.h - 20.0 {
                break;
            }
        }
    }

    pub fn breadcrumb(&mut self, rect: UiRect, items: &[String], selected: usize, base_id: u32) {
        let mut x = rect.x;
        for (index, item) in items.iter().enumerate() {
            let w = (component_label_width(
                item,
                2.0,
                #[cfg(feature = "fontdue-text")]
                self.atlas,
            ) + 24.0)
                .clamp(46.0, 150.0);
            let item_rect = UiRect::new(x, rect.y, w, rect.h.min(32.0));
            self.hit(
                HitKind::Breadcrumb,
                base_id + index as u32,
                item_rect.x,
                item_rect.y,
                item_rect.w,
                item_rect.h,
            );
            self.fill_rect(
                item_rect,
                8.0,
                if index == selected {
                    palette::ACTIVE_ROW
                } else {
                    palette::ROW.with_alpha(0.38)
                },
            );
            self.bounded_label(
                item_rect.x + 10.0,
                item_rect.y + 8.0,
                item_rect.w - 20.0,
                item,
                2.0,
                if index == selected {
                    palette::TEXT
                } else {
                    palette::MUTED
                },
            );
            x += w + 6.0;
            if index + 1 < items.len() {
                self.icon(
                    UiRect::new(x, rect.y + 7.0, 16.0, 16.0),
                    UiIcon::ChevronRight,
                    palette::MUTED,
                );
                x += 22.0;
            }
        }
    }

    pub fn command_palette(&mut self, rect: UiRect, placeholder: &str, id: u32) {
        self.hit(HitKind::Input, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 12.0, palette::COMPOSER);
        self.border_rect(rect, 12.0, palette::BORDER);
        self.icon(
            UiRect::new(rect.x + 14.0, rect.y + (rect.h - 20.0) * 0.5, 20.0, 20.0),
            UiIcon::Search,
            palette::MUTED,
        );
        self.bounded_label(
            rect.x + 44.0,
            rect.y + (rect.h - 14.0) * 0.5,
            rect.w - 58.0,
            placeholder,
            2.0,
            palette::MUTED,
        );
    }

    pub fn tree_item(
        &mut self,
        rect: UiRect,
        label: &str,
        detail: &str,
        depth: u8,
        expanded: bool,
        id: u32,
    ) {
        self.hit(HitKind::TreeItem, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 6.0, palette::PANEL);
        let indent = 12.0 + depth as f32 * 18.0;
        self.icon(
            UiRect::new(rect.x + indent, rect.y + (rect.h - 16.0) * 0.5, 16.0, 16.0),
            if expanded {
                UiIcon::ChevronRight
            } else {
                UiIcon::File
            },
            palette::MUTED,
        );
        self.bounded_label(
            rect.x + indent + 24.0,
            rect.y + 9.0,
            rect.w * 0.48,
            label,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + rect.w * 0.58,
            rect.y + 9.0,
            rect.w * 0.36,
            detail,
            2.0,
            palette::MUTED,
        );
    }

    pub fn section_header(&mut self, rect: UiRect, title: &str, detail: &str) {
        self.bounded_label(rect.x, rect.y, rect.w * 0.55, title, 2.0, palette::TEXT);
        self.bounded_label(
            rect.x + rect.w * 0.58,
            rect.y,
            rect.w * 0.42,
            detail,
            2.0,
            palette::MUTED,
        );
        self.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
    }

    pub fn identity_card(&mut self, rect: UiRect, name: &str, node: &str, policy: &str, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.card(rect.x, rect.y, rect.w, rect.h, 10.0, palette::PANEL);
        self.icon(
            UiRect::new(rect.x + 16.0, rect.y + 18.0, 34.0, 34.0),
            UiIcon::Trust,
            palette::ACCENT,
        );
        self.bounded_label(
            rect.x + 62.0,
            rect.y + 16.0,
            rect.w - 82.0,
            name,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 62.0,
            rect.y + 39.0,
            rect.w - 82.0,
            node,
            2.0,
            palette::MUTED,
        );
        self.badge(
            rect.x + 16.0,
            rect.y + rect.h - 34.0,
            policy,
            palette::ACCENT,
        );
    }

    pub fn contact_card(&mut self, rect: UiRect, name: &str, detail: &str, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 8.0, palette::PANEL);
        self.border_rect(rect, 8.0, palette::BORDER);
        self.avatar(
            rect.x + 12.0,
            rect.y + 12.0,
            36.0,
            name,
            palette::ACCENT,
            true,
        );
        self.bounded_label(
            rect.x + 58.0,
            rect.y + 13.0,
            rect.w - 72.0,
            name,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 58.0,
            rect.y + 35.0,
            rect.w - 72.0,
            detail,
            2.0,
            palette::MUTED,
        );
    }

    pub fn thread_row(
        &mut self,
        rect: UiRect,
        title: &str,
        last_message: &str,
        unread: bool,
        id: u32,
    ) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(
            rect,
            6.0,
            if unread {
                palette::ACTIVE_ROW
            } else {
                palette::PANEL
            },
        );
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 15.0, 24.0, 24.0),
            UiIcon::Chat,
            if unread {
                palette::ACCENT
            } else {
                palette::MUTED
            },
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 10.0,
            rect.w - 62.0,
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 32.0,
            rect.w - 62.0,
            last_message,
            2.0,
            palette::MUTED,
        );
    }

    pub fn attachment_preview(&mut self, rect: UiRect, name: &str, kind: &str, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 8.0, palette::ROW);
        self.border_rect(rect, 8.0, palette::BORDER.with_alpha(0.68));
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 12.0, 28.0, 28.0),
            UiIcon::File,
            palette::ACCENT,
        );
        self.bounded_label(
            rect.x + 52.0,
            rect.y + 11.0,
            rect.w - 66.0,
            name,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 52.0,
            rect.y + 33.0,
            rect.w - 66.0,
            kind,
            2.0,
            palette::MUTED,
        );
    }

    pub fn capability_grant_row(
        &mut self,
        rect: UiRect,
        app: &str,
        capability: &str,
        state: &str,
        id: u32,
    ) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 0.0, palette::PANEL);
        self.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 17.0, 24.0, 24.0),
            UiIcon::Shield,
            palette::VIOLET,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 10.0,
            rect.w * 0.34,
            app,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 32.0,
            rect.w * 0.34,
            capability,
            2.0,
            palette::MUTED,
        );
        self.badge(
            rect.x + rect.w - 96.0,
            rect.y + 18.0,
            state,
            palette::ACCENT,
        );
    }

    pub fn proof_event_row(
        &mut self,
        rect: UiRect,
        title: &str,
        hash: &str,
        status: &str,
        id: u32,
    ) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 0.0, palette::PANEL);
        self.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 17.0, 24.0, 24.0),
            UiIcon::Check,
            palette::GREEN,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 10.0,
            rect.w * 0.36,
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 32.0,
            rect.w * 0.48,
            hash,
            2.0,
            palette::MUTED,
        );
        self.badge(
            rect.x + rect.w - 96.0,
            rect.y + 18.0,
            status,
            palette::GREEN,
        );
    }

    pub fn route_path(&mut self, rect: UiRect, label: &str, hops: &[String]) {
        self.fill_rect(rect, 8.0, palette::PANEL);
        self.border_rect(rect, 8.0, palette::BORDER);
        self.bounded_label(
            rect.x + 14.0,
            rect.y + 10.0,
            rect.w - 28.0,
            label,
            2.0,
            palette::TEXT,
        );
        let mut x = rect.x + 16.0;
        let y = rect.y + 45.0;
        for (index, hop) in hops.iter().enumerate() {
            self.icon(
                UiRect::new(x, y, 22.0, 22.0),
                UiIcon::Route,
                palette::ACCENT,
            );
            self.bounded_label(x + 28.0, y + 4.0, 78.0, hop, 2.0, palette::MUTED);
            x += 112.0;
            if index + 1 < hops.len() {
                self.icon(
                    UiRect::new(x - 22.0, y + 3.0, 16.0, 16.0),
                    UiIcon::ChevronRight,
                    palette::MUTED,
                );
            }
            if x > rect.x + rect.w - 80.0 {
                break;
            }
        }
    }

    pub fn package_card(&mut self, rect: UiRect, name: &str, policy: &str, hash: &str, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.card(rect.x, rect.y, rect.w, rect.h, 10.0, palette::PANEL);
        self.icon(
            UiRect::new(rect.x + 16.0, rect.y + 18.0, 30.0, 30.0),
            UiIcon::App,
            palette::ACCENT,
        );
        self.bounded_label(
            rect.x + 58.0,
            rect.y + 16.0,
            rect.w - 76.0,
            name,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 58.0,
            rect.y + 39.0,
            rect.w - 76.0,
            hash,
            2.0,
            palette::MUTED,
        );
        self.badge(
            rect.x + 16.0,
            rect.y + rect.h - 34.0,
            policy,
            palette::VIOLET,
        );
    }

    pub fn receipt_row(&mut self, rect: UiRect, label: &str, amount: &str, status: &str, id: u32) {
        self.hit(HitKind::TransactionRow, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 0.0, palette::PANEL);
        self.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 17.0, 24.0, 24.0),
            UiIcon::Wallet,
            palette::GREEN,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 18.0,
            rect.w * 0.38,
            label,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + rect.w - 150.0,
            rect.y + 18.0,
            70.0,
            status,
            2.0,
            palette::MUTED,
        );
        self.bounded_label(
            rect.x + rect.w - 76.0,
            rect.y + 18.0,
            64.0,
            amount,
            2.0,
            palette::GREEN,
        );
    }

    pub fn input_field(&mut self, rect: UiRect, placeholder: &str, focused: bool) {
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            12.0,
            palette::COMPOSER,
        ));
        self.scene.push_rect(GpuRect::border(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            12.0,
            if focused {
                palette::ACCENT
            } else {
                palette::BORDER
            },
        ));
        self.bounded_label(
            rect.x + 16.0,
            rect.y + 13.0,
            (rect.w - 32.0).max(0.0),
            placeholder,
            2.0,
            palette::MUTED,
        );
    }

    pub fn app_launcher_item(
        &mut self,
        rect: UiRect,
        title: &str,
        detail: &str,
        icon: UiIcon,
        id: u32,
    ) {
        self.hit(HitKind::AppLauncherItem, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 8.0, palette::ROW.with_alpha(0.72));
        self.border_rect(rect, 8.0, palette::BORDER.with_alpha(0.58));
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 15.0, 26.0, 26.0),
            icon,
            palette::ACCENT,
        );
        self.bounded_label(
            rect.x + 50.0,
            rect.y + 10.0,
            (rect.w - 84.0).max(0.0),
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 50.0,
            rect.y + 32.0,
            (rect.w - 84.0).max(0.0),
            detail,
            2.0,
            palette::MUTED,
        );
        self.icon(
            UiRect::new(rect.x + rect.w - 30.0, rect.y + 20.0, 16.0, 16.0),
            UiIcon::ChevronRight,
            palette::MUTED,
        );
    }

    pub fn toggle(&mut self, x: f32, y: f32, on: bool, id: u32) {
        self.hit(HitKind::Toggle, id, x, y, 46.0, 24.0);
        let color = if on { palette::GREEN } else { palette::MUTED };
        self.scene.push_rect(GpuRect::fill(
            x,
            y,
            46.0,
            24.0,
            12.0,
            color.with_alpha(0.18),
        ));
        self.scene.push_rect(GpuRect::border(
            x,
            y,
            46.0,
            24.0,
            12.0,
            color.with_alpha(0.54),
        ));
        let knob_x = if on { x + 24.0 } else { x + 4.0 };
        self.scene
            .push_rect(GpuRect::fill(knob_x, y + 4.0, 16.0, 16.0, 8.0, color));
    }

    pub fn progress_bar(&mut self, rect: UiRect, fraction: f32, color: Color4) {
        let value = fraction.clamp(0.0, 1.0);
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            rect.h * 0.5,
            palette::ROW,
        ));
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w * value,
            rect.h,
            rect.h * 0.5,
            color,
        ));
    }

    pub fn segmented_tabs(&mut self, rect: UiRect, labels: &[&str], selected: usize, base_id: u32) {
        if labels.is_empty() {
            return;
        }
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            12.0,
            palette::ROW,
        ));
        self.scene.push_rect(GpuRect::border(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            12.0,
            palette::BORDER,
        ));
        let item_w = rect.w / labels.len() as f32;
        for (index, label) in labels.iter().enumerate() {
            let x = rect.x + index as f32 * item_w;
            let active = index == selected;
            self.hit(
                HitKind::Tab,
                base_id + index as u32,
                x,
                rect.y,
                item_w,
                rect.h,
            );
            if active {
                self.scene.push_rect(GpuRect::fill(
                    x + 3.0,
                    rect.y + 3.0,
                    item_w - 6.0,
                    rect.h - 6.0,
                    9.0,
                    palette::ACTIVE_ROW,
                ));
            }
            let label_w = component_label_width(
                label,
                2.0,
                #[cfg(feature = "fontdue-text")]
                self.atlas,
            );
            self.bounded_label(
                x + ((item_w - label_w) * 0.5).max(8.0),
                rect.y + (rect.h - 14.0) * 0.5,
                (item_w - 16.0).max(0.0),
                label,
                2.0,
                if active {
                    palette::TEXT
                } else {
                    palette::MUTED
                },
            );
        }
    }

    pub fn list_row(&mut self, rect: UiRect, title: &str, detail: &str, accent: Color4, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.card(rect.x, rect.y, rect.w, rect.h, 10.0, palette::ROW);
        self.scene
            .push_rect(GpuRect::fill(rect.x, rect.y, 3.0, rect.h, 2.0, accent));
        self.bounded_label(
            rect.x + 16.0,
            rect.y + 10.0,
            (rect.w - 32.0).max(0.0),
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 16.0,
            rect.y + 31.0,
            (rect.w - 32.0).max(0.0),
            detail,
            2.0,
            palette::MUTED,
        );
    }

    pub fn scrollbar(&mut self, rect: UiRect, visible_fraction: f32, offset_fraction: f32) {
        let visible = visible_fraction.clamp(0.08, 1.0);
        let offset = offset_fraction.clamp(0.0, 1.0);
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            rect.w * 0.5,
            palette::ROW.with_alpha(0.42),
        ));
        let thumb_h = rect.h * visible;
        let thumb_y = rect.y + (rect.h - thumb_h) * offset;
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            thumb_y,
            rect.w,
            thumb_h,
            rect.w * 0.5,
            palette::MUTED.with_alpha(0.74),
        ));
    }

    pub fn contact_row(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        contact: &UnifiedContact<'_>,
        selected: bool,
        id: u32,
    ) {
        self.hit(HitKind::Contact, id, x, y, w, 56.0);
        draw_contact_row(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            w,
            contact,
            selected,
        );
    }

    pub fn message_bubble(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        role: &str,
        body: &str,
        fill: Color4,
        accent: Color4,
    ) -> f32 {
        draw_message(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            w,
            role,
            body,
            fill,
            accent,
        )
    }

    pub fn composer(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        placeholder: &str,
        active: bool,
        chips: &[(&str, Color4)],
    ) {
        self.hit(HitKind::Composer, 0, x, y, w, h);
        self.card(x, y, w, h, 16.0, palette::COMPOSER);
        self.scene.push_rect(GpuRect::border(
            x,
            y,
            w,
            h,
            16.0,
            if active {
                palette::ACCENT
            } else {
                palette::BORDER
            },
        ));
        self.bounded_label(
            x + 22.0,
            y + 24.0,
            (w - 96.0).max(0.0),
            placeholder,
            2.0,
            palette::MUTED,
        );

        let mut chip_x = x + 20.0;
        for (label, color) in chips.iter().copied() {
            let chip_w = component_label_width(
                label,
                2.0,
                #[cfg(feature = "fontdue-text")]
                self.atlas,
            ) + 24.0;
            if chip_x + chip_w > x + w - 78.0 {
                break;
            }
            self.pill(chip_x, y + h - 34.0, chip_w, label, color);
            chip_x += chip_w + 10.0;
        }

        self.hit(HitKind::Send, 0, x + w - 58.0, y + h - 56.0, 40.0, 38.0);
        self.scene.push_rect(GpuRect::fill(
            x + w - 58.0,
            y + h - 56.0,
            40.0,
            38.0,
            12.0,
            palette::ACCENT,
        ));
        self.bounded_label(
            x + w - 47.0,
            y + h - 45.0,
            24.0,
            ">",
            3.0,
            palette::ACCENT_TEXT,
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnifiedContactKind {
    Person,
    CodexClient,
    Node,
}

#[derive(Clone, Copy, Debug)]
pub struct UnifiedContact<'a> {
    pub name: &'a str,
    pub detail: &'a str,
    pub kind: UnifiedContactKind,
    pub unread: u16,
    pub online: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct UnifiedMessage<'a> {
    pub author: &'a str,
    pub body: &'a str,
    pub outgoing: bool,
    pub accent: Color4,
}

#[derive(Clone, Debug)]
pub struct UnifiedChatState<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub contacts: &'a [UnifiedContact<'a>],
    pub selected_contact: usize,
    pub messages: &'a [UnifiedMessage<'a>],
    pub composer_placeholder: &'a str,
    pub connected: bool,
}

impl<'a> UnifiedChatState<'a> {
    pub const fn empty() -> Self {
        Self {
            title: "EdgeRun Chat",
            subtitle: "contact book",
            contacts: &[],
            selected_contact: 0,
            messages: &[],
            composer_placeholder: "Select a contact to start a thread...",
            connected: false,
        }
    }
}

pub fn build_unified_chat_shell(scene: &mut GpuScene, width: f32, height: f32) {
    let state = UnifiedChatState::empty();
    build_unified_chat_shell_impl(
        scene,
        width,
        height,
        &state,
        None,
        #[cfg(feature = "fontdue-text")]
        None,
    );
}

#[cfg(feature = "fontdue-text")]
pub fn build_unified_chat_shell_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    state: &UnifiedChatState<'_>,
) {
    build_unified_chat_shell_impl(scene, width, height, state, None, Some(atlas));
}

#[cfg(feature = "fontdue-text")]
pub fn build_unified_chat_shell_with_font_and_runtime(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    state: &UnifiedChatState<'_>,
    runtime: &UiRuntimeState,
) {
    build_unified_chat_shell_impl(scene, width, height, state, Some(runtime), Some(atlas));
}

#[cfg(feature = "fontdue-text")]
pub fn build_edgerun_workspace_shell_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    workspace: &mut UiWorkspace,
    chat_state: &UnifiedChatState<'_>,
) {
    scene.clear = palette::BG;
    scene.clear_rects();
    let mut ui = UiPainter {
        scene,
        atlas: Some(atlas),
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    ui.fill_rect(
        UiRect::new(0.0, 0.0, width.max(360.0), height.max(320.0)),
        0.0,
        palette::BG,
    );
    workspace.render(
        &mut ui,
        UiRect::new(8.0, 8.0, (width - 16.0).max(0.0), (height - 16.0).max(0.0)),
        |ui, bounds, app| match app.kind {
            UiAppKind::Chat => render_workspace_chat_app(ui, bounds, app, chat_state),
            UiAppKind::TrustManager => render_trust_manager_app(ui, bounds, app),
            UiAppKind::Storage => render_storage_app(ui, bounds, app),
            UiAppKind::LockScreen => render_lock_screen_app(ui, bounds, app),
            UiAppKind::CapabilityRequest => render_capability_request_app(ui, bounds, app),
            UiAppKind::ComponentGallery => render_component_gallery_app(ui, bounds, app),
            UiAppKind::Generic => render_generic_workspace_app(ui, bounds, app),
        },
    );
}

#[cfg(feature = "fontdue-text")]
pub fn build_edgerun_workspace_with_shell_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    workspace: &mut UiWorkspace,
    shell: &mut UiShellState,
    chat_state: &UnifiedChatState<'_>,
) {
    build_edgerun_workspace_shell_with_font(scene, atlas, width, height, workspace, chat_state);
    let mut ui = UiPainter {
        scene,
        atlas: Some(atlas),
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    render_edgerun_shell_overlay(&mut ui, UiRect::new(0.0, 0.0, width, height), shell);
}

#[cfg(feature = "fontdue-text")]
pub fn build_edgerun_fullscreen_app_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    app: &mut UiAppSurface,
    chat_state: &UnifiedChatState<'_>,
) {
    scene.clear = palette::BG;
    scene.clear_rects();
    app.full_screen = true;
    app.bounds = None;
    let mut ui = UiPainter {
        scene,
        atlas: Some(atlas),
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    let bounds = UiRect::new(0.0, 0.0, width.max(360.0), height.max(320.0));
    ui.fill_rect(bounds, 0.0, palette::BG);
    let clipped = ui
        .scene
        .push_clip(GpuClip::new(bounds.x, bounds.y, bounds.w, bounds.h));
    if clipped {
        app.bounds = Some(bounds);
        match app.kind {
            UiAppKind::Chat => render_workspace_chat_app(&mut ui, bounds, app, chat_state),
            UiAppKind::TrustManager => render_trust_manager_app(&mut ui, bounds, app),
            UiAppKind::Storage => render_storage_app(&mut ui, bounds, app),
            UiAppKind::LockScreen => render_lock_screen_app(&mut ui, bounds, app),
            UiAppKind::CapabilityRequest => render_capability_request_app(&mut ui, bounds, app),
            UiAppKind::ComponentGallery => render_component_gallery_app(&mut ui, bounds, app),
            UiAppKind::Generic => render_generic_workspace_app(&mut ui, bounds, app),
        }
        ui.scene.pop_clip();
    }
}

#[cfg(feature = "fontdue-text")]
pub fn build_edgerun_shell_overlay_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    shell: &mut UiShellState,
) {
    scene.clear = Color4::rgba(0.0, 0.0, 0.0, 0.0);
    scene.clear_rects();
    let mut ui = UiPainter {
        scene,
        atlas: Some(atlas),
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    render_edgerun_shell_overlay(&mut ui, UiRect::new(0.0, 0.0, width, height), shell);
}

fn build_unified_chat_shell_impl(
    scene: &mut GpuScene,
    width: f32,
    height: f32,
    state: &UnifiedChatState<'_>,
    runtime: Option<&UiRuntimeState>,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) {
    scene.clear = palette::BG;
    scene.clear_rects();
    let mut ui = UiPainter {
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    let m = ChatShellMetrics::default();
    let w = width.max(360.0);
    let h = height.max(320.0);
    let sidebar_w = if w < 760.0 { 0.0 } else { m.sidebar_w + 28.0 };
    let main_x = sidebar_w;
    let main_w = w - sidebar_w;

    if sidebar_w > 0.0 {
        ui.panel(0.0, 0.0, sidebar_w, h, 0.0, palette::SIDEBAR);
        ui.scene.push_rect(GpuRect::fill(
            0.0,
            0.0,
            sidebar_w,
            4.0,
            0.0,
            palette::ACCENT,
        ));
        ui.bounded_label(
            24.0,
            20.0,
            (sidebar_w - 138.0).max(0.0),
            state.title,
            3.0,
            palette::TEXT,
        );
        ui.bounded_label(
            24.0,
            50.0,
            (sidebar_w - 48.0).max(0.0),
            state.subtitle,
            2.0,
            palette::MUTED,
        );
        ui.pill(
            sidebar_w - 104.0,
            22.0,
            78.0,
            if state.connected { "relay" } else { "local" },
            if state.connected {
                palette::GREEN
            } else {
                palette::AMBER
            },
        );
        ui.scene.push_rect(GpuRect::fill(
            sidebar_w - 1.0,
            0.0,
            1.0,
            h,
            0.0,
            palette::BORDER,
        ));

        for (index, contact) in state.contacts.iter().enumerate() {
            let y = 88.0 + index as f32 * 68.0;
            let selected = index == state.selected_contact;
            ui.contact_row(16.0, y, sidebar_w - 32.0, contact, selected, index as u32);
        }
    }

    ui.panel(main_x, 0.0, main_w, m.topbar_h, 0.0, palette::TOPBAR);
    let active = state
        .contacts
        .get(state.selected_contact)
        .or_else(|| state.contacts.first());
    let active_name = active.map(|contact| contact.name).unwrap_or("Unified chat");
    let active_detail = active
        .map(|contact| contact.detail)
        .unwrap_or("contact thread");
    let title_w = if main_w > 620.0 {
        190.0
    } else {
        (main_w - 40.0).max(0.0)
    };
    ui.bounded_label(
        main_x + 20.0,
        14.0,
        title_w,
        active_name,
        2.0,
        palette::TEXT,
    );
    ui.bounded_label(
        main_x + 20.0,
        35.0,
        title_w,
        active_detail,
        2.0,
        palette::MUTED,
    );
    let tabs_w = 226.0_f32.min((main_w - 390.0).max(0.0));
    if tabs_w > 160.0 {
        ui.segmented_tabs(
            UiRect::new(main_x + 220.0, 13.0, tabs_w, 32.0),
            &["chat", "proofs", "files"],
            0,
            20,
        );
    }
    ui.pill(
        main_x + main_w - 322.0,
        15.0,
        132.0,
        "recipient sealed",
        palette::GREEN,
    );
    ui.pill(
        main_x + main_w - 178.0,
        15.0,
        154.0,
        "identity routed",
        palette::ACCENT,
    );
    ui.scene.push_rect(GpuRect::fill(
        main_x,
        m.topbar_h - 1.0,
        main_w,
        1.0,
        0.0,
        palette::BORDER,
    ));

    let transcript_top = m.topbar_h + m.pad;
    let transcript_x = main_x + m.pad;
    let transcript_w = main_w - m.pad * 2.0;
    if state.contacts.is_empty() {
        let empty_w = transcript_w.clamp(260.0, 520.0);
        let empty_h = 126.0;
        let empty_x = transcript_x + (transcript_w - empty_w) * 0.5;
        let empty_y = transcript_top + 46.0;
        ui.card(empty_x, empty_y, empty_w, empty_h, 12.0, palette::PANEL);
        ui.bounded_label(
            empty_x + 22.0,
            empty_y + 24.0,
            empty_w - 44.0,
            "No contacts yet",
            3.0,
            palette::TEXT,
        );
        ui.bounded_label(
            empty_x + 22.0,
            empty_y + 62.0,
            empty_w - 44.0,
            "Connect a contact book or receive an identity-routed contact to show threads here.",
            2.0,
            palette::MUTED,
        );
        let composer = (
            main_x + m.pad,
            h - m.composer_h - m.pad,
            main_w - m.pad * 2.0,
            m.composer_h,
        );
        let composer_text = runtime
            .map(|runtime| runtime.text_value(0, state.composer_placeholder))
            .unwrap_or(state.composer_placeholder);
        let composer_active = runtime.is_some_and(|runtime| {
            runtime
                .focused()
                .is_some_and(|hit| hit.kind == HitKind::Composer)
        });
        ui.composer(
            composer.0,
            composer.1,
            composer.2,
            composer.3,
            composer_text,
            composer_active,
            &[],
        );
        return;
    }

    let rail_w = if transcript_w > 820.0 { 220.0 } else { 0.0 };
    let message_area_w = transcript_w - if rail_w > 0.0 { rail_w + 18.0 } else { 0.0 };
    let message_w = (message_area_w * 0.82).clamp(220.0, 760.0);
    let transcript_limit = h - m.composer_h - m.pad * 2.0;
    let mut message_y = transcript_top;

    for message in state.messages {
        let x = if message.outgoing {
            transcript_x + message_area_w - message_w
        } else {
            transcript_x
        };
        let fill = if message.outgoing {
            palette::USER
        } else {
            palette::ASSISTANT
        };
        let height = estimate_message_height(
            message.body,
            (message_w - 42.0).max(120.0),
            4,
            #[cfg(feature = "fontdue-text")]
            ui.atlas,
        );
        if message_y + height > transcript_limit {
            break;
        }
        let drawn = ui.message_bubble(
            x,
            message_y,
            message_w,
            message.author,
            message.body,
            fill,
            message.accent,
        );
        message_y += drawn + 16.0;
    }

    if rail_w > 0.0 {
        let rail_x = transcript_x + transcript_w - rail_w;
        ui.card(rail_x, transcript_top, rail_w, 220.0, 12.0, palette::PANEL);
        ui.bounded_label(
            rail_x + 16.0,
            transcript_top + 18.0,
            rail_w - 32.0,
            "Thread policy",
            2.0,
            palette::TEXT,
        );
        ui.pill(
            rail_x + 16.0,
            transcript_top + 50.0,
            132.0,
            "2 recipients",
            palette::ACCENT,
        );
        ui.pill(
            rail_x + 16.0,
            transcript_top + 84.0,
            158.0,
            "codex tools scoped",
            palette::VIOLET,
        );
        ui.pill(
            rail_x + 16.0,
            transcript_top + 118.0,
            140.0,
            "relay admitted",
            palette::GREEN,
        );
        ui.divider(
            rail_x + 16.0,
            transcript_top + 152.0,
            rail_w - 32.0,
            Axis::Horizontal,
        );
        ui.bounded_label(
            rail_x + 16.0,
            transcript_top + 166.0,
            rail_w - 88.0,
            "route health",
            2.0,
            palette::MUTED,
        );
        ui.progress_bar(
            UiRect::new(rail_x + 16.0, transcript_top + 190.0, rail_w - 32.0, 8.0),
            if state.connected { 0.86 } else { 0.38 },
            if state.connected {
                palette::GREEN
            } else {
                palette::AMBER
            },
        );
        ui.toggle(
            rail_x + rail_w - 66.0,
            transcript_top + 162.0,
            runtime
                .map(|runtime| runtime.toggle_value(44, state.connected))
                .unwrap_or(state.connected),
            44,
        );
        row("row bg-row border rounded-md p-2 gap-2")
            .child(text("policy").class("w-14 text-muted truncate"))
            .child(text("scoped").class("flex-1 text-green truncate"))
            .child(button("open", 55, ButtonStyle::Ghost).class("w-16 h-8"))
            .render(
                &mut ui,
                UiRect::new(rail_x + 16.0, transcript_top + 206.0, rail_w - 32.0, 42.0),
            );
    }
    if state.messages.len() > 2 {
        ui.scrollbar(
            UiRect::new(
                transcript_x + message_area_w + 6.0,
                transcript_top,
                6.0,
                (transcript_limit - transcript_top).max(80.0),
            ),
            0.72,
            0.0,
        );
    }

    let composer = (
        main_x + m.pad,
        h - m.composer_h - m.pad,
        main_w - m.pad * 2.0,
        m.composer_h,
    );
    let composer_text = runtime
        .map(|runtime| runtime.text_value(0, state.composer_placeholder))
        .unwrap_or(state.composer_placeholder);
    let composer_active = state.connected
        || runtime.is_some_and(|runtime| {
            runtime
                .focused()
                .is_some_and(|hit| hit.kind == HitKind::Composer)
        });
    ui.composer(
        composer.0,
        composer.1,
        composer.2,
        composer.3,
        composer_text,
        composer_active,
        &[
            ("encrypted", palette::GREEN),
            ("contact", palette::ACCENT),
            ("codex tools", palette::VIOLET),
        ],
    );
}

fn render_workspace_chat_app(
    ui: &mut UiPainter<'_, '_>,
    bounds: UiRect,
    app: &UiAppSurface,
    chat_state: &UnifiedChatState<'_>,
) {
    let pad = 14.0;
    ui.fill_rect(bounds, 0.0, palette::BG);
    row("row bg-topbar border rounded-md p-3 gap-3 items-center")
        .child(text("Contacts").class("w-20 text-muted truncate"))
        .child(text(chat_state.subtitle).class("flex-1 text-text truncate"))
        .child(badge(
            if chat_state.connected {
                "relay"
            } else {
                "local"
            },
            if chat_state.connected {
                palette::GREEN
            } else {
                palette::AMBER
            },
        ))
        .render_with_state(
            ui,
            UiRect::new(
                bounds.x + pad,
                bounds.y + pad,
                (bounds.w - pad * 2.0).max(0.0),
                46.0,
            ),
            Some(&app.runtime),
        );

    let composer_h = 76.0;
    let content_top = bounds.y + 72.0;
    let content_bottom = bounds.y + bounds.h - composer_h - pad;
    let content_h = (content_bottom - content_top).max(0.0);
    if chat_state.contacts.is_empty() {
        card("bg-panel border rounded-md p-4 gap-3")
            .child(text("No contacts yet").class("text-text truncate"))
            .child(
                text("Connect a contact book or receive an identity-routed contact.")
                    .class("text-muted truncate"),
            )
            .render_with_state(
                ui,
                UiRect::new(
                    bounds.x + pad,
                    content_top,
                    (bounds.w - pad * 2.0).max(0.0),
                    112.0,
                ),
                Some(&app.runtime),
            );
    } else {
        let rows = chat_state
            .contacts
            .iter()
            .enumerate()
            .map(|(index, contact)| {
                list_row_node(contact.name, contact.detail, index as u32).accent(
                    match contact.kind {
                        UnifiedContactKind::Person => palette::ACCENT,
                        UnifiedContactKind::CodexClient => palette::VIOLET,
                        UnifiedContactKind::Node => palette::GREEN,
                    },
                )
            });
        scroll_area("bg-panel border rounded-md p-2 gap-2", 0.0)
            .scroll_id(101)
            .children(rows)
            .render_with_state(
                ui,
                UiRect::new(
                    bounds.x + pad,
                    content_top,
                    (bounds.w - pad * 2.0).max(0.0),
                    content_h,
                ),
                Some(&app.runtime),
            );
    }

    let draft = app.runtime.text_value(0, chat_state.composer_placeholder);
    ui.composer(
        bounds.x + pad,
        bounds.y + bounds.h - composer_h - pad,
        (bounds.w - pad * 2.0).max(0.0),
        composer_h,
        draft,
        app.runtime
            .focused()
            .is_some_and(|hit| hit.kind == HitKind::Composer),
        &[("encrypted", palette::GREEN), ("identity", palette::ACCENT)],
    );
}

fn render_trust_manager_app(ui: &mut UiPainter<'_, '_>, bounds: UiRect, app: &UiAppSurface) {
    column("bg-panel border rounded-md p-4 gap-3")
        .child(header("Trust Manager").detail("proof dashboard"))
        .child(identity_card(
            "Local identity",
            "browser node",
            "sealed Trust Container",
            221,
        ))
        .child(route_path(
            "Current route",
            &["app", "device", "admission", "relay"],
        ))
        .child(capability_grant_row(
            "EdgeRun Chat",
            "decrypt message",
            "pending",
            220,
        ))
        .child(proof_event_row(
            "Runtime events",
            "proof log empty",
            "0",
            222,
        ))
        .render_with_state(ui, bounds.inset(14.0, 14.0), Some(&app.runtime));
}

fn render_storage_app(ui: &mut UiPainter<'_, '_>, bounds: UiRect, app: &UiAppSurface) {
    column("bg-panel border rounded-md p-4 gap-3")
        .child(header("Storage").detail("verified local cache"))
        .child(package_card(
            "Network apps",
            "run by hash, cache by policy",
            "cache pending",
            301,
        ))
        .child(contact_card(
            "Contact book",
            "IndexedDB projection pending",
            302,
        ))
        .child(attachment_preview(
            "Message payloads",
            "encrypted payload objects",
            303,
        ))
        .child(receipt_row(
            "Cached package bytes",
            "unknown",
            "waiting",
            304,
        ))
        .render_with_state(ui, bounds.inset(14.0, 14.0), Some(&app.runtime));
}

pub const LOCK_UNLOCK_BUTTON_ID: u32 = 900;
pub const LOCK_UNLOCK_FIELD_ID: u32 = 901;
pub const CAPABILITY_ALLOW_BUTTON_ID: u32 = 920;
pub const CAPABILITY_DENY_BUTTON_ID: u32 = 921;
pub const CAPABILITY_DETAILS_BUTTON_ID: u32 = 922;

fn render_lock_screen_app(ui: &mut UiPainter<'_, '_>, bounds: UiRect, app: &UiAppSurface) {
    ui.fill_rect(bounds, 0.0, palette::BG);
    let panel_w = bounds.w.clamp(320.0, 520.0);
    let panel_h = 320.0_f32.min((bounds.h - 32.0).max(220.0));
    let panel = UiRect::new(
        bounds.x + (bounds.w - panel_w) * 0.5,
        bounds.y + (bounds.h - panel_h) * 0.5,
        panel_w,
        panel_h,
    );

    column("bg-panel border rounded-md p-5 gap-4")
        .child(
            row("row gap-3 items-center")
                .child(icon(UiIcon::Lock).accent(palette::ACCENT).class("size-10"))
                .child(
                    column("gap-1 flex-1")
                        .child(text("Trust Container").class("text-text truncate"))
                        .child(text("Unlock required").class("text-muted truncate")),
                ),
        )
        .child(
            text("Your identity, contacts, route policy, app secrets, and decrypt capability are sealed locally.")
                .class("text-muted"),
        )
        .child(checkbox("Keep verified cache available after unlock", true, 902))
        .child(
            field_node("Unlock secret", "Password or passkey ceremony")
                .hit_id(LOCK_UNLOCK_FIELD_ID)
                .class("h-24"),
        )
        .child(
            row("row gap-3")
                .child(button("Unlock", LOCK_UNLOCK_BUTTON_ID, ButtonStyle::Primary).class("h-10 flex-1"))
                .child(button("Offline", LOCK_UNLOCK_BUTTON_ID + 1, ButtonStyle::Ghost).class("h-10 w-28")),
        )
        .render_with_state(ui, panel, Some(&app.runtime));
}

fn render_capability_request_app(ui: &mut UiPainter<'_, '_>, bounds: UiRect, app: &UiAppSurface) {
    ui.fill_rect(bounds, 0.0, palette::BG);
    let panel_w = bounds.w.clamp(340.0, 720.0);
    let panel_h = 430.0_f32.min((bounds.h - 32.0).max(300.0));
    let panel = UiRect::new(
        bounds.x + (bounds.w - panel_w) * 0.5,
        bounds.y + (bounds.h - panel_h) * 0.5,
        panel_w,
        panel_h,
    );

    column("bg-panel border rounded-md p-5 gap-4")
        .child(
            row("row gap-3 items-center")
                .child(
                    icon(UiIcon::Shield)
                        .accent(palette::VIOLET)
                        .class("size-10"),
                )
                .child(
                    column("gap-1 flex-1")
                        .child(text("Capability request").class("text-text truncate"))
                        .child(text("Review before signing").class("text-muted truncate")),
                )
                .child(badge("admission", palette::ACCENT)),
        )
        .child(
            grid("grid grid-cols-2 gap-3", 2)
                .child(
                    metric("Requesting app", "EdgeRun Chat")
                        .detail("session scoped")
                        .class("h-28"),
                )
                .child(
                    metric("Capability", "Decrypt message")
                        .detail("Trust Container")
                        .class("h-28"),
                ),
        )
        .child(
            column("bg-row border rounded-md p-3 gap-2")
                .child(capability_grant_row(
                    "EdgeRun Chat",
                    "decrypt message",
                    "single use",
                    923,
                ))
                .child(route_path("Admission route", &["chat", "device", "trust"])),
        )
        .child(
            row("row gap-3")
                .child(
                    button("Deny", CAPABILITY_DENY_BUTTON_ID, ButtonStyle::Danger)
                        .class("h-10 w-28"),
                )
                .child(
                    button(
                        "Details",
                        CAPABILITY_DETAILS_BUTTON_ID,
                        ButtonStyle::Secondary,
                    )
                    .class("h-10 w-32"),
                )
                .child(
                    button("Allow", CAPABILITY_ALLOW_BUTTON_ID, ButtonStyle::Primary)
                        .class("h-10 flex-1"),
                ),
        )
        .render_with_state(ui, panel, Some(&app.runtime));
}

fn render_component_gallery_app(ui: &mut UiPainter<'_, '_>, bounds: UiRect, app: &UiAppSurface) {
    scroll_area("bg-panel border rounded-md p-4 gap-4", 0.0)
        .scroll_id(760)
        .children([
            header("Component Gallery").detail("shared Rust GPU primitives"),
            section("Foundation", "inputs, buttons, icons"),
            row("gap-2 items-center")
                .child(button("Run", 761, ButtonStyle::Primary).class("h-8 w-24"))
                .child(
                    button("Waiting", 762, ButtonStyle::Secondary)
                        .class("h-8 w-28")
                        .loading(true),
                )
                .child(icon_button(UiIcon::Settings, 763).class("size-8"))
                .child(icon(UiIcon::Shield).accent(palette::VIOLET).class("size-8")),
            row("gap-4 items-center")
                .child(checkbox("Verify cache", true, 764).class("h-8 w-44"))
                .child(radio("DAO admission", true, 765).class("h-8 w-44"))
                .child(select_node("Route", "relay://nodes", 766).class("h-14 w-56")),
            section("Feedback", "system surfaces"),
            row("gap-3")
                .child(
                    toast("Package hash verified", UiIcon::Check, palette::GREEN)
                        .class("h-11 flex-1"),
                )
                .child(progress_ring(0.64, palette::ACCENT).class("size-11")),
            empty_state(
                "No proofs yet",
                "Runtime events will appear after signed work is admitted.",
                UiIcon::Trust,
            )
            .class("h-40"),
            section("Data", "tables and navigation"),
            breadcrumb(&["Trust", "Routes", "Relay"], 2, 770).class("h-8"),
            command_palette("Search contacts, packages, routes", 780).class("h-11"),
            table_labels(
                &["Object", "Policy", "State"],
                &[
                    &["chat.app", "policy:run", "cached"],
                    &["relay path", "admission", "active"],
                    &["receipt", "payable", "pending"],
                ],
                790,
            )
            .class("h-40"),
            section("EdgeRun", "domain components"),
            grid_auto("grid grid-cols-1 md:grid-cols-2 gap-3")
                .child(identity_card(
                    "Local identity",
                    "browser node",
                    "policy:personal",
                    800,
                ))
                .child(package_card(
                    "EdgeRun Chat",
                    "run by hash",
                    "b3:message-ui",
                    801,
                )),
            contact_card("Codex Client", "local app identity", 802),
            thread_row("Alice", "encrypted message available", true, 803),
            capability_grant_row("Chat", "decrypt message", "single use", 804),
            proof_event_row("Relay delivery", "b3:relay-proof", "accepted", 805),
            route_path("Message route", &["app", "device", "admission", "relay"]),
            receipt_row("Relay delivery", "$0.0004", "pending", 806),
        ])
        .render_with_state(ui, bounds.inset(14.0, 14.0), Some(&app.runtime));
}

fn render_generic_workspace_app(ui: &mut UiPainter<'_, '_>, bounds: UiRect, app: &UiAppSurface) {
    card("bg-panel border rounded-md p-4 gap-3")
        .child(text(&app.title).class("text-text truncate"))
        .child(text("No app renderer registered.").class("text-muted truncate"))
        .render_with_state(ui, bounds.inset(14.0, 14.0), Some(&app.runtime));
}

fn push_label(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    text: &str,
    scale: f32,
    color: Color4,
) {
    #[cfg(feature = "fontdue-text")]
    if let Some(atlas) = atlas {
        scene.push_font_text(atlas, x, y, text, color);
        return;
    }
    scene.push_text(x, y, text, scale, color);
}

fn push_bounded_label(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    max_w: f32,
    text: &str,
    scale: f32,
    color: Color4,
) {
    if max_w <= 0.0 {
        return;
    }
    let label = truncate_label_to_width(
        text,
        max_w,
        scale,
        #[cfg(feature = "fontdue-text")]
        atlas,
    );
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x,
        y,
        &label,
        scale,
        color,
    );
}

fn draw_pill(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    color: Color4,
) {
    scene.push_rect(GpuRect::fill(x, y, w, 28.0, 14.0, color.with_alpha(0.14)));
    scene.push_rect(GpuRect::border(x, y, w, 28.0, 14.0, color.with_alpha(0.46)));
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 12.0,
        y + 7.0,
        (w - 24.0).max(0.0),
        label,
        2.0,
        color,
    );
}

fn draw_contact_row(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    w: f32,
    contact: &UnifiedContact<'_>,
    selected: bool,
) {
    soft_card(
        scene,
        x,
        y,
        w,
        56.0,
        10.0,
        if selected {
            palette::ACTIVE_ROW
        } else {
            palette::ROW
        },
    );
    let accent = match contact.kind {
        UnifiedContactKind::Person => palette::ACCENT,
        UnifiedContactKind::CodexClient => palette::VIOLET,
        UnifiedContactKind::Node => palette::GREEN,
    };
    scene.push_rect(GpuRect::fill(
        x + 14.0,
        y + 13.0,
        30.0,
        30.0,
        15.0,
        accent.with_alpha(0.22),
    ));
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 23.0,
        y + 19.0,
        10.0,
        contact_initial(contact.name),
        2.0,
        accent,
    );
    scene.push_rect(GpuRect::fill(
        x + 38.0,
        y + 36.0,
        8.0,
        8.0,
        4.0,
        if contact.online {
            palette::GREEN
        } else {
            palette::MUTED
        },
    ));
    let unread_w = if contact.unread > 0 { 50.0 } else { 0.0 };
    let text_w = (w - 72.0 - unread_w).max(0.0);
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 56.0,
        y + 11.0,
        text_w,
        contact.name,
        2.0,
        if selected {
            palette::TEXT
        } else {
            palette::MUTED
        },
    );
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 56.0,
        y + 32.0,
        text_w,
        contact.detail,
        2.0,
        accent,
    );
    if contact.unread > 0 {
        let label = if contact.unread > 9 { "9+" } else { "new" };
        draw_pill(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            x + w - 58.0,
            y + 14.0,
            42.0,
            label,
            palette::AMBER,
        );
    }
}

fn contact_initial(name: &str) -> &str {
    name.get(0..1).unwrap_or("?")
}

fn draw_message(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    w: f32,
    role: &str,
    body: &str,
    fill: Color4,
    accent: Color4,
) -> f32 {
    let body_w = (w - 42.0).max(120.0);
    let lines = wrap_lines(
        body,
        body_w,
        4,
        #[cfg(feature = "fontdue-text")]
        atlas,
    );
    let h = 56.0 + lines.len() as f32 * 22.0;
    soft_card(scene, x, y, w, h, 14.0, fill);
    scene.push_rect(GpuRect::fill(x, y, 4.0, h, 2.0, accent));
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 22.0,
        y + 16.0,
        (w - 44.0).max(0.0),
        role,
        2.0,
        accent,
    );
    for (index, line) in lines.iter().enumerate() {
        push_bounded_label(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            x + 22.0,
            y + 42.0 + index as f32 * 22.0,
            body_w,
            line,
            2.0,
            palette::TEXT,
        );
    }
    h
}

fn estimate_message_height(
    text: &str,
    max_width: f32,
    max_lines: usize,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> f32 {
    56.0 + wrap_lines(
        text,
        max_width,
        max_lines,
        #[cfg(feature = "fontdue-text")]
        atlas,
    )
    .len() as f32
        * 22.0
}

fn wrap_lines(
    text: &str,
    max_width: f32,
    max_lines: usize,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if measure_label_width(
            &candidate,
            2.0,
            #[cfg(feature = "fontdue-text")]
            atlas,
        ) <= max_width
        {
            current = candidate;
            continue;
        }

        if !current.is_empty() {
            lines.push(current);
        }
        current = word.to_string();

        if lines.len() + 1 >= max_lines {
            break;
        }
    }

    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    let consumed_words = lines
        .iter()
        .map(|line| line.split_whitespace().count())
        .sum::<usize>();
    let total_words = text.split_whitespace().count();
    if consumed_words < total_words {
        if let Some(last) = lines.last_mut() {
            while !last.is_empty()
                && measure_label_width(
                    &format!("{last}..."),
                    2.0,
                    #[cfg(feature = "fontdue-text")]
                    atlas,
                ) > max_width
            {
                last.pop();
            }
            last.push_str("...");
        }
    }

    lines
}

fn measure_label_width(
    text: &str,
    scale: f32,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> f32 {
    #[cfg(feature = "fontdue-text")]
    if let Some(atlas) = atlas {
        return atlas.text_width(text);
    }
    text.chars().count() as f32 * scale.max(1.0) * 6.0
}

fn truncate_label_to_width(
    text: &str,
    max_width: f32,
    scale: f32,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> String {
    if measure_label_width(
        text,
        scale,
        #[cfg(feature = "fontdue-text")]
        atlas,
    ) <= max_width
    {
        return text.to_string();
    }

    let ellipsis = "...";
    let ellipsis_w = measure_label_width(
        ellipsis,
        scale,
        #[cfg(feature = "fontdue-text")]
        atlas,
    );
    if ellipsis_w > max_width {
        return String::new();
    }

    let mut out = String::new();
    for ch in text.chars() {
        out.push(ch);
        let candidate = format!("{out}{ellipsis}");
        if measure_label_width(
            &candidate,
            scale,
            #[cfg(feature = "fontdue-text")]
            atlas,
        ) > max_width
        {
            out.pop();
            break;
        }
    }
    out.push_str(ellipsis);
    out
}

fn component_label_width(
    text: &str,
    scale: f32,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> f32 {
    measure_label_width(
        text,
        scale,
        #[cfg(feature = "fontdue-text")]
        atlas,
    )
}

fn render_children(
    ui: &mut UiPainter<'_, '_>,
    rect: UiRect,
    style: &UiStyle,
    children: &[UiNode],
    state: Option<&UiRuntimeState>,
) {
    if children.is_empty() {
        return;
    }
    let content = UiRect {
        x: rect.x + style.padding[3],
        y: rect.y + style.padding[0],
        w: (rect.w - style.padding[1] - style.padding[3]).max(0.0),
        h: (rect.h - style.padding[0] - style.padding[2]).max(0.0),
    };
    let main_available = match style.direction {
        Axis::Horizontal => content.w,
        Axis::Vertical => content.h,
    };
    let mut fixed = 0.0;
    let mut grow_count = 0usize;
    for child in children {
        if child.style.grow {
            grow_count += 1;
            continue;
        }
        fixed += child_main_size(child, style.direction);
    }
    let gap_total = style.gap * children.len().saturating_sub(1) as f32;
    let grow_size = if grow_count > 0 {
        ((main_available - fixed - gap_total).max(0.0)) / grow_count as f32
    } else {
        0.0
    };
    let mut main_sizes = Vec::with_capacity(children.len());
    for child in children {
        main_sizes.push(if child.style.grow {
            grow_size
        } else {
            child_main_size(child, style.direction)
        });
    }
    let main_sum: f32 = main_sizes.iter().sum();
    let mut gap = style.gap;
    let slack = (main_available - main_sum - gap_total).max(0.0);
    let start_offset = if grow_count > 0 {
        0.0
    } else {
        match style.justify {
            JustifyContent::Start | JustifyContent::Between => 0.0,
            JustifyContent::Center => slack * 0.5,
            JustifyContent::End => slack,
        }
    };
    if grow_count == 0 && matches!(style.justify, JustifyContent::Between) && children.len() > 1 {
        gap = (main_available - main_sum).max(0.0) / children.len().saturating_sub(1) as f32;
    }

    let mut cursor = match style.direction {
        Axis::Horizontal => content.x + start_offset,
        Axis::Vertical => content.y + start_offset,
    };
    for (child, main) in children.iter().zip(main_sizes) {
        let child_rect = match style.direction {
            Axis::Horizontal => {
                let cross =
                    aligned_cross(content.y, content.h, child, Axis::Horizontal, style.align);
                let w = main.max(0.0).min((content.x + content.w - cursor).max(0.0));
                UiRect::new(cursor, cross.0, w, cross.1)
            }
            Axis::Vertical => {
                let cross = aligned_cross(content.x, content.w, child, Axis::Vertical, style.align);
                let h = main.max(0.0).min((content.y + content.h - cursor).max(0.0));
                UiRect::new(cross.0, cursor, cross.1, h)
            }
        };
        child.render_with_state(ui, child_rect, state);
        cursor += main + gap;
    }
}

fn aligned_cross(
    content_start: f32,
    content_size: f32,
    child: &UiNode,
    parent_axis: Axis,
    align: AlignItems,
) -> (f32, f32) {
    let explicit = match parent_axis {
        Axis::Horizontal => child.style.height,
        Axis::Vertical => child.style.width,
    };
    let size = match explicit {
        Some(value) if value >= 0.0 => value.min(content_size),
        _ if matches!(align, AlignItems::Stretch) => content_size,
        _ => child_cross_size(child, parent_axis).min(content_size),
    }
    .max(0.0);
    let offset = match align {
        AlignItems::Start | AlignItems::Stretch => 0.0,
        AlignItems::Center => (content_size - size).max(0.0) * 0.5,
        AlignItems::End => (content_size - size).max(0.0),
    };
    (content_start + offset, size)
}

fn render_grid_children(
    ui: &mut UiPainter<'_, '_>,
    rect: UiRect,
    style: &UiStyle,
    columns: u16,
    children: &[UiNode],
    state: Option<&UiRuntimeState>,
) {
    if children.is_empty() {
        return;
    }
    let content = UiRect {
        x: rect.x + style.padding[3],
        y: rect.y + style.padding[0],
        w: (rect.w - style.padding[1] - style.padding[3]).max(0.0),
        h: (rect.h - style.padding[0] - style.padding[2]).max(0.0),
    };
    let columns = columns.max(1) as usize;
    let gap = style.gap;
    let track_w = ((content.w - gap * columns.saturating_sub(1) as f32) / columns as f32).max(0.0);
    let mut row_y = content.y;
    let mut index = 0usize;

    while index < children.len() && row_y < content.y + content.h {
        let row_start = index;
        let mut row_col = 0usize;
        let mut row_h = 0.0_f32;
        while index < children.len() {
            let child = &children[index];
            let span = child.style.col_span.max(1).min(columns as u16) as usize;
            if row_col > 0 && row_col + span > columns {
                break;
            }
            row_h = row_h.max(child_main_size(child, Axis::Vertical));
            row_col += span;
            index += 1;
            if row_col >= columns {
                break;
            }
        }

        let mut col = 0usize;
        for child in &children[row_start..index] {
            let span = child.style.col_span.max(1).min(columns as u16) as usize;
            let w = track_w * span as f32 + gap * span.saturating_sub(1) as f32;
            let h = child_main_size(child, Axis::Vertical).min(content.y + content.h - row_y);
            let x = content.x + col as f32 * (track_w + gap);
            child.render_with_state(ui, UiRect::new(x, row_y, w, h), state);
            col += span;
        }
        row_y += row_h + gap;
    }
}

fn render_scroll_children(
    ui: &mut UiPainter<'_, '_>,
    rect: UiRect,
    style: &UiStyle,
    offset: f32,
    id: Option<u32>,
    children: &[UiNode],
    state: Option<&UiRuntimeState>,
) {
    if children.is_empty() {
        return;
    }
    let content = UiRect {
        x: rect.x + style.padding[3],
        y: rect.y + style.padding[0],
        w: (rect.w - style.padding[1] - style.padding[3] - 10.0).max(0.0),
        h: (rect.h - style.padding[0] - style.padding[2]).max(0.0),
    };
    if content.w <= 0.0 || content.h <= 0.0 {
        return;
    }

    let total: f32 = children
        .iter()
        .map(|child| child_main_size(child, Axis::Vertical))
        .sum::<f32>()
        + style.gap * children.len().saturating_sub(1) as f32;
    let scrollable = (total - content.h).max(0.0);
    let scroll_y = scrollable * offset.clamp(0.0, 1.0);
    let mut cursor = content.y - scroll_y;

    let clipped = ui
        .scene
        .push_clip(GpuClip::new(content.x, content.y, content.w, content.h));
    for child in children {
        let h = child_main_size(child, Axis::Vertical);
        let bottom = cursor + h;
        if bottom >= content.y && cursor <= content.y + content.h && clipped {
            child.render_with_state(ui, UiRect::new(content.x, cursor, content.w, h), state);
        }
        cursor += h + style.gap;
    }
    if clipped {
        ui.scene.pop_clip();
    }

    if total > content.h {
        let scrollbar = UiRect::new(rect.x + rect.w - 6.0, content.y, 3.0, content.h);
        if let Some(id) = id {
            ui.hit(
                HitKind::Scrollbar,
                id,
                scrollbar.x - 7.0,
                scrollbar.y,
                10.0,
                scrollbar.h,
            );
        }
        ui.scrollbar(scrollbar, content.h / total, offset);
    }
}

fn child_main_size(child: &UiNode, axis: Axis) -> f32 {
    match axis {
        Axis::Horizontal => child.style.width.unwrap_or_else(|| intrinsic_width(child)),
        Axis::Vertical => child
            .style
            .height
            .unwrap_or_else(|| intrinsic_height(child)),
    }
}

fn child_cross_size(child: &UiNode, parent_axis: Axis) -> f32 {
    match parent_axis {
        Axis::Horizontal => child
            .style
            .height
            .unwrap_or_else(|| intrinsic_height(child)),
        Axis::Vertical => child.style.width.unwrap_or_else(|| intrinsic_width(child)),
    }
}

fn intrinsic_width(child: &UiNode) -> f32 {
    match &child.kind {
        UiNodeKind::Text(value) => (value.chars().count() as f32 * 8.0 + 2.0).clamp(24.0, 220.0),
        UiNodeKind::Badge { label, .. } => {
            (label.chars().count() as f32 * 8.0 + 22.0).clamp(36.0, 180.0)
        }
        UiNodeKind::Button { label, .. } => (label.chars().count() as f32 * 8.0 + 28.0).max(44.0),
        UiNodeKind::IconButton { .. } => 34.0,
        UiNodeKind::Icon { .. } => 24.0,
        UiNodeKind::Checkbox { label, .. } | UiNodeKind::Radio { label, .. } => {
            (label.chars().count() as f32 * 8.0 + 40.0).clamp(80.0, 260.0)
        }
        UiNodeKind::Select { .. } => child.style.width.unwrap_or(220.0),
        UiNodeKind::Tooltip { text } => {
            (text.chars().count() as f32 * 8.0 + 22.0).clamp(80.0, 260.0)
        }
        UiNodeKind::Dialog { .. } => child.style.width.unwrap_or(420.0),
        UiNodeKind::Toast { message, .. } => {
            (message.chars().count() as f32 * 8.0 + 62.0).clamp(180.0, 420.0)
        }
        UiNodeKind::EmptyState { .. } => child.style.width.unwrap_or(280.0),
        UiNodeKind::Skeleton => child.style.width.unwrap_or(160.0),
        UiNodeKind::ProgressRing { .. } => child.style.width.unwrap_or(48.0),
        UiNodeKind::Table { .. } => child.style.width.unwrap_or(420.0),
        UiNodeKind::Breadcrumb { .. } => child.style.width.unwrap_or(360.0),
        UiNodeKind::CommandPalette { .. } => child.style.width.unwrap_or(320.0),
        UiNodeKind::TreeItem { .. } => child.style.width.unwrap_or(260.0),
        UiNodeKind::Section { .. } => child.style.width.unwrap_or(260.0),
        UiNodeKind::IdentityCard { .. }
        | UiNodeKind::PackageCard { .. }
        | UiNodeKind::RoutePath { .. } => child.style.width.unwrap_or(280.0),
        UiNodeKind::ContactCard { .. }
        | UiNodeKind::ThreadRow { .. }
        | UiNodeKind::AttachmentPreview { .. }
        | UiNodeKind::CapabilityGrantRow { .. }
        | UiNodeKind::ProofEventRow { .. }
        | UiNodeKind::ReceiptRow { .. }
        | UiNodeKind::AppLauncherItem { .. } => child.style.width.unwrap_or(320.0),
        UiNodeKind::Toggle { .. } => 46.0,
        UiNodeKind::Avatar { .. } => 36.0,
        UiNodeKind::ProgressBar { .. } => 120.0,
        UiNodeKind::Tabs { labels, .. } => (labels.len().max(1) as f32 * 82.0).clamp(120.0, 360.0),
        UiNodeKind::PanelHeader { .. } => 280.0,
        UiNodeKind::MetricCard { .. } => 220.0,
        UiNodeKind::Field { .. } => 240.0,
        UiNodeKind::TextArea { .. } => 280.0,
        UiNodeKind::Slider { .. } => 240.0,
        UiNodeKind::BarChart { .. } => 320.0,
        UiNodeKind::TransactionRow { .. } => 320.0,
        UiNodeKind::MenuItem { .. } => 220.0,
        UiNodeKind::ListRow { .. } => 220.0,
        UiNodeKind::ControlRow { .. } => 260.0,
        UiNodeKind::Divider => 1.0,
        UiNodeKind::Spacer => child.style.width.unwrap_or(12.0),
        UiNodeKind::ScrollArea { .. } => child.style.width.unwrap_or(240.0),
        _ => child.style.width.unwrap_or(120.0),
    }
}

fn intrinsic_height(child: &UiNode) -> f32 {
    match &child.kind {
        UiNodeKind::Text(_) => 20.0,
        UiNodeKind::Badge { .. } => 22.0,
        UiNodeKind::Button { .. } => 34.0,
        UiNodeKind::IconButton { .. } => 34.0,
        UiNodeKind::Icon { .. } => 24.0,
        UiNodeKind::Checkbox { .. } | UiNodeKind::Radio { .. } => 34.0,
        UiNodeKind::Select { .. } => 58.0,
        UiNodeKind::Tooltip { .. } => 34.0,
        UiNodeKind::Dialog { .. } => child.style.height.unwrap_or(220.0),
        UiNodeKind::Toast { .. } => 44.0,
        UiNodeKind::EmptyState { .. } => child.style.height.unwrap_or(170.0),
        UiNodeKind::Skeleton => child.style.height.unwrap_or(22.0),
        UiNodeKind::ProgressRing { .. } => child.style.height.unwrap_or(48.0),
        UiNodeKind::Table { rows, .. } => child
            .style
            .height
            .unwrap_or(48.0 + rows.len() as f32 * 34.0),
        UiNodeKind::Breadcrumb { .. } => 32.0,
        UiNodeKind::CommandPalette { .. } => 46.0,
        UiNodeKind::TreeItem { .. } => 36.0,
        UiNodeKind::Section { .. } => 34.0,
        UiNodeKind::IdentityCard { .. } | UiNodeKind::PackageCard { .. } => {
            child.style.height.unwrap_or(112.0)
        }
        UiNodeKind::RoutePath { .. } => child.style.height.unwrap_or(86.0),
        UiNodeKind::ContactCard { .. } | UiNodeKind::AttachmentPreview { .. } => 64.0,
        UiNodeKind::ThreadRow { .. }
        | UiNodeKind::CapabilityGrantRow { .. }
        | UiNodeKind::ProofEventRow { .. }
        | UiNodeKind::ReceiptRow { .. }
        | UiNodeKind::AppLauncherItem { .. } => 58.0,
        UiNodeKind::Toggle { .. } => 24.0,
        UiNodeKind::Avatar { .. } => 36.0,
        UiNodeKind::ProgressBar { .. } => 8.0,
        UiNodeKind::Tabs { .. } => 34.0,
        UiNodeKind::PanelHeader { .. } => 44.0,
        UiNodeKind::MetricCard { .. } => 132.0,
        UiNodeKind::Field { .. } => 92.0,
        UiNodeKind::TextArea { .. } => 132.0,
        UiNodeKind::Slider { .. } => 78.0,
        UiNodeKind::BarChart { .. } => 180.0,
        UiNodeKind::TransactionRow { .. } => 58.0,
        UiNodeKind::MenuItem { .. } => 58.0,
        UiNodeKind::ListRow { .. } => 58.0,
        UiNodeKind::ControlRow { .. } => 58.0,
        UiNodeKind::Divider => 1.0,
        UiNodeKind::Spacer => child.style.height.unwrap_or(12.0),
        UiNodeKind::ScrollArea { .. } => child.style.height.unwrap_or(180.0),
        _ => child.style.height.unwrap_or(44.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_node_builder_renders_component_tree() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-panel border rounded-md p-4 gap-3")
                .children([
                    tab_labels(&["Routes", "Proofs", "Files"], 1, 80).class("h-9"),
                    row("gap-2 h-10 items-center")
                        .child(avatar_node("EdgeRun", palette::ACCENT).online(true))
                        .child(text("Dashboard").class("flex-1 text-text truncate"))
                        .child(badge("sealed", palette::GREEN)),
                    metric("Relay balance", "$0.00")
                        .detail("pending setup")
                        .progress(0.32)
                        .class("h-32"),
                    field_node("Endpoint", "nodes.edgerun.tech")
                        .detail("identity-routed relay")
                        .focused(true),
                    list_row_node("Admission route", "policy-bound relay", 83)
                        .accent(palette::GREEN),
                    control_row_node("Relay enabled")
                        .detail("use admitted identity route")
                        .control_toggle(true, 84)
                        .hit_id(85),
                    menu_item_node("Payments", 42)
                        .detail("proof-backed receipts")
                        .badge_text("new")
                        .selected(true),
                    button("Open", 43, ButtonStyle::Secondary).class("h-8"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 360.0, 620.0));
        }

        assert!(scene.rects().len() > 20);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Tab && hit.id == 81)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 83)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Toggle && hit.id == 84)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 85)
        );
    }

    #[test]
    fn ui_node_builder_renders_dashboard_primitives() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            let months = ["Jan", "Feb", "Mar", "Apr"];
            let values = [0.42, 0.72, 0.55, 0.91];
            column("bg-panel border rounded-md p-4 gap-3")
                .children([
                    header("Contribution History")
                        .detail("Last 4 months")
                        .action("View", 50),
                    bar_chart_labels("Relay Receipts", &months, &values).class("h-44"),
                    slider_node("Payout threshold", 0.62, 51)
                        .range_labels("$50", "$10,000")
                        .accent(palette::GREEN),
                    text_area_node("Notes", "Route budget and admission notes").focused(true),
                    transaction_node("Stripe payout", "+$4,200.00", 52)
                        .detail("Income")
                        .date("Today")
                        .positive(true),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 420.0, 620.0));
        }

        assert!(scene.rects().len() > 35);
        assert!(scene.hits().len() >= 3);
    }

    #[test]
    fn gpu_ui_macro_composes_nested_trees() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            gpu_ui!(
                column("bg-panel border rounded-md p-4 gap-3"),
                [
                    gpu_ui!(
                        row("gap-2 h-10 items-center"),
                        [
                            avatar_node("EdgeRun", palette::ACCENT).online(true),
                            text("Components").class("flex-1 text-text truncate"),
                            badge("rust", palette::ACCENT),
                            icon_button(UiIcon::ChevronRight, 69).class("size-8")
                        ]
                    ),
                    metric("Storage", "128 MB").detail("verified cache"),
                    progress_bar_node(0.58, palette::GREEN).class("h-2"),
                    toggle_node(true, 68),
                    button("Run", 70, ButtonStyle::Primary).class("h-8")
                ]
            )
            .render(&mut ui, UiRect::new(0.0, 0.0, 320.0, 320.0));
        }

        assert!(scene.rects().len() > 25);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Toggle && hit.id == 68)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 69)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 70)
        );
    }

    #[test]
    fn disabled_and_loading_nodes_suppress_interaction_hits() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("gap-2")
                .children([
                    button("Disabled", 71, ButtonStyle::Secondary)
                        .class("h-8")
                        .disabled(true),
                    button("Loading", 72, ButtonStyle::Primary)
                        .class("h-8")
                        .loading(true),
                    button("Ready", 73, ButtonStyle::Primary).class("h-8"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 240.0, 130.0));
        }

        assert!(!scene.hits().iter().any(|hit| hit.id == 71));
        assert!(!scene.hits().iter().any(|hit| hit.id == 72));
        assert!(scene.hits().iter().any(|hit| hit.id == 73));
        assert!(scene.rects().len() > 10);
    }

    #[test]
    fn keyboard_focus_cycles_and_activates_controls() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("gap-2")
                .children([
                    button("Run", 81, ButtonStyle::Primary).class("h-8"),
                    checkbox("Cache verified bytes", false, 82).class("h-8"),
                    field_node("Filter", "").hit_id(83).class("h-16"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 260.0, 150.0));
        }

        let mut runtime = UiRuntimeState::default();
        assert!(matches!(
            runtime.focus_first(&scene),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Button && hit.id == 81
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Enter }),
            UiAction::Activated(hit) if hit.kind == HitKind::Button && hit.id == 81
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Tab }),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Checkbox && hit.id == 82
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Enter }),
            UiAction::Toggled { id: 82, on: true }
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Tab }),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Input && hit.id == 83
        ));
    }

    #[test]
    fn scroll_area_only_renders_visible_rows() {
        let rows = (0..10).map(|index| {
            list_row_node(
                &format!("Route {}", index + 1),
                "identity-routed relay",
                100 + index,
            )
        });

        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            scroll_area("bg-panel border rounded-md p-2 gap-2", 1.0)
                .scroll_id(99)
                .children(rows)
                .render(&mut ui, UiRect::new(0.0, 0.0, 320.0, 160.0));
        }

        let visible_rows = scene
            .hits()
            .iter()
            .filter(|hit| hit.kind == HitKind::ListRow)
            .count();
        assert!((1..10).contains(&visible_rows));
        assert!(
            !scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 100)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 109)
        );
        assert!(
            scene
                .hits()
                .iter()
                .filter(|hit| hit.kind == HitKind::ListRow)
                .all(|hit| hit.y >= 8.0 && hit.y + hit.h <= 152.0)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Scrollbar && hit.id == 99)
        );
    }

    #[test]
    fn scene_clip_stack_clips_rects_and_hits() {
        let mut scene = GpuScene::new(palette::BG);
        assert!(scene.push_clip(GpuClip::new(10.0, 20.0, 80.0, 40.0)));
        scene.push_rect(GpuRect::fill(0.0, 0.0, 40.0, 40.0, 8.0, palette::ACCENT));
        scene.push_hit(GpuHit::new(HitKind::Button, 7, 0.0, 0.0, 40.0, 40.0));
        scene.push_rect(GpuRect::fill(120.0, 0.0, 20.0, 20.0, 0.0, palette::ACCENT));
        scene.pop_clip();

        assert_eq!(scene.rects().len(), 1);
        assert_eq!(
            (
                scene.rects()[0].x,
                scene.rects()[0].y,
                scene.rects()[0].w,
                scene.rects()[0].h
            ),
            (10.0, 20.0, 30.0, 20.0)
        );
        assert_eq!(scene.hits().len(), 1);
        assert_eq!(
            (
                scene.hits()[0].x,
                scene.hits()[0].y,
                scene.hits()[0].w,
                scene.hits()[0].h
            ),
            (10.0, 20.0, 30.0, 20.0)
        );
    }

    #[test]
    fn ui_runtime_state_emits_semantic_actions() {
        let mut scene = GpuScene::new(palette::BG);
        scene.push_hit(GpuHit::new(HitKind::Toggle, 4, 10.0, 10.0, 46.0, 24.0));
        scene.push_hit(GpuHit::new(HitKind::Input, 7, 10.0, 44.0, 160.0, 34.0));
        scene.push_hit(GpuHit::new(HitKind::Scrollbar, 9, 180.0, 10.0, 8.0, 120.0));
        let mut state = UiRuntimeState::default();

        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerDown { x: 12.0, y: 12.0 }),
            UiAction::Activated(GpuHit::new(HitKind::Toggle, 4, 10.0, 10.0, 46.0, 24.0))
        );
        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerUp { x: 12.0, y: 12.0 }),
            UiAction::Toggled { id: 4, on: true }
        );

        assert!(matches!(
            state.handle_event(&scene, UiEvent::PointerDown { x: 20.0, y: 50.0 }),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Input && hit.id == 7
        ));
        assert_eq!(
            state.handle_event(&scene, UiEvent::TextInput("ok".to_string())),
            UiAction::TextChanged {
                id: 7,
                value: "ok".to_string()
            }
        );
        assert_eq!(
            state.handle_event(
                &scene,
                UiEvent::KeyDown {
                    key: UiKey::Backspace,
                },
            ),
            UiAction::TextChanged {
                id: 7,
                value: "o".to_string()
            }
        );

        assert_eq!(
            state.handle_event(
                &scene,
                UiEvent::Wheel {
                    x: 182.0,
                    y: 20.0,
                    delta_y: 180.0,
                },
            ),
            UiAction::ScrollChanged { id: 9, offset: 0.2 }
        );
    }

    #[test]
    fn ui_node_render_consumes_runtime_scroll_state() {
        let rows = (0..8).map(|index| {
            list_row_node(
                &format!("Runtime row {}", index + 1),
                "state owned by rust",
                130 + index,
            )
        });
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(77, 1.0);

        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            scroll_area("bg-panel border rounded-md p-2 gap-2", 0.0)
                .scroll_id(77)
                .children(rows)
                .render_with_state(&mut ui, UiRect::new(0.0, 0.0, 320.0, 150.0), Some(&runtime));
        }

        assert!(
            !scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 130)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 137)
        );
    }

    #[test]
    fn workspace_tiles_apps_without_overlap_and_clips_surfaces() {
        let mut workspace = UiWorkspace::split(
            UiTileAxis::Horizontal,
            UiAppSurface::new(1, "Chat"),
            UiAppSurface::new(2, "Trust"),
        );
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            workspace.render(
                &mut ui,
                UiRect::new(0.0, 0.0, 640.0, 360.0),
                |ui, bounds, app| {
                    ui.fill_rect(bounds, 0.0, palette::PANEL);
                    ui.bounded_label(
                        bounds.x + 12.0,
                        bounds.y + 14.0,
                        bounds.w - 24.0,
                        &app.title,
                        2.0,
                        palette::TEXT,
                    );
                    ui.hit(
                        HitKind::Button,
                        app.id + 100,
                        bounds.x,
                        bounds.y,
                        bounds.w,
                        bounds.h,
                    );
                },
            );
        }

        let first = workspace.app(1).and_then(UiAppSurface::bounds).unwrap();
        let second = workspace.app(2).and_then(UiAppSurface::bounds).unwrap();
        assert!(first.x + first.w <= second.x);
        assert!(first.h <= 360.0 - WORKSPACE_CHROME_H);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::WorkspaceTab && hit.id == 1)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::WorkspaceClose && hit.id == 2)
        );
        assert!(
            scene
                .hits()
                .iter()
                .filter(|hit| hit.kind == HitKind::Button)
                .all(|hit| hit.y >= WORKSPACE_CHROME_H)
        );
    }

    #[test]
    fn workspace_routes_pointer_events_to_app_runtime() {
        let mut workspace = UiWorkspace::single(UiAppSurface::new(7, "Chat"));
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            workspace.render(
                &mut ui,
                UiRect::new(0.0, 0.0, 320.0, 220.0),
                |ui, bounds, _app| {
                    ui.hit(
                        HitKind::Toggle,
                        55,
                        bounds.x + 20.0,
                        bounds.y + 20.0,
                        46.0,
                        24.0,
                    );
                    ui.toggle(bounds.x + 20.0, bounds.y + 20.0, false, 55);
                },
            );
        }

        let down = workspace.handle_event(&scene, UiEvent::PointerDown { x: 24.0, y: 58.0 });
        assert!(matches!(
            down,
            UiWorkspaceAction::AppAction {
                app_id: 7,
                action: UiAction::Activated(_)
            }
        ));
        let up = workspace.handle_event(&scene, UiEvent::PointerUp { x: 24.0, y: 58.0 });
        assert_eq!(
            up,
            UiWorkspaceAction::AppAction {
                app_id: 7,
                action: UiAction::Toggled { id: 55, on: true }
            }
        );
        assert!(
            workspace
                .app(7)
                .is_some_and(|app| app.runtime.toggle_value(55, false))
        );
    }

    #[test]
    fn shell_overlay_toggles_launcher_and_opens_apps() {
        let mut shell = UiShellState::default();
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            render_edgerun_shell_overlay(&mut ui, UiRect::new(0.0, 0.0, 900.0, 600.0), &mut shell);
        }
        let launcher_hit = scene
            .hits()
            .iter()
            .find(|hit| hit.kind == HitKind::ShellLauncher)
            .copied()
            .expect("shell launcher hit");
        let action = shell.handle_event(
            &scene,
            UiEvent::PointerDown {
                x: launcher_hit.x + 4.0,
                y: launcher_hit.y + 4.0,
            },
        );
        assert_eq!(action, UiShellAction::ToggledLauncher(true));
        assert!(shell.launcher_open);

        scene.clear_rects();
        {
            let mut ui = UiPainter::new(&mut scene);
            render_edgerun_shell_overlay(&mut ui, UiRect::new(0.0, 0.0, 900.0, 600.0), &mut shell);
        }
        let gallery_hit = scene
            .hits()
            .iter()
            .find(|hit| hit.id == LAUNCH_COMPONENT_GALLERY_ITEM_ID)
            .copied()
            .expect("gallery hit");
        let action = shell.handle_event(
            &scene,
            UiEvent::PointerDown {
                x: gallery_hit.x + 4.0,
                y: gallery_hit.y + 4.0,
            },
        );
        assert_eq!(
            action,
            UiShellAction::OpenApp {
                app_id: COMPONENT_GALLERY_APP_ID,
                kind: UiAppKind::ComponentGallery,
            }
        );
        assert!(!shell.launcher_open);
    }

    #[test]
    fn full_screen_system_app_replaces_workspace_chrome() {
        let mut workspace = UiWorkspace::full_screen(UiAppSurface::lock_screen(10));
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            workspace.render(
                &mut ui,
                UiRect::new(0.0, 0.0, 640.0, 360.0),
                |ui, bounds, app| {
                    ui.fill_rect(bounds, 0.0, palette::PANEL);
                    ui.hit(
                        HitKind::Button,
                        app.id + 100,
                        bounds.x,
                        bounds.y,
                        bounds.w,
                        bounds.h,
                    );
                },
            );
        }

        let bounds = workspace.app(10).and_then(UiAppSurface::bounds).unwrap();
        assert_eq!(bounds, UiRect::new(0.0, 0.0, 640.0, 360.0));
        assert!(
            !scene
                .hits()
                .iter()
                .any(|hit| matches!(hit.kind, HitKind::WorkspaceTab | HitKind::WorkspaceClose))
        );
    }

    #[test]
    fn lock_and_capability_apps_render_expected_actions() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            let app = UiAppSurface::lock_screen(10);
            render_lock_screen_app(&mut ui, UiRect::new(0.0, 0.0, 800.0, 520.0), &app);
        }
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == LOCK_UNLOCK_BUTTON_ID)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Input && hit.id == LOCK_UNLOCK_FIELD_ID)
        );

        scene.clear_rects();
        {
            let mut ui = UiPainter::new(&mut scene);
            let app = UiAppSurface::capability_request(11);
            render_capability_request_app(&mut ui, UiRect::new(0.0, 0.0, 900.0, 620.0), &app);
        }
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == CAPABILITY_ALLOW_BUTTON_ID)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == CAPABILITY_DENY_BUTTON_ID)
        );
    }

    #[test]
    fn component_gallery_renders_reusable_primitives() {
        let mut scene = GpuScene::new(palette::BG);
        let app = UiAppSurface::component_gallery(12);
        {
            let mut ui = UiPainter::new(&mut scene);
            render_component_gallery_app(&mut ui, UiRect::new(0.0, 0.0, 900.0, 1100.0), &app);
        }

        assert!(scene.rects().len() > 80);
        assert!(scene.hits().iter().any(|hit| hit.id == 761));
        assert!(!scene.hits().iter().any(|hit| hit.id == 762));
        assert!(scene.hits().iter().any(|hit| hit.id == 780));
        assert!(scene.hits().iter().any(|hit| hit.id == 800));
    }

    #[cfg(feature = "tabler-svg-atlas")]
    #[test]
    fn canonical_icons_resolve_tabler_svg_atlas_rects() {
        let atlas = tabler_svg_icon_atlas();
        assert_eq!(
            atlas.width,
            crate::tabler_svg_atlas_generated::TABLER_SVG_ATLAS_W
        );
        assert_eq!(atlas.alpha.len(), (atlas.width * atlas.height) as usize);
        let rect = UiIcon::Trust
            .tabler_svg_atlas_rect()
            .expect("trust icon atlas rect");
        assert_eq!(rect.name, "shield-check");
        assert!(rect.u0 >= 0.0 && rect.u1 <= 1.0 && rect.u0 < rect.u1);
        assert!(rect.v0 >= 0.0 && rect.v1 <= 1.0 && rect.v0 < rect.v1);
    }

    #[test]
    fn grid_node_places_spanned_dashboard_cards() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            let labels = ["A", "B", "C"];
            let values = [0.25, 0.5, 0.75];
            grid("bg-panel border rounded-md p-3 gap-3", 4)
                .children([
                    metric("Balance", "$42.00")
                        .progress(0.4)
                        .span(2)
                        .class("h-32"),
                    field_node("Relay", "nodes.edgerun.tech")
                        .focused(true)
                        .span(2),
                    bar_chart_labels("Activity", &labels, &values)
                        .span(4)
                        .class("h-44"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 640.0, 420.0));
        }

        assert!(scene.rects().len() > 30);
        assert!(scene.hits().is_empty());
    }

    #[test]
    fn grid_auto_reads_columns_from_classes() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid_auto("grid grid-cols-3 bg-panel border rounded-md p-3 gap-3")
                .children([
                    metric("One", "1").class("h-24"),
                    metric("Two", "2").class("h-24"),
                    metric("Three", "3").class("h-24"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 480.0, 160.0));
        }

        assert!(scene.rects().len() > 20);
    }

    #[test]
    fn grid_auto_for_width_applies_responsive_columns() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid_auto_for_width(
                "grid grid-cols-1 md:grid-cols-3 bg-panel p-3 gap-2 md:gap-3",
                900.0,
            )
            .children([
                metric("One", "1").class("h-24"),
                metric("Two", "2").class("h-24"),
                metric("Three", "3").class("h-24"),
            ])
            .render(&mut ui, UiRect::new(0.0, 0.0, 480.0, 160.0));
        }

        assert!(scene.rects().len() > 20);
    }

    #[test]
    fn alignment_classes_control_child_placement() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            row("bg-panel border rounded-md p-2 items-center justify-between")
                .children([
                    text("Left").class("w-12"),
                    button("Right", 80, ButtonStyle::Secondary).class("w-20 h-8"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 320.0, 80.0));
        }

        let hit = scene
            .hits()
            .iter()
            .find(|hit| hit.id == 80)
            .expect("button hit");
        assert!(hit.x > 220.0);
        assert!((hit.y - 24.0).abs() < 0.1);
    }

    #[test]
    fn column_children_stretch_by_default() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-panel border rounded-md p-2 gap-2")
                .child(button("Wide", 81, ButtonStyle::Secondary).class("h-8"))
                .render(&mut ui, UiRect::new(0.0, 0.0, 220.0, 80.0));
        }

        let hit = scene
            .hits()
            .iter()
            .find(|hit| hit.id == 81)
            .expect("button hit");
        assert!(hit.w > 190.0);
    }

    #[test]
    fn conditional_children_do_not_allocate_placeholder_layout() {
        let mut hidden = row("gap-2")
            .when(false, text("hidden"))
            .child(text("visible"));
        let shown = row("gap-2").when(true, text("visible"));

        assert_eq!(hidden.children.len(), 1);
        assert_eq!(shown.children.len(), 1);
        hidden = hidden.child(text("second"));
        assert_eq!(hidden.children.len(), 2);
    }

    #[test]
    fn canonical_icons_map_to_replaceable_provider_names() {
        assert_eq!(UiIconSet::default(), UiIconSet::Tabler);
        assert_eq!(UiIcon::Trust.name(), "trust");
        assert_eq!(
            UiIcon::Trust.provider_name(UiIconSet::Tabler),
            "shield-check"
        );
        assert_eq!(
            UiIcon::Trust.provider_name(UiIconSet::Lucide),
            "shield-check"
        );
        assert_eq!(
            UiIcon::Terminal.provider_name(UiIconSet::Tabler),
            "terminal-2"
        );
        assert_eq!(
            UiIcon::Terminal.provider_name(UiIconSet::Lucide),
            "square-terminal"
        );
    }

    #[test]
    fn canonical_icon_node_renders_gpu_geometry() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            row("row gap-2 items-center")
                .child(icon(UiIcon::Lock).accent(palette::ACCENT).class("size-8"))
                .child(icon_button(UiIcon::X, 91).class("size-8"))
                .render(&mut ui, UiRect::new(0.0, 0.0, 120.0, 48.0));
        }

        assert!(scene.rects().len() > 8);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 91)
        );
    }

    #[test]
    fn foundation_form_controls_render_and_emit_state() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-panel border rounded-md p-3 gap-2")
                .child(checkbox("Cache verified bytes", false, 201))
                .child(radio("Personal admission", false, 202))
                .child(select_node("Relay endpoint", "assigned by admission", 203))
                .child(tooltip("Only the Trust Container can decrypt."))
                .render(&mut ui, UiRect::new(0.0, 0.0, 360.0, 190.0));
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Checkbox && hit.id == 201)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Radio && hit.id == 202)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Select && hit.id == 203)
        );

        let mut runtime = UiRuntimeState::default();
        let down = runtime.handle_event(&scene, UiEvent::PointerDown { x: 18.0, y: 18.0 });
        assert!(matches!(down, UiAction::Activated(hit) if hit.kind == HitKind::Checkbox));
        assert_eq!(
            runtime.handle_event(&scene, UiEvent::PointerUp { x: 18.0, y: 18.0 }),
            UiAction::Toggled { id: 201, on: true }
        );

        let _ = runtime.handle_event(&scene, UiEvent::PointerDown { x: 18.0, y: 54.0 });
        assert_eq!(
            runtime.handle_event(&scene, UiEvent::PointerUp { x: 18.0, y: 54.0 }),
            UiAction::Toggled { id: 202, on: true }
        );
    }

    #[test]
    fn feedback_components_render_without_custom_surfaces() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid("grid grid-cols-2 bg-bg p-3 gap-3", 2)
                .child(
                    dialog(
                        "Capability request",
                        "Review the admitted action before signing.",
                        UiIcon::Shield,
                    )
                    .class("h-44"),
                )
                .child(empty_state(
                    "No proofs yet",
                    "Runtime events will appear here.",
                    UiIcon::File,
                ))
                .child(toast("Route admitted", UiIcon::Check, palette::GREEN).span(2))
                .child(skeleton().class("h-6"))
                .child(progress_ring(0.64, palette::ACCENT).class("size-12"))
                .render(&mut ui, UiRect::new(0.0, 0.0, 720.0, 420.0));
        }

        assert!(scene.rects().len() > 40);
        assert!(scene.hits().is_empty());
    }

    #[test]
    fn data_navigation_components_render_semantic_hits() {
        let rows: &[&[&str]] = &[
            &["admission", "policy", "ready"],
            &["relay", "route", "pending"],
        ];
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-bg p-3 gap-3")
                .child(breadcrumb(&["Trust", "Routes", "Relay"], 2, 300))
                .child(command_palette("Search commands, apps, proofs...", 310))
                .child(table_labels(&["Node", "Kind", "State"], rows, 320).class("h-32"))
                .child(section("Node instances", "identity + role + policy"))
                .child(tree_item(
                    "admission:personal",
                    "local policy",
                    0,
                    true,
                    330,
                ))
                .child(tree_item("relay:nodes", "websocket", 1, false, 331))
                .render(&mut ui, UiRect::new(0.0, 0.0, 640.0, 420.0));
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Breadcrumb && hit.id == 302)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Input && hit.id == 310)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 320)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::TreeItem && hit.id == 331)
        );
    }

    #[test]
    fn edgerun_domain_components_render_semantic_rows() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid("grid grid-cols-2 bg-bg p-3 gap-3", 2)
                .child(identity_card("Ken", "browser-node", "personal policy", 400))
                .child(package_card("Chat", "free-run", "b3f2...a91", 401))
                .child(route_path("Admitted route", &["app", "device", "relay", "user"]).span(2))
                .child(contact_card("Codex client", "app contact", 402))
                .child(thread_row("Alice", "Encrypted message", true, 403))
                .child(attachment_preview("photo.jpg", "image payload", 404))
                .child(capability_grant_row(
                    "Chat",
                    "decrypt message",
                    "single use",
                    405,
                ))
                .child(proof_event_row("Package verified", "hash:b3f2", "ok", 406))
                .child(receipt_row("Relay delivery", "+$0.01", "settled", 407))
                .render(&mut ui, UiRect::new(0.0, 0.0, 760.0, 620.0));
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 400)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 406)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::TransactionRow && hit.id == 407)
        );
    }
}

fn soft_card(scene: &mut GpuScene, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
    scene.push_rect(GpuRect::shadow(
        x,
        y + 4.0,
        w,
        h,
        radius,
        Color4::rgba(0.0, 0.0, 0.0, 0.24),
        10.0,
    ));
    panel(scene, x, y, w, h, radius, color);
}

fn panel(scene: &mut GpuScene, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
    scene.push_rect(GpuRect::fill(x, y, w, h, radius, color));
    scene.push_rect(GpuRect::border(x, y, w, h, radius, palette::BORDER));
}
