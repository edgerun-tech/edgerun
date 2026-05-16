use std::string::String;
use std::vec::Vec;

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UiOverlayLayer {
    Popover,
    Tooltip,
    Toast,
    Modal,
}

pub const UI_OVERLAY_STACKING_ORDER: &[UiOverlayLayer] = &[
    UiOverlayLayer::Popover,
    UiOverlayLayer::Tooltip,
    UiOverlayLayer::Toast,
    UiOverlayLayer::Modal,
];

impl UiOverlayLayer {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Popover => "popover",
            Self::Tooltip => "tooltip",
            Self::Toast => "toast",
            Self::Modal => "modal",
        }
    }
}

#[derive(Clone, Debug)]
pub enum UiNodeKind {
    Row,
    Column,
    Grid {
        columns: u16,
    },
    Masonry {
        columns: u16,
    },
    BentoGrid {
        columns: u16,
    },
    Card,
    ScrollArea {
        offset: f32,
        offset_px: Option<f32>,
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
        tooltip: String,
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
        masked: bool,
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
        bar_base_id: Option<u32>,
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
    pub(super) kind: UiNodeKind,
    pub(super) style: UiStyle,
    pub(super) children: Vec<UiNode>,
    pub(super) interaction: UiInteraction,
    pub(super) transitions: Vec<UiTransitionSpec>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiInteraction {
    pub drag_source: Option<UiDragSourceSpec>,
    pub drop_target: Option<UiDropTargetSpec>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiDragSourceSpec {
    pub scope_id: u32,
    pub item_id: u32,
    pub index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiDropTargetSpec {
    pub scope_id: u32,
    pub index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiResolvedLayout {
    pub kind: &'static str,
    pub rect: UiRect,
    pub children: Vec<UiResolvedLayout>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiLayoutIssue {
    pub path: String,
    pub kind: &'static str,
    pub rect: UiRect,
    pub parent: UiRect,
    pub message: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiCompositionIssue {
    pub path: String,
    pub parent_kind: &'static str,
    pub child_kind: &'static str,
    pub message: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiLayoutTraceEntry {
    pub path: String,
    pub kind: &'static str,
    pub rect: UiRect,
    pub child_count: usize,
}

impl UiNode {
    pub fn row(classes: &str) -> Self {
        let mut style = UiStyle::parse(classes);
        style.direction = Axis::Horizontal;
        Self {
            kind: UiNodeKind::Row,
            style,
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn column(classes: &str) -> Self {
        let mut style = UiStyle::parse(classes);
        style.direction = Axis::Vertical;
        Self {
            kind: UiNodeKind::Column,
            style,
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn grid(classes: &str, columns: u16) -> Self {
        Self {
            kind: UiNodeKind::Grid {
                columns: columns.max(1),
            },
            style: UiStyle::parse(classes),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn masonry(classes: &str, columns: u16) -> Self {
        Self {
            kind: UiNodeKind::Masonry {
                columns: columns.max(1),
            },
            style: UiStyle::parse(classes),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn masonry_auto(classes: &str) -> Self {
        let style = UiStyle::parse(classes);
        Self {
            kind: UiNodeKind::Masonry {
                columns: style.grid_cols.unwrap_or(1),
            },
            style,
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn masonry_auto_for_width(classes: &str, width: f32) -> Self {
        let style = UiStyle::parse_for_width(classes, width);
        Self {
            kind: UiNodeKind::Masonry {
                columns: style.grid_cols.unwrap_or(1),
            },
            style,
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn bento_grid(classes: &str, columns: u16) -> Self {
        Self {
            kind: UiNodeKind::BentoGrid {
                columns: columns.max(1),
            },
            style: UiStyle::parse(classes),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn bento_grid_auto(classes: &str) -> Self {
        let style = UiStyle::parse(classes);
        Self {
            kind: UiNodeKind::BentoGrid {
                columns: style.grid_cols.unwrap_or(1),
            },
            style,
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn bento_grid_auto_for_width(classes: &str, width: f32) -> Self {
        let style = UiStyle::parse_for_width(classes, width);
        Self {
            kind: UiNodeKind::BentoGrid {
                columns: style.grid_cols.unwrap_or(1),
            },
            style,
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn card(classes: &str) -> Self {
        Self {
            kind: UiNodeKind::Card,
            style: UiStyle::parse(classes),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn scroll_area(classes: &str, offset: f32) -> Self {
        let mut style = UiStyle::parse(classes);
        style.direction = Axis::Vertical;
        Self {
            kind: UiNodeKind::ScrollArea {
                offset,
                offset_px: None,
                id: None,
            },
            style,
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn scroll_area_px(classes: &str, offset_px: f32) -> Self {
        let mut style = UiStyle::parse(classes);
        style.direction = Axis::Vertical;
        Self {
            kind: UiNodeKind::ScrollArea {
                offset: 0.0,
                offset_px: Some(offset_px.max(0.0)),
                id: None,
            },
            style,
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn text(value: &str) -> Self {
        Self {
            kind: UiNodeKind::Text(value.to_string()),
            style: UiStyle::default(),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn icon_button(icon: UiIcon, id: u32) -> Self {
        Self {
            kind: UiNodeKind::IconButton {
                icon,
                id,
                active: true,
                tooltip: icon.tooltip_label().to_string(),
            },
            style: UiStyle::default(),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn icon_button_with_tooltip(icon: UiIcon, id: u32, tooltip: &str) -> Self {
        Self {
            kind: UiNodeKind::IconButton {
                icon,
                id,
                active: true,
                tooltip: tooltip.to_string(),
            },
            style: UiStyle::default(),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn tooltip(text: &str) -> Self {
        Self {
            kind: UiNodeKind::Tooltip {
                text: text.to_string(),
            },
            style: UiStyle::default(),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn overlay_layer(&self) -> Option<UiOverlayLayer> {
        overlay_layer_for_kind(&self.kind)
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn skeleton() -> Self {
        Self {
            kind: UiNodeKind::Skeleton,
            style: UiStyle::default(),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn progress_ring(value: f32, color: Color4) -> Self {
        Self {
            kind: UiNodeKind::ProgressRing { value, color },
            style: UiStyle::default(),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn toggle(on: bool, id: u32) -> Self {
        Self {
            kind: UiNodeKind::Toggle { on, id },
            style: UiStyle::default(),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn progress_bar(value: f32, color: Color4) -> Self {
        Self {
            kind: UiNodeKind::ProgressBar { value, color },
            style: UiStyle::default(),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn field(label: &str, value: &str) -> Self {
        Self {
            kind: UiNodeKind::Field {
                label: label.to_string(),
                value: value.to_string(),
                helper: String::new(),
                focused: false,
                masked: false,
                id: None,
            },
            style: UiStyle::default(),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
                bar_base_id: None,
            },
            style: UiStyle::default(),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
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
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn divider(classes: &str) -> Self {
        Self {
            kind: UiNodeKind::Divider,
            style: UiStyle::parse(classes),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn spacer(classes: &str) -> Self {
        Self {
            kind: UiNodeKind::Spacer,
            style: UiStyle::parse(classes),
            children: Vec::new(),
            interaction: UiInteraction::default(),
            transitions: Vec::new(),
        }
    }

    pub fn class(mut self, classes: &str) -> Self {
        let mut parsed = UiStyle::parse(classes);
        self.apply_parsed_style(&mut parsed);
        overlay_style(&mut self.style, parsed);
        self
    }

    pub fn class_for_width(mut self, classes: &str, width: f32) -> Self {
        let mut parsed = UiStyle::parse_for_width(classes, width);
        self.apply_parsed_style(&mut parsed);
        overlay_style(&mut self.style, parsed);
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
        if self.style.row_span > 1 && parsed.row_span == 1 {
            parsed.row_span = self.style.row_span;
        }
    }

    pub fn span(mut self, span: u16) -> Self {
        self.style.col_span = span.max(1);
        self
    }

    pub fn row_span(mut self, span: u16) -> Self {
        self.style.row_span = span.max(1);
        self
    }

    pub fn scroll_offset(mut self, offset: f32) -> Self {
        if let UiNodeKind::ScrollArea {
            offset: node_offset,
            offset_px,
            ..
        } = &mut self.kind
        {
            *node_offset = offset;
            *offset_px = None;
        }
        self
    }

    pub fn scroll_offset_px(mut self, offset_px_value: f32) -> Self {
        if let UiNodeKind::ScrollArea { offset_px, .. } = &mut self.kind {
            *offset_px = Some(offset_px_value.max(0.0));
        }
        self
    }

    pub fn scroll_id(mut self, id: u32) -> Self {
        if let UiNodeKind::ScrollArea { id: node_id, .. } = &mut self.kind {
            *node_id = Some(id);
        }
        self
    }

    pub fn draggable(mut self, scope_id: u32, item_id: u32, index: usize) -> Self {
        self.interaction.drag_source = Some(UiDragSourceSpec {
            scope_id,
            item_id,
            index,
        });
        self
    }

    pub fn drop_target(mut self, scope_id: u32, index: usize) -> Self {
        self.interaction.drop_target = Some(UiDropTargetSpec { scope_id, index });
        self
    }

    pub fn reorderable(mut self, scope_id: u32, item_id: u32, index: usize) -> Self {
        self.interaction.drag_source = Some(UiDragSourceSpec {
            scope_id,
            item_id,
            index,
        });
        self.interaction.drop_target = Some(UiDropTargetSpec { scope_id, index });
        self
    }

    pub fn transition(mut self, spec: UiTransitionSpec) -> Self {
        self.transitions.push(spec);
        self
    }

    pub fn fade(mut self, id: u32, from: f32, to: f32, duration_ms: u32) -> Self {
        self.transitions
            .push(UiTransitionSpec::opacity(id, from, to, duration_ms));
        self
    }

    pub fn slide_x(mut self, id: u32, from: f32, to: f32, duration_ms: u32) -> Self {
        self.transitions
            .push(UiTransitionSpec::translate_x(id, from, to, duration_ms));
        self
    }

    pub fn slide_y(mut self, id: u32, from: f32, to: f32, duration_ms: u32) -> Self {
        self.transitions
            .push(UiTransitionSpec::translate_y(id, from, to, duration_ms));
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

    pub fn chart_bar_base_id(mut self, id: u32) -> Self {
        if let UiNodeKind::BarChart { bar_base_id, .. } = &mut self.kind {
            *bar_base_id = Some(id);
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

    pub fn masked(mut self, masked_value: bool) -> Self {
        if let UiNodeKind::Field { masked, .. } = &mut self.kind {
            *masked = masked_value;
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

    pub fn tooltip_label(mut self, tooltip: &str) -> Self {
        if let UiNodeKind::IconButton {
            tooltip: node_tooltip,
            ..
        } = &mut self.kind
        {
            *node_tooltip = tooltip.to_string();
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

    pub fn resolve_layout(&self, bounds: UiRect) -> UiResolvedLayout {
        self.resolve_layout_with_state(bounds, None)
    }

    pub fn resolve_layout_with_state(
        &self,
        bounds: UiRect,
        state: Option<&UiRuntimeState>,
    ) -> UiResolvedLayout {
        let rect = self.style.layout_rect(bounds);
        self.resolve_layout_resolved(rect, state)
    }

    pub fn layout_issues(&self, bounds: UiRect) -> Vec<UiLayoutIssue> {
        let layout = self.resolve_layout(bounds);
        let mut issues = Vec::new();
        collect_layout_issues(&layout, "$", &mut issues);
        issues
    }

    pub fn composition_issues(&self) -> Vec<UiCompositionIssue> {
        let mut issues = Vec::new();
        collect_composition_issues(self, "$", None, &mut issues);
        issues
    }

    pub fn layout_trace(&self, bounds: UiRect) -> Vec<UiLayoutTraceEntry> {
        let layout = self.resolve_layout(bounds);
        let mut trace = Vec::new();
        collect_layout_trace(&layout, "$", &mut trace);
        trace
    }

    fn resolve_layout_resolved(
        &self,
        rect: UiRect,
        state: Option<&UiRuntimeState>,
    ) -> UiResolvedLayout {
        let children = match &self.kind {
            UiNodeKind::Grid { columns } => {
                resolve_grid_children(rect, &self.style, *columns, &self.children, state)
            }
            UiNodeKind::Masonry { columns } => {
                resolve_masonry_children(rect, &self.style, *columns, &self.children, state)
            }
            UiNodeKind::BentoGrid { columns } => {
                resolve_bento_children(rect, &self.style, *columns, &self.children, state)
            }
            UiNodeKind::ScrollArea {
                offset,
                offset_px,
                id,
                ..
            } => {
                let offset = id
                    .and_then(|id| {
                        state.map(|state| ScrollOffset::Fraction(state.scroll_offset(id)))
                    })
                    .or_else(|| offset_px.map(ScrollOffset::Pixels))
                    .unwrap_or(ScrollOffset::Fraction(*offset));
                resolve_scroll_children(rect, &self.style, offset, &self.children, state)
            }
            UiNodeKind::Row | UiNodeKind::Column | UiNodeKind::Card => {
                resolve_stack_children(rect, &self.style, &self.children, state)
            }
            UiNodeKind::Dialog { .. } => resolve_stack_children(
                rect.inset(
                    spacing::DIALOG_CONTENT_INSET_X,
                    spacing::DIALOG_CONTENT_INSET_Y,
                ),
                &self.style,
                &self.children,
                state,
            ),
            _ => Vec::new(),
        };
        UiResolvedLayout {
            kind: self.kind_name(),
            rect,
            children,
        }
    }

    pub fn render_with_state(
        &self,
        ui: &mut UiPainter<'_, '_>,
        bounds: UiRect,
        state: Option<&UiRuntimeState>,
    ) {
        let rect = self.style.layout_rect(bounds);
        self.render_resolved_with_state(ui, rect, state);
    }

    fn render_resolved_with_state(
        &self,
        ui: &mut UiPainter<'_, '_>,
        rect: UiRect,
        state: Option<&UiRuntimeState>,
    ) {
        let transition_cursor = ui.scene.cursor();
        self.emit_interaction(ui, rect);
        let hit_start = ui.scene.hit_count();
        match &self.kind {
            UiNodeKind::Text(value) => {
                let color = self.style.text.resolve(ui.theme());
                if self.style.truncate {
                    ui.bounded_label(
                        rect.x,
                        rect.y,
                        rect.w,
                        value.lines().next().unwrap_or(""),
                        2.0,
                        color,
                    );
                } else {
                    let max_lines = (rect.h / 20.0).floor().max(1.0) as usize;
                    ui.wrapped_label(rect.x, rect.y, rect.w, value, max_lines, 20.0, color);
                }
            }
            UiNodeKind::Badge { label, color } => {
                ui.badge(rect.x, rect.y, label, *color);
            }
            UiNodeKind::Button { label, id, style } => {
                let h = self.style.height.unwrap_or(spacing::BUTTON_H).min(rect.h);
                ui.button(
                    UiRect::new(rect.x, rect.y, rect.w, h),
                    label,
                    *style,
                    *id,
                    true,
                );
            }
            UiNodeKind::IconButton {
                icon, id, active, ..
            } => {
                let size = self
                    .style
                    .width
                    .or(self.style.height)
                    .unwrap_or(spacing::ICON_BUTTON)
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
                let cursor = ui.scene.cursor();
                ui.tooltip(rect, text);
                label_overlay_commands(ui, cursor, UiOverlayLayer::Tooltip);
            }
            UiNodeKind::Dialog { title, body, icon } => {
                let cursor = ui.scene.cursor();
                ui.dialog(rect, title, body, *icon);
                render_children(
                    ui,
                    rect.inset(
                        spacing::DIALOG_CONTENT_INSET_X,
                        spacing::DIALOG_CONTENT_INSET_Y,
                    ),
                    &self.style,
                    &self.children,
                    state,
                );
                label_overlay_commands(ui, cursor, UiOverlayLayer::Modal);
            }
            UiNodeKind::Toast {
                message,
                icon,
                accent,
            } => {
                let cursor = ui.scene.cursor();
                ui.toast(rect, message, *icon, *accent);
                label_overlay_commands(ui, cursor, UiOverlayLayer::Toast);
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
                let selected = state
                    .map(|state| state.selected_tab_index(*base_id, labels.len(), *selected))
                    .unwrap_or(*selected);
                ui.segmented_tabs(
                    UiRect::new(rect.x, rect.y, rect.w, h),
                    &label_refs,
                    selected,
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
                masked,
                id,
            } => {
                let value = id
                    .and_then(|id| state.map(|state| state.text_value(id, value)))
                    .unwrap_or(value);
                let display_value = if *masked && !value.is_empty() {
                    "•".repeat(value.chars().count())
                } else {
                    value.to_string()
                };
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
                        value: &display_value,
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
                bar_base_id,
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
                        bar_base_id: *bar_base_id,
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
            | UiNodeKind::Masonry { .. }
            | UiNodeKind::BentoGrid { .. }
            | UiNodeKind::Card
            | UiNodeKind::ScrollArea { .. } => {
                if self.style.shadow > 0.0 {
                    ui.scene.push_rect(GpuRect::shadow(
                        rect.x,
                        rect.y,
                        rect.w,
                        rect.h,
                        self.style.radius,
                        ui.theme().colors.bg.with_alpha(0.78),
                        self.style.shadow,
                    ));
                }
                if let Some(gradient) = self.style.gradient {
                    let radius = if matches!(self.kind, UiNodeKind::Card) {
                        self.style.radius.min(spacing::CARD_RADIUS_MAX)
                    } else {
                        self.style.radius
                    };
                    ui.scene.push_rect(GpuRect::linear_gradient(
                        rect.x,
                        rect.y,
                        rect.w,
                        rect.h,
                        radius,
                        gradient.from.resolve(ui.theme()),
                        gradient.to.resolve(ui.theme()),
                    ));
                    if self.style.border {
                        ui.border_rect(rect, radius, ui.theme().colors.border.with_alpha(0.42));
                    }
                } else if let Some(bg) = self.style.bg.map(|color| color.resolve(ui.theme())) {
                    if matches!(self.kind, UiNodeKind::Card) {
                        let radius = self.style.radius.min(spacing::CARD_RADIUS_MAX);
                        ui.fill_rect(rect, radius, bg);
                        if self.style.border {
                            ui.border_rect(rect, radius, ui.theme().colors.border);
                        }
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
                                ui.theme().colors.border,
                            ));
                        }
                    }
                }
                let clipped = self.style.clip
                    && ui
                        .scene
                        .push_clip(GpuClip::new(rect.x, rect.y, rect.w, rect.h));
                if let UiNodeKind::Grid { columns } = &self.kind {
                    render_grid_children(ui, rect, &self.style, *columns, &self.children, state);
                } else if let UiNodeKind::Masonry { columns } = &self.kind {
                    render_masonry_children(ui, rect, &self.style, *columns, &self.children, state);
                } else if let UiNodeKind::BentoGrid { columns } = &self.kind {
                    render_bento_children(ui, rect, &self.style, *columns, &self.children, state);
                } else if let UiNodeKind::ScrollArea {
                    offset,
                    offset_px,
                    id,
                    ..
                } = &self.kind
                {
                    let offset = id
                        .and_then(|id| {
                            state.map(|state| ScrollOffset::Fraction(state.scroll_offset(id)))
                        })
                        .or_else(|| offset_px.map(ScrollOffset::Pixels))
                        .unwrap_or(ScrollOffset::Fraction(*offset));
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
                if clipped {
                    ui.scene.pop_clip();
                }
            }
        }
        let added_hits = &ui.scene.hits()[hit_start..];
        let hovered_rect = state.and_then(|state| {
            let hovered = state.hovered()?;
            node_owns_hit(&self.kind, hovered).then(|| {
                added_hits
                    .iter()
                    .copied()
                    .find(|hit| hit.kind == hovered.kind && hit.id == hovered.id)
            })?
        });
        let active_rect = state.and_then(|state| {
            let active = state.active()?;
            node_owns_hit(&self.kind, active).then(|| {
                added_hits
                    .iter()
                    .copied()
                    .find(|hit| hit.kind == active.kind && hit.id == active.id)
            })?
        });
        let focused_rect = state.and_then(|state| {
            let focused = state.focused()?;
            if !node_owns_hit(&self.kind, focused) {
                return None;
            }
            added_hits
                .iter()
                .copied()
                .find(|hit| hit.kind == focused.kind && hit.id == focused.id)
        });
        if let Some(active) = active_rect {
            draw_interaction_state(ui, active, self.kind_interaction_radius(ui), true);
        } else if let Some(hovered) = hovered_rect {
            draw_interaction_state(ui, hovered, self.kind_interaction_radius(ui), false);
        }
        if let (UiNodeKind::IconButton { tooltip, .. }, Some(hovered)) = (&self.kind, hovered_rect)
        {
            if !tooltip.is_empty() {
                ui.tooltip(icon_button_tooltip_rect(hovered, tooltip), tooltip);
            }
        }
        if let Some(focused) = focused_rect {
            ui.border_rect(
                UiRect::new(focused.x, focused.y, focused.w, focused.h),
                ui.theme().radius.control,
                ui.theme().colors.accent.with_alpha(0.86),
            );
        }
        if self.style.disabled || self.style.loading {
            ui.scene.truncate_hits(hit_start);
            ui.fill_rect(
                rect,
                self.style
                    .radius
                    .min(spacing::CARD_RADIUS_MAX)
                    .max(ui.theme().radius.card),
                ui.theme().colors.bg.with_alpha(0.34),
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
                ui.theme().colors.accent,
            );
        }
        self.apply_transitions(ui, transition_cursor, state);
    }

    fn apply_transitions(
        &self,
        ui: &mut UiPainter<'_, '_>,
        cursor: GpuSceneCursor,
        state: Option<&UiRuntimeState>,
    ) {
        for transition in &self.transitions {
            ui.transition(*transition);
            let value = state
                .map(|state| state.transition_value(*transition))
                .unwrap_or(transition.to);
            match transition.property {
                UiTransitionProperty::Opacity => ui.scene.apply_opacity_since(cursor, value),
                UiTransitionProperty::TranslateX => ui.scene.translate_since(cursor, value, 0.0),
                UiTransitionProperty::TranslateY => ui.scene.translate_since(cursor, 0.0, value),
            }
        }
    }

    fn emit_interaction(&self, ui: &mut UiPainter<'_, '_>, rect: UiRect) {
        if self.style.disabled {
            return;
        }
        if let Some(source) = self.interaction.drag_source {
            ui.drag_source(
                source.scope_id,
                source.item_id,
                source.index,
                rect.x,
                rect.y,
                rect.w,
                rect.h,
            );
        }
        if let Some(target) = self.interaction.drop_target {
            ui.drop_target(
                target.scope_id,
                target.index,
                rect.x,
                rect.y,
                rect.w,
                rect.h,
            );
        }
    }

    fn kind_name(&self) -> &'static str {
        match self.kind {
            UiNodeKind::Row => "row",
            UiNodeKind::Column => "column",
            UiNodeKind::Grid { .. } => "grid",
            UiNodeKind::Masonry { .. } => "masonry",
            UiNodeKind::BentoGrid { .. } => "bento_grid",
            UiNodeKind::Card => "card",
            UiNodeKind::ScrollArea { .. } => "scroll_area",
            UiNodeKind::Text(_) => "text",
            UiNodeKind::Badge { .. } => "badge",
            UiNodeKind::Button { .. } => "button",
            UiNodeKind::IconButton { .. } => "icon_button",
            UiNodeKind::Icon { .. } => "icon",
            UiNodeKind::Checkbox { .. } => "checkbox",
            UiNodeKind::Radio { .. } => "radio",
            UiNodeKind::Select { .. } => "select",
            UiNodeKind::Tooltip { .. } => "tooltip",
            UiNodeKind::Dialog { .. } => "dialog",
            UiNodeKind::Toast { .. } => "toast",
            UiNodeKind::EmptyState { .. } => "empty_state",
            UiNodeKind::Skeleton => "skeleton",
            UiNodeKind::ProgressRing { .. } => "progress_ring",
            UiNodeKind::Table { .. } => "table",
            UiNodeKind::Breadcrumb { .. } => "breadcrumb",
            UiNodeKind::CommandPalette { .. } => "command_palette",
            UiNodeKind::TreeItem { .. } => "tree_item",
            UiNodeKind::Section { .. } => "section",
            UiNodeKind::IdentityCard { .. } => "identity_card",
            UiNodeKind::ContactCard { .. } => "contact_card",
            UiNodeKind::ThreadRow { .. } => "thread_row",
            UiNodeKind::AttachmentPreview { .. } => "attachment_preview",
            UiNodeKind::CapabilityGrantRow { .. } => "capability_grant_row",
            UiNodeKind::ProofEventRow { .. } => "proof_event_row",
            UiNodeKind::RoutePath { .. } => "route_path",
            UiNodeKind::PackageCard { .. } => "package_card",
            UiNodeKind::ReceiptRow { .. } => "receipt_row",
            UiNodeKind::AppLauncherItem { .. } => "app_launcher_item",
            UiNodeKind::Toggle { .. } => "toggle",
            UiNodeKind::Avatar { .. } => "avatar",
            UiNodeKind::ProgressBar { .. } => "progress_bar",
            UiNodeKind::Tabs { .. } => "tabs",
            UiNodeKind::PanelHeader { .. } => "panel_header",
            UiNodeKind::MetricCard { .. } => "metric_card",
            UiNodeKind::Field { .. } => "field",
            UiNodeKind::TextArea { .. } => "text_area",
            UiNodeKind::Slider { .. } => "slider",
            UiNodeKind::BarChart { .. } => "bar_chart",
            UiNodeKind::TransactionRow { .. } => "transaction_row",
            UiNodeKind::MenuItem { .. } => "menu_item",
            UiNodeKind::ListRow { .. } => "list_row",
            UiNodeKind::ControlRow { .. } => "control_row",
            UiNodeKind::Divider => "divider",
            UiNodeKind::Spacer => "spacer",
        }
    }
}

impl UiNode {
    fn kind_interaction_radius(&self, ui: &UiPainter<'_, '_>) -> f32 {
        match self.kind {
            UiNodeKind::Button { .. }
            | UiNodeKind::IconButton { .. }
            | UiNodeKind::Checkbox { .. }
            | UiNodeKind::Radio { .. }
            | UiNodeKind::Select { .. }
            | UiNodeKind::CommandPalette { .. }
            | UiNodeKind::Toggle { .. }
            | UiNodeKind::Tabs { .. }
            | UiNodeKind::Field { .. }
            | UiNodeKind::TextArea { .. }
            | UiNodeKind::Slider { .. } => ui.theme().radius.control,
            _ => ui.theme().radius.card,
        }
    }
}

fn draw_interaction_state(ui: &mut UiPainter<'_, '_>, hit: GpuHit, radius: f32, active: bool) {
    let colors = ui.theme().colors;
    let rect = UiRect::new(hit.x, hit.y, hit.w, hit.h);
    let (fill, border) = if active {
        (
            colors.accent.with_alpha(0.12),
            colors.accent.with_alpha(0.62),
        )
    } else {
        (colors.row.with_alpha(0.24), colors.accent.with_alpha(0.24))
    };
    ui.fill_rect(rect, radius, fill);
    ui.border_rect(rect, radius, border);
}

fn icon_button_tooltip_rect(hit: GpuHit, tooltip: &str) -> UiRect {
    let w = (tooltip.chars().count() as f32 * 7.0 + 24.0).clamp(44.0, 220.0);
    let h = 30.0;
    let x = hit.x + (hit.w - w) * 0.5;
    let y = if hit.y >= h + 8.0 {
        hit.y - h - 6.0
    } else {
        hit.y + hit.h + 6.0
    };
    UiRect::new(x, y, w, h)
}

fn overlay_layer_for_kind(kind: &UiNodeKind) -> Option<UiOverlayLayer> {
    match kind {
        UiNodeKind::Tooltip { .. } => Some(UiOverlayLayer::Tooltip),
        UiNodeKind::Dialog { .. } => Some(UiOverlayLayer::Modal),
        UiNodeKind::Toast { .. } => Some(UiOverlayLayer::Toast),
        _ => None,
    }
}

fn label_overlay_commands(
    ui: &mut UiPainter<'_, '_>,
    cursor: GpuSceneCursor,
    layer: UiOverlayLayer,
) {
    ui.scene
        .label_commands_since(cursor, &format!("overlay:{}", layer.label()));
}

fn node_owns_hit(kind: &UiNodeKind, hit: GpuHit) -> bool {
    match kind {
        UiNodeKind::Button { id, .. } | UiNodeKind::IconButton { id, .. } => {
            hit.kind == HitKind::Button && hit.id == *id
        }
        UiNodeKind::Checkbox { id, .. } => hit.kind == HitKind::Checkbox && hit.id == *id,
        UiNodeKind::Radio { id, .. } => hit.kind == HitKind::Radio && hit.id == *id,
        UiNodeKind::Select { id, .. } => hit.kind == HitKind::Select && hit.id == *id,
        UiNodeKind::CommandPalette { id, .. } => hit.kind == HitKind::Input && hit.id == *id,
        UiNodeKind::TreeItem { id, .. } => hit.kind == HitKind::TreeItem && hit.id == *id,
        UiNodeKind::IdentityCard { id, .. }
        | UiNodeKind::ContactCard { id, .. }
        | UiNodeKind::ThreadRow { id, .. }
        | UiNodeKind::AttachmentPreview { id, .. }
        | UiNodeKind::CapabilityGrantRow { id, .. }
        | UiNodeKind::ProofEventRow { id, .. }
        | UiNodeKind::PackageCard { id, .. }
        | UiNodeKind::AppLauncherItem { id, .. }
        | UiNodeKind::ListRow { id, .. } => hit.kind == HitKind::ListRow && hit.id == *id,
        UiNodeKind::ReceiptRow { id, .. } | UiNodeKind::TransactionRow { id, .. } => {
            hit.kind == HitKind::TransactionRow && hit.id == *id
        }
        UiNodeKind::Toggle { id, .. } => hit.kind == HitKind::Toggle && hit.id == *id,
        UiNodeKind::Tabs {
            labels, base_id, ..
        } => {
            hit.kind == HitKind::Tab
                && hit.id >= *base_id
                && hit.id < base_id.saturating_add(labels.len() as u32)
        }
        UiNodeKind::Table { rows, id_base, .. } => {
            hit.kind == HitKind::ListRow
                && hit.id >= *id_base
                && hit.id < id_base.saturating_add(rows.len() as u32)
        }
        UiNodeKind::Breadcrumb { items, base_id, .. } => {
            hit.kind == HitKind::Breadcrumb
                && hit.id >= *base_id
                && hit.id < base_id.saturating_add(items.len() as u32)
        }
        UiNodeKind::Field { id: Some(id), .. } => hit.kind == HitKind::Input && hit.id == *id,
        UiNodeKind::TextArea { id: Some(id), .. } => hit.kind == HitKind::TextArea && hit.id == *id,
        UiNodeKind::Slider { id, .. } => hit.kind == HitKind::Slider && hit.id == *id,
        UiNodeKind::MenuItem { id, .. } => hit.kind == HitKind::MenuItem && hit.id == *id,
        UiNodeKind::ControlRow { id: Some(id), .. } => {
            hit.kind == HitKind::ListRow && hit.id == *id
        }
        _ => false,
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

pub fn masonry(classes: &str, columns: u16) -> UiNode {
    UiNode::masonry(classes, columns)
}

pub fn masonry_auto(classes: &str) -> UiNode {
    UiNode::masonry_auto(classes)
}

pub fn masonry_auto_for_width(classes: &str, width: f32) -> UiNode {
    UiNode::masonry_auto_for_width(classes, width)
}

pub fn bento_grid(classes: &str, columns: u16) -> UiNode {
    UiNode::bento_grid(classes, columns)
}

pub fn bento_grid_auto(classes: &str) -> UiNode {
    UiNode::bento_grid_auto(classes)
}

pub fn bento_grid_auto_for_width(classes: &str, width: f32) -> UiNode {
    UiNode::bento_grid_auto_for_width(classes, width)
}

pub fn card(classes: &str) -> UiNode {
    UiNode::card(classes)
}

pub fn scroll_area(classes: &str, offset: f32) -> UiNode {
    UiNode::scroll_area(classes, offset)
}

pub fn scroll_area_px(classes: &str, offset_px: f32) -> UiNode {
    UiNode::scroll_area_px(classes, offset_px)
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

pub fn icon_button_with_tooltip(icon: UiIcon, id: u32, tooltip: &str) -> UiNode {
    UiNode::icon_button_with_tooltip(icon, id, tooltip)
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

fn overlay_style(style: &mut UiStyle, parsed: UiStyle) {
    let default = UiStyle::default();

    if parsed.direction != default.direction {
        style.direction = parsed.direction;
    }
    if parsed.gap != default.gap {
        style.gap = parsed.gap;
    }
    for index in 0..style.padding.len() {
        if parsed.padding[index] != default.padding[index] {
            style.padding[index] = parsed.padding[index];
        }
        if parsed.margin[index] != default.margin[index] {
            style.margin[index] = parsed.margin[index];
        }
    }
    if parsed.width.is_some() {
        style.width = parsed.width;
    }
    if parsed.height.is_some() {
        style.height = parsed.height;
    }
    style.grow |= parsed.grow;
    if parsed.grid_cols.is_some() {
        style.grid_cols = parsed.grid_cols;
    }
    if parsed.col_span != default.col_span {
        style.col_span = parsed.col_span;
    }
    if parsed.row_span != default.row_span {
        style.row_span = parsed.row_span;
    }
    if parsed.align != default.align {
        style.align = parsed.align;
    }
    if parsed.justify != default.justify {
        style.justify = parsed.justify;
    }
    if parsed.bg.is_some() {
        style.bg = parsed.bg;
    }
    if parsed.gradient.is_some() {
        style.gradient = parsed.gradient;
    }
    if parsed.text != default.text {
        style.text = parsed.text;
    }
    style.border |= parsed.border;
    if parsed.radius != default.radius {
        style.radius = parsed.radius;
    }
    if parsed.shadow > 0.0 {
        style.shadow = parsed.shadow;
    }
    style.truncate |= parsed.truncate;
    style.clip |= parsed.clip;
    style.disabled |= parsed.disabled;
    style.loading |= parsed.loading;
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
        if child.style.grow || child_main_fills_parent(child, style.direction) {
            grow_count += 1;
            continue;
        }
        fixed += child_outer_main_size(child, style.direction);
    }
    let gap_total = style.gap * children.len().saturating_sub(1) as f32;
    let grow_size = if grow_count > 0 {
        ((main_available - fixed - gap_total).max(0.0)) / grow_count as f32
    } else {
        0.0
    };
    let main_sum: f32 = children
        .iter()
        .map(|child| {
            if child.style.grow || child_main_fills_parent(child, style.direction) {
                grow_size
            } else {
                child_outer_main_size(child, style.direction)
            }
        })
        .sum();
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
    for child in children {
        let main = if child.style.grow || child_main_fills_parent(child, style.direction) {
            grow_size
        } else {
            child_outer_main_size(child, style.direction)
        };
        let child_rect = match style.direction {
            Axis::Horizontal => {
                let cross =
                    aligned_cross(content.y, content.h, child, Axis::Horizontal, style.align);
                let x = cursor + child.style.margin[3];
                let w = (main - child.style.margin[1] - child.style.margin[3]).max(0.0);
                UiRect::new(x, cross.0, w, cross.1)
            }
            Axis::Vertical => {
                let cross = aligned_cross(content.x, content.w, child, Axis::Vertical, style.align);
                let y = cursor + child.style.margin[0];
                let h = (main - child.style.margin[0] - child.style.margin[2]).max(0.0);
                UiRect::new(cross.0, y, cross.1, h)
            }
        };
        child.render_resolved_with_state(ui, child_rect, state);
        cursor += main + gap;
    }
}

fn resolve_stack_children(
    rect: UiRect,
    style: &UiStyle,
    children: &[UiNode],
    state: Option<&UiRuntimeState>,
) -> Vec<UiResolvedLayout> {
    stack_child_rects(rect, style, children)
        .into_iter()
        .zip(children)
        .map(|(rect, child)| child.resolve_layout_resolved(rect, state))
        .collect()
}

fn stack_child_rects(rect: UiRect, style: &UiStyle, children: &[UiNode]) -> Vec<UiRect> {
    if children.is_empty() {
        return Vec::new();
    }
    let content = content_rect(rect, style, 0.0);
    let main_available = match style.direction {
        Axis::Horizontal => content.w,
        Axis::Vertical => content.h,
    };
    let mut fixed = 0.0;
    let mut grow_count = 0usize;
    for child in children {
        if child.style.grow || child_main_fills_parent(child, style.direction) {
            grow_count += 1;
            continue;
        }
        fixed += child_outer_main_size(child, style.direction);
    }
    let gap_total = style.gap * children.len().saturating_sub(1) as f32;
    let grow_size = if grow_count > 0 {
        ((main_available - fixed - gap_total).max(0.0)) / grow_count as f32
    } else {
        0.0
    };
    let main_sum: f32 = children
        .iter()
        .map(|child| {
            if child.style.grow || child_main_fills_parent(child, style.direction) {
                grow_size
            } else {
                child_outer_main_size(child, style.direction)
            }
        })
        .sum();
    let gap_total = style.gap * children.len().saturating_sub(1) as f32;
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

    let mut rects = Vec::with_capacity(children.len());
    let mut cursor = match style.direction {
        Axis::Horizontal => content.x + start_offset,
        Axis::Vertical => content.y + start_offset,
    };
    for child in children {
        let main = if child.style.grow || child_main_fills_parent(child, style.direction) {
            grow_size
        } else {
            child_outer_main_size(child, style.direction)
        };
        let child_rect = match style.direction {
            Axis::Horizontal => {
                let cross =
                    aligned_cross(content.y, content.h, child, Axis::Horizontal, style.align);
                UiRect::new(
                    cursor + child.style.margin[3],
                    cross.0,
                    (main - child.style.margin[1] - child.style.margin[3]).max(0.0),
                    cross.1,
                )
            }
            Axis::Vertical => {
                let cross = aligned_cross(content.x, content.w, child, Axis::Vertical, style.align);
                UiRect::new(
                    cross.0,
                    cursor + child.style.margin[0],
                    cross.1,
                    (main - child.style.margin[0] - child.style.margin[2]).max(0.0),
                )
            }
        };
        rects.push(child_rect);
        cursor += main + gap;
    }
    rects
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
    let cross_margin_start = match parent_axis {
        Axis::Horizontal => child.style.margin[0],
        Axis::Vertical => child.style.margin[3],
    };
    let cross_margin_end = match parent_axis {
        Axis::Horizontal => child.style.margin[2],
        Axis::Vertical => child.style.margin[1],
    };
    let available = (content_size - cross_margin_start - cross_margin_end).max(0.0);
    let size = match explicit {
        Some(value) if value >= 0.0 => value.min(available),
        Some(_) => available,
        _ if matches!(align, AlignItems::Stretch) => available,
        _ => child_cross_size(child, parent_axis).min(available),
    }
    .max(0.0);
    let offset = match align {
        AlignItems::Start | AlignItems::Stretch => 0.0,
        AlignItems::Center => (available - size).max(0.0) * 0.5,
        AlignItems::End => (available - size).max(0.0),
    };
    (content_start + cross_margin_start + offset, size)
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
            row_h = row_h.max(child_outer_main_size(child, Axis::Vertical));
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
            let outer_h = child_outer_main_size(child, Axis::Vertical);
            let h = child_main_size(child, Axis::Vertical)
                .min(content.y + content.h - row_y)
                .max(0.0);
            let x = content.x + col as f32 * (track_w + gap);
            child.render_resolved_with_state(
                ui,
                UiRect::new(
                    x + child.style.margin[3],
                    row_y + child.style.margin[0],
                    (w - child.style.margin[1] - child.style.margin[3]).max(0.0),
                    h.min((outer_h - child.style.margin[0] - child.style.margin[2]).max(0.0)),
                ),
                state,
            );
            col += span;
        }
        row_y += row_h + gap;
    }
}

fn resolve_grid_children(
    rect: UiRect,
    style: &UiStyle,
    columns: u16,
    children: &[UiNode],
    state: Option<&UiRuntimeState>,
) -> Vec<UiResolvedLayout> {
    grid_child_rects(rect, style, columns, children)
        .into_iter()
        .zip(children)
        .map(|(rect, child)| child.resolve_layout_resolved(rect, state))
        .collect()
}

fn render_masonry_children(
    ui: &mut UiPainter<'_, '_>,
    rect: UiRect,
    style: &UiStyle,
    columns: u16,
    children: &[UiNode],
    state: Option<&UiRuntimeState>,
) {
    for (rect, child) in masonry_child_rects(rect, style, columns, children)
        .into_iter()
        .zip(children)
    {
        child.render_resolved_with_state(ui, rect, state);
    }
}

fn render_bento_children(
    ui: &mut UiPainter<'_, '_>,
    rect: UiRect,
    style: &UiStyle,
    columns: u16,
    children: &[UiNode],
    state: Option<&UiRuntimeState>,
) {
    for (rect, child) in bento_child_rects(rect, style, columns, children)
        .into_iter()
        .zip(children)
    {
        child.render_resolved_with_state(ui, rect, state);
    }
}

fn resolve_masonry_children(
    rect: UiRect,
    style: &UiStyle,
    columns: u16,
    children: &[UiNode],
    state: Option<&UiRuntimeState>,
) -> Vec<UiResolvedLayout> {
    masonry_child_rects(rect, style, columns, children)
        .into_iter()
        .zip(children)
        .map(|(rect, child)| child.resolve_layout_resolved(rect, state))
        .collect()
}

fn resolve_bento_children(
    rect: UiRect,
    style: &UiStyle,
    columns: u16,
    children: &[UiNode],
    state: Option<&UiRuntimeState>,
) -> Vec<UiResolvedLayout> {
    bento_child_rects(rect, style, columns, children)
        .into_iter()
        .zip(children)
        .map(|(rect, child)| child.resolve_layout_resolved(rect, state))
        .collect()
}

fn grid_child_rects(
    rect: UiRect,
    style: &UiStyle,
    columns: u16,
    children: &[UiNode],
) -> Vec<UiRect> {
    if children.is_empty() {
        return Vec::new();
    }
    let content = content_rect(rect, style, 0.0);
    let columns = columns.max(1) as usize;
    let gap = style.gap;
    let track_w = ((content.w - gap * columns.saturating_sub(1) as f32) / columns as f32).max(0.0);
    let mut rects = Vec::with_capacity(children.len());
    let mut row_y = content.y;
    let mut index = 0usize;

    while index < children.len() {
        let row_start = index;
        let mut row_col = 0usize;
        let mut row_h = 0.0_f32;
        while index < children.len() {
            let child = &children[index];
            let span = child.style.col_span.max(1).min(columns as u16) as usize;
            if row_col > 0 && row_col + span > columns {
                break;
            }
            row_h = row_h.max(child_outer_main_size(child, Axis::Vertical));
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
            let h = child_main_size(child, Axis::Vertical);
            let x = content.x + col as f32 * (track_w + gap);
            rects.push(UiRect::new(
                x + child.style.margin[3],
                row_y + child.style.margin[0],
                (w - child.style.margin[1] - child.style.margin[3]).max(0.0),
                h,
            ));
            col += span;
        }
        row_y += row_h + gap;
    }

    rects
}

fn masonry_child_rects(
    rect: UiRect,
    style: &UiStyle,
    columns: u16,
    children: &[UiNode],
) -> Vec<UiRect> {
    if children.is_empty() {
        return Vec::new();
    }
    let content = content_rect(rect, style, 0.0);
    let columns = columns.max(1) as usize;
    let gap = style.gap;
    let track_w = ((content.w - gap * columns.saturating_sub(1) as f32) / columns as f32).max(0.0);
    let mut column_heights = vec![content.y; columns];
    let mut rects = Vec::with_capacity(children.len());

    for child in children {
        let column = column_heights
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, _)| index)
            .unwrap_or(0);
        let x = content.x + column as f32 * (track_w + gap);
        let y = column_heights[column];
        let h = child_main_size(child, Axis::Vertical);
        rects.push(UiRect::new(
            x + child.style.margin[3],
            y + child.style.margin[0],
            (track_w - child.style.margin[1] - child.style.margin[3]).max(0.0),
            h,
        ));
        column_heights[column] += child_outer_main_size(child, Axis::Vertical) + gap;
    }

    rects
}

fn bento_child_rects(
    rect: UiRect,
    style: &UiStyle,
    columns: u16,
    children: &[UiNode],
) -> Vec<UiRect> {
    if children.is_empty() {
        return Vec::new();
    }
    let content = content_rect(rect, style, 0.0);
    let columns = columns.max(1) as usize;
    let gap = style.gap;
    let track_w = ((content.w - gap * columns.saturating_sub(1) as f32) / columns as f32).max(0.0);
    let row_h = 120.0;
    let mut occupancy: Vec<Vec<bool>> = Vec::new();
    let mut rects = Vec::with_capacity(children.len());

    for child in children {
        let col_span = child.style.col_span.max(1).min(columns as u16) as usize;
        let row_span = bento_row_span(child, row_h, gap);
        let (row, col) = reserve_bento_slot(&mut occupancy, columns, col_span, row_span);
        let x = content.x + col as f32 * (track_w + gap);
        let y = content.y + row as f32 * (row_h + gap);
        let w = track_w * col_span as f32 + gap * col_span.saturating_sub(1) as f32;
        let h = row_h * row_span as f32 + gap * row_span.saturating_sub(1) as f32;
        rects.push(UiRect::new(
            x + child.style.margin[3],
            y + child.style.margin[0],
            (w - child.style.margin[1] - child.style.margin[3]).max(0.0),
            (h - child.style.margin[0] - child.style.margin[2]).max(0.0),
        ));
    }

    rects
}

fn bento_row_span(child: &UiNode, row_h: f32, gap: f32) -> usize {
    if child.style.row_span > 1 {
        return child.style.row_span as usize;
    }
    let h = child_main_size(child, Axis::Vertical).max(1.0);
    ((h + gap) / (row_h + gap).max(1.0)).ceil().max(1.0) as usize
}

fn reserve_bento_slot(
    occupancy: &mut Vec<Vec<bool>>,
    columns: usize,
    col_span: usize,
    row_span: usize,
) -> (usize, usize) {
    let col_span = col_span.max(1).min(columns.max(1));
    let row_span = row_span.max(1);
    let mut row = 0usize;
    loop {
        ensure_bento_rows(occupancy, row + row_span, columns);
        for col in 0..=columns - col_span {
            if bento_slot_available(occupancy, row, col, col_span, row_span) {
                for y in row..row + row_span {
                    for x in col..col + col_span {
                        occupancy[y][x] = true;
                    }
                }
                return (row, col);
            }
        }
        row += 1;
    }
}

fn ensure_bento_rows(occupancy: &mut Vec<Vec<bool>>, rows: usize, columns: usize) {
    while occupancy.len() < rows {
        occupancy.push(vec![false; columns]);
    }
}

fn bento_slot_available(
    occupancy: &[Vec<bool>],
    row: usize,
    col: usize,
    col_span: usize,
    row_span: usize,
) -> bool {
    (row..row + row_span).all(|y| (col..col + col_span).all(|x| !occupancy[y][x]))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ScrollOffset {
    Fraction(f32),
    Pixels(f32),
}

fn render_scroll_children(
    ui: &mut UiPainter<'_, '_>,
    rect: UiRect,
    style: &UiStyle,
    offset: ScrollOffset,
    id: Option<u32>,
    children: &[UiNode],
    state: Option<&UiRuntimeState>,
) {
    if children.is_empty() {
        return;
    }
    let content = scroll_content_rect(rect, style);
    if content.w <= 0.0 || content.h <= 0.0 {
        return;
    }

    let total: f32 = children
        .iter()
        .map(|child| child_outer_main_size(child, Axis::Vertical))
        .sum::<f32>()
        + style.gap * children.len().saturating_sub(1) as f32;
    let scrollable = (total - content.h).max(0.0);
    let scroll_y = match offset {
        ScrollOffset::Fraction(offset) => scrollable * offset.clamp(0.0, 1.0),
        ScrollOffset::Pixels(offset) => offset.clamp(0.0, scrollable),
    };
    let offset_fraction = if scrollable > 0.0 {
        (scroll_y / scrollable).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if scrollable > 0.0
        && let Some(id) = id
    {
        ui.hit(
            HitKind::ScrollArea,
            id,
            content.x,
            content.y,
            content.w,
            content.h,
        );
    }
    let mut cursor = content.y - scroll_y;

    let clipped = ui
        .scene
        .push_clip(GpuClip::new(content.x, content.y, content.w, content.h));
    for child in children {
        let outer_h = child_outer_main_size(child, Axis::Vertical);
        let h = child_main_size(child, Axis::Vertical);
        let bottom = cursor + outer_h;
        if bottom >= content.y && cursor <= content.y + content.h && clipped {
            child.render_resolved_with_state(
                ui,
                UiRect::new(
                    content.x + child.style.margin[3],
                    cursor + child.style.margin[0],
                    (content.w - child.style.margin[1] - child.style.margin[3]).max(0.0),
                    h,
                ),
                state,
            );
        }
        cursor += outer_h + style.gap;
    }
    if clipped {
        ui.scene.pop_clip();
    }

    if total > content.h {
        let scrollbar = spacing::scrollbar_track_rect(rect, content);
        if let Some(id) = id {
            let hit = spacing::scrollbar_hit_rect(scrollbar);
            ui.hit(HitKind::Scrollbar, id, hit.x, hit.y, hit.w, hit.h);
        }
        ui.scrollbar(scrollbar, content.h / total, offset_fraction);
    }
}

fn resolve_scroll_children(
    rect: UiRect,
    style: &UiStyle,
    offset: ScrollOffset,
    children: &[UiNode],
    state: Option<&UiRuntimeState>,
) -> Vec<UiResolvedLayout> {
    scroll_child_rects(rect, style, offset, children)
        .into_iter()
        .zip(children)
        .map(|(rect, child)| child.resolve_layout_resolved(rect, state))
        .collect()
}

fn scroll_child_rects(
    rect: UiRect,
    style: &UiStyle,
    offset: ScrollOffset,
    children: &[UiNode],
) -> Vec<UiRect> {
    if children.is_empty() {
        return Vec::new();
    }
    let content = scroll_content_rect(rect, style);
    if content.w <= 0.0 || content.h <= 0.0 {
        return Vec::new();
    }

    let total: f32 = children
        .iter()
        .map(|child| child_outer_main_size(child, Axis::Vertical))
        .sum::<f32>()
        + style.gap * children.len().saturating_sub(1) as f32;
    let scrollable = (total - content.h).max(0.0);
    let scroll_y = match offset {
        ScrollOffset::Fraction(offset) => scrollable * offset.clamp(0.0, 1.0),
        ScrollOffset::Pixels(offset) => offset.clamp(0.0, scrollable),
    };
    let mut rects = Vec::with_capacity(children.len());
    let mut cursor = content.y - scroll_y;
    for child in children {
        let h = child_main_size(child, Axis::Vertical);
        rects.push(UiRect::new(
            content.x + child.style.margin[3],
            cursor + child.style.margin[0],
            (content.w - child.style.margin[1] - child.style.margin[3]).max(0.0),
            h,
        ));
        cursor += child_outer_main_size(child, Axis::Vertical) + style.gap;
    }
    rects
}

fn scroll_content_rect(rect: UiRect, style: &UiStyle) -> UiRect {
    spacing::scroll_content_rect(rect, style.padding)
}

fn content_rect(rect: UiRect, style: &UiStyle, trailing_reserved_w: f32) -> UiRect {
    UiRect {
        x: rect.x + style.padding[3],
        y: rect.y + style.padding[0],
        w: (rect.w - style.padding[1] - style.padding[3] - trailing_reserved_w).max(0.0),
        h: (rect.h - style.padding[0] - style.padding[2]).max(0.0),
    }
}

fn collect_layout_issues(layout: &UiResolvedLayout, path: &str, issues: &mut Vec<UiLayoutIssue>) {
    for (index, child) in layout.children.iter().enumerate() {
        let child_path = format!("{path}/{index}:{}", child.kind);
        if child.kind != "spacer" && rect_has_invalid_size(child.rect) {
            issues.push(UiLayoutIssue {
                path: child_path.clone(),
                kind: child.kind,
                rect: child.rect,
                parent: layout.rect,
                message: "invalid or zero-size layout rect",
            });
        }
        if layout.kind != "scroll_area" && rect_exceeds(child.rect, layout.rect) {
            issues.push(UiLayoutIssue {
                path: child_path.clone(),
                kind: child.kind,
                rect: child.rect,
                parent: layout.rect,
                message: "child exceeds parent bounds",
            });
        }
        for (other_index, other) in layout.children.iter().enumerate().skip(index + 1) {
            if rects_overlap(child.rect, other.rect) {
                issues.push(UiLayoutIssue {
                    path: format!("{child_path} overlaps sibling {other_index}:{}", other.kind),
                    kind: child.kind,
                    rect: child.rect,
                    parent: layout.rect,
                    message: "sibling layout rects overlap",
                });
            }
        }
        collect_layout_issues(child, &child_path, issues);
    }
}

fn collect_composition_issues(
    node: &UiNode,
    path: &str,
    framed_ancestor: Option<&'static str>,
    issues: &mut Vec<UiCompositionIssue>,
) {
    let kind = node.kind_name();
    if kind == "card" {
        if let Some(parent_kind) = framed_ancestor {
            issues.push(UiCompositionIssue {
                path: path.into(),
                parent_kind,
                child_kind: kind,
                message: "card components must not be nested inside another card",
            });
        }
    }

    let next_framed = framed_ancestor.or(if kind == "card" { Some(kind) } else { None });
    for (index, child) in node.children.iter().enumerate() {
        let child_path = format!("{path}/{index}:{}", child.kind_name());
        collect_composition_issues(child, &child_path, next_framed, issues);
    }
}

fn collect_layout_trace(
    layout: &UiResolvedLayout,
    path: &str,
    trace: &mut Vec<UiLayoutTraceEntry>,
) {
    trace.push(UiLayoutTraceEntry {
        path: path.into(),
        kind: layout.kind,
        rect: layout.rect,
        child_count: layout.children.len(),
    });
    for (index, child) in layout.children.iter().enumerate() {
        let child_path = format!("{path}/{index}:{}", child.kind);
        collect_layout_trace(child, &child_path, trace);
    }
}

fn rect_has_invalid_size(rect: UiRect) -> bool {
    rect.w <= 0.5 || rect.h <= 0.5 || !rect.w.is_finite() || !rect.h.is_finite()
}

fn rect_exceeds(rect: UiRect, parent: UiRect) -> bool {
    const EPSILON: f32 = 0.5;
    rect.x < parent.x - EPSILON
        || rect.y < parent.y - EPSILON
        || rect.x + rect.w > parent.x + parent.w + EPSILON
        || rect.y + rect.h > parent.y + parent.h + EPSILON
}

fn rects_overlap(a: UiRect, b: UiRect) -> bool {
    const EPSILON: f32 = 0.5;
    a.x < b.x + b.w - EPSILON
        && a.x + a.w > b.x + EPSILON
        && a.y < b.y + b.h - EPSILON
        && a.y + a.h > b.y + EPSILON
}

fn child_main_size(child: &UiNode, axis: Axis) -> f32 {
    match axis {
        Axis::Horizontal => match child.style.width {
            Some(value) if value >= 0.0 => value,
            Some(_) | None => intrinsic_width(child),
        },
        Axis::Vertical => match child.style.height {
            Some(value) if value >= 0.0 => value,
            Some(_) | None => intrinsic_height(child),
        },
    }
}

fn child_outer_main_size(child: &UiNode, axis: Axis) -> f32 {
    let margin = match axis {
        Axis::Horizontal => child.style.margin[1] + child.style.margin[3],
        Axis::Vertical => child.style.margin[0] + child.style.margin[2],
    };
    child_main_size(child, axis) + margin
}

fn child_main_fills_parent(child: &UiNode, axis: Axis) -> bool {
    match axis {
        Axis::Horizontal => child.style.width.is_some_and(|value| value < 0.0),
        Axis::Vertical => child.style.height.is_some_and(|value| value < 0.0),
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

fn child_outer_cross_size(child: &UiNode, parent_axis: Axis) -> f32 {
    let margin = match parent_axis {
        Axis::Horizontal => child.style.margin[0] + child.style.margin[2],
        Axis::Vertical => child.style.margin[1] + child.style.margin[3],
    };
    child_cross_size(child, parent_axis) + margin
}

fn padding_horizontal(style: &UiStyle) -> f32 {
    style.padding[1] + style.padding[3]
}

fn padding_vertical(style: &UiStyle) -> f32 {
    style.padding[0] + style.padding[2]
}

fn intrinsic_row_width(node: &UiNode) -> f32 {
    let children = node.children.len();
    padding_horizontal(&node.style)
        + node
            .children
            .iter()
            .map(|child| child_outer_main_size(child, Axis::Horizontal))
            .sum::<f32>()
        + node.style.gap * children.saturating_sub(1) as f32
}

fn intrinsic_column_width(node: &UiNode) -> f32 {
    padding_horizontal(&node.style)
        + node
            .children
            .iter()
            .map(|child| child_outer_cross_size(child, Axis::Vertical))
            .fold(0.0_f32, f32::max)
}

fn intrinsic_grid_width(node: &UiNode, columns: u16) -> f32 {
    let columns = columns.max(1) as f32;
    let widest = node
        .children
        .iter()
        .map(|child| child_outer_main_size(child, Axis::Horizontal))
        .fold(120.0_f32, f32::max);
    padding_horizontal(&node.style) + widest * columns + node.style.gap * (columns - 1.0)
}

fn intrinsic_row_height(node: &UiNode) -> f32 {
    padding_vertical(&node.style)
        + node
            .children
            .iter()
            .map(|child| child_outer_cross_size(child, Axis::Horizontal))
            .fold(0.0_f32, f32::max)
}

fn intrinsic_column_height(node: &UiNode) -> f32 {
    let children = node.children.len();
    padding_vertical(&node.style)
        + node
            .children
            .iter()
            .map(|child| child_outer_main_size(child, Axis::Vertical))
            .sum::<f32>()
        + node.style.gap * children.saturating_sub(1) as f32
}

fn intrinsic_grid_height(node: &UiNode, columns: u16) -> f32 {
    let columns = columns.max(1) as usize;
    let mut index = 0usize;
    let mut height = padding_vertical(&node.style);
    let mut row_count = 0usize;

    while index < node.children.len() {
        let mut row_col = 0usize;
        let mut row_h = 0.0_f32;
        while index < node.children.len() {
            let child = &node.children[index];
            let span = child.style.col_span.max(1).min(columns as u16) as usize;
            if row_col > 0 && row_col + span > columns {
                break;
            }
            row_h = row_h.max(child_outer_main_size(child, Axis::Vertical));
            row_col += span;
            index += 1;
            if row_col >= columns {
                break;
            }
        }
        if row_count > 0 {
            height += node.style.gap;
        }
        height += row_h;
        row_count += 1;
    }

    height
}

fn intrinsic_masonry_height(node: &UiNode, columns: u16) -> f32 {
    let columns = columns.max(1) as usize;
    let mut column_heights = vec![padding_vertical(&node.style); columns];
    for child in &node.children {
        let column = column_heights
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, _)| index)
            .unwrap_or(0);
        column_heights[column] += child_outer_main_size(child, Axis::Vertical) + node.style.gap;
    }
    column_heights.into_iter().fold(0.0_f32, f32::max)
}

fn intrinsic_bento_height(node: &UiNode, columns: u16) -> f32 {
    let columns = columns.max(1) as usize;
    let row_h = 120.0;
    let mut occupancy: Vec<Vec<bool>> = Vec::new();
    for child in &node.children {
        let col_span = child.style.col_span.max(1).min(columns as u16) as usize;
        let row_span = bento_row_span(child, row_h, node.style.gap);
        reserve_bento_slot(&mut occupancy, columns, col_span, row_span);
    }
    padding_vertical(&node.style)
        + occupancy.len() as f32 * row_h
        + occupancy.len().saturating_sub(1) as f32 * node.style.gap
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
        UiNodeKind::Row => intrinsic_row_width(child).max(1.0),
        UiNodeKind::Column | UiNodeKind::Card | UiNodeKind::ScrollArea { .. } => {
            intrinsic_column_width(child).max(1.0)
        }
        UiNodeKind::Grid { columns }
        | UiNodeKind::Masonry { columns }
        | UiNodeKind::BentoGrid { columns } => intrinsic_grid_width(child, *columns).max(1.0),
    }
}

fn intrinsic_height(child: &UiNode) -> f32 {
    match &child.kind {
        UiNodeKind::Text(value) => {
            let line_count = value.lines().count().max(1) as f32;
            (line_count * 20.0).clamp(20.0, 160.0)
        }
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
            child.style.height.unwrap_or(spacing::PACKAGE_CARD_H)
        }
        UiNodeKind::RoutePath { .. } => child.style.height.unwrap_or(86.0),
        UiNodeKind::ContactCard { .. } | UiNodeKind::AttachmentPreview { .. } => 64.0,
        UiNodeKind::ThreadRow { .. }
        | UiNodeKind::CapabilityGrantRow { .. }
        | UiNodeKind::ProofEventRow { .. }
        | UiNodeKind::ReceiptRow { .. }
        | UiNodeKind::AppLauncherItem { .. } => spacing::ROW_H,
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
        UiNodeKind::Row => intrinsic_row_height(child).max(1.0),
        UiNodeKind::Column | UiNodeKind::Card => intrinsic_column_height(child).max(1.0),
        UiNodeKind::Grid { columns } => intrinsic_grid_height(child, *columns).max(1.0),
        UiNodeKind::Masonry { columns } => intrinsic_masonry_height(child, *columns).max(1.0),
        UiNodeKind::BentoGrid { columns } => intrinsic_bento_height(child, *columns).max(1.0),
        UiNodeKind::ScrollArea { .. } => child.style.height.unwrap_or(180.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_intrinsic_height_tracks_child_rows() {
        let grid = UiNode::grid("grid grid-cols-2 gap-4 p-2", 2)
            .child(UiNode::text("one").class("h-10"))
            .child(UiNode::text("two").class("h-12"))
            .child(UiNode::text("three").class("h-8"));

        assert_eq!(intrinsic_height(&grid), 16.0 + 48.0 + 16.0 + 32.0);
    }

    #[test]
    fn masonry_layout_packs_into_shortest_column() {
        let layout = UiNode::masonry("masonry grid-cols-3 gap-4 p-0", 3)
            .child(UiNode::spacer("h-30"))
            .child(UiNode::spacer("h-20"))
            .child(UiNode::spacer("h-15"))
            .child(UiNode::spacer("h-17.5"))
            .resolve_layout(UiRect::new(0.0, 0.0, 640.0, 480.0));

        assert_eq!(layout.kind, "masonry");
        assert_eq!(layout.children.len(), 4);
        assert_eq!(
            layout.children[0].rect,
            UiRect::new(0.0, 0.0, 202.66667, 120.0)
        );
        assert_eq!(
            layout.children[1].rect,
            UiRect::new(218.66667, 0.0, 202.66667, 80.0)
        );
        assert_eq!(
            layout.children[2].rect,
            UiRect::new(437.33334, 0.0, 202.66667, 60.0)
        );
        assert_eq!(
            layout.children[3].rect,
            UiRect::new(437.33334, 76.0, 202.66667, 70.0)
        );
    }

    #[test]
    fn bento_grid_reserves_spanned_cells_without_overlap() {
        let node = UiNode::bento_grid("bento grid-cols-4 gap-4 p-0", 4)
            .child(UiNode::spacer("").span(2).row_span(3))
            .child(UiNode::spacer("h-30"))
            .child(UiNode::spacer("h-30"))
            .child(UiNode::spacer("h-30").span(2));
        let layout = node.resolve_layout(UiRect::new(0.0, 0.0, 640.0, 720.0));

        assert_eq!(layout.kind, "bento_grid");
        assert_eq!(layout.children.len(), 4);
        assert_eq!(layout.children[0].rect, UiRect::new(0.0, 0.0, 312.0, 392.0));
        assert_eq!(
            layout.children[1].rect,
            UiRect::new(328.0, 0.0, 148.0, 120.0)
        );
        assert_eq!(
            layout.children[2].rect,
            UiRect::new(492.0, 0.0, 148.0, 120.0)
        );
        assert_eq!(
            layout.children[3].rect,
            UiRect::new(328.0, 136.0, 312.0, 120.0)
        );
        assert_eq!(
            node.layout_issues(UiRect::new(0.0, 0.0, 640.0, 720.0)),
            Vec::new()
        );
    }

    #[test]
    fn responsive_masonry_and_bento_read_grid_column_classes() {
        let masonry = UiNode::masonry_auto_for_width(
            "masonry grid-cols-1 md:grid-cols-2 xl:grid-cols-5 gap-4",
            1400.0,
        );
        let bento = UiNode::bento_grid_auto_for_width(
            "bento grid-cols-1 md:grid-cols-3 xl:grid-cols-6 gap-4",
            1400.0,
        );
        assert!(matches!(masonry.kind, UiNodeKind::Masonry { columns: 5 }));
        assert!(matches!(bento.kind, UiNodeKind::BentoGrid { columns: 6 }));
    }

    #[test]
    fn column_intrinsic_height_includes_padding_and_gaps() {
        let column = UiNode::column("gap-3 py-2")
            .child(UiNode::text("one").class("h-5"))
            .child(UiNode::text("two").class("h-7"));

        assert_eq!(intrinsic_height(&column), 16.0 + 20.0 + 12.0 + 28.0);
    }

    #[test]
    fn stack_layout_applies_child_margins() {
        let column = UiNode::column("gap-2 p-1")
            .child(UiNode::text("one").class("h-5 mt-1 mb-2 mx-3"))
            .child(UiNode::text("two").class("h-5"));

        let layout = column.resolve_layout(UiRect::new(0.0, 0.0, 200.0, 120.0));

        assert_eq!(layout.children[0].rect, UiRect::new(16.0, 8.0, 168.0, 20.0));
        assert_eq!(layout.children[1].rect, UiRect::new(4.0, 44.0, 192.0, 20.0));
    }

    #[test]
    fn scroll_area_px_preserves_pixel_offset() {
        let node = UiNode::scroll_area_px("h-40", 96.0);
        let UiNodeKind::ScrollArea {
            offset,
            offset_px,
            id,
        } = node.kind
        else {
            panic!("expected scroll area");
        };

        assert_eq!(offset, 0.0);
        assert_eq!(offset_px, Some(96.0));
        assert_eq!(id, None);
    }

    #[test]
    fn scroll_area_layout_and_render_share_content_clip_geometry() {
        let bounds = UiRect::new(0.0, 0.0, 220.0, 96.0);
        let node = UiNode::scroll_area_px("p-2 gap-2 h-24", 0.0)
            .scroll_id(77)
            .child(UiNode::text("one").class("h-16"))
            .child(UiNode::text("two").class("h-16"));
        let expected_content = spacing::scroll_content_rect(bounds, node.style.padding);

        let layout = node.resolve_layout(bounds);
        assert_eq!(layout.children[0].rect.x, expected_content.x);
        assert_eq!(layout.children[0].rect.y, expected_content.y);
        assert_eq!(layout.children[0].rect.w, expected_content.w);

        let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut ui = UiPainter::new(&mut scene);
        node.render(&mut ui, bounds);

        let hit = scene
            .hits()
            .iter()
            .find(|hit| hit.kind == HitKind::Scrollbar && hit.id == 77)
            .expect("scrollbar hit");
        let scroll_hit = scene
            .hits()
            .iter()
            .find(|hit| hit.kind == HitKind::ScrollArea && hit.id == 77)
            .expect("scroll area hit");
        assert_eq!(
            UiRect::new(scroll_hit.x, scroll_hit.y, scroll_hit.w, scroll_hit.h),
            expected_content
        );
        let track = spacing::scrollbar_track_rect(bounds, expected_content);
        let expected_hit = spacing::scrollbar_hit_rect(track);
        assert_eq!(UiRect::new(hit.x, hit.y, hit.w, hit.h), expected_hit);
    }

    #[test]
    fn class_appends_without_erasing_existing_layout() {
        let node = UiNode::grid_auto_for_width(
            "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 h-36",
            1180.0,
        )
        .class("w-full overflow-hidden");

        assert_eq!(node.style.grid_cols, Some(4));
        assert_eq!(node.style.gap, 16.0);
        assert_eq!(node.style.height, Some(144.0));
        assert_eq!(node.style.width, Some(-1.0));
        assert!(node.style.clip);
    }

    #[test]
    fn class_overlay_keeps_padding_when_only_axis_is_changed() {
        let node = UiNode::card("p-4 gap-2").class("px-2");

        assert_eq!(node.style.padding, [16.0, 8.0, 16.0, 8.0]);
        assert_eq!(node.style.gap, 8.0);
    }

    #[test]
    fn layout_issues_report_overflowing_children() {
        let node = UiNode::column("h-8").child(UiNode::text("too tall").class("h-12"));

        let issues = node.layout_issues(UiRect::new(0.0, 0.0, 200.0, 32.0));

        assert!(issues.iter().any(|issue| {
            issue.kind == "text" && issue.message == "child exceeds parent bounds"
        }));
    }

    #[test]
    fn layout_issues_allow_scroll_content_to_extend() {
        let node = UiNode::scroll_area_px("h-8", 0.0)
            .child(UiNode::text("one").class("h-12"))
            .child(UiNode::text("two").class("h-12"));

        let issues = node.layout_issues(UiRect::new(0.0, 0.0, 200.0, 32.0));

        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn layout_issues_allow_collapsed_spacers() {
        let node = UiNode::column("h-8")
            .child(UiNode::text("fixed").class("h-8"))
            .child(UiNode::spacer("flex-1"));

        let issues = node.layout_issues(UiRect::new(0.0, 0.0, 200.0, 32.0));

        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn long_operational_text_fixtures_do_not_overlap_layout() {
        let long_hash = "manifest blake3:9bf4076a91d0cc2e1f847cc8f8ce11afadmission";
        let long_policy = "policy:publisher:mail:7f2c35aa01b9d0f7:capability:storage-read";
        let long_route = "route:admission->relay:private-devices->storage:vps-cache->receipt";
        let node = UiNode::column("gap-3 p-3 h-184")
            .child(
                UiNode::section(
                    "Verified package and admission proof",
                    "Long hashes, policy ids, routes, budgets, and identity labels must truncate without layout drift.",
                )
                .class("h-16"),
            )
            .child(
                UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-3 h-84", 720.0)
                    .child(
                        UiNode::package_card(
                            "EdgeRun Mail Publisher Preview With Very Long Release Name",
                            long_policy,
                            long_hash,
                            51,
                        )
                        .class("h-24"),
                    )
                    .child(
                        UiNode::column("gap-2 h-56")
                            .child(
                                UiNode::proof_event_row(
                                    "Admission route commitment with receipt challenge window",
                                    long_hash,
                                    "accepted",
                                    52,
                                )
                                .class("h-16"),
                            )
                            .child(
                                UiNode::list_row(
                                    "identity:edgerun:3f0d2a8b9c114e71",
                                    long_policy,
                                    53,
                                )
                                .class("h-16"),
                            )
                            .child(UiNode::route_path("Route", &[long_route]).class("h-20")),
                    ),
            )
            .child(
                UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-3 gap-3 h-72", 720.0)
                    .child(
                        UiNode::metric_card("Budget", "184 units reserved for verified retrieval")
                            .detail("deterministic settlement amount")
                            .class("h-20"),
                    )
                    .child(
                        UiNode::receipt_row(
                            "receipt blake3:4a0b9e2d1a83c7f6admission",
                            "184 units",
                            "payable",
                            54,
                        )
                        .class("h-20"),
                    )
                    .child(
                        UiNode::capability_grant_row(
                            "EdgeRun Mail Publisher Preview",
                            "storage.read.network-package-cache",
                            "policy-bound",
                            55,
                        )
                        .class("h-20"),
                    ),
            );

        let issues = node.layout_issues(UiRect::new(0.0, 0.0, 720.0, 736.0));

        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn empty_loading_error_and_dense_state_fixtures_do_not_overlap_layout() {
        let node = UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-3 p-3 h-156", 760.0)
            .child(
                UiNode::empty_state(
                    "No verified receipts yet",
                    "When admitted work produces receipt proofs they appear here with policy and challenge refs.",
                    UiIcon::File,
                )
                .class("h-32"),
            )
            .child(
                UiNode::column("gap-2 h-36")
                    .child(UiNode::skeleton().class("h-10"))
                    .child(UiNode::skeleton().class("h-10"))
                    .child(UiNode::skeleton().class("h-10")),
            )
            .child(
                UiNode::toast(
                    "Package verification failed: manifest hash did not match publisher policy commitment",
                    UiIcon::Warning,
                    palette::DANGER,
                )
                .class("h-20"),
            )
            .child(
                UiNode::column("gap-2 h-48")
                    .child(
                        UiNode::list_row(
                            "pkgimg-store-admission-work-request",
                            "accepted route relay:private-devices budget 184 units",
                            61,
                        )
                        .class("h-14"),
                    )
                    .child(
                        UiNode::list_row(
                            "sync-mailbox-import-source",
                            "pending storage:vps-cache policy:user-owned-admission",
                            62,
                        )
                        .class("h-14"),
                    )
                    .child(
                        UiNode::list_row(
                            "receipt-challenge-review",
                            "committed receipt blake3:a8e392d911",
                            63,
                        )
                        .class("h-14"),
                    ),
            );

        let issues = node.layout_issues(UiRect::new(0.0, 0.0, 760.0, 624.0));

        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn composition_issues_report_nested_cards() {
        let node = UiNode::card("p-4")
            .child(UiNode::column("gap-2").child(UiNode::card("p-2").child(UiNode::text("bad"))));

        let issues = node.composition_issues();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].path, "$/0:column/0:card");
        assert_eq!(issues[0].parent_kind, "card");
        assert_eq!(issues[0].child_kind, "card");
        assert_eq!(
            issues[0].message,
            "card components must not be nested inside another card"
        );
    }

    #[test]
    fn composition_issues_allow_unframed_sections_inside_cards() {
        let node = UiNode::card("p-4").child(
            UiNode::column("bg-bg rounded-md p-3 gap-2")
                .child(header("Retirement").detail("$420,000"))
                .child(UiNode::progress_bar(0.65, palette::GREEN)),
        );

        assert!(node.composition_issues().is_empty());
    }

    #[test]
    fn overlay_nodes_use_canonical_stacking_contract_and_debug_labels() {
        assert_eq!(
            UI_OVERLAY_STACKING_ORDER,
            &[
                UiOverlayLayer::Popover,
                UiOverlayLayer::Tooltip,
                UiOverlayLayer::Toast,
                UiOverlayLayer::Modal,
            ]
        );
        assert!(UiOverlayLayer::Tooltip < UiOverlayLayer::Toast);
        assert!(UiOverlayLayer::Toast < UiOverlayLayer::Modal);

        let tooltip = UiNode::tooltip("Verify package");
        let toast = UiNode::toast("Route admitted", UiIcon::Check, palette::GREEN);
        let dialog = UiNode::dialog(
            "Run network app",
            "Verify package bytes first.",
            UiIcon::App,
        );

        assert_eq!(tooltip.overlay_layer(), Some(UiOverlayLayer::Tooltip));
        assert_eq!(toast.overlay_layer(), Some(UiOverlayLayer::Toast));
        assert_eq!(dialog.overlay_layer(), Some(UiOverlayLayer::Modal));

        let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        {
            let mut ui = UiPainter::new(&mut scene);
            tooltip.render(&mut ui, UiRect::new(0.0, 0.0, 180.0, 32.0));
            toast.render(&mut ui, UiRect::new(0.0, 40.0, 220.0, 42.0));
            dialog.render(&mut ui, UiRect::new(0.0, 92.0, 320.0, 180.0));
        }

        let labels: Vec<_> = scene
            .debug_batches()
            .iter()
            .map(|batch| batch.name.as_str())
            .collect();
        assert!(labels.contains(&"overlay:tooltip"));
        assert!(labels.contains(&"overlay:toast"));
        assert!(labels.contains(&"overlay:modal"));
    }

    #[test]
    fn layout_trace_flattens_resolved_geometry_with_paths() {
        let node = UiNode::column("gap-2 p-1")
            .child(UiNode::text("alpha").class("h-5"))
            .child(UiNode::button("Run", 7, ButtonStyle::Primary));

        let trace = node.layout_trace(UiRect::new(0.0, 0.0, 200.0, 120.0));

        assert_eq!(trace[0].path, "$");
        assert_eq!(trace[0].kind, "column");
        assert_eq!(trace[0].child_count, 2);
        assert_eq!(trace[1].path, "$/0:text");
        assert_eq!(trace[2].path, "$/1:button");
        assert_eq!(trace[1].rect, UiRect::new(4.0, 4.0, 192.0, 20.0));
    }

    #[test]
    fn render_with_state_paints_hover_feedback_for_owned_hit() {
        let node = UiNode::button("Run", 42, ButtonStyle::Primary);
        let bounds = UiRect::new(0.0, 0.0, 120.0, 40.0);
        let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut ui = UiPainter::new(&mut scene);
        node.render(&mut ui, bounds);

        let mut state = UiRuntimeState::default();
        let action = state.handle_event(&scene, UiEvent::PointerMove { x: 12.0, y: 12.0 });
        assert!(matches!(action, UiAction::Hovered(Some(hit)) if hit.id == 42));

        let mut hovered_scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut hovered_ui = UiPainter::new(&mut hovered_scene);
        node.render_with_state(&mut hovered_ui, bounds, Some(&state));

        assert!(hovered_scene.rects().iter().any(|rect| {
            rect.mode == RectMode::Fill
                && rect.color == UiResolvedTheme::default().colors.row.with_alpha(0.24)
        }));
    }

    #[test]
    fn gradient_and_shadow_classes_render_canonical_rect_primitives() {
        let node = UiNode::card("bg-panel-gradient rounded-xl shadow-sm p-2");
        let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut ui = UiPainter::new(&mut scene);

        node.render(&mut ui, UiRect::new(0.0, 0.0, 120.0, 80.0));

        assert!(
            scene
                .rects()
                .iter()
                .any(|rect| { rect.mode == RectMode::Shadow && rect.shadow > 0.0 })
        );
        assert!(scene.rects().iter().any(|rect| {
            rect.mode == RectMode::LinearGradient
                && rect.color == UiResolvedTheme::default().colors.panel
                && rect.color2 == UiResolvedTheme::default().colors.row
        }));
    }

    #[test]
    fn icon_button_tooltip_convention_sets_accessible_label_and_hover_tooltip() {
        let node =
            UiNode::icon_button_with_tooltip(UiIcon::Trash, 42, "Remove cache").class("size-8");
        let bounds = UiRect::new(20.0, 48.0, 80.0, 80.0);

        let tree = super::super::accessibility_tree(&node);
        assert_eq!(tree.label, "Remove cache");

        let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut ui = UiPainter::new(&mut scene);
        node.render(&mut ui, bounds);

        let mut state = UiRuntimeState::default();
        let action = state.handle_event(&scene, UiEvent::PointerMove { x: 36.0, y: 64.0 });
        assert!(matches!(action, UiAction::Hovered(Some(hit)) if hit.id == 42));

        let mut hovered_scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut hovered_ui = UiPainter::new(&mut hovered_scene);
        node.render_with_state(&mut hovered_ui, bounds, Some(&state));

        let colors = UiResolvedTheme::default().colors;
        assert!(hovered_scene.rects().iter().any(|rect| {
            rect.mode == RectMode::Fill
                && rect.color == colors.topbar
                && rect.w >= 44.0
                && rect.h == 30.0
        }));
    }

    #[test]
    fn reorderable_nodes_emit_drag_sources_and_drop_targets() {
        let node = UiNode::column("gap-2 p-0")
            .child(UiNode::list_row("Alpha", "first", 11).reorderable(5, 11, 0))
            .child(UiNode::list_row("Beta", "second", 12).reorderable(5, 12, 1));
        let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut ui = UiPainter::new(&mut scene);

        node.render(&mut ui, UiRect::new(0.0, 0.0, 240.0, 160.0));

        assert_eq!(scene.drag_sources().len(), 2);
        assert_eq!(scene.drop_targets().len(), 2);
        assert_eq!(scene.drag_sources()[0].item_id, 11);
        assert_eq!(scene.drag_sources()[1].index, 1);
        assert_eq!(scene.drop_targets()[1].scope_id, 5);
        assert!(scene.drag_sources()[1].y > scene.drag_sources()[0].y);
    }

    #[test]
    fn disabled_reorderable_nodes_do_not_emit_drag_metadata() {
        let node = UiNode::list_row("Disabled", "locked", 21)
            .reorderable(5, 21, 0)
            .disabled(true);
        let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut ui = UiPainter::new(&mut scene);

        node.render(&mut ui, UiRect::new(0.0, 0.0, 240.0, 80.0));

        assert!(scene.drag_sources().is_empty());
        assert!(scene.drop_targets().is_empty());
    }

    #[test]
    fn transition_nodes_apply_runtime_values_to_emitted_scene_commands() {
        let node = UiNode::button("Run", 42, ButtonStyle::Primary)
            .fade(900, 0.0, 1.0, 100)
            .slide_y(901, -20.0, 0.0, 100);
        let bounds = UiRect::new(0.0, 0.0, 120.0, 40.0);

        let mut bootstrap_scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut bootstrap_ui = UiPainter::new(&mut bootstrap_scene);
        node.render(&mut bootstrap_ui, bounds);

        let mut state = UiRuntimeState::default();
        state.sync_transitions(&bootstrap_scene);

        let mut start_scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut start_ui = UiPainter::new(&mut start_scene);
        node.render_with_state(&mut start_ui, bounds, Some(&state));
        let hit = start_scene
            .hits()
            .iter()
            .find(|hit| hit.id == 42)
            .copied()
            .expect("button hit");
        assert_eq!(hit.y, -20.0);
        assert!(start_scene.rects().iter().all(|rect| rect.color.a == 0.0));

        assert!(state.advance_transitions(100));
        let mut end_scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut end_ui = UiPainter::new(&mut end_scene);
        node.render_with_state(&mut end_ui, bounds, Some(&state));
        let hit = end_scene
            .hits()
            .iter()
            .find(|hit| hit.id == 42)
            .copied()
            .expect("button hit");
        assert_eq!(hit.y, 0.0);
        assert!(end_scene.rects().iter().any(|rect| rect.color.a > 0.0));
    }

    #[test]
    fn truncate_text_renders_single_line_in_compact_layouts() {
        let node = UiNode::text("first\nsecond").class("truncate h-12");
        let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut ui = UiPainter::new(&mut scene);

        node.render(&mut ui, UiRect::new(0.0, 0.0, 160.0, 48.0));

        assert!(
            scene.rects().iter().all(|rect| rect.y < 20.0),
            "{:?}",
            scene.rects()
        );
    }
}
