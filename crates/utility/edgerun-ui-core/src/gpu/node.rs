use std::string::String;
use std::vec::Vec;

use super::*;

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
    pub(super) kind: UiNodeKind,
    pub(super) style: UiStyle,
    pub(super) children: Vec<UiNode>,
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
        if condition {
            self.child(child)
        } else {
            self
        }
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
                ui.bounded_label(
                    rect.x,
                    rect.y,
                    rect.w,
                    value,
                    2.0,
                    self.style.text.resolve(ui.theme()),
                );
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
                if let Some(bg) = self.style.bg.map(|color| color.resolve(ui.theme())) {
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
                                ui.theme().colors.border,
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
                ui.theme().radius.control,
                ui.theme().colors.accent.with_alpha(0.86),
            );
        }
        if self.style.disabled || self.style.loading {
            ui.scene.truncate_hits(hit_start);
            ui.fill_rect(
                rect,
                self.style.radius.max(ui.theme().radius.card),
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
