use super::*;

pub fn build_unified_chat_shell(scene: &mut GpuScene, width: f32, height: f32) {
    let state = UnifiedChatState::empty();
    build_unified_chat_shell_impl(
        scene,
        width,
        height,
        &state,
        None,
        #[cfg(feature = "fontdue-text")]
        None,
    );
}

#[cfg(feature = "fontdue-text")]
pub fn build_unified_chat_shell_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    state: &UnifiedChatState<'_>,
) {
    build_unified_chat_shell_impl(scene, width, height, state, None, Some(atlas));
}

#[cfg(feature = "fontdue-text")]
pub fn build_unified_chat_shell_with_font_and_runtime(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    state: &UnifiedChatState<'_>,
    runtime: &UiRuntimeState,
) {
    build_unified_chat_shell_impl(scene, width, height, state, Some(runtime), Some(atlas));
}

#[cfg(feature = "fontdue-text")]
pub fn build_edgerun_workspace_shell_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    workspace: &mut UiWorkspace,
    chat_state: &UnifiedChatState<'_>,
) {
    scene.clear = palette::BG;
    scene.clear_rects();
    let mut ui = UiPainter {
        scene,
        atlas: Some(atlas),
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    ui.fill_rect(
        UiRect::new(0.0, 0.0, width.max(360.0), height.max(320.0)),
        0.0,
        palette::BG,
    );
    workspace.render(
        &mut ui,
        UiRect::new(8.0, 8.0, (width - 16.0).max(0.0), (height - 16.0).max(0.0)),
        |ui, bounds, app| match app.kind {
            UiAppKind::Chat => render_workspace_chat_app(ui, bounds, app, chat_state),
            UiAppKind::TrustManager => render_trust_manager_app(ui, bounds, app),
            UiAppKind::Storage => render_storage_app(ui, bounds, app),
            UiAppKind::LockScreen => render_lock_screen_app(ui, bounds, app),
            UiAppKind::CapabilityRequest => render_capability_request_app(ui, bounds, app),
            UiAppKind::ComponentGallery => render_component_gallery_app(ui, bounds, app),
            UiAppKind::Generic => render_generic_workspace_app(ui, bounds, app),
        },
    );
}

#[cfg(feature = "fontdue-text")]
pub fn build_edgerun_workspace_with_shell_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    workspace: &mut UiWorkspace,
    shell: &mut UiShellState,
    chat_state: &UnifiedChatState<'_>,
) {
    build_edgerun_workspace_shell_with_font(scene, atlas, width, height, workspace, chat_state);
    let mut ui = UiPainter {
        scene,
        atlas: Some(atlas),
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    render_edgerun_shell_overlay(&mut ui, UiRect::new(0.0, 0.0, width, height), shell);
}

#[cfg(feature = "fontdue-text")]
pub fn build_edgerun_fullscreen_app_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    app: &mut UiAppSurface,
    chat_state: &UnifiedChatState<'_>,
) {
    scene.clear = palette::BG;
    scene.clear_rects();
    app.full_screen = true;
    app.bounds = None;
    let mut ui = UiPainter {
        scene,
        atlas: Some(atlas),
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    let bounds = UiRect::new(0.0, 0.0, width.max(360.0), height.max(320.0));
    ui.fill_rect(bounds, 0.0, palette::BG);
    let clipped = ui
        .scene
        .push_clip(GpuClip::new(bounds.x, bounds.y, bounds.w, bounds.h));
    if clipped {
        app.bounds = Some(bounds);
        match app.kind {
            UiAppKind::Chat => render_workspace_chat_app(&mut ui, bounds, app, chat_state),
            UiAppKind::TrustManager => render_trust_manager_app(&mut ui, bounds, app),
            UiAppKind::Storage => render_storage_app(&mut ui, bounds, app),
            UiAppKind::LockScreen => render_lock_screen_app(&mut ui, bounds, app),
            UiAppKind::CapabilityRequest => render_capability_request_app(&mut ui, bounds, app),
            UiAppKind::ComponentGallery => render_component_gallery_app(&mut ui, bounds, app),
            UiAppKind::Generic => render_generic_workspace_app(&mut ui, bounds, app),
        }
        ui.scene.pop_clip();
    }
}

#[cfg(feature = "fontdue-text")]
pub fn build_edgerun_shell_overlay_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    shell: &mut UiShellState,
) {
    scene.clear = Color4::rgba(0.0, 0.0, 0.0, 0.0);
    scene.clear_rects();
    let mut ui = UiPainter {
        scene,
        atlas: Some(atlas),
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    render_edgerun_shell_overlay(&mut ui, UiRect::new(0.0, 0.0, width, height), shell);
}

fn build_unified_chat_shell_impl(
    scene: &mut GpuScene,
    width: f32,
    height: f32,
    state: &UnifiedChatState<'_>,
    runtime: Option<&UiRuntimeState>,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) {
    scene.clear = palette::BG;
    scene.clear_rects();
    let mut ui = UiPainter {
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    let m = ChatShellMetrics::default();
    let w = width.max(360.0);
    let h = height.max(320.0);
    let sidebar_w = if w < 760.0 { 0.0 } else { m.sidebar_w + 28.0 };
    let main_x = sidebar_w;
    let main_w = w - sidebar_w;

    if sidebar_w > 0.0 {
        ui.panel(0.0, 0.0, sidebar_w, h, 0.0, palette::SIDEBAR);
        ui.scene.push_rect(GpuRect::fill(
            0.0,
            0.0,
            sidebar_w,
            4.0,
            0.0,
            palette::ACCENT,
        ));
        ui.bounded_label(
            24.0,
            20.0,
            (sidebar_w - 138.0).max(0.0),
            state.title,
            3.0,
            palette::TEXT,
        );
        ui.bounded_label(
            24.0,
            50.0,
            (sidebar_w - 48.0).max(0.0),
            state.subtitle,
            2.0,
            palette::MUTED,
        );
        ui.pill(
            sidebar_w - 104.0,
            22.0,
            78.0,
            if state.connected { "relay" } else { "local" },
            if state.connected {
                palette::GREEN
            } else {
                palette::AMBER
            },
        );
        ui.scene.push_rect(GpuRect::fill(
            sidebar_w - 1.0,
            0.0,
            1.0,
            h,
            0.0,
            palette::BORDER,
        ));

        for (index, contact) in state.contacts.iter().enumerate() {
            let y = 88.0 + index as f32 * 68.0;
            let selected = index == state.selected_contact;
            ui.contact_row(16.0, y, sidebar_w - 32.0, contact, selected, index as u32);
        }
    }

    ui.panel(main_x, 0.0, main_w, m.topbar_h, 0.0, palette::TOPBAR);
    let active = state
        .contacts
        .get(state.selected_contact)
        .or_else(|| state.contacts.first());
    let active_name = active.map(|contact| contact.name).unwrap_or("Unified chat");
    let active_detail = active
        .map(|contact| contact.detail)
        .unwrap_or("contact thread");
    let title_w = if main_w > 620.0 {
        190.0
    } else {
        (main_w - 40.0).max(0.0)
    };
    ui.bounded_label(
        main_x + 20.0,
        14.0,
        title_w,
        active_name,
        2.0,
        palette::TEXT,
    );
    ui.bounded_label(
        main_x + 20.0,
        35.0,
        title_w,
        active_detail,
        2.0,
        palette::MUTED,
    );
    let tabs_w = 226.0_f32.min((main_w - 390.0).max(0.0));
    if tabs_w > 160.0 {
        ui.segmented_tabs(
            UiRect::new(main_x + 220.0, 13.0, tabs_w, 32.0),
            &["chat", "proofs", "files"],
            0,
            20,
        );
    }
    ui.pill(
        main_x + main_w - 322.0,
        15.0,
        132.0,
        "recipient sealed",
        palette::GREEN,
    );
    ui.pill(
        main_x + main_w - 178.0,
        15.0,
        154.0,
        "identity routed",
        palette::ACCENT,
    );
    ui.scene.push_rect(GpuRect::fill(
        main_x,
        m.topbar_h - 1.0,
        main_w,
        1.0,
        0.0,
        palette::BORDER,
    ));

    let transcript_top = m.topbar_h + m.pad;
    let transcript_x = main_x + m.pad;
    let transcript_w = main_w - m.pad * 2.0;
    if state.contacts.is_empty() {
        let empty_w = transcript_w.clamp(260.0, 520.0);
        let empty_h = 126.0;
        let empty_x = transcript_x + (transcript_w - empty_w) * 0.5;
        let empty_y = transcript_top + 46.0;
        ui.card(empty_x, empty_y, empty_w, empty_h, 12.0, palette::PANEL);
        ui.bounded_label(
            empty_x + 22.0,
            empty_y + 24.0,
            empty_w - 44.0,
            "No contacts yet",
            3.0,
            palette::TEXT,
        );
        ui.bounded_label(
            empty_x + 22.0,
            empty_y + 62.0,
            empty_w - 44.0,
            "Connect a contact book or receive an identity-routed contact to show threads here.",
            2.0,
            palette::MUTED,
        );
        let composer = (
            main_x + m.pad,
            h - m.composer_h - m.pad,
            main_w - m.pad * 2.0,
            m.composer_h,
        );
        let composer_text = runtime
            .map(|runtime| runtime.text_value(0, state.composer_placeholder))
            .unwrap_or(state.composer_placeholder);
        let composer_active = runtime.is_some_and(|runtime| {
            runtime
                .focused()
                .is_some_and(|hit| hit.kind == HitKind::Composer)
        });
        ui.composer(
            composer.0,
            composer.1,
            composer.2,
            composer.3,
            composer_text,
            composer_active,
            &[],
        );
        return;
    }

    let rail_w = if transcript_w > 820.0 { 220.0 } else { 0.0 };
    let message_area_w = transcript_w - if rail_w > 0.0 { rail_w + 18.0 } else { 0.0 };
    let message_w = (message_area_w * 0.82).clamp(220.0, 760.0);
    let transcript_limit = h - m.composer_h - m.pad * 2.0;
    let mut message_y = transcript_top;

    for message in state.messages {
        let x = if message.outgoing {
            transcript_x + message_area_w - message_w
        } else {
            transcript_x
        };
        let fill = if message.outgoing {
            palette::USER
        } else {
            palette::ASSISTANT
        };
        let height = estimate_message_height(
            message.body,
            (message_w - 42.0).max(120.0),
            4,
            #[cfg(feature = "fontdue-text")]
            ui.atlas,
        );
        if message_y + height > transcript_limit {
            break;
        }
        let drawn = ui.message_bubble(
            x,
            message_y,
            message_w,
            message.author,
            message.body,
            fill,
            message.accent,
        );
        message_y += drawn + 16.0;
    }

    if rail_w > 0.0 {
        let rail_x = transcript_x + transcript_w - rail_w;
        ui.card(rail_x, transcript_top, rail_w, 220.0, 12.0, palette::PANEL);
        ui.bounded_label(
            rail_x + 16.0,
            transcript_top + 18.0,
            rail_w - 32.0,
            "Thread policy",
            2.0,
            palette::TEXT,
        );
        ui.pill(
            rail_x + 16.0,
            transcript_top + 50.0,
            132.0,
            "2 recipients",
            palette::ACCENT,
        );
        ui.pill(
            rail_x + 16.0,
            transcript_top + 84.0,
            158.0,
            "codex tools scoped",
            palette::VIOLET,
        );
        ui.pill(
            rail_x + 16.0,
            transcript_top + 118.0,
            140.0,
            "relay admitted",
            palette::GREEN,
        );
        ui.divider(
            rail_x + 16.0,
            transcript_top + 152.0,
            rail_w - 32.0,
            Axis::Horizontal,
        );
        ui.bounded_label(
            rail_x + 16.0,
            transcript_top + 166.0,
            rail_w - 88.0,
            "route health",
            2.0,
            palette::MUTED,
        );
        ui.progress_bar(
            UiRect::new(rail_x + 16.0, transcript_top + 190.0, rail_w - 32.0, 8.0),
            if state.connected { 0.86 } else { 0.38 },
            if state.connected {
                palette::GREEN
            } else {
                palette::AMBER
            },
        );
        ui.toggle(
            rail_x + rail_w - 66.0,
            transcript_top + 162.0,
            runtime
                .map(|runtime| runtime.toggle_value(44, state.connected))
                .unwrap_or(state.connected),
            44,
        );
        row("row bg-row border rounded-md p-2 gap-2")
            .child(text("policy").class("w-14 text-muted truncate"))
            .child(text("scoped").class("flex-1 text-green truncate"))
            .child(button("open", 55, ButtonStyle::Ghost).class("w-16 h-8"))
            .render(
                &mut ui,
                UiRect::new(rail_x + 16.0, transcript_top + 206.0, rail_w - 32.0, 42.0),
            );
    }
    if state.messages.len() > 2 {
        ui.scrollbar(
            UiRect::new(
                transcript_x + message_area_w + 6.0,
                transcript_top,
                6.0,
                (transcript_limit - transcript_top).max(80.0),
            ),
            0.72,
            0.0,
        );
    }

    let composer = (
        main_x + m.pad,
        h - m.composer_h - m.pad,
        main_w - m.pad * 2.0,
        m.composer_h,
    );
    let composer_text = runtime
        .map(|runtime| runtime.text_value(0, state.composer_placeholder))
        .unwrap_or(state.composer_placeholder);
    let composer_active = state.connected
        || runtime.is_some_and(|runtime| {
            runtime
                .focused()
                .is_some_and(|hit| hit.kind == HitKind::Composer)
        });
    ui.composer(
        composer.0,
        composer.1,
        composer.2,
        composer.3,
        composer_text,
        composer_active,
        &[
            ("encrypted", palette::GREEN),
            ("contact", palette::ACCENT),
            ("codex tools", palette::VIOLET),
        ],
    );
}

fn render_workspace_chat_app(
    ui: &mut UiPainter<'_, '_>,
    bounds: UiRect,
    app: &UiAppSurface,
    chat_state: &UnifiedChatState<'_>,
) {
    let pad = 14.0;
    ui.fill_rect(bounds, 0.0, palette::BG);
    row("row bg-topbar border rounded-md p-3 gap-3 items-center")
        .child(text("Contacts").class("w-20 text-muted truncate"))
        .child(text(chat_state.subtitle).class("flex-1 text-text truncate"))
        .child(badge(
            if chat_state.connected {
                "relay"
            } else {
                "local"
            },
            if chat_state.connected {
                palette::GREEN
            } else {
                palette::AMBER
            },
        ))
        .render_with_state(
            ui,
            UiRect::new(
                bounds.x + pad,
                bounds.y + pad,
                (bounds.w - pad * 2.0).max(0.0),
                46.0,
            ),
            Some(&app.runtime),
        );

    let composer_h = 76.0;
    let content_top = bounds.y + 72.0;
    let content_bottom = bounds.y + bounds.h - composer_h - pad;
    let content_h = (content_bottom - content_top).max(0.0);
    if chat_state.contacts.is_empty() {
        card("bg-panel border rounded-md p-4 gap-3")
            .child(text("No contacts yet").class("text-text truncate"))
            .child(
                text("Connect a contact book or receive an identity-routed contact.")
                    .class("text-muted truncate"),
            )
            .render_with_state(
                ui,
                UiRect::new(
                    bounds.x + pad,
                    content_top,
                    (bounds.w - pad * 2.0).max(0.0),
                    112.0,
                ),
                Some(&app.runtime),
            );
    } else {
        let rows = chat_state
            .contacts
            .iter()
            .enumerate()
            .map(|(index, contact)| {
                list_row_node(contact.name, contact.detail, index as u32).accent(
                    match contact.kind {
                        UnifiedContactKind::Person => palette::ACCENT,
                        UnifiedContactKind::CodexClient => palette::VIOLET,
                        UnifiedContactKind::Node => palette::GREEN,
                    },
                )
            });
        scroll_area("bg-panel border rounded-md p-2 gap-2", 0.0)
            .scroll_id(101)
            .children(rows)
            .render_with_state(
                ui,
                UiRect::new(
                    bounds.x + pad,
                    content_top,
                    (bounds.w - pad * 2.0).max(0.0),
                    content_h,
                ),
                Some(&app.runtime),
            );
    }

    let draft = app.runtime.text_value(0, chat_state.composer_placeholder);
    ui.composer(
        bounds.x + pad,
        bounds.y + bounds.h - composer_h - pad,
        (bounds.w - pad * 2.0).max(0.0),
        composer_h,
        draft,
        app.runtime
            .focused()
            .is_some_and(|hit| hit.kind == HitKind::Composer),
        &[("encrypted", palette::GREEN), ("identity", palette::ACCENT)],
    );
}

fn render_trust_manager_app(ui: &mut UiPainter<'_, '_>, bounds: UiRect, app: &UiAppSurface) {
    column("bg-panel border rounded-md p-4 gap-3")
        .child(header("Trust Manager").detail("proof dashboard"))
        .child(identity_card(
            "Local identity",
            "browser node",
            "sealed Trust Container",
            221,
        ))
        .child(route_path(
            "Current route",
            &["app", "device", "admission", "relay"],
        ))
        .child(capability_grant_row(
            "EdgeRun Chat",
            "decrypt message",
            "pending",
            220,
        ))
        .child(proof_event_row(
            "Runtime events",
            "proof log empty",
            "0",
            222,
        ))
        .render_with_state(ui, bounds.inset(14.0, 14.0), Some(&app.runtime));
}

fn render_storage_app(ui: &mut UiPainter<'_, '_>, bounds: UiRect, app: &UiAppSurface) {
    column("bg-panel border rounded-md p-4 gap-3")
        .child(header("Storage").detail("verified local cache"))
        .child(package_card(
            "Network apps",
            "run by hash, cache by policy",
            "cache pending",
            301,
        ))
        .child(contact_card(
            "Contact book",
            "IndexedDB projection pending",
            302,
        ))
        .child(attachment_preview(
            "Message payloads",
            "encrypted payload objects",
            303,
        ))
        .child(receipt_row(
            "Cached package bytes",
            "unknown",
            "waiting",
            304,
        ))
        .render_with_state(ui, bounds.inset(14.0, 14.0), Some(&app.runtime));
}

pub const LOCK_UNLOCK_BUTTON_ID: u32 = 900;
pub const LOCK_UNLOCK_FIELD_ID: u32 = 901;
pub const CAPABILITY_ALLOW_BUTTON_ID: u32 = 920;
pub const CAPABILITY_DENY_BUTTON_ID: u32 = 921;
pub const CAPABILITY_DETAILS_BUTTON_ID: u32 = 922;

fn render_lock_screen_app(ui: &mut UiPainter<'_, '_>, bounds: UiRect, app: &UiAppSurface) {
    ui.fill_rect(bounds, 0.0, palette::BG);
    let panel_w = bounds.w.clamp(320.0, 520.0);
    let panel_h = 320.0_f32.min((bounds.h - 32.0).max(220.0));
    let panel = UiRect::new(
        bounds.x + (bounds.w - panel_w) * 0.5,
        bounds.y + (bounds.h - panel_h) * 0.5,
        panel_w,
        panel_h,
    );

    column("bg-panel border rounded-md p-5 gap-4")
        .child(
            row("row gap-3 items-center")
                .child(icon(UiIcon::Lock).accent(palette::ACCENT).class("size-10"))
                .child(
                    column("gap-1 flex-1")
                        .child(text("Trust Container").class("text-text truncate"))
                        .child(text("Unlock required").class("text-muted truncate")),
                ),
        )
        .child(
            text("Your identity, contacts, route policy, app secrets, and decrypt capability are sealed locally.")
                .class("text-muted"),
        )
        .child(checkbox("Keep verified cache available after unlock", true, 902))
        .child(
            field_node("Unlock secret", "Password or passkey ceremony")
                .hit_id(LOCK_UNLOCK_FIELD_ID)
                .class("h-24"),
        )
        .child(
            row("row gap-3")
                .child(button("Unlock", LOCK_UNLOCK_BUTTON_ID, ButtonStyle::Primary).class("h-10 flex-1"))
                .child(button("Offline", LOCK_UNLOCK_BUTTON_ID + 1, ButtonStyle::Ghost).class("h-10 w-28")),
        )
        .render_with_state(ui, panel, Some(&app.runtime));
}

fn render_capability_request_app(ui: &mut UiPainter<'_, '_>, bounds: UiRect, app: &UiAppSurface) {
    ui.fill_rect(bounds, 0.0, palette::BG);
    let panel_w = bounds.w.clamp(340.0, 720.0);
    let panel_h = 430.0_f32.min((bounds.h - 32.0).max(300.0));
    let panel = UiRect::new(
        bounds.x + (bounds.w - panel_w) * 0.5,
        bounds.y + (bounds.h - panel_h) * 0.5,
        panel_w,
        panel_h,
    );

    column("bg-panel border rounded-md p-5 gap-4")
        .child(
            row("row gap-3 items-center")
                .child(
                    icon(UiIcon::Shield)
                        .accent(palette::VIOLET)
                        .class("size-10"),
                )
                .child(
                    column("gap-1 flex-1")
                        .child(text("Capability request").class("text-text truncate"))
                        .child(text("Review before signing").class("text-muted truncate")),
                )
                .child(badge("admission", palette::ACCENT)),
        )
        .child(
            grid("grid grid-cols-2 gap-3", 2)
                .child(
                    metric("Requesting app", "EdgeRun Chat")
                        .detail("session scoped")
                        .class("h-28"),
                )
                .child(
                    metric("Capability", "Decrypt message")
                        .detail("Trust Container")
                        .class("h-28"),
                ),
        )
        .child(
            column("bg-row border rounded-md p-3 gap-2")
                .child(capability_grant_row(
                    "EdgeRun Chat",
                    "decrypt message",
                    "single use",
                    923,
                ))
                .child(route_path("Admission route", &["chat", "device", "trust"])),
        )
        .child(
            row("row gap-3")
                .child(
                    button("Deny", CAPABILITY_DENY_BUTTON_ID, ButtonStyle::Danger)
                        .class("h-10 w-28"),
                )
                .child(
                    button(
                        "Details",
                        CAPABILITY_DETAILS_BUTTON_ID,
                        ButtonStyle::Secondary,
                    )
                    .class("h-10 w-32"),
                )
                .child(
                    button("Allow", CAPABILITY_ALLOW_BUTTON_ID, ButtonStyle::Primary)
                        .class("h-10 flex-1"),
                ),
        )
        .render_with_state(ui, panel, Some(&app.runtime));
}

pub(super) fn render_component_gallery_app(
    ui: &mut UiPainter<'_, '_>,
    bounds: UiRect,
    app: &UiAppSurface,
) {
    scroll_area("bg-panel border rounded-md p-4 gap-4", 0.0)
        .scroll_id(760)
        .children([
            header("Component Gallery").detail("shared Rust GPU primitives"),
            section("Foundation", "inputs, buttons, icons"),
            row("gap-2 items-center")
                .child(button("Run", 761, ButtonStyle::Primary).class("h-8 w-24"))
                .child(
                    button("Waiting", 762, ButtonStyle::Secondary)
                        .class("h-8 w-28")
                        .loading(true),
                )
                .child(icon_button(UiIcon::Settings, 763).class("size-8"))
                .child(icon(UiIcon::Shield).accent(palette::VIOLET).class("size-8")),
            row("gap-4 items-center")
                .child(checkbox("Verify cache", true, 764).class("h-8 w-44"))
                .child(radio("DAO admission", true, 765).class("h-8 w-44"))
                .child(select_node("Route", "relay://nodes", 766).class("h-14 w-56")),
            section("Feedback", "system surfaces"),
            row("gap-3")
                .child(
                    toast("Package hash verified", UiIcon::Check, palette::GREEN)
                        .class("h-11 flex-1"),
                )
                .child(progress_ring(0.64, palette::ACCENT).class("size-11")),
            empty_state(
                "No proofs yet",
                "Runtime events will appear after signed work is admitted.",
                UiIcon::Trust,
            )
            .class("h-40"),
            section("Data", "tables and navigation"),
            breadcrumb(&["Trust", "Routes", "Relay"], 2, 770).class("h-8"),
            command_palette("Search contacts, packages, routes", 780).class("h-11"),
            table_labels(
                &["Object", "Policy", "State"],
                &[
                    &["chat.app", "policy:run", "cached"],
                    &["relay path", "admission", "active"],
                    &["receipt", "payable", "pending"],
                ],
                790,
            )
            .class("h-40"),
            section("EdgeRun", "domain components"),
            grid_auto("grid grid-cols-1 md:grid-cols-2 gap-3")
                .child(identity_card(
                    "Local identity",
                    "browser node",
                    "policy:personal",
                    800,
                ))
                .child(package_card(
                    "EdgeRun Chat",
                    "run by hash",
                    "b3:message-ui",
                    801,
                )),
            contact_card("Codex Client", "local app identity", 802),
            thread_row("Alice", "encrypted message available", true, 803),
            capability_grant_row("Chat", "decrypt message", "single use", 804),
            proof_event_row("Relay delivery", "b3:relay-proof", "accepted", 805),
            route_path("Message route", &["app", "device", "admission", "relay"]),
            receipt_row("Relay delivery", "$0.0004", "pending", 806),
        ])
        .render_with_state(ui, bounds.inset(14.0, 14.0), Some(&app.runtime));
}

fn render_generic_workspace_app(ui: &mut UiPainter<'_, '_>, bounds: UiRect, app: &UiAppSurface) {
    card("bg-panel border rounded-md p-4 gap-3")
        .child(text(&app.title).class("text-text truncate"))
        .child(text("No app renderer registered.").class("text-muted truncate"))
        .render_with_state(ui, bounds.inset(14.0, 14.0), Some(&app.runtime));
}
