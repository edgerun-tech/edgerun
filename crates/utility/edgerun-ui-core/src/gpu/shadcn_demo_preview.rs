//! Native preview fixtures for shadcn-compatible demo components.

use super::*;

pub const SHADCN_DEMO_PREVIEW_BASE_ID: u32 = 18_000;
const SELECT_CURRENCY_BASE_ID: u32 = SHADCN_DEMO_PREVIEW_BASE_ID + 1_100;
const SELECT_ORDER_BASE_ID: u32 = SHADCN_DEMO_PREVIEW_BASE_ID + 1_120;
const SELECT_TICKER_BASE_ID: u32 = SHADCN_DEMO_PREVIEW_BASE_ID + 1_140;
const CHART_CONTRIBUTION_BASE_ID: u32 = SHADCN_DEMO_PREVIEW_BASE_ID + 1_200;
const CHART_STOCK_BASE_ID: u32 = SHADCN_DEMO_PREVIEW_BASE_ID + 1_220;
const CHART_POWER_BASE_ID: u32 = SHADCN_DEMO_PREVIEW_BASE_ID + 1_240;
const SELECT_PREFERRED_CURRENCY_ID: u32 = SHADCN_DEMO_PREVIEW_BASE_ID + 942;
const SELECT_ORDER_TYPE_ID: u32 = SHADCN_DEMO_PREVIEW_BASE_ID + 946;
const SELECT_DEFAULT_CURRENCY_ID: u32 = SHADCN_DEMO_PREVIEW_BASE_ID + 968;
const SELECT_TICKER_ID: u32 = SHADCN_DEMO_PREVIEW_BASE_ID + 1056;

const CURRENCY_OPTIONS: &[&str] = &[
    "USD - United States Dollar",
    "EUR - Euro",
    "JPY - Japanese Yen",
];
const ORDER_OPTIONS: &[&str] = &["Market Order", "Limit Order", "Stop Order"];
const TICKER_OPTIONS: &[&str] = &["VOO", "VIG", "AAPL", "O"];

#[derive(Clone, Debug, PartialEq)]
pub struct UiShadcnDemoGalleryState {
    open_select: Option<u32>,
    currency_index: usize,
    order_index: usize,
    ticker_index: usize,
    contribution_bar: usize,
    stock_bar: usize,
    power_bar: usize,
    sliders: Vec<(u32, f32)>,
}

impl Default for UiShadcnDemoGalleryState {
    fn default() -> Self {
        Self {
            open_select: None,
            currency_index: 0,
            order_index: 0,
            ticker_index: 0,
            contribution_bar: 5,
            stock_bar: 5,
            power_bar: 6,
            sliders: Vec::new(),
        }
    }
}

impl UiShadcnDemoGalleryState {
    pub fn apply_action(&mut self, action: &UiAction) -> bool {
        match action {
            UiAction::OpenChanged { id, open }
                if matches!(
                    *id,
                    SELECT_PREFERRED_CURRENCY_ID
                        | SELECT_ORDER_TYPE_ID
                        | SELECT_DEFAULT_CURRENCY_ID
                        | SELECT_TICKER_ID
                ) =>
            {
                self.open_select = open.then_some(*id);
                true
            }
            UiAction::Activated(hit)
                if hit.kind == HitKind::MenuItem
                    && option_index(hit.id, SELECT_CURRENCY_BASE_ID, CURRENCY_OPTIONS.len())
                        .is_some() =>
            {
                self.currency_index =
                    option_index(hit.id, SELECT_CURRENCY_BASE_ID, CURRENCY_OPTIONS.len()).unwrap();
                self.open_select = None;
                true
            }
            UiAction::Activated(hit)
                if hit.kind == HitKind::MenuItem
                    && option_index(hit.id, SELECT_ORDER_BASE_ID, ORDER_OPTIONS.len())
                        .is_some() =>
            {
                self.order_index =
                    option_index(hit.id, SELECT_ORDER_BASE_ID, ORDER_OPTIONS.len()).unwrap();
                self.open_select = None;
                true
            }
            UiAction::Activated(hit)
                if hit.kind == HitKind::MenuItem
                    && option_index(hit.id, SELECT_TICKER_BASE_ID, TICKER_OPTIONS.len())
                        .is_some() =>
            {
                self.ticker_index =
                    option_index(hit.id, SELECT_TICKER_BASE_ID, TICKER_OPTIONS.len()).unwrap();
                self.open_select = None;
                true
            }
            UiAction::SliderChanged { id, value } => {
                set_gallery_slider(&mut self.sliders, *id, *value);
                true
            }
            UiAction::Activated(hit)
                if hit.kind == HitKind::Button
                    && option_index(hit.id, CHART_CONTRIBUTION_BASE_ID, 6).is_some() =>
            {
                self.contribution_bar =
                    option_index(hit.id, CHART_CONTRIBUTION_BASE_ID, 6).unwrap();
                true
            }
            UiAction::Activated(hit)
                if hit.kind == HitKind::Button
                    && option_index(hit.id, CHART_STOCK_BASE_ID, 6).is_some() =>
            {
                self.stock_bar = option_index(hit.id, CHART_STOCK_BASE_ID, 6).unwrap();
                true
            }
            UiAction::Activated(hit)
                if hit.kind == HitKind::Button
                    && option_index(hit.id, CHART_POWER_BASE_ID, 8).is_some() =>
            {
                self.power_bar = option_index(hit.id, CHART_POWER_BASE_ID, 8).unwrap();
                true
            }
            _ => false,
        }
    }

