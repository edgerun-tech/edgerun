//! Typed shadcn-compatible production props.

use std::vec::Vec;

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnPropsSurface {
    pub slug: &'static str,
    pub source_component: &'static str,
    pub props_type: &'static str,
    pub exact_builder: &'static str,
    pub stateful: bool,
    pub event_adapter: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnPropsSurfaceManifest {
    pub surfaces: &'static [UiShadcnPropsSurface],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnPropsSurfaceSummary {
    pub total: usize,
    pub stateful: usize,
    pub event_adapted: usize,
    pub static_only: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnResolvedPropsSurface {
    pub surface: &'static UiShadcnPropsSurface,
    pub demo: &'static UiShadcnDemoSpec,
    pub parity: Option<UiShadcnParityContract>,
    pub resolve_kind: UiShadcnResolveKind,
}

impl UiShadcnPropsSurface {
    pub const fn new(
        slug: &'static str,
        source_component: &'static str,
        props_type: &'static str,
        exact_builder: &'static str,
    ) -> Self {
        Self {
            slug,
            source_component,
            props_type,
            exact_builder,
            stateful: false,
            event_adapter: false,
        }
    }

    pub const fn stateful(mut self) -> Self {
        self.stateful = true;
        self
    }

    pub const fn event_adapter(mut self) -> Self {
        self.event_adapter = true;
        self
    }

    pub fn demo_spec(self) -> Option<&'static UiShadcnDemoSpec> {
        find_shadcn_demo_by_slug(self.slug)
    }

    pub fn parity_contract(self) -> Option<UiShadcnParityContract> {
        shadcn_parity_contract_for_slug(self.slug)
    }

    pub fn docs_route(self) -> Option<&'static str> {
        self.demo_spec().map(|demo| demo.route)
    }

    pub fn category(self) -> Option<UiShadcnDemoCategory> {
        self.demo_spec().map(|demo| demo.category)
    }

    pub fn slots(self) -> &'static [&'static str] {
        self.demo_spec().map(|demo| demo.slots).unwrap_or(&[])
    }

    pub fn states(self) -> &'static [&'static str] {
        self.demo_spec().map(|demo| demo.states).unwrap_or(&[])
    }
}

impl UiShadcnPropsSurfaceManifest {
    pub const fn new(surfaces: &'static [UiShadcnPropsSurface]) -> Self {
        Self { surfaces }
    }

    pub const fn total(self) -> usize {
        self.surfaces.len()
    }

    pub fn stateful(self) -> impl Iterator<Item = &'static UiShadcnPropsSurface> {
        self.surfaces.iter().filter(|surface| surface.stateful)
    }

    pub fn event_adapted(self) -> impl Iterator<Item = &'static UiShadcnPropsSurface> {
        self.surfaces.iter().filter(|surface| surface.event_adapter)
    }

    pub fn static_only(self) -> impl Iterator<Item = &'static UiShadcnPropsSurface> {
        self.surfaces
            .iter()
            .filter(|surface| !surface.stateful && !surface.event_adapter)
    }

    pub fn find_by_slug(self, slug: &str) -> Option<&'static UiShadcnPropsSurface> {
        self.surfaces.iter().find(|surface| surface.slug == slug)
    }

    pub fn find_by_source_component(
        self,
        source_component: &str,
    ) -> Option<&'static UiShadcnPropsSurface> {
        self.surfaces
            .iter()
            .find(|surface| surface.source_component == source_component)
    }

    pub fn summary(self) -> UiShadcnPropsSurfaceSummary {
        let stateful = self.stateful().count();
        let event_adapted = self.event_adapted().count();
        let static_only = self.static_only().count();
        UiShadcnPropsSurfaceSummary {
            total: self.total(),
            stateful,
            event_adapted,
            static_only,
        }
    }

    pub fn resolved(self) -> impl Iterator<Item = UiShadcnResolvedPropsSurface> {
        self.surfaces.iter().filter_map(|surface| {
            let demo = surface.demo_spec()?;
            Some(UiShadcnResolvedPropsSurface {
                surface,
                demo,
                parity: demo.parity_contract(),
                resolve_kind: UiShadcnResolveKind::Slug,
            })
        })
    }
}

