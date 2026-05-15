//! Exact shadcn-compatible component builders.

use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiShadcnButtonVariant {
    #[default]
    Default,
    Destructive,
    Outline,
    Secondary,
    Ghost,
    Link,
}

impl UiShadcnButtonVariant {
    pub const fn class_name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Destructive => "destructive",
            Self::Outline => "outline",
            Self::Secondary => "secondary",
            Self::Ghost => "ghost",
            Self::Link => "link",
        }
    }

    pub const fn button_style(self) -> ButtonStyle {
        match self {
            Self::Default => ButtonStyle::Primary,
            Self::Destructive => ButtonStyle::Danger,
            Self::Outline | Self::Secondary => ButtonStyle::Secondary,
            Self::Ghost | Self::Link => ButtonStyle::Ghost,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiShadcnButtonSize {
    #[default]
    Default,
    Sm,
    Lg,
    Icon,
}

impl UiShadcnButtonSize {
    pub const fn class_name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Sm => "sm",
            Self::Lg => "lg",
            Self::Icon => "icon",
        }
    }

    pub const fn class_suffix(self) -> &'static str {
        match self {
            Self::Default => "h-9 px-4 py-2",
            Self::Sm => "h-8 px-3",
            Self::Lg => "h-10 px-6",
            Self::Icon => "h-9 w-9 px-0",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiShadcnBadgeVariant {
    #[default]
    Default,
    Secondary,
    Destructive,
    Outline,
}

impl UiShadcnBadgeVariant {
    pub const fn class_name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Secondary => "secondary",
            Self::Destructive => "destructive",
            Self::Outline => "outline",
        }
    }

    pub const fn color(self) -> Color4 {
        match self {
            Self::Default => palette::ACCENT,
            Self::Secondary => palette::MUTED,
            Self::Destructive => palette::DANGER,
            Self::Outline => palette::BORDER,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiShadcnChatRole {
    User,
    #[default]
    Assistant,
    Reasoning,
    Diff,
    ToolRunning,
    ToolSuccess,
    ToolError,
    Error,
}

impl UiShadcnChatRole {
    pub const fn label(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Reasoning => "reasoning",
            Self::Diff => "diff",
            Self::ToolRunning => "tool running",
            Self::ToolSuccess => "tool ok",
            Self::ToolError => "tool failed",
            Self::Error => "error",
        }
    }

    pub const fn badge_variant(self) -> UiShadcnBadgeVariant {
        match self {
            Self::ToolError | Self::Error => UiShadcnBadgeVariant::Destructive,
            Self::ToolRunning => UiShadcnBadgeVariant::Default,
            Self::Diff | Self::Reasoning | Self::ToolSuccess => UiShadcnBadgeVariant::Secondary,
            Self::User | Self::Assistant => UiShadcnBadgeVariant::Outline,
        }
    }

    pub const fn icon(self) -> UiIcon {
        match self {
            Self::User => UiIcon::User,
            Self::Assistant => UiIcon::Chat,
            Self::Reasoning => UiIcon::Sparkles,
            Self::Diff => UiIcon::File,
            Self::ToolRunning => UiIcon::Terminal,
            Self::ToolSuccess => UiIcon::Check,
            Self::ToolError | Self::Error => UiIcon::Warning,
        }
    }

    pub const fn is_timeline_event(self) -> bool {
        matches!(
            self,
            Self::Reasoning | Self::ToolRunning | Self::ToolSuccess | Self::ToolError | Self::Error
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiShadcnStatusTone {
    #[default]
    Neutral,
    Active,
    Success,
    Error,
}

impl UiShadcnStatusTone {
    pub const fn badge_variant(self) -> UiShadcnBadgeVariant {
        match self {
            Self::Neutral => UiShadcnBadgeVariant::Secondary,
            Self::Active => UiShadcnBadgeVariant::Default,
            Self::Success => UiShadcnBadgeVariant::Secondary,
            Self::Error => UiShadcnBadgeVariant::Destructive,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnConversationMessage<'a> {
    pub role: UiShadcnChatRole,
    pub body: &'a str,
    pub height: f32,
}

impl<'a> UiShadcnConversationMessage<'a> {
    pub const fn new(role: UiShadcnChatRole, body: &'a str, height: f32) -> Self {
        Self { role, body, height }
    }

    #[cfg(feature = "fontdue-text")]
    pub fn with_measured_height(
        role: UiShadcnChatRole,
        body: &'a str,
        atlas: &FontAtlas,
        card_width: f32,
    ) -> Self {
        Self::new(
            role,
            body,
            shadcn_chat_message_height_for_role(atlas, role, body, card_width),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnChatClientAction<'a> {
    pub label: &'a str,
    pub icon: Option<UiIcon>,
    pub id: u32,
    pub variant: UiShadcnButtonVariant,
}

impl<'a> UiShadcnChatClientAction<'a> {
    pub const fn new(label: &'a str, id: u32, variant: UiShadcnButtonVariant) -> Self {
        Self {
            label,
            icon: None,
            id,
            variant,
        }
    }

    pub const fn icon(icon: UiIcon, id: u32, variant: UiShadcnButtonVariant) -> Self {
        Self {
            label: "",
            icon: Some(icon),
            id,
            variant,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnChatClientIconAction {
    pub icon: UiIcon,
    pub id: u32,
    pub active: bool,
}

impl UiShadcnChatClientIconAction {
    pub const fn new(icon: UiIcon, id: u32, active: bool) -> Self {
        Self { icon, id, active }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnSessionRow<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    pub active: bool,
    pub state: UiShadcnSessionState,
}

impl<'a> UiShadcnSessionRow<'a> {
    pub const fn new(title: &'a str, detail: &'a str, active: bool) -> Self {
        Self {
            title,
            detail,
            active,
            state: UiShadcnSessionState::Idle,
        }
    }

    pub const fn with_state(mut self, state: UiShadcnSessionState) -> Self {
        self.state = state;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiShadcnSessionState {
    Idle,
    Running,
    Error,
}

impl UiShadcnSessionState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "run",
            Self::Error => "err",
        }
    }

    pub const fn accent(self) -> Color4 {
        match self {
            Self::Idle => palette::MUTED,
            Self::Running => palette::ACCENT,
            Self::Error => palette::DANGER,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnComposerNotice<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub icon: UiIcon,
}

impl<'a> UiShadcnComposerNotice<'a> {
    pub const fn new(title: &'a str, body: &'a str, icon: UiIcon) -> Self {
        Self { title, body, icon }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnActivity<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    pub icon: UiIcon,
}

impl<'a> UiShadcnActivity<'a> {
    pub const fn new(title: &'a str, detail: &'a str, icon: UiIcon) -> Self {
        Self {
            title,
            detail,
            icon,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiShadcnChatClientSpec<'a> {
    pub show_sidebar: bool,
    pub sidebar_title: &'a str,
    pub sidebar_detail: &'a str,
    pub sidebar_actions: &'a [UiShadcnChatClientAction<'a>],
    pub session_rows: &'a [UiShadcnSessionRow<'a>],
    pub activity: UiShadcnActivity<'a>,
    pub footer_lines: &'a [&'a str],
    pub header_title: &'a str,
    pub header_status: &'a str,
    pub header_badges: &'a [&'a str],
    pub header_action: Option<UiShadcnChatClientIconAction>,
    pub header_actions: &'a [UiShadcnChatClientIconAction],
    pub header_tone: UiShadcnStatusTone,
    pub activity_phase: u8,
    pub messages: &'a [UiShadcnConversationMessage<'a>],
    pub scroll_offset: f32,
    pub scroll_id: u32,
    pub input_label: &'a str,
    pub input_value: &'a str,
    pub input_id: u32,
    pub composer_action: Option<UiShadcnChatClientIconAction>,
    pub composer_notice: Option<UiShadcnComposerNotice<'a>>,
    pub composer_expanded: bool,
    pub send_label: &'a str,
    pub send_id: u32,
    pub busy: bool,
    pub hints: &'a [&'a str],
}

pub fn shadcn_button(
    label: &str,
    id: u32,
    variant: UiShadcnButtonVariant,
    size: UiShadcnButtonSize,
) -> UiNode {
    button(label, id, variant.button_style()).class(size.class_suffix())
}

pub fn shadcn_badge(label: &str, variant: UiShadcnBadgeVariant) -> UiNode {
    badge(label, variant.color())
}

pub fn shadcn_alert(title: &str, body: &str, icon_kind: UiIcon) -> UiNode {
    row("gap-3 items-start border rounded-lg p-3")
        .child(icon(icon_kind).class("w-5 h-5"))
        .child(column("gap-1 flex-1").child(text(title)).child(text(body)))
}

pub fn shadcn_accordion(items: &[(&str, &str)], base_id: u32) -> UiNode {
    let mut node = card("bg-panel border rounded-lg p-2 gap-1");
    for (index, (title, body)) in items.iter().enumerate() {
        node = node
            .child(
                row("items-center justify-between h-10")
                    .child(text(title))
                    .child(icon_button(UiIcon::ChevronRight, base_id + index as u32)),
            )
            .child(text(body));
        if index + 1 < items.len() {
            node = node.child(divider(""));
        }
    }
    node
}

pub fn shadcn_alert_dialog(title: &str, body: &str, icon: UiIcon) -> UiNode {
    dialog(title, body, icon).class("h-52")
}

pub fn shadcn_aspect_ratio(label: &str, icon_kind: UiIcon) -> UiNode {
    card("bg-panel border rounded-lg p-0 overflow-hidden").child(
        column("aspect-video bg-muted items-center justify-center")
            .child(icon(icon_kind).class("w-8 h-8"))
            .child(text(label)),
    )
}

pub fn shadcn_avatar(label: &str, color: Color4) -> UiNode {
    avatar_node(label, color)
}

pub fn shadcn_breadcrumb(labels: &[&str], selected: usize, base_id: u32) -> UiNode {
    breadcrumb(labels, selected, base_id)
}

pub fn shadcn_button_group(labels: &[&str], base_id: u32) -> UiNode {
    let mut node = row("gap-0 items-center h-10");
    for (index, label) in labels.iter().enumerate() {
        let class = match (index, labels.len().saturating_sub(1)) {
            (0, 0) => "h-9",
            (0, _) => "h-9 rounded-r-none",
            (i, last) if i == last => "h-9 rounded-l-none",
            _ => "h-9 rounded-none",
        };
        node =
            node.child(button(label, base_id + index as u32, ButtonStyle::Secondary).class(class));
    }
    node
}

pub fn shadcn_calendar(month: &str, days: &[&str], selected: usize, base_id: u32) -> UiNode {
    let mut grid_node = grid("grid-cols-7 gap-1", 7)
        .child(text("S"))
        .child(text("M"))
        .child(text("T"))
        .child(text("W"))
        .child(text("T"))
        .child(text("F"))
        .child(text("S"));
    for (index, day) in days.iter().enumerate() {
        let style = if index == selected {
            ButtonStyle::Secondary
        } else {
            ButtonStyle::Ghost
        };
        grid_node = grid_node.child(button(day, base_id + 2 + index as u32, style));
    }
    card("bg-panel border rounded-lg p-3 gap-3")
        .child(
            row("items-center justify-between h-8")
                .child(icon_button(UiIcon::ChevronRight, base_id).class("rotate-180"))
                .child(text(month))
                .child(icon_button(UiIcon::ChevronRight, base_id + 1)),
        )
        .child(grid_node)
}

pub fn shadcn_checkbox(label: &str, checked: bool, id: u32) -> UiNode {
    checkbox(label, checked, id)
}

pub fn shadcn_carousel(items: &[&str], base_id: u32) -> UiNode {
    let mut node = row("gap-3 items-center")
        .child(icon_button(UiIcon::ChevronRight, base_id).class("rotate-180"));
    for item in items {
        node = node.child(
            card("bg-panel border rounded-lg p-6 items-center justify-center").child(text(item)),
        );
    }
    node.child(icon_button(UiIcon::ChevronRight, base_id + 1))
}

pub fn shadcn_chart(title: &str, labels: &[&str], values: &[f32]) -> UiNode {
    bar_chart_labels(title, labels, values)
}

pub fn shadcn_collapsible(title: &str, rows: &[(&str, &str)], base_id: u32) -> UiNode {
    let mut node = card("bg-panel border rounded-lg p-3 gap-2").child(
        row("items-center justify-between h-9")
            .child(text(title))
            .child(icon_button(UiIcon::ChevronRight, base_id)),
    );
    for (index, (label, detail)) in rows.iter().enumerate() {
        node = node.child(list_row_node(label, detail, base_id + 1 + index as u32));
    }
    node
}

pub fn shadcn_combobox(
    label: &str,
    value: &str,
    placeholder: &str,
    options: &[&str],
    selected: usize,
    base_id: u32,
) -> UiNode {
    let mut node = column("gap-2")
        .child(select_node(label, value, base_id))
        .child(command_palette(placeholder, base_id + 1));
    for (index, option) in options.iter().enumerate() {
        node = node
            .child(menu_item_node(option, base_id + 2 + index as u32).selected(index == selected));
    }
    node
}

pub fn shadcn_card(title: &str, detail: &str) -> UiNode {
    card("bg-panel border rounded-lg p-4 gap-3").child(header(title).detail(detail))
}

pub fn shadcn_chat_message(role: UiShadcnChatRole, body: &str) -> UiNode {
    let (heading, detail) = shadcn_message_heading(role, body);
    if role == UiShadcnChatRole::Diff {
        return card("bg-panel border rounded-lg p-3 gap-2")
            .child(
                row("h-7 items-center gap-2")
                    .child(icon(role.icon()).class("w-4 h-4"))
                    .child(shadcn_badge(role.label(), role.badge_variant()).class("h-6"))
                    .child(shadcn_label(heading).class("h-5 text-muted-foreground")),
            )
            .child(shadcn_diff_body(detail));
    }
    if role.is_timeline_event() {
        return row("h-full gap-3 items-start border rounded-lg p-3 bg-bg")
            .child(icon(role.icon()).class("w-5 h-5"))
            .child(
                column("gap-1 flex-1")
                    .child(
                        row("h-6 gap-2 items-center")
                            .child(shadcn_badge(role.label(), role.badge_variant()).class("h-6"))
                            .child(shadcn_label(heading).class("h-5 text-muted-foreground")),
                    )
                    .child(shadcn_label(detail).class("h-full text-text")),
            );
    }
    card("bg-panel border rounded-lg p-3 gap-2")
        .child(
            row("h-7 items-center gap-2")
                .child(icon(role.icon()).class("w-4 h-4"))
                .child(shadcn_badge(role.label(), role.badge_variant()).class("h-6"))
                .child(shadcn_label(heading).class("h-5 text-muted-foreground")),
        )
        .child(shadcn_label(detail).class("h-full text-text"))
}

fn shadcn_message_heading(role: UiShadcnChatRole, body: &str) -> (&str, &str) {
    let Some((heading, detail)) = body.split_once('\n') else {
        return ("", body);
    };
    if heading.trim().is_empty() || detail.trim().is_empty() {
        return ("", body);
    }
    let known_heading = match role {
        UiShadcnChatRole::Reasoning => matches!(heading, "Thinking"),
        UiShadcnChatRole::Assistant => matches!(heading, "Response"),
        UiShadcnChatRole::Diff => matches!(heading, "Patch / tool input" | "Patch" | "Tool input"),
        UiShadcnChatRole::ToolRunning => matches!(heading, "Started"),
        UiShadcnChatRole::ToolSuccess => matches!(heading, "Completed"),
        UiShadcnChatRole::ToolError => matches!(heading, "Failed"),
        UiShadcnChatRole::User | UiShadcnChatRole::Error => false,
    };
    if known_heading {
        (heading, detail)
    } else {
        ("", body)
    }
}

#[cfg(feature = "fontdue-text")]
pub fn shadcn_chat_message_height_for_role(
    atlas: &FontAtlas,
    role: UiShadcnChatRole,
    body: &str,
    card_width: f32,
) -> f32 {
    let (_, detail) = shadcn_message_heading(role, body);
    let measured = shadcn_chat_message_height(atlas, detail, card_width);
    if role.is_timeline_event() {
        measured.max(62.0) - 12.0
    } else {
        measured
    }
}

#[cfg(feature = "fontdue-text")]
pub fn shadcn_chat_message_height(atlas: &FontAtlas, body: &str, card_width: f32) -> f32 {
    let body_width = (card_width - 32.0).max(1.0);
    58.0 + atlas.wrapped_line_count(body, body_width) as f32 * 22.0
}

pub fn shadcn_diff_body(body: &str) -> UiNode {
    let mut node = column("h-full gap-1");
    for line in body.lines().take(240) {
        let class = if line.starts_with('+') && !line.starts_with("+++") {
            "h-5 text-green"
        } else if line.starts_with('-') && !line.starts_with("---") {
            "h-5 text-danger"
        } else if line.starts_with("@@") || line.starts_with("***") {
            "h-5 text-muted-foreground"
        } else {
            "h-5 text-text"
        };
        node = node.child(shadcn_label(line).class(class));
    }
    if body.lines().count() > 240 {
        node =
            node.child(shadcn_label("[diff preview truncated]").class("h-5 text-muted-foreground"));
    }
    node
}

pub fn shadcn_conversation(
    messages: &[UiShadcnConversationMessage<'_>],
    scroll_offset: f32,
    scroll_id: u32,
) -> UiNode {
    let mut node = scroll_area("h-full p-4 gap-4", scroll_offset).scroll_id(scroll_id);
    for message in messages {
        node = node.child(
            shadcn_chat_message(message.role, message.body)
                .class(&format!("h-{}", (message.height / 4.0).max(11.0))),
        );
    }
    node
}

pub fn shadcn_prompt_composer(
    label: &str,
    value: &str,
    input_id: u32,
    action: Option<UiShadcnChatClientIconAction>,
    send_label: &str,
    send_id: u32,
    busy: bool,
    hints: &[&str],
) -> UiNode {
    let mut hint_row = row("gap-4 items-center h-5");
    for hint in hints {
        hint_row = hint_row.child(shadcn_label(hint).class("h-5 text-muted-foreground"));
    }

    let send = if send_label.is_empty() {
        icon_button(UiIcon::Send, send_id)
            .class("size-9")
            .disabled(busy)
    } else {
        shadcn_button(
            send_label,
            send_id,
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Sm,
        )
        .disabled(busy)
    };

    let mut input_row = row("h-full gap-2 items-end").child(
        shadcn_textarea(label, value)
            .hit_id(input_id)
            .class("h-full flex-1"),
    );
    if let Some(action) = action {
        input_row = input_row.child(
            icon_button(action.icon, action.id)
                .class("size-9")
                .active(action.active),
        );
    }
    input_row = input_row.child(send);

    let mut node = column("h-full gap-2").child(input_row);
    if !hints.is_empty() {
        node = node.child(hint_row);
    }
    node
}

pub fn shadcn_chat_client_shell(
    sidebar: Option<UiNode>,
    header: UiNode,
    conversation: UiNode,
    composer: UiNode,
    composer_expanded: bool,
) -> UiNode {
    let composer_class = if composer_expanded {
        "h-36 bg-bg border p-4"
    } else {
        "h-28 bg-bg border p-4"
    };
    let main = column("h-full flex-1 bg-bg")
        .child(column("h-11 bg-panel border").child(header))
        .child(column("flex-1 h-full").child(conversation))
        .child(column(composer_class).child(composer));

    let mut shell = row("h-full bg-bg");
    if let Some(sidebar) = sidebar {
        shell = shell.child(column("w-65 h-full bg-sidebar border").child(sidebar));
    }
    shell.child(main)
}

pub fn shadcn_chat_client(spec: UiShadcnChatClientSpec<'_>) -> UiNode {
    let sidebar = spec.show_sidebar.then(|| {
        let actions = spec
            .sidebar_actions
            .iter()
            .map(|action| (action.label, action.icon, action.id, action.variant))
            .collect::<Vec<_>>();
        let session_rows = spec
            .session_rows
            .iter()
            .map(|row| (row.title, row.detail, row.active, row.state))
            .collect::<Vec<_>>();
        shadcn_session_sidebar(
            spec.sidebar_title,
            spec.sidebar_detail,
            &actions,
            &session_rows,
            spec.activity.title,
            spec.activity.detail,
            spec.activity.icon,
            spec.footer_lines,
        )
    });
    let mut header_actions = Vec::new();
    if let Some(action) = spec.header_action {
        header_actions.push(action);
    }
    header_actions.extend_from_slice(spec.header_actions);

    let mut composer = shadcn_prompt_composer(
        spec.input_label,
        spec.input_value,
        spec.input_id,
        spec.composer_action,
        spec.send_label,
        spec.send_id,
        spec.busy,
        spec.hints,
    );
    if let Some(notice) = spec.composer_notice {
        composer = column("h-full gap-2")
            .child(shadcn_alert(notice.title, notice.body, notice.icon).class("h-14"))
            .child(composer.class("flex-1"));
    }

    shadcn_chat_client_shell(
        sidebar,
        shadcn_status_header(
            spec.header_title,
            spec.header_status,
            spec.header_badges,
            &header_actions,
            spec.header_tone,
            spec.activity_phase,
        ),
        shadcn_conversation(spec.messages, spec.scroll_offset, spec.scroll_id),
        composer,
        spec.composer_expanded || spec.composer_notice.is_some(),
    )
}

pub fn shadcn_status_header(
    title: &str,
    status: &str,
    badges: &[&str],
    actions: &[UiShadcnChatClientIconAction],
    tone: UiShadcnStatusTone,
    activity_phase: u8,
) -> UiNode {
    let mut badge_row = row("gap-2 items-center");
    for badge_label in badges {
        badge_row = badge_row
            .child(shadcn_badge(badge_label, UiShadcnBadgeVariant::Secondary).class("h-7"));
    }

    let mut status_row = row("gap-2 items-center flex-1")
        .child(shadcn_badge("", tone.badge_variant()).class("size-3"))
        .child(shadcn_label(title).class("h-5 text-text"))
        .child(shadcn_label(status).class("h-5 text-muted-foreground flex-1"));
    if tone == UiShadcnStatusTone::Active {
        let active = activity_phase as usize % 4;
        let mut shine = row("gap-1 items-center h-2");
        for index in 0..4 {
            let variant = if index == active {
                UiShadcnBadgeVariant::Default
            } else {
                UiShadcnBadgeVariant::Secondary
            };
            shine = shine.child(shadcn_badge("", variant).class("h-1 w-5"));
        }
        status_row = status_row.child(shine);
    }

    let mut header = row("h-full items-center justify-between gap-3")
        .child(status_row)
        .child(badge_row);
    for action in actions {
        header = header.child(
            icon_button(action.icon, action.id)
                .class("h-8 w-8")
                .active(action.active),
        );
    }
    header
}

pub fn shadcn_session_sidebar(
    title: &str,
    _detail: &str,
    actions: &[(&str, Option<UiIcon>, u32, UiShadcnButtonVariant)],
    session_rows: &[(&str, &str, bool, UiShadcnSessionState)],
    activity_title: &str,
    activity_detail: &str,
    activity_icon: UiIcon,
    footer_lines: &[&str],
) -> UiNode {
    let mut action_row = row("gap-2 items-center h-9");
    for (label, icon, id, variant) in actions {
        let action = if let Some(icon) = icon {
            icon_button(*icon, *id).class("h-9 w-9")
        } else {
            shadcn_button(label, *id, *variant, UiShadcnButtonSize::Sm)
        };
        action_row = action_row.child(action);
    }

    let mut session_group =
        column("gap-2").child(shadcn_label("Session").class("h-5 text-muted-foreground"));
    for (index, (title, detail, active, state)) in session_rows.iter().enumerate() {
        let accent = if *active {
            palette::ACCENT
        } else {
            state.accent()
        };
        session_group = session_group.child(
            menu_item_node(title, 88_000 + index as u32)
                .detail(detail)
                .badge_text(state.label())
                .accent(accent)
                .selected(*active)
                .class("h-14"),
        );
    }

    let mut footer = column("gap-1");
    for line in footer_lines {
        footer = footer.child(shadcn_label(line).class("h-5 text-muted-foreground"));
    }

    column("h-full gap-4 p-4")
        .child(shadcn_label(title).class("h-5 text-text"))
        .child(action_row)
        .child(session_group)
        .child(shadcn_alert(activity_title, activity_detail, activity_icon).class("h-14"))
        .child(spacer("flex-1"))
        .child(footer)
}

pub fn shadcn_command(placeholder: &str, id: u32) -> UiNode {
    command_palette(placeholder, id)
}

pub fn shadcn_context_menu(
    title: &str,
    detail: &str,
    items: &[(&str, &str, bool)],
    base_id: u32,
) -> UiNode {
    let mut node = card("bg-panel border rounded-lg p-3 gap-2").child(header(title).detail(detail));
    for (index, (label, shortcut, selected)) in items.iter().enumerate() {
        let mut item = menu_item_node(label, base_id + index as u32).selected(*selected);
        if !shortcut.is_empty() {
            item = item.detail(shortcut);
        }
        node = node.child(item);
    }
    node
}

pub fn shadcn_data_table(headers: &[&str], rows: &[&[&str]], id_base: u32) -> UiNode {
    table_labels(headers, rows, id_base)
}

pub fn shadcn_date_picker(
    label: &str,
    month: &str,
    days: &[&str],
    selected: usize,
    base_id: u32,
) -> UiNode {
    let mut day_row = row("gap-1");
    for (index, day) in days.iter().enumerate() {
        let style = if index == selected {
            ButtonStyle::Secondary
        } else {
            ButtonStyle::Ghost
        };
        day_row = day_row.child(button(day, base_id + 1 + index as u32, style));
    }
    column("gap-2")
        .child(button(label, base_id, ButtonStyle::Secondary).class("h-9"))
        .child(
            card("bg-panel border rounded-lg p-3 gap-2")
                .child(text(month))
                .child(day_row),
        )
}

pub fn shadcn_dialog(title: &str, body: &str, icon: UiIcon) -> UiNode {
    dialog(title, body, icon)
}

pub fn shadcn_direction(ltr: &str, rtl: &str) -> UiNode {
    column("gap-2")
        .child(
            row("gap-2 items-center")
                .child(badge("LTR", palette::ACCENT))
                .child(text(ltr)),
        )
        .child(
            row("gap-2 items-center justify-end")
                .child(text(rtl))
                .child(badge("RTL", palette::MUTED)),
        )
}

pub fn shadcn_drawer(
    title: &str,
    detail: &str,
    slider_label: &str,
    value: f32,
    base_id: u32,
) -> UiNode {
    card("bg-panel border rounded-t-xl p-4 gap-3")
        .child(header(title).detail(detail))
        .child(slider_node(slider_label, value, base_id))
        .child(row("gap-2").child(button("Submit", base_id + 1, ButtonStyle::Primary)))
}

pub fn shadcn_dropdown_menu(items: &[(&str, &str, bool)], base_id: u32) -> UiNode {
    let mut node = column("gap-1");
    for (index, (label, shortcut, selected)) in items.iter().enumerate() {
        let mut item = menu_item_node(label, base_id + index as u32).selected(*selected);
        if !shortcut.is_empty() {
            item = item.detail(shortcut);
        }
        node = node.child(item);
    }
    node
}

pub fn shadcn_empty(title: &str, body: &str, icon: UiIcon) -> UiNode {
    empty_state(title, body, icon)
}

pub fn shadcn_field(label: &str, value: &str) -> UiNode {
    field_node(label, value)
}

pub fn shadcn_hover_card(label: &str, detail: &str, body: &str, color: Color4) -> UiNode {
    column("gap-2")
        .child(
            row("gap-3 items-center")
                .child(avatar_node(label, color))
                .child(column("gap-1").child(text(label)).child(text(detail))),
        )
        .child(text(body))
}

pub fn shadcn_input(label: &str, value: &str) -> UiNode {
    field_node(label, value)
}

pub fn shadcn_input_group(label: &str, value: &str, button_label: &str, id: u32) -> UiNode {
    row("gap-2 h-12 items-center")
        .child(field_node(label, value).class("flex-1"))
        .child(button(button_label, id, ButtonStyle::Secondary).class("h-9 w-20"))
}

pub fn shadcn_input_otp(values: &[&str], focused_index: usize) -> UiNode {
    let mut node = row("gap-2 items-center h-12");
    for (index, value) in values.iter().enumerate() {
        if *value == "-" {
            node = node.child(text("-"));
            continue;
        }
        node = node.child(
            field_node("", value)
                .class("w-10")
                .focused(index == focused_index),
        );
    }
    node
}

pub fn shadcn_item(title: &str, detail: &str, id: u32, accent: Color4) -> UiNode {
    list_row_node(title, detail, id).accent(accent)
}

pub fn shadcn_kbd(keys: &[&str], label: &str) -> UiNode {
    let mut node = row("gap-2 items-center h-10");
    for key in keys {
        node = node.child(badge(key, palette::MUTED));
    }
    node.child(text(label))
}

pub fn shadcn_label(value: &str) -> UiNode {
    text(value)
}

pub fn shadcn_menubar(items: &[&str], selected: usize, base_id: u32) -> UiNode {
    let mut node = row("gap-1 items-center border rounded-lg p-1");
    for (index, item) in items.iter().enumerate() {
        let style = if index == selected {
            ButtonStyle::Secondary
        } else {
            ButtonStyle::Ghost
        };
        node = node.child(button(item, base_id + index as u32, style));
    }
    node
}

pub fn shadcn_native_select(label: &str, value: &str, id: u32) -> UiNode {
    select_node(label, value, id)
}

pub fn shadcn_navigation_menu(
    tabs: &[&str],
    selected: usize,
    title: &str,
    detail: &str,
    row_title: &str,
    row_detail: &str,
    base_id: u32,
) -> UiNode {
    let mut nav = row("gap-1 items-center");
    for (index, tab) in tabs.iter().enumerate() {
        let style = if index == selected {
            ButtonStyle::Secondary
        } else {
            ButtonStyle::Ghost
        };
        nav = nav.child(button(tab, base_id + index as u32, style));
    }
    column("gap-2").child(nav).child(
        card("bg-panel border rounded-lg p-3 gap-2")
            .child(header(title).detail(detail))
            .child(list_row_node(
                row_title,
                row_detail,
                base_id + tabs.len() as u32,
            )),
    )
}

pub fn shadcn_pagination(pages: &[&str], selected: usize, base_id: u32) -> UiNode {
    let mut node = row("gap-1 items-center h-10")
        .child(button("Previous", base_id, ButtonStyle::Ghost).disabled(true));
    for (index, page) in pages.iter().enumerate() {
        let style = if index == selected {
            ButtonStyle::Secondary
        } else {
            ButtonStyle::Ghost
        };
        node = node.child(button(page, base_id + 1 + index as u32, style));
    }
    node.child(button(
        "Next",
        base_id + 1 + pages.len() as u32,
        ButtonStyle::Ghost,
    ))
}

pub fn shadcn_popover(
    button_label: &str,
    title: &str,
    detail: &str,
    field_label: &str,
    field_value: &str,
    base_id: u32,
) -> UiNode {
    column("gap-2")
        .child(button(button_label, base_id, ButtonStyle::Secondary).class("h-9"))
        .child(
            card("bg-panel border rounded-lg p-3 gap-2")
                .child(header(title).detail(detail))
                .child(field_node(field_label, field_value)),
        )
}

pub fn shadcn_progress(value: f32) -> UiNode {
    progress_bar_node(value, palette::ACCENT)
}

pub fn shadcn_radio_group(options: &[(&str, bool)], base_id: u32) -> UiNode {
    let mut node = column("gap-2");
    for (index, (label, selected)) in options.iter().enumerate() {
        node = node.child(radio(label, *selected, base_id + index as u32));
    }
    node
}

pub fn shadcn_resizable(labels: &[&str]) -> UiNode {
    let first = labels.first().copied().unwrap_or("One");
    let second = labels.get(1).copied().unwrap_or("Two");
    let third = labels.get(2).copied().unwrap_or("Three");
    row("gap-1 h-28")
        .child(card("bg-panel border rounded-lg p-3 flex-1").child(text(first)))
        .child(divider("w-1"))
        .child(
            column("gap-1 flex-1")
                .child(card("bg-panel border rounded-lg p-3 flex-1").child(text(second)))
                .child(card("bg-panel border rounded-lg p-3 flex-1").child(text(third))),
        )
}

pub fn shadcn_scroll_area(rows: &[(&str, &str)], base_id: u32) -> UiNode {
    let mut node = scroll_area("h-32 border rounded-lg p-2", 0.0);
    for (index, (title, detail)) in rows.iter().enumerate() {
        node = node.child(list_row_node(title, detail, base_id + index as u32));
    }
    node
}

pub fn shadcn_select(label: &str, value: &str, id: u32) -> UiNode {
    select_node(label, value, id)
}

pub fn shadcn_separator() -> UiNode {
    divider("")
}

pub fn shadcn_skeleton() -> UiNode {
    skeleton()
}

pub fn shadcn_sheet(
    title: &str,
    detail: &str,
    field_label: &str,
    field_value: &str,
    button_label: &str,
    base_id: u32,
) -> UiNode {
    card("bg-panel border rounded-lg p-4 gap-3")
        .child(header(title).detail(detail))
        .child(field_node(field_label, field_value))
        .child(row("gap-2").child(button(button_label, base_id, ButtonStyle::Primary)))
}

pub fn shadcn_sidebar(
    title: &str,
    detail: &str,
    items: &[&str],
    selected: usize,
    main_title: &str,
    main_detail: &str,
    base_id: u32,
) -> UiNode {
    let mut side =
        card("bg-sidebar border rounded-lg p-2 gap-1 w-44").child(header(title).detail(detail));
    for (index, item) in items.iter().enumerate() {
        side = side.child(menu_item_node(item, base_id + index as u32).selected(index == selected));
    }
    row("gap-3 h-44").child(side).child(
        card("bg-panel border rounded-lg p-4 flex-1").child(header(main_title).detail(main_detail)),
    )
}

pub fn shadcn_slider(label: &str, value: f32, id: u32) -> UiNode {
    slider_node(label, value, id)
}

pub fn shadcn_sonner(messages: &[(&str, UiIcon, Color4)]) -> UiNode {
    let mut node = column("gap-2");
    for (message, icon, accent) in messages {
        node = node.child(toast(message, *icon, *accent));
    }
    node
}

pub fn shadcn_switch(checked: bool, id: u32) -> UiNode {
    toggle_node(checked, id)
}

pub fn shadcn_table(headers: &[&str], rows: &[&[&str]], id_base: u32) -> UiNode {
    table_labels(headers, rows, id_base)
}

pub fn shadcn_tabs(labels: &[&str], selected: usize, base_id: u32) -> UiNode {
    tab_labels(labels, selected, base_id)
}

pub fn shadcn_textarea(label: &str, value: &str) -> UiNode {
    text_area_node(label, value)
}

pub fn shadcn_toast(message: &str, icon: UiIcon, accent: Color4) -> UiNode {
    toast(message, icon, accent)
}

pub fn shadcn_toggle(pressed: bool, id: u32) -> UiNode {
    toggle_node(pressed, id)
}

pub fn shadcn_toggle_group(labels: &[&str], selected: usize, base_id: u32) -> UiNode {
    let mut node = row("gap-1 items-center h-10");
    for (index, label) in labels.iter().enumerate() {
        let style = if index == selected {
            ButtonStyle::Secondary
        } else {
            ButtonStyle::Ghost
        };
        node = node.child(button(label, base_id + index as u32, style));
    }
    node
}

pub fn shadcn_tooltip(text: &str) -> UiNode {
    tooltip(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_button_variants_map_to_native_button_styles() {
        assert_eq!(
            UiShadcnButtonVariant::Default.button_style(),
            ButtonStyle::Primary
        );
        assert_eq!(
            UiShadcnButtonVariant::Destructive.button_style(),
            ButtonStyle::Danger
        );
        assert_eq!(
            UiShadcnButtonVariant::Secondary.button_style(),
            ButtonStyle::Secondary
        );
        assert_eq!(
            UiShadcnButtonVariant::Ghost.button_style(),
            ButtonStyle::Ghost
        );
    }

    #[test]
    fn exact_badge_variants_map_to_theme_tokens() {
        assert_eq!(UiShadcnBadgeVariant::Default.color(), palette::ACCENT);
        assert_eq!(UiShadcnBadgeVariant::Secondary.color(), palette::MUTED);
        assert_eq!(UiShadcnBadgeVariant::Destructive.color(), palette::DANGER);
        assert_eq!(UiShadcnBadgeVariant::Outline.color(), palette::BORDER);
    }

    #[test]
    fn exact_builders_return_expected_node_kinds() {
        assert!(matches!(
            shadcn_button(
                "Save",
                7,
                UiShadcnButtonVariant::Default,
                UiShadcnButtonSize::Default
            )
            .kind,
            UiNodeKind::Button { .. }
        ));
        assert!(matches!(
            shadcn_badge("Active", UiShadcnBadgeVariant::Default).kind,
            UiNodeKind::Badge { .. }
        ));
        assert!(matches!(
            shadcn_alert("Heads up", "Body", UiIcon::Warning).kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            shadcn_alert_dialog("Confirm", "Body", UiIcon::Warning).kind,
            UiNodeKind::Dialog { .. }
        ));
        assert!(matches!(
            shadcn_aspect_ratio("16:9", UiIcon::Eye).kind,
            UiNodeKind::Card
        ));
        assert!(matches!(
            shadcn_avatar("ER", palette::ACCENT).kind,
            UiNodeKind::Avatar { .. }
        ));
        assert!(matches!(
            shadcn_breadcrumb(&["Docs", "Components"], 1, 1).kind,
            UiNodeKind::Breadcrumb { .. }
        ));
        assert!(matches!(
            shadcn_checkbox("Accept", true, 2).kind,
            UiNodeKind::Checkbox { .. }
        ));
        assert!(matches!(
            shadcn_card("Title", "Detail").kind,
            UiNodeKind::Card
        ));
        assert!(matches!(
            shadcn_chat_message(UiShadcnChatRole::Assistant, "Hello\nworld").kind,
            UiNodeKind::Card
        ));
        assert!(matches!(
            shadcn_conversation(
                &[UiShadcnConversationMessage::new(
                    UiShadcnChatRole::Assistant,
                    "Hello",
                    88.0
                )],
                1.0,
                77
            )
            .kind,
            UiNodeKind::ScrollArea { .. }
        ));
        assert!(matches!(
            shadcn_status_header(
                "Codex",
                "Ready",
                &["gpt-5.5", "ready"],
                &[],
                UiShadcnStatusTone::Success,
                0
            )
            .kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            shadcn_session_sidebar(
                "edgerun codex",
                "workspace",
                &[
                    (
                        "",
                        Some(UiIcon::MessagePlus),
                        1,
                        UiShadcnButtonVariant::Default
                    ),
                    ("", Some(UiIcon::Trash), 2, UiShadcnButtonVariant::Secondary)
                ],
                &[
                    ("1 turn", "", true, UiShadcnSessionState::Idle),
                    ("0 tools", "", false, UiShadcnSessionState::Running),
                ],
                "Ready",
                "idle",
                UiIcon::Check,
                &["model gpt-5.5"]
            )
            .kind,
            UiNodeKind::Column
        ));
        assert!(matches!(
            shadcn_prompt_composer("", "Hi", 11, None, "Send", 12, false, &["Enter sends"]).kind,
            UiNodeKind::Column
        ));
        let composer = shadcn_prompt_composer(
            "",
            "Hi",
            11,
            Some(UiShadcnChatClientIconAction::new(UiIcon::Trash, 13, false)),
            "",
            12,
            false,
            &[],
        );
        let input_row = composer.children.first().expect("composer input row");
        assert!(matches!(input_row.kind, UiNodeKind::Row));
        assert!(matches!(
            input_row.children.first().map(|child| &child.kind),
            Some(UiNodeKind::TextArea { .. })
        ));
        assert!(matches!(
            input_row.children.get(1).map(|child| &child.kind),
            Some(UiNodeKind::IconButton {
                icon: UiIcon::Trash,
                ..
            })
        ));
        assert!(matches!(
            input_row.children.get(2).map(|child| &child.kind),
            Some(UiNodeKind::IconButton {
                icon: UiIcon::Send,
                ..
            })
        ));
        assert!(matches!(
            shadcn_chat_client_shell(
                None,
                shadcn_status_header(
                    "Codex",
                    "Ready",
                    &["ready"],
                    &[],
                    UiShadcnStatusTone::Success,
                    0
                ),
                shadcn_conversation(&[], 1.0, 7),
                shadcn_prompt_composer("", "Hi", 11, None, "Send", 12, false, &[]),
                false
            )
            .kind,
            UiNodeKind::Row
        ));
        let compact_shell = shadcn_chat_client_shell(
            None,
            shadcn_status_header("Codex", "Ready", &[], &[], UiShadcnStatusTone::Success, 0),
            shadcn_conversation(&[], 1.0, 7),
            shadcn_prompt_composer("", "Hi", 11, None, "Send", 12, false, &[]),
            false,
        );
        let expanded_shell = shadcn_chat_client_shell(
            None,
            shadcn_status_header("Codex", "Ready", &[], &[], UiShadcnStatusTone::Success, 0),
            shadcn_conversation(&[], 1.0, 7),
            shadcn_prompt_composer("", "Hi", 11, None, "Send", 12, false, &[]),
            true,
        );
        assert_eq!(
            compact_shell.children[0].children[2].style.height,
            Some(112.0)
        );
        assert_eq!(
            expanded_shell.children[0].children[2].style.height,
            Some(144.0)
        );
        assert!(matches!(
            shadcn_chat_client(UiShadcnChatClientSpec {
                show_sidebar: true,
                sidebar_title: "edgerun codex",
                sidebar_detail: "workspace",
                sidebar_actions: &[UiShadcnChatClientAction::icon(
                    UiIcon::MessagePlus,
                    1,
                    UiShadcnButtonVariant::Default
                )],
                session_rows: &[UiShadcnSessionRow::new("1 turn", "", true)],
                activity: UiShadcnActivity::new("Ready", "idle", UiIcon::Check),
                footer_lines: &["model gpt-5.5"],
                header_title: "Codex",
                header_status: "Ready",
                header_badges: &["ready"],
                header_action: None,
                header_actions: &[],
                header_tone: UiShadcnStatusTone::Success,
                activity_phase: 0,
                messages: &[UiShadcnConversationMessage::new(
                    UiShadcnChatRole::Assistant,
                    "Hello",
                    88.0
                )],
                scroll_offset: 1.0,
                scroll_id: 77,
                input_label: "",
                input_value: "Hi",
                input_id: 11,
                composer_action: None,
                composer_notice: None,
                composer_expanded: false,
                send_label: "Send",
                send_id: 12,
                busy: false,
                hints: &["Enter sends"],
            })
            .kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            shadcn_command("Search...", 7).kind,
            UiNodeKind::CommandPalette { .. }
        ));
        assert!(matches!(
            shadcn_dialog("Edit", "Body", UiIcon::Settings).kind,
            UiNodeKind::Dialog { .. }
        ));
        assert!(matches!(
            shadcn_empty("Empty", "Nothing here", UiIcon::Search).kind,
            UiNodeKind::EmptyState { .. }
        ));
        assert!(matches!(
            shadcn_field("Email", "a@b.test").kind,
            UiNodeKind::Field { .. }
        ));
        assert!(matches!(
            shadcn_hover_card("ER", "UI", "Body", palette::ACCENT).kind,
            UiNodeKind::Column
        ));
        assert!(matches!(
            shadcn_input("Email", "a@b.test").kind,
            UiNodeKind::Field { .. }
        ));
        assert!(matches!(
            shadcn_input_group("URL", "https://example.com", "Copy", 6).kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            shadcn_input_otp(&["1", "2", "3", "-", "", ""], 4).kind,
            UiNodeKind::Row
        ));
        assert!(matches!(shadcn_label("Email").kind, UiNodeKind::Text(_)));
        assert!(matches!(
            shadcn_progress(0.5).kind,
            UiNodeKind::ProgressBar { .. }
        ));
        assert!(matches!(
            shadcn_radio_group(&[("Default", true)], 3).kind,
            UiNodeKind::Column
        ));
        assert!(matches!(
            shadcn_select("Framework", "Next.js", 8).kind,
            UiNodeKind::Select { .. }
        ));
        assert!(matches!(shadcn_separator().kind, UiNodeKind::Divider));
        assert!(matches!(shadcn_skeleton().kind, UiNodeKind::Skeleton));
        assert!(matches!(
            shadcn_slider("Volume", 0.42, 9).kind,
            UiNodeKind::Slider { .. }
        ));
        assert!(matches!(
            shadcn_sonner(&[("Saved", UiIcon::Check, palette::GREEN)]).kind,
            UiNodeKind::Column
        ));
        assert!(matches!(
            shadcn_switch(true, 9).kind,
            UiNodeKind::Toggle { .. }
        ));
        assert!(matches!(
            shadcn_table(&["A"], &[&["B"]], 4).kind,
            UiNodeKind::Table { .. }
        ));
        assert!(matches!(
            shadcn_tabs(&["A", "B"], 0, 5).kind,
            UiNodeKind::Tabs { .. }
        ));
        assert!(matches!(
            shadcn_textarea("Message", "Hi").kind,
            UiNodeKind::TextArea { .. }
        ));
        assert!(matches!(
            shadcn_chat_message(UiShadcnChatRole::ToolRunning, "Started\nshell").kind,
            UiNodeKind::Row
        ));
        assert!(matches!(
            shadcn_chat_message(UiShadcnChatRole::Assistant, "Response\nDone").kind,
            UiNodeKind::Card { .. }
        ));
        assert_eq!(
            shadcn_message_heading(UiShadcnChatRole::Reasoning, "Thinking\nchecking files"),
            ("Thinking", "checking files")
        );
        assert_eq!(
            shadcn_message_heading(UiShadcnChatRole::Assistant, "plain body"),
            ("", "plain body")
        );
        assert_eq!(
            shadcn_message_heading(UiShadcnChatRole::Assistant, "Summary\n- item"),
            ("", "Summary\n- item")
        );
        assert_eq!(
            shadcn_message_heading(UiShadcnChatRole::User, "Response\nkeep literal"),
            ("", "Response\nkeep literal")
        );
        assert!(matches!(
            shadcn_toast("Saved", UiIcon::Check, palette::GREEN).kind,
            UiNodeKind::Toast { .. }
        ));
        assert!(matches!(
            shadcn_toggle(true, 10).kind,
            UiNodeKind::Toggle { .. }
        ));
        assert!(matches!(
            shadcn_tooltip("Help").kind,
            UiNodeKind::Tooltip { .. }
        ));
    }

    #[test]
    fn shadcn_chat_client_renders_semantic_hits() {
        let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut ui = UiPainter::new(&mut scene);
        shadcn_chat_client(UiShadcnChatClientSpec {
            show_sidebar: true,
            sidebar_title: "edgerun codex",
            sidebar_detail: "workspace",
            sidebar_actions: &[UiShadcnChatClientAction::icon(
                UiIcon::MessagePlus,
                1,
                UiShadcnButtonVariant::Default,
            )],
            session_rows: &[UiShadcnSessionRow::new("1 turn", "", true)],
            activity: UiShadcnActivity::new("Ready", "idle", UiIcon::Check),
            footer_lines: &["model gpt-5.5"],
            header_title: "Codex",
            header_status: "Ready",
            header_badges: &["ready"],
            header_action: Some(UiShadcnChatClientIconAction::new(UiIcon::Trash, 2, true)),
            header_actions: &[UiShadcnChatClientIconAction::new(
                UiIcon::Settings,
                3,
                false,
            )],
            header_tone: UiShadcnStatusTone::Success,
            activity_phase: 0,
            messages: &[UiShadcnConversationMessage::new(
                UiShadcnChatRole::Assistant,
                "Hello",
                88.0,
            )],
            scroll_offset: 1.0,
            scroll_id: 77,
            input_label: "",
            input_value: "Hi",
            input_id: 11,
            composer_action: None,
            composer_notice: Some(UiShadcnComposerNotice::new(
                "Clear transcript?",
                "Activate clear again.",
                UiIcon::Warning,
            )),
            composer_expanded: false,
            send_label: "Send",
            send_id: 12,
            busy: false,
            hints: &["Enter sends"],
        })
        .render(&mut ui, UiRect::new(0.0, 0.0, 1120.0, 720.0));

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 1),
            "scene hits: {:?}",
            scene.hits()
        );
        assert!(scene
            .hits()
            .iter()
            .any(|hit| hit.kind == HitKind::Button && hit.id == 2));
        assert!(scene
            .hits()
            .iter()
            .any(|hit| hit.kind == HitKind::Button && hit.id == 3));
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::TextArea && hit.id == 11),
            "scene hits: {:?}",
            scene.hits()
        );
        assert!(scene
            .hits()
            .iter()
            .any(|hit| hit.kind == HitKind::Button && hit.id == 12));
    }

    #[test]
    fn shadcn_diff_body_renders_line_nodes() {
        let node = shadcn_diff_body("@@ hunk\n-old\n+new\n context");

        match node.kind {
            UiNodeKind::Column => assert_eq!(node.children.len(), 4),
            other => panic!("expected diff column, got {other:?}"),
        }
    }

    #[cfg(feature = "fontdue-text")]
    #[test]
    fn shadcn_chat_message_height_compacts_timeline_events() {
        let atlas = FontAtlas::load_inter(18.0).expect("font atlas");
        let body = "Started\nshell";
        let card_height =
            shadcn_chat_message_height_for_role(&atlas, UiShadcnChatRole::Assistant, body, 640.0);
        let timeline_height =
            shadcn_chat_message_height_for_role(&atlas, UiShadcnChatRole::ToolRunning, body, 640.0);

        assert!(timeline_height < card_height);
        assert!(timeline_height >= 50.0);
    }
}