    fn slider(&self, id: u32, fallback: f32) -> f32 {
        self.sliders
            .iter()
            .find_map(|(stored_id, value)| (*stored_id == id).then_some(*value))
            .unwrap_or(fallback)
            .clamp(0.0, 1.0)
    }

    fn select_open(&self, id: u32) -> bool {
        self.open_select == Some(id)
    }
}

fn option_index(id: u32, base: u32, len: usize) -> Option<usize> {
    (id >= base && id < base + len as u32).then_some((id - base) as usize)
}

fn set_gallery_slider(sliders: &mut Vec<(u32, f32)>, id: u32, value: f32) {
    if let Some((_, stored)) = sliders.iter_mut().find(|(stored_id, _)| *stored_id == id) {
        *stored = value;
    } else {
        sliders.push((id, value));
    }
}

pub fn build_shadcn_demo_preview(slug: &str) -> Option<UiNode> {
    let spec = find_shadcn_demo_by_slug(slug)?;
    if !spec.has_native_renderer() {
        return None;
    }
    Some(frame(spec, build_shadcn_component_preview(spec.slug)?))
}

pub fn build_shadcn_demo_gallery() -> UiNode {
    build_shadcn_demo_gallery_with_state(&UiShadcnDemoGalleryState::default())
}

pub fn build_shadcn_demo_gallery_with_state(state: &UiShadcnDemoGalleryState) -> UiNode {
    build_shadcn_components_wall(state)
}

pub fn build_shadcn_components_wall(state: &UiShadcnDemoGalleryState) -> UiNode {
    row("h-full bg-bg")
        .child(shadcn_showcase_style_rail())
        .child(
            column("h-full flex-1 bg-bg")
                .child(shadcn_showcase_topbar())
                .child(
                    scroll_area("h-full p-7 gap-0", 0.0)
                        .scroll_id(SHADCN_DEMO_PREVIEW_BASE_ID + 900)
                        .child(shadcn_showcase_board(state)),
                ),
        )
}

