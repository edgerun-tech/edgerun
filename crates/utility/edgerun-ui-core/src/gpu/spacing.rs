//! Shared spacing and sizing tokens for GPU UI layouts.
//!
//! Keep geometry decisions here when they are reused across shell, workspace,
//! and components. Local one-off measurements can stay near their component
//! until they repeat or become part of a public layout contract.

use super::primitives::UiRect;

pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 6.0;
pub const SPACE_3: f32 = 8.0;
pub const SPACE_4: f32 = 10.0;
pub const SPACE_5: f32 = 12.0;
pub const SPACE_6: f32 = 14.0;
pub const SPACE_7: f32 = 16.0;
pub const SPACE_8: f32 = 18.0;
pub const SPACE_9: f32 = 20.0;
pub const SPACE_10: f32 = 22.0;
pub const SPACE_11: f32 = 24.0;
pub const SPACE_12: f32 = 32.0;
pub const SPACE_13: f32 = 40.0;
pub const SPACE_14: f32 = 48.0;
pub const SPACE_15: f32 = 56.0;
pub const SPACE_16: f32 = 64.0;

pub const CARD_RADIUS_MAX: f32 = SPACE_3;
pub const CARD_PAD_X: f32 = SPACE_7;
pub const CARD_PAD_Y: f32 = SPACE_6;
pub const COMPONENT_PAD_X_DENSE: f32 = SPACE_5;
pub const COMPONENT_PAD_Y_DENSE: f32 = SPACE_4;
pub const COMPONENT_PAD_X: f32 = CARD_PAD_X;
pub const COMPONENT_PAD_Y: f32 = CARD_PAD_Y;
pub const COMPONENT_PAD_X_SPACIOUS: f32 = SPACE_9;
pub const COMPONENT_PAD_Y_SPACIOUS: f32 = SPACE_8;
pub const CONTROL_PAD_X: f32 = SPACE_6;
pub const COMPACT_CONTROL_H: f32 = SPACE_12;
pub const CONTROL_H: f32 = SPACE_13;
pub const LARGE_CONTROL_H: f32 = SPACE_14;
pub const BUTTON_H_COMPACT: f32 = COMPACT_CONTROL_H;
pub const BUTTON_H: f32 = CONTROL_H;
pub const BUTTON_H_LARGE: f32 = LARGE_CONTROL_H;
pub const ICON_BUTTON_COMPACT: f32 = SPACE_12;
pub const ICON_BUTTON: f32 = CONTROL_H;
pub const ICON_BUTTON_LARGE: f32 = LARGE_CONTROL_H;
pub const TOOLBAR_CONTROL: f32 = CONTROL_H;
pub const TOOLBAR_ICON_BUTTON: f32 = ICON_BUTTON;
pub const FORM_LABEL_H: f32 = SPACE_9;
pub const FORM_LABEL_GAP: f32 = SPACE_1;
pub const FORM_FIELD_Y: f32 = FORM_LABEL_H + FORM_LABEL_GAP;
pub const FORM_HELPER_GAP: f32 = SPACE_2;
pub const FORM_HELPER_Y: f32 = FORM_FIELD_Y + CONTROL_H + FORM_HELPER_GAP;
pub const TEXT_AREA_PAD_Y: f32 = SPACE_5;
pub const TEXT_AREA_LINE_H: f32 = SPACE_10;
pub const TEXT_AREA_RESERVED_Y: f32 = FORM_FIELD_Y;
pub const ROW_PAD_X: f32 = SPACE_6;
pub const ROW_ICON: f32 = 34.0;
pub const ROW_ICON_GAP: f32 = SPACE_5;
pub const ROW_TEXT_INSET: f32 = ROW_PAD_X + ROW_ICON + ROW_ICON_GAP;
pub const ROW_H: f32 = 58.0;
pub const LIST_ROW_H: f32 = ROW_H;
pub const MENU_ROW_H: f32 = SPACE_14;
pub const COMMAND_ROW_H: f32 = SPACE_14;
pub const TABLE_ROW_H: f32 = SPACE_16;
pub const TABLE_CELL_PAD_X: f32 = SPACE_5;
pub const TABLE_CELL_PAD_Y: f32 = SPACE_3;
pub const OPERATION_ROW_H: f32 = 78.0;
pub const OPERATION_ROW_CONTENT_H: f32 = OPERATION_ROW_H - SPACE_1 * 0.5;
pub const OPERATION_ROW_PANEL_PAD: f32 = SPACE_2;
pub const PACKAGE_CARD_H: f32 = 112.0;
pub const APP_STORE_CARD_H: f32 = 138.0;
pub const APP_CARD_GRID_GAP: f32 = SPACE_4;
pub const NARROW_VIEWPORT_W: f32 = 520.0;
pub const WIDE_VIEWPORT_W: f32 = 1180.0;
pub const APP_SURFACE_INSET_X_NARROW: f32 = SPACE_4;
pub const APP_SURFACE_INSET_Y_NARROW: f32 = SPACE_4;
pub const APP_SURFACE_INSET_X: f32 = SPACE_6;
pub const APP_SURFACE_INSET_Y: f32 = SPACE_6;
pub const APP_SURFACE_INSET_X_WIDE: f32 = SPACE_7;
pub const APP_SURFACE_INSET_Y_WIDE: f32 = SPACE_7;
pub const SHELL_VIEWPORT_INSET: f32 = SPACE_4;
pub const SHELL_PANEL_GAP: f32 = SPACE_3;
pub const SHELL_TOPBAR_H: f32 = 42.0;
pub const SHELL_PANEL_W: f32 = 360.0;
pub const SHELL_PANEL_H: f32 = 392.0;
pub const SYSTEM_SURFACE_SAFE_INSET: f32 = SHELL_VIEWPORT_INSET;
pub const WORKSPACE_CHROME_H: f32 = 34.0;
pub const WORKSPACE_GAP: f32 = SPACE_2;
pub const WORKSPACE_MIN_TILE_W: f32 = 220.0;
pub const WORKSPACE_MIN_TILE_H: f32 = 160.0;
pub const WORKSPACE_TAB_MIN_W: f32 = 82.0;
pub const WORKSPACE_TAB_MAX_W: f32 = 180.0;
pub const TOOLTIP_PAD_X: f32 = SPACE_4;
pub const DIALOG_CONTENT_INSET_X: f32 = SPACE_8;
pub const DIALOG_CONTENT_INSET_Y: f32 = 88.0;
pub const TOAST_PAD_X: f32 = SPACE_5;
pub const POPOVER_PAD_X: f32 = SPACE_5;
pub const POPOVER_GAP: f32 = SPACE_2;
pub const SCROLLBAR_RESERVED_W: f32 = SPACE_4;
pub const SCROLLBAR_TRACK_W: f32 = 3.0;
pub const SCROLLBAR_HIT_W: f32 = SPACE_4;
pub const SCROLLBAR_EDGE_INSET: f32 = SPACE_2;
pub const MIN_TOUCH_TARGET: f32 = 32.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiComponentDensity {
    Dense,
    Default,
    Spacious,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiComponentPadding {
    pub x: f32,
    pub y: f32,
}

impl UiComponentPadding {
    pub const DENSE: Self = Self {
        x: COMPONENT_PAD_X_DENSE,
        y: COMPONENT_PAD_Y_DENSE,
    };
    pub const DEFAULT: Self = Self {
        x: COMPONENT_PAD_X,
        y: COMPONENT_PAD_Y,
    };
    pub const SPACIOUS: Self = Self {
        x: COMPONENT_PAD_X_SPACIOUS,
        y: COMPONENT_PAD_Y_SPACIOUS,
    };

    pub const fn for_density(density: UiComponentDensity) -> Self {
        match density {
            UiComponentDensity::Dense => Self::DENSE,
            UiComponentDensity::Default => Self::DEFAULT,
            UiComponentDensity::Spacious => Self::SPACIOUS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiSpacing {
    pub card_radius_max: f32,
    pub card_pad_x: f32,
    pub card_pad_y: f32,
    pub component_pad_dense: UiComponentPadding,
    pub component_pad: UiComponentPadding,
    pub component_pad_spacious: UiComponentPadding,
    pub control_pad_x: f32,
    pub control_h: f32,
    pub compact_control_h: f32,
    pub large_control_h: f32,
    pub button_h_compact: f32,
    pub button_h: f32,
    pub button_h_large: f32,
    pub icon_button_compact: f32,
    pub icon_button: f32,
    pub icon_button_large: f32,
    pub toolbar_control: f32,
    pub toolbar_icon_button: f32,
    pub form_label_h: f32,
    pub form_label_gap: f32,
    pub form_field_y: f32,
    pub form_helper_gap: f32,
    pub form_helper_y: f32,
    pub text_area_pad_y: f32,
    pub text_area_line_h: f32,
    pub text_area_reserved_y: f32,
    pub row_pad_x: f32,
    pub row_icon: f32,
    pub row_icon_gap: f32,
    pub row_text_inset: f32,
    pub row_h: f32,
    pub list_row_h: f32,
    pub menu_row_h: f32,
    pub command_row_h: f32,
    pub table_row_h: f32,
    pub table_cell_pad_x: f32,
    pub table_cell_pad_y: f32,
    pub operation_row_h: f32,
    pub operation_row_content_h: f32,
    pub operation_row_panel_pad: f32,
    pub package_card_h: f32,
    pub app_store_card_h: f32,
    pub app_card_grid_gap: f32,
    pub app_surface_inset_x: f32,
    pub app_surface_inset_y: f32,
    pub shell_viewport_inset: f32,
    pub shell_panel_gap: f32,
    pub shell_topbar_h: f32,
    pub shell_panel_w: f32,
    pub shell_panel_h: f32,
    pub system_surface_safe_inset: f32,
    pub workspace_chrome_h: f32,
    pub workspace_gap: f32,
    pub workspace_min_tile_w: f32,
    pub workspace_min_tile_h: f32,
    pub workspace_tab_min_w: f32,
    pub workspace_tab_max_w: f32,
    pub tooltip_pad_x: f32,
    pub dialog_content_inset_x: f32,
    pub dialog_content_inset_y: f32,
    pub toast_pad_x: f32,
    pub popover_pad_x: f32,
    pub popover_gap: f32,
    pub scrollbar_reserved_w: f32,
    pub scrollbar_track_w: f32,
    pub scrollbar_hit_w: f32,
    pub scrollbar_edge_inset: f32,
    pub min_touch_target: f32,
}

impl UiSpacing {
    pub const DEFAULT: Self = Self {
        card_radius_max: CARD_RADIUS_MAX,
        card_pad_x: CARD_PAD_X,
        card_pad_y: CARD_PAD_Y,
        component_pad_dense: UiComponentPadding::DENSE,
        component_pad: UiComponentPadding::DEFAULT,
        component_pad_spacious: UiComponentPadding::SPACIOUS,
        control_pad_x: CONTROL_PAD_X,
        control_h: CONTROL_H,
        compact_control_h: COMPACT_CONTROL_H,
        large_control_h: LARGE_CONTROL_H,
        button_h_compact: BUTTON_H_COMPACT,
        button_h: BUTTON_H,
        button_h_large: BUTTON_H_LARGE,
        icon_button_compact: ICON_BUTTON_COMPACT,
        icon_button: ICON_BUTTON,
        icon_button_large: ICON_BUTTON_LARGE,
        toolbar_control: TOOLBAR_CONTROL,
        toolbar_icon_button: TOOLBAR_ICON_BUTTON,
        form_label_h: FORM_LABEL_H,
        form_label_gap: FORM_LABEL_GAP,
        form_field_y: FORM_FIELD_Y,
        form_helper_gap: FORM_HELPER_GAP,
        form_helper_y: FORM_HELPER_Y,
        text_area_pad_y: TEXT_AREA_PAD_Y,
        text_area_line_h: TEXT_AREA_LINE_H,
        text_area_reserved_y: TEXT_AREA_RESERVED_Y,
        row_pad_x: ROW_PAD_X,
        row_icon: ROW_ICON,
        row_icon_gap: ROW_ICON_GAP,
        row_text_inset: ROW_TEXT_INSET,
        row_h: ROW_H,
        list_row_h: LIST_ROW_H,
        menu_row_h: MENU_ROW_H,
        command_row_h: COMMAND_ROW_H,
        table_row_h: TABLE_ROW_H,
        table_cell_pad_x: TABLE_CELL_PAD_X,
        table_cell_pad_y: TABLE_CELL_PAD_Y,
        operation_row_h: OPERATION_ROW_H,
        operation_row_content_h: OPERATION_ROW_CONTENT_H,
        operation_row_panel_pad: OPERATION_ROW_PANEL_PAD,
        package_card_h: PACKAGE_CARD_H,
        app_store_card_h: APP_STORE_CARD_H,
        app_card_grid_gap: APP_CARD_GRID_GAP,
        app_surface_inset_x: APP_SURFACE_INSET_X,
        app_surface_inset_y: APP_SURFACE_INSET_Y,
        shell_viewport_inset: SHELL_VIEWPORT_INSET,
        shell_panel_gap: SHELL_PANEL_GAP,
        shell_topbar_h: SHELL_TOPBAR_H,
        shell_panel_w: SHELL_PANEL_W,
        shell_panel_h: SHELL_PANEL_H,
        system_surface_safe_inset: SYSTEM_SURFACE_SAFE_INSET,
        workspace_chrome_h: WORKSPACE_CHROME_H,
        workspace_gap: WORKSPACE_GAP,
        workspace_min_tile_w: WORKSPACE_MIN_TILE_W,
        workspace_min_tile_h: WORKSPACE_MIN_TILE_H,
        workspace_tab_min_w: WORKSPACE_TAB_MIN_W,
        workspace_tab_max_w: WORKSPACE_TAB_MAX_W,
        tooltip_pad_x: TOOLTIP_PAD_X,
        dialog_content_inset_x: DIALOG_CONTENT_INSET_X,
        dialog_content_inset_y: DIALOG_CONTENT_INSET_Y,
        toast_pad_x: TOAST_PAD_X,
        popover_pad_x: POPOVER_PAD_X,
        popover_gap: POPOVER_GAP,
        scrollbar_reserved_w: SCROLLBAR_RESERVED_W,
        scrollbar_track_w: SCROLLBAR_TRACK_W,
        scrollbar_hit_w: SCROLLBAR_HIT_W,
        scrollbar_edge_inset: SCROLLBAR_EDGE_INSET,
        min_touch_target: MIN_TOUCH_TARGET,
    };
}

pub fn row_icon_slot(row: UiRect) -> UiRect {
    UiRect::new(row.x + ROW_PAD_X, row.y, ROW_ICON, row.h).with_height_centered(ROW_ICON)
}

pub fn row_text_rect(row: UiRect, trailing_reserved_w: f32) -> UiRect {
    UiRect::new(
        row.x + ROW_TEXT_INSET,
        row.y,
        (row.w - ROW_TEXT_INSET - ROW_PAD_X - trailing_reserved_w).max(0.0),
        row.h,
    )
}

pub fn app_surface_padding_for_width(width: f32) -> UiComponentPadding {
    if width <= NARROW_VIEWPORT_W {
        UiComponentPadding {
            x: APP_SURFACE_INSET_X_NARROW,
            y: APP_SURFACE_INSET_Y_NARROW,
        }
    } else if width >= WIDE_VIEWPORT_W {
        UiComponentPadding {
            x: APP_SURFACE_INSET_X_WIDE,
            y: APP_SURFACE_INSET_Y_WIDE,
        }
    } else {
        UiComponentPadding {
            x: APP_SURFACE_INSET_X,
            y: APP_SURFACE_INSET_Y,
        }
    }
}

pub fn app_surface_content_rect(bounds: UiRect) -> UiRect {
    let pad = app_surface_padding_for_width(bounds.w);
    bounds.inset(pad.x, pad.y)
}

pub fn system_surface_safe_rect(bounds: UiRect) -> UiRect {
    bounds.inset(SYSTEM_SURFACE_SAFE_INSET, SYSTEM_SURFACE_SAFE_INSET)
}

pub fn centered_system_panel(
    safe: UiRect,
    min_w: f32,
    max_w: f32,
    preferred_h: f32,
    min_h: f32,
) -> UiRect {
    let panel_w = if safe.w >= min_w {
        safe.w.clamp(min_w, max_w)
    } else {
        safe.w.max(0.0)
    };
    let panel_h = if safe.h >= min_h {
        preferred_h.min(safe.h).max(min_h)
    } else {
        safe.h.max(0.0)
    };
    UiRect::new(
        safe.x + (safe.w - panel_w) * 0.5,
        safe.y + (safe.h - panel_h) * 0.5,
        panel_w,
        panel_h,
    )
}

pub fn scroll_content_rect(bounds: UiRect, padding: [f32; 4]) -> UiRect {
    UiRect::new(
        bounds.x + padding[3],
        bounds.y + padding[0],
        (bounds.w - padding[1] - padding[3] - SCROLLBAR_RESERVED_W).max(0.0),
        (bounds.h - padding[0] - padding[2]).max(0.0),
    )
}

pub fn scrollbar_track_rect(bounds: UiRect, content: UiRect) -> UiRect {
    UiRect::new(
        bounds.x + bounds.w - SCROLLBAR_EDGE_INSET,
        content.y,
        SCROLLBAR_TRACK_W,
        content.h,
    )
}

pub fn scrollbar_hit_rect(track: UiRect) -> UiRect {
    UiRect::new(
        track.x - (SCROLLBAR_HIT_W - track.w),
        track.y,
        SCROLLBAR_HIT_W,
        track.h,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_spacing_matches_exported_tokens() {
        assert_eq!(UiSpacing::DEFAULT.card_radius_max, CARD_RADIUS_MAX);
        assert_eq!(UiSpacing::DEFAULT.card_pad_x, CARD_PAD_X);
        assert_eq!(
            UiSpacing::DEFAULT.component_pad,
            UiComponentPadding::DEFAULT
        );
        assert_eq!(UiSpacing::DEFAULT.row_icon_gap, ROW_ICON_GAP);
        assert_eq!(UiSpacing::DEFAULT.row_text_inset, ROW_TEXT_INSET);
        assert_eq!(UiSpacing::DEFAULT.app_surface_inset_x, APP_SURFACE_INSET_X);
        assert_eq!(
            UiSpacing::DEFAULT.shell_viewport_inset,
            SHELL_VIEWPORT_INSET
        );
        assert_eq!(UiSpacing::DEFAULT.shell_topbar_h, SHELL_TOPBAR_H);
        assert_eq!(
            UiSpacing::DEFAULT.system_surface_safe_inset,
            SYSTEM_SURFACE_SAFE_INSET
        );
        assert_eq!(UiSpacing::DEFAULT.workspace_gap, WORKSPACE_GAP);
        assert_eq!(UiSpacing::DEFAULT.tooltip_pad_x, TOOLTIP_PAD_X);
        assert_eq!(
            UiSpacing::DEFAULT.scrollbar_reserved_w,
            SCROLLBAR_RESERVED_W
        );
        assert_eq!(UiSpacing::DEFAULT.min_touch_target, MIN_TOUCH_TARGET);
    }

    #[test]
    fn card_radius_and_padding_follow_product_contract() {
        assert_eq!(CARD_RADIUS_MAX, 8.0);
        assert_eq!(CARD_PAD_X, COMPONENT_PAD_X);
        assert_eq!(CARD_PAD_Y, COMPONENT_PAD_Y);
        assert!(COMPONENT_PAD_X_DENSE < CARD_PAD_X);
        assert!(COMPONENT_PAD_Y_DENSE < CARD_PAD_Y);
        assert!(CARD_RADIUS_MAX <= 8.0);
    }

    #[test]
    fn control_and_toolbar_tokens_are_stable_size_contracts() {
        assert_eq!(BUTTON_H_COMPACT, COMPACT_CONTROL_H);
        assert_eq!(BUTTON_H, CONTROL_H);
        assert_eq!(BUTTON_H_LARGE, LARGE_CONTROL_H);
        assert_eq!(ICON_BUTTON, TOOLBAR_ICON_BUTTON);
        assert_eq!(TOOLBAR_CONTROL, CONTROL_H);
        assert!(BUTTON_H_COMPACT <= BUTTON_H);
        assert!(BUTTON_H <= BUTTON_H_LARGE);
        assert!(ICON_BUTTON_COMPACT <= ICON_BUTTON);
        assert!(ICON_BUTTON <= ICON_BUTTON_LARGE);
    }

    #[test]
    fn component_padding_density_tokens_are_monotonic() {
        let dense = UiComponentPadding::for_density(UiComponentDensity::Dense);
        let default = UiComponentPadding::for_density(UiComponentDensity::Default);
        let spacious = UiComponentPadding::for_density(UiComponentDensity::Spacious);

        assert!(dense.x < default.x);
        assert!(dense.y < default.y);
        assert!(default.x < spacious.x);
        assert!(default.y < spacious.y);
        assert_eq!(default.x, CARD_PAD_X);
        assert_eq!(default.y, CARD_PAD_Y);
    }

    #[test]
    fn row_and_table_tokens_are_separate_contracts() {
        assert_eq!(LIST_ROW_H, ROW_H);
        assert_eq!(MENU_ROW_H, COMMAND_ROW_H);
        assert!(TABLE_ROW_H >= LIST_ROW_H);
        assert!(OPERATION_ROW_H > TABLE_ROW_H);
        assert!(TABLE_CELL_PAD_X < ROW_PAD_X);
        assert!(TABLE_CELL_PAD_Y <= TABLE_CELL_PAD_X);
    }

    #[test]
    fn operational_card_and_row_rhythm_is_shared() {
        assert_eq!(OPERATION_ROW_CONTENT_H, OPERATION_ROW_H - SPACE_1 * 0.5);
        assert!(OPERATION_ROW_CONTENT_H > ROW_H);
        assert_eq!(OPERATION_ROW_PANEL_PAD, WORKSPACE_GAP);
        assert_eq!(APP_CARD_GRID_GAP, SPACE_4);
        assert!(PACKAGE_CARD_H > OPERATION_ROW_H);
        assert!(APP_STORE_CARD_H > PACKAGE_CARD_H);
        assert_eq!(UiSpacing::DEFAULT.operation_row_h, OPERATION_ROW_H);
        assert_eq!(UiSpacing::DEFAULT.package_card_h, PACKAGE_CARD_H);
        assert_eq!(UiSpacing::DEFAULT.app_store_card_h, APP_STORE_CARD_H);
    }

    #[test]
    fn shell_workspace_and_app_surface_spacing_are_separate_contracts() {
        assert_ne!(SHELL_VIEWPORT_INSET, WORKSPACE_GAP);
        assert_ne!(APP_SURFACE_INSET_X, WORKSPACE_GAP);
        assert_eq!(APP_SURFACE_INSET_X, APP_SURFACE_INSET_Y);
        assert!(SHELL_PANEL_GAP < APP_SURFACE_INSET_Y);
        assert!(WORKSPACE_GAP < SHELL_PANEL_GAP);
    }

    #[test]
    fn responsive_app_surface_spacing_changes_without_font_scaling() {
        let narrow = app_surface_padding_for_width(390.0);
        let default = app_surface_padding_for_width(900.0);
        let wide = app_surface_padding_for_width(1440.0);

        assert_eq!(narrow.x, APP_SURFACE_INSET_X_NARROW);
        assert_eq!(default.x, APP_SURFACE_INSET_X);
        assert_eq!(wide.x, APP_SURFACE_INSET_X_WIDE);
        assert!(narrow.x < default.x);
        assert!(default.x < wide.x);
        assert_eq!(narrow.y, narrow.x);
        assert_eq!(default.y, default.x);
        assert_eq!(wide.y, wide.x);
        assert_eq!(TEXT_AREA_LINE_H, SPACE_10);
        assert_eq!(FORM_LABEL_H, SPACE_9);
    }

    #[test]
    fn overlay_spacing_tokens_capture_distinct_surface_insets() {
        assert!(TOOLTIP_PAD_X < DIALOG_CONTENT_INSET_X);
        assert!(TOAST_PAD_X <= APP_SURFACE_INSET_X);
        assert_eq!(POPOVER_PAD_X, TOAST_PAD_X);
        assert_eq!(POPOVER_GAP, WORKSPACE_GAP);
        assert!(DIALOG_CONTENT_INSET_Y > DIALOG_CONTENT_INSET_X);
    }

    #[test]
    fn system_surface_safe_rect_keeps_panels_inside_viewport_inset() {
        let viewport = UiRect::new(0.0, 0.0, 360.0, 260.0);
        let safe = system_surface_safe_rect(viewport);
        let panel = centered_system_panel(safe, 320.0, 520.0, 320.0, 220.0);

        assert_eq!(
            safe,
            UiRect::new(
                SYSTEM_SURFACE_SAFE_INSET,
                SYSTEM_SURFACE_SAFE_INSET,
                360.0 - SYSTEM_SURFACE_SAFE_INSET * 2.0,
                260.0 - SYSTEM_SURFACE_SAFE_INSET * 2.0,
            )
        );
        assert!(panel.x >= safe.x);
        assert!(panel.y >= safe.y);
        assert!(panel.x + panel.w <= safe.x + safe.w);
        assert!(panel.y + panel.h <= safe.y + safe.h);
    }

    #[test]
    fn scroll_container_geometry_uses_shared_padding_and_hit_contract() {
        let bounds = UiRect::new(10.0, 20.0, 220.0, 180.0);
        let padding = [12.0, 14.0, 16.0, 18.0];
        let content = scroll_content_rect(bounds, padding);
        let track = scrollbar_track_rect(bounds, content);
        let hit = scrollbar_hit_rect(track);

        assert_eq!(
            content,
            UiRect::new(
                28.0,
                32.0,
                220.0 - 18.0 - 14.0 - SCROLLBAR_RESERVED_W,
                180.0 - 12.0 - 16.0,
            )
        );
        assert_eq!(track.w, SCROLLBAR_TRACK_W);
        assert_eq!(track.h, content.h);
        assert_eq!(hit.w, SCROLLBAR_HIT_W);
        assert_eq!(hit.h, track.h);
        assert!(hit.x <= track.x);
    }

    #[test]
    fn form_spacing_tokens_match_field_geometry() {
        assert_eq!(FORM_FIELD_Y, FORM_LABEL_H + FORM_LABEL_GAP);
        assert_eq!(FORM_HELPER_Y, FORM_FIELD_Y + CONTROL_H + FORM_HELPER_GAP);
        assert_eq!(TEXT_AREA_RESERVED_Y, FORM_FIELD_Y);
        assert!(FORM_LABEL_GAP < FORM_HELPER_GAP);
        assert!(TEXT_AREA_LINE_H <= FORM_FIELD_Y);
    }

    #[test]
    fn row_icon_and_text_geometry_share_one_contract() {
        let row = UiRect::new(10.0, 20.0, 360.0, ROW_H);
        let icon = row_icon_slot(row);
        let text = row_text_rect(row, 120.0);

        assert_eq!(icon, UiRect::new(24.0, 32.0, ROW_ICON, ROW_ICON));
        assert_eq!(text.x, row.x + ROW_TEXT_INSET);
        assert_eq!(text.w, row.w - ROW_TEXT_INSET - ROW_PAD_X - 120.0);
        assert!(text.x >= icon.x + icon.w + ROW_ICON_GAP);
    }
}
