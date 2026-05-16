//! Reusable GPU UI components for the shared native/WASM scene.
//!
//! Components are semantic Rust structs rendered through `UiPainter`. Browser
//! and native hosts only consume the resulting `GpuScene`, keeping application
//! layout and state out of JavaScript, SDL, and OpenGL glue.

use super::{
    Axis, ButtonStyle, Color4, HitKind, UiA11yRole, UiIcon, UiPainter, UiRect, palette, spacing,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiComponentTestId {
    NetworkAppPrompt,
    AppStoreCard,
    TrustManagerActions,
    RuntimeEventRow,
    PackageProofRow,
    ImportSyncSourceRow,
    PublishFromNodeRow,
    NodeInstanceRow,
    AdmissionPolicyRow,
    RouteBudgetRow,
    DataTableControls,
    IconOnlyButton,
    SegmentedControl,
    ReceiptPaymentRow,
    CapabilityGrantDetailRow,
    SystemSurfaceStatePanel,
}

pub const UI_COMPONENT_TEST_IDS: &[UiComponentTestId] = &[
    UiComponentTestId::NetworkAppPrompt,
    UiComponentTestId::AppStoreCard,
    UiComponentTestId::TrustManagerActions,
    UiComponentTestId::RuntimeEventRow,
    UiComponentTestId::PackageProofRow,
    UiComponentTestId::ImportSyncSourceRow,
    UiComponentTestId::PublishFromNodeRow,
    UiComponentTestId::NodeInstanceRow,
    UiComponentTestId::AdmissionPolicyRow,
    UiComponentTestId::RouteBudgetRow,
    UiComponentTestId::DataTableControls,
    UiComponentTestId::IconOnlyButton,
    UiComponentTestId::SegmentedControl,
    UiComponentTestId::ReceiptPaymentRow,
    UiComponentTestId::CapabilityGrantDetailRow,
    UiComponentTestId::SystemSurfaceStatePanel,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiComponentState {
    Default,
    Hover,
    Focus,
    Active,
    Disabled,
    Loading,
    Error,
}

pub const UI_COMPONENT_STATES: &[UiComponentState] = &[
    UiComponentState::Default,
    UiComponentState::Hover,
    UiComponentState::Focus,
    UiComponentState::Active,
    UiComponentState::Disabled,
    UiComponentState::Loading,
    UiComponentState::Error,
];

impl UiComponentState {
    pub const fn selector(self) -> &'static str {
        match self {
            Self::Default => "state.default",
            Self::Hover => "state.hover",
            Self::Focus => "state.focus",
            Self::Active => "state.active",
            Self::Disabled => "state.disabled",
            Self::Loading => "state.loading",
            Self::Error => "state.error",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Hover => "Hover",
            Self::Focus => "Focus",
            Self::Active => "Active",
            Self::Disabled => "Disabled",
            Self::Loading => "Loading",
            Self::Error => "Error",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiComponentStateMatrix {
    pub component: UiComponentTestId,
    pub states: &'static [UiComponentState],
}

impl UiComponentStateMatrix {
    pub fn has_state(self, state: UiComponentState) -> bool {
        self.states
            .iter()
            .copied()
            .any(|candidate| candidate == state)
    }

    pub const fn component_selector(self) -> &'static str {
        self.component.selector()
    }
}

const fn full_state_matrix(component: UiComponentTestId) -> UiComponentStateMatrix {
    UiComponentStateMatrix {
        component,
        states: UI_COMPONENT_STATES,
    }
}

pub const UI_COMPONENT_STATE_MATRICES: &[UiComponentStateMatrix] = &[
    full_state_matrix(UiComponentTestId::NetworkAppPrompt),
    full_state_matrix(UiComponentTestId::AppStoreCard),
    full_state_matrix(UiComponentTestId::TrustManagerActions),
    full_state_matrix(UiComponentTestId::RuntimeEventRow),
    full_state_matrix(UiComponentTestId::PackageProofRow),
    full_state_matrix(UiComponentTestId::ImportSyncSourceRow),
    full_state_matrix(UiComponentTestId::PublishFromNodeRow),
    full_state_matrix(UiComponentTestId::NodeInstanceRow),
    full_state_matrix(UiComponentTestId::AdmissionPolicyRow),
    full_state_matrix(UiComponentTestId::RouteBudgetRow),
    full_state_matrix(UiComponentTestId::DataTableControls),
    full_state_matrix(UiComponentTestId::IconOnlyButton),
    full_state_matrix(UiComponentTestId::SegmentedControl),
    full_state_matrix(UiComponentTestId::ReceiptPaymentRow),
    full_state_matrix(UiComponentTestId::CapabilityGrantDetailRow),
    full_state_matrix(UiComponentTestId::SystemSurfaceStatePanel),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiComponentProjectedField {
    pub name: &'static str,
    pub required: bool,
}

impl UiComponentProjectedField {
    pub const fn required(name: &'static str) -> Self {
        Self {
            name,
            required: true,
        }
    }

    pub const fn optional(name: &'static str) -> Self {
        Self {
            name,
            required: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiComponentProjectionContract {
    pub component: UiComponentTestId,
    pub fields: &'static [UiComponentProjectedField],
}

impl UiComponentProjectionContract {
    pub fn has_field(self, name: &str) -> bool {
        self.fields.iter().any(|field| field.name == name)
    }

    pub fn required_fields(self) -> impl Iterator<Item = UiComponentProjectedField> {
        self.fields.iter().copied().filter(|field| field.required)
    }
}

const NETWORK_APP_PROMPT_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("app_name"),
    UiComponentProjectedField::optional("package_size"),
    UiComponentProjectedField::optional("retrieval_cost"),
    UiComponentProjectedField::optional("policy_hash"),
    UiComponentProjectedField::required("run_once_id"),
    UiComponentProjectedField::required("verify_cache_id"),
    UiComponentProjectedField::required("cancel_id"),
];
const APP_STORE_CARD_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("name"),
    UiComponentProjectedField::optional("developer"),
    UiComponentProjectedField::optional("release"),
    UiComponentProjectedField::optional("package_hash"),
    UiComponentProjectedField::optional("app_policy_hash"),
    UiComponentProjectedField::optional("access_mode"),
    UiComponentProjectedField::required("detail_id"),
    UiComponentProjectedField::required("run_id"),
];
const TRUST_MANAGER_ACTION_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("open_identity_id"),
    UiComponentProjectedField::required("open_app_store_id"),
    UiComponentProjectedField::required("revoke_grant_id"),
    UiComponentProjectedField::required("remove_cache_id"),
];
const RUNTIME_EVENT_ROW_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("title"),
    UiComponentProjectedField::optional("detail"),
    UiComponentProjectedField::required("event_hash"),
    UiComponentProjectedField::optional("status"),
    UiComponentProjectedField::required("id"),
    UiComponentProjectedField::optional("accent"),
];
const PACKAGE_PROOF_ROW_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("package_hash"),
    UiComponentProjectedField::required("manifest_hash"),
    UiComponentProjectedField::optional("developer"),
    UiComponentProjectedField::optional("release"),
    UiComponentProjectedField::optional("status"),
    UiComponentProjectedField::required("id"),
];
const IMPORT_SYNC_SOURCE_ROW_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("kind"),
    UiComponentProjectedField::optional("name"),
    UiComponentProjectedField::optional("detail"),
    UiComponentProjectedField::optional("policy_hash"),
    UiComponentProjectedField::optional("status"),
    UiComponentProjectedField::required("id"),
    UiComponentProjectedField::optional("sync_id"),
    UiComponentProjectedField::optional("configure_id"),
];
const PUBLISH_FROM_NODE_ROW_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("kind"),
    UiComponentProjectedField::optional("title"),
    UiComponentProjectedField::optional("node_instance"),
    UiComponentProjectedField::optional("route_scope"),
    UiComponentProjectedField::optional("policy_hash"),
    UiComponentProjectedField::optional("budget"),
    UiComponentProjectedField::optional("status"),
    UiComponentProjectedField::required("id"),
    UiComponentProjectedField::optional("publish_id"),
    UiComponentProjectedField::optional("configure_id"),
];
const NODE_INSTANCE_ROW_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("role"),
    UiComponentProjectedField::optional("runtime_target"),
    UiComponentProjectedField::optional("route_scope"),
    UiComponentProjectedField::optional("policy_hash"),
    UiComponentProjectedField::optional("status"),
    UiComponentProjectedField::required("id"),
    UiComponentProjectedField::optional("open_id"),
];
const ADMISSION_POLICY_ROW_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("source"),
    UiComponentProjectedField::required("policy_hash"),
    UiComponentProjectedField::optional("admission_node"),
    UiComponentProjectedField::optional("validity"),
    UiComponentProjectedField::optional("status"),
    UiComponentProjectedField::required("id"),
    UiComponentProjectedField::optional("inspect_id"),
];
const ROUTE_BUDGET_ROW_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("route"),
    UiComponentProjectedField::required("admitted_budget"),
    UiComponentProjectedField::optional("spent"),
    UiComponentProjectedField::optional("channel"),
    UiComponentProjectedField::optional("status"),
    UiComponentProjectedField::required("id"),
    UiComponentProjectedField::optional("inspect_id"),
];
const DATA_TABLE_CONTROL_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::optional("title"),
    UiComponentProjectedField::optional("filter_label"),
    UiComponentProjectedField::optional("filter_value"),
    UiComponentProjectedField::required("filter_id"),
    UiComponentProjectedField::optional("clear_filter_id"),
    UiComponentProjectedField::required("columns"),
];
const ICON_ONLY_BUTTON_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("icon"),
    UiComponentProjectedField::required("id"),
    UiComponentProjectedField::required("label"),
    UiComponentProjectedField::optional("active"),
];
const SEGMENTED_CONTROL_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("items"),
    UiComponentProjectedField::required("selected"),
];
const RECEIPT_PAYMENT_ROW_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("label"),
    UiComponentProjectedField::optional("receipt_id"),
    UiComponentProjectedField::optional("policy_hash"),
    UiComponentProjectedField::required("amount"),
    UiComponentProjectedField::required("status"),
    UiComponentProjectedField::required("id"),
    UiComponentProjectedField::optional("action"),
];
const CAPABILITY_GRANT_ROW_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("app"),
    UiComponentProjectedField::required("capability"),
    UiComponentProjectedField::optional("scope"),
    UiComponentProjectedField::optional("policy_hash"),
    UiComponentProjectedField::optional("expiry"),
    UiComponentProjectedField::optional("status"),
    UiComponentProjectedField::required("id"),
    UiComponentProjectedField::optional("revoke_id"),
];
const SYSTEM_SURFACE_STATE_PANEL_FIELDS: &[UiComponentProjectedField] = &[
    UiComponentProjectedField::required("kind"),
    UiComponentProjectedField::required("state"),
    UiComponentProjectedField::optional("title"),
    UiComponentProjectedField::optional("detail"),
    UiComponentProjectedField::optional("reference"),
    UiComponentProjectedField::required("id"),
    UiComponentProjectedField::optional("action"),
];

