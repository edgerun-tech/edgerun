//! Data-only inventory for components extracted from the HTML captures.

use super::{UiIcon, UiIconSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiExtractedComponentKind {
    Shell,
    Layout,
    Navigation,
    Overlay,
    Card,
    Form,
    InputGroup,
    Chart,
    DataRow,
    Control,
    Selection,
    Feedback,
    EdgeRunDomain,
}

pub const EXTRACTED_COMPONENT_KINDS: [UiExtractedComponentKind; 13] = [
    UiExtractedComponentKind::Shell,
    UiExtractedComponentKind::Layout,
    UiExtractedComponentKind::Navigation,
    UiExtractedComponentKind::Overlay,
    UiExtractedComponentKind::Card,
    UiExtractedComponentKind::Form,
    UiExtractedComponentKind::InputGroup,
    UiExtractedComponentKind::Chart,
    UiExtractedComponentKind::DataRow,
    UiExtractedComponentKind::Control,
    UiExtractedComponentKind::Selection,
    UiExtractedComponentKind::Feedback,
    UiExtractedComponentKind::EdgeRunDomain,
];

impl UiExtractedComponentKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Shell => "Shell",
            Self::Layout => "Layout",
            Self::Navigation => "Navigation",
            Self::Overlay => "Overlay",
            Self::Card => "Card",
            Self::Form => "Form",
            Self::InputGroup => "Input Group",
            Self::Chart => "Chart",
            Self::DataRow => "Data Row",
            Self::Control => "Control",
            Self::Selection => "Selection",
            Self::Feedback => "Feedback",
            Self::EdgeRunDomain => "EdgeRun Domain",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedComponentSpec {
    pub name: &'static str,
    pub kind: UiExtractedComponentKind,
    pub source_classes: &'static str,
    pub edgerun_builder: &'static str,
    pub role: &'static str,
}

impl UiExtractedComponentSpec {
    pub const fn kind_label(self) -> &'static str {
        self.kind.label()
    }

    pub fn uses_source_class(self, class_name: &str) -> bool {
        self.source_classes
            .split_ascii_whitespace()
            .any(|candidate| candidate == class_name)
    }

    pub fn uses_builder(self, builder: &str) -> bool {
        self.edgerun_builder == builder
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedSlotSpec {
    pub slot: &'static str,
    pub kind: UiExtractedComponentKind,
    pub component: &'static str,
    pub source_classes: &'static str,
    pub role: &'static str,
}

impl UiExtractedSlotSpec {
    pub const fn kind_label(self) -> &'static str {
        self.kind.label()
    }

    pub fn uses_source_class(self, class_name: &str) -> bool {
        self.source_classes
            .split_ascii_whitespace()
            .any(|candidate| candidate == class_name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedIconSpec {
    pub name: &'static str,
    pub canonical: Option<UiIcon>,
    pub lucide_name: &'static str,
    pub tabler_name: &'static str,
    pub role: &'static str,
}

impl UiExtractedIconSpec {
    pub const fn provider_name(self, set: UiIconSet) -> &'static str {
        match set {
            UiIconSet::Lucide => self.lucide_name,
            UiIconSet::Tabler => self.tabler_name,
        }
    }

    pub const fn has_canonical_icon(self) -> bool {
        self.canonical.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiExtractedStateKind {
    Interaction,
    Accessibility,
    Validation,
    Selection,
    Disclosure,
    Loading,
    Locking,
}

pub const EXTRACTED_STATE_KINDS: [UiExtractedStateKind; 7] = [
    UiExtractedStateKind::Interaction,
    UiExtractedStateKind::Accessibility,
    UiExtractedStateKind::Validation,
    UiExtractedStateKind::Selection,
    UiExtractedStateKind::Disclosure,
    UiExtractedStateKind::Loading,
    UiExtractedStateKind::Locking,
];

impl UiExtractedStateKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Interaction => "Interaction",
            Self::Accessibility => "Accessibility",
            Self::Validation => "Validation",
            Self::Selection => "Selection",
            Self::Disclosure => "Disclosure",
            Self::Loading => "Loading",
            Self::Locking => "Locking",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedStateSpec {
    pub name: &'static str,
    pub kind: UiExtractedStateKind,
    pub selectors: &'static [&'static str],
    pub role: &'static str,
}

impl UiExtractedStateSpec {
    pub const fn kind_label(self) -> &'static str {
        self.kind.label()
    }

    pub fn has_selector(self, selector: &str) -> bool {
        self.selectors
            .iter()
            .any(|candidate| *candidate == selector)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiExtractedPatternKind {
    Shell,
    Card,
    Control,
    Form,
    Overlay,
    Feedback,
    Navigation,
    EdgeRunDomain,
}

pub const EXTRACTED_PATTERN_KINDS: [UiExtractedPatternKind; 8] = [
    UiExtractedPatternKind::Shell,
    UiExtractedPatternKind::Card,
    UiExtractedPatternKind::Control,
    UiExtractedPatternKind::Form,
    UiExtractedPatternKind::Overlay,
    UiExtractedPatternKind::Feedback,
    UiExtractedPatternKind::Navigation,
    UiExtractedPatternKind::EdgeRunDomain,
];

impl UiExtractedPatternKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Shell => "Shell",
            Self::Card => "Card",
            Self::Control => "Control",
            Self::Form => "Form",
            Self::Overlay => "Overlay",
            Self::Feedback => "Feedback",
            Self::Navigation => "Navigation",
            Self::EdgeRunDomain => "EdgeRun Domain",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedPatternSpec {
    pub name: &'static str,
    pub kind: UiExtractedPatternKind,
    pub components: &'static [&'static str],
    pub slots: &'static [&'static str],
    pub states: &'static [&'static str],
    pub tokens: &'static [&'static str],
    pub icons: &'static [&'static str],
    pub builder: &'static str,
    pub role: &'static str,
}