pub const SHADCN_PROPS_SURFACES: &[UiShadcnPropsSurface] = &[
    UiShadcnPropsSurface::new(
        "accordion",
        "Accordion",
        "UiShadcnAccordionProps",
        "shadcn_accordion",
    ),
    UiShadcnPropsSurface::new("alert", "Alert", "UiShadcnAlertProps", "shadcn_alert"),
    UiShadcnPropsSurface::new(
        "alert-dialog",
        "AlertDialog",
        "UiShadcnAlertDialogProps",
        "shadcn_alert_dialog",
    ),
    UiShadcnPropsSurface::new(
        "aspect-ratio",
        "AspectRatio",
        "UiShadcnAspectRatioProps",
        "shadcn_aspect_ratio",
    ),
    UiShadcnPropsSurface::new("avatar", "Avatar", "UiShadcnAvatarProps", "shadcn_avatar"),
    UiShadcnPropsSurface::new("badge", "Badge", "UiShadcnBadgeProps", "shadcn_badge"),
    UiShadcnPropsSurface::new(
        "breadcrumb",
        "Breadcrumb",
        "UiShadcnBreadcrumbProps",
        "shadcn_breadcrumb",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new("button", "Button", "UiShadcnButtonProps", "shadcn_button")
        .event_adapter(),
    UiShadcnPropsSurface::new(
        "button-group",
        "ButtonGroup",
        "UiShadcnButtonGroupProps",
        "shadcn_button_group",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "calendar",
        "Calendar",
        "UiShadcnCalendarProps",
        "shadcn_calendar",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new("card", "Card", "UiShadcnCardProps", "shadcn_card"),
    UiShadcnPropsSurface::new(
        "carousel",
        "Carousel",
        "UiShadcnCarouselProps",
        "shadcn_carousel",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new("chart", "Chart", "UiShadcnChartProps", "shadcn_chart"),
    UiShadcnPropsSurface::new(
        "checkbox",
        "Checkbox",
        "UiShadcnCheckboxProps",
        "shadcn_checkbox",
    )
    .stateful()
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "collapsible",
        "Collapsible",
        "UiShadcnCollapsibleProps",
        "shadcn_collapsible",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "combobox",
        "Combobox",
        "UiShadcnComboboxProps",
        "shadcn_combobox",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "command",
        "Command",
        "UiShadcnCommandProps",
        "shadcn_command",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "context-menu",
        "ContextMenu",
        "UiShadcnContextMenuProps",
        "shadcn_context_menu",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "data-table",
        "DataTable",
        "UiShadcnDataTableProps",
        "shadcn_data_table",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "date-picker",
        "DatePicker",
        "UiShadcnDatePickerProps",
        "shadcn_date_picker",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new("dialog", "Dialog", "UiShadcnDialogProps", "shadcn_dialog")
        .stateful(),
    UiShadcnPropsSurface::new(
        "direction",
        "DirectionProvider",
        "UiShadcnDirectionProps",
        "shadcn_direction",
    ),
    UiShadcnPropsSurface::new("drawer", "Drawer", "UiShadcnDrawerProps", "shadcn_drawer")
        .stateful()
        .event_adapter(),
    UiShadcnPropsSurface::new(
        "dropdown-menu",
        "DropdownMenu",
        "UiShadcnDropdownMenuProps",
        "shadcn_dropdown_menu",
    )
    .stateful()
    .event_adapter(),
    UiShadcnPropsSurface::new("empty", "Empty", "UiShadcnEmptyProps", "shadcn_empty"),
    UiShadcnPropsSurface::new("field", "Field", "UiShadcnFieldProps", "shadcn_field")
        .event_adapter(),
    UiShadcnPropsSurface::new(
        "hover-card",
        "HoverCard",
        "UiShadcnHoverCardProps",
        "shadcn_hover_card",
    ),
    UiShadcnPropsSurface::new("input", "Input", "UiShadcnInputProps", "shadcn_input")
        .event_adapter(),
    UiShadcnPropsSurface::new(
        "input-group",
        "InputGroup",
        "UiShadcnInputGroupProps",
        "shadcn_input_group",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "input-otp",
        "InputOTP",
        "UiShadcnInputOtpProps",
        "shadcn_input_otp",
    )
    .stateful()
    .event_adapter(),
    UiShadcnPropsSurface::new("item", "Item", "UiShadcnItemProps", "shadcn_item").event_adapter(),
    UiShadcnPropsSurface::new("kbd", "Kbd", "UiShadcnKbdProps", "shadcn_kbd"),
    UiShadcnPropsSurface::new("label", "Label", "UiShadcnLabelProps", "shadcn_label"),
    UiShadcnPropsSurface::new(
        "menubar",
        "Menubar",
        "UiShadcnMenubarProps",
        "shadcn_menubar",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "native-select",
        "NativeSelect",
        "UiShadcnNativeSelectProps",
        "shadcn_native_select",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "navigation-menu",
        "NavigationMenu",
        "UiShadcnNavigationMenuProps",
        "shadcn_navigation_menu",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "pagination",
        "Pagination",
        "UiShadcnPaginationProps",
        "shadcn_pagination",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "popover",
        "Popover",
        "UiShadcnPopoverProps",
        "shadcn_popover",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "progress",
        "Progress",
        "UiShadcnProgressProps",
        "shadcn_progress",
    )
    .stateful(),
    UiShadcnPropsSurface::new(
        "radio-group",
        "RadioGroup",
        "UiShadcnRadioGroupProps",
        "shadcn_radio_group",
    )
    .stateful()
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "resizable",
        "Resizable",
        "UiShadcnResizableProps",
        "shadcn_resizable",
    ),
    UiShadcnPropsSurface::new(
        "scroll-area",
        "ScrollArea",
        "UiShadcnScrollAreaProps",
        "shadcn_scroll_area",
    )
    .stateful()
    .event_adapter(),
    UiShadcnPropsSurface::new("select", "Select", "UiShadcnSelectProps", "shadcn_select")
        .stateful()
        .event_adapter(),
    UiShadcnPropsSurface::new(
        "separator",
        "Separator",
        "UiShadcnSeparatorProps",
        "shadcn_separator",
    ),
    UiShadcnPropsSurface::new("sheet", "Sheet", "UiShadcnSheetProps", "shadcn_sheet")
        .event_adapter(),
    UiShadcnPropsSurface::new(
        "sidebar",
        "Sidebar",
        "UiShadcnSidebarProps",
        "shadcn_sidebar",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "skeleton",
        "Skeleton",
        "UiShadcnSkeletonProps",
        "shadcn_skeleton",
    ),
    UiShadcnPropsSurface::new("slider", "Slider", "UiShadcnSliderProps", "shadcn_slider")
        .stateful()
        .event_adapter(),
    UiShadcnPropsSurface::new("sonner", "Sonner", "UiShadcnSonnerProps", "shadcn_sonner"),
    UiShadcnPropsSurface::new("switch", "Switch", "UiShadcnSwitchProps", "shadcn_switch")
        .stateful()
        .event_adapter(),
    UiShadcnPropsSurface::new("table", "Table", "UiShadcnTableProps", "shadcn_table")
        .event_adapter(),
    UiShadcnPropsSurface::new("tabs", "Tabs", "UiShadcnTabsProps", "shadcn_tabs")
        .stateful()
        .event_adapter(),
    UiShadcnPropsSurface::new(
        "textarea",
        "Textarea",
        "UiShadcnTextareaProps",
        "shadcn_textarea",
    )
    .stateful()
    .event_adapter(),
    UiShadcnPropsSurface::new("toast", "Toast", "UiShadcnToastProps", "shadcn_toast"),
    UiShadcnPropsSurface::new("toggle", "Toggle", "UiShadcnToggleProps", "shadcn_toggle")
        .stateful()
        .event_adapter(),
    UiShadcnPropsSurface::new(
        "toggle-group",
        "ToggleGroup",
        "UiShadcnToggleGroupProps",
        "shadcn_toggle_group",
    )
    .event_adapter(),
    UiShadcnPropsSurface::new(
        "tooltip",
        "Tooltip",
        "UiShadcnTooltipProps",
        "shadcn_tooltip",
    ),
];

pub const SHADCN_PROPS_SURFACE_MANIFEST: UiShadcnPropsSurfaceManifest =
    UiShadcnPropsSurfaceManifest::new(SHADCN_PROPS_SURFACES);

pub fn shadcn_props_surface_for_slug(slug: &str) -> Option<&'static UiShadcnPropsSurface> {
    SHADCN_PROPS_SURFACE_MANIFEST.find_by_slug(slug)
}

pub fn shadcn_props_surface_for_source_component(
    source_component: &str,
) -> Option<&'static UiShadcnPropsSurface> {
    SHADCN_PROPS_SURFACE_MANIFEST.find_by_source_component(source_component)
}

pub fn shadcn_props_surface_count() -> usize {
    SHADCN_PROPS_SURFACE_MANIFEST.total()
}

pub fn shadcn_stateful_props_surfaces() -> impl Iterator<Item = &'static UiShadcnPropsSurface> {
    SHADCN_PROPS_SURFACE_MANIFEST.stateful()
}

pub fn shadcn_event_adapted_props_surfaces() -> impl Iterator<Item = &'static UiShadcnPropsSurface>
{
    SHADCN_PROPS_SURFACE_MANIFEST.event_adapted()
}

pub fn shadcn_static_props_surfaces() -> impl Iterator<Item = &'static UiShadcnPropsSurface> {
    SHADCN_PROPS_SURFACE_MANIFEST.static_only()
}

pub fn shadcn_props_surface_summary() -> UiShadcnPropsSurfaceSummary {
    SHADCN_PROPS_SURFACE_MANIFEST.summary()
}

pub fn shadcn_resolved_props_surfaces() -> impl Iterator<Item = UiShadcnResolvedPropsSurface> {
    SHADCN_PROPS_SURFACE_MANIFEST.resolved()
}

