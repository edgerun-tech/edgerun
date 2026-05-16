//! Renderable preview blocks built from the extracted component inventory.

use super::style_family::colors_for_style_family;
use super::{
    ButtonStyle, UiIcon, UiNode, UiRect, UiStyleFamily, app_launcher_item, attachment_preview,
    badge, breadcrumb, button, capability_grant_row, card, checkbox, command_palette, contact_card,
    control_row_node, dialog, empty_state, field_node, grid, header, icon_button, identity_card,
    list_row_node, menu_item_node, metric, package_card, progress_bar_node, progress_ring,
    proof_event_row, radio, receipt_row, route_path, section, select_node, skeleton, slider_node,
    tab_labels, table_labels, text, text_area_node, thread_row, toast, tooltip, transaction_node,
    tree_item,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiExtractedBlockId {
    StyleAuthority,
    Finance,
    NetworkApp,
    StyleFamilyPicker,
    InputGroup,
    OverlaySelection,
    DataFeedback,
    TrustActivity,
    Directory,
    ComponentStudio,
}

pub const EXTRACTED_BLOCK_IDS: [UiExtractedBlockId; 10] = [
    UiExtractedBlockId::StyleAuthority,
    UiExtractedBlockId::Finance,
    UiExtractedBlockId::NetworkApp,
    UiExtractedBlockId::StyleFamilyPicker,
    UiExtractedBlockId::InputGroup,
    UiExtractedBlockId::OverlaySelection,
    UiExtractedBlockId::DataFeedback,
    UiExtractedBlockId::TrustActivity,
    UiExtractedBlockId::Directory,
    UiExtractedBlockId::ComponentStudio,
];

impl UiExtractedBlockId {
    pub const fn label(self) -> &'static str {
        match self {
            Self::StyleAuthority => "Style Authority",
            Self::Finance => "Finance",
            Self::NetworkApp => "Network App",
            Self::StyleFamilyPicker => "Style Family Picker",
            Self::InputGroup => "Input Group",
            Self::OverlaySelection => "Overlay Selection",
            Self::DataFeedback => "Data Feedback",
            Self::TrustActivity => "Trust Activity",
            Self::Directory => "Directory",
            Self::ComponentStudio => "Component Studio",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiExtractedBlockKind {
    Studio,
    Dashboard,
    DomainSurface,
    Navigation,
    Form,
    Overlay,
    Feedback,
}

pub const EXTRACTED_BLOCK_KINDS: [UiExtractedBlockKind; 7] = [
    UiExtractedBlockKind::Studio,
    UiExtractedBlockKind::Dashboard,
    UiExtractedBlockKind::DomainSurface,
    UiExtractedBlockKind::Navigation,
    UiExtractedBlockKind::Form,
    UiExtractedBlockKind::Overlay,
    UiExtractedBlockKind::Feedback,
];

impl UiExtractedBlockKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Studio => "Studio",
            Self::Dashboard => "Dashboard",
            Self::DomainSurface => "Domain Surface",
            Self::Navigation => "Navigation",
            Self::Form => "Form",
            Self::Overlay => "Overlay",
            Self::Feedback => "Feedback",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedBlockSpec {
    pub id: UiExtractedBlockId,
    pub kind: UiExtractedBlockKind,
    pub name: &'static str,
    pub builder: &'static str,
    pub role: &'static str,
    pub preview_width: u16,
    pub preview_height: u16,
}

impl UiExtractedBlockSpec {
    pub const fn preview_rect(self) -> UiRect {
        UiRect::new(
            0.0,
            0.0,
            self.preview_width as f32,
            self.preview_height as f32,
        )
    }

    pub const fn preview_aspect_ratio(self) -> f32 {
        self.preview_width as f32 / self.preview_height as f32
    }
}

pub const EXTRACTED_BLOCKS: &[UiExtractedBlockSpec] = &[
    block_spec(
        UiExtractedBlockId::StyleAuthority,
        UiExtractedBlockKind::Navigation,
        "style_authority_panel",
        "user-owned style controls and author vision boundary",
        280,
        720,
    ),
    block_spec(
        UiExtractedBlockId::Finance,
        UiExtractedBlockKind::Dashboard,
        "finance_block",
        "dashboard cards, forms, chart, and transaction rows",
        960,
        880,
    ),
    block_spec(
        UiExtractedBlockId::NetworkApp,
        UiExtractedBlockKind::DomainSurface,
        "network_app_block",
        "run/cache package policy summary",
        420,
        360,
    ),
    block_spec(
        UiExtractedBlockId::StyleFamilyPicker,
        UiExtractedBlockKind::Navigation,
        "style_family_picker_block",
        "seven-family radio menu from the captures",
        260,
        340,
    ),
    block_spec(
        UiExtractedBlockId::InputGroup,
        UiExtractedBlockKind::Form,
        "input_group_block",
        "command/search input and prompt composer",
        760,
        420,
    ),
    block_spec(
        UiExtractedBlockId::OverlaySelection,
        UiExtractedBlockKind::Overlay,
        "overlay_selection_block",
        "dropdown, radio selection, tooltip, and dialog patterns",
        760,
        500,
    ),
    block_spec(
        UiExtractedBlockId::DataFeedback,
        UiExtractedBlockKind::Feedback,
        "data_feedback_block",
        "breadcrumb, table, empty, toast, and feedback text",
        760,
        430,
    ),
    block_spec(
        UiExtractedBlockId::TrustActivity,
        UiExtractedBlockKind::DomainSurface,
        "trust_activity_block",
        "proof, receipt, capability, and progress rows",
        760,
        460,
    ),
    block_spec(
        UiExtractedBlockId::Directory,
        UiExtractedBlockKind::DomainSurface,
        "directory_block",
        "launcher, contact, thread, tree, attachment, and skeleton rows",
        760,
        460,
    ),
    block_spec(
        UiExtractedBlockId::ComponentStudio,
        UiExtractedBlockKind::Studio,
        "component_studio",
        "full extracted component gallery surface",
        960,
        4000,
    ),
];

const fn block_spec(
    id: UiExtractedBlockId,
    kind: UiExtractedBlockKind,
    builder: &'static str,
    role: &'static str,
    preview_width: u16,
    preview_height: u16,
) -> UiExtractedBlockSpec {
    UiExtractedBlockSpec {
        id,
        kind,
        name: id.label(),
        builder,
        role,
        preview_width,
        preview_height,
    }
}

pub fn build_extracted_block(id: UiExtractedBlockId) -> UiNode {
    match id {
        UiExtractedBlockId::StyleAuthority => style_authority_panel(),
        UiExtractedBlockId::Finance => finance_block(),
        UiExtractedBlockId::NetworkApp => network_app_block(),
        UiExtractedBlockId::StyleFamilyPicker => style_family_picker_block(UiStyleFamily::Mira),
        UiExtractedBlockId::InputGroup => input_group_block(),
        UiExtractedBlockId::OverlaySelection => overlay_selection_block(),
        UiExtractedBlockId::DataFeedback => data_feedback_block(),
        UiExtractedBlockId::TrustActivity => trust_activity_block(),
        UiExtractedBlockId::Directory => directory_block(),
        UiExtractedBlockId::ComponentStudio => component_studio(),
    }
}

pub fn style_authority_panel() -> UiNode {
    card("bg-topbar border rounded-md p-3 gap-3")
        .child(section("Style", ""))
        .child(
            menu_item_node("Mira", 761)
                .detail("user style")
                .badge_text("active")
                .selected(true),
        )
        .child(
            control_row_node("Base color")
                .detail("Neutral")
                .value_text("Neutral"),
        )
        .child(
            control_row_node("Theme")
                .detail("Neutral")
                .value_text("Neutral"),
        )
        .child(
            control_row_node("Chart color")
                .detail("Neutral")
                .value_text("Neutral"),
        )
        .child(section("Typography", ""))
        .child(control_row_node("Heading").detail("Inter").value_text("Aa"))
        .child(control_row_node("Font").detail("Inter").value_text("Aa"))
        .child(section("System", ""))
        .child(
            control_row_node("Icon library")
                .detail("Tabler")
                .value_text("Tabler"),
        )
        .child(control_row_node("Radius").detail("Default").value_text("8"))
        .child(button("Get Code", 765, ButtonStyle::Primary).class("h-9"))
}

pub fn finance_block() -> UiNode {
    let months = ["Dec", "Jan", "Feb", "Mar", "Apr", "May"];
    let values = [0.56, 0.78, 0.62, 0.92, 0.52, 0.98];

    grid("grid grid-cols-2 gap-4", 2)
        .child(
            card("bg-panel border rounded-md p-3 gap-3 h-108")
                .child(
                    super::bar_chart_labels("Contribution History", &months, &values)
                        .detail("Last 6 months of activity")
                        .class("h-48"),
                )
                .child(
                    grid("grid grid-cols-2 gap-3", 2)
                        .child(metric("Upcoming", "May 25, 2024").detail("$1,000 scheduled"))
                        .child(metric("Auto-save Plan", "Accelerated").detail("Recurring weekly")),
                )
                .child(button("View Full Report", 772, ButtonStyle::Primary).class("h-9")),
        )
        .child(
            card("bg-panel border rounded-md p-3 gap-3 h-108")
                .child(
                    header("Payout Threshold")
                        .detail("Set the minimum balance required before payout."),
                )
                .child(select_node(
                    "Preferred Currency",
                    "USD - United States Dollar",
                    773,
                ))
                .child(
                    slider_node("Minimum Payout Amount", 0.25, 774)
                        .range_labels("$50 (MIN)", "$10,000 (MAX)")
                        .class("h-20"),
                )
                .child(text_area_node(
                    "Notes",
                    "Add any notes for this payout configuration...",
                ))
                .child(button("Save Threshold", 776, ButtonStyle::Primary).class("h-9")),
        )
        .child(
            card("bg-panel border rounded-md p-3 gap-3 h-108")
                .child(header("Account Access").detail("Update credentials or re-authenticate."))
                .child(field_node("Email Address", "artist@studio.inc"))
                .child(field_node("Current Password", "**********"))
                .child(button("Update Security", 812, ButtonStyle::Primary).class("h-9"))
                .child(
                    control_row_node("Danger Zone").detail("Archive account and remove catalog"),
                ),
        )
        .child(
            card("bg-panel border rounded-md p-3 gap-3 h-108")
                .child(header("Recent Transactions").detail("Your latest account activity."))
                .child(
                    transaction_node("Blue Bottle Coffee", "-$6.50", 781)
                        .detail("Food & Drink")
                        .date("Today"),
                )
                .child(
                    transaction_node("Whole Foods Market", "-$142.30", 782)
                        .detail("Groceries")
                        .date("Yesterday"),
                )
                .child(
                    transaction_node("Stripe Payout", "+$4,200.00", 783)
                        .detail("Income")
                        .date("Oct 12")
                        .positive(true),
                ),
        )
}

pub fn network_app_block() -> UiNode {
    card("bg-panel border rounded-md p-3 gap-3")
        .child(
            header("Run Network App").detail("signed package, publisher policy, and route proof"),
        )
        .child(package_card(
            "EdgeRun Chat",
            "free-run / verify-cache",
            "b3:message-ui",
            801,
        ))
        .child(route_path(
            "Admission route",
            &["browser", "admission", "relay", "storage"],
        ))
        .child(checkbox("Verify and cache package bytes", true, 764).class("h-9"))
        .child(button("Run", 779, ButtonStyle::Primary).class("h-9"))
}

pub fn style_family_picker_block(selected: UiStyleFamily) -> UiNode {
    let families = [
        UiStyleFamily::Vega,
        UiStyleFamily::Nova,
        UiStyleFamily::Maia,
        UiStyleFamily::Lyra,
        UiStyleFamily::Mira,
        UiStyleFamily::Luma,
        UiStyleFamily::Sera,
    ];

    let mut menu = card("bg-popover border rounded-xl p-2 gap-1");
    for (index, family) in families.iter().enumerate() {
        menu = menu.child(
            radio(family.name(), *family == selected, 830 + index as u32).class("h-8 rounded-lg"),
        );
    }
    menu
}

pub fn input_group_block() -> UiNode {
    let colors = colors_for_style_family(UiStyleFamily::Mira);

    grid("grid grid-cols-2 gap-3", 2)
        .child(
            card("bg-panel border rounded-md p-3 gap-3")
                .child(header("Command Input").detail("search with result count and action"))
                .child(command_palette("Search documentation...", 840))
                .child(
                    card("bg-composer border rounded-md p-2 gap-2")
                        .child(field_node("Endpoint", "https://node.local/api"))
                        .child(badge("12 results", colors.active))
                        .child(icon_button(UiIcon::Search, 841).class("h-8")),
                ),
        )
        .child(
            card("bg-panel border rounded-md p-3 gap-3")
                .child(header("Prompt Composer").detail("attachment, mode, usage, send"))
                .child(text_area_node(
                    "Message",
                    "Ask an agent to inspect package proofs...",
                ))
                .child(
                    grid("grid grid-cols-4 gap-2", 4)
                        .child(icon_button(UiIcon::File, 842).class("h-8"))
                        .child(badge("Agent", colors.active))
                        .child(badge("1.2k", colors.row))
                        .child(button("Send", 843, ButtonStyle::Primary).class("h-8")),
                ),
        )
}

pub fn overlay_selection_block() -> UiNode {
    grid("grid grid-cols-2 gap-3", 2)
        .child(
            card("bg-panel border rounded-md p-3 gap-3")
                .child(header("Style Menu").detail("captured seven-item radio menu"))
                .child(style_family_picker_block(UiStyleFamily::Mira)),
        )
        .child(
            card("bg-panel border rounded-md p-3 gap-3")
                .child(header("Choice Group").detail("single-choice onboarding control"))
                .child(radio("Import old services into EdgeRun storage", true, 850))
                .child(radio(
                    "Use external services as permanent homes",
                    false,
                    851,
                ))
                .child(radio("Skip import for now", false, 852))
                .child(tooltip(
                    "User style wins unless author vision is explicitly selected.",
                ))
                .child(dialog(
                    "Run network app",
                    "Retrieve signed bytes, verify hashes, then run locally.",
                    UiIcon::App,
                )),
        )
}

pub fn data_feedback_block() -> UiNode {
    let colors = colors_for_style_family(UiStyleFamily::Mira);
    let rows: &[&[&str]] = &[
        &["package", "b3:message-ui", "verified"],
        &["policy", "policy:free-run", "active"],
        &["receipt", "work:storage", "pending"],
    ];

    grid("grid grid-cols-2 gap-3", 2)
        .child(
            card("bg-panel border rounded-md p-3 gap-3")
                .child(breadcrumb(
                    &["Home", "Trust Manager", "Package Proofs"],
                    2,
                    860,
                ))
                .child(table_labels(&["Object", "Hash", "State"], rows, 870)),
        )
        .child(
            card("bg-panel border rounded-md p-3 gap-3")
                .child(empty_state(
                    "Processing your request",
                    "Admission is checking policy, route budget, and replay window.",
                    UiIcon::Sparkles,
                ))
                .child(toast(
                    "Package hash verified and cached locally.",
                    UiIcon::Check,
                    colors.accent,
                ))
                .child(text("Feedback components use semantic status colors.")),
        )
}

pub fn trust_activity_block() -> UiNode {
    let colors = colors_for_style_family(UiStyleFamily::Mira);

    grid("grid grid-cols-2 gap-3", 2)
        .child(
            card("bg-panel border rounded-md p-3 gap-3")
                .child(header("Proof Dashboard").detail("real rows should project store state"))
                .child(proof_event_row(
                    "Package bytes verified",
                    "b3:message-ui",
                    "verified",
                    880,
                ))
                .child(proof_event_row(
                    "Admission policy committed",
                    "policy:personal",
                    "active",
                    881,
                ))
                .child(receipt_row("Storage retrieval", "$0.0008", "pending", 882))
                .child(receipt_row("Relay delivery", "$0.0002", "settled", 883)),
        )
        .child(
            card("bg-panel border rounded-md p-3 gap-3")
                .child(header("Capability Grants").detail("explicit scoped app access"))
                .child(capability_grant_row(
                    "EdgeRun Chat",
                    "contacts:read",
                    "granted",
                    884,
                ))
                .child(capability_grant_row(
                    "Finance Agent",
                    "receipts:read",
                    "pending",
                    885,
                ))
                .child(progress_bar_node(0.64, colors.accent))
                .child(progress_ring(0.64, colors.accent).class("h-16")),
        )
}

pub fn directory_block() -> UiNode {
    grid("grid grid-cols-2 gap-3", 2)
        .child(
            card("bg-panel border rounded-md p-3 gap-3")
                .child(header("Directory Rows").detail("launcher, contacts, and threads"))
                .child(app_launcher_item(
                    "Trust Manager",
                    "proof dashboard",
                    UiIcon::Trust,
                    890,
                ))
                .child(app_launcher_item(
                    "App Store",
                    "run and cache network apps",
                    UiIcon::App,
                    891,
                ))
                .child(contact_card("Ari Lane", "admission:family", 892))
                .child(thread_row(
                    "Publisher policy update",
                    "Policy hash changed after review",
                    true,
                    893,
                )),
        )
        .child(
            card("bg-panel border rounded-md p-3 gap-3")
                .child(header("Object Tree").detail("package files and import artifacts"))
                .child(tree_item("app.edapp", "manifest", 0, false, 894))
                .child(tree_item("app.eapp", "package graph", 0, true, 895))
                .child(tree_item("developer.esig", "signature", 1, false, 896))
                .child(attachment_preview(
                    "gmail-import.ndjson",
                    "import/sync source",
                    897,
                ))
                .child(list_row_node("Local cache", "verified package bytes", 898))
                .child(skeleton().class("h-8")),
        )
}

pub fn component_studio() -> UiNode {
    card("bg-panel border rounded-md p-4 gap-4")
        .child(command_palette(
            "Search components, blocks, charts, app surfaces",
            780,
        ))
        .child(tab_labels(
            &["Components", "Blocks", "Charts", "Directory"],
            0,
            900,
        ))
        .child(finance_block().class("h-220"))
        .child(input_group_block().class("h-104"))
        .child(overlay_selection_block().class("h-124"))
        .child(data_feedback_block().class("h-108"))
        .child(trust_activity_block().class("h-116"))
        .child(directory_block().class("h-116"))
        .child(network_app_block().class("h-96"))
        .child(identity_card(
            "Local identity",
            "node instance",
            "policy:personal",
            800,
        ))
}

pub const EXTRACTED_ICON_LIBRARY: &[UiIcon] = &[
    UiIcon::Settings,
    UiIcon::Search,
    UiIcon::Shield,
    UiIcon::Wallet,
    UiIcon::App,
    UiIcon::Route,
    UiIcon::Storage,
];