pub const UI_COMPONENT_PROJECTION_CONTRACTS: &[UiComponentProjectionContract] = &[
    UiComponentProjectionContract {
        component: UiComponentTestId::NetworkAppPrompt,
        fields: NETWORK_APP_PROMPT_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::AppStoreCard,
        fields: APP_STORE_CARD_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::TrustManagerActions,
        fields: TRUST_MANAGER_ACTION_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::RuntimeEventRow,
        fields: RUNTIME_EVENT_ROW_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::PackageProofRow,
        fields: PACKAGE_PROOF_ROW_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::ImportSyncSourceRow,
        fields: IMPORT_SYNC_SOURCE_ROW_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::PublishFromNodeRow,
        fields: PUBLISH_FROM_NODE_ROW_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::NodeInstanceRow,
        fields: NODE_INSTANCE_ROW_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::AdmissionPolicyRow,
        fields: ADMISSION_POLICY_ROW_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::RouteBudgetRow,
        fields: ROUTE_BUDGET_ROW_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::DataTableControls,
        fields: DATA_TABLE_CONTROL_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::IconOnlyButton,
        fields: ICON_ONLY_BUTTON_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::SegmentedControl,
        fields: SEGMENTED_CONTROL_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::ReceiptPaymentRow,
        fields: RECEIPT_PAYMENT_ROW_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::CapabilityGrantDetailRow,
        fields: CAPABILITY_GRANT_ROW_FIELDS,
    },
    UiComponentProjectionContract {
        component: UiComponentTestId::SystemSurfaceStatePanel,
        fields: SYSTEM_SURFACE_STATE_PANEL_FIELDS,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiComponentAccessibilityMetadata {
    pub component: UiComponentTestId,
    pub role: UiA11yRole,
    pub label_fields: &'static [&'static str],
}

impl UiComponentAccessibilityMetadata {
    pub fn has_label_field(self, name: &str) -> bool {
        self.label_fields.iter().any(|field| *field == name)
    }
}

pub const UI_COMPONENT_ACCESSIBILITY_METADATA: &[UiComponentAccessibilityMetadata] = &[
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::NetworkAppPrompt,
        role: UiA11yRole::Dialog,
        label_fields: &["app_name"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::AppStoreCard,
        role: UiA11yRole::ListItem,
        label_fields: &["name", "developer", "app_policy_hash"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::TrustManagerActions,
        role: UiA11yRole::Group,
        label_fields: &[
            "open_identity_id",
            "open_app_store_id",
            "revoke_grant_id",
            "remove_cache_id",
        ],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::RuntimeEventRow,
        role: UiA11yRole::ListItem,
        label_fields: &["title", "event_hash", "status"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::PackageProofRow,
        role: UiA11yRole::ListItem,
        label_fields: &["package_hash", "manifest_hash", "status"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::ImportSyncSourceRow,
        role: UiA11yRole::ListItem,
        label_fields: &["kind", "name", "policy_hash", "status"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::PublishFromNodeRow,
        role: UiA11yRole::ListItem,
        label_fields: &["kind", "title", "node_instance", "policy_hash", "status"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::NodeInstanceRow,
        role: UiA11yRole::ListItem,
        label_fields: &["role", "runtime_target", "policy_hash", "status"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::AdmissionPolicyRow,
        role: UiA11yRole::ListItem,
        label_fields: &["source", "policy_hash", "admission_node", "status"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::RouteBudgetRow,
        role: UiA11yRole::ListItem,
        label_fields: &["route", "admitted_budget", "status"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::DataTableControls,
        role: UiA11yRole::Group,
        label_fields: &["title", "filter_label", "columns"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::IconOnlyButton,
        role: UiA11yRole::Button,
        label_fields: &["label"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::SegmentedControl,
        role: UiA11yRole::TabList,
        label_fields: &["items"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::ReceiptPaymentRow,
        role: UiA11yRole::ListItem,
        label_fields: &["label", "receipt_id", "amount", "status"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::CapabilityGrantDetailRow,
        role: UiA11yRole::ListItem,
        label_fields: &["app", "capability", "scope", "status"],
    },
    UiComponentAccessibilityMetadata {
        component: UiComponentTestId::SystemSurfaceStatePanel,
        role: UiA11yRole::Status,
        label_fields: &["kind", "state", "title", "detail"],
    },
];

impl UiComponentTestId {
    pub const fn selector(self) -> &'static str {
        match self {
            Self::NetworkAppPrompt => "edgerun.network_app_prompt",
            Self::AppStoreCard => "edgerun.app_store_card",
            Self::TrustManagerActions => "edgerun.trust_manager_actions",
            Self::RuntimeEventRow => "edgerun.runtime_event_row",
            Self::PackageProofRow => "edgerun.package_proof_row",
            Self::ImportSyncSourceRow => "edgerun.import_sync_source_row",
            Self::PublishFromNodeRow => "edgerun.publish_from_node_row",
            Self::NodeInstanceRow => "edgerun.node_instance_row",
            Self::AdmissionPolicyRow => "edgerun.admission_policy_row",
            Self::RouteBudgetRow => "edgerun.route_budget_row",
            Self::DataTableControls => "edgerun.data_table_controls",
            Self::IconOnlyButton => "edgerun.icon_only_button",
            Self::SegmentedControl => "edgerun.segmented_control",
            Self::ReceiptPaymentRow => "edgerun.receipt_payment_row",
            Self::CapabilityGrantDetailRow => "edgerun.capability_grant_detail_row",
            Self::SystemSurfaceStatePanel => "edgerun.system_surface_state_panel",
        }
    }

    pub const fn component_name(self) -> &'static str {
        match self {
            Self::NetworkAppPrompt => "NetworkAppPrompt",
            Self::AppStoreCard => "AppStoreCard",
            Self::TrustManagerActions => "TrustManagerActions",
            Self::RuntimeEventRow => "RuntimeEventRow",
            Self::PackageProofRow => "PackageProofRow",
            Self::ImportSyncSourceRow => "ImportSyncSourceRow",
            Self::PublishFromNodeRow => "PublishFromNodeRow",
            Self::NodeInstanceRow => "NodeInstanceRow",
            Self::AdmissionPolicyRow => "AdmissionPolicyRow",
            Self::RouteBudgetRow => "RouteBudgetRow",
            Self::DataTableControls => "DataTableControls",
            Self::IconOnlyButton => "IconOnlyButton",
            Self::SegmentedControl => "SegmentedControl",
            Self::ReceiptPaymentRow => "ReceiptPaymentRow",
            Self::CapabilityGrantDetailRow => "CapabilityGrantDetailRow",
            Self::SystemSurfaceStatePanel => "SystemSurfaceStatePanel",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelHeader<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub action: Option<(&'a str, u32)>,
}

impl<'a> PanelHeader<'a> {
    pub const fn new(title: &'a str) -> Self {
        Self {
            title,
            subtitle: "",
            action: None,
        }
    }

    pub const fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = subtitle;
        self
    }

    pub const fn action(mut self, label: &'a str, id: u32) -> Self {
        self.action = Some((label, id));
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetricCard<'a> {
    pub title: &'a str,
    pub value: &'a str,
    pub detail: &'a str,
    pub progress: Option<f32>,
    pub accent: Color4,
}

impl<'a> MetricCard<'a> {
    pub const fn new(title: &'a str, value: &'a str) -> Self {
        Self {
            title,
            value,
            detail: "",
            progress: None,
            accent: palette::ACCENT,
        }
    }

    pub const fn detail(mut self, detail: &'a str) -> Self {
        self.detail = detail;
        self
    }

    pub const fn progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress);
        self
    }

    pub const fn accent(mut self, accent: Color4) -> Self {
        self.accent = accent;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Field<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub helper: &'a str,
    pub focused: bool,
    pub id: Option<u32>,
}

impl<'a> Field<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            value: "",
            helper: "",
            focused: false,
            id: None,
        }
    }

    pub const fn value(mut self, value: &'a str) -> Self {
        self.value = value;
        self
    }

    pub const fn helper(mut self, helper: &'a str) -> Self {
        self.helper = helper;
        self
    }

    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub const fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextArea<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub focused: bool,
    pub id: Option<u32>,
}

impl<'a> TextArea<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            value: "",
            focused: false,
            id: None,
        }
    }

    pub const fn value(mut self, value: &'a str) -> Self {
        self.value = value;
        self
    }

    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub const fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slider<'a> {
    pub label: &'a str,
    pub value: f32,
    pub min_label: &'a str,
    pub max_label: &'a str,
    pub accent: Color4,
    pub id: u32,
}

impl<'a> Slider<'a> {
    pub const fn new(label: &'a str, value: f32, id: u32) -> Self {
        Self {
            label,
            value,
            min_label: "",
            max_label: "",
            accent: palette::ACCENT,
            id,
        }
    }

    pub const fn range_labels(mut self, min_label: &'a str, max_label: &'a str) -> Self {
        self.min_label = min_label;
        self.max_label = max_label;
        self
    }

    pub const fn accent(mut self, accent: Color4) -> Self {
        self.accent = accent;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarChart<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub labels: &'a [&'a str],
    pub values: &'a [f32],
    pub accent: Color4,
    pub bar_base_id: Option<u32>,
}

impl<'a> BarChart<'a> {
    pub const fn new(title: &'a str, labels: &'a [&'a str], values: &'a [f32]) -> Self {
        Self {
            title,
            subtitle: "",
            labels,
            values,
            accent: palette::ACCENT,
            bar_base_id: None,
        }
    }

    pub const fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = subtitle;
        self
    }

    pub const fn accent(mut self, accent: Color4) -> Self {
        self.accent = accent;
        self
    }

    pub const fn bar_base_id(mut self, id: u32) -> Self {
        self.bar_base_id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransactionRow<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub date: &'a str,
    pub amount: &'a str,
    pub positive: bool,
    pub id: u32,
}

impl<'a> TransactionRow<'a> {
    pub const fn new(title: &'a str, amount: &'a str, id: u32) -> Self {
        Self {
            title,
            subtitle: "",
            date: "",
            amount,
            positive: false,
            id,
        }
    }

    pub const fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = subtitle;
        self
    }

    pub const fn date(mut self, date: &'a str) -> Self {
        self.date = date;
        self
    }

    pub const fn positive(mut self, positive: bool) -> Self {
        self.positive = positive;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuItem<'a> {
    pub label: &'a str,
    pub detail: &'a str,
    pub badge: &'a str,
    pub selected: bool,
    pub accent: Color4,
    pub id: u32,
}

impl<'a> MenuItem<'a> {
    pub const fn new(label: &'a str, id: u32) -> Self {
        Self {
            label,
            detail: "",
            badge: "",
            selected: false,
            accent: palette::ACCENT,
            id,
        }
    }

    pub const fn detail(mut self, detail: &'a str) -> Self {
        self.detail = detail;
        self
    }

    pub const fn badge(mut self, badge: &'a str) -> Self {
        self.badge = badge;
        self
    }

    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub const fn accent(mut self, accent: Color4) -> Self {
        self.accent = accent;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlAccessory<'a> {
    None,
    Value(&'a str),
    Badge(&'a str, Color4),
    Toggle {
        on: bool,
        id: u32,
    },
    Button {
        label: &'a str,
        id: u32,
        style: ButtonStyle,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlRow<'a> {
    pub label: &'a str,
    pub detail: &'a str,
    pub accessory: ControlAccessory<'a>,
    pub id: Option<u32>,
}

impl<'a> ControlRow<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            detail: "",
            accessory: ControlAccessory::None,
            id: None,
        }
    }

    pub const fn detail(mut self, detail: &'a str) -> Self {
        self.detail = detail;
        self
    }

    pub const fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }

    pub const fn value(mut self, value: &'a str) -> Self {
        self.accessory = ControlAccessory::Value(value);
        self
    }

    pub const fn badge(mut self, label: &'a str, color: Color4) -> Self {
        self.accessory = ControlAccessory::Badge(label, color);
        self
    }

    pub const fn toggle(mut self, on: bool, id: u32) -> Self {
        self.accessory = ControlAccessory::Toggle { on, id };
        self
    }

    pub const fn button(mut self, label: &'a str, id: u32, style: ButtonStyle) -> Self {
        self.accessory = ControlAccessory::Button { label, id, style };
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetworkAppPrompt<'a> {
    pub app_name: &'a str,
    pub package_size: &'a str,
    pub retrieval_cost: &'a str,
    pub policy_hash: &'a str,
    pub run_once_id: u32,
    pub verify_cache_id: u32,
    pub cancel_id: u32,
}

impl<'a> NetworkAppPrompt<'a> {
    pub const fn new(
        app_name: &'a str,
        run_once_id: u32,
        verify_cache_id: u32,
        cancel_id: u32,
    ) -> Self {
        Self {
            app_name,
            package_size: "",
            retrieval_cost: "",
            policy_hash: "",
            run_once_id,
            verify_cache_id,
            cancel_id,
        }
    }

    pub const fn package_size(mut self, package_size: &'a str) -> Self {
        self.package_size = package_size;
        self
    }

    pub const fn retrieval_cost(mut self, retrieval_cost: &'a str) -> Self {
        self.retrieval_cost = retrieval_cost;
        self
    }

    pub const fn policy_hash(mut self, policy_hash: &'a str) -> Self {
        self.policy_hash = policy_hash;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppStoreCard<'a> {
    pub name: &'a str,
    pub developer: &'a str,
    pub release: &'a str,
    pub package_hash: &'a str,
    pub app_policy_hash: &'a str,
    pub access_mode: &'a str,
    pub detail_id: u32,
    pub run_id: u32,
}

impl<'a> AppStoreCard<'a> {
    pub const fn new(name: &'a str, detail_id: u32, run_id: u32) -> Self {
        Self {
            name,
            developer: "",
            release: "",
            package_hash: "",
            app_policy_hash: "",
            access_mode: "",
            detail_id,
            run_id,
        }
    }

    pub const fn developer(mut self, developer: &'a str) -> Self {
        self.developer = developer;
        self
    }

    pub const fn release(mut self, release: &'a str) -> Self {
        self.release = release;
        self
    }

    pub const fn package_hash(mut self, package_hash: &'a str) -> Self {
        self.package_hash = package_hash;
        self
    }

    pub const fn app_policy_hash(mut self, app_policy_hash: &'a str) -> Self {
        self.app_policy_hash = app_policy_hash;
        self
    }

    pub const fn access_mode(mut self, access_mode: &'a str) -> Self {
        self.access_mode = access_mode;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrustManagerActions {
    pub open_identity_id: u32,
    pub open_app_store_id: u32,
    pub revoke_grant_id: u32,
    pub remove_cache_id: u32,
}

impl TrustManagerActions {
    pub const fn new(
        open_identity_id: u32,
        open_app_store_id: u32,
        revoke_grant_id: u32,
        remove_cache_id: u32,
    ) -> Self {
        Self {
            open_identity_id,
            open_app_store_id,
            revoke_grant_id,
            remove_cache_id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuntimeEventRow<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    pub event_hash: &'a str,
    pub status: &'a str,
    pub id: u32,
    pub accent: Color4,
}

impl<'a> RuntimeEventRow<'a> {
    pub const fn new(title: &'a str, event_hash: &'a str, id: u32) -> Self {
        Self {
            title,
            detail: "",
            event_hash,
            status: "recorded",
            id,
            accent: palette::ACCENT,
        }
    }

    pub const fn detail(mut self, detail: &'a str) -> Self {
        self.detail = detail;
        self
    }

    pub const fn status(mut self, status: &'a str) -> Self {
        self.status = status;
        self
    }

    pub const fn accent(mut self, accent: Color4) -> Self {
        self.accent = accent;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PackageProofRow<'a> {
    pub package_hash: &'a str,
    pub manifest_hash: &'a str,
    pub developer: &'a str,
    pub release: &'a str,
    pub status: &'a str,
    pub id: u32,
}

impl<'a> PackageProofRow<'a> {
    pub const fn new(package_hash: &'a str, manifest_hash: &'a str, id: u32) -> Self {
        Self {
            package_hash,
            manifest_hash,
            developer: "",
            release: "",
            status: "verified",
            id,
        }
    }

    pub const fn developer(mut self, developer: &'a str) -> Self {
        self.developer = developer;
        self
    }

    pub const fn release(mut self, release: &'a str) -> Self {
        self.release = release;
        self
    }

    pub const fn status(mut self, status: &'a str) -> Self {
        self.status = status;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportSyncSourceKind {
    Gmail,
    Drive,
    GitHub,
    Folder,
    Mailbox,
}

impl ImportSyncSourceKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Gmail => "Gmail",
            Self::Drive => "Drive",
            Self::GitHub => "GitHub",
            Self::Folder => "Folder",
            Self::Mailbox => "Mailbox",
        }
    }

    pub const fn icon(self) -> UiIcon {
        match self {
            Self::Gmail => UiIcon::MessagePlus,
            Self::Drive => UiIcon::Storage,
            Self::GitHub => UiIcon::Code,
            Self::Folder => UiIcon::File,
            Self::Mailbox => UiIcon::Database,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImportSyncSourceRow<'a> {
    pub kind: ImportSyncSourceKind,
    pub name: &'a str,
    pub detail: &'a str,
    pub policy_hash: &'a str,
    pub status: &'a str,
    pub id: u32,
    pub sync_id: Option<u32>,
    pub configure_id: Option<u32>,
}

impl<'a> ImportSyncSourceRow<'a> {
    pub const fn new(kind: ImportSyncSourceKind, id: u32) -> Self {
        Self {
            kind,
            name: "",
            detail: "",
            policy_hash: "",
            status: "ready",
            id,
            sync_id: None,
            configure_id: None,
        }
    }

    pub const fn name(mut self, name: &'a str) -> Self {
        self.name = name;
        self
    }

    pub const fn detail(mut self, detail: &'a str) -> Self {
        self.detail = detail;
        self
    }

    pub const fn policy_hash(mut self, policy_hash: &'a str) -> Self {
        self.policy_hash = policy_hash;
        self
    }

    pub const fn status(mut self, status: &'a str) -> Self {
        self.status = status;
        self
    }

    pub const fn sync_action(mut self, id: u32) -> Self {
        self.sync_id = Some(id);
        self
    }

    pub const fn configure_action(mut self, id: u32) -> Self {
        self.configure_id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishFromNodeKind {
    Site,
    App,
    Api,
    File,
}

impl PublishFromNodeKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Site => "Site",
            Self::App => "App",
            Self::Api => "API",
            Self::File => "File",
        }
    }

    pub const fn icon(self) -> UiIcon {
        match self {
            Self::Site => UiIcon::Network,
            Self::App => UiIcon::App,
            Self::Api => UiIcon::Code,
            Self::File => UiIcon::File,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PublishFromNodeRow<'a> {
    pub kind: PublishFromNodeKind,
    pub title: &'a str,
    pub node_instance: &'a str,
    pub route_scope: &'a str,
    pub policy_hash: &'a str,
    pub budget: &'a str,
    pub status: &'a str,
    pub id: u32,
    pub publish_id: Option<u32>,
    pub configure_id: Option<u32>,
}

impl<'a> PublishFromNodeRow<'a> {
    pub const fn new(kind: PublishFromNodeKind, id: u32) -> Self {
        Self {
            kind,
            title: "",
            node_instance: "",
            route_scope: "",
            policy_hash: "",
            budget: "",
            status: "ready",
            id,
            publish_id: None,
            configure_id: None,
        }
    }

    pub const fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub const fn node_instance(mut self, node_instance: &'a str) -> Self {
        self.node_instance = node_instance;
        self
    }

    pub const fn route_scope(mut self, route_scope: &'a str) -> Self {
        self.route_scope = route_scope;
        self
    }

    pub const fn policy_hash(mut self, policy_hash: &'a str) -> Self {
        self.policy_hash = policy_hash;
        self
    }

    pub const fn budget(mut self, budget: &'a str) -> Self {
        self.budget = budget;
        self
    }

    pub const fn status(mut self, status: &'a str) -> Self {
        self.status = status;
        self
    }

    pub const fn publish_action(mut self, id: u32) -> Self {
        self.publish_id = Some(id);
        self
    }

    pub const fn configure_action(mut self, id: u32) -> Self {
        self.configure_id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeInstanceRow<'a> {
    pub role: &'a str,
    pub runtime_target: &'a str,
    pub route_scope: &'a str,
    pub policy_hash: &'a str,
    pub status: &'a str,
    pub id: u32,
    pub open_id: Option<u32>,
}

impl<'a> NodeInstanceRow<'a> {
    pub const fn new(role: &'a str, id: u32) -> Self {
        Self {
            role,
            runtime_target: "",
            route_scope: "",
            policy_hash: "",
            status: "ready",
            id,
            open_id: None,
        }
    }

    pub const fn runtime_target(mut self, runtime_target: &'a str) -> Self {
        self.runtime_target = runtime_target;
        self
    }

    pub const fn route_scope(mut self, route_scope: &'a str) -> Self {
        self.route_scope = route_scope;
        self
    }

    pub const fn policy_hash(mut self, policy_hash: &'a str) -> Self {
        self.policy_hash = policy_hash;
        self
    }

    pub const fn status(mut self, status: &'a str) -> Self {
        self.status = status;
        self
    }

    pub const fn open_action(mut self, id: u32) -> Self {
        self.open_id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdmissionPolicyRow<'a> {
    pub source: &'a str,
    pub policy_hash: &'a str,
    pub admission_node: &'a str,
    pub validity: &'a str,
    pub status: &'a str,
    pub id: u32,
    pub inspect_id: Option<u32>,
}

impl<'a> AdmissionPolicyRow<'a> {
    pub const fn new(source: &'a str, policy_hash: &'a str, id: u32) -> Self {
        Self {
            source,
            policy_hash,
            admission_node: "",
            validity: "",
            status: "committed",
            id,
            inspect_id: None,
        }
    }

    pub const fn admission_node(mut self, admission_node: &'a str) -> Self {
        self.admission_node = admission_node;
        self
    }

    pub const fn validity(mut self, validity: &'a str) -> Self {
        self.validity = validity;
        self
    }

    pub const fn status(mut self, status: &'a str) -> Self {
        self.status = status;
        self
    }

    pub const fn inspect_action(mut self, id: u32) -> Self {
        self.inspect_id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RouteBudgetRow<'a> {
    pub route: &'a str,
    pub admitted_budget: &'a str,
    pub spent: &'a str,
    pub channel: &'a str,
    pub status: &'a str,
    pub id: u32,
    pub inspect_id: Option<u32>,
}

impl<'a> RouteBudgetRow<'a> {
    pub const fn new(route: &'a str, admitted_budget: &'a str, id: u32) -> Self {
        Self {
            route,
            admitted_budget,
            spent: "",
            channel: "",
            status: "active",
            id,
            inspect_id: None,
        }
    }

    pub const fn spent(mut self, spent: &'a str) -> Self {
        self.spent = spent;
        self
    }

    pub const fn channel(mut self, channel: &'a str) -> Self {
        self.channel = channel;
        self
    }

    pub const fn status(mut self, status: &'a str) -> Self {
        self.status = status;
        self
    }

    pub const fn inspect_action(mut self, id: u32) -> Self {
        self.inspect_id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataTableSort {
    None,
    Asc,
    Desc,
}

impl DataTableSort {
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "sort",
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DataTableColumn<'a> {
    pub label: &'a str,
    pub sort: DataTableSort,
    pub sort_id: Option<u32>,
}

impl<'a> DataTableColumn<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            sort: DataTableSort::None,
            sort_id: None,
        }
    }

    pub const fn sort(mut self, sort: DataTableSort, id: u32) -> Self {
        self.sort = sort;
        self.sort_id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DataTableControls<'a> {
    pub title: &'a str,
    pub filter_label: &'a str,
    pub filter_value: &'a str,
    pub filter_id: u32,
    pub clear_filter_id: Option<u32>,
    pub columns: &'a [DataTableColumn<'a>],
}

impl<'a> DataTableControls<'a> {
    pub const fn new(filter_id: u32, columns: &'a [DataTableColumn<'a>]) -> Self {
        Self {
            title: "",
            filter_label: "Filter",
            filter_value: "",
            filter_id,
            clear_filter_id: None,
            columns,
        }
    }

    pub const fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub const fn filter_label(mut self, filter_label: &'a str) -> Self {
        self.filter_label = filter_label;
        self
    }

    pub const fn filter_value(mut self, filter_value: &'a str) -> Self {
        self.filter_value = filter_value;
        self
    }

    pub const fn clear_filter_action(mut self, id: u32) -> Self {
        self.clear_filter_id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconOnlyButton<'a> {
    pub icon: UiIcon,
    pub id: u32,
    pub label: &'a str,
    pub active: bool,
}

impl<'a> IconOnlyButton<'a> {
    pub const fn new(icon: UiIcon, id: u32, label: &'a str) -> Self {
        Self {
            icon,
            id,
            label,
            active: true,
        }
    }

    pub const fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentedControlItem<'a> {
    pub label: &'a str,
    pub id: u32,
}

impl<'a> SegmentedControlItem<'a> {
    pub const fn new(label: &'a str, id: u32) -> Self {
        Self { label, id }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentedControl<'a> {
    pub items: &'a [SegmentedControlItem<'a>],
    pub selected: usize,
}

impl<'a> SegmentedControl<'a> {
    pub const fn new(items: &'a [SegmentedControlItem<'a>], selected: usize) -> Self {
        Self { items, selected }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptPaymentStatus {
    Payable,
    Pending,
    Challenged,
    Settled,
}

impl ReceiptPaymentStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Payable => "payable",
            Self::Pending => "pending",
            Self::Challenged => "challenged",
            Self::Settled => "settled",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReceiptPaymentRow<'a> {
    pub label: &'a str,
    pub receipt_id: &'a str,
    pub policy_hash: &'a str,
    pub amount: &'a str,
    pub status: ReceiptPaymentStatus,
    pub id: u32,
    pub action: Option<(&'a str, u32)>,
}

impl<'a> ReceiptPaymentRow<'a> {
    pub const fn new(
        label: &'a str,
        amount: &'a str,
        status: ReceiptPaymentStatus,
        id: u32,
    ) -> Self {
        Self {
            label,
            receipt_id: "",
            policy_hash: "",
            amount,
            status,
            id,
            action: None,
        }
    }

    pub const fn receipt_id(mut self, receipt_id: &'a str) -> Self {
        self.receipt_id = receipt_id;
        self
    }

    pub const fn policy_hash(mut self, policy_hash: &'a str) -> Self {
        self.policy_hash = policy_hash;
        self
    }

    pub const fn action(mut self, label: &'a str, id: u32) -> Self {
        self.action = Some((label, id));
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapabilityGrantRow<'a> {
    pub app: &'a str,
    pub capability: &'a str,
    pub scope: &'a str,
    pub policy_hash: &'a str,
    pub expiry: &'a str,
    pub status: &'a str,
    pub id: u32,
    pub revoke_id: Option<u32>,
}

impl<'a> CapabilityGrantRow<'a> {
    pub const fn new(app: &'a str, capability: &'a str, id: u32) -> Self {
        Self {
            app,
            capability,
            scope: "",
            policy_hash: "",
            expiry: "",
            status: "granted",
            id,
            revoke_id: None,
        }
    }

    pub const fn scope(mut self, scope: &'a str) -> Self {
        self.scope = scope;
        self
    }

    pub const fn policy_hash(mut self, policy_hash: &'a str) -> Self {
        self.policy_hash = policy_hash;
        self
    }

    pub const fn expiry(mut self, expiry: &'a str) -> Self {
        self.expiry = expiry;
        self
    }

    pub const fn status(mut self, status: &'a str) -> Self {
        self.status = status;
        self
    }

    pub const fn revoke_action(mut self, id: u32) -> Self {
        self.revoke_id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemSurfaceKind {
    Storage,
    Proof,
    Admission,
    App,
}

impl SystemSurfaceKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Storage => "storage",
            Self::Proof => "proof",
            Self::Admission => "admission",
            Self::App => "app",
        }
    }

    pub const fn icon(self) -> UiIcon {
        match self {
            Self::Storage => UiIcon::Storage,
            Self::Proof => UiIcon::Shield,
            Self::Admission => UiIcon::Route,
            Self::App => UiIcon::App,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemSurfaceState {
    Empty,
    Error,
}

impl SystemSurfaceState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SystemSurfaceStatePanel<'a> {
    pub kind: SystemSurfaceKind,
    pub state: SystemSurfaceState,
    pub title: &'a str,
    pub detail: &'a str,
    pub reference: &'a str,
    pub id: u32,
    pub action: Option<(&'a str, u32)>,
}

impl<'a> SystemSurfaceStatePanel<'a> {
    pub const fn new(kind: SystemSurfaceKind, state: SystemSurfaceState, id: u32) -> Self {
        Self {
            kind,
            state,
            title: "",
            detail: "",
            reference: "",
            id,
            action: None,
        }
    }

    pub const fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub const fn detail(mut self, detail: &'a str) -> Self {
        self.detail = detail;
        self
    }

    pub const fn reference(mut self, reference: &'a str) -> Self {
        self.reference = reference;
        self
    }

    pub const fn action(mut self, label: &'a str, id: u32) -> Self {
        self.action = Some((label, id));
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiGrid {
    pub rect: UiRect,
    pub columns: u16,
    pub gap: f32,
}

impl UiGrid {
    pub const fn new(rect: UiRect, columns: u16, gap: f32) -> Self {
        Self { rect, columns, gap }
    }

    pub fn cell(&self, column: u16, span: u16, y: f32, height: f32) -> UiRect {
        let columns_u = self.columns.max(1);
        let column_u = column.min(columns_u - 1);
        let span_u = span.max(1).min(columns_u - column_u);
        let columns = columns_u as f32;
        let column = column_u as f32;
        let span = span_u as f32;
        let track = (self.rect.w - self.gap * (columns - 1.0)).max(0.0) / columns;
        UiRect::new(
            self.rect.x + column * (track + self.gap),
            self.rect.y + y,
            track * span + self.gap * (span - 1.0),
            height,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiStack {
    pub rect: UiRect,
    pub axis: Axis,
    pub gap: f32,
    cursor: f32,
}

impl UiStack {
    pub const fn vertical(rect: UiRect, gap: f32) -> Self {
        Self {
            rect,
            axis: Axis::Vertical,
            gap,
            cursor: rect.y,
        }
    }

    pub const fn horizontal(rect: UiRect, gap: f32) -> Self {
        Self {
            rect,
            axis: Axis::Horizontal,
            gap,
            cursor: rect.x,
        }
    }

    pub fn next(&mut self, size: f32) -> UiRect {
        match self.axis {
            Axis::Vertical => {
                let y = self.cursor;
                self.cursor += size + self.gap;
                UiRect::new(self.rect.x, y, self.rect.w, size)
            }
            Axis::Horizontal => {
                let x = self.cursor;
                self.cursor += size + self.gap;
                UiRect::new(x, self.rect.y, size, self.rect.h)
            }
        }
    }

    pub fn remaining(&self) -> UiRect {
        match self.axis {
            Axis::Vertical => UiRect::new(
                self.rect.x,
                self.cursor,
                self.rect.w,
                (self.rect.y + self.rect.h - self.cursor).max(0.0),
            ),
            Axis::Horizontal => UiRect::new(
                self.cursor,
                self.rect.y,
                (self.rect.x + self.rect.w - self.cursor).max(0.0),
                self.rect.h,
            ),
        }
    }
}

pub fn panel_header(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: PanelHeader<'_>) {
    let colors = ui.theme().colors;
    ui.bounded_label(
        rect.x,
        rect.y,
        action_reserved_width(rect, spec.action),
        spec.title,
        2.0,
        colors.text,
    );
    if !spec.subtitle.is_empty() {
        ui.bounded_label(
            rect.x,
            rect.y + 22.0,
            action_reserved_width(rect, spec.action),
            spec.subtitle,
            2.0,
            colors.muted,
        );
    }
    if let Some((label, id)) = spec.action {
        let w = (label.chars().count() as f32 * 9.0 + 26.0).clamp(72.0, 136.0);
        ui.button(
            UiRect::new(rect.x + rect.w - w, rect.y, w, 32.0),
            label,
            ButtonStyle::Secondary,
            id,
            true,
        );
    }
}

pub fn metric_card(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: MetricCard<'_>) {
    let theme = ui.theme();
    let colors = theme.colors;
    ui.fill_rect(rect, theme.radius.card, colors.row.with_alpha(0.24));
    let pad = spacing::UiComponentPadding::DEFAULT;
    let content = rect.inset(pad.x, pad.y);
    let title_y = content.y;
    let value_y = if rect.h < 118.0 {
        content.y + 25.0
    } else {
        content.y + 34.0
    };
    let detail_y = if rect.h < 118.0 {
        rect.y + rect.h - 24.0
    } else {
        rect.y + rect.h - 30.0
    };
    ui.bounded_label(content.x, title_y, content.w, spec.title, 2.0, colors.muted);
    let value_len = spec.value.chars().count();
    let value_scale = if rect.h < 118.0 || rect.w < 230.0 || value_len > 10 {
        3.0
    } else {
        4.0
    };
    ui.bounded_label(
        content.x,
        value_y,
        content.w,
        spec.value,
        value_scale,
        colors.text,
    );
    if !spec.detail.is_empty() {
        ui.bounded_label(
            content.x,
            detail_y,
            content.w,
            spec.detail,
            2.0,
            colors.muted,
        );
    }
    if let Some(progress) = spec.progress {
        ui.progress_bar(
            UiRect::new(content.x, rect.y + rect.h - 47.0, content.w, 6.0),
            progress,
            resolve_component_accent(ui, spec.accent),
        );
    }
}

pub fn field(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: Field<'_>) {
    let colors = ui.theme().colors;
    ui.bounded_label(rect.x, rect.y, rect.w, spec.label, 2.0, colors.muted);
    let field_rect = UiRect::new(
        rect.x,
        rect.y + spacing::FORM_FIELD_Y,
        rect.w,
        spacing::CONTROL_H,
    );
    if let Some(id) = spec.id {
        ui.hit(
            HitKind::Input,
            id,
            field_rect.x,
            field_rect.y,
            field_rect.w,
            field_rect.h,
        );
    }
    ui.input_field(field_rect, spec.value, spec.focused);
    if !spec.helper.is_empty() {
        ui.bounded_label(
            rect.x,
            rect.y + spacing::FORM_HELPER_Y,
            rect.w,
            spec.helper,
            2.0,
            colors.muted,
        );
    }
}

pub fn text_area(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: TextArea<'_>) {
    let theme = ui.theme();
    let colors = theme.colors;
    let field_rect = if spec.label.is_empty() {
        rect
    } else {
        ui.bounded_label(rect.x, rect.y, rect.w, spec.label, 2.0, colors.muted);
        UiRect::new(
            rect.x,
            rect.y + spacing::TEXT_AREA_RESERVED_Y,
            rect.w,
            rect.h - spacing::TEXT_AREA_RESERVED_Y,
        )
    };
    if let Some(id) = spec.id {
        ui.hit(
            HitKind::TextArea,
            id,
            field_rect.x,
            field_rect.y,
            field_rect.w,
            field_rect.h,
        );
    }
    ui.fill_rect(field_rect, theme.radius.card, colors.row.with_alpha(0.42));
    ui.border_rect(
        field_rect,
        theme.radius.card,
        if spec.focused {
            colors.accent
        } else {
            colors.border.with_alpha(0.58)
        },
    );
    let text = field_rect.inset(spacing::CONTROL_PAD_X, spacing::TEXT_AREA_PAD_Y);
    let text_x = text.x;
    let text_y = text.y;
    let text_w = text.w;
    let max_lines = ((field_rect.h - spacing::TEXT_AREA_RESERVED_Y) / spacing::TEXT_AREA_LINE_H)
        .floor()
        .max(1.0) as usize;
    ui.wrapped_label(
        text_x,
        text_y,
        text_w,
        spec.value,
        max_lines,
        spacing::TEXT_AREA_LINE_H,
        if spec.value.is_empty() {
            colors.muted
        } else {
            colors.text
        },
    );
}

pub fn slider(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: Slider<'_>) {
    let colors = ui.theme().colors;
    ui.hit(HitKind::Slider, spec.id, rect.x, rect.y, rect.w, rect.h);
    ui.bounded_label(rect.x, rect.y, rect.w, spec.label, 2.0, colors.text);
    let track = UiRect::new(rect.x, rect.y + 34.0, rect.w, 4.0);
    ui.progress_bar(track, spec.value, resolve_component_accent(ui, spec.accent));
    let x = track.x + track.w * spec.value.clamp(0.0, 1.0);
    ui.fill_rect(
        UiRect::new(x - 7.0, track.y - 5.0, 14.0, 14.0),
        7.0,
        colors.text,
    );
    ui.border_rect(
        UiRect::new(x - 8.0, track.y - 6.0, 16.0, 16.0),
        8.0,
        colors.bg.with_alpha(0.72),
    );
    if !spec.min_label.is_empty() {
        ui.bounded_label(
            rect.x,
            rect.y + 51.0,
            rect.w * 0.5,
            spec.min_label,
            2.0,
            colors.muted,
        );
    }
    if !spec.max_label.is_empty() {
        ui.bounded_label(
            rect.x + rect.w * 0.5,
            rect.y + 51.0,
            rect.w * 0.5,
            spec.max_label,
            2.0,
            colors.muted,
        );
    }
}

pub fn bar_chart(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: BarChart<'_>) {
    let theme = ui.theme();
    let colors = theme.colors;
    let has_header = !spec.title.is_empty() || !spec.subtitle.is_empty();
    if has_header {
        panel_header(
            ui,
            UiRect::new(rect.x, rect.y, rect.w, 44.0),
            PanelHeader {
                title: spec.title,
                subtitle: spec.subtitle,
                action: None,
            },
        );
    }

    let count = spec.values.len().min(spec.labels.len()).min(12);
    if count == 0 {
        return;
    }
    let top = if has_header { 58.0 } else { 8.0 };
    let chart = UiRect::new(
        rect.x,
        rect.y + top,
        rect.w,
        (rect.h - top - 25.0).max(24.0),
    );
    for row in 0..3 {
        let y = chart.y + chart.h * (row as f32 + 1.0) / 4.0;
        ui.fill_rect(
            UiRect::new(chart.x, y, chart.w, 1.0),
            0.0,
            colors.border.with_alpha(0.24),
        );
    }
    let max = spec.values.iter().copied().fold(0.0_f32, f32::max).max(1.0);
    let gap = 11.0_f32.min(chart.w / count as f32 * 0.30);
    let bar_w = ((chart.w - gap * (count.saturating_sub(1) as f32)) / count as f32).max(3.0);
    let accent = resolve_component_accent(ui, spec.accent);
    for index in 0..count {
        let value = spec.values[index].max(0.0);
        let bar_h = (chart.h * (value / max)).clamp(3.0, chart.h);
        let x = chart.x + index as f32 * (bar_w + gap);
        let y = chart.y + chart.h - bar_h;
        if let Some(base_id) = spec.bar_base_id {
            ui.hit(
                HitKind::Button,
                base_id + index as u32,
                x,
                chart.y,
                bar_w,
                chart.h,
            );
        }
        ui.fill_rect(
            UiRect::new(x, y, bar_w, bar_h),
            2.0,
            accent.with_alpha(0.30),
        );
        ui.fill_rect(
            UiRect::new(x, y, bar_w, 3.0),
            2.0,
            colors.text.with_alpha(0.86),
        );
        ui.bounded_label(
            x,
            chart.y + chart.h + 11.0,
            bar_w + gap,
            spec.labels[index],
            2.0,
            colors.muted,
        );
    }
}

pub fn transaction_row(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: TransactionRow<'_>) {
    let colors = ui.theme().colors;
    ui.hit(
        HitKind::TransactionRow,
        spec.id,
        rect.x,
        rect.y,
        rect.w,
        rect.h,
    );
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
    let icon = spacing::row_icon_slot(rect);
    let text = spacing::row_text_rect(rect, 276.0);
    let amount_color = if spec.positive {
        colors.success
    } else {
        colors.text
    };
    ui.fill_rect(icon, 4.0, colors.row);
    ui.border_rect(icon, 4.0, colors.border.with_alpha(0.68));
    ui.bounded_label(text.x, rect.y + 10.0, text.w, spec.title, 2.0, colors.text);
    ui.bounded_label(
        text.x,
        rect.y + 32.0,
        text.w,
        spec.subtitle,
        2.0,
        colors.muted,
    );
    ui.bounded_label(
        rect.x + rect.w * 0.54,
        rect.y + 22.0,
        rect.w * 0.22,
        spec.date,
        2.0,
        colors.muted,
    );
    ui.bounded_label(
        rect.x + rect.w - 146.0,
        rect.y + 22.0,
        130.0,
        spec.amount,
        2.0,
        amount_color,
    );
}

pub fn menu_item(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: MenuItem<'_>) {
    let theme = ui.theme();
    let colors = theme.colors;
    ui.hit(HitKind::MenuItem, spec.id, rect.x, rect.y, rect.w, rect.h);
    let content = rect.inset_ltrb(spacing::ROW_PAD_X, 0.0, spacing::ROW_PAD_X, 0.0);
    if spec.selected {
        ui.fill_rect(rect, theme.radius.card, colors.active);
        let accent = resolve_component_accent(ui, spec.accent);
        ui.border_rect(rect, theme.radius.card, accent.with_alpha(0.42));
        ui.fill_rect(
            UiRect::new(rect.x, rect.y + 10.0, 3.0, rect.h - 20.0),
            2.0,
            accent,
        );
    }
    ui.bounded_label(
        content.x,
        rect.y
            + if spec.detail.is_empty() {
                (rect.h - 14.0) * 0.5
            } else {
                10.0
            },
        content.w,
        spec.label,
        2.0,
        if spec.selected {
            colors.text
        } else {
            colors.muted
        },
    );
    if !spec.detail.is_empty() {
        ui.bounded_label(
            content.x,
            rect.y + 31.0,
            content.w,
            spec.detail,
            2.0,
            colors.muted,
        );
    }
    if !spec.badge.is_empty() {
        let badge_w = (spec.badge.chars().count() as f32 * 9.0 + 18.0).clamp(34.0, 92.0);
        ui.badge(
            rect.x + rect.w - badge_w - 10.0,
            rect.y + 11.0,
            spec.badge,
            resolve_component_accent(ui, spec.accent),
        );
    }
}

pub fn control_row(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: ControlRow<'_>) {
    let colors = ui.theme().colors;
    if let Some(id) = spec.id {
        ui.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
    }
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);

    let accessory_w = match spec.accessory {
        ControlAccessory::None => 0.0,
        ControlAccessory::Value(value) => {
            (value.chars().count() as f32 * 8.0 + 8.0).clamp(48.0, 150.0)
        }
        ControlAccessory::Badge(label, _) => {
            (label.chars().count() as f32 * 9.0 + 18.0).clamp(34.0, 96.0)
        }
        ControlAccessory::Toggle { .. } => 54.0,
        ControlAccessory::Button { label, .. } => {
            (label.chars().count() as f32 * 9.0 + 26.0).clamp(72.0, 136.0)
        }
    };
    let content = rect.inset_ltrb(spacing::ROW_PAD_X, 0.0, spacing::ROW_PAD_X, 0.0);
    let text_w = (content.w - accessory_w - 12.0).max(0.0);
    ui.bounded_label(
        content.x,
        rect.y
            + if spec.detail.is_empty() {
                (rect.h - 14.0) * 0.5
            } else {
                10.0
            },
        text_w,
        spec.label,
        2.0,
        colors.text,
    );
    if !spec.detail.is_empty() {
        ui.bounded_label(
            content.x,
            rect.y + 31.0,
            text_w,
            spec.detail,
            2.0,
            colors.muted,
        );
    }

    let right = content.x + content.w;
    match spec.accessory {
        ControlAccessory::None => {}
        ControlAccessory::Value(value) => ui.bounded_label(
            right - accessory_w,
            rect.y + 21.0,
            accessory_w,
            value,
            2.0,
            colors.muted,
        ),
        ControlAccessory::Badge(label, color) => {
            ui.badge(right - accessory_w, rect.y + 18.0, label, color);
        }
        ControlAccessory::Toggle { on, id } => {
            ui.toggle(right - 46.0, rect.y + (rect.h - 24.0) * 0.5, on, id);
        }
        ControlAccessory::Button { label, id, style } => {
            ui.button(
                UiRect::new(right - accessory_w, rect.y + 13.0, accessory_w, 32.0),
                label,
                style,
                id,
                true,
            );
        }
    }
}

pub fn network_app_prompt(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: NetworkAppPrompt<'_>) {
    let theme = ui.theme();
    let colors = theme.colors;
    ui.card(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        theme.radius.card,
        colors.panel,
    );

    let pad = spacing::UiComponentPadding::SPACIOUS;
    let content = rect.inset(pad.x, pad.y);
    ui.bounded_label(
        content.x,
        content.y,
        content.w,
        if spec.app_name.is_empty() {
            "Network app"
        } else {
            spec.app_name
        },
        3.0,
        colors.text,
    );
    ui.wrapped_label(
        content.x,
        content.y + 34.0,
        content.w,
        "This app runs from EdgeRun network storage. Your browser node will retrieve signed package bytes, verify hashes, and run locally.",
        3,
        20.0,
        colors.muted,
    );
    ui.wrapped_label(
        content.x,
        content.y + 102.0,
        content.w,
        "Retrieval cost is deterministic from the package size and policy schedule. Cache verified bytes locally to avoid repeated retrieval payments.",
        3,
        20.0,
        colors.muted,
    );

    let meta_y = rect.y + rect.h - 108.0;
    if !spec.package_size.is_empty() {
        ui.bounded_label(
            content.x,
            meta_y,
            content.w * 0.34,
            spec.package_size,
            2.0,
            colors.muted,
        );
    }
    if !spec.retrieval_cost.is_empty() {
        ui.bounded_label(
            content.x + content.w * 0.36,
            meta_y,
            content.w * 0.28,
            spec.retrieval_cost,
            2.0,
            colors.muted,
        );
    }
    if !spec.policy_hash.is_empty() {
        ui.bounded_label(
            content.x,
            meta_y + 22.0,
            content.w,
            spec.policy_hash,
            2.0,
            colors.muted,
        );
    }

    let button_y = rect.y + rect.h - 54.0;
    let gap = spacing::SPACE_3;
    let cancel_w = 82.0;
    let run_w = 102.0;
    let cache_w = 136.0;
    let total_w = cancel_w + run_w + cache_w + gap * 2.0;
    let start_x = content.x + (content.w - total_w).max(0.0);
    ui.button(
        UiRect::new(start_x, button_y, cancel_w, spacing::BUTTON_H_COMPACT),
        "Cancel",
        ButtonStyle::Ghost,
        spec.cancel_id,
        true,
    );
    ui.button(
        UiRect::new(
            start_x + cancel_w + gap,
            button_y,
            run_w,
            spacing::BUTTON_H_COMPACT,
        ),
        "Run once",
        ButtonStyle::Secondary,
        spec.run_once_id,
        true,
    );
    ui.button(
        UiRect::new(
            start_x + cancel_w + run_w + gap * 2.0,
            button_y,
            cache_w,
            spacing::BUTTON_H_COMPACT,
        ),
        "Verify & cache",
        ButtonStyle::Primary,
        spec.verify_cache_id,
        true,
    );
}

pub fn app_store_card(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: AppStoreCard<'_>) {
    let theme = ui.theme();
    let colors = theme.colors;
    ui.hit(
        HitKind::ListRow,
        spec.detail_id,
        rect.x,
        rect.y,
        rect.w,
        rect.h,
    );
    ui.card(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        theme.radius.card,
        colors.panel,
    );

    let pad = spacing::UiComponentPadding::DEFAULT;
    let content = rect.inset(pad.x, pad.y);
    let icon = UiRect::new(
        content.x,
        content.y + 2.0,
        spacing::ROW_ICON,
        spacing::ROW_ICON,
    );
    ui.fill_rect(icon, 6.0, colors.row);
    ui.border_rect(icon, 6.0, colors.border.with_alpha(0.68));
    ui.bounded_label(
        icon.x + icon.w + spacing::ROW_ICON_GAP,
        content.y,
        (content.w - spacing::ROW_TEXT_INSET - 118.0).max(0.0),
        spec.name,
        2.0,
        colors.text,
    );
    let developer_line = if spec.developer.is_empty() {
        spec.release
    } else {
        spec.developer
    };
    ui.bounded_label(
        icon.x + icon.w + spacing::ROW_ICON_GAP,
        content.y + 22.0,
        (content.w - spacing::ROW_TEXT_INSET - 118.0).max(0.0),
        developer_line,
        2.0,
        colors.muted,
    );
    ui.button(
        UiRect::new(
            rect.x + rect.w - pad.x - 92.0,
            content.y + 1.0,
            92.0,
            spacing::BUTTON_H_COMPACT,
        ),
        "Run",
        ButtonStyle::Primary,
        spec.run_id,
        true,
    );

    let meta_y = content.y + 58.0;
    let half = (content.w - spacing::SPACE_5) * 0.5;
    ui.bounded_label(
        content.x,
        meta_y,
        half,
        if spec.package_hash.is_empty() {
            "package hash pending"
        } else {
            spec.package_hash
        },
        2.0,
        colors.muted,
    );
    ui.bounded_label(
        content.x + half + spacing::SPACE_5,
        meta_y,
        half,
        if spec.app_policy_hash.is_empty() {
            "app policy hash pending"
        } else {
            spec.app_policy_hash
        },
        2.0,
        colors.muted,
    );
    ui.badge(
        content.x,
        rect.y + rect.h - pad.y - 28.0,
        if spec.access_mode.is_empty() {
            "access mode pending"
        } else {
            spec.access_mode
        },
        colors.info,
    );
    if !spec.release.is_empty() && spec.developer != spec.release {
        ui.bounded_label(
            content.x + 154.0,
            rect.y + rect.h - pad.y - 22.0,
            (content.w - 154.0).max(0.0),
            spec.release,
            2.0,
            colors.muted,
        );
    }
}

pub fn trust_manager_actions(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: TrustManagerActions) {
    let theme = ui.theme();
    let colors = theme.colors;
    ui.card(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        theme.radius.card,
        colors.panel,
    );

    let content = rect.inset(spacing::COMPONENT_PAD_X, spacing::COMPONENT_PAD_Y);
    ui.bounded_label(
        content.x,
        content.y,
        content.w,
        "Trust Manager actions",
        2.0,
        colors.text,
    );
    ui.bounded_label(
        content.x,
        content.y + 22.0,
        content.w,
        "Identity, network apps, grants, and verified package cache",
        2.0,
        colors.muted,
    );

    let button_y = content.y + 52.0;
    let gap = spacing::SPACE_3;
    let button_h = spacing::BUTTON_H_COMPACT;
    let total_w = 118.0 + 124.0 + 108.0 + 116.0 + gap * 3.0;
    if content.w >= total_w {
        let mut x = content.x;
        ui.button(
            UiRect::new(x, button_y, 118.0, button_h),
            "Open Identity",
            ButtonStyle::Secondary,
            spec.open_identity_id,
            true,
        );
        x += 118.0 + gap;
        ui.button(
            UiRect::new(x, button_y, 124.0, button_h),
            "Open App Store",
            ButtonStyle::Secondary,
            spec.open_app_store_id,
            true,
        );
        x += 124.0 + gap;
        ui.button(
            UiRect::new(x, button_y, 108.0, button_h),
            "Revoke grant",
            ButtonStyle::Danger,
            spec.revoke_grant_id,
            true,
        );
        x += 108.0 + gap;
        ui.button(
            UiRect::new(x, button_y, 116.0, button_h),
            "Remove cache",
            ButtonStyle::Danger,
            spec.remove_cache_id,
            true,
        );
    } else {
        let col_gap = gap;
        let row_gap = spacing::SPACE_3;
        let w = ((content.w - col_gap) * 0.5).max(0.0);
        ui.button(
            UiRect::new(content.x, button_y, w, button_h),
            "Open Identity",
            ButtonStyle::Secondary,
            spec.open_identity_id,
            true,
        );
        ui.button(
            UiRect::new(content.x + w + col_gap, button_y, w, button_h),
            "Open App Store",
            ButtonStyle::Secondary,
            spec.open_app_store_id,
            true,
        );
        ui.button(
            UiRect::new(content.x, button_y + button_h + row_gap, w, button_h),
            "Revoke grant",
            ButtonStyle::Danger,
            spec.revoke_grant_id,
            true,
        );
        ui.button(
            UiRect::new(
                content.x + w + col_gap,
                button_y + button_h + row_gap,
                w,
                button_h,
            ),
            "Remove cache",
            ButtonStyle::Danger,
            spec.remove_cache_id,
            true,
        );
    }
}

pub fn runtime_event_row(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: RuntimeEventRow<'_>) {
    let colors = ui.theme().colors;
    let accent = resolve_component_accent(ui, spec.accent);
    ui.hit(HitKind::ListRow, spec.id, rect.x, rect.y, rect.w, rect.h);
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);

    let icon = spacing::row_icon_slot(rect);
    let badge_w = status_badge_width(spec.status);
    let text = spacing::row_text_rect(rect, badge_w + spacing::SPACE_5);
    ui.fill_rect(icon, 6.0, accent.with_alpha(0.14));
    ui.border_rect(icon, 6.0, accent.with_alpha(0.4));
    ui.icon(icon.inset(7.0, 7.0), UiIcon::Activity, accent);
    ui.bounded_label(text.x, rect.y + 9.0, text.w, spec.title, 2.0, colors.text);

    let second_line = if spec.detail.is_empty() {
        spec.event_hash
    } else {
        spec.detail
    };
    ui.bounded_label(
        text.x,
        rect.y + 31.0,
        text.w,
        second_line,
        2.0,
        colors.muted,
    );
    if !spec.event_hash.is_empty() && !spec.detail.is_empty() {
        ui.bounded_label(
            text.x,
            rect.y + 51.0,
            text.w,
            spec.event_hash,
            2.0,
            colors.muted.with_alpha(0.82),
        );
    }
    ui.badge(
        rect.x + rect.w - spacing::ROW_PAD_X - badge_w,
        rect.y + 18.0,
        spec.status,
        accent,
    );
}

pub fn package_proof_row(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: PackageProofRow<'_>) {
    let colors = ui.theme().colors;
    ui.hit(HitKind::ListRow, spec.id, rect.x, rect.y, rect.w, rect.h);
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);

    let icon = spacing::row_icon_slot(rect);
    let badge_w = status_badge_width(spec.status);
    let release_w = if spec.release.is_empty() { 0.0 } else { 128.0 };
    let text = spacing::row_text_rect(rect, badge_w + release_w + spacing::SPACE_5);
    ui.fill_rect(icon, 6.0, colors.success.with_alpha(0.14));
    ui.border_rect(icon, 6.0, colors.success.with_alpha(0.4));
    ui.icon(icon.inset(7.0, 7.0), UiIcon::Shield, colors.success);

    let title = if spec.developer.is_empty() {
        "Package proof"
    } else {
        spec.developer
    };
    ui.bounded_label(text.x, rect.y + 9.0, text.w, title, 2.0, colors.text);
    ui.bounded_label(
        text.x,
        rect.y + 31.0,
        text.w,
        spec.package_hash,
        2.0,
        colors.muted,
    );
    ui.bounded_label(
        text.x,
        rect.y + 51.0,
        text.w,
        spec.manifest_hash,
        2.0,
        colors.muted.with_alpha(0.82),
    );
    if !spec.release.is_empty() {
        let badge_x = rect.x + rect.w - spacing::ROW_PAD_X - badge_w;
        ui.bounded_label(
            badge_x - release_w - spacing::SPACE_5,
            rect.y + 51.0,
            release_w,
            spec.release,
            2.0,
            colors.muted.with_alpha(0.82),
        );
    }
    ui.badge(
        rect.x + rect.w - spacing::ROW_PAD_X - badge_w,
        rect.y + 18.0,
        spec.status,
        colors.success,
    );
}

pub fn import_sync_source_row(
    ui: &mut UiPainter<'_, '_>,
    rect: UiRect,
    spec: ImportSyncSourceRow<'_>,
) {
    let colors = ui.theme().colors;
    ui.hit(HitKind::ListRow, spec.id, rect.x, rect.y, rect.w, rect.h);
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);

    let button_gap = spacing::SPACE_3;
    let mut trailing_w = status_badge_width(spec.status);
    if spec.sync_id.is_some() {
        trailing_w += 74.0 + button_gap;
    }
    if spec.configure_id.is_some() {
        trailing_w += 96.0 + button_gap;
    }

    let icon = spacing::row_icon_slot(rect);
    let text = spacing::row_text_rect(rect, trailing_w + spacing::SPACE_5);
    ui.fill_rect(icon, 6.0, colors.info.with_alpha(0.14));
    ui.border_rect(icon, 6.0, colors.info.with_alpha(0.4));
    ui.icon(icon.inset(7.0, 7.0), spec.kind.icon(), colors.info);

    let title = if spec.name.is_empty() {
        spec.kind.label()
    } else {
        spec.name
    };
    ui.bounded_label(text.x, rect.y + 9.0, text.w, title, 2.0, colors.text);
    ui.bounded_label(
        text.x,
        rect.y + 31.0,
        text.w,
        spec.detail,
        2.0,
        colors.muted,
    );
    ui.bounded_label(
        text.x,
        rect.y + 51.0,
        text.w,
        spec.policy_hash,
        2.0,
        colors.muted.with_alpha(0.82),
    );

    let mut right = rect.x + rect.w - spacing::ROW_PAD_X;
    if let Some(id) = spec.sync_id {
        right -= 74.0;
        ui.button(
            UiRect::new(right, rect.y + 23.0, 74.0, spacing::BUTTON_H_COMPACT),
            "Sync",
            ButtonStyle::Primary,
            id,
            true,
        );
        right -= button_gap;
    }
    if let Some(id) = spec.configure_id {
        right -= 96.0;
        ui.button(
            UiRect::new(right, rect.y + 23.0, 96.0, spacing::BUTTON_H_COMPACT),
            "Configure",
            ButtonStyle::Secondary,
            id,
            true,
        );
        right -= button_gap;
    }
    let badge_w = status_badge_width(spec.status);
    ui.badge(right - badge_w, rect.y + 18.0, spec.status, colors.info);
}

pub fn publish_from_node_row(
    ui: &mut UiPainter<'_, '_>,
    rect: UiRect,
    spec: PublishFromNodeRow<'_>,
) {
    let colors = ui.theme().colors;
    ui.hit(HitKind::ListRow, spec.id, rect.x, rect.y, rect.w, rect.h);
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);

    let button_gap = spacing::SPACE_3;
    let mut trailing_w = status_badge_width(spec.status);
    if spec.publish_id.is_some() {
        trailing_w += 82.0 + button_gap;
    }
    if spec.configure_id.is_some() {
        trailing_w += 96.0 + button_gap;
    }
    if !spec.budget.is_empty() {
        trailing_w += 92.0 + button_gap;
    }

    let icon = spacing::row_icon_slot(rect);
    let text = spacing::row_text_rect(rect, trailing_w + spacing::SPACE_5);
    ui.fill_rect(icon, 6.0, colors.accent.with_alpha(0.14));
    ui.border_rect(icon, 6.0, colors.accent.with_alpha(0.4));
    ui.icon(icon.inset(7.0, 7.0), spec.kind.icon(), colors.accent);

    let title = if spec.title.is_empty() {
        spec.kind.label()
    } else {
        spec.title
    };
    ui.bounded_label(text.x, rect.y + 9.0, text.w, title, 2.0, colors.text);
    ui.bounded_label(
        text.x,
        rect.y + 31.0,
        text.w,
        spec.node_instance,
        2.0,
        colors.muted,
    );
    let proof_line = if spec.route_scope.is_empty() {
        spec.policy_hash
    } else {
        spec.route_scope
    };
    ui.bounded_label(
        text.x,
        rect.y + 51.0,
        text.w,
        proof_line,
        2.0,
        colors.muted.with_alpha(0.82),
    );

    let mut right = rect.x + rect.w - spacing::ROW_PAD_X;
    if let Some(id) = spec.publish_id {
        right -= 82.0;
        ui.button(
            UiRect::new(right, rect.y + 23.0, 82.0, spacing::BUTTON_H_COMPACT),
            "Publish",
            ButtonStyle::Primary,
            id,
            true,
        );
        right -= button_gap;
    }
    if let Some(id) = spec.configure_id {
        right -= 96.0;
        ui.button(
            UiRect::new(right, rect.y + 23.0, 96.0, spacing::BUTTON_H_COMPACT),
            "Configure",
            ButtonStyle::Secondary,
            id,
            true,
        );
        right -= button_gap;
    }
    if !spec.budget.is_empty() {
        right -= 92.0;
        ui.bounded_label(
            right,
            rect.y + 51.0,
            92.0,
            spec.budget,
            2.0,
            colors.muted.with_alpha(0.82),
        );
        right -= button_gap;
    }
    let badge_w = status_badge_width(spec.status);
    ui.badge(right - badge_w, rect.y + 18.0, spec.status, colors.accent);
}

pub fn node_instance_row(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: NodeInstanceRow<'_>) {
    let colors = ui.theme().colors;
    ui.hit(HitKind::ListRow, spec.id, rect.x, rect.y, rect.w, rect.h);
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);

    let trailing_w = status_badge_width(spec.status)
        + if spec.open_id.is_some() {
            78.0 + spacing::SPACE_3
        } else {
            0.0
        };
    let icon = spacing::row_icon_slot(rect);
    let text = spacing::row_text_rect(rect, trailing_w + spacing::SPACE_5);
    ui.fill_rect(icon, 6.0, colors.info.with_alpha(0.14));
    ui.border_rect(icon, 6.0, colors.info.with_alpha(0.4));
    ui.icon(icon.inset(7.0, 7.0), UiIcon::Server, colors.info);
    ui.bounded_label(text.x, rect.y + 9.0, text.w, spec.role, 2.0, colors.text);
    ui.bounded_label(
        text.x,
        rect.y + 31.0,
        text.w,
        spec.runtime_target,
        2.0,
        colors.muted,
    );
    let detail = if spec.route_scope.is_empty() {
        spec.policy_hash
    } else {
        spec.route_scope
    };
    ui.bounded_label(
        text.x,
        rect.y + 51.0,
        text.w,
        detail,
        2.0,
        colors.muted.with_alpha(0.82),
    );
    let mut right = rect.x + rect.w - spacing::ROW_PAD_X;
    if let Some(id) = spec.open_id {
        right -= 78.0;
        ui.button(
            UiRect::new(right, rect.y + 23.0, 78.0, spacing::BUTTON_H_COMPACT),
            "Open",
            ButtonStyle::Secondary,
            id,
            true,
        );
        right -= spacing::SPACE_3;
    }
    let badge_w = status_badge_width(spec.status);
    ui.badge(right - badge_w, rect.y + 18.0, spec.status, colors.info);
}

pub fn admission_policy_row(
    ui: &mut UiPainter<'_, '_>,
    rect: UiRect,
    spec: AdmissionPolicyRow<'_>,
) {
    let colors = ui.theme().colors;
    ui.hit(HitKind::ListRow, spec.id, rect.x, rect.y, rect.w, rect.h);
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);

    let trailing_w = status_badge_width(spec.status)
        + if spec.inspect_id.is_some() {
            86.0 + spacing::SPACE_3
        } else {
            0.0
        };
    let icon = spacing::row_icon_slot(rect);
    let text = spacing::row_text_rect(rect, trailing_w + spacing::SPACE_5);
    ui.fill_rect(icon, 6.0, colors.success.with_alpha(0.14));
    ui.border_rect(icon, 6.0, colors.success.with_alpha(0.4));
    ui.icon(icon.inset(7.0, 7.0), UiIcon::Shield, colors.success);
    ui.bounded_label(text.x, rect.y + 9.0, text.w, spec.source, 2.0, colors.text);
    ui.bounded_label(
        text.x,
        rect.y + 31.0,
        text.w,
        spec.policy_hash,
        2.0,
        colors.muted,
    );
    let detail = if spec.admission_node.is_empty() {
        spec.validity
    } else {
        spec.admission_node
    };
    ui.bounded_label(
        text.x,
        rect.y + 51.0,
        text.w,
        detail,
        2.0,
        colors.muted.with_alpha(0.82),
    );
    let mut right = rect.x + rect.w - spacing::ROW_PAD_X;
    if let Some(id) = spec.inspect_id {
        right -= 86.0;
        ui.button(
            UiRect::new(right, rect.y + 23.0, 86.0, spacing::BUTTON_H_COMPACT),
            "Inspect",
            ButtonStyle::Secondary,
            id,
            true,
        );
        right -= spacing::SPACE_3;
    }
    let badge_w = status_badge_width(spec.status);
    ui.badge(right - badge_w, rect.y + 18.0, spec.status, colors.success);
}

pub fn route_budget_row(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: RouteBudgetRow<'_>) {
    let colors = ui.theme().colors;
    ui.hit(HitKind::ListRow, spec.id, rect.x, rect.y, rect.w, rect.h);
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);

    let trailing_w = status_badge_width(spec.status)
        + 112.0
        + if spec.inspect_id.is_some() {
            86.0 + spacing::SPACE_3
        } else {
            0.0
        };
    let icon = spacing::row_icon_slot(rect);
    let text = spacing::row_text_rect(rect, trailing_w + spacing::SPACE_5);
    ui.fill_rect(icon, 6.0, colors.accent.with_alpha(0.14));
    ui.border_rect(icon, 6.0, colors.accent.with_alpha(0.4));
    ui.icon(icon.inset(7.0, 7.0), UiIcon::Route, colors.accent);
    ui.bounded_label(text.x, rect.y + 9.0, text.w, spec.route, 2.0, colors.text);
    ui.bounded_label(
        text.x,
        rect.y + 31.0,
        text.w,
        spec.channel,
        2.0,
        colors.muted,
    );
    ui.bounded_label(
        text.x,
        rect.y + 51.0,
        text.w,
        spec.admitted_budget,
        2.0,
        colors.muted.with_alpha(0.82),
    );
    let mut right = rect.x + rect.w - spacing::ROW_PAD_X;
    if let Some(id) = spec.inspect_id {
        right -= 86.0;
        ui.button(
            UiRect::new(right, rect.y + 23.0, 86.0, spacing::BUTTON_H_COMPACT),
            "Inspect",
            ButtonStyle::Secondary,
            id,
            true,
        );
        right -= spacing::SPACE_3;
    }
    right -= 112.0;
    ui.bounded_label(
        right,
        rect.y + 51.0,
        112.0,
        spec.spent,
        2.0,
        colors.muted.with_alpha(0.82),
    );
    right -= spacing::SPACE_3;
    let badge_w = status_badge_width(spec.status);
    ui.badge(right - badge_w, rect.y + 18.0, spec.status, colors.accent);
}

pub fn data_table_controls(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: DataTableControls<'_>) {
    let colors = ui.theme().colors;
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);

    let content = rect.inset_ltrb(
        spacing::ROW_PAD_X,
        spacing::SPACE_3,
        spacing::ROW_PAD_X,
        spacing::SPACE_3,
    );
    let title_w = if spec.title.is_empty() {
        0.0
    } else {
        (spec.title.chars().count() as f32 * 9.0 + spacing::SPACE_7).clamp(92.0, 180.0)
    };
    if !spec.title.is_empty() {
        ui.bounded_label(
            content.x,
            content.y + 10.0,
            title_w,
            spec.title,
            2.0,
            colors.text,
        );
    }

    let clear_w = if spec.clear_filter_id.is_some() {
        72.0 + spacing::SPACE_3
    } else {
        0.0
    };
    let column_count = spec.columns.len().clamp(1, 6);
    let header_gap = spacing::SPACE_3;
    let header_w = (content.w * 0.45).clamp(180.0, 420.0);
    let filter_x = content.x + title_w + if title_w > 0.0 { spacing::SPACE_5 } else { 0.0 };
    let filter_w = (content.w - title_w - header_w - clear_w - spacing::SPACE_8).max(96.0);
    let filter = UiRect::new(
        filter_x,
        content.y + 2.0,
        filter_w,
        spacing::BUTTON_H_COMPACT,
    );
    ui.hit(
        HitKind::Input,
        spec.filter_id,
        filter.x,
        filter.y,
        filter.w,
        filter.h,
    );
    ui.fill_rect(filter, 6.0, colors.row);
    ui.border_rect(filter, 6.0, colors.border.with_alpha(0.82));
    ui.icon(
        UiRect::new(filter.x + 9.0, filter.y + 8.0, 16.0, 16.0),
        UiIcon::Search,
        colors.muted,
    );
    let filter_text = if spec.filter_value.is_empty() {
        spec.filter_label
    } else {
        spec.filter_value
    };
    ui.bounded_label(
        filter.x + 32.0,
        filter.y + 9.0,
        (filter.w - 42.0).max(0.0),
        filter_text,
        2.0,
        if spec.filter_value.is_empty() {
            colors.muted
        } else {
            colors.text
        },
    );

    let mut right = content.x + content.w;
    if let Some(id) = spec.clear_filter_id {
        right -= 72.0;
        ui.button(
            UiRect::new(right, content.y + 2.0, 72.0, spacing::BUTTON_H_COMPACT),
            "Clear",
            ButtonStyle::Secondary,
            id,
            true,
        );
        right -= spacing::SPACE_3;
    }

    let total_gap = header_gap * column_count.saturating_sub(1) as f32;
    let column_w = ((right - filter.x - filter.w - spacing::SPACE_5 - total_gap)
        / column_count as f32)
        .max(42.0);
    let mut x = right - column_w * column_count as f32 - total_gap;
    for column in spec.columns.iter().take(column_count) {
        let cell = UiRect::new(x, content.y + 2.0, column_w, spacing::BUTTON_H_COMPACT);
        if let Some(id) = column.sort_id {
            ui.hit(HitKind::Button, id, cell.x, cell.y, cell.w, cell.h);
        }
        ui.fill_rect(cell, 6.0, colors.row);
        ui.border_rect(cell, 6.0, colors.border.with_alpha(0.62));
        ui.bounded_label(
            cell.x + spacing::SPACE_3,
            cell.y + 9.0,
            (cell.w - 48.0).max(0.0),
            column.label,
            2.0,
            colors.text,
        );
        ui.bounded_label(
            cell.x + cell.w - 40.0,
            cell.y + 9.0,
            34.0,
            column.sort.label(),
            2.0,
            if column.sort == DataTableSort::None {
                colors.muted
            } else {
                colors.accent
            },
        );
        x += column_w + header_gap;
    }
}

pub fn icon_only_button(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: IconOnlyButton<'_>) {
    ui.icon_button(rect, spec.icon, spec.id, spec.active);
}

pub fn segmented_control(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: SegmentedControl<'_>) {
    if spec.items.is_empty() {
        return;
    }
    let theme = ui.theme();
    let colors = theme.colors;
    let radius = theme.radius.control;
    ui.fill_rect(rect, radius, colors.row);
    ui.border_rect(rect, radius, colors.border);

    let count = spec.items.len();
    let item_w = rect.w / count as f32;
    for (index, item) in spec.items.iter().enumerate() {
        let item_rect = UiRect::new(rect.x + index as f32 * item_w, rect.y, item_w, rect.h);
        ui.hit(
            HitKind::Tab,
            item.id,
            item_rect.x,
            item_rect.y,
            item_rect.w,
            item_rect.h,
        );
        let selected = index == spec.selected.min(count - 1);
        if selected {
            ui.fill_rect(item_rect.inset(3.0, 3.0), radius.min(9.0), colors.active);
        }
        ui.bounded_label(
            item_rect.x + spacing::SPACE_3,
            item_rect.y + ((item_rect.h - 14.0) * 0.5),
            (item_rect.w - spacing::SPACE_6).max(0.0),
            item.label,
            2.0,
            if selected { colors.text } else { colors.muted },
        );
    }
}

pub fn receipt_payment_row(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: ReceiptPaymentRow<'_>) {
    let colors = ui.theme().colors;
    let status_color = match spec.status {
        ReceiptPaymentStatus::Payable => colors.accent,
        ReceiptPaymentStatus::Pending => colors.info,
        ReceiptPaymentStatus::Challenged => colors.danger,
        ReceiptPaymentStatus::Settled => colors.success,
    };
    ui.hit(
        HitKind::TransactionRow,
        spec.id,
        rect.x,
        rect.y,
        rect.w,
        rect.h,
    );
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);

    let action_w = spec
        .action
        .map(|(label, _)| (label.chars().count() as f32 * 9.0 + 26.0).clamp(72.0, 118.0))
        .unwrap_or(0.0);
    let badge_w = status_badge_width(spec.status.label());
    let amount_w = 112.0;
    let trailing_w = badge_w
        + amount_w
        + spacing::SPACE_5
        + if action_w > 0.0 {
            action_w + spacing::SPACE_3
        } else {
            0.0
        };
    let icon = spacing::row_icon_slot(rect);
    let text = spacing::row_text_rect(rect, trailing_w + spacing::SPACE_5);
    ui.fill_rect(icon, 6.0, status_color.with_alpha(0.14));
    ui.border_rect(icon, 6.0, status_color.with_alpha(0.4));
    ui.icon(icon.inset(7.0, 7.0), UiIcon::Wallet, status_color);
    ui.bounded_label(text.x, rect.y + 9.0, text.w, spec.label, 2.0, colors.text);
    ui.bounded_label(
        text.x,
        rect.y + 31.0,
        text.w,
        spec.receipt_id,
        2.0,
        colors.muted,
    );
    ui.bounded_label(
        text.x,
        rect.y + 51.0,
        text.w,
        spec.policy_hash,
        2.0,
        colors.muted.with_alpha(0.82),
    );

    let mut right = rect.x + rect.w - spacing::ROW_PAD_X;
    if let Some((label, id)) = spec.action {
        right -= action_w;
        ui.button(
            UiRect::new(right, rect.y + 23.0, action_w, spacing::BUTTON_H_COMPACT),
            label,
            if spec.status == ReceiptPaymentStatus::Challenged {
                ButtonStyle::Danger
            } else {
                ButtonStyle::Secondary
            },
            id,
            true,
        );
        right -= spacing::SPACE_3;
    }
    right -= amount_w;
    ui.bounded_label(
        right,
        rect.y + 31.0,
        amount_w,
        spec.amount,
        2.0,
        colors.text,
    );
    right -= spacing::SPACE_5;
    ui.badge(
        right - badge_w,
        rect.y + 18.0,
        spec.status.label(),
        status_color,
    );
}

pub fn capability_grant_detail_row(
    ui: &mut UiPainter<'_, '_>,
    rect: UiRect,
    spec: CapabilityGrantRow<'_>,
) {
    let colors = ui.theme().colors;
    ui.hit(HitKind::ListRow, spec.id, rect.x, rect.y, rect.w, rect.h);
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);

    let revoke_w = if spec.revoke_id.is_some() { 86.0 } else { 0.0 };
    let expiry_w = if spec.expiry.is_empty() { 0.0 } else { 112.0 };
    let badge_w = status_badge_width(spec.status);
    let trailing_w = badge_w
        + expiry_w
        + if expiry_w > 0.0 {
            spacing::SPACE_5
        } else {
            0.0
        }
        + if revoke_w > 0.0 {
            revoke_w + spacing::SPACE_3
        } else {
            0.0
        };
    let icon = spacing::row_icon_slot(rect);
    let text = spacing::row_text_rect(rect, trailing_w + spacing::SPACE_5);
    ui.fill_rect(icon, 6.0, colors.info.with_alpha(0.14));
    ui.border_rect(icon, 6.0, colors.info.with_alpha(0.4));
    ui.icon(icon.inset(7.0, 7.0), UiIcon::Key, colors.info);

    ui.bounded_label(text.x, rect.y + 9.0, text.w, spec.app, 2.0, colors.text);
    ui.bounded_label(
        text.x,
        rect.y + 31.0,
        text.w,
        spec.capability,
        2.0,
        colors.muted,
    );
    let proof_line = if spec.scope.is_empty() {
        spec.policy_hash
    } else {
        spec.scope
    };
    ui.bounded_label(
        text.x,
        rect.y + 51.0,
        text.w,
        proof_line,
        2.0,
        colors.muted.with_alpha(0.82),
    );

    let mut right = rect.x + rect.w - spacing::ROW_PAD_X;
    if let Some(id) = spec.revoke_id {
        right -= revoke_w;
        ui.button(
            UiRect::new(right, rect.y + 23.0, revoke_w, spacing::BUTTON_H_COMPACT),
            "Revoke",
            ButtonStyle::Danger,
            id,
            true,
        );
        right -= spacing::SPACE_3;
    }
    if !spec.expiry.is_empty() {
        right -= expiry_w;
        ui.bounded_label(
            right,
            rect.y + 51.0,
            expiry_w,
            spec.expiry,
            2.0,
            colors.muted.with_alpha(0.82),
        );
        right -= spacing::SPACE_5;
    }
    ui.badge(right - badge_w, rect.y + 18.0, spec.status, colors.info);
}

pub fn system_surface_state_panel(
    ui: &mut UiPainter<'_, '_>,
    rect: UiRect,
    spec: SystemSurfaceStatePanel<'_>,
) {
    let theme = ui.theme();
    let colors = theme.colors;
    let accent = match spec.state {
        SystemSurfaceState::Empty => colors.info,
        SystemSurfaceState::Error => colors.danger,
    };
    ui.hit(HitKind::ListRow, spec.id, rect.x, rect.y, rect.w, rect.h);
    ui.card(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        theme.radius.card,
        colors.panel,
    );
    ui.border_rect(rect, theme.radius.card, accent.with_alpha(0.34));

    let pad = spacing::UiComponentPadding::SPACIOUS;
    let content = rect.inset(pad.x, pad.y);
    let icon = UiRect::new(content.x, content.y + 2.0, 42.0, 42.0);
    ui.fill_rect(icon, 8.0, accent.with_alpha(0.14));
    ui.border_rect(icon, 8.0, accent.with_alpha(0.4));
    ui.icon(icon.inset(9.0, 9.0), spec.kind.icon(), accent);

    let badge_label = spec.state.label();
    let badge_w = status_badge_width(badge_label);
    let action_w = spec
        .action
        .map(|(label, _)| (label.chars().count() as f32 * 9.0 + 26.0).clamp(72.0, 138.0))
        .unwrap_or(0.0);
    let text_x = icon.x + icon.w + spacing::SPACE_5;
    let trailing = badge_w
        + if action_w > 0.0 {
            action_w + spacing::SPACE_3
        } else {
            0.0
        };
    let text_w = (content.x + content.w - text_x - trailing - spacing::SPACE_5).max(0.0);
    let title = if spec.title.is_empty() {
        spec.kind.label()
    } else {
        spec.title
    };
    ui.bounded_label(text_x, content.y, text_w, title, 2.0, colors.text);
    ui.bounded_label(
        text_x,
        content.y + 24.0,
        text_w,
        spec.detail,
        2.0,
        colors.muted,
    );
    ui.bounded_label(
        text_x,
        content.y + 48.0,
        text_w,
        spec.reference,
        2.0,
        colors.muted.with_alpha(0.82),
    );

    let mut right = content.x + content.w;
    if let Some((label, id)) = spec.action {
        right -= action_w;
        ui.button(
            UiRect::new(right, content.y + 18.0, action_w, spacing::BUTTON_H_COMPACT),
            label,
            if spec.state == SystemSurfaceState::Error {
                ButtonStyle::Danger
            } else {
                ButtonStyle::Secondary
            },
            id,
            true,
        );
        right -= spacing::SPACE_3;
    }
    ui.badge(right - badge_w, content.y + 23.0, badge_label, accent);
}

fn action_reserved_width(rect: UiRect, action: Option<(&str, u32)>) -> f32 {
    if action.is_some() {
        (rect.w - 156.0).max(0.0)
    } else {
        rect.w
    }
}

fn status_badge_width(label: &str) -> f32 {
    (label.chars().count() as f32 * 9.0 + 18.0).clamp(54.0, 128.0)
}

fn resolve_component_accent(ui: &UiPainter<'_, '_>, accent: Color4) -> Color4 {
    if accent == palette::ACCENT {
        ui.theme().colors.accent
    } else {
        accent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::{GpuScene, RectMode, palette};

    #[test]
    fn stack_lays_out_vertical_children() {
        let mut stack = UiStack::vertical(UiRect::new(10.0, 20.0, 300.0, 400.0), 8.0);

        assert_eq!(stack.next(40.0), UiRect::new(10.0, 20.0, 300.0, 40.0));
        assert_eq!(stack.next(60.0), UiRect::new(10.0, 68.0, 300.0, 60.0));
        assert_eq!(stack.remaining(), UiRect::new(10.0, 136.0, 300.0, 284.0));
    }

    #[test]
    fn grid_returns_predictable_dashboard_cells() {
        let grid = UiGrid::new(UiRect::new(20.0, 40.0, 1000.0, 800.0), 4, 16.0);

        assert_eq!(
            grid.cell(0, 1, 0.0, 120.0),
            UiRect::new(20.0, 40.0, 238.0, 120.0)
        );
        assert_eq!(
            grid.cell(1, 2, 140.0, 240.0),
            UiRect::new(274.0, 180.0, 492.0, 240.0)
        );
    }

    #[test]
    fn dashboard_components_render_into_gpu_scene() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            metric_card(
                &mut ui,
                UiRect::new(16.0, 16.0, 240.0, 132.0),
                MetricCard::new("Balance", "$0.00")
                    .detail("pending setup")
                    .progress(0.42),
            );
            field(
                &mut ui,
                UiRect::new(280.0, 16.0, 260.0, 92.0),
                Field::new("Endpoint")
                    .value("nodes.edgerun.tech")
                    .focused(true)
                    .id(6),
            );
            slider(
                &mut ui,
                UiRect::new(280.0, 126.0, 260.0, 78.0),
                Slider::new("Budget", 0.64, 7).range_labels("$0", "$100"),
            );
            let labels = ["Jan", "Feb", "Mar"];
            let values = [0.4, 0.9, 0.62];
            bar_chart(
                &mut ui,
                UiRect::new(16.0, 170.0, 240.0, 180.0),
                BarChart::new("Activity", &labels, &values),
            );
            transaction_row(
                &mut ui,
                UiRect::new(280.0, 230.0, 320.0, 58.0),
                TransactionRow::new("Relay receipt", "+$4.20", 8)
                    .subtitle("Income")
                    .date("Today")
                    .positive(true),
            );
            menu_item(
                &mut ui,
                UiRect::new(620.0, 16.0, 220.0, 58.0),
                MenuItem::new("Payments", 9)
                    .detail("proof-backed receipts")
                    .badge("new")
                    .selected(true),
            );
            control_row(
                &mut ui,
                UiRect::new(620.0, 92.0, 300.0, 58.0),
                ControlRow::new("Admission policy")
                    .detail("personal relay budget")
                    .toggle(true, 10)
                    .id(11),
            );
        }

        assert!(scene.rects().len() > 20);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Input && hit.id == 6)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Slider && hit.id == 7)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::TransactionRow && hit.id == 8)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::MenuItem && hit.id == 9)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Toggle && hit.id == 10)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 11)
        );
    }

    #[test]
    fn row_accessories_keep_fixed_hit_geometry_when_labels_change() {
        let row = UiRect::new(20.0, 20.0, 360.0, spacing::ROW_H);
        let mut short_scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut short_scene);
            control_row(
                &mut ui,
                row,
                ControlRow::new("Mode")
                    .detail("short")
                    .toggle(true, 51)
                    .id(50),
            );
        }
        let mut long_scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut long_scene);
            control_row(
                &mut ui,
                row,
                ControlRow::new("Extremely long admission policy mode label")
                    .detail("long value that must truncate instead of shifting controls")
                    .toggle(false, 51)
                    .id(50),
            );
        }

        let short_toggle = short_scene
            .hits()
            .iter()
            .find(|hit| hit.kind == HitKind::Toggle && hit.id == 51)
            .expect("short toggle hit");
        let long_toggle = long_scene
            .hits()
            .iter()
            .find(|hit| hit.kind == HitKind::Toggle && hit.id == 51)
            .expect("long toggle hit");
        assert_eq!(short_toggle.x, long_toggle.x);
        assert_eq!(short_toggle.y, long_toggle.y);
        assert_eq!(short_toggle.w, long_toggle.w);
        assert_eq!(short_toggle.h, long_toggle.h);
    }

    #[test]
    fn network_app_prompt_emits_three_semantic_actions() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            network_app_prompt(
                &mut ui,
                UiRect::new(24.0, 32.0, 520.0, 312.0),
                NetworkAppPrompt::new("Mail from network storage", 101, 102, 103)
                    .package_size("Package 18.4 MB")
                    .retrieval_cost("Cost 184 units")
                    .policy_hash("policy 9f2c...7a10"),
            );
        }

        for id in [101, 102, 103] {
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::Button && hit.id == id),
                "missing prompt button hit {id}"
            );
        }
        assert!(scene.rects().len() >= 7);
    }

    #[test]
    fn app_store_card_exposes_policy_hash_access_mode_and_actions() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            app_store_card(
                &mut ui,
                UiRect::new(24.0, 24.0, 460.0, 148.0),
                AppStoreCard::new("Mail", 301, 302)
                    .developer("Sylchi")
                    .release("release 2026.05")
                    .package_hash("pkg blake3:abc123")
                    .app_policy_hash("policy blake3:def456")
                    .access_mode("Verify & cache"),
            );
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 301)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 302)
        );
        assert!(scene.rects().len() >= 6);
    }

    #[test]
    fn trust_manager_actions_emit_canonical_action_hits() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            trust_manager_actions(
                &mut ui,
                UiRect::new(16.0, 16.0, 560.0, 120.0),
                TrustManagerActions::new(201, 202, 203, 204),
            );
        }

        for id in [201, 202, 203, 204] {
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::Button && hit.id == id),
                "missing Trust Manager action hit {id}"
            );
        }
    }

    #[test]
    fn proof_components_emit_shared_list_row_hits() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            runtime_event_row(
                &mut ui,
                UiRect::new(16.0, 16.0, 560.0, 78.0),
                RuntimeEventRow::new("Runtime event", "event blake3:abc123", 401)
                    .detail("package verified")
                    .status("accepted"),
            );
            package_proof_row(
                &mut ui,
                UiRect::new(16.0, 94.0, 560.0, 78.0),
                PackageProofRow::new("pkg blake3:def456", "manifest blake3:fedcba", 402)
                    .developer("Sylchi")
                    .release("release 2026.05")
                    .status("verified"),
            );
        }

        for id in [401, 402] {
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::ListRow && hit.id == id),
                "missing proof component row hit {id}"
            );
        }
        assert!(scene.rects().len() >= 8);
    }

    #[test]
    fn operational_cards_and_rows_share_spacing_rhythm() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            app_store_card(
                &mut ui,
                UiRect::new(16.0, 16.0, 560.0, spacing::APP_STORE_CARD_H),
                AppStoreCard::new("Network app", 430, 431)
                    .developer("publisher:edgerun")
                    .package_hash("pkg blake3:rhythm")
                    .app_policy_hash("policy blake3:rhythm")
                    .access_mode("Verify & cache"),
            );
            package_proof_row(
                &mut ui,
                UiRect::new(16.0, 170.0, 560.0, spacing::OPERATION_ROW_H),
                PackageProofRow::new("pkg blake3:def456", "manifest blake3:fedcba", 432)
                    .developer("publisher")
                    .status("verified"),
            );
            receipt_payment_row(
                &mut ui,
                UiRect::new(16.0, 248.0, 700.0, spacing::OPERATION_ROW_H),
                ReceiptPaymentRow::new(
                    "Relay receipt",
                    "100 work units",
                    ReceiptPaymentStatus::Payable,
                    433,
                )
                .receipt_id("receipt blake3:abc123")
                .policy_hash("policy blake3:settlement")
                .action("Inspect", 434),
            );
        }

        let app = scene
            .hits()
            .iter()
            .find(|hit| hit.kind == HitKind::ListRow && hit.id == 430)
            .expect("app store card hit");
        let proof = scene
            .hits()
            .iter()
            .find(|hit| hit.kind == HitKind::ListRow && hit.id == 432)
            .expect("package proof row hit");
        let receipt = scene
            .hits()
            .iter()
            .find(|hit| hit.kind == HitKind::TransactionRow && hit.id == 433)
            .expect("receipt row hit");

        assert_eq!(app.h, spacing::APP_STORE_CARD_H);
        assert_eq!(proof.h, spacing::OPERATION_ROW_H);
        assert_eq!(receipt.h, spacing::OPERATION_ROW_H);
    }

    #[test]
    fn import_sync_source_rows_cover_canonical_source_kinds() {
        let kinds = [
            ImportSyncSourceKind::Gmail,
            ImportSyncSourceKind::Drive,
            ImportSyncSourceKind::GitHub,
            ImportSyncSourceKind::Folder,
            ImportSyncSourceKind::Mailbox,
        ];
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            for (index, kind) in kinds.iter().copied().enumerate() {
                import_sync_source_row(
                    &mut ui,
                    UiRect::new(16.0, 16.0 + index as f32 * 78.0, 640.0, 78.0),
                    ImportSyncSourceRow::new(kind, 500 + index as u32)
                        .detail("import/sync source")
                        .policy_hash("policy blake3:source")
                        .status("ready")
                        .configure_action(600 + index as u32)
                        .sync_action(700 + index as u32),
                );
            }
        }

        for index in 0..kinds.len() as u32 {
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::ListRow && hit.id == 500 + index),
                "missing import source row hit {index}"
            );
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::Button && hit.id == 600 + index),
                "missing import source configure hit {index}"
            );
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::Button && hit.id == 700 + index),
                "missing import source sync hit {index}"
            );
        }
    }

    #[test]
    fn publish_from_node_rows_cover_canonical_publish_flows() {
        let kinds = [
            PublishFromNodeKind::Site,
            PublishFromNodeKind::App,
            PublishFromNodeKind::Api,
            PublishFromNodeKind::File,
        ];
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            for (index, kind) in kinds.iter().copied().enumerate() {
                publish_from_node_row(
                    &mut ui,
                    UiRect::new(16.0, 16.0 + index as f32 * 78.0, 700.0, 78.0),
                    PublishFromNodeRow::new(kind, 800 + index as u32)
                        .title(kind.label())
                        .node_instance("publishing:personal-site")
                        .route_scope("identity-routed publishing")
                        .policy_hash("policy blake3:publish")
                        .budget("budget 100")
                        .status("ready")
                        .configure_action(900 + index as u32)
                        .publish_action(1000 + index as u32),
                );
            }
        }

        for index in 0..kinds.len() as u32 {
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::ListRow && hit.id == 800 + index),
                "missing publish row hit {index}"
            );
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::Button && hit.id == 900 + index),
                "missing publish configure hit {index}"
            );
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::Button && hit.id == 1000 + index),
                "missing publish action hit {index}"
            );
        }
    }

    #[test]
    fn domain_rows_cover_node_admission_and_route_budget_surfaces() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            node_instance_row(
                &mut ui,
                UiRect::new(16.0, 16.0, 700.0, 78.0),
                NodeInstanceRow::new("admission:family", 1200)
                    .runtime_target("browser wasm")
                    .route_scope("family devices")
                    .policy_hash("policy blake3:node")
                    .status("ready")
                    .open_action(1201),
            );
            admission_policy_row(
                &mut ui,
                UiRect::new(16.0, 94.0, 700.0, 78.0),
                AdmissionPolicyRow::new("family admission policy", "policy blake3:admit", 1210)
                    .admission_node("admission:family")
                    .validity("valid for 10m")
                    .status("committed")
                    .inspect_action(1211),
            );
            route_budget_row(
                &mut ui,
                UiRect::new(16.0, 172.0, 700.0, 78.0),
                RouteBudgetRow::new("route relay:private-devices", "admitted budget 100", 1220)
                    .channel("channel 42")
                    .spent("spent 18")
                    .status("active")
                    .inspect_action(1221),
            );
        }

        for id in [1200, 1210, 1220] {
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::ListRow && hit.id == id),
                "missing domain row hit {id}"
            );
        }
        for id in [1201, 1211, 1221] {
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::Button && hit.id == id),
                "missing domain row action hit {id}"
            );
        }
    }

    #[test]
    fn data_table_controls_emit_keyboard_accessible_filter_and_sort_hits() {
        let columns = [
            DataTableColumn::new("Object").sort(DataTableSort::Asc, 1301),
            DataTableColumn::new("Hash").sort(DataTableSort::None, 1302),
            DataTableColumn::new("State").sort(DataTableSort::Desc, 1303),
        ];
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            data_table_controls(
                &mut ui,
                UiRect::new(16.0, 16.0, 760.0, 54.0),
                DataTableControls::new(1300, &columns)
                    .title("Objects")
                    .filter_label("Filter objects")
                    .filter_value("policy")
                    .clear_filter_action(1304),
            );
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Input && hit.id == 1300),
            "missing filter input hit"
        );
        for id in [1301, 1302, 1303, 1304] {
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::Button && hit.id == id),
                "missing table control button hit {id}"
            );
        }
    }

    #[test]
    fn receipt_payment_rows_distinguish_payment_states() {
        let states = [
            ReceiptPaymentStatus::Payable,
            ReceiptPaymentStatus::Pending,
            ReceiptPaymentStatus::Challenged,
            ReceiptPaymentStatus::Settled,
        ];
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            for (index, status) in states.iter().copied().enumerate() {
                receipt_payment_row(
                    &mut ui,
                    UiRect::new(16.0, 16.0 + index as f32 * 78.0, 700.0, 78.0),
                    ReceiptPaymentRow::new(
                        "Relay receipt",
                        "100 work units",
                        status,
                        1500 + index as u32,
                    )
                    .receipt_id("receipt blake3:abc123")
                    .policy_hash("policy blake3:settlement")
                    .action("Inspect", 1600 + index as u32),
                );
            }
        }

        for index in 0..states.len() as u32 {
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::TransactionRow && hit.id == 1500 + index),
                "missing receipt row hit {index}"
            );
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::Button && hit.id == 1600 + index),
                "missing receipt action hit {index}"
            );
        }
    }

    #[test]
    fn capability_grant_rows_expose_policy_scope_expiry_and_revoke() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            capability_grant_detail_row(
                &mut ui,
                UiRect::new(16.0, 16.0, 700.0, 78.0),
                CapabilityGrantRow::new("Mail", "storage:read", 1700)
                    .scope("scope inbox attachments")
                    .policy_hash("policy blake3:grant")
                    .expiry("expires 10m")
                    .status("granted")
                    .revoke_action(1701),
            );
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 1700),
            "missing capability grant row hit"
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 1701),
            "missing capability revoke hit"
        );
        assert!(scene.rects().len() >= 6);
    }

    #[test]
    fn system_surface_state_panels_cover_empty_and_error_states() {
        let specs = [
            SystemSurfaceStatePanel::new(
                SystemSurfaceKind::Storage,
                SystemSurfaceState::Empty,
                1800,
            )
            .title("No cached packages")
            .detail("Verified package bytes will appear after cache approval.")
            .reference("storage payload hash pending")
            .action("Run app", 1810),
            SystemSurfaceStatePanel::new(SystemSurfaceKind::Proof, SystemSurfaceState::Error, 1801)
                .title("Proof rejected")
                .detail("The receipt did not match the admitted route policy.")
                .reference("policy blake3:proof")
                .action("Inspect", 1811),
            SystemSurfaceStatePanel::new(
                SystemSurfaceKind::Admission,
                SystemSurfaceState::Empty,
                1802,
            )
            .title("No admission records")
            .detail("Work has not entered an admitted route yet.")
            .reference("admission pending"),
            SystemSurfaceStatePanel::new(SystemSurfaceKind::App, SystemSurfaceState::Error, 1803)
                .title("App verification failed")
                .detail("Package signature or manifest hash was rejected.")
                .reference("manifest blake3:app")
                .action("Retry", 1813),
        ];
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            for (index, spec) in specs.iter().copied().enumerate() {
                system_surface_state_panel(
                    &mut ui,
                    UiRect::new(16.0, 16.0 + index as f32 * 108.0, 680.0, 100.0),
                    spec,
                );
            }
        }

        for id in [1800, 1801, 1802, 1803] {
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::ListRow && hit.id == id),
                "missing system state panel hit {id}"
            );
        }
        for id in [1810, 1811, 1813] {
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == HitKind::Button && hit.id == id),
                "missing system state action hit {id}"
            );
        }
    }

    #[test]
    fn component_test_selectors_are_stable_unique_and_namespaced() {
        assert!(UI_COMPONENT_TEST_IDS.len() >= 14);
        for (index, id) in UI_COMPONENT_TEST_IDS.iter().copied().enumerate() {
            let selector = id.selector();
            assert!(
                selector.starts_with("edgerun."),
                "component selector must be namespaced: {selector}"
            );
            assert!(
                !id.component_name().is_empty(),
                "component selector needs a component name"
            );
            for other in UI_COMPONENT_TEST_IDS.iter().copied().skip(index + 1) {
                assert_ne!(
                    selector,
                    other.selector(),
                    "duplicate component selector {selector}"
                );
            }
        }
    }

    #[test]
    fn component_state_matrices_cover_required_states_for_each_selector() {
        assert_eq!(
            UI_COMPONENT_STATE_MATRICES.len(),
            UI_COMPONENT_TEST_IDS.len(),
            "every component selector needs a state matrix"
        );
        assert_eq!(UI_COMPONENT_STATES.len(), 7);

        for state in UI_COMPONENT_STATES.iter().copied() {
            assert!(
                state.selector().starts_with("state."),
                "state selector must be namespaced: {}",
                state.selector()
            );
            assert!(!state.label().is_empty(), "state needs a display label");
        }

        for id in UI_COMPONENT_TEST_IDS.iter().copied() {
            let matrix = UI_COMPONENT_STATE_MATRICES
                .iter()
                .copied()
                .find(|matrix| matrix.component == id)
                .unwrap_or_else(|| panic!("missing state matrix for {}", id.component_name()));
            assert_eq!(matrix.component_selector(), id.selector());

            for state in UI_COMPONENT_STATES.iter().copied() {
                assert!(
                    matrix.has_state(state),
                    "{} missing state {}",
                    id.component_name(),
                    state.label()
                );
            }
        }
    }

    #[test]
    fn component_projection_contracts_cover_every_stable_component() {
        assert_eq!(
            UI_COMPONENT_PROJECTION_CONTRACTS.len(),
            UI_COMPONENT_TEST_IDS.len(),
            "every stable component needs a projected-state contract"
        );

        for id in UI_COMPONENT_TEST_IDS.iter().copied() {
            let contract = UI_COMPONENT_PROJECTION_CONTRACTS
                .iter()
                .copied()
                .find(|contract| contract.component == id)
                .unwrap_or_else(|| {
                    panic!("missing projection contract for {}", id.component_name())
                });

            assert!(
                contract
                    .fields
                    .iter()
                    .any(|field| field.name == "id" || field.name.ends_with("_id"))
                    || matches!(
                        id,
                        UiComponentTestId::NetworkAppPrompt
                            | UiComponentTestId::TrustManagerActions
                            | UiComponentTestId::DataTableControls
                            | UiComponentTestId::SegmentedControl
                    ),
                "{} needs projected hit/action identity",
                id.component_name()
            );
            assert!(
                contract.required_fields().next().is_some(),
                "{} needs at least one required projected field",
                id.component_name()
            );
            for field in contract.fields {
                assert!(
                    !field.name.is_empty(),
                    "projected field name cannot be empty"
                );
                assert!(
                    !field.name.contains("demo") && !field.name.contains("preview"),
                    "{} projection field must not be demo-owned: {}",
                    id.component_name(),
                    field.name
                );
            }
        }
    }

    #[test]
    fn projection_contracts_capture_domain_authority_fields() {
        let app_card = UI_COMPONENT_PROJECTION_CONTRACTS
            .iter()
            .copied()
            .find(|contract| contract.component == UiComponentTestId::AppStoreCard)
            .expect("app card projection contract");
        assert!(app_card.has_field("package_hash"));
        assert!(app_card.has_field("app_policy_hash"));
        assert!(app_card.has_field("access_mode"));

        let admission = UI_COMPONENT_PROJECTION_CONTRACTS
            .iter()
            .copied()
            .find(|contract| contract.component == UiComponentTestId::AdmissionPolicyRow)
            .expect("admission projection contract");
        assert!(admission.has_field("policy_hash"));
        assert!(admission.has_field("admission_node"));
        assert!(admission.has_field("validity"));

        let receipt = UI_COMPONENT_PROJECTION_CONTRACTS
            .iter()
            .copied()
            .find(|contract| contract.component == UiComponentTestId::ReceiptPaymentRow)
            .expect("receipt projection contract");
        assert!(receipt.has_field("receipt_id"));
        assert!(receipt.has_field("policy_hash"));
        assert!(receipt.has_field("status"));
    }

    #[test]
    fn component_accessibility_metadata_covers_every_stable_component() {
        assert_eq!(
            UI_COMPONENT_ACCESSIBILITY_METADATA.len(),
            UI_COMPONENT_TEST_IDS.len(),
            "every stable component needs accessibility metadata"
        );

        for id in UI_COMPONENT_TEST_IDS.iter().copied() {
            let metadata = UI_COMPONENT_ACCESSIBILITY_METADATA
                .iter()
                .copied()
                .find(|metadata| metadata.component == id)
                .unwrap_or_else(|| {
                    panic!("missing accessibility metadata for {}", id.component_name())
                });
            let projection = UI_COMPONENT_PROJECTION_CONTRACTS
                .iter()
                .copied()
                .find(|contract| contract.component == id)
                .expect("projection contract");

            assert_ne!(
                metadata.role,
                UiA11yRole::Generic,
                "{} should declare a semantic accessibility role",
                id.component_name()
            );
            assert!(
                !metadata.label_fields.is_empty(),
                "{} needs accessible label fields",
                id.component_name()
            );
            for field in metadata.label_fields {
                assert!(
                    projection.has_field(field),
                    "{} accessibility label field must be projected: {}",
                    id.component_name(),
                    field
                );
            }
        }
    }

    #[test]
    fn accessibility_metadata_uses_expected_roles_for_operational_components() {
        let prompt = UI_COMPONENT_ACCESSIBILITY_METADATA
            .iter()
            .copied()
            .find(|metadata| metadata.component == UiComponentTestId::NetworkAppPrompt)
            .expect("prompt accessibility metadata");
        assert_eq!(prompt.role, UiA11yRole::Dialog);
        assert!(prompt.has_label_field("app_name"));

        let state_panel = UI_COMPONENT_ACCESSIBILITY_METADATA
            .iter()
            .copied()
            .find(|metadata| metadata.component == UiComponentTestId::SystemSurfaceStatePanel)
            .expect("state panel accessibility metadata");
        assert_eq!(state_panel.role, UiA11yRole::Status);

        for row in [
            UiComponentTestId::RuntimeEventRow,
            UiComponentTestId::PackageProofRow,
            UiComponentTestId::ReceiptPaymentRow,
            UiComponentTestId::CapabilityGrantDetailRow,
        ] {
            let metadata = UI_COMPONENT_ACCESSIBILITY_METADATA
                .iter()
                .copied()
                .find(|metadata| metadata.component == row)
                .expect("row accessibility metadata");
            assert_eq!(metadata.role, UiA11yRole::ListItem);
        }
    }

    #[test]
    fn stable_components_emit_semantic_hits_and_scene_primitives() {
        for component in UI_COMPONENT_TEST_IDS.iter().copied() {
            let scene = render_component_fixture(component);
            assert!(
                scene.rects().iter().any(|rect| rect.mode == RectMode::Fill),
                "{} should emit fill primitives",
                component.component_name()
            );
            assert!(
                !scene.rects().is_empty(),
                "{} should emit scene rect primitives",
                component.component_name()
            );

            match component {
                UiComponentTestId::NetworkAppPrompt => assert_component_hits(
                    component,
                    &scene,
                    &[
                        (HitKind::Button, 1),
                        (HitKind::Button, 2),
                        (HitKind::Button, 3),
                    ],
                ),
                UiComponentTestId::AppStoreCard => assert_component_hits(
                    component,
                    &scene,
                    &[(HitKind::ListRow, 10), (HitKind::Button, 11)],
                ),
                UiComponentTestId::TrustManagerActions => assert_component_hits(
                    component,
                    &scene,
                    &[
                        (HitKind::Button, 20),
                        (HitKind::Button, 21),
                        (HitKind::Button, 22),
                        (HitKind::Button, 23),
                    ],
                ),
                UiComponentTestId::RuntimeEventRow => {
                    assert_component_hits(component, &scene, &[(HitKind::ListRow, 30)])
                }
                UiComponentTestId::PackageProofRow => {
                    assert_component_hits(component, &scene, &[(HitKind::ListRow, 40)])
                }
                UiComponentTestId::ImportSyncSourceRow => assert_component_hits(
                    component,
                    &scene,
                    &[
                        (HitKind::ListRow, 50),
                        (HitKind::Button, 51),
                        (HitKind::Button, 52),
                    ],
                ),
                UiComponentTestId::PublishFromNodeRow => assert_component_hits(
                    component,
                    &scene,
                    &[
                        (HitKind::ListRow, 60),
                        (HitKind::Button, 61),
                        (HitKind::Button, 62),
                    ],
                ),
                UiComponentTestId::NodeInstanceRow => assert_component_hits(
                    component,
                    &scene,
                    &[(HitKind::ListRow, 70), (HitKind::Button, 71)],
                ),
                UiComponentTestId::AdmissionPolicyRow => assert_component_hits(
                    component,
                    &scene,
                    &[(HitKind::ListRow, 80), (HitKind::Button, 81)],
                ),
                UiComponentTestId::RouteBudgetRow => assert_component_hits(
                    component,
                    &scene,
                    &[(HitKind::ListRow, 90), (HitKind::Button, 91)],
                ),
                UiComponentTestId::DataTableControls => assert_component_hits(
                    component,
                    &scene,
                    &[
                        (HitKind::Input, 100),
                        (HitKind::Button, 101),
                        (HitKind::Button, 102),
                        (HitKind::Button, 103),
                    ],
                ),
                UiComponentTestId::IconOnlyButton => {
                    assert_component_hits(component, &scene, &[(HitKind::Button, 140)])
                }
                UiComponentTestId::SegmentedControl => assert_component_hits(
                    component,
                    &scene,
                    &[
                        (HitKind::Tab, 150),
                        (HitKind::Tab, 151),
                        (HitKind::Tab, 152),
                    ],
                ),
                UiComponentTestId::ReceiptPaymentRow => assert_component_hits(
                    component,
                    &scene,
                    &[(HitKind::TransactionRow, 110), (HitKind::Button, 111)],
                ),
                UiComponentTestId::CapabilityGrantDetailRow => assert_component_hits(
                    component,
                    &scene,
                    &[(HitKind::ListRow, 120), (HitKind::Button, 121)],
                ),
                UiComponentTestId::SystemSurfaceStatePanel => assert_component_hits(
                    component,
                    &scene,
                    &[(HitKind::ListRow, 130), (HitKind::Button, 131)],
                ),
            }
        }
    }

    fn assert_component_hits(
        component: UiComponentTestId,
        scene: &GpuScene,
        expected: &[(HitKind, u32)],
    ) {
        for (kind, id) in expected.iter().copied() {
            assert!(
                scene
                    .hits()
                    .iter()
                    .any(|hit| hit.kind == kind && hit.id == id),
                "{} missing {:?} hit {id}",
                component.component_name(),
                kind
            );
        }
    }

    fn render_component_fixture(component: UiComponentTestId) -> GpuScene {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            match component {
                UiComponentTestId::NetworkAppPrompt => network_app_prompt(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 520.0, 312.0),
                    NetworkAppPrompt::new("Network Mail", 1, 2, 3)
                        .package_size("Package 18 MB")
                        .retrieval_cost("Cost 184 units")
                        .policy_hash("policy blake3:prompt"),
                ),
                UiComponentTestId::AppStoreCard => app_store_card(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 460.0, 148.0),
                    AppStoreCard::new("Mail", 10, 11)
                        .developer("Sylchi")
                        .release("release 2026.05")
                        .package_hash("pkg blake3:card")
                        .app_policy_hash("policy blake3:card")
                        .access_mode("Verify & cache"),
                ),
                UiComponentTestId::TrustManagerActions => trust_manager_actions(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 560.0, 120.0),
                    TrustManagerActions::new(20, 21, 22, 23),
                ),
                UiComponentTestId::RuntimeEventRow => runtime_event_row(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 560.0, 78.0),
                    RuntimeEventRow::new("Runtime event", "event blake3:row", 30)
                        .detail("package verified")
                        .status("accepted"),
                ),
                UiComponentTestId::PackageProofRow => package_proof_row(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 560.0, 78.0),
                    PackageProofRow::new("pkg blake3:proof", "manifest blake3:proof", 40)
                        .developer("Sylchi")
                        .release("release 2026.05")
                        .status("verified"),
                ),
                UiComponentTestId::ImportSyncSourceRow => import_sync_source_row(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 640.0, 78.0),
                    ImportSyncSourceRow::new(ImportSyncSourceKind::Gmail, 50)
                        .detail("mailbox import source")
                        .policy_hash("policy blake3:source")
                        .status("ready")
                        .configure_action(51)
                        .sync_action(52),
                ),
                UiComponentTestId::PublishFromNodeRow => publish_from_node_row(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 700.0, 78.0),
                    PublishFromNodeRow::new(PublishFromNodeKind::Site, 60)
                        .title("Personal site")
                        .node_instance("publishing:personal-site")
                        .route_scope("identity-routed publishing")
                        .policy_hash("policy blake3:publish")
                        .budget("budget 100")
                        .status("ready")
                        .configure_action(61)
                        .publish_action(62),
                ),
                UiComponentTestId::NodeInstanceRow => node_instance_row(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 700.0, 78.0),
                    NodeInstanceRow::new("admission:family", 70)
                        .runtime_target("browser wasm")
                        .route_scope("family devices")
                        .policy_hash("policy blake3:node")
                        .status("ready")
                        .open_action(71),
                ),
                UiComponentTestId::AdmissionPolicyRow => admission_policy_row(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 700.0, 78.0),
                    AdmissionPolicyRow::new("family admission policy", "policy blake3:admit", 80)
                        .admission_node("admission:family")
                        .validity("valid for 10m")
                        .status("committed")
                        .inspect_action(81),
                ),
                UiComponentTestId::RouteBudgetRow => route_budget_row(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 700.0, 78.0),
                    RouteBudgetRow::new("route relay:private-devices", "admitted budget 100", 90)
                        .channel("channel 42")
                        .spent("spent 18")
                        .status("active")
                        .inspect_action(91),
                ),
                UiComponentTestId::DataTableControls => {
                    let columns = [
                        DataTableColumn::new("Object").sort(DataTableSort::Asc, 101),
                        DataTableColumn::new("Hash").sort(DataTableSort::None, 102),
                    ];
                    data_table_controls(
                        &mut ui,
                        UiRect::new(12.0, 12.0, 760.0, 54.0),
                        DataTableControls::new(100, &columns)
                            .title("Objects")
                            .filter_label("Filter objects")
                            .filter_value("policy")
                            .clear_filter_action(103),
                    );
                }
                UiComponentTestId::IconOnlyButton => icon_only_button(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 36.0, 36.0),
                    IconOnlyButton::new(UiIcon::Settings, 140, "Settings"),
                ),
                UiComponentTestId::SegmentedControl => {
                    let items = [
                        SegmentedControlItem::new("Run", 150),
                        SegmentedControlItem::new("Cache", 151),
                        SegmentedControlItem::new("Proof", 152),
                    ];
                    segmented_control(
                        &mut ui,
                        UiRect::new(12.0, 12.0, 300.0, 36.0),
                        SegmentedControl::new(&items, 1),
                    );
                }
                UiComponentTestId::ReceiptPaymentRow => receipt_payment_row(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 700.0, 78.0),
                    ReceiptPaymentRow::new(
                        "Relay receipt",
                        "100 work units",
                        ReceiptPaymentStatus::Payable,
                        110,
                    )
                    .receipt_id("receipt blake3:payment")
                    .policy_hash("policy blake3:settlement")
                    .action("Inspect", 111),
                ),
                UiComponentTestId::CapabilityGrantDetailRow => capability_grant_detail_row(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 700.0, 78.0),
                    CapabilityGrantRow::new("Mail", "storage:read", 120)
                        .scope("scope inbox attachments")
                        .policy_hash("policy blake3:grant")
                        .expiry("expires 10m")
                        .status("granted")
                        .revoke_action(121),
                ),
                UiComponentTestId::SystemSurfaceStatePanel => system_surface_state_panel(
                    &mut ui,
                    UiRect::new(12.0, 12.0, 680.0, 100.0),
                    SystemSurfaceStatePanel::new(
                        SystemSurfaceKind::Proof,
                        SystemSurfaceState::Error,
                        130,
                    )
                    .title("Proof rejected")
                    .detail("Receipt did not match the admitted route policy.")
                    .reference("policy blake3:proof")
                    .action("Inspect", 131),
                ),
            }
        }
        scene
    }
}