impl UiExtractedPatternSpec {
    pub const fn kind_label(self) -> &'static str {
        self.kind.label()
    }

    pub fn uses_component(self, name: &str) -> bool {
        self.components.iter().any(|candidate| *candidate == name)
    }

    pub fn uses_slot(self, slot: &str) -> bool {
        self.slots.iter().any(|candidate| *candidate == slot)
    }

    pub fn uses_state(self, state: &str) -> bool {
        self.states.iter().any(|candidate| *candidate == state)
    }

    pub fn uses_token(self, token: &str) -> bool {
        self.tokens.iter().any(|candidate| *candidate == token)
    }

    pub fn uses_icon(self, icon: &str) -> bool {
        self.icons.iter().any(|candidate| *candidate == icon)
    }
}

pub const EXTRACTED_COMPONENTS: &[UiExtractedComponentSpec] = &[
    spec(
        "Application Header",
        UiExtractedComponentKind::Shell,
        "sticky top-0 h-(--header-height) bg-background",
        "row + button + command_palette",
        "top navigation, search, user actions",
    ),
    spec(
        "Style Authority Rail",
        UiExtractedComponentKind::Navigation,
        "rounded-md border bg-card p-3 gap-3",
        "card + menu_item_node + control_row_node",
        "user-owned theme choices and optional author vision",
    ),
    spec(
        "Dashboard Card",
        UiExtractedComponentKind::Card,
        "rounded-xl bg-card text-card-foreground ring-1",
        "card + header",
        "compact grouped surface for repeated content",
    ),
    spec(
        "Primary Button",
        UiExtractedComponentKind::Control,
        "rounded-lg bg-primary text-primary-foreground h-8 px-2.5",
        "button(label, id, ButtonStyle::Primary)",
        "main action",
    ),
    spec(
        "Secondary Button",
        UiExtractedComponentKind::Control,
        "rounded-lg border bg-background hover:bg-muted h-8 px-2.5",
        "button(label, id, ButtonStyle::Secondary)",
        "non-destructive secondary action",
    ),
    spec(
        "Badge",
        UiExtractedComponentKind::Control,
        "inline-flex rounded-md border px-1.5 py-0.5 text-xs",
        "badge(label, color)",
        "compact status, usage, and filter marker",
    ),
    spec(
        "Input Group",
        UiExtractedComponentKind::InputGroup,
        "flex items-center rounded-md border bg-background",
        "card + field_node + icon_button + badge",
        "search, URL, amount, and command fields with accessories",
    ),
    spec(
        "Prompt Composer",
        UiExtractedComponentKind::InputGroup,
        "rounded-xl border bg-card p-2 gap-2",
        "text_area_node + icon_button + button",
        "agent prompt input with attachment, mode, usage, and send controls",
    ),
    spec(
        "Text Field",
        UiExtractedComponentKind::Form,
        "rounded-lg border border-input bg-transparent px-2.5 py-2",
        "field_node(label, value)",
        "labeled input with optional helper text",
    ),
    spec(
        "Text Area",
        UiExtractedComponentKind::Form,
        "min-h-16 rounded-lg border bg-transparent px-2.5 py-2",
        "text_area_node(label, value)",
        "multi-line notes and prompts",
    ),
    spec(
        "Select",
        UiExtractedComponentKind::Form,
        "rounded-md border bg-background h-8 px-3",
        "select_node(label, value, id)",
        "single-value option picker",
    ),
    spec(
        "Slider",
        UiExtractedComponentKind::Form,
        "h-2 rounded-full bg-muted",
        "slider_node(label, value, id)",
        "budget, threshold, and percentage controls",
    ),
    spec(
        "Metric Card",
        UiExtractedComponentKind::Card,
        "rounded-lg bg-muted/50 p-4",
        "metric(title, value)",
        "large value, small detail, optional progress",
    ),
    spec(
        "Bar Chart",
        UiExtractedComponentKind::Chart,
        "rounded-md bg-card p-4",
        "bar_chart_labels(title, labels, values)",
        "small activity history",
    ),
    spec(
        "Transaction Row",
        UiExtractedComponentKind::DataRow,
        "grid h-14 border-b text-sm",
        "transaction_node(title, amount, id)",
        "ledger-style rows with date and signed amount",
    ),
    spec(
        "Menu Item",
        UiExtractedComponentKind::Navigation,
        "rounded-md px-3 py-2 hover:bg-muted data-active:bg-muted",
        "menu_item_node(label, id)",
        "sidebar and settings navigation",
    ),
    spec(
        "Dropdown Radio Menu",
        UiExtractedComponentKind::Overlay,
        "rounded-xl bg-neutral-950/80 text-neutral-100 ring-1 backdrop-blur-xl",
        "card + radio options",
        "style picker and compact option menu",
    ),
    spec(
        "Dialog Panel",
        UiExtractedComponentKind::Overlay,
        "rounded-xl border bg-card p-6 shadow-lg",
        "dialog(title, body, icon)",
        "confirmations, search modal, first-run app prompt",
    ),
    spec(
        "Breadcrumb",
        UiExtractedComponentKind::Navigation,
        "flex items-center gap-1 text-sm text-muted-foreground",
        "breadcrumb(labels, selected, id)",
        "nested settings and policy path navigation",
    ),
    spec(
        "Table",
        UiExtractedComponentKind::DataRow,
        "w-full caption-bottom text-sm [&_tr]:border-b",
        "table_labels(headers, rows, id)",
        "proofs, receipts, package files, and policy rows",
    ),
    spec(
        "Separator",
        UiExtractedComponentKind::Layout,
        "shrink-0 bg-border data-[orientation=horizontal]:h-px",
        "divider() / field separator spacing",
        "separate nav groups, fields, menus, and card footers",
    ),
    spec(
        "Card Footer",
        UiExtractedComponentKind::Card,
        "flex items-center border-t px-6 py-4",
        "row + button group",
        "form save/cancel, payment confirm, and paging actions",
    ),
    spec(
        "Skeleton",
        UiExtractedComponentKind::Feedback,
        "animate-pulse rounded-md bg-muted",
        "skeleton()",
        "loading placeholder for cards, rows, and charts",
    ),
    spec(
        "Progress",
        UiExtractedComponentKind::Feedback,
        "h-2 rounded-full bg-muted",
        "progress_bar_node(value, color) / progress_ring(value, color)",
        "target completion, package retrieval, and verification progress",
    ),
    spec(
        "Proof Event Row",
        UiExtractedComponentKind::EdgeRunDomain,
        "rounded-md border bg-card p-3 text-sm",
        "proof_event_row(title, hash, status, id)",
        "Trust Manager proof dashboard event",
    ),
    spec(
        "Receipt Row",
        UiExtractedComponentKind::EdgeRunDomain,
        "grid grid-cols-[1fr_auto] border-b py-2",
        "receipt_row(label, amount, status, id)",
        "payment, storage, relay, and app run receipts",
    ),
    spec(
        "Thread Row",
        UiExtractedComponentKind::DataRow,
        "flex items-center gap-3 rounded-md px-3 py-2",
        "thread_row(title, last_message, unread, id)",
        "message and activity inbox rows",
    ),
    spec(
        "Contact Card",
        UiExtractedComponentKind::DataRow,
        "rounded-md border bg-card p-3 gap-2",
        "contact_card(name, detail, id)",
        "identity contact and agent assignment card",
    ),
    spec(
        "Attachment Preview",
        UiExtractedComponentKind::DataRow,
        "rounded-md border bg-muted/50 px-3 py-2",
        "attachment_preview(name, kind, id)",
        "prompt, message, package, and import-source attachments",
    ),
    spec(
        "Capability Grant Row",
        UiExtractedComponentKind::EdgeRunDomain,
        "grid grid-cols-[auto_1fr_auto] rounded-md border p-3",
        "capability_grant_row(app, capability, state, id)",
        "Trust Container scoped app or agent capability",
    ),
    spec(
        "App Launcher Item",
        UiExtractedComponentKind::Navigation,
        "rounded-md px-3 py-2 hover:bg-muted data-active:bg-muted",
        "app_launcher_item(title, detail, icon, id)",
        "workspace launcher and pinned system apps",
    ),
    spec(
        "Tree Item",
        UiExtractedComponentKind::Navigation,
        "group/tree flex items-center rounded-md px-2 py-1.5",
        "tree_item(label, detail, depth, expanded, id)",
        "file, package object, and policy hierarchy navigation",
    ),
    spec(
        "Radio Chip Group",
        UiExtractedComponentKind::Selection,
        "grid gap-2 [&_label]:rounded-md [&_label]:border",
        "radio(label, selected, id)",
        "single-choice onboarding and settings questions",
    ),
    spec(
        "Toggle Row",
        UiExtractedComponentKind::Control,
        "flex items-center justify-between border-b",
        "control_row_node(label).toggle(on, id)",
        "preference and policy switch rows",
    ),
    spec(
        "Empty State",
        UiExtractedComponentKind::Feedback,
        "rounded-xl border-dashed p-6 text-center",
        "empty_state(title, body, icon)",
        "blank and processing states",
    ),
    spec(
        "Network App Card",
        UiExtractedComponentKind::EdgeRunDomain,
        "rounded-md border bg-card p-3 gap-3",
        "package_card + route_path + button Run",
        "run/cache package policy summary",
    ),
];

