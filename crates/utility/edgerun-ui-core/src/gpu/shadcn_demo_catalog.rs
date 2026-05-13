//! shadcn demo compatibility catalog.
//!
//! This is the compatibility contract for porting existing shadcn sites to
//! EdgeRun UI. Entries use the shadcn component names and docs routes so a
//! migration tool can target familiar component surfaces while the renderer
//! stays native.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiShadcnDemoCategory {
    Foundation,
    Form,
    Overlay,
    Navigation,
    DataDisplay,
    Feedback,
    Layout,
    Media,
}

impl UiShadcnDemoCategory {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Foundation => "Foundation",
            Self::Form => "Form",
            Self::Overlay => "Overlay",
            Self::Navigation => "Navigation",
            Self::DataDisplay => "Data Display",
            Self::Feedback => "Feedback",
            Self::Layout => "Layout",
            Self::Media => "Media",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiShadcnDemoStatus {
    Cataloged,
    NativePrimitive,
    ExactPort,
}

impl UiShadcnDemoStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cataloged => "Cataloged",
            Self::NativePrimitive => "Native primitive",
            Self::ExactPort => "Exact port",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnDemoSpec {
    pub name: &'static str,
    pub slug: &'static str,
    pub route: &'static str,
    pub category: UiShadcnDemoCategory,
    pub source_component: &'static str,
    pub edge_builder: &'static str,
    pub slots: &'static [&'static str],
    pub states: &'static [&'static str],
    pub status: UiShadcnDemoStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiShadcnResolveKind {
    Slug,
    SourceComponent,
    ModulePath,
    Slot,
}

impl UiShadcnResolveKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Slug => "Slug",
            Self::SourceComponent => "Source component",
            Self::ModulePath => "Module path",
            Self::Slot => "Slot",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnResolvedDemo {
    pub spec: &'static UiShadcnDemoSpec,
    pub kind: UiShadcnResolveKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnPortMapping {
    pub identifier: &'static str,
    pub resolve_kind: UiShadcnResolveKind,
    pub slug: &'static str,
    pub source_component: &'static str,
    pub edge_builder: &'static str,
    pub category: UiShadcnDemoCategory,
    pub status: UiShadcnDemoStatus,
    pub native_renderer: bool,
    pub exact_port: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnPortManifest {
    pub identifiers: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnPortCategorySummary {
    pub category: UiShadcnDemoCategory,
    pub label: &'static str,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnPortStatusSummary {
    pub status: UiShadcnDemoStatus,
    pub label: &'static str,
    pub count: usize,
}

impl UiShadcnPortManifest {
    pub const fn new(identifiers: &'static [&'static str]) -> Self {
        Self { identifiers }
    }

    pub const fn identifier_count(self) -> usize {
        self.identifiers.len()
    }

    pub fn mappings(self) -> impl Iterator<Item = UiShadcnPortMapping> {
        self.identifiers
            .iter()
            .filter_map(|identifier| shadcn_port_mapping_for_identifier(identifier))
    }

    pub fn missing_identifiers(self) -> impl Iterator<Item = &'static str> {
        self.identifiers
            .iter()
            .copied()
            .filter(|identifier| shadcn_port_mapping_for_identifier(identifier).is_none())
    }

    pub fn resolved_count(self) -> usize {
        self.mappings().count()
    }

    pub fn missing_count(self) -> usize {
        self.missing_identifiers().count()
    }

    pub fn complete(self) -> bool {
        self.missing_count() == 0
    }

    pub fn native_renderer_count(self) -> usize {
        self.mappings()
            .filter(|mapping| mapping.native_renderer)
            .count()
    }

    pub fn exact_port_count(self) -> usize {
        self.mappings().filter(|mapping| mapping.exact_port).count()
    }

    pub fn count_by_category(self, category: UiShadcnDemoCategory) -> usize {
        self.mappings()
            .filter(|mapping| mapping.category == category)
            .count()
    }

    pub fn count_by_status(self, status: UiShadcnDemoStatus) -> usize {
        self.mappings()
            .filter(|mapping| mapping.status == status)
            .count()
    }

    pub fn category_summary(self, category: UiShadcnDemoCategory) -> UiShadcnPortCategorySummary {
        UiShadcnPortCategorySummary {
            category,
            label: category.label(),
            count: self.count_by_category(category),
        }
    }

    pub fn category_summaries(self) -> [UiShadcnPortCategorySummary; 8] {
        SHADCN_DEMO_CATEGORIES.map(|category| self.category_summary(category))
    }

    pub fn status_summary(self, status: UiShadcnDemoStatus) -> UiShadcnPortStatusSummary {
        UiShadcnPortStatusSummary {
            status,
            label: status.label(),
            count: self.count_by_status(status),
        }
    }

    pub fn status_summaries(self) -> [UiShadcnPortStatusSummary; 3] {
        SHADCN_DEMO_STATUSES.map(|status| self.status_summary(status))
    }
}

impl UiShadcnDemoSpec {
    pub const fn category_label(self) -> &'static str {
        self.category.label()
    }

    pub const fn status_label(self) -> &'static str {
        self.status.label()
    }

    pub fn uses_slot(self, slot: &str) -> bool {
        self.slots.iter().any(|candidate| *candidate == slot)
    }

    pub fn uses_state(self, state: &str) -> bool {
        self.states.iter().any(|candidate| *candidate == state)
    }

    pub const fn has_native_renderer(self) -> bool {
        matches!(
            self.status,
            UiShadcnDemoStatus::NativePrimitive | UiShadcnDemoStatus::ExactPort
        )
    }

    pub const fn is_exact_port(self) -> bool {
        matches!(self.status, UiShadcnDemoStatus::ExactPort)
    }

    pub const fn port_mapping(
        &'static self,
        identifier: &'static str,
        resolve_kind: UiShadcnResolveKind,
    ) -> UiShadcnPortMapping {
        UiShadcnPortMapping {
            identifier,
            resolve_kind,
            slug: self.slug,
            source_component: self.source_component,
            edge_builder: self.edge_builder,
            category: self.category,
            status: self.status,
            native_renderer: self.has_native_renderer(),
            exact_port: self.is_exact_port(),
        }
    }
}

