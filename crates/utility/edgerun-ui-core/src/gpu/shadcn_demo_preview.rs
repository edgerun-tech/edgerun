//! Native preview fixtures for shadcn-compatible demo components.

use super::*;

pub const SHADCN_DEMO_PREVIEW_BASE_ID: u32 = 18_000;

pub fn build_shadcn_demo_preview(slug: &str) -> Option<UiNode> {
    let spec = find_shadcn_demo_by_slug(slug)?;
    if !spec.has_native_renderer() {
        return None;
    }
    Some(frame(spec, build_shadcn_component_preview(spec.slug)?))
}

pub fn build_shadcn_demo_gallery() -> UiNode {
    let mut grid = grid_auto("grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4");
    for spec in SHADCN_DEMO_COMPONENTS {
        if let Some(preview) = build_shadcn_demo_preview(spec.slug) {
            grid = grid.child(preview);
        }
    }
    scroll_area("h-full p-4", 0.0).child(grid)
}

pub fn build_shadcn_component_preview(slug: &str) -> Option<UiNode> {
    let spec = find_shadcn_demo_by_slug(slug)?;
    if !spec.has_native_renderer() {
        return None;
    }
    shadcn_demo_body(spec.slug)
}

pub fn build_shadcn_component_preview_by_source_component(
    source_component: &str,
) -> Option<UiNode> {
    let spec = find_shadcn_demo_by_source_component(source_component)?;
    build_shadcn_component_preview(spec.slug)
}

pub fn build_shadcn_component_preview_by_identifier(identifier: &str) -> Option<UiNode> {
    let resolved = resolve_shadcn_demo_identifier(identifier)?;
    build_shadcn_component_preview(resolved.spec.slug)
}

pub fn build_shadcn_demo_preview_by_identifier(identifier: &str) -> Option<UiNode> {
    let resolved = resolve_shadcn_demo_identifier(identifier)?;
    build_shadcn_demo_preview(resolved.spec.slug)
}

fn frame(spec: &UiShadcnDemoSpec, body: UiNode) -> UiNode {
    card("bg-card border rounded-xl p-4 gap-3")
        .child(
            header(spec.name)
                .detail(spec.category_label())
                .class("h-12"),
        )
        .child(body)
}