pub const EXTRACTED_SLOTS: &[UiExtractedSlotSpec] = &[
    slot_spec(
        "layout",
        UiExtractedComponentKind::Shell,
        "Application Shell",
        "group/layout relative z-10 flex min-h-svh flex-col bg-background",
        "root page layout with designer-mode containment",
    ),
    slot_spec(
        "designer",
        UiExtractedComponentKind::Shell,
        "Designer Surface",
        "relative flex flex-1 flex-col justify-center overflow-hidden rounded-2xl ring",
        "first-viewport preview surface for extracted component work",
    ),
    slot_spec(
        "button",
        UiExtractedComponentKind::Control,
        "Button",
        "group/button inline-flex shrink-0 items-center justify-center rounded-lg",
        "clickable command with icon, label, focus, disabled, and active states",
    ),
    slot_spec(
        "separator",
        UiExtractedComponentKind::Layout,
        "Separator",
        "shrink-0 bg-border data-[orientation=horizontal]:h-px",
        "horizontal and vertical visual divider",
    ),
    slot_spec(
        "card",
        UiExtractedComponentKind::Card,
        "Card",
        "group/card flex flex-col gap-4 overflow-hidden rounded-xl bg-card",
        "grouped content surface with optional header/content/footer slots",
    ),
    slot_spec(
        "card-header",
        UiExtractedComponentKind::Card,
        "Card Header",
        "group/card-header auto-rows-min rounded-t-xl px-4",
        "title/action area for cards and panels",
    ),
    slot_spec(
        "card-content",
        UiExtractedComponentKind::Card,
        "Card Content",
        "group-data-[size=sm]/card:px-3 px-4",
        "main body area inside a card",
    ),
    slot_spec(
        "card-footer",
        UiExtractedComponentKind::Card,
        "Card Footer",
        "flex items-center border-t px-6 py-4",
        "trailing actions, confirmations, and paging controls",
    ),
    slot_spec(
        "popover-trigger",
        UiExtractedComponentKind::Overlay,
        "Popover Trigger",
        "relative flex h-8 w-4 items-center justify-center",
        "button or icon target that opens a small anchored overlay",
    ),
    slot_spec(
        "dialog-trigger",
        UiExtractedComponentKind::Overlay,
        "Dialog Trigger",
        "inline-flex shrink-0 items-center justify-center",
        "button target that opens a modal dialog",
    ),
    slot_spec(
        "dialog-header",
        UiExtractedComponentKind::Overlay,
        "Dialog Header",
        "flex flex-col gap-2 text-center sm:text-left",
        "modal title and description grouping",
    ),
    slot_spec(
        "dialog-title",
        UiExtractedComponentKind::Overlay,
        "Dialog Title",
        "text-lg font-semibold leading-none",
        "modal headline text",
    ),
    slot_spec(
        "dialog-description",
        UiExtractedComponentKind::Overlay,
        "Dialog Description",
        "text-sm text-muted-foreground",
        "modal helper text",
    ),
    slot_spec(
        "dropdown-menu-trigger",
        UiExtractedComponentKind::Overlay,
        "Dropdown Trigger",
        "touch-manipulation select-none hover:bg-muted",
        "anchored menu target for style and option pickers",
    ),
    slot_spec(
        "dropdown-menu-content",
        UiExtractedComponentKind::Overlay,
        "Dropdown Menu",
        "rounded-xl bg-neutral-950/80 p-1.5 text-neutral-100 ring-1 backdrop-blur-xl",
        "floating menu panel with blur, ring, and scroll constraints",
    ),
    slot_spec(
        "dropdown-menu-radio-group",
        UiExtractedComponentKind::Selection,
        "Dropdown Radio Group",
        "role=group",
        "single-choice grouped menu options",
    ),
    slot_spec(
        "dropdown-menu-radio-item",
        UiExtractedComponentKind::Selection,
        "Dropdown Radio Item",
        "relative flex cursor-default items-center gap-2 rounded-lg py-1.5",
        "single option row with checked/unchecked state",
    ),
    slot_spec(
        "dropdown-menu-radio-item-indicator",
        UiExtractedComponentKind::Selection,
        "Dropdown Radio Indicator",
        "pointer-events-none absolute right-2 flex items-center justify-center",
        "checkmark position for selected radio menu item",
    ),
    slot_spec(
        "field-group",
        UiExtractedComponentKind::Form,
        "Field Group",
        "group/field-group flex w-full gap-3",
        "stacked or row-based form group",
    ),
    slot_spec(
        "field-separator",
        UiExtractedComponentKind::Form,
        "Field Separator",
        "relative -my-2 h-5 text-sm",
        "section break inside a form group",
    ),
    slot_spec(
        "checkbox-group",
        UiExtractedComponentKind::Selection,
        "Checkbox Group",
        "data-[slot=checkbox-group]:gap-3",
        "multi-choice settings or policy group",
    ),
    slot_spec(
        "textarea",
        UiExtractedComponentKind::Form,
        "Textarea",
        "min-h-16 w-full rounded-lg border border-input bg-transparent",
        "multi-line prompt, note, or task input",
    ),
    slot_spec(
        "empty",
        UiExtractedComponentKind::Feedback,
        "Empty State",
        "flex min-w-0 flex-1 flex-col items-center justify-center gap-4 rounded-xl border-dashed",
        "blank, loading, and processing state wrapper",
    ),
    slot_spec(
        "empty-header",
        UiExtractedComponentKind::Feedback,
        "Empty Header",
        "flex max-w-sm flex-col items-center gap-2",
        "icon/title/description grouping for empty states",
    ),
    slot_spec(
        "empty-icon",
        UiExtractedComponentKind::Feedback,
        "Empty Icon",
        "flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted",
        "compact icon well in an empty state",
    ),
    slot_spec(
        "empty-title",
        UiExtractedComponentKind::Feedback,
        "Empty Title",
        "text-sm font-medium tracking-tight",
        "short empty-state headline",
    ),
    slot_spec(
        "empty-description",
        UiExtractedComponentKind::Feedback,
        "Empty Description",
        "text-sm/relaxed text-muted-foreground",
        "supporting empty-state copy",
    ),
    slot_spec(
        "empty-content",
        UiExtractedComponentKind::Feedback,
        "Empty Content",
        "flex w-full max-w-sm min-w-0 flex-col items-center gap-2.5",
        "optional empty-state action area",
    ),
];

