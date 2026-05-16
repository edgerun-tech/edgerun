use std::string::String;
use std::vec::Vec;

use super::*;

pub const POCKETBASE_ADMIN_COLLECTIONS_ID: u32 = 7101;
pub const POCKETBASE_ADMIN_SETTINGS_ID: u32 = 7102;
pub const POCKETBASE_ADMIN_LOGS_ID: u32 = 7103;
pub const POCKETBASE_ADMIN_BACKUPS_ID: u32 = 7104;
pub const POCKETBASE_ADMIN_CRONS_ID: u32 = 7105;
pub const POCKETBASE_ADMIN_DNS_ID: u32 = 7106;
pub const POCKETBASE_ADMIN_ACME_ID: u32 = 7107;
pub const POCKETBASE_ADMIN_HEALTH_ID: u32 = 7108;
pub const POCKETBASE_ADMIN_IMPORT_ID: u32 = 7109;
pub const POCKETBASE_ADMIN_OPTIONS_ID: u32 = 7110;
pub const POCKETBASE_ADMIN_NEW_RECORD_ID: u32 = 7111;
pub const POCKETBASE_ADMIN_COLLECTION_ROW_BASE_ID: u32 = 7200;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiPocketBaseAdminState {
    pub active_view: UiPocketBaseAdminView,
    pub selected_collection: String,
    pub collections: Vec<UiPocketBaseCollection>,
    pub logs: Vec<UiPocketBaseLog>,
    pub superuser_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiPocketBaseAdminView {
    #[default]
    Collections,
    Logs,
    Settings,
    Backups,
    Crons,
}

impl UiPocketBaseAdminView {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Collections => "collections",
            Self::Logs => "logs",
            Self::Settings => "settings",
            Self::Backups => "backups",
            Self::Crons => "crons",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Collections => "Collections",
            Self::Logs => "Logs",
            Self::Settings => "Settings",
            Self::Backups => "Backups",
            Self::Crons => "Crons",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiPocketBaseCollection {
    pub name: String,
    pub kind: String,
    pub field_count: usize,
    pub record_count: usize,
    pub records: Vec<UiPocketBaseRecord>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiPocketBaseRecord {
    pub id: String,
    pub email: String,
    pub created: String,
    pub updated: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiPocketBaseLog {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub created: String,
}

pub fn build_pocketbase_admin_scene(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    state: &UiPocketBaseAdminState,
    width: f32,
    height: f32,
) {
    scene.clear = Color4::rgb_u8(9, 9, 9);
    let mut ui = UiPainter::with_font(scene, atlas);
    render_pocketbase_admin(&mut ui, state, width, height);
}

fn render_pocketbase_admin(
    ui: &mut UiPainter<'_, '_>,
    state: &UiPocketBaseAdminState,
    width: f32,
    height: f32,
) {
    let bg = Color4::rgb_u8(9, 9, 9);
    let header = Color4::rgb_u8(16, 85, 201);
    let header_hi = Color4::rgba_u8(255, 255, 255, 30);
    let surface = Color4::rgb_u8(31, 31, 31);
    let surface_alt = Color4::rgb_u8(37, 37, 37);
    let surface_alt2 = Color4::rgb_u8(48, 48, 48);
    let border = Color4::rgb_u8(64, 64, 64);
    let text_color = Color4::rgb_u8(235, 235, 235);
    let muted = Color4::rgba_u8(235, 235, 235, 140);
    let accent = Color4::rgb_u8(51, 118, 229);
    let cyan = Color4::rgb_u8(20, 184, 215);

    ui.fill_rect(UiRect::new(0.0, 0.0, width, height), 0.0, bg);
    ui.fill_rect(UiRect::new(0.0, 0.0, width, 38.0), 0.0, header);
    ui.icon(
        UiRect::new(13.0, 8.0, 22.0, 22.0),
        UiIcon::Database,
        text_color,
    );
    let header_label_y = centered_label_y(ui, 0.0, 38.0, "EdgeRun PocketBase", 2.0);
    ui.bounded_label(
        43.0,
        header_label_y,
        164.0,
        "EdgeRun PocketBase",
        2.0,
        text_color,
    );
    header_link(
        ui,
        225.0,
        94.0,
        "Collections",
        POCKETBASE_ADMIN_COLLECTIONS_ID,
        state.active_view == UiPocketBaseAdminView::Collections,
    );
    header_link(
        ui,
        326.0,
        45.0,
        "Logs",
        POCKETBASE_ADMIN_LOGS_ID,
        state.active_view == UiPocketBaseAdminView::Logs,
    );
    header_link(
        ui,
        379.0,
        67.0,
        "Settings",
        POCKETBASE_ADMIN_SETTINGS_ID,
        state.active_view == UiPocketBaseAdminView::Settings,
    );
    header_link(
        ui,
        454.0,
        66.0,
        "Backups",
        POCKETBASE_ADMIN_BACKUPS_ID,
        state.active_view == UiPocketBaseAdminView::Backups,
    );
    header_link(
        ui,
        532.0,
        50.0,
        "Crons",
        POCKETBASE_ADMIN_CRONS_ID,
        state.active_view == UiPocketBaseAdminView::Crons,
    );

    let user_x = (width - 162.0).max(620.0);
    ui.fill_rect(UiRect::new(user_x, 6.0, 146.0, 26.0), 5.0, header_hi);
    ui.icon(
        UiRect::new(user_x + 8.0, 9.0, 18.0, 18.0),
        UiIcon::User,
        text_color,
    );
    ui.bounded_label(
        user_x + 32.0,
        centered_label_y(ui, 6.0, 26.0, "superuser", 2.0),
        104.0,
        "superuser",
        2.0,
        text_color,
    );

    let page_y = 38.0;
    let sidebar_w = if width < 820.0 { 196.0 } else { 240.0 };
    let page_h = height - page_y;
    ui.fill_rect(UiRect::new(0.0, page_y, width, page_h), 0.0, surface);
    ui.fill_rect(
        UiRect::new(0.0, page_y, sidebar_w, page_h),
        0.0,
        surface_alt,
    );
    ui.border_rect(UiRect::new(sidebar_w, page_y, 1.0, page_h), 0.0, border);

    ui.bounded_label(20.0, 63.0, sidebar_w - 40.0, "Collections", 2.0, text_color);
    ui.fill_rect(
        UiRect::new(16.0, 96.0, sidebar_w - 32.0, 33.0),
        5.0,
        surface,
    );
    ui.icon(UiRect::new(27.0, 104.0, 17.0, 17.0), UiIcon::Search, muted);
    ui.bounded_label(
        53.0,
        103.0,
        sidebar_w - 72.0,
        "Search collection",
        2.0,
        muted,
    );

    let mut sidebar_y = 146.0;
    let active_collection_index = active_collection_index(state);
    for (index, collection) in state.collections.iter().take(12).enumerate() {
        let active = index == active_collection_index;
        if active {
            ui.fill_rect(
                UiRect::new(12.0, sidebar_y - 6.0, sidebar_w - 24.0, 38.0),
                5.0,
                Color4::rgba_u8(16, 85, 201, 54),
            );
            ui.fill_rect(UiRect::new(12.0, sidebar_y - 6.0, 3.0, 38.0), 1.5, accent);
        }
        ui.icon(
            UiRect::new(23.0, sidebar_y, 18.0, 18.0),
            if collection.kind == "auth" {
                UiIcon::User
            } else {
                UiIcon::Database
            },
            if active { text_color } else { muted },
        );
        ui.bounded_label(
            51.0,
            sidebar_y - 1.0,
            sidebar_w - 86.0,
            &collection.name,
            2.0,
            if active { text_color } else { muted },
        );
        ui.bounded_label(
            sidebar_w - 40.0,
            sidebar_y,
            30.0,
            &collection.record_count.to_string(),
            2.0,
            muted,
        );
        ui.hit(
            HitKind::ListRow,
            POCKETBASE_ADMIN_COLLECTION_ROW_BASE_ID + index as u32,
            12.0,
            sidebar_y - 6.0,
            sidebar_w - 24.0,
            38.0,
        );
        sidebar_y += 38.0;
    }
    if state.collections.is_empty() {
        ui.bounded_label(
            20.0,
            sidebar_y,
            sidebar_w - 40.0,
            "No collections yet",
            2.0,
            muted,
        );
    }

    let content_x = sidebar_w + 30.0;
    let content_w = (width - content_x - 30.0).max(320.0);
    let right_w = if content_w > 860.0 { 320.0 } else { 0.0 };
    let main_w = if right_w > 0.0 {
        content_w - right_w - 24.0
    } else {
        content_w
    };
    let active = state.collections.get(active_collection_index);
    let active_name = active.map(|c| c.name.as_str()).unwrap_or("Collections");
    let record_count = active.map(|c| c.record_count).unwrap_or(0);

    if state.active_view != UiPocketBaseAdminView::Collections {
        render_admin_view_page(
            ui,
            state,
            content_x,
            62.0,
            content_w,
            height,
            text_color,
            muted,
            surface_alt,
            surface_alt2,
            border,
        );
        return;
    }

    ui.bounded_label(content_x, 62.0, 110.0, "Collections", 2.0, muted);
    ui.bounded_label(content_x + 106.0, 62.0, 180.0, active_name, 2.0, text_color);
    ui.fill_rect(UiRect::new(content_x, 94.0, main_w, 1.0), 0.0, border);
    ui.fill_rect(
        UiRect::new(content_x, 113.0, main_w * 0.56, 35.0),
        5.0,
        surface_alt2,
    );
    ui.icon(
        UiRect::new(content_x + 12.0, 122.0, 17.0, 17.0),
        UiIcon::Search,
        muted,
    );
    ui.bounded_label(
        content_x + 39.0,
        121.0,
        main_w * 0.48,
        "Filter records",
        2.0,
        muted,
    );

    let btn_x = content_x + main_w - 292.0;
    scene_button(
        ui,
        btn_x,
        113.0,
        82.0,
        "Import",
        UiIcon::File,
        POCKETBASE_ADMIN_IMPORT_ID,
        false,
    );
    scene_button(
        ui,
        btn_x + 90.0,
        113.0,
        86.0,
        "Options",
        UiIcon::Settings,
        POCKETBASE_ADMIN_OPTIONS_ID,
        false,
    );
    scene_button(
        ui,
        btn_x + 184.0,
        113.0,
        108.0,
        "New record",
        UiIcon::MessagePlus,
        POCKETBASE_ADMIN_NEW_RECORD_ID,
        true,
    );

    ui.bounded_label(content_x, 170.0, main_w, active_name, 2.6, text_color);
    let subtitle = active
        .map(|c| {
            format!(
                "{} collection | {} records | {} fields",
                c.kind, c.record_count, c.field_count
            )
        })
        .unwrap_or_else(|| "No active collection".to_string());
    ui.bounded_label(content_x, 199.0, main_w, &subtitle, 2.0, muted);

    render_records_table(
        ui,
        active,
        content_x,
        244.0,
        main_w,
        height,
        text_color,
        muted,
        surface_alt,
        surface_alt2,
        border,
    );
    ui.bounded_label(
        content_x,
        height - 22.0,
        180.0,
        &format!("{record_count} total records"),
        2.0,
        muted,
    );

    if right_w > 0.0 {
        let right_x = content_x + main_w + 24.0;
        side_panel(
            ui,
            right_x,
            113.0,
            right_w,
            158.0,
            "Collection",
            active_name,
            UiIcon::Database,
        );
        let detail = active
            .map(|c| {
                format!(
                    "{} fields | API rules | {} records",
                    c.field_count, c.record_count
                )
            })
            .unwrap_or_else(|| "No schema selected".to_string());
        ui.bounded_label(right_x + 18.0, 190.0, right_w - 36.0, &detail, 2.0, muted);

        side_panel(
            ui,
            right_x,
            293.0,
            right_w,
            height - 323.0,
            "Recent Logs",
            "Live request activity",
            UiIcon::Activity,
        );
        let mut y = 369.0;
        for (index, log) in state
            .logs
            .iter()
            .filter(|log| !log.path.starts_with("/_/assets"))
            .take(7)
            .enumerate()
        {
            let row_h = 52.0;
            ui.fill_rect(
                UiRect::new(right_x + 14.0, y, right_w - 28.0, row_h),
                6.0,
                Color4::rgba_u8(255, 255, 255, 8),
            );
            ui.fill_rect(UiRect::new(right_x + 14.0, y, 3.0, row_h), 1.5, cyan);
            ui.hit(
                HitKind::ListRow,
                7300 + index as u32,
                right_x + 14.0,
                y,
                right_w - 28.0,
                row_h,
            );
            ui.bounded_label(
                right_x + 30.0,
                y + 8.0,
                right_w - 52.0,
                &log.path,
                2.0,
                text_color,
            );
            ui.bounded_label(
                right_x + 30.0,
                y + 29.0,
                right_w - 52.0,
                &format!("{} {} | {}", log.method, log.status, log.created),
                2.0,
                muted,
            );
            y += row_h + 10.0;
        }
    }
}

fn active_collection_index(state: &UiPocketBaseAdminState) -> usize {
    if state.selected_collection.is_empty() {
        return 0;
    }
    state
        .collections
        .iter()
        .position(|collection| collection.name == state.selected_collection)
        .unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
fn render_admin_view_page(
    ui: &mut UiPainter<'_, '_>,
    state: &UiPocketBaseAdminState,
    x: f32,
    y: f32,
    w: f32,
    height: f32,
    text_color: Color4,
    muted: Color4,
    surface_alt: Color4,
    surface_alt2: Color4,
    border: Color4,
) {
    ui.bounded_label(x, y, 160.0, state.active_view.title(), 2.6, text_color);
    ui.bounded_label(
        x,
        y + 31.0,
        w,
        "Admin state projected from the PocketBase-compatible service.",
        2.0,
        muted,
    );
    match state.active_view {
        UiPocketBaseAdminView::Logs => {
            render_logs_table(
                ui,
                state,
                x,
                y + 75.0,
                w,
                height,
                text_color,
                muted,
                surface_alt,
                surface_alt2,
                border,
            );
        }
        UiPocketBaseAdminView::Settings => {
            render_metric_grid(
                ui,
                x,
                y + 75.0,
                w,
                &[
                    ("Superusers", state.superuser_count),
                    ("Collections", state.collections.len()),
                    (
                        "Records",
                        state.collections.iter().map(|c| c.record_count).sum(),
                    ),
                    ("Visible logs", state.logs.len()),
                ],
                text_color,
                muted,
                surface_alt,
                border,
            );
        }
        UiPocketBaseAdminView::Backups => {
            render_empty_admin_panel(
                ui,
                x,
                y + 75.0,
                w,
                height,
                "Backups are available through /api/backups.",
                text_color,
                muted,
                surface_alt,
                border,
            );
        }
        UiPocketBaseAdminView::Crons => {
            render_empty_admin_panel(
                ui,
                x,
                y + 75.0,
                w,
                height,
                "Cron jobs are available through /api/crons.",
                text_color,
                muted,
                surface_alt,
                border,
            );
        }
        UiPocketBaseAdminView::Collections => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn render_logs_table(
    ui: &mut UiPainter<'_, '_>,
    state: &UiPocketBaseAdminState,
    x: f32,
    y: f32,
    w: f32,
    height: f32,
    text_color: Color4,
    muted: Color4,
    surface_alt: Color4,
    surface_alt2: Color4,
    border: Color4,
) {
    let table_h = (height - y - 30.0).max(180.0);
    ui.fill_rect(UiRect::new(x, y, w, table_h), 8.0, surface_alt);
    ui.border_rect(UiRect::new(x, y, w, table_h), 8.0, border);
    ui.fill_rect(UiRect::new(x, y, w, 40.0), 8.0, surface_alt2);
    ui.bounded_label(x + 20.0, y + 12.0, 80.0, "status", 2.0, muted);
    ui.bounded_label(x + 112.0, y + 12.0, 90.0, "method", 2.0, muted);
    ui.bounded_label(x + 210.0, y + 12.0, w * 0.42, "path", 2.0, muted);
    ui.bounded_label(x + w - 180.0, y + 12.0, 160.0, "created", 2.0, muted);

    if state.logs.is_empty() {
        ui.icon(
            UiRect::new(x + w * 0.5 - 16.0, y + table_h * 0.5 - 44.0, 32.0, 32.0),
            UiIcon::Activity,
            muted,
        );
        ui.bounded_label(
            x + w * 0.5 - 80.0,
            y + table_h * 0.5,
            160.0,
            "No logs yet",
            2.2,
            text_color,
        );
        return;
    }

    let mut row_y = y + 40.0;
    for (index, log) in state.logs.iter().take(12).enumerate() {
        let fill = if index % 2 == 0 {
            Color4::rgba_u8(255, 255, 255, 10)
        } else {
            Color4::rgba_u8(255, 255, 255, 4)
        };
        ui.fill_rect(UiRect::new(x, row_y, w, 40.0), 0.0, fill);
        ui.hit(HitKind::ListRow, 7600 + index as u32, x, row_y, w, 40.0);
        ui.bounded_label(
            x + 20.0,
            row_y + 11.0,
            76.0,
            &log.status.to_string(),
            2.0,
            text_color,
        );
        ui.bounded_label(x + 112.0, row_y + 11.0, 88.0, &log.method, 2.0, muted);
        ui.bounded_label(
            x + 210.0,
            row_y + 11.0,
            w * 0.42,
            &log.path,
            2.0,
            text_color,
        );
        ui.bounded_label(x + w - 180.0, row_y + 11.0, 160.0, &log.created, 2.0, muted);
        row_y += 40.0;
    }
}

fn render_metric_grid(
    ui: &mut UiPainter<'_, '_>,
    x: f32,
    y: f32,
    w: f32,
    metrics: &[(&str, usize)],
    text_color: Color4,
    muted: Color4,
    surface_alt: Color4,
    border: Color4,
) {
    let card_w = ((w - 24.0) / 2.0).max(180.0);
    for (index, (label, value)) in metrics.iter().enumerate() {
        let col = index % 2;
        let row = index / 2;
        let card_x = x + col as f32 * (card_w + 24.0);
        let card_y = y + row as f32 * 116.0;
        ui.fill_rect(UiRect::new(card_x, card_y, card_w, 92.0), 8.0, surface_alt);
        ui.border_rect(UiRect::new(card_x, card_y, card_w, 92.0), 8.0, border);
        ui.bounded_label(
            card_x + 18.0,
            card_y + 16.0,
            card_w - 36.0,
            label,
            2.0,
            muted,
        );
        ui.bounded_label(
            card_x + 18.0,
            card_y + 44.0,
            card_w - 36.0,
            &value.to_string(),
            3.0,
            text_color,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_empty_admin_panel(
    ui: &mut UiPainter<'_, '_>,
    x: f32,
    y: f32,
    w: f32,
    height: f32,
    message: &str,
    text_color: Color4,
    muted: Color4,
    surface_alt: Color4,
    border: Color4,
) {
    let h = (height - y - 30.0).max(180.0);
    ui.fill_rect(UiRect::new(x, y, w, h), 8.0, surface_alt);
    ui.border_rect(UiRect::new(x, y, w, h), 8.0, border);
    ui.icon(
        UiRect::new(x + w * 0.5 - 16.0, y + h * 0.5 - 44.0, 32.0, 32.0),
        UiIcon::File,
        muted,
    );
    ui.bounded_label(
        x + w * 0.5 - 180.0,
        y + h * 0.5,
        360.0,
        message,
        2.1,
        text_color,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_records_table(
    ui: &mut UiPainter<'_, '_>,
    active: Option<&UiPocketBaseCollection>,
    x: f32,
    y: f32,
    w: f32,
    height: f32,
    text_color: Color4,
    muted: Color4,
    surface_alt: Color4,
    surface_alt2: Color4,
    border: Color4,
) {
    let table_h = (height - y - 30.0).max(180.0);
    ui.fill_rect(UiRect::new(x, y, w, table_h), 8.0, surface_alt);
    ui.border_rect(UiRect::new(x, y, w, table_h), 8.0, border);
    ui.fill_rect(UiRect::new(x, y, w, 40.0), 8.0, surface_alt2);
    let col_id = x + 20.0;
    let col_email = x + w * 0.28;
    let col_created = x + w * 0.58;
    let col_updated = x + w * 0.78;
    ui.bounded_label(col_id, y + 12.0, 120.0, "id", 2.0, muted);
    ui.bounded_label(col_email, y + 12.0, 160.0, "email", 2.0, muted);
    ui.bounded_label(col_created, y + 12.0, 140.0, "created", 2.0, muted);
    ui.bounded_label(col_updated, y + 12.0, 140.0, "updated", 2.0, muted);

    let records = active.map(|c| c.records.as_slice()).unwrap_or(&[]);
    if records.is_empty() {
        ui.icon(
            UiRect::new(x + w * 0.5 - 16.0, y + table_h * 0.5 - 44.0, 32.0, 32.0),
            UiIcon::File,
            muted,
        );
        ui.bounded_label(
            x + w * 0.5 - 116.0,
            y + table_h * 0.5,
            232.0,
            "No records found",
            2.2,
            text_color,
        );
        ui.bounded_label(
            x + w * 0.5 - 160.0,
            y + table_h * 0.5 + 28.0,
            320.0,
            "Create a record or import data to populate this collection.",
            2.0,
            muted,
        );
        return;
    }

    let mut row_y = y + 40.0;
    for (index, record) in records.iter().take(10).enumerate() {
        let fill = if index % 2 == 0 {
            Color4::rgba_u8(255, 255, 255, 10)
        } else {
            Color4::rgba_u8(255, 255, 255, 4)
        };
        ui.fill_rect(UiRect::new(x, row_y, w, 38.0), 0.0, fill);
        ui.hit(HitKind::ListRow, 7400 + index as u32, x, row_y, w, 38.0);
        ui.bounded_label(col_id, row_y + 10.0, w * 0.24, &record.id, 2.0, text_color);
        ui.bounded_label(col_email, row_y + 10.0, w * 0.26, &record.email, 2.0, muted);
        ui.bounded_label(
            col_created,
            row_y + 10.0,
            w * 0.18,
            &record.created,
            2.0,
            muted,
        );
        ui.bounded_label(
            col_updated,
            row_y + 10.0,
            w * 0.18,
            &record.updated,
            2.0,
            muted,
        );
        row_y += 38.0;
    }
}

fn header_link(ui: &mut UiPainter<'_, '_>, x: f32, w: f32, label: &str, id: u32, active: bool) {
    let text_color = if active {
        Color4::rgb_u8(255, 255, 255)
    } else {
        Color4::rgba_u8(255, 255, 255, 170)
    };
    if active {
        ui.fill_rect(
            UiRect::new(x - 6.0, 6.0, w + 12.0, 26.0),
            5.0,
            Color4::rgba_u8(255, 255, 255, 28),
        );
    }
    ui.hit(HitKind::Button, id, x - 6.0, 6.0, w + 12.0, 26.0);
    ui.bounded_label(
        x,
        centered_label_y(ui, 6.0, 26.0, label, 2.0),
        w,
        label,
        2.0,
        text_color,
    );
}

fn centered_label_y(ui: &UiPainter<'_, '_>, y: f32, h: f32, label: &str, scale: f32) -> f32 {
    ui.visual_centered_label_y(y, h, label, scale)
}

fn scene_button(
    ui: &mut UiPainter<'_, '_>,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    icon_kind: UiIcon,
    id: u32,
    primary: bool,
) {
    let fill = if primary {
        Color4::rgb_u8(16, 85, 201)
    } else {
        Color4::rgb_u8(48, 48, 48)
    };
    let stroke = if primary {
        Color4::rgb_u8(47, 119, 235)
    } else {
        Color4::rgb_u8(64, 64, 64)
    };
    let color = Color4::rgb_u8(235, 235, 235);
    ui.fill_rect(UiRect::new(x, y, w, 35.0), 5.0, fill);
    ui.border_rect(UiRect::new(x, y, w, 35.0), 5.0, stroke);
    ui.hit(HitKind::Button, id, x, y, w, 35.0);
    ui.icon(UiRect::new(x + 11.0, y + 9.0, 17.0, 17.0), icon_kind, color);
    ui.bounded_label(x + 34.0, y + 9.0, w - 42.0, label, 2.0, color);
}

fn side_panel(
    ui: &mut UiPainter<'_, '_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    title: &str,
    subtitle: &str,
    icon_kind: UiIcon,
) {
    let surface_alt = Color4::rgb_u8(37, 37, 37);
    let border = Color4::rgb_u8(64, 64, 64);
    let text_color = Color4::rgb_u8(235, 235, 235);
    let muted = Color4::rgba_u8(235, 235, 235, 140);
    ui.fill_rect(UiRect::new(x, y, w, h), 8.0, surface_alt);
    ui.border_rect(UiRect::new(x, y, w, h), 8.0, border);
    ui.icon(
        UiRect::new(x + 18.0, y + 18.0, 22.0, 22.0),
        icon_kind,
        muted,
    );
    ui.bounded_label(x + 52.0, y + 15.0, w - 70.0, title, 2.2, text_color);
    ui.bounded_label(x + 52.0, y + 40.0, w - 70.0, subtitle, 2.0, muted);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pocketbase_admin_scene_is_dark_and_interactive() {
        let state = UiPocketBaseAdminState {
            active_view: UiPocketBaseAdminView::Collections,
            selected_collection: "users".to_string(),
            superuser_count: 1,
            collections: vec![UiPocketBaseCollection {
                name: "users".to_string(),
                kind: "auth".to_string(),
                field_count: 4,
                record_count: 1,
                records: vec![UiPocketBaseRecord {
                    id: "abc123".to_string(),
                    email: "root@example.test".to_string(),
                    created: "2026-05-16 00:00:00".to_string(),
                    updated: "2026-05-16 00:00:00".to_string(),
                }],
            }],
            logs: vec![UiPocketBaseLog {
                method: "GET".to_string(),
                path: "/api/collections".to_string(),
                status: 200,
                created: "2026-05-16 00:00:00".to_string(),
            }],
        };
        let atlas = FontAtlas::load_geist(16.0).expect("font atlas");
        let mut scene = GpuScene::new(Color4::rgb_u8(255, 255, 255));
        build_pocketbase_admin_scene(&mut scene, &atlas, &state, 1280.0, 820.0);

        assert_eq!(scene.clear, Color4::rgb_u8(9, 9, 9));
        assert!(scene.rects().len() > 20);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == POCKETBASE_ADMIN_NEW_RECORD_ID)
        );
        assert!(scene.hits().iter().any(|hit| hit.kind == HitKind::ListRow));

        let mut logs_state = state;
        logs_state.active_view = UiPocketBaseAdminView::Logs;
        let mut logs_scene = GpuScene::new(Color4::rgb_u8(255, 255, 255));
        build_pocketbase_admin_scene(&mut logs_scene, &atlas, &logs_state, 1280.0, 820.0);
        assert!(
            logs_scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == POCKETBASE_ADMIN_LOGS_ID)
        );
        assert!(
            logs_scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id >= 7600)
        );
    }
}
