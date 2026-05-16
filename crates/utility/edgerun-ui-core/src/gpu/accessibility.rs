//! Accessibility projection for EdgeRun UI nodes.

use std::string::String;
use std::vec::Vec;

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiA11yRole {
    Generic,
    Group,
    Text,
    Button,
    Checkbox,
    Radio,
    Textbox,
    Combobox,
    Dialog,
    Tooltip,
    Status,
    Progressbar,
    Table,
    Row,
    Cell,
    TabList,
    Tab,
    MenuItem,
    ListItem,
    Navigation,
    Separator,
    Image,
    Slider,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiA11yState {
    Disabled,
    Checked,
    Selected,
    Expanded,
    Focused,
    Current,
    Invalid,
    Open,
    Value(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiA11yNode {
    pub role: UiA11yRole,
    pub label: String,
    pub id: Option<u32>,
    pub states: Vec<UiA11yState>,
    pub children: Vec<UiA11yNode>,
}

impl UiA11yNode {
    pub fn has_state(&self, state: &UiA11yState) -> bool {
        self.states.iter().any(|candidate| candidate == state)
    }

    pub fn find_by_id(&self, id: u32) -> Option<&UiA11yNode> {
        if self.id == Some(id) {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find_by_id(id))
    }
}

pub fn accessibility_tree(node: &UiNode) -> UiA11yNode {
    accessibility_tree_with_state(node, None)
}

pub fn accessibility_tree_with_state(node: &UiNode, state: Option<&UiRuntimeState>) -> UiA11yNode {
    let mut out = accessibility_node_for_kind(&node.kind, node.style.disabled, state);
    out.children.extend(
        node.children
            .iter()
            .map(|child| accessibility_tree_with_state(child, state)),
    );
    out
}

fn accessibility_node_for_kind(
    kind: &UiNodeKind,
    disabled: bool,
    state: Option<&UiRuntimeState>,
) -> UiA11yNode {
    let mut node = match kind {
        UiNodeKind::Row
        | UiNodeKind::Column
        | UiNodeKind::Grid { .. }
        | UiNodeKind::Masonry { .. }
        | UiNodeKind::BentoGrid { .. }
        | UiNodeKind::Card => base(UiA11yRole::Group, "", None),
        UiNodeKind::ScrollArea { id, .. } => base(UiA11yRole::Group, "scroll area", *id),
        UiNodeKind::Text(value) => base(UiA11yRole::Text, value, None),
        UiNodeKind::Badge { label, .. } => base(UiA11yRole::Text, label, None),
        UiNodeKind::Button { label, id, .. } => base(UiA11yRole::Button, label, Some(*id)),
        UiNodeKind::IconButton {
            icon, id, tooltip, ..
        } => {
            let label = if tooltip.is_empty() {
                icon.accessibility_label()
            } else {
                tooltip
            };
            base(UiA11yRole::Button, label, Some(*id))
        }
        UiNodeKind::Icon { icon, .. } => base(UiA11yRole::Image, icon.accessibility_label(), None),
        UiNodeKind::Checkbox { label, checked, id } => {
            let checked = state
                .map(|state| state.toggle_value(*id, *checked))
                .unwrap_or(*checked);
            let mut node = base(UiA11yRole::Checkbox, label, Some(*id));
            if checked {
                node.states.push(UiA11yState::Checked);
            }
            node
        }
        UiNodeKind::Radio {
            label,
            selected,
            id,
        } => {
            let selected = state
                .map(|state| state.toggle_value(*id, *selected))
                .unwrap_or(*selected);
            let mut node = base(UiA11yRole::Radio, label, Some(*id));
            if selected {
                node.states.push(UiA11yState::Checked);
            }
            node
        }
        UiNodeKind::Select { label, value, id } => {
            let mut node = base(UiA11yRole::Combobox, label, Some(*id));
            node.states.push(UiA11yState::Value(value.clone()));
            if state.is_some_and(|state| state.open_value(*id, false)) {
                node.states.push(UiA11yState::Open);
                node.states.push(UiA11yState::Expanded);
            }
            node
        }
        UiNodeKind::Tooltip { text } => base(UiA11yRole::Tooltip, text, None),
        UiNodeKind::Dialog { title, .. } => base(UiA11yRole::Dialog, title, None),
        UiNodeKind::Toast { message, .. } => base(UiA11yRole::Status, message, None),
        UiNodeKind::EmptyState { title, body, .. } => {
            base(UiA11yRole::Group, &format_label(title, body), None)
        }
        UiNodeKind::Skeleton => base(UiA11yRole::Generic, "loading", None),
        UiNodeKind::ProgressRing { value, .. } | UiNodeKind::ProgressBar { value, .. } => {
            let mut node = base(UiA11yRole::Progressbar, "progress", None);
            node.states.push(UiA11yState::Value(format!("{value:.2}")));
            node
        }
        UiNodeKind::Table {
            headers,
            rows,
            id_base,
        } => table_accessibility(headers, rows, *id_base),
        UiNodeKind::Breadcrumb {
            items, selected, ..
        } => {
            let mut node = base(UiA11yRole::Navigation, "breadcrumb", None);
            node.children = items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let mut child = base(UiA11yRole::ListItem, item, None);
                    if index == *selected {
                        child.states.push(UiA11yState::Current);
                    }
                    child
                })
                .collect();
            node
        }
        UiNodeKind::CommandPalette { placeholder, id } => {
            base(UiA11yRole::Combobox, placeholder, Some(*id))
        }
        UiNodeKind::TreeItem {
            label,
            expanded,
            id,
            ..
        } => {
            let mut node = base(UiA11yRole::ListItem, label, Some(*id));
            if *expanded {
                node.states.push(UiA11yState::Expanded);
            }
            node
        }
        UiNodeKind::Section { title, detail } => {
            base(UiA11yRole::Group, &format_label(title, detail), None)
        }
        UiNodeKind::IdentityCard { name, id, .. }
        | UiNodeKind::ContactCard { name, id, .. }
        | UiNodeKind::AttachmentPreview { name, id, .. }
        | UiNodeKind::PackageCard { name, id, .. } => base(UiA11yRole::Group, name, Some(*id)),
        UiNodeKind::ThreadRow {
            title, unread, id, ..
        } => {
            let mut node = base(UiA11yRole::ListItem, title, Some(*id));
            if *unread {
                node.states.push(UiA11yState::Current);
            }
            node
        }
        UiNodeKind::CapabilityGrantRow { app, id, .. }
        | UiNodeKind::ProofEventRow { title: app, id, .. }
        | UiNodeKind::ReceiptRow { label: app, id, .. }
        | UiNodeKind::TransactionRow { title: app, id, .. }
        | UiNodeKind::ListRow { title: app, id, .. } => base(UiA11yRole::ListItem, app, Some(*id)),
        UiNodeKind::RoutePath { label, .. } => base(UiA11yRole::Group, label, None),
        UiNodeKind::AppLauncherItem { title, id, .. } => base(UiA11yRole::Button, title, Some(*id)),
        UiNodeKind::Toggle { on, id } => {
            let on = state
                .map(|state| state.toggle_value(*id, *on))
                .unwrap_or(*on);
            let mut node = base(UiA11yRole::Button, "toggle", Some(*id));
            if on {
                node.states.push(UiA11yState::Checked);
            }
            node
        }
        UiNodeKind::Avatar { label, online, .. } => {
            let mut node = base(UiA11yRole::Image, label, None);
            if *online {
                node.states.push(UiA11yState::Current);
            }
            node
        }
        UiNodeKind::Tabs {
            labels,
            selected,
            base_id,
        } => {
            let selected = state
                .map(|state| state.selected_tab_index(*base_id, labels.len(), *selected))
                .unwrap_or(*selected);
            let mut node = base(UiA11yRole::TabList, "tabs", None);
            node.children = labels
                .iter()
                .enumerate()
                .map(|(index, label)| {
                    let mut child = base(UiA11yRole::Tab, label, Some(*base_id + index as u32));
                    if index == selected {
                        child.states.push(UiA11yState::Selected);
                    }
                    child
                })
                .collect();
            node
        }
        UiNodeKind::PanelHeader { title, .. } => base(UiA11yRole::Group, title, None),
        UiNodeKind::MetricCard { title, value, .. } => {
            base(UiA11yRole::Group, &format_label(title, value), None)
        }
        UiNodeKind::Field {
            label,
            value,
            focused,
            id,
            ..
        } => textbox_accessibility(label, value, *focused, *id, state, UiA11yRole::Textbox),
        UiNodeKind::TextArea {
            label,
            value,
            focused,
            id,
        } => textbox_accessibility(label, value, *focused, *id, state, UiA11yRole::Textbox),
        UiNodeKind::Slider {
            label, value, id, ..
        } => {
            let value = state
                .map(|state| state.slider_value(*id, *value))
                .unwrap_or(*value);
            let mut node = base(UiA11yRole::Slider, label, Some(*id));
            node.states.push(UiA11yState::Value(format!("{value:.2}")));
            node
        }
        UiNodeKind::BarChart { title, .. } => base(UiA11yRole::Image, title, None),
        UiNodeKind::MenuItem {
            label,
            selected,
            id,
            ..
        } => {
            let mut node = base(UiA11yRole::MenuItem, label, Some(*id));
            if *selected {
                node.states.push(UiA11yState::Selected);
            }
            node
        }
        UiNodeKind::ControlRow {
            label,
            accessory,
            id,
            ..
        } => control_row_accessibility(label, accessory, *id, state),
        UiNodeKind::Divider => base(UiA11yRole::Separator, "", None),
        UiNodeKind::Spacer => base(UiA11yRole::Generic, "", None),
    };
    if disabled {
        node.states.push(UiA11yState::Disabled);
    }
    node
}

fn base(role: UiA11yRole, label: &str, id: Option<u32>) -> UiA11yNode {
    UiA11yNode {
        role,
        label: label.to_string(),
        id,
        states: Vec::new(),
        children: Vec::new(),
    }
}

fn textbox_accessibility(
    label: &str,
    value: &str,
    focused: bool,
    id: Option<u32>,
    state: Option<&UiRuntimeState>,
    role: UiA11yRole,
) -> UiA11yNode {
    let value = id
        .and_then(|id| state.map(|state| state.text_value(id, value)))
        .unwrap_or(value);
    let focused = focused
        || state.is_some_and(|state| {
            state.focused().is_some_and(|hit| {
                matches!(hit.kind, HitKind::Input | HitKind::TextArea) && Some(hit.id) == id
            })
        });
    let mut node = base(role, label, id);
    node.states.push(UiA11yState::Value(value.to_string()));
    if focused {
        node.states.push(UiA11yState::Focused);
    }
    node
}

fn table_accessibility(headers: &[String], rows: &[Vec<String>], id_base: u32) -> UiA11yNode {
    let mut node = base(UiA11yRole::Table, "table", Some(id_base));
    let mut header = base(UiA11yRole::Row, "header", None);
    header.children = headers
        .iter()
        .map(|value| base(UiA11yRole::Cell, value, None))
        .collect();
    node.children.push(header);
    node.children
        .extend(rows.iter().enumerate().map(|(index, row)| {
            let mut row_node = base(UiA11yRole::Row, "", Some(id_base + index as u32));
            row_node.children = row
                .iter()
                .map(|value| base(UiA11yRole::Cell, value, None))
                .collect();
            row_node
        }));
    node
}

fn control_row_accessibility(
    label: &str,
    accessory: &UiControlAccessory,
    id: Option<u32>,
    state: Option<&UiRuntimeState>,
) -> UiA11yNode {
    match accessory {
        UiControlAccessory::Toggle { on, id } => {
            let on = state
                .map(|state| state.toggle_value(*id, *on))
                .unwrap_or(*on);
            let mut node = base(UiA11yRole::Checkbox, label, Some(*id));
            if on {
                node.states.push(UiA11yState::Checked);
            }
            node
        }
        UiControlAccessory::Button {
            label: button_label,
            id,
            ..
        } => base(
            UiA11yRole::Button,
            &format_label(label, button_label),
            Some(*id),
        ),
        UiControlAccessory::Badge { label: badge, .. } | UiControlAccessory::Value(badge) => {
            base(UiA11yRole::ListItem, &format_label(label, badge), id)
        }
        UiControlAccessory::None => base(UiA11yRole::ListItem, label, id),
    }
}

fn format_label(a: &str, b: &str) -> String {
    if b.is_empty() {
        a.to_string()
    } else {
        format!("{a}: {b}")
    }
}

trait UiIconA11y {
    fn accessibility_label(self) -> &'static str;
}

impl UiIconA11y for UiIcon {
    fn accessibility_label(self) -> &'static str {
        match self {
            UiIcon::Check => "check",
            UiIcon::Warning => "warning",
            UiIcon::Search => "search",
            UiIcon::Settings => "settings",
            UiIcon::Bell => "notification",
            UiIcon::ChevronRight => "next",
            UiIcon::Eye => "view",
            UiIcon::Lock => "lock",
            UiIcon::User => "user",
            UiIcon::Wallet => "wallet",
            UiIcon::Database => "database",
            UiIcon::Shield => "shield",
            UiIcon::Network => "network",
            UiIcon::Cpu => "compute",
            UiIcon::Code => "code",
            UiIcon::File => "file",
            UiIcon::Activity => "activity",
            UiIcon::App => "app",
            UiIcon::Chat => "chat",
            UiIcon::Key => "key",
            UiIcon::Menu => "menu",
            UiIcon::MessagePlus => "new session",
            UiIcon::Route => "route",
            UiIcon::Send => "send",
            UiIcon::Server => "server",
            UiIcon::Sparkles => "sparkles",
            UiIcon::Storage => "storage",
            UiIcon::Terminal => "terminal",
            UiIcon::Trust => "trust",
            UiIcon::Trash => "clear session",
            UiIcon::X => "close",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_basic_shadcn_roles_states_and_values() {
        let tree = accessibility_tree(
            &column("gap-2")
                .child(shadcn_button(
                    "Save",
                    1,
                    UiShadcnButtonVariant::Default,
                    UiShadcnButtonSize::Default,
                ))
                .child(shadcn_checkbox("Accept", true, 2))
                .child(field_node("Email", "a@b.test").hit_id(3)),
        );

        assert_eq!(tree.role, UiA11yRole::Group);
        assert_eq!(tree.find_by_id(1).unwrap().role, UiA11yRole::Button);
        assert_eq!(tree.find_by_id(1).unwrap().label, "Save");
        assert!(tree.find_by_id(2).unwrap().has_state(&UiA11yState::Checked));
        assert!(
            tree.find_by_id(3)
                .unwrap()
                .has_state(&UiA11yState::Value("a@b.test".to_string()))
        );
    }

    #[test]
    fn projects_runtime_backed_open_and_selected_state() {
        let mut state = UiRuntimeState::default();
        state.set_open(10, true);
        state.set_open(20, true);
        state.set_focus_scope(
            20,
            &[GpuHit::new(HitKind::Button, 99, 0.0, 0.0, 10.0, 10.0)],
        );

        let select =
            accessibility_tree_with_state(&shadcn_select("Framework", "Next.js", 10), Some(&state));
        assert_eq!(select.role, UiA11yRole::Combobox);
        assert!(select.has_state(&UiA11yState::Open));
        assert!(select.has_state(&UiA11yState::Expanded));

        let tabs = accessibility_tree_with_state(&shadcn_tabs(&["A", "B"], 1, 30), Some(&state));
        assert_eq!(tabs.role, UiA11yRole::TabList);
        assert!(tabs.children[1].has_state(&UiA11yState::Selected));
    }

    #[test]
    fn projects_compound_structures() {
        let table = accessibility_tree(&shadcn_table(
            &["Invoice", "Status"],
            &[&["INV001", "Paid"]],
            50,
        ));
        assert_eq!(table.role, UiA11yRole::Table);
        assert_eq!(table.children[0].role, UiA11yRole::Row);
        assert_eq!(table.children[0].children[0].label, "Invoice");

        let dialog = accessibility_tree(&shadcn_dialog("Edit profile", "Body", UiIcon::Settings));
        assert_eq!(dialog.role, UiA11yRole::Dialog);
        assert_eq!(dialog.label, "Edit profile");
    }
}
