//! Shared GPU UI scene primitives and a small native OpenGL renderer.
//!
//! The scene types are platform neutral and are intended to be consumed by both
//! native OpenGL/EGL/SDL hosts and browser WebGL hosts. The native GL renderer
//! below is only one backend for the scene.

use std::string::String;
use std::vec::Vec;

#[cfg(test)]
use crate::gpu_ui;

mod accessibility;
mod app_host;
mod assets;
mod bitmap_font;
mod component_inventory;
pub mod components;
mod extracted_blocks;
mod extracted_system;
#[cfg(all(feature = "gpu-gl", not(target_arch = "wasm32")))]
pub mod gl;
mod icons;
mod initial_setup_ui;
mod node;
mod paint;
mod painter;
pub mod palette;
#[cfg(feature = "fontdue-text")]
mod pocketbase_admin;
mod preset_code;
mod primitives;
mod runtime;
mod scene;
#[cfg(all(feature = "sdl", not(target_arch = "wasm32")))]
pub mod sdl;
mod shadcn_demo_catalog;
mod shadcn_demo_preview;
mod shadcn_events;
mod shadcn_exact;
mod shadcn_props;
mod shell;
mod source_captures;
pub mod spacing;
pub mod style;
mod style_family;
#[cfg(feature = "fontdue-text")]
mod text;
mod theme;
pub mod webgl2;
mod workspace;
pub use accessibility::{
    UiA11yNode, UiA11yRole, UiA11yState, accessibility_tree, accessibility_tree_with_state,
};
pub use app_host::{UiAppControl, UiSurfaceApp, UiSurfaceHost};
pub use assets::{
    EDGERUN_COMPONENT_PACK_ENTRIES, EDGERUN_GEIST_FONT_FACES, EDGERUN_INTER_FONT_FACES,
    EDGERUN_LUCIDE_GEIST_ASSET_PACK, EDGERUN_LUCIDE_ICON_PACK_ENTRIES, EDGERUN_REQUIRED_EMOJI,
    EDGERUN_TABLER_ICON_PACK_ENTRIES, EDGERUN_TABLER_INTER_ASSET_PACK, REQUIRED_FONT_CHARS,
    REQUIRED_UI_EMOJI, UiAssetLimits, UiAssetPackError, UiAssetPackRuntime, UiAssetPackSpec,
    UiComponentPackEntry, UiComponentPackSpec, UiEmojiPackSpec, UiEmojiSpec, UiFontFaceSpec,
    UiFontPackSpec, UiIconPackEntry, UiIconPackSpec,
};
pub use component_inventory::{
    EXTRACTED_COMPONENT_KINDS, EXTRACTED_COMPONENTS, EXTRACTED_PATTERN_KINDS, EXTRACTED_PATTERNS,
    EXTRACTED_SLOTS, EXTRACTED_SOURCE_ICONS, EXTRACTED_STATE_KINDS, EXTRACTED_STATES,
    UiExtractedComponentKind, UiExtractedComponentSpec, UiExtractedIconSpec,
    UiExtractedPatternKind, UiExtractedPatternSpec, UiExtractedSlotSpec, UiExtractedStateKind,
    UiExtractedStateSpec,
};
pub use components::{
    AdmissionPolicyRow, AppStoreCard, BarChart, CapabilityGrantRow, ControlAccessory, ControlRow,
    DataTableColumn, DataTableControls, DataTableSort, Field, IconOnlyButton, ImportSyncSourceKind,
    ImportSyncSourceRow, MenuItem, MetricCard, NetworkAppPrompt, NodeInstanceRow, PackageProofRow,
    PanelHeader, PublishFromNodeKind, PublishFromNodeRow, ReceiptPaymentRow, ReceiptPaymentStatus,
    RouteBudgetRow, RuntimeEventRow, SegmentedControl, SegmentedControlItem, Slider,
    SystemSurfaceKind, SystemSurfaceState, SystemSurfaceStatePanel, TextArea, TransactionRow,
    TrustManagerActions, UI_COMPONENT_ACCESSIBILITY_METADATA, UI_COMPONENT_PROJECTION_CONTRACTS,
    UI_COMPONENT_STATE_MATRICES, UI_COMPONENT_STATES, UI_COMPONENT_TEST_IDS,
    UiComponentAccessibilityMetadata, UiComponentProjectedField, UiComponentProjectionContract,
    UiComponentState, UiComponentStateMatrix, UiComponentTestId, UiGrid, UiStack,
    admission_policy_row, app_store_card, bar_chart, capability_grant_detail_row, control_row,
    data_table_controls, field, icon_only_button, import_sync_source_row, menu_item, metric_card,
    network_app_prompt, node_instance_row, package_proof_row, panel_header, publish_from_node_row,
    receipt_payment_row, route_budget_row, runtime_event_row, segmented_control, slider,
    system_surface_state_panel, text_area, transaction_row, trust_manager_actions,
};
pub use extracted_blocks::{
    EXTRACTED_BLOCK_IDS, EXTRACTED_BLOCK_KINDS, EXTRACTED_BLOCKS, EXTRACTED_ICON_LIBRARY,
    UiExtractedBlockId, UiExtractedBlockKind, UiExtractedBlockSpec, build_extracted_block,
    component_studio, data_feedback_block, directory_block, finance_block, input_group_block,
    network_app_block, overlay_selection_block, style_authority_panel, style_family_picker_block,
    trust_activity_block,
};
pub use extracted_system::{
    EDGERUN_EXTRACTED_UI_SYSTEM, EXTRACTED_INVENTORY_KINDS, UiExtractedBlockKindSummary,
    UiExtractedCatalogCounts, UiExtractedComponentKindSummary, UiExtractedCoverageReport,
    UiExtractedDesignSystem, UiExtractedInventoryKind, UiExtractedInventorySection,
    UiExtractedPatternKindSummary, UiExtractedPatternReferenceReport, UiExtractedStateKindSummary,
    UiExtractedStyleTokenKindSummary, UiExtractedWorkItem, UiExtractedWorkItemKind,
    UiShadcnCompatibilityReport, UiShadcnDemoCategorySummary, UiShadcnDemoStatusSummary,
};
#[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
pub use icons::UiIconAtlas;
#[cfg(not(feature = "tabler-svg-atlas"))]
use icons::draw_canonical_icon;
#[cfg(feature = "lucide-svg-atlas")]
pub use icons::lucide_svg_icon_atlas;
#[cfg(feature = "tabler-svg-atlas")]
pub use icons::tabler_svg_icon_atlas;
pub use icons::{
    ICON_VISUAL_MAX, ROW_ICON_VISUAL, STATUS_BADGE_ICON_VISUAL, UiIcon, UiIconAtlasRect, UiIconSet,
    UiIconTone, center_icon_rect, icon_hit_rect, icon_only_button_icon_rect, row_icon_rect,
    status_badge_icon_rect,
};
use icons::{icon_circle, icon_line};
pub use initial_setup_ui::{
    INITIAL_SETUP_CONFIRM_FIELD_ID, INITIAL_SETUP_CREATE_BUTTON_ID,
    INITIAL_SETUP_PASSWORD_FIELD_ID, InitialSetupUiIntent, InitialSetupUiState,
    YUBIKEY_GRANT_PIN_FIELD_ID, YUBIKEY_GRANT_SIGN_BUTTON_ID, YubiKeyGrantCeremonyIntent,
    YubiKeyGrantCeremonyState, build_initial_setup_surface, build_yubikey_grant_ceremony_surface,
};
pub use node::{
    UI_OVERLAY_STACKING_ORDER, UiCompositionIssue, UiDragSourceSpec, UiDropTargetSpec,
    UiInteraction, UiLayoutIssue, UiLayoutTraceEntry, UiNode, UiNodeKind, UiOverlayLayer,
    UiResolvedLayout, app_launcher_item, attachment_preview, avatar_node, badge, bar_chart_labels,
    bar_chart_node, bento_grid, bento_grid_auto, bento_grid_auto_for_width, breadcrumb, button,
    capability_grant_row, card, checkbox, column, command_palette, contact_card, control_row_node,
    dialog, divider, empty_state, field_node, grid, grid_auto, grid_auto_for_width, header, icon,
    icon_button, icon_button_with_tooltip, identity_card, list_row_node, masonry, masonry_auto,
    masonry_auto_for_width, menu_item_node, metric, package_card, progress_bar_node, progress_ring,
    proof_event_row, radio, receipt_row, route_path, row, scroll_area, scroll_area_px, section,
    select_node, skeleton, slider_node, spacer, tab_labels, table_labels, tabs_node, text,
    text_area_node, thread_row, toast, toggle_node, tooltip, transaction_node, tree_item,
};
use paint::{
    component_label_width, contact_initial, draw_contact_row, draw_message, draw_pill, panel,
    push_bounded_label, push_label, soft_card,
};
pub use painter::UiPainter;
#[cfg(feature = "fontdue-text")]
pub use pocketbase_admin::{
    POCKETBASE_ADMIN_ACME_ID, POCKETBASE_ADMIN_BACKUPS_ID, POCKETBASE_ADMIN_COLLECTIONS_ID,
    POCKETBASE_ADMIN_CRONS_ID, POCKETBASE_ADMIN_DNS_ID, POCKETBASE_ADMIN_HEALTH_ID,
    POCKETBASE_ADMIN_IMPORT_ID, POCKETBASE_ADMIN_LOGS_ID, POCKETBASE_ADMIN_NEW_RECORD_ID,
    POCKETBASE_ADMIN_OPTIONS_ID, POCKETBASE_ADMIN_SETTINGS_ID, UiPocketBaseAdminState,
    UiPocketBaseCollection, UiPocketBaseLog, UiPocketBaseRecord, build_pocketbase_admin_scene,
};
pub use preset_code::{
    PRESET_CODE_ALPHABET, UiPresetRecipe, decode_preset_code, encode_preset_code, is_preset_code,
    preset_recipe_for_style_family,
};
pub use primitives::{
    ButtonStyle, UiControlAccessory, UiRect, UiTrustProjectionRow, UiWorkProjection,
    UnifiedChatState, UnifiedContact, UnifiedContactKind, UnifiedMessage,
};
pub use runtime::{
    GpuHit, HitKind, UiAction, UiDragSession, UiEvent, UiFrameInput, UiFrameOutput, UiFrameReplay,
    UiKey, UiKeyModifiers, UiRuntimeState, UiTextBuffer, UiTextBufferAction,
};
#[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
pub use scene::IconQuad;
pub use scene::{
    Color4, GpuClip, GpuDragSource, GpuDropTarget, GpuFrameTimeBudget, GpuRect, GpuScene,
    GpuSceneBudget, GpuSceneBudgetViolation, GpuSceneCursor, GpuSceneStats, RectMode,
    UiColorScheme, UiTransitionEasing, UiTransitionProperty, UiTransitionSpec,
};
#[cfg(all(feature = "sdl", not(target_arch = "wasm32")))]
pub use sdl::{SdlGlWindowOptions, instantiate_sdl_app};
pub use shadcn_demo_catalog::{
    SHADCN_DEMO_CATEGORIES, SHADCN_DEMO_COMPONENTS, SHADCN_DEMO_STATUSES, UiShadcnDemoCategory,
    UiShadcnDemoSpec, UiShadcnDemoStatus, UiShadcnParityContract, UiShadcnPortCategorySummary,
    UiShadcnPortManifest, UiShadcnPortMapping, UiShadcnPortStatusSummary, UiShadcnResolveKind,
    UiShadcnResolvedDemo, find_shadcn_demo_by_slug, find_shadcn_demo_by_source_component,
    resolve_shadcn_demo_identifier, shadcn_components_missing_parity_contract,
    shadcn_demos_by_category, shadcn_demos_by_edge_builder, shadcn_demos_using_slot,
    shadcn_demos_using_state, shadcn_exact_demo_count, shadcn_exact_parity_count,
    shadcn_native_demo_count, shadcn_parity_contract_for_slug, shadcn_port_manifest,
    shadcn_port_mapping_for_identifier,
};
pub use shadcn_demo_preview::{
    SHADCN_DEMO_PREVIEW_BASE_ID, UiShadcnDemoGalleryState, build_shadcn_component_preview,
    build_shadcn_component_preview_by_identifier,
    build_shadcn_component_preview_by_source_component, build_shadcn_demo_gallery,
    build_shadcn_demo_gallery_with_state, build_shadcn_demo_preview,
    build_shadcn_demo_preview_by_identifier,
};
pub use shadcn_events::{
    UiShadcnEvent, UiShadcnEventContext, UiShadcnEventValue, shadcn_event_from_action,
    shadcn_event_from_action_with_context,
};
pub use shadcn_exact::{
    UiShadcnActivity, UiShadcnBadgeVariant, UiShadcnButtonSize, UiShadcnButtonVariant,
    UiShadcnChatClientAction, UiShadcnChatClientIconAction, UiShadcnChatClientSpec,
    UiShadcnChatRole, UiShadcnComposerNotice, UiShadcnConversationMessage, UiShadcnSessionRow,
    UiShadcnSessionState, UiShadcnStatusTone, shadcn_accordion, shadcn_alert, shadcn_alert_dialog,
    shadcn_aspect_ratio, shadcn_avatar, shadcn_badge, shadcn_breadcrumb, shadcn_button,
    shadcn_button_group, shadcn_calendar, shadcn_card, shadcn_carousel, shadcn_chart,
    shadcn_chat_client, shadcn_chat_client_shell, shadcn_chat_message, shadcn_checkbox,
    shadcn_collapsible, shadcn_combobox, shadcn_command, shadcn_context_menu, shadcn_conversation,
    shadcn_data_table, shadcn_date_picker, shadcn_dialog, shadcn_direction, shadcn_drawer,
    shadcn_dropdown_menu, shadcn_empty, shadcn_field, shadcn_hover_card, shadcn_input,
    shadcn_input_group, shadcn_input_otp, shadcn_item, shadcn_kbd, shadcn_label, shadcn_menubar,
    shadcn_native_select, shadcn_navigation_menu, shadcn_pagination, shadcn_popover,
    shadcn_progress, shadcn_prompt_composer, shadcn_radio_group, shadcn_resizable,
    shadcn_scroll_area, shadcn_select, shadcn_separator, shadcn_session_sidebar, shadcn_sheet,
    shadcn_sidebar, shadcn_skeleton, shadcn_slider, shadcn_sonner, shadcn_status_header,
    shadcn_switch, shadcn_table, shadcn_tabs, shadcn_textarea, shadcn_toast, shadcn_toggle,
    shadcn_toggle_group, shadcn_tooltip,
};
#[cfg(feature = "fontdue-text")]
pub use shadcn_exact::{shadcn_chat_message_height, shadcn_chat_message_height_for_role};
pub use shadcn_props::{
    SHADCN_PROPS_SURFACE_MANIFEST, SHADCN_PROPS_SURFACES, UiShadcnAccordionProps,
    UiShadcnAlertDialogProps, UiShadcnAlertProps, UiShadcnAspectRatioProps, UiShadcnAvatarProps,
    UiShadcnBadgeProps, UiShadcnBreadcrumbProps, UiShadcnButtonGroupProps, UiShadcnButtonProps,
    UiShadcnCalendarProps, UiShadcnCardProps, UiShadcnCarouselProps, UiShadcnChartProps,
    UiShadcnCheckboxProps, UiShadcnCollapsibleProps, UiShadcnComboboxProps, UiShadcnCommandProps,
    UiShadcnContextMenuProps, UiShadcnDataTableProps, UiShadcnDatePickerProps, UiShadcnDialogProps,
    UiShadcnDirectionProps, UiShadcnDrawerProps, UiShadcnDropdownMenuProps, UiShadcnEmptyProps,
    UiShadcnFieldProps, UiShadcnHoverCardProps, UiShadcnInputGroupProps, UiShadcnInputOtpProps,
    UiShadcnInputProps, UiShadcnItemProps, UiShadcnKbdProps, UiShadcnLabelProps,
    UiShadcnMenubarProps, UiShadcnNativeSelectProps, UiShadcnNavigationMenuProps,
    UiShadcnPaginationProps, UiShadcnPopoverProps, UiShadcnProgressProps, UiShadcnPropsSurface,
    UiShadcnPropsSurfaceManifest, UiShadcnPropsSurfaceSummary, UiShadcnRadioGroupProps,
    UiShadcnRadioOption, UiShadcnResizableProps, UiShadcnResolvedPropsSurface,
    UiShadcnScrollAreaProps, UiShadcnSelectOption, UiShadcnSelectProps, UiShadcnSeparatorProps,
    UiShadcnSheetProps, UiShadcnSidebarProps, UiShadcnSkeletonProps, UiShadcnSliderProps,
    UiShadcnSonnerProps, UiShadcnSwitchProps, UiShadcnTableProps, UiShadcnTabsProps,
    UiShadcnTextareaProps, UiShadcnToastProps, UiShadcnToggleGroupProps, UiShadcnToggleProps,
    UiShadcnTooltipProps, resolve_shadcn_props_surface, shadcn_event_adapted_props_surfaces,
    shadcn_props_surface_count, shadcn_props_surface_for_slug,
    shadcn_props_surface_for_source_component, shadcn_props_surface_summary,
    shadcn_resolved_props_surfaces, shadcn_stateful_props_surfaces, shadcn_static_props_surfaces,
};
pub use shell::{
    SHELL_LAUNCHER_BUTTON_ID, UiShellAction, UiShellFrameOutput, UiShellState,
    UiShellWorkspaceAction,
};
pub use source_captures::{EXTRACTED_SOURCE_CAPTURES, UiExtractedSourceCapture};
pub use spacing::UiSpacing;
pub use style::{AlignItems, Axis, JustifyContent, UiColorToken, UiStyle, UiStyleColor};
pub use style_family::{
    EXTRACTED_STYLE_TOKEN_KINDS, EXTRACTED_STYLE_TOKENS, STYLE_FAMILY_SPECS, UiExtractedStyleToken,
    UiExtractedStyleTokenKind, UiStyleFamilySpec, colors_for_style_family,
};
#[cfg(feature = "fontdue-text")]
pub use text::{FontAtlas, TextQuad};
pub use theme::{
    UI_STYLE_FAMILIES, UiAccentPreset, UiComponentPreviewState, UiDensity, UiRadiusPreset,
    UiRadiusScale, UiResolvedTheme, UiSemanticColors, UiStyleAuthority, UiStyleFamily,
    UiStylePreset, accent_color, style_family_from_name,
};
pub use webgl2::{
    GPU_SCENE_ABI_VERSION, HIT_FLOAT_STRIDE, ICON_VERTEX_FLOAT_STRIDE, PackedGpuScene,
    PackedGpuSceneStats, RECT_FLOAT_STRIDE, TEXT_VERTEX_FLOAT_STRIDE, UiRendererBackend,
    UiRendererCapabilities, UiWebGlSurfaceHost, hit_kind_code, instantiate_webgl_app,
    rect_mode_code,
};
#[cfg(test)]
use workspace::WORKSPACE_CHROME_H;
pub use workspace::{
    UiTileAxis, UiTileNode, UiWorkspace, UiWorkspaceAction, UiWorkspaceFrameOutput,
    UiWorkspaceSurface, UiWorkspaceSurfaceKind,
};