pub fn resolve_shadcn_props_surface(identifier: &str) -> Option<UiShadcnResolvedPropsSurface> {
    let resolved = resolve_shadcn_demo_identifier(identifier)?;
    let surface = shadcn_props_surface_for_slug(resolved.spec.slug)?;
    Some(UiShadcnResolvedPropsSurface {
        surface,
        demo: resolved.spec,
        parity: resolved.spec.parity_contract(),
        resolve_kind: resolved.kind,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnButtonProps<'a> {
    pub label: &'a str,
    pub id: u32,
    pub variant: UiShadcnButtonVariant,
    pub size: UiShadcnButtonSize,
    pub disabled: bool,
    pub loading: bool,
}

impl<'a> UiShadcnButtonProps<'a> {
    pub const fn new(label: &'a str, id: u32) -> Self {
        Self {
            label,
            id,
            variant: UiShadcnButtonVariant::Default,
            size: UiShadcnButtonSize::Default,
            disabled: false,
            loading: false,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_button(self.label, self.id, self.variant, self.size)
            .disabled(self.disabled || self.loading)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnButtonGroupProps<'a> {
    pub labels: &'a [&'a str],
    pub base_id: u32,
}

impl<'a> UiShadcnButtonGroupProps<'a> {
    pub const fn new(labels: &'a [&'a str], base_id: u32) -> Self {
        Self { labels, base_id }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_button_group(self.labels, self.base_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnBadgeProps<'a> {
    pub label: &'a str,
    pub variant: UiShadcnBadgeVariant,
}

impl<'a> UiShadcnBadgeProps<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            variant: UiShadcnBadgeVariant::Default,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_badge(self.label, self.variant)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnAccordionProps<'a> {
    pub items: &'a [(&'a str, &'a str)],
    pub base_id: u32,
}

impl<'a> UiShadcnAccordionProps<'a> {
    pub const fn new(items: &'a [(&'a str, &'a str)], base_id: u32) -> Self {
        Self { items, base_id }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_accordion(self.items, self.base_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnAlertProps<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub icon: UiIcon,
}

impl<'a> UiShadcnAlertProps<'a> {
    pub const fn new(title: &'a str, body: &'a str) -> Self {
        Self {
            title,
            body,
            icon: UiIcon::Warning,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_alert(self.title, self.body, self.icon)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnAlertDialogProps<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub icon: UiIcon,
}

impl<'a> UiShadcnAlertDialogProps<'a> {
    pub const fn new(title: &'a str, body: &'a str) -> Self {
        Self {
            title,
            body,
            icon: UiIcon::Warning,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_alert_dialog(self.title, self.body, self.icon)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnAspectRatioProps<'a> {
    pub label: &'a str,
    pub icon: UiIcon,
}

impl<'a> UiShadcnAspectRatioProps<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            icon: UiIcon::Eye,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_aspect_ratio(self.label, self.icon)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnAvatarProps<'a> {
    pub label: &'a str,
    pub color: Color4,
}

impl<'a> UiShadcnAvatarProps<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            color: palette::ACCENT,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_avatar(self.label, self.color)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnFieldProps<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub helper: &'a str,
    pub id: Option<u32>,
    pub disabled: bool,
    pub invalid: bool,
}

impl<'a> UiShadcnFieldProps<'a> {
    pub const fn new(label: &'a str, value: &'a str) -> Self {
        Self {
            label,
            value,
            helper: "",
            id: None,
            disabled: false,
            invalid: false,
        }
    }

    pub fn to_node(self) -> UiNode {
        let mut node = field_node(self.label, self.value).detail(self.helper);
        if let Some(id) = self.id {
            node = node.hit_id(id);
        }
        node.disabled(self.disabled).accent(if self.invalid {
            palette::DANGER
        } else {
            palette::ACCENT
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnCheckboxProps<'a> {
    pub label: &'a str,
    pub id: u32,
    pub checked: bool,
    pub disabled: bool,
}

impl<'a> UiShadcnCheckboxProps<'a> {
    pub const fn new(label: &'a str, id: u32) -> Self {
        Self {
            label,
            id,
            checked: false,
            disabled: false,
        }
    }

    pub fn checked_with_state(self, state: Option<&UiRuntimeState>) -> bool {
        state
            .map(|state| state.toggle_value(self.id, self.checked))
            .unwrap_or(self.checked)
    }

    pub fn to_node(self) -> UiNode {
        shadcn_checkbox(self.label, self.checked, self.id).disabled(self.disabled)
    }

    pub fn event_from_action(self, action: &UiAction) -> UiShadcnEvent {
        shadcn_event_from_action(action)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnRadioOption<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub disabled: bool,
}

impl<'a> UiShadcnRadioOption<'a> {
    pub const fn new(label: &'a str, value: &'a str) -> Self {
        Self {
            label,
            value,
            disabled: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnRadioGroupProps<'a> {
    pub options: &'a [UiShadcnRadioOption<'a>],
    pub value: &'a str,
    pub base_id: u32,
}

impl<'a> UiShadcnRadioGroupProps<'a> {
    pub const fn new(options: &'a [UiShadcnRadioOption<'a>], value: &'a str, base_id: u32) -> Self {
        Self {
            options,
            value,
            base_id,
        }
    }

    pub fn selected_index(self) -> Option<usize> {
        self.options
            .iter()
            .position(|option| option.value == self.value)
    }

    pub fn event_context(self) -> UiShadcnEventContext {
        let values = self
            .options
            .iter()
            .map(|option| option.value)
            .collect::<Vec<_>>();
        UiShadcnEventContext::new().radio_options(self.base_id, &values)
    }

    pub fn event_from_action(self, action: &UiAction) -> UiShadcnEvent {
        shadcn_event_from_action_with_context(action, Some(&self.event_context()))
    }

    pub fn to_node(self) -> UiNode {
        let mut node = column("gap-2");
        for (index, option) in self.options.iter().enumerate() {
            node = node.child(
                radio(
                    option.label,
                    option.value == self.value,
                    self.base_id + index as u32,
                )
                .disabled(option.disabled),
            );
        }
        node
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnCalendarProps<'a> {
    pub month: &'a str,
    pub days: &'a [&'a str],
    pub selected: usize,
    pub base_id: u32,
}

impl<'a> UiShadcnCalendarProps<'a> {
    pub const fn new(month: &'a str, days: &'a [&'a str], base_id: u32) -> Self {
        Self {
            month,
            days,
            selected: 0,
            base_id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_calendar(self.month, self.days, self.selected, self.base_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnCarouselProps<'a> {
    pub items: &'a [&'a str],
    pub base_id: u32,
}

impl<'a> UiShadcnCarouselProps<'a> {
    pub const fn new(items: &'a [&'a str], base_id: u32) -> Self {
        Self { items, base_id }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_carousel(self.items, self.base_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnChartProps<'a> {
    pub title: &'a str,
    pub labels: &'a [&'a str],
    pub values: &'a [f32],
}

impl<'a> UiShadcnChartProps<'a> {
    pub const fn new(title: &'a str, labels: &'a [&'a str], values: &'a [f32]) -> Self {
        Self {
            title,
            labels,
            values,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_chart(self.title, self.labels, self.values)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnCollapsibleProps<'a> {
    pub title: &'a str,
    pub rows: &'a [(&'a str, &'a str)],
    pub base_id: u32,
}

impl<'a> UiShadcnCollapsibleProps<'a> {
    pub const fn new(title: &'a str, rows: &'a [(&'a str, &'a str)], base_id: u32) -> Self {
        Self {
            title,
            rows,
            base_id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_collapsible(self.title, self.rows, self.base_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnCommandProps<'a> {
    pub placeholder: &'a str,
    pub id: u32,
}

impl<'a> UiShadcnCommandProps<'a> {
    pub const fn new(placeholder: &'a str, id: u32) -> Self {
        Self { placeholder, id }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_command(self.placeholder, self.id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnComboboxProps<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub placeholder: &'a str,
    pub options: &'a [&'a str],
    pub selected: usize,
    pub base_id: u32,
}

impl<'a> UiShadcnComboboxProps<'a> {
    pub const fn new(
        label: &'a str,
        value: &'a str,
        placeholder: &'a str,
        options: &'a [&'a str],
        base_id: u32,
    ) -> Self {
        Self {
            label,
            value,
            placeholder,
            options,
            selected: 0,
            base_id,
        }
    }

    pub fn selected_value(self) -> Option<&'a str> {
        self.options.get(self.selected).copied()
    }

    pub fn event_context(self) -> UiShadcnEventContext {
        UiShadcnEventContext::new().select_options(self.base_id + 2, self.options)
    }

    pub fn event_from_action(self, action: &UiAction) -> UiShadcnEvent {
        shadcn_event_from_action_with_context(action, Some(&self.event_context()))
    }

    pub fn to_node(self) -> UiNode {
        shadcn_combobox(
            self.label,
            self.value,
            self.placeholder,
            self.options,
            self.selected,
            self.base_id,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnInputProps<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub id: u32,
    pub placeholder: &'a str,
    pub disabled: bool,
    pub invalid: bool,
}

impl<'a> UiShadcnInputProps<'a> {
    pub const fn new(label: &'a str, value: &'a str, id: u32) -> Self {
        Self {
            label,
            value,
            id,
            placeholder: "",
            disabled: false,
            invalid: false,
        }
    }

    pub fn to_node(self) -> UiNode {
        let value = if self.value.is_empty() {
            self.placeholder
        } else {
            self.value
        };
        UiShadcnFieldProps {
            label: self.label,
            value,
            helper: "",
            id: Some(self.id),
            disabled: self.disabled,
            invalid: self.invalid,
        }
        .to_node()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnContextMenuProps<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    pub items: &'a [(&'a str, &'a str, bool)],
    pub base_id: u32,
}

impl<'a> UiShadcnContextMenuProps<'a> {
    pub const fn new(
        title: &'a str,
        detail: &'a str,
        items: &'a [(&'a str, &'a str, bool)],
        base_id: u32,
    ) -> Self {
        Self {
            title,
            detail,
            items,
            base_id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_context_menu(self.title, self.detail, self.items, self.base_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnDatePickerProps<'a> {
    pub label: &'a str,
    pub month: &'a str,
    pub days: &'a [&'a str],
    pub selected: usize,
    pub base_id: u32,
}

impl<'a> UiShadcnDatePickerProps<'a> {
    pub const fn new(label: &'a str, month: &'a str, days: &'a [&'a str], base_id: u32) -> Self {
        Self {
            label,
            month,
            days,
            selected: 0,
            base_id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_date_picker(
            self.label,
            self.month,
            self.days,
            self.selected,
            self.base_id,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnDataTableProps<'a> {
    pub headers: &'a [&'a str],
    pub rows: &'a [&'a [&'a str]],
    pub id_base: u32,
}

impl<'a> UiShadcnDataTableProps<'a> {
    pub const fn new(headers: &'a [&'a str], rows: &'a [&'a [&'a str]], id_base: u32) -> Self {
        Self {
            headers,
            rows,
            id_base,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_data_table(self.headers, self.rows, self.id_base)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnDirectionProps<'a> {
    pub ltr: &'a str,
    pub rtl: &'a str,
}

impl<'a> UiShadcnDirectionProps<'a> {
    pub const fn new(ltr: &'a str, rtl: &'a str) -> Self {
        Self { ltr, rtl }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_direction(self.ltr, self.rtl)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnDrawerProps<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    pub slider_label: &'a str,
    pub value: f32,
    pub base_id: u32,
}

impl<'a> UiShadcnDrawerProps<'a> {
    pub const fn new(
        title: &'a str,
        detail: &'a str,
        slider_label: &'a str,
        value: f32,
        base_id: u32,
    ) -> Self {
        Self {
            title,
            detail,
            slider_label,
            value,
            base_id,
        }
    }

    pub fn value_with_state(self, state: Option<&UiRuntimeState>) -> f32 {
        state
            .map(|state| state.slider_value(self.base_id, self.value))
            .unwrap_or(self.value)
    }

    pub fn to_node(self) -> UiNode {
        shadcn_drawer(
            self.title,
            self.detail,
            self.slider_label,
            self.value,
            self.base_id,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnEmptyProps<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub icon: UiIcon,
}

impl<'a> UiShadcnEmptyProps<'a> {
    pub const fn new(title: &'a str, body: &'a str) -> Self {
        Self {
            title,
            body,
            icon: UiIcon::Search,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_empty(self.title, self.body, self.icon)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnHoverCardProps<'a> {
    pub label: &'a str,
    pub detail: &'a str,
    pub body: &'a str,
    pub color: Color4,
}

impl<'a> UiShadcnHoverCardProps<'a> {
    pub const fn new(label: &'a str, detail: &'a str, body: &'a str) -> Self {
        Self {
            label,
            detail,
            body,
            color: palette::ACCENT,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_hover_card(self.label, self.detail, self.body, self.color)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnInputGroupProps<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub button_label: &'a str,
    pub id: u32,
}

impl<'a> UiShadcnInputGroupProps<'a> {
    pub const fn new(label: &'a str, value: &'a str, button_label: &'a str, id: u32) -> Self {
        Self {
            label,
            value,
            button_label,
            id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_input_group(self.label, self.value, self.button_label, self.id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnInputOtpProps<'a> {
    pub values: &'a [&'a str],
    pub focused_index: usize,
}

impl<'a> UiShadcnInputOtpProps<'a> {
    pub const fn new(values: &'a [&'a str]) -> Self {
        Self {
            values,
            focused_index: 0,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_input_otp(self.values, self.focused_index)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnSelectOption<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub disabled: bool,
}

impl<'a> UiShadcnSelectOption<'a> {
    pub const fn new(label: &'a str, value: &'a str) -> Self {
        Self {
            label,
            value,
            disabled: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnSelectProps<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub id: u32,
    pub options: &'a [UiShadcnSelectOption<'a>],
    pub base_option_id: u32,
    pub disabled: bool,
    pub open: bool,
}

impl<'a> UiShadcnSelectProps<'a> {
    pub const fn new(label: &'a str, value: &'a str, id: u32) -> Self {
        Self {
            label,
            value,
            id,
            options: &[],
            base_option_id: id + 1,
            disabled: false,
            open: false,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_select(self.label, self.value, self.id).disabled(self.disabled)
    }

    pub fn open_with_state(self, state: Option<&UiRuntimeState>) -> bool {
        state
            .map(|state| state.open_value(self.id, self.open))
            .unwrap_or(self.open)
    }

    pub fn with_options(
        mut self,
        options: &'a [UiShadcnSelectOption<'a>],
        base_option_id: u32,
    ) -> Self {
        self.options = options;
        self.base_option_id = base_option_id;
        self
    }

    pub fn selected_index(self) -> Option<usize> {
        self.options
            .iter()
            .position(|option| option.value == self.value)
    }

    pub fn event_context(self) -> UiShadcnEventContext {
        let values = self
            .options
            .iter()
            .map(|option| option.value)
            .collect::<Vec<_>>();
        UiShadcnEventContext::new().select_options(self.base_option_id, &values)
    }

    pub fn event_from_action(self, action: &UiAction) -> UiShadcnEvent {
        shadcn_event_from_action_with_context(action, Some(&self.event_context()))
    }

    pub fn to_node_with_state(self, state: Option<&UiRuntimeState>) -> UiNode {
        let trigger = self.to_node();
        if !self.open_with_state(state) || self.options.is_empty() {
            return trigger;
        }
        let mut menu = column("gap-1 bg-popover border rounded-lg p-1");
        for (index, option) in self.options.iter().enumerate() {
            menu = menu.child(
                menu_item_node(option.label, self.base_option_id + index as u32)
                    .selected(option.value == self.value)
                    .disabled(option.disabled),
            );
        }
        column("gap-2").child(trigger).child(menu)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnItemProps<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    pub id: u32,
    pub accent: Color4,
}

impl<'a> UiShadcnItemProps<'a> {
    pub const fn new(title: &'a str, detail: &'a str, id: u32) -> Self {
        Self {
            title,
            detail,
            id,
            accent: palette::ACCENT,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_item(self.title, self.detail, self.id, self.accent)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnKbdProps<'a> {
    pub keys: &'a [&'a str],
    pub label: &'a str,
}

impl<'a> UiShadcnKbdProps<'a> {
    pub const fn new(keys: &'a [&'a str], label: &'a str) -> Self {
        Self { keys, label }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_kbd(self.keys, self.label)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnLabelProps<'a> {
    pub value: &'a str,
}

impl<'a> UiShadcnLabelProps<'a> {
    pub const fn new(value: &'a str) -> Self {
        Self { value }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_label(self.value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnMenubarProps<'a> {
    pub items: &'a [&'a str],
    pub selected: usize,
    pub base_id: u32,
}

impl<'a> UiShadcnMenubarProps<'a> {
    pub const fn new(items: &'a [&'a str], base_id: u32) -> Self {
        Self {
            items,
            selected: 0,
            base_id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_menubar(self.items, self.selected, self.base_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnNativeSelectProps<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub id: u32,
    pub disabled: bool,
}

impl<'a> UiShadcnNativeSelectProps<'a> {
    pub const fn new(label: &'a str, value: &'a str, id: u32) -> Self {
        Self {
            label,
            value,
            id,
            disabled: false,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_native_select(self.label, self.value, self.id).disabled(self.disabled)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnTabsProps<'a> {
    pub labels: &'a [&'a str],
    pub selected: usize,
    pub base_id: u32,
}

impl<'a> UiShadcnTabsProps<'a> {
    pub const fn new(labels: &'a [&'a str], base_id: u32) -> Self {
        Self {
            labels,
            selected: 0,
            base_id,
        }
    }

    pub fn selected_with_state(self, state: Option<&UiRuntimeState>) -> usize {
        state
            .map(|state| state.selected_tab_index(self.base_id, self.labels.len(), self.selected))
            .unwrap_or(self.selected)
    }

    pub fn to_node(self) -> UiNode {
        shadcn_tabs(self.labels, self.selected, self.base_id)
    }

    pub fn event_context(self) -> UiShadcnEventContext {
        UiShadcnEventContext::new().tab_group(self.base_id, self.labels.len())
    }

    pub fn event_from_action(self, action: &UiAction) -> UiShadcnEvent {
        shadcn_event_from_action_with_context(action, Some(&self.event_context()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnDialogProps<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub icon: UiIcon,
    pub id: u32,
    pub open: bool,
}

impl<'a> UiShadcnDialogProps<'a> {
    pub const fn new(title: &'a str, body: &'a str, icon: UiIcon, id: u32) -> Self {
        Self {
            title,
            body,
            icon,
            id,
            open: false,
        }
    }

    pub fn open_with_state(self, state: Option<&UiRuntimeState>) -> bool {
        state
            .map(|state| state.open_value(self.id, self.open))
            .unwrap_or(self.open)
    }

    pub fn to_node(self) -> Option<UiNode> {
        self.open
            .then(|| shadcn_dialog(self.title, self.body, self.icon))
    }

    pub fn to_node_with_state(self, state: Option<&UiRuntimeState>) -> Option<UiNode> {
        self.open_with_state(state)
            .then(|| shadcn_dialog(self.title, self.body, self.icon))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnDropdownMenuProps<'a> {
    pub items: &'a [(&'a str, &'a str, bool)],
    pub id: u32,
    pub base_item_id: u32,
    pub open: bool,
}

impl<'a> UiShadcnDropdownMenuProps<'a> {
    pub const fn new(items: &'a [(&'a str, &'a str, bool)], id: u32, base_item_id: u32) -> Self {
        Self {
            items,
            id,
            base_item_id,
            open: false,
        }
    }

    pub fn open_with_state(self, state: Option<&UiRuntimeState>) -> bool {
        state
            .map(|state| state.open_value(self.id, self.open))
            .unwrap_or(self.open)
    }

    pub fn to_node(self) -> Option<UiNode> {
        self.open
            .then(|| shadcn_dropdown_menu(self.items, self.base_item_id))
    }

    pub fn to_node_with_state(self, state: Option<&UiRuntimeState>) -> Option<UiNode> {
        self.open_with_state(state)
            .then(|| shadcn_dropdown_menu(self.items, self.base_item_id))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnNavigationMenuProps<'a> {
    pub tabs: &'a [&'a str],
    pub selected: usize,
    pub title: &'a str,
    pub detail: &'a str,
    pub row_title: &'a str,
    pub row_detail: &'a str,
    pub base_id: u32,
}

impl<'a> UiShadcnNavigationMenuProps<'a> {
    pub const fn new(
        tabs: &'a [&'a str],
        title: &'a str,
        detail: &'a str,
        row_title: &'a str,
        row_detail: &'a str,
        base_id: u32,
    ) -> Self {
        Self {
            tabs,
            selected: 0,
            title,
            detail,
            row_title,
            row_detail,
            base_id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_navigation_menu(
            self.tabs,
            self.selected,
            self.title,
            self.detail,
            self.row_title,
            self.row_detail,
            self.base_id,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnCardProps<'a> {
    pub title: &'a str,
    pub detail: &'a str,
}

impl<'a> UiShadcnCardProps<'a> {
    pub const fn new(title: &'a str, detail: &'a str) -> Self {
        Self { title, detail }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_card(self.title, self.detail)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnBreadcrumbProps<'a> {
    pub labels: &'a [&'a str],
    pub selected: usize,
    pub base_id: u32,
}

impl<'a> UiShadcnBreadcrumbProps<'a> {
    pub const fn new(labels: &'a [&'a str], base_id: u32) -> Self {
        Self {
            labels,
            selected: 0,
            base_id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_breadcrumb(self.labels, self.selected, self.base_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnPaginationProps<'a> {
    pub pages: &'a [&'a str],
    pub selected: usize,
    pub base_id: u32,
}

impl<'a> UiShadcnPaginationProps<'a> {
    pub const fn new(pages: &'a [&'a str], base_id: u32) -> Self {
        Self {
            pages,
            selected: 0,
            base_id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_pagination(self.pages, self.selected, self.base_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnPopoverProps<'a> {
    pub button_label: &'a str,
    pub title: &'a str,
    pub detail: &'a str,
    pub field_label: &'a str,
    pub field_value: &'a str,
    pub base_id: u32,
}

impl<'a> UiShadcnPopoverProps<'a> {
    pub const fn new(
        button_label: &'a str,
        title: &'a str,
        detail: &'a str,
        field_label: &'a str,
        field_value: &'a str,
        base_id: u32,
    ) -> Self {
        Self {
            button_label,
            title,
            detail,
            field_label,
            field_value,
            base_id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_popover(
            self.button_label,
            self.title,
            self.detail,
            self.field_label,
            self.field_value,
            self.base_id,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnToastProps<'a> {
    pub message: &'a str,
    pub icon: UiIcon,
    pub accent: Color4,
}

impl<'a> UiShadcnToastProps<'a> {
    pub const fn new(message: &'a str) -> Self {
        Self {
            message,
            icon: UiIcon::Bell,
            accent: palette::ACCENT,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_toast(self.message, self.icon, self.accent)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnProgressProps {
    pub value: f32,
}

impl UiShadcnProgressProps {
    pub const fn new(value: f32) -> Self {
        Self { value }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_progress(self.value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnSeparatorProps;

impl UiShadcnSeparatorProps {
    pub const fn new() -> Self {
        Self
    }

    pub fn to_node(self) -> UiNode {
        shadcn_separator()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnSkeletonProps;

impl UiShadcnSkeletonProps {
    pub const fn new() -> Self {
        Self
    }

    pub fn to_node(self) -> UiNode {
        shadcn_skeleton()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnResizableProps<'a> {
    pub labels: &'a [&'a str],
}

impl<'a> UiShadcnResizableProps<'a> {
    pub const fn new(labels: &'a [&'a str]) -> Self {
        Self { labels }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_resizable(self.labels)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnScrollAreaProps<'a> {
    pub rows: &'a [(&'a str, &'a str)],
    pub base_id: u32,
}

impl<'a> UiShadcnScrollAreaProps<'a> {
    pub const fn new(rows: &'a [(&'a str, &'a str)], base_id: u32) -> Self {
        Self { rows, base_id }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_scroll_area(self.rows, self.base_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnSheetProps<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    pub field_label: &'a str,
    pub field_value: &'a str,
    pub button_label: &'a str,
    pub base_id: u32,
}

impl<'a> UiShadcnSheetProps<'a> {
    pub const fn new(
        title: &'a str,
        detail: &'a str,
        field_label: &'a str,
        field_value: &'a str,
        button_label: &'a str,
        base_id: u32,
    ) -> Self {
        Self {
            title,
            detail,
            field_label,
            field_value,
            button_label,
            base_id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_sheet(
            self.title,
            self.detail,
            self.field_label,
            self.field_value,
            self.button_label,
            self.base_id,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnSidebarProps<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    pub items: &'a [&'a str],
    pub selected: usize,
    pub main_title: &'a str,
    pub main_detail: &'a str,
    pub base_id: u32,
}

impl<'a> UiShadcnSidebarProps<'a> {
    pub const fn new(
        title: &'a str,
        detail: &'a str,
        items: &'a [&'a str],
        main_title: &'a str,
        main_detail: &'a str,
        base_id: u32,
    ) -> Self {
        Self {
            title,
            detail,
            items,
            selected: 0,
            main_title,
            main_detail,
            base_id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_sidebar(
            self.title,
            self.detail,
            self.items,
            self.selected,
            self.main_title,
            self.main_detail,
            self.base_id,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnSonnerProps<'a> {
    pub messages: &'a [(&'a str, UiIcon, Color4)],
}

impl<'a> UiShadcnSonnerProps<'a> {
    pub const fn new(messages: &'a [(&'a str, UiIcon, Color4)]) -> Self {
        Self { messages }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_sonner(self.messages)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnSwitchProps {
    pub id: u32,
    pub checked: bool,
    pub disabled: bool,
}

impl UiShadcnSwitchProps {
    pub const fn new(id: u32) -> Self {
        Self {
            id,
            checked: false,
            disabled: false,
        }
    }

    pub fn checked_with_state(self, state: Option<&UiRuntimeState>) -> bool {
        state
            .map(|state| state.toggle_value(self.id, self.checked))
            .unwrap_or(self.checked)
    }

    pub fn to_node(self) -> UiNode {
        shadcn_switch(self.checked, self.id).disabled(self.disabled)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnTextareaProps<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub id: u32,
    pub disabled: bool,
    pub invalid: bool,
}

impl<'a> UiShadcnTextareaProps<'a> {
    pub const fn new(label: &'a str, value: &'a str, id: u32) -> Self {
        Self {
            label,
            value,
            id,
            disabled: false,
            invalid: false,
        }
    }

    pub fn value_with_state<'b>(self, state: Option<&'b UiRuntimeState>) -> &'b str
    where
        'a: 'b,
    {
        state
            .map(|state| state.text_value(self.id, self.value))
            .unwrap_or(self.value)
    }

    pub fn to_node(self) -> UiNode {
        shadcn_textarea(self.label, self.value)
            .hit_id(self.id)
            .disabled(self.disabled)
            .accent(if self.invalid {
                palette::DANGER
            } else {
                palette::ACCENT
            })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnSliderProps<'a> {
    pub label: &'a str,
    pub value: f32,
    pub id: u32,
    pub disabled: bool,
}

impl<'a> UiShadcnSliderProps<'a> {
    pub const fn new(label: &'a str, value: f32, id: u32) -> Self {
        Self {
            label,
            value,
            id,
            disabled: false,
        }
    }

    pub fn value_with_state(self, state: Option<&UiRuntimeState>) -> f32 {
        state
            .map(|state| state.slider_value(self.id, self.value))
            .unwrap_or(self.value)
    }

    pub fn to_node(self) -> UiNode {
        shadcn_slider(self.label, self.value, self.id).disabled(self.disabled)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnToggleProps {
    pub id: u32,
    pub pressed: bool,
    pub disabled: bool,
}

impl UiShadcnToggleProps {
    pub const fn new(id: u32) -> Self {
        Self {
            id,
            pressed: false,
            disabled: false,
        }
    }

    pub fn pressed_with_state(self, state: Option<&UiRuntimeState>) -> bool {
        state
            .map(|state| state.toggle_value(self.id, self.pressed))
            .unwrap_or(self.pressed)
    }

    pub fn to_node(self) -> UiNode {
        shadcn_toggle(self.pressed, self.id).disabled(self.disabled)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnToggleGroupProps<'a> {
    pub labels: &'a [&'a str],
    pub selected: usize,
    pub base_id: u32,
}

impl<'a> UiShadcnToggleGroupProps<'a> {
    pub const fn new(labels: &'a [&'a str], base_id: u32) -> Self {
        Self {
            labels,
            selected: 0,
            base_id,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_toggle_group(self.labels, self.selected, self.base_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnTooltipProps<'a> {
    pub text: &'a str,
}

impl<'a> UiShadcnTooltipProps<'a> {
    pub const fn new(text: &'a str) -> Self {
        Self { text }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_tooltip(self.text)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnTableProps<'a> {
    pub headers: &'a [&'a str],
    pub rows: &'a [&'a [&'a str]],
    pub id_base: u32,
}

impl<'a> UiShadcnTableProps<'a> {
    pub const fn new(headers: &'a [&'a str], rows: &'a [&'a [&'a str]], id_base: u32) -> Self {
        Self {
            headers,
            rows,
            id_base,
        }
    }

    pub fn to_node(self) -> UiNode {
        shadcn_table(self.headers, self.rows, self.id_base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_props_render_expected_native_nodes() {
        assert!(matches!(
            UiShadcnButtonProps::new("Save", 1).to_node().kind,
            UiNodeKind::Button { .. }
        ));
        assert!(matches!(
            UiShadcnButtonGroupProps::new(&["A", "B"], 10)
                .to_node()
                .kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            UiShadcnAccordionProps::new(&[("One", "Body")], 20)
                .to_node()
                .kind,
            UiNodeKind::Card
        ));
        assert!(matches!(
            UiShadcnBadgeProps::new("Beta").to_node().kind,
            UiNodeKind::Badge { .. }
        ));
        assert!(matches!(
            UiShadcnAlertProps::new("Heads up", "Body").to_node().kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            UiShadcnAlertDialogProps::new("Delete", "Confirm")
                .to_node()
                .kind,
            UiNodeKind::Dialog { .. }
        ));
        assert!(matches!(
            UiShadcnAspectRatioProps::new("16:9").to_node().kind,
            UiNodeKind::Card
        ));
        assert!(matches!(
            UiShadcnAvatarProps::new("ER").to_node().kind,
            UiNodeKind::Avatar { .. }
        ));
        assert!(matches!(
            UiShadcnCalendarProps::new("May", &["1", "2"], 30)
                .to_node()
                .kind,
            UiNodeKind::Card
        ));
        assert!(matches!(
            UiShadcnCarouselProps::new(&["One"], 40).to_node().kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            UiShadcnChartProps::new("Sales", &["A"], &[0.5])
                .to_node()
                .kind,
            UiNodeKind::BarChart { .. }
        ));
        assert!(matches!(
            UiShadcnCollapsibleProps::new("Open", &[("Row", "Detail")], 50)
                .to_node()
                .kind,
            UiNodeKind::Card
        ));
        assert!(matches!(
            UiShadcnCommandProps::new("Search...", 60).to_node().kind,
            UiNodeKind::CommandPalette { .. }
        ));
        assert!(matches!(
            UiShadcnComboboxProps::new("Framework", "Next.js", "Search...", &["Next.js"], 61)
                .to_node()
                .kind,
            UiNodeKind::Column
        ));
        assert!(matches!(
            UiShadcnInputProps::new("Email", "a@b.test", 2)
                .to_node()
                .kind,
            UiNodeKind::Field { id: Some(2), .. }
        ));
        assert!(matches!(
            UiShadcnContextMenuProps::new("Menu", "Detail", &[("Copy", "Cmd+C", false)], 65)
                .to_node()
                .kind,
            UiNodeKind::Card
        ));
        assert!(matches!(
            UiShadcnDatePickerProps::new("Pick", "May", &["1", "2"], 70)
                .to_node()
                .kind,
            UiNodeKind::Column
        ));
        assert!(matches!(
            UiShadcnDataTableProps::new(&["A"], &[&["B"]], 71)
                .to_node()
                .kind,
            UiNodeKind::Table { .. }
        ));
        assert!(matches!(
            UiShadcnDirectionProps::new("Left", "Right").to_node().kind,
            UiNodeKind::Column
        ));
        assert!(matches!(
            UiShadcnDrawerProps::new("Drawer", "Detail", "Amount", 0.5, 72)
                .to_node()
                .kind,
            UiNodeKind::Card
        ));
        assert!(matches!(
            UiShadcnEmptyProps::new("Empty", "Nothing").to_node().kind,
            UiNodeKind::EmptyState { .. }
        ));
        assert!(matches!(
            UiShadcnHoverCardProps::new("ER", "UI", "Body")
                .to_node()
                .kind,
            UiNodeKind::Column
        ));
        assert!(matches!(
            UiShadcnInputGroupProps::new("URL", "https://edgerun.dev", "Copy", 75)
                .to_node()
                .kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            UiShadcnInputOtpProps::new(&["1", "2", "-", ""])
                .to_node()
                .kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            UiShadcnCheckboxProps::new("Accept", 5).to_node().kind,
            UiNodeKind::Checkbox { .. }
        ));
        assert!(matches!(
            UiShadcnTabsProps::new(&["A", "B"], 3).to_node().kind,
            UiNodeKind::Tabs { .. }
        ));
        assert!(matches!(
            UiShadcnSwitchProps::new(6).to_node().kind,
            UiNodeKind::Toggle { .. }
        ));
        assert!(matches!(
            UiShadcnItemProps::new("Item", "Detail", 80).to_node().kind,
            UiNodeKind::ListRow { .. }
        ));
        assert!(matches!(
            UiShadcnKbdProps::new(&["Cmd", "K"], "Command")
                .to_node()
                .kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            UiShadcnLabelProps::new("Label").to_node().kind,
            UiNodeKind::Text(_)
        ));
        assert!(matches!(
            UiShadcnMenubarProps::new(&["File", "Edit"], 90)
                .to_node()
                .kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            UiShadcnNativeSelectProps::new("Theme", "Sera", 92)
                .to_node()
                .kind,
            UiNodeKind::Select { .. }
        ));
        assert!(matches!(
            UiShadcnNavigationMenuProps::new(
                &["Docs"],
                "Components",
                "Detail",
                "Button",
                "Control",
                95
            )
            .to_node()
            .kind,
            UiNodeKind::Column
        ));
        assert!(matches!(
            UiShadcnTextareaProps::new("Message", "Hi", 7)
                .to_node()
                .kind,
            UiNodeKind::TextArea { id: Some(7), .. }
        ));
        assert!(matches!(
            UiShadcnSliderProps::new("Volume", 0.5, 8).to_node().kind,
            UiNodeKind::Slider { .. }
        ));
        assert!(matches!(
            UiShadcnProgressProps::new(0.5).to_node().kind,
            UiNodeKind::ProgressBar { .. }
        ));
        assert!(matches!(
            UiShadcnSeparatorProps::new().to_node().kind,
            UiNodeKind::Divider
        ));
        assert!(matches!(
            UiShadcnSkeletonProps::new().to_node().kind,
            UiNodeKind::Skeleton
        ));
        assert!(matches!(
            UiShadcnPopoverProps::new("Open", "Profile", "Detail", "Name", "EdgeRun", 155)
                .to_node()
                .kind,
            UiNodeKind::Column
        ));
        assert!(matches!(
            UiShadcnResizableProps::new(&["One", "Two"]).to_node().kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            UiShadcnScrollAreaProps::new(&[("A", "B")], 160)
                .to_node()
                .kind,
            UiNodeKind::ScrollArea { .. }
        ));
        assert!(matches!(
            UiShadcnSheetProps::new("Sheet", "Detail", "Name", "EdgeRun", "Save", 170)
                .to_node()
                .kind,
            UiNodeKind::Card
        ));
        assert!(matches!(
            UiShadcnSidebarProps::new("Menu", "Detail", &["Home"], "Main", "Body", 180)
                .to_node()
                .kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            UiShadcnSonnerProps::new(&[("Saved", UiIcon::Check, palette::ACCENT)])
                .to_node()
                .kind,
            UiNodeKind::Column
        ));
        assert!(matches!(
            UiShadcnBreadcrumbProps::new(&["Home", "Docs"], 140)
                .to_node()
                .kind,
            UiNodeKind::Breadcrumb { .. }
        ));
        assert!(matches!(
            UiShadcnPaginationProps::new(&["1", "2"], 150)
                .to_node()
                .kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            UiShadcnToggleProps::new(190).to_node().kind,
            UiNodeKind::Toggle { .. }
        ));
        assert!(matches!(
            UiShadcnToggleGroupProps::new(&["Left", "Right"], 200)
                .to_node()
                .kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            UiShadcnTooltipProps::new("Help").to_node().kind,
            UiNodeKind::Tooltip { .. }
        ));
        assert!(matches!(
            UiShadcnTableProps::new(&["A"], &[&["B"]], 4).to_node().kind,
            UiNodeKind::Table { .. }
        ));
    }

    #[test]
    fn typed_props_surface_catalog_covers_exact_shadcn_ports() {
        assert_eq!(shadcn_props_surface_count(), shadcn_exact_demo_count());

        for demo in SHADCN_DEMO_COMPONENTS {
            if !demo.is_exact_port() {
                continue;
            }
            let surface = shadcn_props_surface_for_slug(demo.slug)
                .unwrap_or_else(|| panic!("missing typed props surface for {}", demo.slug));
            assert_eq!(surface.slug, demo.slug);
            assert_eq!(surface.source_component, demo.source_component);
            assert!(surface.props_type.starts_with("UiShadcn"));
            assert!(surface.exact_builder.starts_with("shadcn_"));
        }

        assert_eq!(
            shadcn_props_surface_for_source_component("Select").map(|surface| surface.slug),
            Some("select")
        );
        assert!(
            shadcn_props_surface_for_slug("select")
                .map(|surface| surface.stateful && surface.event_adapter)
                .unwrap_or(false)
        );

        let summary = shadcn_props_surface_summary();
        assert_eq!(summary.total, SHADCN_PROPS_SURFACES.len());
        assert_eq!(summary.stateful, shadcn_stateful_props_surfaces().count());
        assert_eq!(
            summary.event_adapted,
            shadcn_event_adapted_props_surfaces().count()
        );
        assert_eq!(summary.static_only, shadcn_static_props_surfaces().count());
        assert_eq!(
            summary.total,
            summary.static_only
                + SHADCN_PROPS_SURFACES
                    .iter()
                    .filter(|surface| surface.stateful || surface.event_adapter)
                    .count()
        );
        assert!(shadcn_stateful_props_surfaces().any(|surface| surface.slug == "tabs"));
        assert!(shadcn_event_adapted_props_surfaces().any(|surface| surface.slug == "combobox"));
        assert!(shadcn_static_props_surfaces().any(|surface| surface.slug == "badge"));

        assert_eq!(shadcn_resolved_props_surfaces().count(), summary.total);
        let resolved_select = resolve_shadcn_props_surface("@/components/ui/select").unwrap();
        assert_eq!(resolved_select.surface.props_type, "UiShadcnSelectProps");
        assert_eq!(resolved_select.demo.route, "/docs/components/select");
        assert_eq!(
            resolved_select.resolve_kind,
            UiShadcnResolveKind::ModulePath
        );
        assert!(
            resolved_select
                .parity
                .unwrap()
                .supports_interaction("select")
        );

        let resolved_card_slot = resolve_shadcn_props_surface("CardHeader").unwrap();
        assert_eq!(resolved_card_slot.surface.slug, "card");
        assert_eq!(resolved_card_slot.resolve_kind, UiShadcnResolveKind::Slot);

        let button_surface = shadcn_props_surface_for_slug("button").unwrap();
        assert_eq!(button_surface.docs_route(), Some("/docs/components/button"));
        assert_eq!(
            button_surface.category(),
            Some(UiShadcnDemoCategory::Foundation)
        );
        assert!(button_surface.slots().contains(&"button"));
        assert!(button_surface.states().contains(&"loading"));
    }

    #[test]
    fn typed_form_controls_read_runtime_state_and_map_values() {
        const RADIOS: &[UiShadcnRadioOption<'_>] = &[
            UiShadcnRadioOption::new("Default", "default"),
            UiShadcnRadioOption::new("Compact", "compact"),
        ];
        let checkbox = UiShadcnCheckboxProps::new("Accept", 90);
        let radio_group = UiShadcnRadioGroupProps::new(RADIOS, "default", 100);
        let slider = UiShadcnSliderProps::new("Volume", 0.25, 120);
        let textarea = UiShadcnTextareaProps::new("Message", "initial", 130);
        let mut state = UiRuntimeState::default();

        assert!(!checkbox.checked_with_state(Some(&state)));
        state.handle_event(
            &scene_with_hit(GpuHit::new(HitKind::Checkbox, 90, 0.0, 0.0, 20.0, 20.0)),
            UiEvent::PointerDown { x: 1.0, y: 1.0 },
        );
        state.handle_event(
            &scene_with_hit(GpuHit::new(HitKind::Checkbox, 90, 0.0, 0.0, 20.0, 20.0)),
            UiEvent::PointerUp { x: 1.0, y: 1.0 },
        );
        assert!(checkbox.checked_with_state(Some(&state)));
        assert_eq!(
            checkbox.event_from_action(&UiAction::Toggled { id: 90, on: true }),
            UiShadcnEvent::OnCheckedChange {
                id: 90,
                checked: true,
            }
        );

        assert_eq!(radio_group.selected_index(), Some(0));
        assert!(matches!(radio_group.to_node().kind, UiNodeKind::Column));
        assert_eq!(
            radio_group.event_from_action(&UiAction::Toggled { id: 101, on: true }),
            UiShadcnEvent::OnValueChange {
                id: 101,
                value: UiShadcnEventValue::Text("compact".to_string()),
            }
        );

        assert_eq!(slider.value_with_state(Some(&state)), 0.25);
        assert_eq!(textarea.value_with_state(Some(&state)), "initial");

        let drawer = UiShadcnDrawerProps::new("Drawer", "Detail", "Amount", 0.25, 140);
        assert_eq!(drawer.value_with_state(Some(&state)), 0.25);
    }

    #[test]
    fn typed_props_read_executable_runtime_state() {
        let mut state = UiRuntimeState::default();
        state.set_open(9, true);
        assert!(UiShadcnSelectProps::new("Framework", "Next.js", 9).open_with_state(Some(&state)));
        assert!(
            UiShadcnDialogProps::new("Edit", "Body", UiIcon::Settings, 9)
                .to_node_with_state(Some(&state))
                .is_some()
        );
        assert!(
            UiShadcnDropdownMenuProps::new(&[("Profile", "", false)], 9, 40)
                .to_node_with_state(Some(&state))
                .is_some()
        );
    }

    #[test]
    fn typed_props_execute_runtime_selection_from_rendered_hits() {
        let mut scene = GpuScene::new(palette::BG);
        let tabs = UiShadcnTabsProps::new(&["Account", "Billing"], 20).to_node();
        {
            let mut ui = UiPainter::new(&mut scene);
            tabs.render(
                &mut ui,
                UiRect {
                    x: 0.0,
                    y: 0.0,
                    w: 200.0,
                    h: 40.0,
                },
            );
        }

        let mut state = UiRuntimeState::default();
        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerDown { x: 150.0, y: 20.0 }),
            UiAction::Activated(GpuHit::new(HitKind::Tab, 21, 100.0, 0.0, 100.0, 34.0))
        );
        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerUp { x: 150.0, y: 20.0 }),
            UiAction::TabSelected { id: 21 }
        );
        assert_eq!(state.selected_tab_index(20, 2, 0), 1);
        assert_eq!(
            UiShadcnTabsProps::new(&["Account", "Billing"], 20)
                .event_from_action(&UiAction::TabSelected { id: 21 }),
            UiShadcnEvent::OnValueChange {
                id: 21,
                value: UiShadcnEventValue::TabIndex(1),
            }
        );

        let mut scene = GpuScene::new(palette::BG);
        let select = UiShadcnSelectProps::new("Framework", "Next.js", 31).to_node();
        {
            let mut ui = UiPainter::new(&mut scene);
            select.render(
                &mut ui,
                UiRect {
                    x: 0.0,
                    y: 0.0,
                    w: 220.0,
                    h: 58.0,
                },
            );
        }

        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerDown { x: 20.0, y: 20.0 }),
            UiAction::Activated(GpuHit::new(HitKind::Select, 31, 0.0, 0.0, 220.0, 58.0))
        );
        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerUp { x: 20.0, y: 20.0 }),
            UiAction::OpenChanged { id: 31, open: true }
        );
        assert!(UiShadcnSelectProps::new("Framework", "Next.js", 31).open_with_state(Some(&state)));
    }

    #[test]
    fn typed_select_renders_options_and_maps_selection_events_to_values() {
        const OPTIONS: &[UiShadcnSelectOption<'_>] = &[
            UiShadcnSelectOption::new("Next.js", "next"),
            UiShadcnSelectOption::new("SvelteKit", "svelte"),
        ];
        let props = UiShadcnSelectProps::new("Framework", "next", 70).with_options(OPTIONS, 80);
        let mut state = UiRuntimeState::default();

        assert_eq!(props.selected_index(), Some(0));
        assert!(matches!(
            props.to_node_with_state(Some(&state)).kind,
            UiNodeKind::Select { .. }
        ));

        state.set_open(70, true);
        let expanded = props.to_node_with_state(Some(&state));
        assert!(matches!(expanded.kind, UiNodeKind::Column));
        assert_eq!(expanded.children.len(), 2);

        assert_eq!(
            props.event_from_action(&UiAction::Activated(GpuHit::new(
                HitKind::MenuItem,
                81,
                0.0,
                0.0,
                10.0,
                10.0
            ))),
            UiShadcnEvent::OnValueChange {
                id: 81,
                value: UiShadcnEventValue::Text("svelte".to_string()),
            }
        );
    }

    #[test]
    fn typed_combobox_maps_option_events_to_values() {
        let props = UiShadcnComboboxProps::new(
            "Framework",
            "Next.js",
            "Search...",
            &["Next", "Svelte"],
            90,
        );

        assert_eq!(props.selected_value(), Some("Next"));
        assert_eq!(
            props.event_from_action(&UiAction::Activated(GpuHit::new(
                HitKind::MenuItem,
                93,
                0.0,
                0.0,
                10.0,
                10.0
            ))),
            UiShadcnEvent::OnValueChange {
                id: 93,
                value: UiShadcnEventValue::Text("Svelte".to_string()),
            }
        );
    }

    fn scene_with_hit(hit: GpuHit) -> GpuScene {
        let mut scene = GpuScene::new(palette::BG);
        scene.push_hit(hit);
        scene
    }
}