pub const EXTRACTED_SOURCE_ICONS: &[UiExtractedIconSpec] = &[
    icon_spec(
        "search",
        Some(UiIcon::Search),
        "search",
        "search",
        "search field and command palette trigger",
    ),
    icon_spec("plus", None, "plus", "plus", "add/create command"),
    icon_spec(
        "gallery horizontal",
        None,
        "gallery-horizontal",
        "layout-grid",
        "component gallery and preview switcher",
    ),
    icon_spec(
        "chevron right",
        Some(UiIcon::ChevronRight),
        "chevron-right",
        "chevron-right",
        "row disclosure and navigation affordance",
    ),
    icon_spec(
        "chevron down",
        None,
        "chevron-down",
        "chevron-down",
        "select and dropdown trigger affordance",
    ),
    icon_spec(
        "check",
        Some(UiIcon::Check),
        "check",
        "check",
        "selected radio menu item and completed state",
    ),
    icon_spec(
        "loader circle",
        None,
        "loader-circle",
        "loader-2",
        "loading and processing state",
    ),
    icon_spec(
        "bot",
        None,
        "bot",
        "robot",
        "agent task and automation action",
    ),
    icon_spec(
        "badge check",
        Some(UiIcon::Trust),
        "badge-check",
        "shield-check",
        "verified package, identity, or policy marker",
    ),
    icon_spec(
        "message plus",
        Some(UiIcon::MessagePlus),
        "message-circle-plus",
        "message-plus",
        "new chat or session action",
    ),
    icon_spec(
        "arrow up",
        Some(UiIcon::Send),
        "arrow-up",
        "arrow-up",
        "send or submit action",
    ),
    icon_spec(
        "arrow right",
        Some(UiIcon::ChevronRight),
        "arrow-right",
        "arrow-right",
        "continue, next, and outbound navigation",
    ),
    icon_spec(
        "trash",
        Some(UiIcon::Trash),
        "trash-2",
        "trash",
        "clear or remove action",
    ),
    icon_spec("x", Some(UiIcon::X), "x", "x", "close or dismiss action"),
];

