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
    build_edgerun_workspace_shell_with_font_and_work(
        scene, atlas, width, height, workspace, chat_state, None,
    );
}

#[cfg(feature = "fontdue-text")]
pub fn build_edgerun_workspace_shell_with_font_and_work(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    workspace: &mut UiWorkspace,
    chat_state: &UnifiedChatState<'_>,
    work: Option<&UiWorkProjection>,
) {
    let theme = workspace.user_style.resolved_theme();
    scene.clear = theme.colors.bg;
    scene.clear_rects();
    let mut ui = UiPainter {
        scene,
        theme,
        atlas: Some(atlas),
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    ui.fill_rect(
        UiRect::new(0.0, 0.0, width.max(360.0), height.max(320.0)),
        0.0,
        theme.colors.bg,
    );
    workspace.render(
        &mut ui,
        UiRect::new(8.0, 8.0, (width - 16.0).max(0.0), (height - 16.0).max(0.0)),
        |ui, bounds, app| match app.kind {
            UiAppKind::Chat => render_workspace_chat_app(ui, bounds, app, chat_state),
            UiAppKind::TrustManager => render_trust_manager_app(ui, bounds, app, work),
            UiAppKind::Storage => render_storage_app(ui, bounds, app, work),
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
        theme: UiResolvedTheme::default(),
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
    let theme = app.style_preview.resolved_theme();
    scene.clear = theme.colors.bg;
    scene.clear_rects();
    app.full_screen = true;
    app.bounds = None;
    let mut ui = UiPainter {
        scene,
        theme,
        atlas: Some(atlas),
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    let bounds = UiRect::new(0.0, 0.0, width.max(360.0), height.max(320.0));
    ui.fill_rect(bounds, 0.0, theme.colors.bg);
    let clipped = ui
        .scene
        .push_clip(GpuClip::new(bounds.x, bounds.y, bounds.w, bounds.h));
    if clipped {
        app.bounds = Some(bounds);
        match app.kind {
            UiAppKind::Chat => render_workspace_chat_app(&mut ui, bounds, app, chat_state),
            UiAppKind::TrustManager => render_trust_manager_app(&mut ui, bounds, app, None),
            UiAppKind::Storage => render_storage_app(&mut ui, bounds, app, None),
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
        theme: UiResolvedTheme::default(),
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
        theme: UiResolvedTheme::default(),
        #[cfg(feature = "fontdue-text")]
        atlas,
        #[cfg(not(feature = "fontdue-text"))]
        _font: PhantomData,
    };
    let w = width.max(360.0);
    let h = height.max(320.0);
    let active = state
        .contacts
        .get(state.selected_contact)
        .or_else(|| state.contacts.first());
    let active_name = active.map(|contact| contact.name).unwrap_or("Unified chat");
    let active_detail = active
        .map(|contact| contact.detail)
        .unwrap_or("contact thread");
    let composer_text = runtime
        .map(|runtime| runtime.text_value(0, state.composer_placeholder))
        .unwrap_or(state.composer_placeholder);

    let mut contacts = card("bg-sidebar border rounded-lg p-3 gap-2")
        .child(shadcn_command(state.title, 1))
        .child(shadcn_badge(
            if state.connected { "relay" } else { "local" },
            UiShadcnBadgeVariant::Secondary,
        ));
    for (index, contact) in state.contacts.iter().enumerate() {
        contacts = contacts.child(
            shadcn_item(
                contact.name,
                contact.detail,
                index as u32,
                contact_accent(contact.kind),
            )
            .selected(index == state.selected_contact),
        );
    }
    if state.contacts.is_empty() {
        contacts = contacts.child(shadcn_empty(
            "No contacts yet",
            "Connect a contact book or receive an identity-routed contact.",
            UiIcon::Search,
        ));
    }

    let mut transcript = card("bg-panel border rounded-lg p-3 gap-3")
        .child(shadcn_breadcrumb(
            &[state.title, active_name, "Thread"],
            2,
            20,
        ))
        .child(shadcn_tabs(&["Chat", "Proofs", "Files"], 0, 30))
        .child(shadcn_alert("Thread policy", active_detail, UiIcon::Shield));
    for (index, message) in state.messages.iter().enumerate() {
        transcript = transcript.child(shadcn_item(
            message.author,
            message.body,
            100 + index as u32,
            message.accent,
        ));
    }
    if state.messages.is_empty() {
        transcript = transcript.child(shadcn_empty(
            "No messages yet",
            "Start a verified, identity-routed thread.",
            UiIcon::Chat,
        ));
    }
    transcript = transcript
        .child(
            shadcn_textarea("Message", composer_text)
                .hit_id(0)
                .class("h-24"),
        )
        .child(shadcn_button(
            "Send",
            2,
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Default,
        ));

    let root = if w >= 760.0 {
        row("row gap-3 h-full")
            .child(contacts.class("w-80 h-full"))
            .child(transcript.class("flex-1 h-full"))
    } else {
        column("gap-3 h-full")
            .child(contacts.class("h-64"))
            .child(transcript.class("flex-1"))
    };
    root.render_with_state(&mut ui, UiRect::new(0.0, 0.0, w, h), runtime);
}

#[cfg(feature = "fontdue-text")]
fn render_workspace_chat_app(
    ui: &mut UiPainter<'_, '_>,
    bounds: UiRect,
    app: &UiAppSurface,
    chat_state: &UnifiedChatState<'_>,
) {
    let pad = 14.0;
    let colors = ui.theme().colors;
    ui.fill_rect(bounds, 0.0, colors.bg);
    let content = bounds.inset(pad, pad);
    let mut panel = shadcn_card("EdgeRun Chat", chat_state.subtitle)
        .child(shadcn_tabs(&["Contacts", "Thread", "Proofs"], 0, 120))
        .child(shadcn_badge(
            if chat_state.connected {
                "relay"
            } else {
                "local"
            },
            UiShadcnBadgeVariant::Secondary,
        ));
    if chat_state.contacts.is_empty() {
        panel = panel.child(shadcn_empty(
            "No contacts yet",
            "Connect a contact book or receive an identity-routed contact.",
            UiIcon::Search,
        ));
    } else {
        for (index, contact) in chat_state.contacts.iter().enumerate() {
            panel = panel.child(
                shadcn_item(
                    contact.name,
                    contact.detail,
                    index as u32,
                    contact_accent(contact.kind),
                )
                .selected(index == chat_state.selected_contact),
            );
        }
    }

    let draft = app.runtime.text_value(0, chat_state.composer_placeholder);
    panel
        .child(shadcn_textarea("Message", draft).hit_id(0).class("h-24"))
        .child(
            row("row gap-2")
                .child(shadcn_badge("encrypted", UiShadcnBadgeVariant::Secondary))
                .child(shadcn_badge("identity", UiShadcnBadgeVariant::Secondary))
                .child(shadcn_button(
                    "Send",
                    2,
                    UiShadcnButtonVariant::Default,
                    UiShadcnButtonSize::Default,
                )),
        )
        .render_with_state(ui, content, Some(&app.runtime));
}

#[cfg(feature = "fontdue-text")]
fn render_trust_manager_app(
    ui: &mut UiPainter<'_, '_>,
    bounds: UiRect,
    app: &UiAppSurface,
    work: Option<&UiWorkProjection>,
) {
    let fallback;
    let work = match work {
        Some(work) => work,
        None => {
            fallback = UiWorkProjection::preview();
            &fallback
        }
    };
    shadcn_card("Trust Manager", "proof dashboard")
        .child(shadcn_item(
            "Local identity",
            &work.local_node,
            221,
            ui.theme().colors.accent,
        ))
        .child(shadcn_alert(
            "Admitted route",
            "node instance -> admission -> relay -> capability",
            UiIcon::Shield,
        ))
        .child(shadcn_table(
            &["Object", "Hash", "State"],
            &[
                &[
                    "Admission policy",
                    &work.policy_hash,
                    if work.admission_verified {
                        "verified"
                    } else {
                        "pending"
                    },
                ],
                &[
                    "WorkRequest",
                    &work.request_hash,
                    if work.request_verified {
                        "ok"
                    } else {
                        "pending"
                    },
                ],
                &[
                    "WorkAdmission",
                    &work.admission_hash,
                    if work.admission_verified {
                        "ok"
                    } else {
                        "pending"
                    },
                ],
            ],
            220,
        ))
        .render_with_state(ui, bounds.inset(14.0, 14.0), Some(&app.runtime));
}

#[cfg(feature = "fontdue-text")]
fn render_storage_app(
    ui: &mut UiPainter<'_, '_>,
    bounds: UiRect,
    app: &UiAppSurface,
    work: Option<&UiWorkProjection>,
) {
    let fallback;
    let work = match work {
        Some(work) => work,
        None => {
            fallback = UiWorkProjection::preview();
            &fallback
        }
    };
    let budget = std::format!("{} units admitted", work.admitted_budget);
    let cost = std::format!("{} units deterministic", work.retrieval_cost);
    shadcn_card("Storage", "verified local cache")
        .child(shadcn_item(
            "Network app payload",
            &work.storage_payload_hash,
            301,
            ui.theme().colors.accent,
        ))
        .child(shadcn_table(
            &["Storage ref", "Value"],
            &[
                &["Admission path", &work.admission_node],
                &["Manifest", &work.manifest_hash],
                &["Budget", &budget],
                &["Retrieval", &cost],
                &["Relay", &work.relay_node],
                &["Channel", &work.channel],
            ],
            302,
        ))
        .child(shadcn_alert(
            "Payload verification",
            if work.storage_payload_verified {
                "typed payload verified"
            } else {
                "payload pending"
            },
            UiIcon::Storage,
        ))
        .render_with_state(ui, bounds.inset(14.0, 14.0), Some(&app.runtime));
}

pub const LOCK_UNLOCK_BUTTON_ID: u32 = 900;
pub const LOCK_UNLOCK_FIELD_ID: u32 = 901;
pub const CAPABILITY_ALLOW_BUTTON_ID: u32 = 920;
pub const CAPABILITY_DENY_BUTTON_ID: u32 = 921;
pub const CAPABILITY_DETAILS_BUTTON_ID: u32 = 922;

#[cfg(any(feature = "fontdue-text", test))]
pub(super) fn render_lock_screen_app(
    ui: &mut UiPainter<'_, '_>,
    bounds: UiRect,
    app: &UiAppSurface,
) {
    let colors = ui.theme().colors;
    ui.fill_rect(bounds, 0.0, colors.bg);
    let panel_w = bounds.w.clamp(320.0, 520.0);
    let panel_h = 320.0_f32.min((bounds.h - 32.0).max(220.0));
    let panel = UiRect::new(
        bounds.x + (bounds.w - panel_w) * 0.5,
        bounds.y + (bounds.h - panel_h) * 0.5,
        panel_w,
        panel_h,
    );

    shadcn_card("Trust Container", "Unlock required")
        .child(shadcn_alert(
            "Local sealed root",
            "Your identity, contacts, route policy, app secrets, and decrypt capability are sealed locally.",
            UiIcon::Lock,
        ))
        .child(shadcn_checkbox(
            "Keep verified cache available after unlock",
            true,
            902,
        ))
        .child(
            shadcn_input("Unlock secret", "Password or passkey ceremony")
                .hit_id(LOCK_UNLOCK_FIELD_ID)
                .class("h-24"),
        )
        .child(
            row("row gap-3")
                .child(
                    shadcn_button(
                        "Unlock",
                        LOCK_UNLOCK_BUTTON_ID,
                        UiShadcnButtonVariant::Default,
                        UiShadcnButtonSize::Default,
                    )
                    .class("flex-1"),
                )
                .child(shadcn_button(
                    "Offline",
                    LOCK_UNLOCK_BUTTON_ID + 1,
                    UiShadcnButtonVariant::Ghost,
                    UiShadcnButtonSize::Default,
                )),
        )
        .render_with_state(ui, panel, Some(&app.runtime));
}

#[cfg(any(feature = "fontdue-text", test))]
pub(super) fn render_capability_request_app(
    ui: &mut UiPainter<'_, '_>,
    bounds: UiRect,
    app: &UiAppSurface,
) {
    let colors = ui.theme().colors;
    ui.fill_rect(bounds, 0.0, colors.bg);
    let panel_w = bounds.w.clamp(340.0, 720.0);
    let panel_h = 430.0_f32.min((bounds.h - 32.0).max(300.0));
    let panel = UiRect::new(
        bounds.x + (bounds.w - panel_w) * 0.5,
        bounds.y + (bounds.h - panel_h) * 0.5,
        panel_w,
        panel_h,
    );

    shadcn_card("Capability request", "Review before signing")
        .child(shadcn_badge("admission", UiShadcnBadgeVariant::Secondary))
        .child(shadcn_table(
            &["Field", "Value"],
            &[
                &["Requesting app", "EdgeRun Chat"],
                &["Scope", "session scoped"],
                &["Capability", "Decrypt message"],
                &["Authority", "Trust Container"],
            ],
            923,
        ))
        .child(shadcn_alert(
            "Admission route",
            "chat -> device -> trust",
            UiIcon::Shield,
        ))
        .child(
            row("row gap-3")
                .child(shadcn_button(
                    "Deny",
                    CAPABILITY_DENY_BUTTON_ID,
                    UiShadcnButtonVariant::Destructive,
                    UiShadcnButtonSize::Default,
                ))
                .child(shadcn_button(
                    "Details",
                    CAPABILITY_DETAILS_BUTTON_ID,
                    UiShadcnButtonVariant::Secondary,
                    UiShadcnButtonSize::Default,
                ))
                .child(
                    shadcn_button(
                        "Allow",
                        CAPABILITY_ALLOW_BUTTON_ID,
                        UiShadcnButtonVariant::Default,
                        UiShadcnButtonSize::Default,
                    )
                    .class("flex-1"),
                ),
        )
        .render_with_state(ui, panel, Some(&app.runtime));
}

#[cfg(any(feature = "fontdue-text", test))]
pub(super) fn render_component_gallery_app(
    ui: &mut UiPainter<'_, '_>,
    bounds: UiRect,
    app: &UiAppSurface,
) {
    let preview = app.style_preview;
    let previous_theme = ui.theme();
    ui.set_theme(preview.resolved_theme());
    let content = bounds.inset(14.0, 14.0);
    let wide = content.w >= 980.0;

    let studio = if wide {
        row("row gap-4 h-full")
            .child(component_style_authority_panel(preview, true).class("w-60 h-full"))
            .child(
                scroll_area("bg-panel border rounded-md p-4 gap-4 flex-1 h-full", 0.0)
                    .scroll_id(760)
                    .child(component_studio_toolbar(preview))
                    .child(shadcn_component_wall(
                        preview,
                        (content.w - 256.0).max(360.0),
                    )),
            )
    } else {
        scroll_area("bg-panel border rounded-md p-4 gap-4 h-full", 0.0)
            .scroll_id(760)
            .child(component_style_authority_panel(preview, false).class("h-96"))
            .child(component_studio_toolbar(preview))
            .child(shadcn_component_wall(preview, content.w))
    };

    studio.render_with_state(ui, content, Some(&app.runtime));
    ui.set_theme(previous_theme);
}

#[cfg(any(feature = "fontdue-text", test))]
fn component_style_authority_panel(preview: UiComponentPreviewState, rail: bool) -> UiNode {
    let user = preview.user_preset;
    let author = preview.author_preset;
    let active = preview.resolved_theme();
    let classes = if rail {
        "bg-sidebar border rounded-lg p-3 gap-3"
    } else {
        "bg-sidebar border rounded-lg p-3 gap-3"
    };

    card(classes)
        .child(
            row("row h-9 items-center")
                .child(text("Menu").class("flex-1 text-text truncate"))
                .child(icon_button(UiIcon::Settings, 760).class("size-8")),
        )
        .child(divider("h-px"))
        .child(shadcn_label("Style"))
        .child(
            shadcn_item(
                user.name,
                active.authority_label(),
                761,
                active.colors.accent,
            )
            .selected(matches!(preview.authority, UiStyleAuthority::User)),
        )
        .child(shadcn_select("Base Color", user.base_color, 766))
        .child(shadcn_item(
            "Theme",
            user.scheme_label(),
            766,
            active.colors.info,
        ))
        .child(shadcn_item(
            "Accent",
            user.accent_label(),
            767,
            active.colors.accent,
        ))
        .child(divider("h-px"))
        .child(shadcn_label("Typography"))
        .child(shadcn_field("Heading", "Geist"))
        .child(shadcn_field("Font", "Geist"))
        .child(divider("h-px"))
        .child(shadcn_label("System"))
        .child(shadcn_select("Icon Library", user.icon_set, 769))
        .child(shadcn_item(
            "Radius",
            user.radius_label(),
            768,
            active.colors.success,
        ))
        .child(divider("h-px"))
        .child(shadcn_label("Author Preset"))
        .child(
            shadcn_item(author.name, author.scheme_label(), 771, active.colors.info)
                .selected(matches!(preview.authority, UiStyleAuthority::AuthorVision)),
        )
        .child(shadcn_button(
            "Preview Author",
            762,
            UiShadcnButtonVariant::Secondary,
            UiShadcnButtonSize::Default,
        ))
        .child(shadcn_button(
            "Keep User Style",
            763,
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Default,
        ))
        .child(
            identity_card("Local identity", "node instance", "policy:personal", 800).class("h-28"),
        )
}

#[cfg(any(feature = "fontdue-text", test))]
fn component_studio_toolbar(preview: UiComponentPreviewState) -> UiNode {
    let active = preview.resolved_theme();

    row("row gap-3 items-center h-11")
        .child(shadcn_command("Search documentation...", 780).class("h-11 flex-1"))
        .child(shadcn_badge(
            active.authority_label(),
            UiShadcnBadgeVariant::Secondary,
        ))
        .child(shadcn_button(
            "Open in v0",
            764,
            UiShadcnButtonVariant::Outline,
            UiShadcnButtonSize::Default,
        ))
        .child(shadcn_button(
            "Get Code",
            765,
            UiShadcnButtonVariant::Default,
            UiShadcnButtonSize::Default,
        ))
}

#[cfg(any(feature = "fontdue-text", test))]
fn shadcn_component_wall(preview: UiComponentPreviewState, width: f32) -> UiNode {
    let active = preview.resolved_theme();
    let months = ["Dec", "Jan", "Feb", "Mar", "Apr", "May"];
    let activity = [0.56, 0.78, 0.62, 0.92, 0.52, 0.98];

    column("gap-4 h-980")
        .child(
            shadcn_menubar(
                &[
                    "Docs",
                    "Components",
                    "Blocks",
                    "Charts",
                    "Directory",
                    "Create",
                ],
                1,
                820,
            )
            .class("h-11"),
        )
        .child(
            grid_auto_for_width(
                "grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4 h-760",
                width,
            )
            .child(
                shadcn_card("Contribution History", "Last 6 months of activity")
                    .class("h-108")
                    .child(
                        shadcn_chart("Activity", &months, &activity)
                            .accent(active.colors.accent)
                            .class("h-48"),
                    )
                    .child(
                        grid("grid grid-cols-2 gap-3", 2)
                            .child(
                                shadcn_card("Upcoming", "May 25, 2024")
                                    .child(shadcn_badge(
                                        "$1,000 scheduled",
                                        UiShadcnBadgeVariant::Secondary,
                                    ))
                                    .class("h-28"),
                            )
                            .child(
                                shadcn_card("Auto-save Plan", "Accelerated")
                                    .child(shadcn_badge(
                                        "Recurring weekly",
                                        UiShadcnBadgeVariant::Secondary,
                                    ))
                                    .class("h-28"),
                            ),
                    )
                    .child(shadcn_button(
                        "View Full Report",
                        772,
                        UiShadcnButtonVariant::Default,
                        UiShadcnButtonSize::Default,
                    )),
            )
            .child(
                shadcn_card(
                    "Payout Threshold",
                    "Set the minimum balance required before payout is triggered.",
                )
                .class("h-108")
                .child(shadcn_select(
                    "Preferred Currency",
                    "USD - United States Dollar",
                    773,
                ))
                .child(
                    shadcn_slider("Minimum Payout Amount", 0.25, 774)
                        .range_labels("$50 (MIN)", "$10,000 (MAX)")
                        .accent(active.colors.accent)
                        .class("h-20"),
                )
                .child(
                    shadcn_textarea("Notes", "Add any notes for this payout configuration...")
                        .hit_id(775)
                        .class("h-28"),
                )
                .child(shadcn_button(
                    "Save Threshold",
                    776,
                    UiShadcnButtonVariant::Default,
                    UiShadcnButtonSize::Default,
                )),
            )
            .child(
                shadcn_card("Savings Targets", "Active milestones for 2024")
                    .class("h-108")
                    .child(
                        shadcn_card("Retirement", "$420,000")
                            .child(shadcn_progress(0.65).class("w-full"))
                            .child(shadcn_badge(
                                "65% achieved",
                                UiShadcnBadgeVariant::Secondary,
                            ))
                            .class("h-32"),
                    )
                    .child(
                        shadcn_card("Real Estate", "$85,000")
                            .child(shadcn_progress(0.32).class("w-full"))
                            .child(shadcn_badge(
                                "32% achieved",
                                UiShadcnBadgeVariant::Secondary,
                            ))
                            .class("h-32"),
                    )
                    .child(
                        text("You have not met your targets for this year.")
                            .class("text-muted truncate"),
                    ),
            )
            .child(
                shadcn_card("Buy Investment", "Review before sending an order")
                    .class("h-108")
                    .child(shadcn_input("Amount to Invest", "$1,000.00").hit_id(810))
                    .child(shadcn_select("Order Type", "Market Order", 811))
                    .child(
                        shadcn_table(
                            &["Estimate", "Value"],
                            &[
                                &["Estimated Shares", "1.95"],
                                &["Buying Power", "$12,450.00"],
                            ],
                            812,
                        )
                        .class("h-24"),
                    )
                    .child(shadcn_button(
                        "Review Order",
                        813,
                        UiShadcnButtonVariant::Default,
                        UiShadcnButtonSize::Default,
                    )),
            )
            .child(
                shadcn_empty(
                    "Distribute Track",
                    "Upload your first master to start reaching listeners.",
                    UiIcon::App,
                )
                .child(shadcn_button(
                    "Create Release",
                    779,
                    UiShadcnButtonVariant::Default,
                    UiShadcnButtonSize::Default,
                ))
                .class("h-104"),
            )
            .child(
                shadcn_card("Claimable Balance", "$0.00")
                    .class("h-104")
                    .child(shadcn_badge("Pending Setup", UiShadcnBadgeVariant::Outline))
                    .child(
                        shadcn_table(
                            &["Royalty", "Amount"],
                            &[
                                &["Net Royalties", "$0.00"],
                                &["Processing Fee", "-$0.00"],
                                &["Total Ready to Claim", "$0.00 USD"],
                            ],
                            781,
                        )
                        .class("h-36"),
                    ),
            )
            .child(
                shadcn_card("Recent Transactions", "Your latest account activity.")
                    .class("h-104")
                    .child(
                        shadcn_table(
                            &["Merchant", "Date", "Amount"],
                            &[
                                &["Blue Bottle Coffee", "Today", "-$6.50"],
                                &["Whole Foods Market", "Yesterday", "-$142.30"],
                                &["Stripe Payout", "Oct 12", "+$4,200.00"],
                                &["Uber Technologies", "Oct 11", "-$24.10"],
                                &["Netflix Subscription", "Oct 10", "-$19.99"],
                            ],
                            784,
                        )
                        .class("h-52"),
                    ),
            )
            .child(
                shadcn_sheet(
                    "Account Access",
                    "Update your credentials",
                    "Email Address",
                    "artist@studio.inc",
                    "Update Security",
                    814,
                )
                .child(shadcn_alert(
                    "Danger Zone",
                    "Archive account",
                    UiIcon::Warning,
                ))
                .class("h-104"),
            ),
        )
        .child(
            grid_auto_for_width(
                "grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3 h-52",
                width,
            )
            .child(shadcn_sidebar(
                "Overview",
                "Dashboard",
                &[
                    "Dashboard",
                    "Transactions",
                    "Investments",
                    "Goals",
                    "Budget",
                ],
                0,
                "Account",
                "Profile, billing, notifications, security",
                802,
            ))
            .child(shadcn_breadcrumb(&["Home", "Payments", "Transfer"], 2, 810))
            .child(shadcn_calendar(
                "May 2024",
                &["", "", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10"],
                8,
                830,
            ))
            .child(shadcn_sonner(&[
                ("Order reviewed", UiIcon::Check, active.colors.success),
                (
                    "Policy requires approval",
                    UiIcon::Warning,
                    active.colors.warning,
                ),
            ])),
        )
        .child(
            shadcn_table(
                &["Component", "Authority", "State"],
                &[
                    &["Button", "shadcn_button", "ready"],
                    &["Card", "shadcn_card", "ready"],
                    &["Input", "shadcn_input", "ready"],
                    &["Table", "shadcn_table", "ready"],
                    &["Dialog", "shadcn_dialog", "ready"],
                ],
                790,
            )
            .class("h-40"),
        )
}

#[cfg(feature = "fontdue-text")]
fn render_generic_workspace_app(ui: &mut UiPainter<'_, '_>, bounds: UiRect, app: &UiAppSurface) {
    shadcn_empty(&app.title, "No app renderer registered.", UiIcon::App).render_with_state(
        ui,
        bounds.inset(14.0, 14.0),
        Some(&app.runtime),
    );
}

fn contact_accent(kind: UnifiedContactKind) -> Color4 {
    match kind {
        UnifiedContactKind::Person => palette::ACCENT,
        UnifiedContactKind::CodexClient => palette::VIOLET,
        UnifiedContactKind::Node => palette::GREEN,
    }
}