pub const SHADCN_DEMO_CATEGORIES: [UiShadcnDemoCategory; 8] = [
    UiShadcnDemoCategory::Foundation,
    UiShadcnDemoCategory::Form,
    UiShadcnDemoCategory::Overlay,
    UiShadcnDemoCategory::Navigation,
    UiShadcnDemoCategory::DataDisplay,
    UiShadcnDemoCategory::Feedback,
    UiShadcnDemoCategory::Layout,
    UiShadcnDemoCategory::Media,
];

pub const SHADCN_DEMO_STATUSES: [UiShadcnDemoStatus; 3] = [
    UiShadcnDemoStatus::Cataloged,
    UiShadcnDemoStatus::NativePrimitive,
    UiShadcnDemoStatus::ExactPort,
];

pub const SHADCN_DEMO_COMPONENTS: &[UiShadcnDemoSpec] = &[
    demo(
        "Accordion",
        "accordion",
        "/docs/components/accordion",
        UiShadcnDemoCategory::Layout,
        "Accordion",
        "accordion_node",
        &[
            "accordion",
            "accordion-item",
            "accordion-trigger",
            "accordion-content",
        ],
        &["data-state=open", "data-state=closed", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Alert",
        "alert",
        "/docs/components/alert",
        UiShadcnDemoCategory::Feedback,
        "Alert",
        "alert_node",
        &["alert", "alert-title", "alert-description"],
        &["default", "destructive"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Alert Dialog",
        "alert-dialog",
        "/docs/components/alert-dialog",
        UiShadcnDemoCategory::Overlay,
        "AlertDialog",
        "alert_dialog_node",
        &[
            "alert-dialog",
            "alert-dialog-trigger",
            "alert-dialog-content",
            "alert-dialog-header",
            "alert-dialog-footer",
            "alert-dialog-title",
            "alert-dialog-description",
            "alert-dialog-action",
            "alert-dialog-cancel",
        ],
        &["open", "closed", "focus-trap"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Aspect Ratio",
        "aspect-ratio",
        "/docs/components/aspect-ratio",
        UiShadcnDemoCategory::Media,
        "AspectRatio",
        "aspect_ratio_node",
        &["aspect-ratio"],
        &[],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Avatar",
        "avatar",
        "/docs/components/avatar",
        UiShadcnDemoCategory::DataDisplay,
        "Avatar",
        "avatar_node",
        &["avatar", "avatar-image", "avatar-fallback"],
        &["loaded", "fallback"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Badge",
        "badge",
        "/docs/components/badge",
        UiShadcnDemoCategory::Foundation,
        "Badge",
        "badge",
        &["badge"],
        &["default", "secondary", "outline", "destructive"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Breadcrumb",
        "breadcrumb",
        "/docs/components/breadcrumb",
        UiShadcnDemoCategory::Navigation,
        "Breadcrumb",
        "breadcrumb",
        &[
            "breadcrumb",
            "breadcrumb-list",
            "breadcrumb-item",
            "breadcrumb-link",
            "breadcrumb-page",
            "breadcrumb-separator",
            "breadcrumb-ellipsis",
        ],
        &["current-page"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Button",
        "button",
        "/docs/components/button",
        UiShadcnDemoCategory::Foundation,
        "Button",
        "button",
        &["button"],
        &[
            "default",
            "destructive",
            "outline",
            "secondary",
            "ghost",
            "link",
            "disabled",
            "loading",
        ],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Button Group",
        "button-group",
        "/docs/components/button-group",
        UiShadcnDemoCategory::Foundation,
        "ButtonGroup",
        "button_group_node",
        &[
            "button-group",
            "button-group-item",
            "button-group-separator",
        ],
        &["horizontal", "vertical", "attached"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Calendar",
        "calendar",
        "/docs/components/calendar",
        UiShadcnDemoCategory::Form,
        "Calendar",
        "calendar_node",
        &[
            "calendar",
            "calendar-month",
            "calendar-day",
            "calendar-caption",
        ],
        &["selected", "today", "disabled", "range-start", "range-end"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Card",
        "card",
        "/docs/components/card",
        UiShadcnDemoCategory::Layout,
        "Card",
        "card",
        &[
            "card",
            "card-header",
            "card-title",
            "card-description",
            "card-content",
            "card-footer",
        ],
        &["default", "sm"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Carousel",
        "carousel",
        "/docs/components/carousel",
        UiShadcnDemoCategory::Media,
        "Carousel",
        "carousel_node",
        &[
            "carousel",
            "carousel-content",
            "carousel-item",
            "carousel-previous",
            "carousel-next",
        ],
        &["can-scroll-prev", "can-scroll-next"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Chart",
        "chart",
        "/docs/components/chart",
        UiShadcnDemoCategory::DataDisplay,
        "Chart",
        "chart_node",
        &["chart-container", "chart-tooltip", "chart-legend"],
        &["hovered", "active"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Checkbox",
        "checkbox",
        "/docs/components/checkbox",
        UiShadcnDemoCategory::Form,
        "Checkbox",
        "checkbox",
        &["checkbox"],
        &["checked", "unchecked", "indeterminate", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Collapsible",
        "collapsible",
        "/docs/components/collapsible",
        UiShadcnDemoCategory::Layout,
        "Collapsible",
        "collapsible_node",
        &["collapsible", "collapsible-trigger", "collapsible-content"],
        &["open", "closed", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Combobox",
        "combobox",
        "/docs/components/combobox",
        UiShadcnDemoCategory::Form,
        "Combobox",
        "combobox_node",
        &[
            "combobox",
            "popover",
            "command",
            "command-input",
            "command-item",
        ],
        &["open", "closed", "selected", "empty"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Command",
        "command",
        "/docs/components/command",
        UiShadcnDemoCategory::Overlay,
        "Command",
        "command_palette",
        &[
            "command",
            "command-input",
            "command-list",
            "command-group",
            "command-item",
            "command-empty",
        ],
        &["selected", "empty", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Context Menu",
        "context-menu",
        "/docs/components/context-menu",
        UiShadcnDemoCategory::Overlay,
        "ContextMenu",
        "context_menu_node",
        &[
            "context-menu",
            "context-menu-trigger",
            "context-menu-content",
            "context-menu-item",
        ],
        &["open", "closed", "checked", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Data Table",
        "data-table",
        "/docs/components/data-table",
        UiShadcnDemoCategory::DataDisplay,
        "DataTable",
        "data_table_node",
        &[
            "table",
            "table-header",
            "table-body",
            "table-row",
            "table-cell",
        ],
        &["sorted", "selected", "loading", "empty"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Date Picker",
        "date-picker",
        "/docs/components/date-picker",
        UiShadcnDemoCategory::Form,
        "DatePicker",
        "date_picker_node",
        &["popover", "calendar", "button", "field"],
        &["open", "selected", "empty"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Dialog",
        "dialog",
        "/docs/components/dialog",
        UiShadcnDemoCategory::Overlay,
        "Dialog",
        "dialog",
        &[
            "dialog",
            "dialog-trigger",
            "dialog-content",
            "dialog-header",
            "dialog-footer",
        ],
        &["open", "closed", "focus-trap"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Direction",
        "direction",
        "/docs/components/direction",
        UiShadcnDemoCategory::Foundation,
        "DirectionProvider",
        "direction_node",
        &["direction-provider"],
        &["ltr", "rtl"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Drawer",
        "drawer",
        "/docs/components/drawer",
        UiShadcnDemoCategory::Overlay,
        "Drawer",
        "drawer_node",
        &[
            "drawer",
            "drawer-trigger",
            "drawer-content",
            "drawer-header",
            "drawer-footer",
        ],
        &["open", "closed", "dragging"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Dropdown Menu",
        "dropdown-menu",
        "/docs/components/dropdown-menu",
        UiShadcnDemoCategory::Overlay,
        "DropdownMenu",
        "dropdown_menu_node",
        &[
            "dropdown-menu",
            "dropdown-menu-trigger",
            "dropdown-menu-content",
            "dropdown-menu-item",
        ],
        &["open", "closed", "checked", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Empty",
        "empty",
        "/docs/components/empty",
        UiShadcnDemoCategory::Feedback,
        "Empty",
        "empty_state",
        &[
            "empty",
            "empty-header",
            "empty-icon",
            "empty-title",
            "empty-description",
            "empty-content",
        ],
        &["default", "loading"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Field",
        "field",
        "/docs/components/field",
        UiShadcnDemoCategory::Form,
        "Field",
        "field_node",
        &[
            "field",
            "field-label",
            "field-title",
            "field-description",
            "field-error",
        ],
        &["invalid", "disabled", "required"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Hover Card",
        "hover-card",
        "/docs/components/hover-card",
        UiShadcnDemoCategory::Overlay,
        "HoverCard",
        "hover_card_node",
        &["hover-card", "hover-card-trigger", "hover-card-content"],
        &["open", "closed"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Input",
        "input",
        "/docs/components/input",
        UiShadcnDemoCategory::Form,
        "Input",
        "field_node",
        &["input"],
        &["placeholder", "focus", "disabled", "invalid"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Input Group",
        "input-group",
        "/docs/components/input-group",
        UiShadcnDemoCategory::Form,
        "InputGroup",
        "input_group_node",
        &[
            "input-group",
            "input-group-input",
            "input-group-addon",
            "input-group-button",
        ],
        &["focus-within", "disabled", "invalid"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Input OTP",
        "input-otp",
        "/docs/components/input-otp",
        UiShadcnDemoCategory::Form,
        "InputOTP",
        "input_otp_node",
        &[
            "input-otp",
            "input-otp-group",
            "input-otp-slot",
            "input-otp-separator",
        ],
        &["active", "filled", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Item",
        "item",
        "/docs/components/item",
        UiShadcnDemoCategory::DataDisplay,
        "Item",
        "list_row_node",
        &[
            "item",
            "item-media",
            "item-content",
            "item-title",
            "item-description",
            "item-actions",
        ],
        &["selected", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Kbd",
        "kbd",
        "/docs/components/kbd",
        UiShadcnDemoCategory::Foundation,
        "Kbd",
        "kbd_node",
        &["kbd"],
        &[],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Label",
        "label",
        "/docs/components/label",
        UiShadcnDemoCategory::Form,
        "Label",
        "text",
        &["label"],
        &["disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Menubar",
        "menubar",
        "/docs/components/menubar",
        UiShadcnDemoCategory::Navigation,
        "Menubar",
        "menubar_node",
        &[
            "menubar",
            "menubar-menu",
            "menubar-trigger",
            "menubar-content",
            "menubar-item",
        ],
        &["open", "closed", "checked", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Native Select",
        "native-select",
        "/docs/components/native-select",
        UiShadcnDemoCategory::Form,
        "NativeSelect",
        "select_node",
        &["native-select"],
        &["disabled", "invalid"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Navigation Menu",
        "navigation-menu",
        "/docs/components/navigation-menu",
        UiShadcnDemoCategory::Navigation,
        "NavigationMenu",
        "navigation_menu_node",
        &[
            "navigation-menu",
            "navigation-menu-list",
            "navigation-menu-item",
            "navigation-menu-content",
        ],
        &["open", "closed", "active"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Pagination",
        "pagination",
        "/docs/components/pagination",
        UiShadcnDemoCategory::Navigation,
        "Pagination",
        "pagination_node",
        &[
            "pagination",
            "pagination-content",
            "pagination-item",
            "pagination-link",
        ],
        &["active", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Popover",
        "popover",
        "/docs/components/popover",
        UiShadcnDemoCategory::Overlay,
        "Popover",
        "popover_node",
        &[
            "popover",
            "popover-trigger",
            "popover-content",
            "popover-anchor",
        ],
        &["open", "closed"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Progress",
        "progress",
        "/docs/components/progress",
        UiShadcnDemoCategory::Feedback,
        "Progress",
        "progress_bar_node",
        &["progress", "progress-indicator"],
        &["determinate", "indeterminate"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Radio Group",
        "radio-group",
        "/docs/components/radio-group",
        UiShadcnDemoCategory::Form,
        "RadioGroup",
        "radio",
        &["radio-group", "radio-group-item"],
        &["checked", "unchecked", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Resizable",
        "resizable",
        "/docs/components/resizable",
        UiShadcnDemoCategory::Layout,
        "Resizable",
        "resizable_node",
        &[
            "resizable-panel-group",
            "resizable-panel",
            "resizable-handle",
        ],
        &["dragging", "horizontal", "vertical"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Scroll Area",
        "scroll-area",
        "/docs/components/scroll-area",
        UiShadcnDemoCategory::Layout,
        "ScrollArea",
        "scroll_area",
        &[
            "scroll-area",
            "scroll-area-viewport",
            "scroll-area-scrollbar",
            "scroll-area-thumb",
        ],
        &["scrolling", "horizontal", "vertical"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Select",
        "select",
        "/docs/components/select",
        UiShadcnDemoCategory::Form,
        "Select",
        "select_node",
        &[
            "select",
            "select-trigger",
            "select-content",
            "select-item",
            "select-value",
        ],
        &["open", "closed", "selected", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Separator",
        "separator",
        "/docs/components/separator",
        UiShadcnDemoCategory::Layout,
        "Separator",
        "divider",
        &["separator"],
        &["horizontal", "vertical"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Sheet",
        "sheet",
        "/docs/components/sheet",
        UiShadcnDemoCategory::Overlay,
        "Sheet",
        "sheet_node",
        &[
            "sheet",
            "sheet-trigger",
            "sheet-content",
            "sheet-header",
            "sheet-footer",
        ],
        &[
            "open",
            "closed",
            "side-top",
            "side-right",
            "side-bottom",
            "side-left",
        ],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Sidebar",
        "sidebar",
        "/docs/components/sidebar",
        UiShadcnDemoCategory::Navigation,
        "Sidebar",
        "sidebar_node",
        &[
            "sidebar",
            "sidebar-header",
            "sidebar-content",
            "sidebar-footer",
            "sidebar-menu",
        ],
        &["expanded", "collapsed", "mobile", "active"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Skeleton",
        "skeleton",
        "/docs/components/skeleton",
        UiShadcnDemoCategory::Feedback,
        "Skeleton",
        "skeleton",
        &["skeleton"],
        &["loading"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Slider",
        "slider",
        "/docs/components/slider",
        UiShadcnDemoCategory::Form,
        "Slider",
        "slider_node",
        &["slider", "slider-track", "slider-range", "slider-thumb"],
        &["dragging", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Sonner",
        "sonner",
        "/docs/components/sonner",
        UiShadcnDemoCategory::Feedback,
        "Sonner",
        "toast",
        &[
            "toaster",
            "toast",
            "toast-title",
            "toast-description",
            "toast-action",
        ],
        &["success", "info", "warning", "error", "loading"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Switch",
        "switch",
        "/docs/components/switch",
        UiShadcnDemoCategory::Form,
        "Switch",
        "toggle_node",
        &["switch", "switch-thumb"],
        &["checked", "unchecked", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Table",
        "table",
        "/docs/components/table",
        UiShadcnDemoCategory::DataDisplay,
        "Table",
        "table_node",
        &[
            "table",
            "table-header",
            "table-body",
            "table-footer",
            "table-row",
            "table-cell",
        ],
        &["selected", "sortable"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Tabs",
        "tabs",
        "/docs/components/tabs",
        UiShadcnDemoCategory::Navigation,
        "Tabs",
        "tabs_node",
        &["tabs", "tabs-list", "tabs-trigger", "tabs-content"],
        &["active", "inactive", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Textarea",
        "textarea",
        "/docs/components/textarea",
        UiShadcnDemoCategory::Form,
        "Textarea",
        "text_area_node",
        &["textarea"],
        &["placeholder", "focus", "disabled", "invalid"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Toast",
        "toast",
        "/docs/components/toast",
        UiShadcnDemoCategory::Feedback,
        "Toast",
        "toast",
        &[
            "toast",
            "toast-title",
            "toast-description",
            "toast-action",
            "toast-close",
        ],
        &["open", "closed", "success", "destructive"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Toggle",
        "toggle",
        "/docs/components/toggle",
        UiShadcnDemoCategory::Foundation,
        "Toggle",
        "toggle_node",
        &["toggle"],
        &["pressed", "unpressed", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Toggle Group",
        "toggle-group",
        "/docs/components/toggle-group",
        UiShadcnDemoCategory::Foundation,
        "ToggleGroup",
        "toggle_group_node",
        &["toggle-group", "toggle-group-item"],
        &["single", "multiple", "pressed", "disabled"],
        UiShadcnDemoStatus::NativePrimitive,
    ),
    demo(
        "Tooltip",
        "tooltip",
        "/docs/components/tooltip",
        UiShadcnDemoCategory::Overlay,
        "Tooltip",
        "tooltip",
        &["tooltip", "tooltip-trigger", "tooltip-content"],
        &[
            "open",
            "closed",
            "side-top",
            "side-right",
            "side-bottom",
            "side-left",
        ],
        UiShadcnDemoStatus::NativePrimitive,
    ),
];

const fn demo(
    name: &'static str,
    slug: &'static str,
    route: &'static str,
    category: UiShadcnDemoCategory,
    source_component: &'static str,
    edge_builder: &'static str,
    slots: &'static [&'static str],
    states: &'static [&'static str],
    status: UiShadcnDemoStatus,
) -> UiShadcnDemoSpec {
    UiShadcnDemoSpec {
        name,
        slug,
        route,
        category,
        source_component,
        edge_builder,
        slots,
        states,
        status,
    }
}

pub fn find_shadcn_demo_by_slug(slug: &str) -> Option<&'static UiShadcnDemoSpec> {
    SHADCN_DEMO_COMPONENTS
        .iter()
        .find(|component| component.slug == slug)
}

pub fn find_shadcn_demo_by_source_component(
    source_component: &str,
) -> Option<&'static UiShadcnDemoSpec> {
    SHADCN_DEMO_COMPONENTS
        .iter()
        .find(|component| component.source_component == source_component)
}

pub fn shadcn_demos_by_edge_builder(
    edge_builder: &str,
) -> impl Iterator<Item = &'static UiShadcnDemoSpec> + '_ {
    SHADCN_DEMO_COMPONENTS
        .iter()
        .filter(move |component| component.edge_builder == edge_builder)
}

pub fn shadcn_demos_by_category(
    category: UiShadcnDemoCategory,
) -> impl Iterator<Item = &'static UiShadcnDemoSpec> {
    SHADCN_DEMO_COMPONENTS
        .iter()
        .filter(move |component| component.category == category)
}

pub fn shadcn_demos_using_slot(slot: &str) -> impl Iterator<Item = &'static UiShadcnDemoSpec> + '_ {
    SHADCN_DEMO_COMPONENTS
        .iter()
        .filter(move |component| component.uses_slot(slot))
}

pub fn shadcn_demos_using_state(
    state: &str,
) -> impl Iterator<Item = &'static UiShadcnDemoSpec> + '_ {
    SHADCN_DEMO_COMPONENTS
        .iter()
        .filter(move |component| component.uses_state(state))
}

pub fn resolve_shadcn_demo_identifier(identifier: &str) -> Option<UiShadcnResolvedDemo> {
    if let Some(spec) = find_shadcn_demo_by_source_component(identifier.trim()) {
        return Some(UiShadcnResolvedDemo {
            spec,
            kind: UiShadcnResolveKind::SourceComponent,
        });
    }

    let (candidate, from_path) = normalize_identifier(identifier);
    let candidate = candidate.as_str();

    if let Some(spec) = find_shadcn_demo_by_slug(candidate) {
        return Some(UiShadcnResolvedDemo {
            spec,
            kind: if from_path {
                UiShadcnResolveKind::ModulePath
            } else {
                UiShadcnResolveKind::Slug
            },
        });
    }

    if let Some(spec) = find_shadcn_demo_by_source_component(candidate) {
        return Some(UiShadcnResolvedDemo {
            spec,
            kind: UiShadcnResolveKind::SourceComponent,
        });
    }

    if let Some(spec) = shadcn_demos_using_slot(candidate).next() {
        return Some(UiShadcnResolvedDemo {
            spec,
            kind: UiShadcnResolveKind::Slot,
        });
    }

    None
}

pub fn shadcn_port_mapping_for_identifier(identifier: &'static str) -> Option<UiShadcnPortMapping> {
    let resolved = resolve_shadcn_demo_identifier(identifier)?;
    Some(resolved.spec.port_mapping(identifier, resolved.kind))
}

pub const fn shadcn_port_manifest(identifiers: &'static [&'static str]) -> UiShadcnPortManifest {
    UiShadcnPortManifest::new(identifiers)
}

pub fn shadcn_native_demo_count() -> usize {
    SHADCN_DEMO_COMPONENTS
        .iter()
        .filter(|component| component.has_native_renderer())
        .count()
}

fn normalize_identifier(identifier: &str) -> (String, bool) {
    let mut candidate = identifier.trim();
    if let Some(value) = candidate.strip_prefix("data-slot=") {
        candidate = value.trim_matches('"').trim_matches('\'');
    }
    let from_path = candidate.contains('/');
    if let Some((_, last)) = candidate.rsplit_once('/') {
        candidate = last;
    }
    candidate = candidate
        .strip_suffix(".tsx")
        .or_else(|| candidate.strip_suffix(".ts"))
        .or_else(|| candidate.strip_suffix(".jsx"))
        .or_else(|| candidate.strip_suffix(".js"))
        .unwrap_or(candidate);

    let normalized = if candidate
        .bytes()
        .any(|byte| byte.is_ascii_uppercase() || byte == b'_' || byte == b' ')
    {
        pascal_or_snake_to_kebab(candidate)
    } else {
        candidate.to_ascii_lowercase()
    };
    (normalized, from_path)
}

fn pascal_or_snake_to_kebab(identifier: &str) -> String {
    let mut out = String::new();
    let mut previous_was_separator = true;
    for byte in identifier.bytes() {
        if byte == b'_' || byte == b' ' {
            if !previous_was_separator {
                out.push('-');
            }
            previous_was_separator = true;
            continue;
        }
        if byte.is_ascii_uppercase() {
            if !previous_was_separator {
                out.push('-');
            }
            out.push((byte + 32) as char);
            previous_was_separator = false;
            continue;
        }
        out.push(byte.to_ascii_lowercase() as char);
        previous_was_separator = byte == b'-';
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_covers_shadcn_sidebar_components() {
        assert_eq!(SHADCN_DEMO_COMPONENTS.len(), 57);
        assert!(find_shadcn_demo_by_slug("accordion").is_some());
        assert!(find_shadcn_demo_by_slug("tooltip").is_some());
        assert!(find_shadcn_demo_by_slug("data-table").is_some());
        assert!(find_shadcn_demo_by_slug("input-otp").is_some());
    }

    #[test]
    fn catalog_supports_porting_surface_queries() {
        assert_eq!(
            find_shadcn_demo_by_source_component("InputGroup")
                .unwrap()
                .slug,
            "input-group"
        );
        assert!(shadcn_demos_by_edge_builder("button").any(|component| component.slug == "button"));
        assert!(
            shadcn_demos_using_slot("dialog-content").any(|component| component.slug == "dialog")
        );
        assert!(shadcn_demos_using_state("disabled").any(|component| component.slug == "button"));
    }

    #[test]
    fn catalog_resolves_common_migration_identifiers() {
        assert_eq!(
            resolve_shadcn_demo_identifier("button").unwrap().spec.slug,
            "button"
        );
        assert_eq!(
            resolve_shadcn_demo_identifier("Button").unwrap().kind,
            UiShadcnResolveKind::SourceComponent
        );
        assert_eq!(
            resolve_shadcn_demo_identifier("@/components/ui/input-otp")
                .unwrap()
                .kind,
            UiShadcnResolveKind::ModulePath
        );
        assert_eq!(
            resolve_shadcn_demo_identifier("components/ui/card.tsx")
                .unwrap()
                .spec
                .slug,
            "card"
        );
        assert_eq!(
            resolve_shadcn_demo_identifier("data-slot=\"card-header\"")
                .unwrap()
                .spec
                .slug,
            "card"
        );
        assert_eq!(
            resolve_shadcn_demo_identifier("CardHeader").unwrap().kind,
            UiShadcnResolveKind::Slot
        );
        assert!(resolve_shadcn_demo_identifier("UnknownThing").is_none());
    }

    #[test]
    fn catalog_builds_stable_port_mapping_rows() {
        let mapping = shadcn_port_mapping_for_identifier("@/components/ui/button").unwrap();

        assert_eq!(mapping.identifier, "@/components/ui/button");
        assert_eq!(mapping.resolve_kind, UiShadcnResolveKind::ModulePath);
        assert_eq!(mapping.slug, "button");
        assert_eq!(mapping.source_component, "Button");
        assert_eq!(mapping.edge_builder, "button");
        assert_eq!(mapping.category, UiShadcnDemoCategory::Foundation);
        assert_eq!(mapping.status, UiShadcnDemoStatus::NativePrimitive);
        assert!(mapping.native_renderer);
        assert!(!mapping.exact_port);
        assert!(shadcn_port_mapping_for_identifier("UnknownThing").is_none());
    }

    #[test]
    fn catalog_builds_batch_port_manifest_reports() {
        const IDS: &[&str] = &[
            "@/components/ui/button",
            "CardHeader",
            "data-slot=\"dialog-content\"",
            "UnknownThing",
        ];
        let manifest = shadcn_port_manifest(IDS);
        let slugs: Vec<&str> = manifest.mappings().map(|mapping| mapping.slug).collect();
        let missing: Vec<&str> = manifest.missing_identifiers().collect();

        assert_eq!(manifest.identifier_count(), 4);
        assert_eq!(manifest.resolved_count(), 3);
        assert_eq!(manifest.missing_count(), 1);
        assert!(!manifest.complete());
        assert_eq!(manifest.native_renderer_count(), 3);
        assert_eq!(manifest.exact_port_count(), 0);
        assert_eq!(
            manifest.count_by_category(UiShadcnDemoCategory::Foundation),
            1
        );
        assert_eq!(manifest.count_by_category(UiShadcnDemoCategory::Layout), 1);
        assert_eq!(
            manifest.count_by_status(UiShadcnDemoStatus::NativePrimitive),
            3
        );
        assert_eq!(
            manifest
                .category_summary(UiShadcnDemoCategory::Overlay)
                .count,
            1
        );
        assert_eq!(
            manifest.status_summary(UiShadcnDemoStatus::Cataloged).count,
            0
        );
        assert!(manifest
            .category_summaries()
            .iter()
            .any(
                |summary| summary.category == UiShadcnDemoCategory::Foundation
                    && summary.count == 1
            ));
        assert!(manifest
            .status_summaries()
            .iter()
            .any(
                |summary| summary.status == UiShadcnDemoStatus::NativePrimitive
                    && summary.count == 3
            ));
        assert_eq!(slugs, ["button", "card", "dialog"]);
        assert_eq!(missing, ["UnknownThing"]);
    }

    #[test]
    fn every_demo_has_stable_docs_route_and_builder() {
        for component in SHADCN_DEMO_COMPONENTS {
            assert!(component.route.starts_with("/docs/components/"));
            assert!(!component.edge_builder.is_empty());
            assert!(!component.source_component.is_empty());
        }
    }

    #[test]
    fn native_primitive_count_tracks_porting_progress() {
        assert_eq!(shadcn_native_demo_count(), 57);
    }
}