pub const EXTRACTED_STATES: &[UiExtractedStateSpec] = &[
    state_spec(
        "hover",
        UiExtractedStateKind::Interaction,
        &[
            "hover:bg-muted",
            "hover:bg-accent",
            "hover:text-accent-foreground",
        ],
        "pointer hover emphasis for buttons, rows, pickers, and links",
    ),
    state_spec(
        "pressed",
        UiExtractedStateKind::Interaction,
        &[
            "active:not-aria-[haspopup]:translate-y-px",
            "active:bg-transparent",
        ],
        "pressed button feedback without changing layout",
    ),
    state_spec(
        "focus visible",
        UiExtractedStateKind::Accessibility,
        &[
            "focus-visible:border-ring",
            "focus-visible:ring-3",
            "focus-visible:ring-ring/50",
        ],
        "keyboard focus ring and border treatment",
    ),
    state_spec(
        "disabled",
        UiExtractedStateKind::Accessibility,
        &["disabled:pointer-events-none", "disabled:opacity-50"],
        "unavailable controls suppress pointer interaction and reduce emphasis",
    ),
    state_spec(
        "invalid",
        UiExtractedStateKind::Validation,
        &[
            "aria-invalid:border-destructive",
            "aria-invalid:ring-destructive/20",
        ],
        "form and input validation error treatment",
    ),
    state_spec(
        "active",
        UiExtractedStateKind::Selection,
        &[
            "data-[active=true]:bg-accent",
            "data-[active=true]:text-accent-foreground",
        ],
        "selected tab, toolbar, or segmented-control state",
    ),
    state_spec(
        "checked",
        UiExtractedStateKind::Selection,
        &["data-checked", "data-[state=checked]", "aria-checked=true"],
        "selected radio, checkbox, or menu item state",
    ),
    state_spec(
        "unchecked",
        UiExtractedStateKind::Selection,
        &["data-unchecked", "aria-checked=false"],
        "unselected radio, checkbox, or menu item state",
    ),
    state_spec(
        "open",
        UiExtractedStateKind::Disclosure,
        &[
            "data-open",
            "data-popup-open:bg-muted",
            "aria-expanded=true",
        ],
        "visible menu, dialog, popover, or disclosure state",
    ),
    state_spec(
        "closed",
        UiExtractedStateKind::Disclosure,
        &["data-closed:overflow-hidden", "aria-expanded=false"],
        "hidden menu, dialog, popover, or disclosure state",
    ),
    state_spec(
        "loading",
        UiExtractedStateKind::Loading,
        &["animate-spin", "role=status", "aria-label=Loading"],
        "processing or pending async work state",
    ),
    state_spec(
        "locked",
        UiExtractedStateKind::Locking,
        &["data-[locked=true]:opacity-100"],
        "user-locked picker or author-style control state",
    ),
];