fn shadcn_showcase_topbar() -> UiNode {
    row("h-14 px-5 gap-3 items-center bg-bg")
        .child(icon(UiIcon::Sparkles).class("w-5 h-5"))
        .child(
            shadcn_button(
                "Docs",
                id(900),
                UiShadcnButtonVariant::Ghost,
                UiShadcnButtonSize::Sm,
            )
            .class("w-16"),
        )
        .child(
            shadcn_button(
                "Components",
                id(901),
                UiShadcnButtonVariant::Ghost,
                UiShadcnButtonSize::Sm,
            )
            .class("w-32"),
        )
        .child(
            shadcn_button(
                "Blocks",
                id(902),
                UiShadcnButtonVariant::Ghost,
                UiShadcnButtonSize::Sm,
            )
            .class("w-20"),
        )
        .child(
            shadcn_button(
                "Charts",
                id(903),
                UiShadcnButtonVariant::Ghost,
                UiShadcnButtonSize::Sm,
            )
            .class("w-20"),
        )
        .child(
            shadcn_button(
                "Directory",
                id(904),
                UiShadcnButtonVariant::Ghost,
                UiShadcnButtonSize::Sm,
            )
            .class("w-24"),
        )
        .child(spacer("flex-1"))
        .child(shadcn_command("Search documentation...", id(905)).class("w-80 h-9"))
        .child(icon_button(UiIcon::Network, id(906)).class("size-9"))
        .child(shadcn_badge("11.4k", UiShadcnBadgeVariant::Secondary))
        .child(shadcn_button(
            "Open in v0",
            id(907),
            UiShadcnButtonVariant::Secondary,
            UiShadcnButtonSize::Sm,
        ))
        .child(shadcn_button(
            "Get Code",
            id(908),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
}

fn shadcn_showcase_style_rail() -> UiNode {
    column("w-44 h-full bg-sidebar p-2 gap-2")
        .child(
            row("h-9 gap-2 items-center bg-panel rounded-lg px-2")
                .child(text("Menu").class("text-text flex-1"))
                .child(icon_button(UiIcon::Menu, id(920)).class("size-8")),
        )
        .child(shadcn_rail_item("Style", "Nova", UiIcon::App, id(921)))
        .child(shadcn_rail_item(
            "Base Color",
            "Neutral",
            UiIcon::Sparkles,
            id(922),
        ))
        .child(shadcn_rail_item("Theme", "Neutral", UiIcon::Eye, id(923)))
        .child(shadcn_rail_item(
            "Chart Color",
            "Neutral",
            UiIcon::Activity,
            id(924),
        ))
        .child(divider("h-px"))
        .child(shadcn_rail_item(
            "Heading",
            "Geist VF",
            UiIcon::Code,
            id(925),
        ))
        .child(shadcn_rail_item("Font", "Geist VF", UiIcon::Code, id(926)))
        .child(divider("h-px"))
        .child(shadcn_rail_item(
            "Icon Library",
            "Lucide",
            UiIcon::Sparkles,
            id(927),
        ))
        .child(shadcn_rail_item(
            "Radius",
            "Default",
            UiIcon::Route,
            id(928),
        ))
        .child(divider("h-px"))
        .child(shadcn_rail_item("Menu", "Solid", UiIcon::Menu, id(929)))
        .child(shadcn_rail_item(
            "Menu Accent",
            "Subtle",
            UiIcon::Shield,
            id(930),
        ))
        .child(shadcn_button(
            "--preset b0",
            id(931),
            UiShadcnButtonVariant::Secondary,
            UiShadcnButtonSize::Sm,
        ))
        .child(shadcn_button(
            "Open Preset",
            id(932),
            UiShadcnButtonVariant::Secondary,
            UiShadcnButtonSize::Sm,
        ))
        .child(shadcn_button(
            "Shuffle",
            id(933),
            UiShadcnButtonVariant::Secondary,
            UiShadcnButtonSize::Sm,
        ))
        .child(shadcn_button(
            "Get Code",
            id(934),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
}

fn shadcn_rail_item(label: &str, value: &str, item_icon: UiIcon, item_id: u32) -> UiNode {
    row("h-12 gap-2 items-center rounded-lg px-2")
        .child(
            column("gap-0 flex-1")
                .child(text(label).class("text-muted truncate"))
                .child(text(value).class("text-text truncate")),
        )
        .child(icon_button(item_icon, item_id).class("size-8"))
}

fn shadcn_showcase_board(state: &UiShadcnDemoGalleryState) -> UiNode {
    masonry("masonry grid-cols-4 gap-7", 4)
        .child(contribution_history_card(state))
        .child(payout_threshold_card(state))
        .child(savings_targets_card())
        .child(buy_investment_card(state))
        .child(create_release_card())
        .child(claimable_balance_card())
        .child(recent_transactions_card())
        .child(account_access_card())
        .child(mobile_pairing_card())
        .child(preferences_card(state))
        .child(navigation_cards())
        .child(transfer_funds_card())
        .child(dividend_income_card())
        .child(room_controls_card(state))
        .child(support_tabs_card())
        .child(cover_art_card())
        .child(dollar_cost_card())
        .child(savings_ring_card())
        .child(holdings_table_card())
        .child(skeleton_loading_card())
        .child(syncing_accounts_card())
        .child(payout_preferences_card())
        .child(power_usage_card(state))
        .child(connect_bank_card())
        .child(upcoming_payments_card())
        .child(smart_lock_card())
        .child(stock_performance_card(state))
        .child(catalog_empty_card())
        .child(milestone_card())
        .child(social_links_card())
        .child(notifications_card())
}

fn gallery_card(classes: &str) -> UiNode {
    card(&format!("bg-panel-gradient rounded-xl shadow-sm {classes}"))
}

fn contribution_history_card(state: &UiShadcnDemoGalleryState) -> UiNode {
    let values = boosted_values(
        &[0.42, 0.66, 0.48, 0.78, 0.38, 0.84],
        state.contribution_bar,
        state.slider(943, 0.24) * 0.16,
    );
    gallery_card("p-4 gap-4")
        .child(header("Contribution History").detail("Last 6 months of activity"))
        .child(
            shadcn_chart("", &["Dec", "Jan", "Feb", "Mar", "Apr", "May"], &values)
                .chart_bar_base_id(CHART_CONTRIBUTION_BASE_ID)
                .class("h-40"),
        )
        .child(
            grid("grid-cols-2 gap-3", 2)
                .child(compact_metric(
                    "Upcoming",
                    "May 25, 2024",
                    "$1,000 scheduled",
                ))
                .child(compact_metric(
                    "Auto-save plan",
                    "Accelerated",
                    "Recurring weekly",
                )),
        )
        .child(divider("h-px"))
        .child(shadcn_button(
            "View Full Report",
            id(940),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
}

fn payout_threshold_card(state: &UiShadcnDemoGalleryState) -> UiNode {
    let payout = state.slider(943, 0.24);
    let payout_amount = format!("${:.2}", 50.0 + payout * 9_950.0);
    gallery_card("p-4 gap-3")
        .child(
            row("h-8 gap-2 items-center")
                .child(
                    header("Payout Threshold")
                        .detail("Set minimum balance")
                        .class("flex-1"),
                )
                .child(icon_button(UiIcon::X, id(941)).class("size-8")),
        )
        .child(gallery_select(
            "Preferred Currency",
            CURRENCY_OPTIONS[state.currency_index],
            id(942),
            CURRENCY_OPTIONS,
            SELECT_CURRENCY_BASE_ID,
            state.currency_index,
            state.select_open(id(942)),
        ))
        .child(metric("Minimum Payout Amount", &payout_amount).class("h-20"))
        .child(shadcn_slider("Minimum payout", payout, id(943)).range_labels("$50", "$10,000"))
        .child(
            shadcn_textarea("Notes", "Add any notes for this payout configuration...")
                .class("h-24"),
        )
        .child(shadcn_button(
            "Save Threshold",
            id(944),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
}

fn savings_targets_card() -> UiNode {
    gallery_card("p-4 gap-4")
        .child(
            row("h-9 gap-2 items-center")
                .child(
                    header("Savings Targets")
                        .detail("Active milestones for 2024")
                        .class("flex-1"),
                )
                .child(shadcn_button(
                    "New Goal",
                    id(945),
                    UiShadcnButtonVariant::Secondary,
                    UiShadcnButtonSize::Sm,
                )),
        )
        .child(savings_target_row(
            "Retirement",
            "$420,000",
            0.65,
            "$273,000",
        ))
        .child(savings_target_row(
            "Real Estate",
            "$85,000",
            0.32,
            "$27,200",
        ))
        .child(divider("h-px"))
        .child(text("You have not met your targets for this year.").class("text-muted"))
}

fn savings_target_row(title: &str, value: &str, progress: f32, detail: &str) -> UiNode {
    column("bg-muted-gradient rounded-lg p-3 gap-2")
        .child(text(title).class("text-muted"))
        .child(text(value).class("text-text"))
        .child(progress_bar_node(progress, palette::MUTED).class("h-2"))
        .child(text(detail).class("text-muted"))
}

fn buy_investment_card(state: &UiShadcnDemoGalleryState) -> UiNode {
    gallery_card("p-4 gap-3")
        .child(header("Buy Investment").detail("Market orders execute at the current price."))
        .child(shadcn_field("Amount to Invest", "$ 1,000.00"))
        .child(gallery_select(
            "Order Type",
            ORDER_OPTIONS[state.order_index],
            id(946),
            ORDER_OPTIONS,
            SELECT_ORDER_BASE_ID,
            state.order_index,
            state.select_open(id(946)),
        ))
        .child(
            grid("grid-cols-2 gap-3", 2)
                .child(metric("Estimated Shares", "1.95"))
                .child(metric("Buying Power", "$12,450.00")),
        )
        .child(divider("h-px"))
        .child(shadcn_button(
            "Review Order",
            id(947),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
        .child(
            text("Trades are typically executed within minutes during market hours.")
                .class("text-muted"),
        )
}

fn account_access_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(header("Account Access").detail("Update your credentials or re-authenticate."))
        .child(shadcn_field("Email Address", "artist@studio.inc"))
        .child(shadcn_field("Current Password", "**********").detail("FORGOT?"))
        .child(shadcn_button(
            "Update Security",
            id(948),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
        .child(
            list_row_node("Danger Zone", "Archive account and remove catalog", id(949))
                .accent(palette::DANGER),
        )
}

fn recent_transactions_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(
            row("h-9 gap-2 items-center")
                .child(
                    header("Recent Transactions")
                        .detail("Your latest account activity.")
                        .class("flex-1"),
                )
                .child(shadcn_button(
                    "View All",
                    id(950),
                    UiShadcnButtonVariant::Secondary,
                    UiShadcnButtonSize::Sm,
                )),
        )
        .child(amount_row(
            UiIcon::Wallet,
            "Blue Bottle Coffee",
            "Food & Drink",
            "Today, 10:24 AM",
            "-$6.50",
            false,
            id(951),
        ))
        .child(amount_row(
            UiIcon::Storage,
            "Whole Foods Market",
            "Groceries",
            "Yesterday",
            "-$142.30",
            false,
            id(952),
        ))
        .child(amount_row(
            UiIcon::Wallet,
            "Stripe Payout",
            "Income",
            "Oct 12",
            "+$4,200.00",
            true,
            id(953),
        ))
        .child(amount_row(
            UiIcon::Route,
            "Uber Technologies",
            "Transport",
            "Oct 11",
            "-$24.10",
            false,
            id(954),
        ))
        .child(amount_row(
            UiIcon::Bell,
            "Netflix Subscription",
            "Entertainment",
            "Oct 10",
            "-$19.99",
            false,
            id(955),
        ))
}

fn create_release_card() -> UiNode {
    gallery_card("p-4 gap-3 items-center")
        .child(icon_button(UiIcon::MessagePlus, id(956)).class("size-10"))
        .child(
            header("Distribute Track")
                .detail("Upload your first master to start reaching listeners."),
        )
        .child(text("Spotify, Apple Music, and more.").class("text-muted"))
        .child(shadcn_button(
            "Create Release",
            id(957),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
}

fn mobile_pairing_card() -> UiNode {
    gallery_card("p-4 gap-3 items-center")
        .child(qr_preview())
        .child(
            header("Scan to connect your mobile device")
                .detail("Open the Ledger mobile app and scan this code."),
        )
        .child(shadcn_button(
            "Got it",
            id(958),
            UiShadcnButtonVariant::Secondary,
            UiShadcnButtonSize::Sm,
        ))
}

fn qr_preview() -> UiNode {
    let mut node = grid(
        "grid-cols-13 gap-1 bg-slate-50 rounded-lg p-3 shadow-sm",
        13,
    );
    for row in 0..13 {
        for col in 0..13 {
            node = node.child(qr_cell(qr_module_on(row, col)));
        }
    }
    node.class("w-40")
}

fn qr_cell(on: bool) -> UiNode {
    let class = if on {
        "bg-black h-2"
    } else {
        "bg-slate-50 h-2"
    };
    spacer(class)
}

fn qr_module_on(row: usize, col: usize) -> bool {
    fn finder(row: usize, col: usize, origin_row: usize, origin_col: usize) -> bool {
        let r = row.wrapping_sub(origin_row);
        let c = col.wrapping_sub(origin_col);
        r < 5 && c < 5 && (r == 0 || r == 4 || c == 0 || c == 4 || (r == 2 && c == 2))
    }

    finder(row, col, 0, 0)
        || finder(row, col, 0, 8)
        || finder(row, col, 8, 0)
        || ((row * 7 + col * 11 + row * col) % 5 == 0)
        || ((row + col * 3) % 7 == 0)
}

fn dividend_income_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(
            row("h-9 gap-2 items-center")
                .child(
                    header("Q2 Dividend Income")
                        .detail("Quarterly dividend payouts.")
                        .class("flex-1"),
                )
                .child(icon_button(UiIcon::X, id(959)).class("size-8")),
        )
        .child(holding_row(
            "Vanguard VIG",
            "450 Shares",
            "$1,842.10",
            0.82,
            id(960),
        ))
        .child(holding_row(
            "S&P 500 VOO",
            "112 Shares",
            "$928.40",
            0.62,
            id(961),
        ))
        .child(holding_row(
            "Apple AAPL",
            "85 Shares",
            "$340.00",
            0.44,
            id(962),
        ))
        .child(holding_row(
            "Realty Income",
            "320 Shares",
            "$1,139.50",
            0.74,
            id(963),
        ))
}

fn holding_row(title: &str, detail: &str, value: &str, progress: f32, row_id: u32) -> UiNode {
    row("h-16 bg-row rounded-md p-2 gap-3 items-center")
        .child(
            column("gap-0 flex-1")
                .child(text(title).class("text-text truncate"))
                .child(text(detail).class("text-muted truncate")),
        )
        .child(progress_bar_node(progress, palette::MUTED).class("w-14 h-2"))
        .child(text(value).class("text-text truncate").class("w-24"))
        .hit_id(row_id)
}

fn compact_metric(title: &str, value: &str, detail: &str) -> UiNode {
    column("bg-row rounded-md p-3 gap-2")
        .child(text(title).class("text-muted truncate"))
        .child(text(value).class("text-text truncate"))
        .child(text(detail).class("text-muted truncate"))
}

fn amount_row(
    row_icon: UiIcon,
    title: &str,
    category: &str,
    date: &str,
    amount: &str,
    positive: bool,
    row_id: u32,
) -> UiNode {
    let amount_class = if positive {
        "text-green truncate"
    } else {
        "text-muted truncate"
    };
    row("h-16 bg-row rounded-md p-2 gap-3 items-center")
        .child(
            row("size-9 bg-muted rounded-md items-center justify-center")
                .child(icon(row_icon).class("w-5 h-5")),
        )
        .child(
            column("gap-0 flex-1")
                .child(text(title).class("text-text truncate"))
                .child(text(&format!("{category} - {date}")).class("text-muted truncate")),
        )
        .child(text(amount).class(amount_class).class("w-28"))
        .hit_id(row_id)
}

fn dollar_cost_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(header("Dollar-Cost Averaging").detail("A strategy for building wealth over time."))
        .child(text("Over time, this smooths out the average cost of your investments. When prices drop, your fixed amount buys more shares. When prices rise, you buy fewer.").class("text-muted"))
}

fn syncing_accounts_card() -> UiNode {
    gallery_card("p-4 gap-3 items-center")
        .child(icon_button(UiIcon::Activity, id(964)).class("size-10"))
        .child(header("Syncing your accounts").detail("We're pulling in your latest transactions."))
        .child(text("This usually takes a few seconds.").class("text-muted"))
        .child(shadcn_button(
            "Cancel",
            id(965),
            UiShadcnButtonVariant::Secondary,
            UiShadcnButtonSize::Sm,
        ))
}

fn claimable_balance_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(metric("Claimable Balance", "$0.00"))
        .child(
            row("h-6 gap-2 items-center")
                .child(shadcn_badge(
                    "Pending Setup",
                    UiShadcnBadgeVariant::Secondary,
                ))
                .child(spacer("flex-1")),
        )
        .child(shadcn_table(
            &["Item", "Amount"],
            &[
                &["Net Royalties", "$0.00"],
                &["Processing Fee", "-$0.00"],
                &["Total Ready to Claim", "$0.00 USD"],
            ],
            id(966),
        ))
        .child(
            text(
                "Once your bank is connected, balances over $10.00 are eligible for distribution.",
            )
            .class("text-muted"),
        )
}

fn preferences_card(state: &UiShadcnDemoGalleryState) -> UiNode {
    gallery_card("p-4 gap-3")
        .child(
            row("h-8 gap-2 items-center")
                .child(
                    header("Preferences")
                        .detail("Manage account settings.")
                        .class("flex-1"),
                )
                .child(icon_button(UiIcon::X, id(967)).class("size-8")),
        )
        .child(gallery_select(
            "Default Currency",
            CURRENCY_OPTIONS[state.currency_index],
            id(968),
            CURRENCY_OPTIONS,
            SELECT_CURRENCY_BASE_ID,
            state.currency_index,
            state.select_open(id(968)),
        ))
        .child(setting_switch(
            "Public Statistics",
            "Allow others to see stream count.",
            true,
            id(969),
        ))
        .child(setting_switch(
            "Email Notifications",
            "Monthly royalty reports and updates.",
            true,
            id(970),
        ))
        .child(
            row("h-10 gap-2")
                .child(shadcn_button(
                    "Reset",
                    id(971),
                    UiShadcnButtonVariant::Secondary,
                    UiShadcnButtonSize::Sm,
                ))
                .child(spacer("flex-1"))
                .child(shadcn_button(
                    "Save Preferences",
                    id(972),
                    UiShadcnButtonVariant::Default,
                    UiShadcnButtonSize::Sm,
                )),
        )
}

fn setting_switch(title: &str, detail: &str, checked: bool, item_id: u32) -> UiNode {
    row("h-14 gap-3 items-center")
        .child(
            column("gap-1 flex-1")
                .child(text(title).class("text-text"))
                .child(text(detail).class("text-muted")),
        )
        .child(shadcn_switch(checked, item_id))
}

fn savings_ring_card() -> UiNode {
    gallery_card("p-4 gap-3 items-center")
        .child(progress_ring(0.8, palette::MUTED).class("h-40"))
        .child(metric("Projected Finish", "October 2024"))
        .child(metric("Monthly Average", "$1,250"))
        .child(metric("Top Contributor", "Auto-Transfer"))
}

fn room_controls_card(state: &UiShadcnDemoGalleryState) -> UiNode {
    gallery_card("p-4 gap-3")
        .child(setting_switch(
            "Kitchen Island",
            "Hue Color Ambient",
            true,
            id(973),
        ))
        .child(
            grid("grid-cols-2 gap-2", 2)
                .child(shadcn_badge("Cooking", UiShadcnBadgeVariant::Secondary))
                .child(shadcn_badge("Dining", UiShadcnBadgeVariant::Secondary))
                .child(shadcn_badge("Nightlight", UiShadcnBadgeVariant::Default))
                .child(shadcn_badge("Focus", UiShadcnBadgeVariant::Secondary)),
        )
        .child(shadcn_slider(
            "Brightness",
            state.slider(974, 0.82),
            id(974),
        ))
        .child(shadcn_slider(
            "Color Temp",
            state.slider(975, 0.66),
            id(975),
        ))
        .child(shadcn_slider("Volume", state.slider(976, 0.42), id(976)))
        .child(shadcn_slider("Fade", state.slider(977, 0.26), id(977)))
}

fn navigation_cards() -> UiNode {
    column("gap-4")
        .child(
            grid("grid-cols-2 gap-4", 2)
                .child(
                    gallery_card("p-3 gap-2")
                        .child(header("Overview"))
                        .child(menu_item_node("Dashboard", id(980)).selected(true))
                        .child(menu_item_node("Transactions", id(981)))
                        .child(menu_item_node("Investments", id(982))),
                )
                .child(
                    gallery_card("p-3 gap-2")
                        .child(header("Account"))
                        .child(menu_item_node("Profile", id(983)))
                        .child(menu_item_node("Billing", id(984)).selected(true))
                        .child(menu_item_node("Notifications", id(985)))
                        .child(menu_item_node("Security", id(986))),
                ),
        )
        .child(
            gallery_card("p-4 gap-3")
                .child(shadcn_breadcrumb(&["Home", "Payments"], 1, id(990)).class("h-9"))
                .child(list_row_node(
                    "Change transfer limit",
                    "Adjust how much you can send.",
                    id(992),
                ))
                .child(list_row_node(
                    "Scheduled transfers",
                    "Set up a transfer for later.",
                    id(993),
                ))
                .child(list_row_node(
                    "Direct Debits",
                    "Manage recurring payments.",
                    id(994),
                )),
        )
}

fn support_tabs_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(shadcn_tabs(&["General", "Billing", "Goals"], 0, id(1000)))
        .child(
            header("How secure is my financial data with Ledger?")
                .detail("We use bank-level AES-256 encryption and never store credentials."),
        )
        .child(list_row_node(
            "Connect accounts",
            "Approve read-only access in Account Access.",
            id(1004),
        ))
        .child(list_row_node(
            "Export tax data",
            "Exports are available from Reports.",
            id(1005),
        ))
        .child(shadcn_button(
            "Contact Support",
            id(1008),
            UiShadcnButtonVariant::Secondary,
            UiShadcnButtonSize::Sm,
        ))
}

fn holdings_table_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(shadcn_command("Search holdings or tickers...", id(1009)).class("h-10"))
        .child(
            row("h-9 gap-2")
                .child(shadcn_badge("Stocks", UiShadcnBadgeVariant::Secondary))
                .child(shadcn_badge("ETFs", UiShadcnBadgeVariant::Default))
                .child(shadcn_badge("REITs", UiShadcnBadgeVariant::Secondary)),
        )
        .child(holding_row(
            "Vanguard S&P 500 ETF",
            "112 Shares - Jan 2021",
            "$48,230.40",
            0.78,
            id(1010),
        ))
        .child(holding_row(
            "Vanguard Dividend Appreciation",
            "450 Shares - Mar 2022",
            "$26,033.79",
            0.48,
            id(1011),
        ))
        .child(holding_row(
            "Apple Inc.",
            "85 Shares - Nov 2020",
            "$18,488.90",
            0.64,
            id(1012),
        ))
        .child(holding_row(
            "Realty Income Corp",
            "320 Shares - Jun 2023",
            "$15,136.59",
            0.52,
            id(1013),
        ))
}

fn transfer_funds_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(header("Transfer Funds").detail("Move money between connected accounts."))
        .child(shadcn_field("Amount to Transfer", "$ 1,200.00"))
        .child(shadcn_select(
            "From Account",
            "Main Checking (-8402) - $12,450.00",
            id(1014),
        ))
        .child(shadcn_select(
            "To Account",
            "High Yield Savings (-1192) - $42,100.00",
            id(1015),
        ))
        .child(shadcn_table(
            &["Estimate", "Value"],
            &[
                &["Estimated arrival", "Today, Apr 14"],
                &["Transaction fee", "$0.00"],
                &["Total amount", "$1,200.00"],
            ],
            id(1016),
        ))
        .child(shadcn_button(
            "Confirm Transfer",
            id(1019),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
}

fn cover_art_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(header("Cover Art").detail("Minimum 3000 x 3000px JPEG or PNG only."))
        .child(
            empty_state(
                "Upload Artwork",
                "Drag cover art into this area.",
                UiIcon::File,
            )
            .class("h-56"),
        )
        .child(shadcn_button(
            "Upload Artwork",
            id(1020),
            UiShadcnButtonVariant::Secondary,
            UiShadcnButtonSize::Sm,
        ))
}

fn skeleton_loading_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(shadcn_skeleton().class("h-6 w-32"))
        .child(shadcn_skeleton().class("h-6 w-48"))
        .child(shadcn_skeleton().class("h-28 w-full"))
        .child(shadcn_skeleton().class("h-5 w-full"))
        .child(shadcn_skeleton().class("h-5 w-2/3"))
        .child(
            grid("grid-cols-2 gap-3", 2)
                .child(shadcn_skeleton().class("h-10"))
                .child(shadcn_skeleton().class("h-10")),
        )
}

fn payout_preferences_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(header("Payout Preferences").detail("Receiving Method"))
        .child(shadcn_field(
            "Account Holder Name",
            "Synthetic Horizons Music LLC",
        ))
        .child(shadcn_radio_group(
            &[
                ("Bank Transfer - SWIFT / IBAN", true),
                ("PayPal - Instant Payout", false),
            ],
            id(1021),
        ))
        .child(shadcn_field("IBAN / Account Number", "DE89 3704 0044 ..."))
        .child(shadcn_button(
            "Save Payout Settings",
            id(1024),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
}

fn power_usage_card(state: &UiShadcnDemoGalleryState) -> UiNode {
    let brightness = state.slider(974, 0.82);
    let values = boosted_values(
        &[0.45, 0.62, 0.66, 0.51, 0.71, 0.6, 0.82, 0.68],
        state.power_bar,
        brightness * 0.12,
    );
    gallery_card("p-4 gap-3")
        .child(header("Power Usage").detail("Whole Home"))
        .child(
            shadcn_chart(
                "Usage",
                &["6a", "8a", "10a", "12p", "2p", "4p", "6p", "8p"],
                &values,
            )
            .chart_bar_base_id(CHART_POWER_BASE_ID)
            .class("h-36"),
        )
        .child(
            grid("grid-cols-2 gap-3", 2)
                .child(metric("Currently Using", "3.4 kW"))
                .child(metric("Solar Gen", "+1.2 kW")),
        )
        .child(shadcn_slider(
            "Battery Level",
            state.slider(1025, 0.85),
            id(1025),
        ))
}

fn connect_bank_card() -> UiNode {
    gallery_card("p-4 gap-3 items-center")
        .child(icon_button(UiIcon::Wallet, id(1026)).class("size-10"))
        .child(header("Connect Bank").detail(
            "Link your payout method to receive monthly royalty distributions automatically.",
        ))
        .child(shadcn_button(
            "Set Up Payouts",
            id(1027),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
}

fn upcoming_payments_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(header("Upcoming Payments").detail("Select a date to view scheduled payments."))
        .child(shadcn_calendar(
            "May 2026",
            &[
                "26", "27", "28", "29", "30", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10",
                "11", "12", "13",
            ],
            16,
            id(1028),
        ))
        .child(amount_row(
            UiIcon::Bell,
            "Netflix Subscription",
            "Entertainment",
            "Apr 15, 2024",
            "$19.99",
            false,
            id(1050),
        ))
        .child(amount_row(
            UiIcon::Wallet,
            "Rent Payment",
            "Housing",
            "Apr 1, 2024",
            "$2,400.00",
            false,
            id(1051),
        ))
}

fn smart_lock_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(
            row("h-9 gap-2")
                .child(
                    header("Front Door")
                        .detail("Smart Lock Pro")
                        .class("flex-1"),
                )
                .child(shadcn_badge("Locked", UiShadcnBadgeVariant::Secondary)),
        )
        .child(
            column("bg-muted rounded-lg h-36 p-4 items-end")
                .child(shadcn_badge("Live", UiShadcnBadgeVariant::Destructive)),
        )
        .child(shadcn_slider("Open", 0.35, id(1052)).range_labels("Open", "Close"))
        .child(
            row("h-9 gap-2")
                .child(shadcn_button(
                    "Open",
                    id(1053),
                    UiShadcnButtonVariant::Secondary,
                    UiShadcnButtonSize::Sm,
                ))
                .child(shadcn_button(
                    "Half",
                    id(1054),
                    UiShadcnButtonVariant::Secondary,
                    UiShadcnButtonSize::Sm,
                ))
                .child(shadcn_button(
                    "Closed",
                    id(1055),
                    UiShadcnButtonVariant::Secondary,
                    UiShadcnButtonSize::Sm,
                )),
        )
}

fn stock_performance_card(state: &UiShadcnDemoGalleryState) -> UiNode {
    let values = ticker_values(state.ticker_index, state.stock_bar);
    gallery_card("p-4 gap-3")
        .child(header("Stock Performance").detail("6-month price history."))
        .child(gallery_select(
            "Ticker",
            TICKER_OPTIONS[state.ticker_index],
            id(1056),
            TICKER_OPTIONS,
            SELECT_TICKER_BASE_ID,
            state.ticker_index,
            state.select_open(id(1056)),
        ))
        .child(
            shadcn_chart(
                "Price",
                &["Jan", "Feb", "Mar", "Apr", "May", "Jun"],
                &values,
            )
            .chart_bar_base_id(CHART_STOCK_BASE_ID)
            .class("h-36"),
        )
}

fn catalog_empty_card() -> UiNode {
    gallery_card("p-4 gap-3 items-center")
        .child(icon_button(UiIcon::Code, id(1057)).class("size-10"))
        .child(
            header("Explore Catalog")
                .detail("Check your ISRC codes, metadata, and visual assets before going live."),
        )
        .child(shadcn_button(
            "View Catalog",
            id(1058),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
}

fn milestone_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(header("Set a new milestone").detail("Define your financial target and pace."))
        .child(shadcn_field("Goal Name", "e.g. New Car, Home Downpayment"))
        .child(
            row("h-12 gap-3")
                .child(shadcn_field("Target Amount", "$15,000").class("flex-1"))
                .child(shadcn_field("Target Date", "Dec 2025").class("flex-1")),
        )
        .child(shadcn_button(
            "Create Goal",
            id(1059),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
        .child(shadcn_button(
            "Cancel",
            id(1060),
            UiShadcnButtonVariant::Secondary,
            UiShadcnButtonSize::Sm,
        ))
}

fn social_links_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(row("h-8 gap-2").child(header("Social Links").detail("01    02").class("flex-1")))
        .child(shadcn_field(
            "Spotify Artist URL",
            "spotify.com/artist/3j...2k",
        ))
        .child(shadcn_field("Instagram Handle", "@julianduryea_music"))
        .child(shadcn_field("SoundCloud URL", "soundcloud.com/username"))
        .child(shadcn_field("Website", "https://yoursite.com"))
        .child(
            row("h-10 gap-2")
                .child(spacer("flex-1"))
                .child(shadcn_button(
                    "Discard",
                    id(1061),
                    UiShadcnButtonVariant::Secondary,
                    UiShadcnButtonSize::Sm,
                ))
                .child(shadcn_button(
                    "Save Changes",
                    id(1062),
                    UiShadcnButtonVariant::Default,
                    UiShadcnButtonSize::Sm,
                )),
        )
}

fn notifications_card() -> UiNode {
    gallery_card("p-4 gap-3")
        .child(header("Notifications").detail("Choose what you want to be notified about."))
        .child(shadcn_checkbox("Select all", true, id(1063)))
        .child(shadcn_checkbox("Transaction alerts", true, id(1064)))
        .child(shadcn_checkbox("Security alerts", true, id(1065)))
        .child(shadcn_checkbox("Goal milestones", false, id(1066)))
        .child(shadcn_checkbox("Market updates", false, id(1067)))
        .child(shadcn_button(
            "Save Preferences",
            id(1068),
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        ))
}

fn gallery_select(
    label: &str,
    value: &str,
    select_id: u32,
    options: &[&str],
    option_base_id: u32,
    selected: usize,
    open: bool,
) -> UiNode {
    let trigger = shadcn_select(label, value, select_id);
    if !open {
        return trigger;
    }
    let mut menu = column("gap-1 bg-popover border rounded-lg p-1");
    for (index, option) in options.iter().enumerate() {
        menu = menu.child(
            menu_item_node(option, option_base_id + index as u32)
                .selected(index == selected)
                .disabled(index == selected),
        );
    }
    column("gap-2").child(trigger).child(menu)
}

fn boosted_values(base: &[f32], selected: usize, boost: f32) -> Vec<f32> {
    base.iter()
        .enumerate()
        .map(|(index, value)| {
            if index == selected {
                (value + boost.max(0.12)).min(1.0)
            } else {
                *value
            }
        })
        .collect()
}

fn ticker_values(ticker_index: usize, selected: usize) -> Vec<f32> {
    let base = match ticker_index {
        1 => &[0.38, 0.45, 0.51, 0.57, 0.61, 0.66][..],
        2 => &[0.34, 0.58, 0.49, 0.74, 0.63, 0.82][..],
        3 => &[0.62, 0.6, 0.64, 0.61, 0.68, 0.7][..],
        _ => &[0.42, 0.51, 0.46, 0.62, 0.56, 0.68][..],
    };
    boosted_values(base, selected, 0.14)
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
    gallery_card("p-4 gap-3")
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

    #[test]
    fn native_demo_gallery_layout_does_not_overlap_preview_viewport() {
        let issues =
            build_shadcn_demo_gallery().layout_issues(UiRect::new(0.0, 0.0, 1440.0, 940.0));

        assert_eq!(issues, Vec::new());
    }

    #[test]
    fn demo_gallery_state_handles_select_slider_and_chart_actions() {
        let mut state = UiShadcnDemoGalleryState::default();

        assert!(state.apply_action(&UiAction::OpenChanged {
            id: id(946),
            open: true,
        }));
        assert!(state.select_open(id(946)));
        assert!(state.apply_action(&UiAction::Activated(GpuHit::new(
            HitKind::MenuItem,
            SELECT_ORDER_BASE_ID + 1,
            0.0,
            0.0,
            10.0,
            10.0,
        ))));
        assert_eq!(state.order_index, 1);
        assert_eq!(state.open_select, None);

        assert!(state.apply_action(&UiAction::SliderChanged {
            id: id(943),
            value: 0.72,
        }));
        assert_eq!(state.slider(id(943), 0.0), 0.72);

        assert!(state.apply_action(&UiAction::Activated(GpuHit::new(
            HitKind::Button,
            CHART_STOCK_BASE_ID + 2,
            0.0,
            0.0,
            10.0,
            10.0,
        ))));
        assert_eq!(state.stock_bar, 2);
    }
}