fn shadcn_demo_body(slug: &str) -> Option<UiNode> {
    match slug {
        "accordion" => Some(shadcn_accordion(
            &[
                (
                    "Is it accessible?",
                    "Yes. It follows the WAI-ARIA design pattern.",
                ),
                ("Is it styled?", ""),
            ],
            id(47),
        )),
        "alert" => Some(shadcn_alert(
            "Heads up",
            "You can add components to your app using the CLI.",
            UiIcon::Warning,
        )),
        "alert-dialog" => Some(shadcn_alert_dialog(
            "Are you absolutely sure?",
            "This action cannot be undone. This will permanently remove the selected item.",
            UiIcon::Warning,
        )),
        "aspect-ratio" => Some(shadcn_aspect_ratio("16:9", UiIcon::Eye)),
        "avatar" => Some(
            row("gap-3 items-center h-12")
                .child(shadcn_avatar("CN", palette::ACCENT).online(true))
                .child(shadcn_avatar("ER", palette::GREEN))
                .child(shadcn_avatar("UI", palette::VIOLET)),
        ),
        "badge" => Some(
            row("gap-2 items-center h-9")
                .child(shadcn_badge("Default", UiShadcnBadgeVariant::Default))
                .child(shadcn_badge("Secondary", UiShadcnBadgeVariant::Secondary))
                .child(shadcn_badge("Outline", UiShadcnBadgeVariant::Outline))
                .child(shadcn_badge(
                    "Destructive",
                    UiShadcnBadgeVariant::Destructive,
                )),
        ),
        "breadcrumb" => {
            Some(shadcn_breadcrumb(&["Docs", "Components", "Breadcrumb"], 2, id(1)).class("h-9"))
        }
        "button" => Some(
            row("gap-2 items-center h-10")
                .child(shadcn_button(
                    "Button",
                    id(2),
                    UiShadcnButtonVariant::Default,
                    UiShadcnButtonSize::Default,
                ))
                .child(shadcn_button(
                    "Secondary",
                    id(3),
                    UiShadcnButtonVariant::Secondary,
                    UiShadcnButtonSize::Default,
                ))
                .child(shadcn_button(
                    "Ghost",
                    id(4),
                    UiShadcnButtonVariant::Ghost,
                    UiShadcnButtonSize::Default,
                )),
        ),
        "button-group" => Some(shadcn_button_group(&["Copy", "Paste", "More"], id(27))),
        "calendar" => Some(shadcn_calendar(
            "June 2025",
            &["8", "9", "10", "11", "12", "13", "14"],
            2,
            id(48),
        )),
        "card" => Some(
            shadcn_card("Create project", "Deploy your new project in one click.")
                .child(field_node("Name", "shadcn-demo"))
                .child(row("gap-2 h-9").child(button("Deploy", id(5), ButtonStyle::Primary))),
        ),
        "carousel" => Some(shadcn_carousel(&["1", "2", "3"], id(57))),
        "chart" => Some(
            shadcn_chart(
                "Visitors",
                &["Jan", "Feb", "Mar", "Apr", "May", "Jun"],
                &[0.42, 0.68, 0.51, 0.82, 0.56, 0.74],
            )
            .class("h-44"),
        ),
        "checkbox" => Some(
            column("gap-2")
                .child(shadcn_checkbox("Accept terms and conditions", true, id(6)))
                .child(shadcn_checkbox("Receive security emails", false, id(7)).disabled(true)),
        ),
        "collapsible" => Some(shadcn_collapsible(
            "@peduarte starred 3 repositories",
            &[
                ("@radix-ui/primitives", "Open source UI components"),
                ("@radix-ui/colors", "Beautiful color scales"),
            ],
            id(30),
        )),
        "combobox" => Some(shadcn_combobox(
            "Framework",
            "Select framework...",
            "Search framework...",
            &["Next.js", "SvelteKit"],
            0,
            id(59),
        )),
        "command" => Some(shadcn_command("Type a command or search...", id(8)).class("h-12")),
        "context-menu" => Some(shadcn_context_menu(
            "Right click area",
            "Open menu",
            &[
                ("Back", "", false),
                ("Reload", "", true),
                ("Save page as...", "⌘S", false),
            ],
            id(63),
        )),
        "data-table" => Some(shadcn_data_table(
            &["Task", "Status", "Owner"],
            &[
                &["INV001", "Paid", "Olivia"],
                &["INV002", "Pending", "Jackson"],
                &["INV003", "Failed", "Isabella"],
            ],
            id(33),
        )),
        "date-picker" => Some(shadcn_date_picker(
            "Pick a date",
            "June 2025",
            &["10", "11", "12"],
            0,
            id(66),
        )),
        "dialog" => Some(
            shadcn_dialog(
                "Edit profile",
                "Make changes to your profile here. Click save when you're done.",
                UiIcon::Settings,
            )
            .class("h-52"),
        ),
        "drawer" => Some(shadcn_drawer(
            "Move goal",
            "Set your daily activity target.",
            "Calories",
            0.58,
            id(34),
        )),
        "dropdown-menu" => Some(shadcn_dropdown_menu(
            &[
                ("Profile", "⌘P", false),
                ("Billing", "⌘B", true),
                ("Log out", "⇧⌘Q", false),
            ],
            id(9),
        )),
        "direction" => Some(shadcn_direction(
            "Left to right content",
            "Right to left content",
        )),
        "empty" => Some(shadcn_empty(
            "No results found",
            "Try adjusting your search or filters.",
            UiIcon::Search,
        )),
        "field" => Some(
            shadcn_field("Email", "name@example.com")
                .detail("Enter the email address for notifications.")
                .focused(true),
        ),
        "hover-card" => Some(shadcn_hover_card(
            "ER",
            "UI infrastructure",
            "User-owned app surfaces with reusable native components.",
            palette::ACCENT,
        )),
        "input" => Some(shadcn_input("Email", "m@example.com").focused(true)),
        "input-group" => Some(shadcn_input_group(
            "URL",
            "https://example.com",
            "Copy",
            id(12),
        )),
        "input-otp" => Some(shadcn_input_otp(&["1", "2", "3", "-", "", "", ""], 4)),
        "item" => Some(shadcn_item(
            "Payment successful",
            "Stripe payout completed",
            id(13),
            palette::GREEN,
        )),
        "kbd" => Some(shadcn_kbd(&["⌘", "K"], "Command menu")),
        "label" => Some(
            column("gap-2")
                .child(shadcn_label("Email"))
                .child(shadcn_input("", "name@example.com")),
        ),
        "menubar" => Some(shadcn_menubar(
            &["File", "Edit", "View", "Profiles"],
            0,
            id(70),
        )),
        "native-select" => Some(shadcn_native_select("Country", "United States", id(36))),
        "navigation-menu" => Some(shadcn_navigation_menu(
            &["Getting started", "Components", "Docs"],
            0,
            "Introduction",
            "Reusable components built with EdgeRun primitives.",
            "Installation",
            "Add components to your app",
            id(74),
        )),
        "pagination" => Some(shadcn_pagination(&["1", "2"], 0, id(37))),
        "popover" => Some(shadcn_popover(
            "Open popover",
            "Dimensions",
            "Set the dimensions for the layer.",
            "Width",
            "100%",
            id(41),
        )),
        "progress" => Some(column("gap-3").child(shadcn_progress(0.66).class("w-full"))),
        "radio-group" => Some(shadcn_radio_group(
            &[
                ("Default", true),
                ("Comfortable", false),
                ("Compact", false),
            ],
            id(14),
        )),
        "resizable" => Some(shadcn_resizable(&["One", "Two", "Three"])),
        "scroll-area" => Some(shadcn_scroll_area(
            &[
                ("v1.0.0", "Initial release"),
                ("v1.1.0", "Component updates"),
                ("v1.2.0", "Preset builder"),
            ],
            id(17),
        )),
        "select" => Some(shadcn_select("Framework", "Next.js", id(20))),
        "separator" => Some(
            column("gap-3")
                .child(text("Radix Primitives"))
                .child(shadcn_separator())
                .child(text("Styled with EdgeRun UI tokens")),
        ),
        "sidebar" => Some(shadcn_sidebar(
            "App",
            "Workspace",
            &["Dashboard", "Transactions", "Settings"],
            0,
            "Dashboard",
            "Main content area",
            id(78),
        )),
        "sheet" => Some(shadcn_sheet(
            "Edit profile",
            "Make changes to your profile here.",
            "Name",
            "EdgeRun",
            "Save changes",
            id(42),
        )),
        "skeleton" => Some(
            column("gap-3")
                .child(shadcn_skeleton().class("w-full h-6"))
                .child(shadcn_skeleton().class("w-2/3 h-6"))
                .child(shadcn_skeleton().class("w-1/2 h-6")),
        ),
        "slider" => Some(shadcn_slider("Volume", 0.42, id(21)).range_labels("0", "100")),
        "sonner" => Some(shadcn_sonner(&[
            ("Event has been created", UiIcon::Check, palette::GREEN),
            ("Upload failed", UiIcon::Warning, palette::DANGER),
        ])),
        "switch" => Some(
            row("gap-3 items-center h-10")
                .child(shadcn_switch(true, id(22)))
                .child(text("Airplane mode")),
        ),
        "table" => Some(shadcn_table(
            &["Invoice", "Status", "Amount"],
            &[
                &["INV001", "Paid", "$250.00"],
                &["INV002", "Pending", "$150.00"],
            ],
            id(43),
        )),
        "tabs" => Some(shadcn_tabs(&["Account", "Password", "Settings"], 0, id(23))),
        "textarea" => Some(shadcn_textarea("Message", "Type your message here.").focused(true)),
        "toast" => Some(shadcn_toast(
            "Scheduled: Catch up",
            UiIcon::Bell,
            palette::ACCENT,
        )),
        "toggle" => Some(
            row("gap-3 items-center h-10")
                .child(shadcn_toggle(true, id(24)))
                .child(shadcn_toggle(false, id(25))),
        ),
        "toggle-group" => Some(shadcn_toggle_group(&["B", "I", "U"], 0, id(44))),
        "tooltip" => Some(
            row("gap-3 items-center h-10")
                .child(button("Hover", id(26), ButtonStyle::Secondary).class("h-9"))
                .child(shadcn_tooltip("Add to library")),
        ),
        _ => None,
    }
}