/// Canonical imports for application crates.
///
/// App crates should build one `UiSurfaceApp` and instantiate it through the
/// provided host helpers. Lower-level SDL/WebGL renderer APIs are framework
/// internals and should not be used for product UI.
pub mod prelude {
    pub use super::{
        ButtonStyle, Color4, HitKind, UiAction, UiAppControl, UiIcon, UiNode, UiRect, UiSurfaceApp,
        UiSurfaceHost, button, column, grid_auto_for_width, instantiate_webgl_app,
        masonry_auto_for_width, row, scroll_area, text,
    };
    #[cfg(all(feature = "sdl", not(target_arch = "wasm32")))]
    pub use super::{SdlGlWindowOptions, instantiate_sdl_app};
}

#[derive(Clone, Copy, Debug)]
pub struct ChatShellMetrics {
    pub sidebar_w: f32,
    pub topbar_h: f32,
    pub composer_h: f32,
    pub pad: f32,
}

impl Default for ChatShellMetrics {
    fn default() -> Self {
        Self {
            sidebar_w: 284.0,
            topbar_h: 58.0,
            composer_h: 86.0,
            pad: 18.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_node_builder_renders_component_tree() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-panel border rounded-md p-4 gap-3")
                .children([
                    tab_labels(&["Routes", "Proofs", "Files"], 1, 80).class("h-9"),
                    row("gap-2 h-10 items-center")
                        .child(avatar_node("EdgeRun", palette::ACCENT).online(true))
                        .child(text("Dashboard").class("flex-1 text-text truncate"))
                        .child(badge("sealed", palette::GREEN)),
                    metric("Relay balance", "$0.00")
                        .detail("pending setup")
                        .progress(0.32)
                        .class("h-32"),
                    field_node("Endpoint", "nodes.edgerun.tech")
                        .detail("identity-routed relay")
                        .focused(true),
                    list_row_node("Admission route", "policy-bound relay", 83)
                        .accent(palette::GREEN),
                    control_row_node("Relay enabled")
                        .detail("use admitted identity route")
                        .control_toggle(true, 84)
                        .hit_id(85),
                    menu_item_node("Payments", 42)
                        .detail("proof-backed receipts")
                        .badge_text("new")
                        .selected(true),
                    button("Open", 43, ButtonStyle::Secondary).class("h-8"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 360.0, 620.0));
        }

        assert!(scene.rects().len() > 20);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Tab && hit.id == 81)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 83)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Toggle && hit.id == 84)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 85)
        );
    }

    #[test]
    fn ui_node_builder_renders_dashboard_primitives() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            let months = ["Jan", "Feb", "Mar", "Apr"];
            let values = [0.42, 0.72, 0.55, 0.91];
            column("bg-panel border rounded-md p-4 gap-3")
                .children([
                    header("Contribution History")
                        .detail("Last 4 months")
                        .action("View", 50),
                    bar_chart_labels("Relay Receipts", &months, &values).class("h-44"),
                    slider_node("Payout threshold", 0.62, 51)
                        .range_labels("$50", "$10,000")
                        .accent(palette::GREEN),
                    text_area_node("Notes", "Route budget and admission notes").focused(true),
                    transaction_node("Stripe payout", "+$4,200.00", 52)
                        .detail("Income")
                        .date("Today")
                        .positive(true),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 420.0, 620.0));
        }

        assert!(scene.rects().len() > 35);
        assert!(scene.hits().len() >= 3);
    }

    #[test]
    fn gpu_ui_macro_composes_nested_trees() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            gpu_ui!(
                column("bg-panel border rounded-md p-4 gap-3"),
                [
                    gpu_ui!(
                        row("gap-2 h-10 items-center"),
                        [
                            avatar_node("EdgeRun", palette::ACCENT).online(true),
                            text("Components").class("flex-1 text-text truncate"),
                            badge("rust", palette::ACCENT),
                            icon_button(UiIcon::ChevronRight, 69).class("size-8")
                        ]
                    ),
                    metric("Storage", "128 MB").detail("verified cache"),
                    progress_bar_node(0.58, palette::GREEN).class("h-2"),
                    toggle_node(true, 68),
                    button("Run", 70, ButtonStyle::Primary).class("h-8")
                ]
            )
            .render(&mut ui, UiRect::new(0.0, 0.0, 320.0, 320.0));
        }

        assert!(scene.rects().len() > 25);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Toggle && hit.id == 68)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 69)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 70)
        );
    }

    #[test]
    fn disabled_and_loading_nodes_suppress_interaction_hits() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("gap-2")
                .children([
                    button("Disabled", 71, ButtonStyle::Secondary)
                        .class("h-8")
                        .disabled(true),
                    button("Loading", 72, ButtonStyle::Primary)
                        .class("h-8")
                        .loading(true),
                    button("Ready", 73, ButtonStyle::Primary).class("h-8"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 240.0, 130.0));
        }

        assert!(!scene.hits().iter().any(|hit| hit.id == 71));
        assert!(!scene.hits().iter().any(|hit| hit.id == 72));
        assert!(scene.hits().iter().any(|hit| hit.id == 73));
        assert!(scene.rects().len() > 10);
    }

    #[test]
    fn keyboard_focus_cycles_and_activates_controls() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("gap-2")
                .children([
                    button("Run", 81, ButtonStyle::Primary).class("h-8"),
                    checkbox("Cache verified bytes", false, 82).class("h-8"),
                    field_node("Filter", "").hit_id(83).class("h-16"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 260.0, 150.0));
        }

        let mut runtime = UiRuntimeState::default();
        assert!(matches!(
            runtime.focus_first(&scene),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Button && hit.id == 81
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Enter }),
            UiAction::Activated(hit) if hit.kind == HitKind::Button && hit.id == 81
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Tab }),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Checkbox && hit.id == 82
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Enter }),
            UiAction::Toggled { id: 82, on: true }
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Tab }),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Input && hit.id == 83
        ));
    }

    #[test]
    fn scroll_area_only_renders_visible_rows() {
        let rows = (0..10).map(|index| {
            list_row_node(
                &format!("Route {}", index + 1),
                "identity-routed relay",
                100 + index,
            )
        });

        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            scroll_area("bg-panel border rounded-md p-2 gap-2", 1.0)
                .scroll_id(99)
                .children(rows)
                .render(&mut ui, UiRect::new(0.0, 0.0, 320.0, 160.0));
        }

        let visible_rows = scene
            .hits()
            .iter()
            .filter(|hit| hit.kind == HitKind::ListRow)
            .count();
        assert!((1..10).contains(&visible_rows));
        assert!(
            !scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 100)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 109)
        );
        assert!(
            scene
                .hits()
                .iter()
                .filter(|hit| hit.kind == HitKind::ListRow)
                .all(|hit| hit.y >= 8.0 && hit.y + hit.h <= 152.0)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Scrollbar && hit.id == 99)
        );
    }

    #[test]
    fn scene_clip_stack_clips_rects_and_hits() {
        let mut scene = GpuScene::new(palette::BG);
        assert!(scene.push_clip(GpuClip::new(10.0, 20.0, 80.0, 40.0)));
        scene.push_rect(GpuRect::fill(0.0, 0.0, 40.0, 40.0, 8.0, palette::ACCENT));
        scene.push_hit(GpuHit::new(HitKind::Button, 7, 0.0, 0.0, 40.0, 40.0));
        scene.push_rect(GpuRect::fill(120.0, 0.0, 20.0, 20.0, 0.0, palette::ACCENT));
        scene.pop_clip();

        assert_eq!(scene.rects().len(), 1);
        assert_eq!(
            (
                scene.rects()[0].x,
                scene.rects()[0].y,
                scene.rects()[0].w,
                scene.rects()[0].h
            ),
            (10.0, 20.0, 30.0, 20.0)
        );
        assert_eq!(scene.hits().len(), 1);
        assert_eq!(
            (
                scene.hits()[0].x,
                scene.hits()[0].y,
                scene.hits()[0].w,
                scene.hits()[0].h
            ),
            (10.0, 20.0, 30.0, 20.0)
        );
    }

    #[test]
    fn ui_runtime_state_emits_semantic_actions() {
        let mut scene = GpuScene::new(palette::BG);
        scene.push_hit(GpuHit::new(HitKind::Toggle, 4, 10.0, 10.0, 46.0, 24.0));
        scene.push_hit(GpuHit::new(HitKind::Input, 7, 10.0, 44.0, 160.0, 34.0));
        scene.push_hit(GpuHit::new(HitKind::Scrollbar, 9, 180.0, 10.0, 8.0, 120.0));
        let mut state = UiRuntimeState::default();

        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerDown { x: 12.0, y: 12.0 }),
            UiAction::Activated(GpuHit::new(HitKind::Toggle, 4, 10.0, 10.0, 46.0, 24.0))
        );
        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerUp { x: 12.0, y: 12.0 }),
            UiAction::Toggled { id: 4, on: true }
        );

        assert!(matches!(
            state.handle_event(&scene, UiEvent::PointerDown { x: 20.0, y: 50.0 }),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Input && hit.id == 7
        ));
        assert_eq!(
            state.handle_event(&scene, UiEvent::TextInput("ok".to_string())),
            UiAction::TextChanged {
                id: 7,
                value: "ok".to_string()
            }
        );
        assert_eq!(
            state.handle_event(
                &scene,
                UiEvent::KeyDown {
                    key: UiKey::Backspace,
                },
            ),
            UiAction::TextChanged {
                id: 7,
                value: "o".to_string()
            }
        );

        assert_eq!(
            state.handle_event(
                &scene,
                UiEvent::Wheel {
                    x: 182.0,
                    y: 20.0,
                    delta_y: 180.0,
                },
            ),
            UiAction::ScrollChanged { id: 9, offset: 0.2 }
        );
    }

    #[test]
    fn ui_node_render_consumes_runtime_scroll_state() {
        let rows = (0..8).map(|index| {
            list_row_node(
                &format!("Runtime row {}", index + 1),
                "state owned by rust",
                130 + index,
            )
        });
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(77, 1.0);

        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            scroll_area("bg-panel border rounded-md p-2 gap-2", 0.0)
                .scroll_id(77)
                .children(rows)
                .render_with_state(&mut ui, UiRect::new(0.0, 0.0, 320.0, 150.0), Some(&runtime));
        }

        assert!(
            !scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 130)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 137)
        );
    }

    #[test]
    fn workspace_tiles_surfaces_without_overlap_and_clips_surfaces() {
        let mut workspace = UiWorkspace::split(
            UiTileAxis::Horizontal,
            UiWorkspaceSurface::new(1, "Chat"),
            UiWorkspaceSurface::new(2, "Trust"),
        );
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            workspace.render(
                &mut ui,
                UiRect::new(0.0, 0.0, 640.0, 360.0),
                |ui, bounds, surface| {
                    ui.fill_rect(bounds, 0.0, palette::PANEL);
                    ui.bounded_label(
                        bounds.x + 12.0,
                        bounds.y + 14.0,
                        bounds.w - 24.0,
                        &surface.title,
                        2.0,
                        palette::TEXT,
                    );
                    ui.hit(
                        HitKind::Button,
                        surface.id + 100,
                        bounds.x,
                        bounds.y,
                        bounds.w,
                        bounds.h,
                    );
                },
            );
        }

        let first = workspace
            .surface(1)
            .and_then(UiWorkspaceSurface::bounds)
            .unwrap();
        let second = workspace
            .surface(2)
            .and_then(UiWorkspaceSurface::bounds)
            .unwrap();
        assert!(first.x + first.w <= second.x);
        assert!(first.h <= 360.0 - WORKSPACE_CHROME_H);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::WorkspaceTab && hit.id == 1)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::WorkspaceClose && hit.id == 2)
        );
        assert!(
            scene
                .hits()
                .iter()
                .filter(|hit| hit.kind == HitKind::Button)
                .all(|hit| hit.y >= WORKSPACE_CHROME_H)
        );
    }

    #[test]
    fn workspace_user_style_themes_chrome_without_changing_surface_contract() {
        let mut style = UiComponentPreviewState::default();
        style.authority = UiStyleAuthority::AuthorVision;
        let theme = style.resolved_theme();
        let mut workspace =
            UiWorkspace::single(UiWorkspaceSurface::new(7, "Chat")).user_style(style);
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            workspace.render(
                &mut ui,
                UiRect::new(0.0, 0.0, 320.0, 220.0),
                |ui, bounds, surface| {
                    ui.hit(
                        HitKind::Button,
                        surface.id + 100,
                        bounds.x,
                        bounds.y,
                        bounds.w,
                        bounds.h,
                    );
                },
            );
        }

        assert!(
            scene
                .rects()
                .iter()
                .any(|rect| rect.color == theme.colors.topbar && rect.radius == theme.radius.card)
        );
        assert!(
            scene
                .rects()
                .iter()
                .any(|rect| rect.color == theme.colors.accent.with_alpha(0.68))
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::WorkspaceTab && hit.id == 7)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 107)
        );
    }

    #[test]
    fn workspace_routes_pointer_events_to_surface_runtime() {
        let mut workspace = UiWorkspace::single(UiWorkspaceSurface::new(7, "Chat"));
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            workspace.render(
                &mut ui,
                UiRect::new(0.0, 0.0, 320.0, 220.0),
                |ui, bounds, _surface| {
                    ui.hit(
                        HitKind::Toggle,
                        55,
                        bounds.x + 20.0,
                        bounds.y + 20.0,
                        46.0,
                        24.0,
                    );
                    ui.toggle(bounds.x + 20.0, bounds.y + 20.0, false, 55);
                },
            );
        }

        let down = workspace.handle_event(&scene, UiEvent::PointerDown { x: 24.0, y: 58.0 });
        assert!(matches!(
            down,
            UiWorkspaceAction::SurfaceAction {
                surface_id: 7,
                action: UiAction::Activated(_)
            }
        ));
        let up = workspace.handle_event(&scene, UiEvent::PointerUp { x: 24.0, y: 58.0 });
        assert_eq!(
            up,
            UiWorkspaceAction::SurfaceAction {
                surface_id: 7,
                action: UiAction::Toggled { id: 55, on: true }
            }
        );
        assert!(
            workspace
                .surface(7)
                .is_some_and(|surface| surface.runtime.toggle_value(55, false))
        );
    }

    #[test]
    fn full_screen_surface_replaces_workspace_chrome() {
        let mut workspace = UiWorkspace::full_screen(UiWorkspaceSurface::new(10, "Full screen"));
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            workspace.render(
                &mut ui,
                UiRect::new(0.0, 0.0, 640.0, 360.0),
                |ui, bounds, surface| {
                    ui.fill_rect(bounds, 0.0, palette::PANEL);
                    ui.hit(
                        HitKind::Button,
                        surface.id + 100,
                        bounds.x,
                        bounds.y,
                        bounds.w,
                        bounds.h,
                    );
                },
            );
        }

        let bounds = workspace
            .surface(10)
            .and_then(UiWorkspaceSurface::bounds)
            .unwrap();
        assert_eq!(bounds, UiRect::new(0.0, 0.0, 640.0, 360.0));
        assert!(
            !scene
                .hits()
                .iter()
                .any(|hit| matches!(hit.kind, HitKind::WorkspaceTab | HitKind::WorkspaceClose))
        );
    }

    #[test]
    fn component_preview_style_defaults_to_user_authority() {
        let preview = UiComponentPreviewState::default();

        assert_eq!(preview.authority, UiStyleAuthority::User);
        assert_eq!(preview.active_preset(), preview.user_preset);
        assert!(preview.author_available());
    }

    #[test]
    fn resolved_theme_changes_presentation_without_changing_component_hits() {
        let user_theme = UiResolvedTheme::default();
        let author_theme = UiResolvedTheme::from_preset(
            UiStyleAuthority::AuthorVision,
            UiStylePreset::author_vision(),
        );
        let app_tree = button("Run", 900, ButtonStyle::Primary).class("h-10 w-24");

        let mut user_scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut user_scene).with_theme(user_theme);
            app_tree.render(&mut ui, UiRect::new(0.0, 0.0, 120.0, 48.0));
        }

        let mut author_scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut author_scene).with_theme(author_theme);
            app_tree.render(&mut ui, UiRect::new(0.0, 0.0, 120.0, 48.0));
        }

        assert_eq!(user_scene.hits(), author_scene.hits());
        assert!(
            user_scene
                .rects()
                .iter()
                .any(|rect| rect.color == user_theme.colors.accent)
        );
        assert!(
            author_scene
                .rects()
                .iter()
                .any(|rect| rect.color == author_theme.colors.accent)
        );
        assert_ne!(user_theme.colors.accent, author_theme.colors.accent);
    }

    #[test]
    fn painter_primitives_resolve_through_active_theme() {
        let user_theme = UiResolvedTheme::default();
        let author_theme = UiResolvedTheme::from_preset(
            UiStyleAuthority::AuthorVision,
            UiStylePreset::author_vision(),
        );

        let mut user_scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut user_scene).with_theme(user_theme);
            ui.checkbox(UiRect::new(0.0, 0.0, 180.0, 28.0), "Cache", true, 1);
            ui.command_palette(UiRect::new(0.0, 36.0, 220.0, 38.0), "Search", 2);
            ui.list_row(
                UiRect::new(0.0, 84.0, 220.0, 58.0),
                "Admission",
                "policy route",
                user_theme.colors.accent,
                3,
            );
            ui.composer(0.0, 152.0, 280.0, 100.0, "Message", true, &[]);
        }

        let mut author_scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut author_scene).with_theme(author_theme);
            ui.checkbox(UiRect::new(0.0, 0.0, 180.0, 28.0), "Cache", true, 1);
            ui.command_palette(UiRect::new(0.0, 36.0, 220.0, 38.0), "Search", 2);
            ui.list_row(
                UiRect::new(0.0, 84.0, 220.0, 58.0),
                "Admission",
                "policy route",
                author_theme.colors.accent,
                3,
            );
            ui.composer(0.0, 152.0, 280.0, 100.0, "Message", true, &[]);
        }

        assert_eq!(user_scene.hits(), author_scene.hits());
        assert!(
            user_scene
                .rects()
                .iter()
                .any(|rect| rect.color == user_theme.colors.composer)
        );
        assert!(
            author_scene
                .rects()
                .iter()
                .any(|rect| rect.color == author_theme.colors.composer)
        );
        assert!(
            user_scene
                .rects()
                .iter()
                .any(|rect| rect.color == user_theme.colors.accent)
        );
        assert!(
            author_scene
                .rects()
                .iter()
                .any(|rect| rect.color == author_theme.colors.accent)
        );
    }

    #[test]
    fn semantic_style_classes_resolve_through_active_theme() {
        let user_theme = UiResolvedTheme::default();
        let author_theme = UiResolvedTheme::from_preset(
            UiStyleAuthority::AuthorVision,
            UiStylePreset::author_vision(),
        );
        let app_tree = column("bg-panel border rounded-md p-2")
            .child(text("Accent label").class("text-accent"));

        let mut user_scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut user_scene).with_theme(user_theme);
            app_tree.render(&mut ui, UiRect::new(0.0, 0.0, 180.0, 60.0));
        }

        let mut author_scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut author_scene).with_theme(author_theme);
            app_tree.render(&mut ui, UiRect::new(0.0, 0.0, 180.0, 60.0));
        }

        assert!(
            user_scene
                .rects()
                .iter()
                .any(|rect| rect.color == user_theme.colors.panel)
        );
        assert!(
            author_scene
                .rects()
                .iter()
                .any(|rect| rect.color == author_theme.colors.panel)
        );
        assert!(
            user_scene
                .rects()
                .iter()
                .any(|rect| rect.color == user_theme.colors.accent)
        );
        assert!(
            author_scene
                .rects()
                .iter()
                .any(|rect| rect.color == author_theme.colors.accent)
        );
    }

    #[cfg(feature = "tabler-svg-atlas")]
    #[test]
    fn canonical_icons_resolve_tabler_svg_atlas_rects() {
        let atlas = tabler_svg_icon_atlas();
        assert_eq!(
            atlas.width,
            crate::tabler_svg_atlas_generated::TABLER_SVG_ATLAS_W
        );
        assert_eq!(atlas.alpha.len(), (atlas.width * atlas.height) as usize);
        let rect = UiIcon::Trust.tabler_svg_atlas_rect();
        assert_eq!(rect.name, "shield-check");
        assert!(rect.u0 >= 0.0 && rect.u1 <= 1.0 && rect.u0 < rect.u1);
        assert!(rect.v0 >= 0.0 && rect.v1 <= 1.0 && rect.v0 < rect.v1);
    }

    #[cfg(feature = "lucide-svg-atlas")]
    #[test]
    fn canonical_icons_resolve_lucide_svg_atlas_rects() {
        let atlas = lucide_svg_icon_atlas();
        assert_eq!(
            atlas.width,
            crate::lucide_svg_atlas_generated::LUCIDE_SVG_ATLAS_W
        );
        assert_eq!(atlas.alpha.len(), (atlas.width * atlas.height) as usize);
        let rect = UiIcon::Trust.lucide_svg_atlas_rect();
        assert_eq!(rect.name, "shield-check");
        assert!(rect.u0 >= 0.0 && rect.u1 <= 1.0 && rect.u0 < rect.u1);
        assert!(rect.v0 >= 0.0 && rect.v1 <= 1.0 && rect.v0 < rect.v1);
    }

    #[test]
    fn grid_node_places_spanned_dashboard_cards() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            let labels = ["A", "B", "C"];
            let values = [0.25, 0.5, 0.75];
            grid("bg-panel border rounded-md p-3 gap-3", 4)
                .children([
                    metric("Balance", "$42.00")
                        .progress(0.4)
                        .span(2)
                        .class("h-32"),
                    field_node("Relay", "nodes.edgerun.tech")
                        .focused(true)
                        .span(2),
                    bar_chart_labels("Activity", &labels, &values)
                        .span(4)
                        .class("h-44"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 640.0, 420.0));
        }

        assert!(scene.rects().len() > 30);
        assert!(scene.hits().is_empty());
    }

    #[test]
    fn grid_auto_reads_columns_from_classes() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid_auto("grid grid-cols-3 bg-panel border rounded-md p-3 gap-3")
                .children([
                    metric("One", "1").class("h-24"),
                    metric("Two", "2").class("h-24"),
                    metric("Three", "3").class("h-24"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 480.0, 160.0));
        }

        assert!(scene.rects().len() > 20);
    }

    #[test]
    fn grid_auto_for_width_applies_responsive_columns() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid_auto_for_width(
                "grid grid-cols-1 md:grid-cols-3 bg-panel p-3 gap-2 md:gap-3",
                900.0,
            )
            .children([
                metric("One", "1").class("h-24"),
                metric("Two", "2").class("h-24"),
                metric("Three", "3").class("h-24"),
            ])
            .render(&mut ui, UiRect::new(0.0, 0.0, 480.0, 160.0));
        }

        assert!(scene.rects().len() > 20);
    }

    #[test]
    fn alignment_classes_control_child_placement() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            row("bg-panel border rounded-md p-2 items-center justify-between")
                .children([
                    text("Left").class("w-12"),
                    button("Right", 80, ButtonStyle::Secondary).class("w-20 h-8"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 320.0, 80.0));
        }

        let hit = scene
            .hits()
            .iter()
            .find(|hit| hit.id == 80)
            .expect("button hit");
        assert!(hit.x > 220.0);
        assert!((hit.y - 24.0).abs() < 0.1);
    }

    #[test]
    fn full_cross_axis_size_survives_non_stretch_alignment() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            row("bg-panel border rounded-md p-2 items-end")
                .children([
                    text_area_node("", "full height")
                        .hit_id(82)
                        .class("h-full flex-1"),
                    icon_button(UiIcon::Send, 83).class("size-9"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 360.0, 96.0));
        }

        let hit = scene
            .hits()
            .iter()
            .find(|hit| hit.kind == HitKind::TextArea && hit.id == 82)
            .expect("textarea hit");
        assert!(hit.h > 70.0, "hit: {hit:?}");
    }

    #[test]
    fn column_children_stretch_by_default() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-panel border rounded-md p-2 gap-2")
                .child(button("Wide", 81, ButtonStyle::Secondary).class("h-8"))
                .render(&mut ui, UiRect::new(0.0, 0.0, 220.0, 80.0));
        }

        let hit = scene
            .hits()
            .iter()
            .find(|hit| hit.id == 81)
            .expect("button hit");
        assert!(hit.w > 190.0);
    }

    #[test]
    fn conditional_children_do_not_allocate_placeholder_layout() {
        let mut hidden = row("gap-2")
            .when(false, text("hidden"))
            .child(text("visible"));
        let shown = row("gap-2").when(true, text("visible"));

        assert_eq!(hidden.children.len(), 1);
        assert_eq!(shown.children.len(), 1);
        hidden = hidden.child(text("second"));
        assert_eq!(hidden.children.len(), 2);
    }

    #[test]
    fn canonical_icons_map_to_replaceable_provider_names() {
        assert_eq!(UiIconSet::default(), UiIconSet::Tabler);
        assert_eq!(UiIcon::Trust.name(), "trust");
        assert_eq!(
            UiIcon::Trust.provider_name(UiIconSet::Tabler),
            "shield-check"
        );
        assert_eq!(
            UiIcon::Trust.provider_name(UiIconSet::Lucide),
            "shield-check"
        );
        assert_eq!(
            UiIcon::Terminal.provider_name(UiIconSet::Tabler),
            "terminal-2"
        );
        assert_eq!(
            UiIcon::Terminal.provider_name(UiIconSet::Lucide),
            "square-terminal"
        );
        assert_eq!(
            UiIcon::MessagePlus.provider_name(UiIconSet::Tabler),
            "message-plus"
        );
        assert_eq!(UiIcon::Trash.provider_name(UiIconSet::Tabler), "trash");
        assert_eq!(UiIcon::Send.provider_name(UiIconSet::Tabler), "arrow-up");
    }

    #[cfg(feature = "tabler-svg-atlas")]
    #[test]
    fn canonical_tabler_icons_have_svg_atlas_entries() {
        let icons = [
            UiIcon::Activity,
            UiIcon::App,
            UiIcon::Bell,
            UiIcon::Chat,
            UiIcon::Check,
            UiIcon::ChevronRight,
            UiIcon::Code,
            UiIcon::Cpu,
            UiIcon::Database,
            UiIcon::Eye,
            UiIcon::File,
            UiIcon::Key,
            UiIcon::Lock,
            UiIcon::Menu,
            UiIcon::MessagePlus,
            UiIcon::Network,
            UiIcon::Route,
            UiIcon::Search,
            UiIcon::Send,
            UiIcon::Server,
            UiIcon::Settings,
            UiIcon::Shield,
            UiIcon::Sparkles,
            UiIcon::Storage,
            UiIcon::Terminal,
            UiIcon::Trust,
            UiIcon::Trash,
            UiIcon::User,
            UiIcon::Wallet,
            UiIcon::Warning,
            UiIcon::X,
        ];
        for icon in icons {
            let rect = icon.tabler_svg_atlas_rect();
            assert_eq!(rect.name, icon.provider_name(UiIconSet::Tabler));
            assert!(rect.w > 0);
            assert!(rect.h > 0);
        }
    }

    #[cfg(all(feature = "lucide-svg-atlas", feature = "tabler-svg-atlas"))]
    #[test]
    fn painter_can_render_with_lucide_replacement_pack() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene)
                .with_asset_pack(EDGERUN_LUCIDE_GEIST_ASSET_PACK)
                .expect("valid Lucide/Geist asset pack");
            ui.icon(
                UiRect::new(0.0, 0.0, 24.0, 24.0),
                UiIcon::App,
                palette::TEXT,
            );
        }

        let expected = UiIcon::App.lucide_svg_atlas_rect();
        let quads = scene.icon_quads();
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].u0, expected.u0);
        assert_eq!(quads[0].v0, expected.v0);
        assert_eq!(quads[0].u1, expected.u1);
        assert_eq!(quads[0].v1, expected.v1);
    }

    #[cfg(feature = "tabler-svg-atlas")]
    #[test]
    fn icon_node_prefers_tabler_svg_atlas_quads() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            row("row gap-2")
                .child(icon(UiIcon::MessagePlus).class("size-8"))
                .child(icon_button(UiIcon::Trash, 91).class("size-8"))
                .render(&mut ui, UiRect::new(0.0, 0.0, 120.0, 48.0));
        }

        assert!(scene.icon_quads().len() >= 2);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 91)
        );
    }

    #[test]
    fn canonical_icon_node_renders_gpu_geometry() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            row("row gap-2 items-center")
                .child(icon(UiIcon::Lock).accent(palette::ACCENT).class("size-8"))
                .child(icon_button(UiIcon::X, 91).class("size-8"))
                .render(&mut ui, UiRect::new(0.0, 0.0, 120.0, 48.0));
        }

        #[cfg(feature = "tabler-svg-atlas")]
        assert!(scene.icon_quads().len() >= 2);
        #[cfg(not(feature = "tabler-svg-atlas"))]
        assert!(scene.rects().len() > 8);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 91)
        );
    }

    #[test]
    fn foundation_form_controls_render_and_emit_state() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-panel border rounded-md p-3 gap-2")
                .child(checkbox("Cache verified bytes", false, 201))
                .child(radio("Personal admission", false, 202))
                .child(select_node("Relay endpoint", "assigned by admission", 203))
                .child(tooltip("Only the Trust Container can decrypt."))
                .render(&mut ui, UiRect::new(0.0, 0.0, 360.0, 190.0));
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Checkbox && hit.id == 201)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Radio && hit.id == 202)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Select && hit.id == 203)
        );

        let mut runtime = UiRuntimeState::default();
        let down = runtime.handle_event(&scene, UiEvent::PointerDown { x: 18.0, y: 18.0 });
        assert!(matches!(down, UiAction::Activated(hit) if hit.kind == HitKind::Checkbox));
        assert_eq!(
            runtime.handle_event(&scene, UiEvent::PointerUp { x: 18.0, y: 18.0 }),
            UiAction::Toggled { id: 201, on: true }
        );

        let _ = runtime.handle_event(&scene, UiEvent::PointerDown { x: 18.0, y: 54.0 });
        assert_eq!(
            runtime.handle_event(&scene, UiEvent::PointerUp { x: 18.0, y: 54.0 }),
            UiAction::Toggled { id: 202, on: true }
        );
    }

    #[test]
    fn feedback_components_render_without_custom_surfaces() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid("grid grid-cols-2 bg-bg p-3 gap-3", 2)
                .child(
                    dialog(
                        "Capability request",
                        "Review the admitted action before signing.",
                        UiIcon::Shield,
                    )
                    .class("h-44"),
                )
                .child(empty_state(
                    "No proofs yet",
                    "Runtime events will appear here.",
                    UiIcon::File,
                ))
                .child(toast("Route admitted", UiIcon::Check, palette::GREEN).span(2))
                .child(skeleton().class("h-6"))
                .child(progress_ring(0.64, palette::ACCENT).class("size-12"))
                .render(&mut ui, UiRect::new(0.0, 0.0, 720.0, 420.0));
        }

        assert!(scene.rects().len() > 40);
        assert!(scene.hits().is_empty());
    }

    #[test]
    fn data_navigation_components_render_semantic_hits() {
        let rows: &[&[&str]] = &[
            &["admission", "policy", "ready"],
            &["relay", "route", "pending"],
        ];
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-bg p-3 gap-3")
                .child(breadcrumb(&["Trust", "Routes", "Relay"], 2, 300))
                .child(command_palette("Search commands, apps, proofs...", 310))
                .child(table_labels(&["Node", "Kind", "State"], rows, 320).class("h-32"))
                .child(section("Node instances", "identity + role + policy"))
                .child(tree_item(
                    "admission:personal",
                    "local policy",
                    0,
                    true,
                    330,
                ))
                .child(tree_item("relay:nodes", "websocket", 1, false, 331))
                .render(&mut ui, UiRect::new(0.0, 0.0, 640.0, 420.0));
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Breadcrumb && hit.id == 302)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Input && hit.id == 310)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 320)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::TreeItem && hit.id == 331)
        );
    }

    #[test]
    fn menu_select_command_and_tree_support_keyboard_interaction() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-bg p-3 gap-2")
                .child(menu_item_node("Proof dashboard", 1400).selected(true))
                .child(select_node("Route scope", "relay:private-devices", 1401))
                .child(command_palette("Search proofs...", 1402))
                .child(tree_item(
                    "storage:vps-cache",
                    "node instance",
                    0,
                    true,
                    1403,
                ))
                .render(&mut ui, UiRect::new(0.0, 0.0, 360.0, 240.0));
        }

        let mut runtime = UiRuntimeState::default();
        assert!(matches!(
            runtime.focus_first(&scene),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::MenuItem && hit.id == 1400
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Enter }),
            UiAction::Activated(hit) if hit.kind == HitKind::MenuItem && hit.id == 1400
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Tab }),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Select && hit.id == 1401
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Enter }),
            UiAction::OpenChanged {
                id: 1401,
                open: true
            }
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Tab }),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Input && hit.id == 1402
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::TextInput("route".into())),
            UiAction::TextChanged { id: 1402, value } if value == "route"
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Tab }),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::TreeItem && hit.id == 1403
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Enter }),
            UiAction::Activated(hit) if hit.kind == HitKind::TreeItem && hit.id == 1403
        ));
    }

    #[test]
    fn edgerun_domain_components_render_semantic_rows() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid("grid grid-cols-2 bg-bg p-3 gap-3", 2)
                .child(identity_card(
                    "Ken",
                    "wasm storage node",
                    "personal policy",
                    400,
                ))
                .child(package_card("Chat", "free-run", "b3f2...a91", 401))
                .child(route_path("Admitted route", &["app", "device", "relay", "user"]).span(2))
                .child(contact_card("Codex client", "app contact", 402))
                .child(thread_row("Alice", "Encrypted message", true, 403))
                .child(attachment_preview("photo.jpg", "image payload", 404))
                .child(capability_grant_row(
                    "Chat",
                    "decrypt message",
                    "single use",
                    405,
                ))
                .child(proof_event_row("Package verified", "hash:b3f2", "ok", 406))
                .child(receipt_row("Relay delivery", "+$0.01", "settled", 407))
                .render(&mut ui, UiRect::new(0.0, 0.0, 760.0, 620.0));
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 400)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 406)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::TransactionRow && hit.id == 407)
        );
    }
}