pub const EXTRACTED_PATTERNS: &[UiExtractedPatternSpec] = &[
    pattern_spec(
        "Application Shell",
        UiExtractedPatternKind::Shell,
        &["Application Header", "Style Authority Rail", "Separator"],
        &["layout", "designer", "button", "separator"],
        &["hover", "focus visible", "disabled", "active"],
        &[
            "background",
            "foreground",
            "border",
            "accent",
            "accent foreground",
        ],
        &["gallery horizontal", "search"],
        "build_edgerun_workspace_shell + style_authority_panel",
        "top-level app chrome with user-owned style controls",
    ),
    pattern_spec(
        "Button",
        UiExtractedPatternKind::Control,
        &["Primary Button", "Secondary Button", "Badge"],
        &["button"],
        &["hover", "pressed", "focus visible", "disabled", "invalid"],
        &[
            "primary",
            "primary foreground",
            "secondary",
            "secondary foreground",
            "border",
        ],
        &["arrow right", "arrow up", "plus"],
        "button(label, id, ButtonStyle)",
        "small command surface with icon, label, focus, disabled, and validation states",
    ),
    pattern_spec(
        "Card",
        UiExtractedPatternKind::Card,
        &["Dashboard Card", "Card Footer", "Metric Card"],
        &["card", "card-header", "card-content", "card-footer"],
        &["hover", "active"],
        &["card", "card foreground", "muted", "border"],
        &[],
        "card + header + row/footer children",
        "bounded repeated surface for grouped content and tools",
    ),
    pattern_spec(
        "Input Group",
        UiExtractedPatternKind::Form,
        &["Input Group", "Text Field", "Text Area", "Select", "Slider"],
        &["field-group", "field-separator", "textarea"],
        &["focus visible", "disabled", "invalid", "locked"],
        &["background", "input", "ring", "muted foreground", "border"],
        &["search", "plus", "arrow up", "check"],
        "field_node + text_area_node + select_node + slider_node",
        "forms, search fields, prompts, and amount inputs with accessories",
    ),
    pattern_spec(
        "Dropdown Menu",
        UiExtractedPatternKind::Overlay,
        &["Dropdown Radio Menu", "Menu Item"],
        &[
            "dropdown-menu-trigger",
            "dropdown-menu-content",
            "dropdown-menu-radio-group",
            "dropdown-menu-radio-item",
            "dropdown-menu-radio-item-indicator",
        ],
        &["open", "closed", "checked", "unchecked", "focus visible"],
        &[
            "popover",
            "popover foreground",
            "accent",
            "accent foreground",
        ],
        &["chevron down", "check"],
        "card + radio options",
        "anchored option picker and style-family menu",
    ),
    pattern_spec(
        "Dialog",
        UiExtractedPatternKind::Overlay,
        &["Dialog Panel", "Primary Button", "Secondary Button"],
        &[
            "dialog-trigger",
            "dialog-header",
            "dialog-title",
            "dialog-description",
        ],
        &["open", "closed", "focus visible", "disabled"],
        &["card", "card foreground", "muted foreground", "border"],
        &["x", "badge check"],
        "dialog(title, body, icon) + footer buttons",
        "modal confirmation, command, and first-run app prompt",
    ),
    pattern_spec(
        "Empty State",
        UiExtractedPatternKind::Feedback,
        &["Empty State", "Skeleton", "Progress"],
        &[
            "empty",
            "empty-header",
            "empty-icon",
            "empty-title",
            "empty-description",
            "empty-content",
        ],
        &["loading", "disabled"],
        &["muted", "foreground", "muted foreground", "border"],
        &["loader circle"],
        "empty_state + skeleton + progress_ring",
        "blank, loading, and processing state composition",
    ),
    pattern_spec(
        "Navigation List",
        UiExtractedPatternKind::Navigation,
        &["Menu Item", "Breadcrumb", "App Launcher Item", "Tree Item"],
        &["button", "separator"],
        &["hover", "active", "focus visible", "open", "closed"],
        &["muted", "muted foreground", "accent", "accent foreground"],
        &["chevron right", "search"],
        "menu_item_node + breadcrumb + tree_item",
        "sidebars, launchers, breadcrumbs, and hierarchy navigation",
    ),
    pattern_spec(
        "Network App Card",
        UiExtractedPatternKind::EdgeRunDomain,
        &[
            "Network App Card",
            "Proof Event Row",
            "Receipt Row",
            "Capability Grant Row",
        ],
        &[
            "card",
            "card-header",
            "card-content",
            "card-footer",
            "button",
        ],
        &["hover", "focus visible", "loading", "disabled"],
        &["card", "primary", "primary foreground", "success", "border"],
        &["badge check", "arrow right", "loader circle"],
        "package_card + route_path + button Run",
        "run/cache package policy surface with proof and receipt affordances",
    ),
];