const fn id(offset: u32) -> u32 {
    SHADCN_DEMO_PREVIEW_BASE_ID + offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_demo_previews_match_catalog_progress() {
        let preview_count = SHADCN_DEMO_COMPONENTS
            .iter()
            .filter(|spec| build_shadcn_demo_preview(spec.slug).is_some())
            .count();
        assert_eq!(preview_count, shadcn_native_demo_count());
    }

    #[test]
    fn component_previews_are_available_without_demo_frames() {
        for spec in SHADCN_DEMO_COMPONENTS {
            assert!(build_shadcn_component_preview(spec.slug).is_some());
        }
        assert!(build_shadcn_component_preview_by_source_component("InputGroup").is_some());
        assert!(build_shadcn_component_preview_by_identifier("@/components/ui/button").is_some());
        assert!(build_shadcn_component_preview_by_identifier("CardHeader").is_some());
        assert!(build_shadcn_demo_preview_by_identifier("data-slot=\"dialog-content\"").is_some());
        assert!(build_shadcn_component_preview_by_source_component("Unknown").is_none());
        assert!(build_shadcn_component_preview_by_identifier("Unknown").is_none());
    }

    #[test]
    fn cataloged_only_demos_do_not_claim_native_preview() {
        assert!(build_shadcn_demo_preview("unknown-demo").is_none());
        assert!(build_shadcn_component_preview("unknown-demo").is_none());
        assert!(build_shadcn_demo_preview("accordion").is_some());
        assert!(build_shadcn_demo_preview("button").is_some());
        assert!(build_shadcn_demo_preview("input-group").is_some());
    }

    #[test]
    fn native_demo_gallery_renders_to_scene() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            build_shadcn_demo_gallery().render(
                &mut ui,
                UiRect {
                    x: 0.0,
                    y: 0.0,
                    w: 900.0,
                    h: 720.0,
                },
            );
        }
        assert!(scene.rects().len() > 20);
    }
}
