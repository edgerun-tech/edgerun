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
        "accordion" => Some(
            card("bg-panel border rounded-lg p-2 gap-1")
                .child(
                    row("items-center justify-between h-10")
                        .child(text("Is it accessible?"))
                        .child(icon_button(UiIcon::ChevronRight, id(47))),
                )
                .child(text("Yes. It follows the WAI-ARIA design pattern."))
                .child(divider(""))
                .child(
                    row("items-center justify-between h-10")
                        .child(text("Is it styled?"))
                        .child(icon(UiIcon::ChevronRight)),
                ),
        ),
        "alert" => Some(
            row("gap-3 items-start border rounded-lg p-3")
                .child(icon(UiIcon::Warning).class("w-5 h-5"))
                .child(
                    column("gap-1 flex-1")
                        .child(text("Heads up"))
                        .child(text("You can add components to your app using the CLI.")),
                ),
        ),
        "alert-dialog" => Some(
            dialog(
                "Are you absolutely sure?",
                "This action cannot be undone. This will permanently remove the selected item.",
                UiIcon::Warning,
            )
            .class("h-52"),
        ),
        "aspect-ratio" => Some(
            card("bg-panel border rounded-lg p-0 overflow-hidden").child(
                column("aspect-video bg-muted items-center justify-center")
                    .child(icon(UiIcon::Eye).class("w-8 h-8"))
                    .child(text("16:9")),
            ),
        ),
        "avatar" => Some(
            row("gap-3 items-center h-12")
                .child(avatar_node("CN", palette::ACCENT).online(true))
                .child(avatar_node("ER", palette::GREEN))
                .child(avatar_node("UI", palette::VIOLET)),
        ),
        "badge" => Some(
            row("gap-2 items-center h-9")
                .child(badge("Default", palette::ACCENT))
                .child(badge("Secondary", palette::MUTED))
                .child(badge("Outline", palette::BORDER))
                .child(badge("Destructive", palette::DANGER)),
        ),
        "breadcrumb" => {
            Some(breadcrumb(&["Docs", "Components", "Breadcrumb"], 2, id(1)).class("h-9"))
        }
        "button" => Some(
            row("gap-2 items-center h-10")
                .child(button("Button", id(2), ButtonStyle::Primary).class("h-9"))
                .child(button("Secondary", id(3), ButtonStyle::Secondary).class("h-9"))
                .child(button("Ghost", id(4), ButtonStyle::Ghost).class("h-9")),
        ),
        "button-group" => Some(
            row("gap-0 items-center h-10")
                .child(button("Copy", id(27), ButtonStyle::Secondary).class("h-9 rounded-r-none"))
                .child(button("Paste", id(28), ButtonStyle::Secondary).class("h-9 rounded-none"))
                .child(button("More", id(29), ButtonStyle::Secondary).class("h-9 rounded-l-none")),
        ),
        "calendar" => Some(
            card("bg-panel border rounded-lg p-3 gap-3")
                .child(
                    row("items-center justify-between h-8")
                        .child(icon_button(UiIcon::ChevronRight, id(48)).class("rotate-180"))
                        .child(text("June 2025"))
                        .child(icon_button(UiIcon::ChevronRight, id(49))),
                )
                .child(
                    grid("grid-cols-7 gap-1", 7)
                        .child(text("S"))
                        .child(text("M"))
                        .child(text("T"))
                        .child(text("W"))
                        .child(text("T"))
                        .child(text("F"))
                        .child(text("S"))
                        .child(button("8", id(50), ButtonStyle::Ghost))
                        .child(button("9", id(51), ButtonStyle::Ghost))
                        .child(button("10", id(52), ButtonStyle::Secondary))
                        .child(button("11", id(53), ButtonStyle::Ghost))
                        .child(button("12", id(54), ButtonStyle::Ghost))
                        .child(button("13", id(55), ButtonStyle::Ghost))
                        .child(button("14", id(56), ButtonStyle::Ghost)),
                ),
        ),
        "card" => Some(
            card("bg-panel border rounded-lg p-4 gap-3")
                .child(header("Create project").detail("Deploy your new project in one click."))
                .child(field_node("Name", "shadcn-demo"))
                .child(row("gap-2 h-9").child(button("Deploy", id(5), ButtonStyle::Primary))),
        ),
        "carousel" => Some(
            row("gap-3 items-center")
                .child(icon_button(UiIcon::ChevronRight, id(57)).class("rotate-180"))
                .child(
                    card("bg-panel border rounded-lg p-6 items-center justify-center")
                        .child(text("1")),
                )
                .child(
                    card("bg-panel border rounded-lg p-6 items-center justify-center")
                        .child(text("2")),
                )
                .child(
                    card("bg-panel border rounded-lg p-6 items-center justify-center")
                        .child(text("3")),
                )
                .child(icon_button(UiIcon::ChevronRight, id(58))),
        ),
        "chart" => Some(
            bar_chart_labels(
                "Visitors",
                &["Jan", "Feb", "Mar", "Apr", "May", "Jun"],
                &[0.42, 0.68, 0.51, 0.82, 0.56, 0.74],
            )
            .class("h-44"),
        ),
        "checkbox" => Some(
            column("gap-2")
                .child(checkbox("Accept terms and conditions", true, id(6)))
                .child(checkbox("Receive security emails", false, id(7)).disabled(true)),
        ),
        "collapsible" => Some(
            card("bg-panel border rounded-lg p-3 gap-2")
                .child(
                    row("items-center justify-between h-9")
                        .child(text("@peduarte starred 3 repositories"))
                        .child(icon_button(UiIcon::ChevronRight, id(30))),
                )
                .child(list_row_node(
                    "@radix-ui/primitives",
                    "Open source UI components",
                    id(31),
                ))
                .child(list_row_node(
                    "@radix-ui/colors",
                    "Beautiful color scales",
                    id(32),
                )),
        ),
        "combobox" => Some(
            column("gap-2")
                .child(select_node("Framework", "Select framework...", id(59)))
                .child(command_palette("Search framework...", id(60)))
                .child(menu_item_node("Next.js", id(61)).selected(true))
                .child(menu_item_node("SvelteKit", id(62))),
        ),
        "command" => Some(command_palette("Type a command or search...", id(8)).class("h-12")),
        "context-menu" => Some(
            card("bg-panel border rounded-lg p-3 gap-2")
                .child(header("Right click area").detail("Open menu"))
                .child(menu_item_node("Back", id(63)))
                .child(menu_item_node("Reload", id(64)).selected(true))
                .child(menu_item_node("Save page as...", id(65)).detail("⌘S")),
        ),
        "data-table" => Some(table_labels(
            &["Task", "Status", "Owner"],
            &[
                &["INV001", "Paid", "Olivia"],
                &["INV002", "Pending", "Jackson"],
                &["INV003", "Failed", "Isabella"],
            ],
            id(33),
        )),
        "date-picker" => Some(
            column("gap-2")
                .child(button("Pick a date", id(66), ButtonStyle::Secondary).class("h-9"))
                .child(
                    card("bg-panel border rounded-lg p-3 gap-2")
                        .child(text("June 2025"))
                        .child(
                            row("gap-1")
                                .child(button("10", id(67), ButtonStyle::Secondary))
                                .child(button("11", id(68), ButtonStyle::Ghost))
                                .child(button("12", id(69), ButtonStyle::Ghost)),
                        ),
                ),
        ),
        "dialog" => Some(
            dialog(
                "Edit profile",
                "Make changes to your profile here. Click save when you're done.",
                UiIcon::Settings,
            )
            .class("h-52"),
        ),
        "drawer" => Some(
            card("bg-panel border rounded-t-xl p-4 gap-3")
                .child(header("Move goal").detail("Set your daily activity target."))
                .child(slider_node("Calories", 0.58, id(34)))
                .child(row("gap-2").child(button("Submit", id(35), ButtonStyle::Primary))),
        ),
        "dropdown-menu" => Some(
            column("gap-1")
                .child(menu_item_node("Profile", id(9)).detail("⌘P"))
                .child(
                    menu_item_node("Billing", id(10))
                        .detail("⌘B")
                        .selected(true),
                )
                .child(menu_item_node("Log out", id(11)).detail("⇧⌘Q")),
        ),
        "direction" => Some(
            column("gap-2")
                .child(
                    row("gap-2 items-center")
                        .child(badge("LTR", palette::ACCENT))
                        .child(text("Left to right content")),
                )
                .child(
                    row("gap-2 items-center justify-end")
                        .child(text("Right to left content"))
                        .child(badge("RTL", palette::MUTED)),
                ),
        ),
        "empty" => Some(empty_state(
            "No results found",
            "Try adjusting your search or filters.",
            UiIcon::Search,
        )),
        "field" => Some(
            field_node("Email", "name@example.com")
                .detail("Enter the email address for notifications.")
                .focused(true),
        ),
        "hover-card" => Some(
            column("gap-2")
                .child(
                    row("gap-3 items-center")
                        .child(avatar_node("ER", palette::ACCENT))
                        .child(
                            column("gap-1")
                                .child(text("@edgerun"))
                                .child(text("UI infrastructure")),
                        ),
                )
                .child(text(
                    "User-owned app surfaces with reusable native components.",
                )),
        ),
        "input" => Some(field_node("Email", "m@example.com").focused(true)),
        "input-group" => Some(
            row("gap-2 h-12 items-center")
                .child(field_node("URL", "https://example.com").class("flex-1"))
                .child(button("Copy", id(12), ButtonStyle::Secondary).class("h-9 w-20")),
        ),
        "input-otp" => Some(
            row("gap-2 items-center h-12")
                .child(field_node("", "1").class("w-10"))
                .child(field_node("", "2").class("w-10"))
                .child(field_node("", "3").class("w-10"))
                .child(text("-"))
                .child(field_node("", "").class("w-10").focused(true))
                .child(field_node("", "").class("w-10"))
                .child(field_node("", "").class("w-10")),
        ),
        "item" => Some(
            list_row_node("Payment successful", "Stripe payout completed", id(13))
                .accent(palette::GREEN),
        ),
        "kbd" => Some(
            row("gap-2 items-center h-10")
                .child(badge("⌘", palette::MUTED))
                .child(badge("K", palette::MUTED))
                .child(text("Command menu")),
        ),
        "label" => Some(
            column("gap-2")
                .child(text("Email"))
                .child(field_node("", "name@example.com")),
        ),
        "menubar" => Some(
            row("gap-1 items-center border rounded-lg p-1")
                .child(button("File", id(70), ButtonStyle::Secondary))
                .child(button("Edit", id(71), ButtonStyle::Ghost))
                .child(button("View", id(72), ButtonStyle::Ghost))
                .child(button("Profiles", id(73), ButtonStyle::Ghost)),
        ),
        "native-select" => Some(select_node("Country", "United States", id(36))),
        "navigation-menu" => Some(
            column("gap-2")
                .child(
                    row("gap-1 items-center")
                        .child(button("Getting started", id(74), ButtonStyle::Secondary))
                        .child(button("Components", id(75), ButtonStyle::Ghost))
                        .child(button("Docs", id(76), ButtonStyle::Ghost)),
                )
                .child(
                    card("bg-panel border rounded-lg p-3 gap-2")
                        .child(
                            header("Introduction")
                                .detail("Reusable components built with EdgeRun primitives."),
                        )
                        .child(list_row_node(
                            "Installation",
                            "Add components to your app",
                            id(77),
                        )),
                ),
        ),
        "pagination" => Some(
            row("gap-1 items-center h-10")
                .child(button("Previous", id(37), ButtonStyle::Ghost).disabled(true))
                .child(button("1", id(38), ButtonStyle::Secondary))
                .child(button("2", id(39), ButtonStyle::Ghost))
                .child(button("Next", id(40), ButtonStyle::Ghost)),
        ),
        "popover" => Some(
            column("gap-2")
                .child(button("Open popover", id(41), ButtonStyle::Secondary).class("h-9"))
                .child(
                    card("bg-panel border rounded-lg p-3 gap-2")
                        .child(header("Dimensions").detail("Set the dimensions for the layer."))
                        .child(field_node("Width", "100%")),
                ),
        ),
        "progress" => Some(
            column("gap-3")
                .child(progress_bar_node(0.66, palette::ACCENT).class("w-full"))
                .child(progress_ring(0.66, palette::ACCENT).class("w-12 h-12")),
        ),
        "radio-group" => Some(
            column("gap-2")
                .child(radio("Default", true, id(14)))
                .child(radio("Comfortable", false, id(15)))
                .child(radio("Compact", false, id(16))),
        ),
        "resizable" => Some(
            row("gap-1 h-28")
                .child(card("bg-panel border rounded-lg p-3 flex-1").child(text("One")))
                .child(divider("w-1"))
                .child(
                    column("gap-1 flex-1")
                        .child(card("bg-panel border rounded-lg p-3 flex-1").child(text("Two")))
                        .child(card("bg-panel border rounded-lg p-3 flex-1").child(text("Three"))),
                ),
        ),
        "scroll-area" => Some(
            scroll_area("h-32 border rounded-lg p-2", 0.0)
                .child(list_row_node("v1.0.0", "Initial release", id(17)))
                .child(list_row_node("v1.1.0", "Component updates", id(18)))
                .child(list_row_node("v1.2.0", "Preset builder", id(19))),
        ),
        "select" => Some(select_node("Framework", "Next.js", id(20))),
        "separator" => Some(
            column("gap-3")
                .child(text("Radix Primitives"))
                .child(divider(""))
                .child(text("Styled with EdgeRun UI tokens")),
        ),
        "sidebar" => Some(
            row("gap-3 h-44")
                .child(
                    card("bg-sidebar border rounded-lg p-2 gap-1 w-44")
                        .child(header("App").detail("Workspace"))
                        .child(menu_item_node("Dashboard", id(78)).selected(true))
                        .child(menu_item_node("Transactions", id(79)))
                        .child(menu_item_node("Settings", id(80))),
                )
                .child(
                    card("bg-panel border rounded-lg p-4 flex-1")
                        .child(header("Dashboard").detail("Main content area")),
                ),
        ),
        "sheet" => Some(
            card("bg-panel border rounded-lg p-4 gap-3")
                .child(header("Edit profile").detail("Make changes to your profile here."))
                .child(field_node("Name", "EdgeRun"))
                .child(row("gap-2").child(button("Save changes", id(42), ButtonStyle::Primary))),
        ),
        "skeleton" => Some(
            column("gap-3")
                .child(skeleton().class("w-full h-6"))
                .child(skeleton().class("w-2/3 h-6"))
                .child(skeleton().class("w-1/2 h-6")),
        ),
        "slider" => Some(slider_node("Volume", 0.42, id(21)).range_labels("0", "100")),
        "sonner" => Some(
            column("gap-2")
                .child(toast(
                    "Event has been created",
                    UiIcon::Check,
                    palette::GREEN,
                ))
                .child(toast("Upload failed", UiIcon::Warning, palette::DANGER)),
        ),
        "switch" => Some(
            row("gap-3 items-center h-10")
                .child(toggle_node(true, id(22)))
                .child(text("Airplane mode")),
        ),
        "table" => Some(table_labels(
            &["Invoice", "Status", "Amount"],
            &[
                &["INV001", "Paid", "$250.00"],
                &["INV002", "Pending", "$150.00"],
            ],
            id(43),
        )),
        "tabs" => Some(tabs_node(
            ["Account", "Password", "Settings"]
                .into_iter()
                .map(str::to_string),
            0,
            id(23),
        )),
        "textarea" => Some(text_area_node("Message", "Type your message here.").focused(true)),
        "toast" => Some(toast("Scheduled: Catch up", UiIcon::Bell, palette::ACCENT)),
        "toggle" => Some(
            row("gap-3 items-center h-10")
                .child(toggle_node(true, id(24)))
                .child(toggle_node(false, id(25))),
        ),
        "toggle-group" => Some(
            row("gap-1 items-center h-10")
                .child(button("B", id(44), ButtonStyle::Secondary))
                .child(button("I", id(45), ButtonStyle::Ghost))
                .child(button("U", id(46), ButtonStyle::Ghost)),
        ),
        "tooltip" => Some(
            row("gap-3 items-center h-10")
                .child(button("Hover", id(26), ButtonStyle::Secondary).class("h-9"))
                .child(tooltip("Add to library")),
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