const fn spec(
    name: &'static str,
    kind: UiExtractedComponentKind,
    source_classes: &'static str,
    edgerun_builder: &'static str,
    role: &'static str,
) -> UiExtractedComponentSpec {
    UiExtractedComponentSpec {
        name,
        kind,
        source_classes,
        edgerun_builder,
        role,
    }
}

const fn slot_spec(
    slot: &'static str,
    kind: UiExtractedComponentKind,
    component: &'static str,
    source_classes: &'static str,
    role: &'static str,
) -> UiExtractedSlotSpec {
    UiExtractedSlotSpec {
        slot,
        kind,
        component,
        source_classes,
        role,
    }
}

const fn icon_spec(
    name: &'static str,
    canonical: Option<UiIcon>,
    lucide_name: &'static str,
    tabler_name: &'static str,
    role: &'static str,
) -> UiExtractedIconSpec {
    UiExtractedIconSpec {
        name,
        canonical,
        lucide_name,
        tabler_name,
        role,
    }
}

const fn state_spec(
    name: &'static str,
    kind: UiExtractedStateKind,
    selectors: &'static [&'static str],
    role: &'static str,
) -> UiExtractedStateSpec {
    UiExtractedStateSpec {
        name,
        kind,
        selectors,
        role,
    }
}

const fn pattern_spec(
    name: &'static str,
    kind: UiExtractedPatternKind,
    components: &'static [&'static str],
    slots: &'static [&'static str],
    states: &'static [&'static str],
    tokens: &'static [&'static str],
    icons: &'static [&'static str],
    builder: &'static str,
    role: &'static str,
) -> UiExtractedPatternSpec {
    UiExtractedPatternSpec {
        name,
        kind,
        components,
        slots,
        states,
        tokens,
        icons,
        builder,
        role,
    }
}
