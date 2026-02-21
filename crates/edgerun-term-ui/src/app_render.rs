use std::borrow::Cow;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::overlay;
use log::{error, info, warn};
use pixels::{Error as PixelsError, Pixels, wgpu};
use term_core::gpu::{GlyphAtlas, GlyphVertex, GpuRenderer, RectVertex};
use term_core::render::layout::LayoutMetrics;
use term_core::render::primitives::fill_rect;
use term_core::render::{
    GlyphCache, OVERLAY_BADGE, OVERLAY_TEXT, TabVisual, build_border_gpu, draw_background,
    draw_text_line_clipped, rgba_bytes,
};
use term_core::render::{
    build_help_bar_gpu, build_tab_bar_gpu, draw_border_cpu, draw_cursor_overlay, draw_grid,
    draw_help_bar_cpu, draw_tab_bar_cpu,
};
use term_core::terminal::Rgba;

use crate::debug::{DebugOverlay, DebugRenderMode, DebugRendererUsed};
use crate::widgets::cheatsheet::{Cheatsheet, build_cheatsheet_gpu, draw_cheatsheet_cpu};
use crate::widgets::context::ContextMenu;
use crate::widgets::history::HistoryMenu;
use crate::widgets::log_viewer::{LogViewer, build_log_viewer_gpu, draw_log_viewer_cpu};
use crate::widgets::settings::{SettingsPanel, build_settings_panel_gpu, draw_settings_panel_cpu};

static LOG_CURSOR_CELLS: OnceLock<bool> = OnceLock::new();
static LAST_CELL_LOG: OnceLock<Mutex<Option<CellSnapshot>>> = OnceLock::new();
static LAST_CPU_FRAME: OnceLock<Mutex<Option<CachedFrame>>> = OnceLock::new();
static LAST_ROW_VERSIONS: OnceLock<Mutex<Vec<u64>>> = OnceLock::new();
static LAST_BLINK_STATE: OnceLock<Mutex<Option<bool>>> = OnceLock::new();
static LAST_ACTIVE_TAB: OnceLock<Mutex<Option<usize>>> = OnceLock::new();
static LAST_UI_DAMAGE: OnceLock<Mutex<UiDamageState>> = OnceLock::new();

struct CachedFrame {
    width: u32,
    height: u32,
    frame: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CellSnapshot {
    col: usize,
    row: usize,
    view_offset: usize,
    text: String,
    wide: bool,
    cont: bool,
    blank: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct UiDamageState {
    selection: Option<((usize, usize), (usize, usize))>,
    hover: Option<(usize, usize)>,
    hover_link_range: Option<(usize, usize, usize)>,
}

fn row_snapshot(term: &term_core::terminal::Terminal, row: usize, cols: usize) -> String {
    let mut s = String::new();
    let limit = cols.min(120);
    for c in 0..limit {
        let cell = term.display_cell(c, row);
        let ch = if cell.wide_continuation {
            '_'
        } else if cell.text.is_empty() {
            ' '
        } else {
            cell.text.chars().next().unwrap_or(' ')
        };
        s.push(ch);
    }
    s
}

#[allow(clippy::too_many_arguments)]
fn build_gpu_overlays(
    rects: &mut Vec<RectVertex>,
    glyphs_out: &mut Vec<GlyphVertex>,
    atlas: &mut GlyphAtlas,
    queue: &wgpu::Queue,
    glyphs_cache: &mut GlyphCache,
    tab_visuals: &[TabVisual<'_>],
    active_tab: usize,
    tab: &TabRender<'_>,
    frame_width: u32,
    frame_height: u32,
    tab_bar_height: u32,
    border_thickness: u32,
    border_radius: u32,
    border_inset: u32,
    cell_h: u32,
    start_time: Instant,
    focused: bool,
    help_visible: bool,
    history_menu: &mut HistoryMenu,
    context_menu: &mut ContextMenu,
    cheatsheet: &Cheatsheet,
    settings: &SettingsPanel,
    log_viewer: &LogViewer,
    debug_overlay: &DebugOverlay,
    fps: f32,
    notice_text: Option<&str>,
    overlay_active: bool,
) {
    build_tab_bar_gpu(
        rects,
        glyphs_out,
        atlas,
        queue,
        tab_visuals,
        active_tab,
        glyphs_cache,
        frame_width,
        tab_bar_height,
        border_thickness,
        start_time,
    );
    if settings.show_fps {
        build_fps_in_tab_bar_gpu(
            rects,
            glyphs_out,
            atlas,
            queue,
            glyphs_cache,
            frame_width,
            frame_height,
            fps,
            debug_overlay
                .last_used_renderer()
                .unwrap_or(DebugRendererUsed::Gpu),
            tab_bar_height,
            border_thickness,
        );
    }
    if overlay_active {
        overlay::build_debug_overlay_gpu(
            rects,
            glyphs_out,
            atlas,
            queue,
            glyphs_cache,
            frame_width,
            frame_height,
            debug_overlay,
        );
    } else {
        let selection = tab.selection;
        let mode = current_ui_mode(
            selection,
            history_menu,
            context_menu,
            cheatsheet,
            settings,
            log_viewer,
        );
        if help_visible && !cheatsheet.is_open() {
            let help_text = help_text_with_app(mode, tab.terminal.help_text.as_deref());
            build_help_bar_gpu(
                rects,
                glyphs_out,
                atlas,
                queue,
                glyphs_cache,
                frame_width,
                frame_height,
                cell_h,
                border_thickness,
                help_text.as_ref(),
                tab.link_hover
                    .and_then(|(c, r)| tab.terminal.display_cell(c, r).hyperlink.clone())
                    .as_deref(),
                status_label_for_terminal(&tab.terminal),
            );
        }
        history_menu.draw_gpu(
            rects,
            glyphs_out,
            atlas,
            queue,
            glyphs_cache,
            frame_width,
            frame_height,
        );
        context_menu.draw_gpu(
            rects,
            glyphs_out,
            atlas,
            queue,
            glyphs_cache,
            frame_width,
            frame_height,
        );
        build_cheatsheet_gpu(
            rects,
            glyphs_out,
            atlas,
            queue,
            cheatsheet,
            glyphs_cache,
            frame_width,
            frame_height,
        );
        build_settings_panel_gpu(
            rects,
            glyphs_out,
            atlas,
            queue,
            settings,
            glyphs_cache,
            frame_width,
            frame_height,
        );
        build_log_viewer_gpu(
            rects,
            glyphs_out,
            atlas,
            queue,
            glyphs_cache,
            frame_width,
            frame_height,
            log_viewer,
        );
    }
    build_border_gpu(
        rects,
        frame_width,
        frame_height,
        border_thickness,
        border_radius,
        start_time,
        focused,
        border_inset,
    );
    if let Some(text) = notice_text {
        let y_offset = overlay_offset_y(glyphs_cache, settings.show_fps);
        build_notice_overlay_gpu(
            rects,
            glyphs_out,
            atlas,
            queue,
            glyphs_cache,
            frame_width,
            frame_height,
            text,
            y_offset,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_gpu_frame(
    renderer: &mut GpuRenderer,
    pixels: &mut Pixels,
    tabs: &[TabRender<'_>],
    active_tab: usize,
    glyphs: &mut GlyphCache,
    layout: &LayoutMetrics,
    frame_width: u32,
    frame_height: u32,
    cell_w: u32,
    cell_h: u32,
    tab_bar_height: u32,
    border_thickness: u32,
    border_radius: u32,
    border_inset: u32,
    start_time: Instant,
    focused: bool,
    help_visible: bool,
    history_menu: &mut HistoryMenu,
    context_menu: &mut ContextMenu,
    cheatsheet: &Cheatsheet,
    settings: &SettingsPanel,
    log_viewer: &LogViewer,
    debug_overlay: &DebugOverlay,
    fps: f32,
    notice_text: Option<&str>,
    overlay_active: bool,
    draw_grid: bool,
    cell_blink_on: bool,
    cursor_blink_on: bool,
) -> Result<(), PixelsError> {
    if let Some(tab) = tabs.get(active_tab) {
        let tab_visuals: Vec<_> = tabs.iter().map(|t| TabVisual { title: &t.title }).collect();
        let selection = tab.selection;
        let tab_changed = match LAST_ACTIVE_TAB.get_or_init(|| Mutex::new(None)).lock() {
            Ok(mut last) => {
                let changed = last.map_or(true, |prev| prev != active_tab);
                *last = Some(active_tab);
                changed
            }
            Err(_) => true,
        };
        pixels.render_with(|encoder, target, context| {
            if tab_changed {
                renderer.invalidate_base();
            }
            renderer.ensure_base_target(&context.device, frame_width, frame_height);
            // Compute row-level damage from terminal row version stamps.
            let mut damage_rects: Vec<(u32, u32, u32, u32)> = Vec::new();
            let mut dirty_rows: Vec<usize> = Vec::new();
            {
                let rows = tab.terminal.rows as usize;
                let mut last = LAST_ROW_VERSIONS
                    .get_or_init(|| Mutex::new(Vec::new()))
                    .lock()
                    .unwrap();
                if last.len() != rows {
                    *last = vec![u64::MAX; rows];
                }
                if tab_changed {
                    *last = vec![u64::MAX; rows];
                }
                let ui_state = UiDamageState {
                    selection,
                    hover: tab.link_hover,
                    hover_link_range: tab.hover_link_range,
                };
                let ui_dirty = match LAST_UI_DAMAGE.get_or_init(|| Mutex::new(ui_state)).lock() {
                    Ok(mut last_ui) => {
                        let changed = *last_ui != ui_state;
                        *last_ui = ui_state;
                        changed
                    }
                    Err(_) => true,
                };
                if ui_dirty {
                    dirty_rows.extend(0..rows);
                    for row in 0..rows {
                        last[row] = tab.terminal.row_version(row);
                    }
                } else {
                    for row in 0..rows {
                        let version = tab.terminal.row_version(row);
                        if last[row] != version {
                            dirty_rows.push(row);
                            last[row] = version;
                        }
                    }
                }
                if !dirty_rows.is_empty() {
                    let mut band_start = dirty_rows[0];
                    let mut band_end = dirty_rows[0];
                    for &row in dirty_rows.iter().skip(1) {
                        if row == band_end + 1 {
                            band_end = row;
                        } else {
                            let x = layout.content_x;
                            let y = layout.content_y + (band_start as u32).saturating_mul(cell_h);
                            let w = frame_width.saturating_sub(layout.content_x);
                            let hh = ((band_end - band_start + 1) as u32).saturating_mul(cell_h);
                            damage_rects.push((x, y, w, hh));
                            band_start = row;
                            band_end = row;
                        }
                    }
                    let x = layout.content_x;
                    let y = layout.content_y + (band_start as u32).saturating_mul(cell_h);
                    let w = frame_width.saturating_sub(layout.content_x);
                    let hh = ((band_end - band_start + 1) as u32).saturating_mul(cell_h);
                    damage_rects.push((x, y, w, hh));
                }
            }
            let blink_changed = match LAST_BLINK_STATE.get_or_init(|| Mutex::new(None)).lock() {
                Ok(mut last) => {
                    let changed = last.map_or(true, |prev| prev != cell_blink_on);
                    *last = Some(cell_blink_on);
                    changed
                }
                Err(_) => true,
            };
            if blink_changed {
                dirty_rows.clear();
                dirty_rows.extend(0..tab.terminal.rows as usize);
                damage_rects.clear();
            }

            let mut draw_grid_now =
                draw_grid || !renderer.base_valid() || blink_changed || tab_changed;
            if renderer.base_valid() && damage_rects.is_empty() && !blink_changed && !tab_changed {
                draw_grid_now = false;
            }
            // Disable GPU damage rects; some drivers flicker with partial redraws.
            damage_rects.clear();
            dirty_rows.clear();
            let render_cell_blink_on = cell_blink_on;
            if draw_grid_now {
                let damage = None;
                let base_view = renderer.base_view() as *const wgpu::TextureView;
                let base_view = unsafe { &*base_view };
                renderer.render_grid(
                    encoder,
                    base_view,
                    context,
                    &tab.terminal,
                    tab.terminal.default_bg(),
                    tab.terminal.cursor_color(),
                    glyphs,
                    cell_w,
                    cell_h,
                    layout.content_x,
                    layout.content_y,
                    frame_width,
                    frame_height,
                    selection,
                    tab.link_hover,
                    tab.hover_link_range,
                    tab.link_ranges,
                    render_cell_blink_on,
                    cursor_blink_on,
                    true,
                    |_rects, _glyphs_out, _atlas, _queue, _glyphs_cache| {},
                    damage,
                    None,
                );
                renderer.mark_base_valid();
            }
            let (overlay_rects, overlay_glyphs) = renderer.build_overlay(
                context,
                glyphs,
                |rects, glyphs_out, atlas, queue, glyphs_cache| {
                    build_gpu_overlays(
                        rects,
                        glyphs_out,
                        atlas,
                        queue,
                        glyphs_cache,
                        &tab_visuals,
                        active_tab,
                        tab,
                        frame_width,
                        frame_height,
                        tab_bar_height,
                        border_thickness,
                        border_radius,
                        border_inset,
                        cell_h,
                        start_time,
                        focused,
                        help_visible,
                        history_menu,
                        context_menu,
                        cheatsheet,
                        settings,
                        log_viewer,
                        debug_overlay,
                        fps,
                        notice_text,
                        overlay_active,
                    );
                },
            );
            renderer.blit_base_to_target(encoder, target);
            renderer.render_overlay(encoder, target, context, &overlay_rects, &overlay_glyphs);
            Ok(())
        })
    } else {
        Ok(())
    }
}

pub struct TabRender<'a> {
    pub title: &'a str,
    pub terminal: &'a term_core::terminal::Terminal,
    pub link_hover: Option<(usize, usize)>,
    pub hover_link: Option<&'a str>,
    pub hover_link_range: Option<(usize, usize, usize)>,
    pub link_ranges: Option<&'a [Vec<(usize, usize)>]>,
    pub selection: Option<((usize, usize), (usize, usize))>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UiMode {
    Normal,
    Selecting,
    HistoryMenu,
    ContextMenu,
    Cheatsheet,
    Settings,
    LogViewer,
}

#[cfg(target_os = "macos")]
const HELP_NORMAL: &str = "Alt+T new tab • Alt+Q close tab • Alt+1-9 switch • Ctrl+Tab / Ctrl+Shift+Tab cycle • F5 logs • ˇ autocomplete • Cmd+C/V copy/paste";
#[cfg(not(target_os = "macos"))]
const HELP_NORMAL: &str = "Alt+T new tab • Alt+Q close tab • Alt+1-9 switch • Ctrl+Tab / Ctrl+Shift+Tab cycle • F5 logs • ˇ autocomplete • Ctrl+Shift+C/V copy/paste";

#[cfg(target_os = "macos")]
const HELP_SELECTING: &str =
    "Drag to select • Mouse release copies • Cmd+C copies selection • Esc cancels selection";
#[cfg(not(target_os = "macos"))]
const HELP_SELECTING: &str =
    "Drag to select • Mouse release copies • Ctrl+Shift+C copies selection • Esc cancels selection";

pub(crate) fn current_ui_mode(
    selection: Option<((usize, usize), (usize, usize))>,
    history_menu: &HistoryMenu,
    context_menu: &ContextMenu,
    cheatsheet: &Cheatsheet,
    settings: &SettingsPanel,
    log_viewer: &LogViewer,
) -> UiMode {
    if settings.open {
        UiMode::Settings
    } else if log_viewer.is_open() {
        UiMode::LogViewer
    } else if cheatsheet.is_open() {
        UiMode::Cheatsheet
    } else if history_menu.open {
        UiMode::HistoryMenu
    } else if context_menu.open {
        UiMode::ContextMenu
    } else if selection.is_some() {
        UiMode::Selecting
    } else {
        UiMode::Normal
    }
}

fn help_text_for_mode(mode: UiMode) -> &'static str {
    match mode {
        UiMode::Normal => HELP_NORMAL,
        UiMode::Selecting => HELP_SELECTING,
        UiMode::HistoryMenu => {
            "↑/↓/PgUp/PgDn move • Enter run • Tab/→ insert • Esc close • Scroll to browse"
        }
        UiMode::ContextMenu => {
            "Click Copy/Paste • Right-click to close • Middle-click pastes clipboard"
        }
        UiMode::Cheatsheet => "Cheatsheet open • F2 or Esc closes • Click to dismiss",
        UiMode::Settings => {
            "Settings open • F4/Esc close • S scrollback • P FPS • C copy notice • G render • L logs • D/N downloads"
        }
        UiMode::LogViewer => {
            "Log viewer open • F5/Esc close • / search • F follow • S sudo • R refresh"
        }
    }
}

fn help_text_with_app<'a>(mode: UiMode, app_help: Option<&'a str>) -> Cow<'a, str> {
    let base = help_text_for_mode(mode);
    match app_help {
        Some(text) if !text.trim().is_empty() => Cow::Borrowed(text),
        _ => Cow::Borrowed(base),
    }
}

fn status_label_for_terminal(term: &term_core::terminal::Terminal) -> Option<String> {
    match (term.status_text.as_deref(), term.prompt_status) {
        (Some(status), Some(code)) => Some(format!("{status} • Exit {code}")),
        (Some(status), None) => Some(status.to_string()),
        (None, Some(code)) => Some(format!("Exit {code}")),
        (None, None) => None,
    }
}

pub struct RenderInputs<'a, 'win> {
    pub pixels: &'a mut Pixels<'win>,
    pub gpu_renderer: Option<&'a mut GpuRenderer>,
    pub tabs: &'a [TabRender<'a>],
    pub active_tab: usize,
    pub glyphs: &'a mut GlyphCache,
    pub layout: &'a LayoutMetrics,
    pub tab_bar_height: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    pub cell_w: u32,
    pub cell_h: u32,
    pub start_time: Instant,
    pub focused: bool,
    pub help_visible: bool,
    pub history_menu: &'a mut HistoryMenu,
    pub context_menu: &'a mut ContextMenu,
    pub cheatsheet: &'a Cheatsheet,
    pub settings: &'a SettingsPanel,
    pub log_viewer: &'a LogViewer,
    pub debug_overlay: &'a mut DebugOverlay,
    pub fps: f32,
    pub notice_text: Option<&'a str>,
    pub border_thickness: u32,
    pub border_radius: u32,
    pub border_inset: u32,
    pub cursor_only: bool,
    pub cell_blink_on: bool,
    pub cursor_blink_on: bool,
}

pub struct RenderOutcome {
    pub keep_running: bool,
    pub needs_redraw: bool,
}

/// Centralized render entrypoint that decides GPU vs CPU and also draws the debug overlay.
pub fn render_frame(inputs: RenderInputs<'_, '_>) -> RenderOutcome {
    let RenderInputs {
        pixels,
        mut gpu_renderer,
        tabs,
        active_tab,
        glyphs,
        layout,
        tab_bar_height,
        frame_width,
        frame_height,
        cell_w,
        cell_h,
        start_time,
        focused,
        help_visible,
        history_menu,
        context_menu,
        cheatsheet,
        settings,
        log_viewer,
        debug_overlay,
        fps,
        notice_text,
        border_thickness,
        border_radius,
        border_inset,
        cursor_only,
        cell_blink_on,
        cursor_blink_on,
    } = inputs;

    let overlay_active = debug_overlay.is_active() && !settings.open;
    let log_cursor_cells = *LOG_CURSOR_CELLS.get_or_init(|| {
        std::env::var("TERM_DEBUG_CURSOR_CELLS")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false)
    });

    if log_cursor_cells {
        if let Some(tab) = tabs.get(active_tab) {
            let c = tab.terminal.cursor_col;
            let r = tab.terminal.cursor_row;
            let cell = tab.terminal.display_cell(c, r);
            let snapshot = CellSnapshot {
                col: c,
                row: r,
                view_offset: tab.terminal.view_offset,
                text: cell.text.clone(),
                wide: cell.wide,
                cont: cell.wide_continuation,
                blank: cell.is_blank(),
            };
            let last = LAST_CELL_LOG
                .get_or_init(|| Mutex::new(None))
                .lock()
                .expect("cell log mutex poisoned")
                .clone();
            if last.as_ref() != Some(&snapshot) {
                info!(
                    "debug cell log: cursor ({}, {}) view_offset={} text='{}' wide={} cont={} blank={} fg=({}, {}, {}, {}) bg=({}, {}, {}, {})",
                    c,
                    r,
                    tab.terminal.view_offset,
                    cell.text.escape_default(),
                    cell.wide,
                    cell.wide_continuation,
                    cell.is_blank(),
                    cell.fg.r,
                    cell.fg.g,
                    cell.fg.b,
                    cell.fg.a,
                    cell.bg.r,
                    cell.bg.g,
                    cell.bg.b,
                    cell.bg.a,
                );
                let row_str = row_snapshot(&tab.terminal, r, tab.terminal.cols);
                info!("debug row {}: {}", r, row_str);
                if let Ok(mut guard) = LAST_CELL_LOG.get_or_init(|| Mutex::new(None)).lock() {
                    *guard = Some(snapshot);
                }
            }
        }
    }

    let mut renderer_used = None;
    let mut needs_redraw = false;
    let gpu_allowed = matches!(
        debug_overlay.render_mode(),
        DebugRenderMode::Auto | DebugRenderMode::GpuOnly
    );
    let overlays_open = help_visible
        || history_menu.open
        || context_menu.open
        || cheatsheet.is_open()
        || settings.open
        || log_viewer.is_open();
    let render_cursor = true;

    if gpu_allowed && !settings.open {
        let allow_cursor_only = false;
        if let Some(renderer) = gpu_renderer.as_mut() {
            if allow_cursor_only
                && cursor_only
                && !overlay_active
                && !overlays_open
                && renderer.base_valid()
            {
                let render_result = render_gpu_frame(
                    renderer,
                    pixels,
                    tabs,
                    active_tab,
                    glyphs,
                    layout,
                    frame_width,
                    frame_height,
                    cell_w,
                    cell_h,
                    tab_bar_height,
                    border_thickness,
                    border_radius,
                    border_inset,
                    start_time,
                    focused,
                    help_visible,
                    history_menu,
                    context_menu,
                    cheatsheet,
                    settings,
                    log_viewer,
                    debug_overlay,
                    fps,
                    notice_text,
                    overlay_active,
                    render_cursor,
                    cell_blink_on,
                    cursor_blink_on,
                );

                if let Err(err) = render_result {
                    renderer.invalidate_base();
                    if !handle_render_error(err, frame_width, frame_height, pixels, Some(renderer))
                    {
                        return RenderOutcome {
                            keep_running: false,
                            needs_redraw,
                        };
                    }
                    needs_redraw = true;
                } else {
                    debug_overlay.record_renderer(DebugRendererUsed::Gpu);
                    return RenderOutcome {
                        keep_running: true,
                        needs_redraw,
                    };
                }
            }

            let render_result = render_gpu_frame(
                renderer,
                pixels,
                tabs,
                active_tab,
                glyphs,
                layout,
                frame_width,
                frame_height,
                cell_w,
                cell_h,
                tab_bar_height,
                border_thickness,
                border_radius,
                border_inset,
                start_time,
                focused,
                help_visible,
                history_menu,
                context_menu,
                cheatsheet,
                settings,
                log_viewer,
                debug_overlay,
                fps,
                notice_text,
                overlay_active,
                render_cursor,
                cell_blink_on,
                cursor_blink_on,
            );

            if let Err(err) = render_result {
                renderer.invalidate_base();
                if !handle_render_error(err, frame_width, frame_height, pixels, Some(renderer)) {
                    return RenderOutcome {
                        keep_running: false,
                        needs_redraw,
                    };
                }
                needs_redraw = true;
            } else {
                renderer_used = Some(DebugRendererUsed::Gpu);
                renderer.invalidate_base();
            }
        } else if matches!(debug_overlay.render_mode(), DebugRenderMode::GpuOnly) {
            log::warn!("GPU renderer unavailable; falling back to CPU for debug view");
        }
    }

    let can_cursor_only = cursor_only
        && renderer_used.is_none()
        && !overlay_active
        && !overlays_open
        && matches!(
            debug_overlay.render_mode(),
            DebugRenderMode::Auto | DebugRenderMode::CpuOnly
        );
    if can_cursor_only {
        if let Some(tab) = tabs.get(active_tab) {
            if let Ok(cached) = LAST_CPU_FRAME.get_or_init(|| Mutex::new(None)).lock() {
                let valid = cached
                    .as_ref()
                    .map(|frame| frame.width == frame_width && frame.height == frame_height)
                    .unwrap_or(false);
                if valid {
                    let buffer = pixels.frame_mut();
                    if let Some(frame) = cached.as_ref() {
                        buffer.copy_from_slice(&frame.frame);
                    }
                    if render_cursor {
                        let selection = tab.selection;
                        draw_cursor_overlay(
                            &tab.terminal,
                            glyphs,
                            buffer,
                            frame_width,
                            frame_height,
                            cell_w,
                            cell_h,
                            layout.content_x,
                            layout.content_y,
                            selection,
                            cursor_blink_on,
                            cell_blink_on,
                        );
                    }
                    if let Err(err) = pixels.render() {
                        if !handle_render_error(err, frame_width, frame_height, pixels, None) {
                            return RenderOutcome {
                                keep_running: false,
                                needs_redraw,
                            };
                        }
                        needs_redraw = true;
                    } else {
                        renderer_used = Some(DebugRendererUsed::Cpu);
                    }
                }
            }
        }
    }

    if renderer_used.is_none() {
        let draw_cursor_in_scene = overlays_open && render_cursor;
        draw_scene(
            tabs,
            active_tab,
            glyphs,
            pixels.frame_mut(),
            frame_width,
            frame_height,
            cell_w,
            cell_h,
            layout,
            tab_bar_height,
            start_time,
            focused,
            help_visible,
            history_menu,
            context_menu,
            cheatsheet,
            settings,
            log_viewer,
            fps,
            DebugRendererUsed::Cpu,
            notice_text,
            border_thickness,
            border_radius,
            border_inset,
            !overlay_active,
            !overlay_active,
            cell_blink_on,
            cursor_blink_on,
            draw_cursor_in_scene,
        );
        if !draw_cursor_in_scene {
            if let Ok(mut cached) = LAST_CPU_FRAME.get_or_init(|| Mutex::new(None)).lock() {
                *cached = Some(CachedFrame {
                    width: frame_width,
                    height: frame_height,
                    frame: pixels.frame().to_vec(),
                });
            }
            if render_cursor {
                if let Some(tab) = tabs.get(active_tab) {
                    draw_cursor_overlay(
                        &tab.terminal,
                        glyphs,
                        pixels.frame_mut(),
                        frame_width,
                        frame_height,
                        cell_w,
                        cell_h,
                        layout.content_x,
                        layout.content_y,
                        tab.selection,
                        cursor_blink_on,
                        cell_blink_on,
                    );
                }
            }
        }
        if overlay_active {
            overlay::draw_debug_overlay_cpu(
                glyphs,
                pixels.frame_mut(),
                frame_width,
                frame_height,
                debug_overlay,
                debug_overlay.last_used_renderer(),
                cell_w,
                cell_h,
            );
            if let Some(text) = notice_text {
                let y_offset = overlay_offset_y(glyphs, settings.show_fps);
                draw_notice_overlay_cpu(
                    glyphs,
                    pixels.frame_mut(),
                    frame_width,
                    frame_height,
                    text,
                    y_offset,
                );
            }
        }

        if let Err(err) = pixels.render() {
            if !handle_render_error(err, frame_width, frame_height, pixels, None) {
                return RenderOutcome {
                    keep_running: false,
                    needs_redraw,
                };
            }
            needs_redraw = true;
        } else {
            renderer_used = Some(DebugRendererUsed::Cpu);
        }
    }

    if let Some(used) = renderer_used {
        debug_overlay.record_renderer(used);
    }
    RenderOutcome {
        keep_running: true,
        needs_redraw,
    }
}

fn handle_render_error(
    err: PixelsError,
    frame_width: u32,
    frame_height: u32,
    pixels: &mut Pixels,
    renderer: Option<&mut GpuRenderer>,
) -> bool {
    match err {
        PixelsError::Surface(surface_err) => match surface_err {
            pixels::wgpu::SurfaceError::Lost | pixels::wgpu::SurfaceError::Outdated => {
                let _ = pixels.resize_surface(frame_width, frame_height);
                if let Some(r) = renderer {
                    r.resize(frame_width, frame_height, pixels.queue());
                    r.clear_atlas();
                }
                true
            }
            pixels::wgpu::SurfaceError::Timeout => {
                warn!("render timeout; retrying next frame");
                true
            }
            pixels::wgpu::SurfaceError::OutOfMemory => {
                error!("render failed: out of memory");
                false
            }
        },
        other => {
            error!("render failed: {other:?}");
            false
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_scene(
    tabs: &[TabRender<'_>],
    active: usize,
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    cell_w: u32,
    cell_h: u32,
    layout: &LayoutMetrics,
    tab_bar_height: u32,
    start_time: Instant,
    focused: bool,
    help_visible: bool,
    history_menu: &mut HistoryMenu,
    context_menu: &mut ContextMenu,
    cheatsheet: &Cheatsheet,
    settings: &SettingsPanel,
    log_viewer: &LogViewer,
    fps: f32,
    renderer_used: DebugRendererUsed,
    notice_text: Option<&str>,
    border_thickness: u32,
    border_radius: u32,
    border_inset: u32,
    show_overlays: bool,
    show_fps_notice: bool,
    cell_blink_on: bool,
    cursor_blink_on: bool,
    draw_cursor: bool,
) {
    let bg = tabs
        .get(active)
        .map(|t| t.terminal.default_bg())
        .unwrap_or(term_core::terminal::DEFAULT_BG);
    draw_background(frame, frame_width, frame_height, start_time, bg);
    let tab_visuals: Vec<_> = tabs.iter().map(|t| TabVisual { title: &t.title }).collect();
    draw_tab_bar_cpu(
        &tab_visuals,
        active,
        glyphs,
        frame,
        frame_width,
        frame_height,
        tab_bar_height,
        border_thickness,
        start_time,
    );
    if settings.show_fps {
        draw_fps_in_tab_bar_cpu(
            glyphs,
            frame,
            frame_width,
            frame_height,
            fps,
            renderer_used,
            tab_bar_height,
            border_thickness,
        );
    }

    if let Some(tab) = tabs.get(active) {
        let selection = tab.selection;
        let link_label = tab.hover_link.or_else(|| {
            tab.link_hover
                .and_then(|(c, r)| tab.terminal.display_cell_ref(c, r).hyperlink.as_deref())
        });
        let status_label = status_label_for_terminal(&tab.terminal);
        draw_grid(
            &tab.terminal,
            glyphs,
            frame,
            frame_width,
            frame_height,
            cell_w,
            cell_h,
            layout.content_x,
            layout.content_y,
            selection,
            tab.link_hover,
            cell_blink_on,
            tab.hover_link_range,
            tab.link_ranges,
        );
        draw_ghost_suggestion(
            &tab.terminal,
            glyphs,
            frame,
            frame_width,
            frame_height,
            cell_w,
            cell_h,
            layout.content_x,
            layout.content_y,
        );
        if draw_cursor {
            draw_cursor_overlay(
                &tab.terminal,
                glyphs,
                frame,
                frame_width,
                frame_height,
                cell_w,
                cell_h,
                layout.content_x,
                layout.content_y,
                selection,
                cursor_blink_on,
                cell_blink_on,
            );
        }

        if show_overlays {
            let mode = current_ui_mode(
                selection,
                history_menu,
                context_menu,
                cheatsheet,
                settings,
                log_viewer,
            );
            if help_visible && !cheatsheet.is_open() {
                let help_text = help_text_with_app(mode, tab.terminal.help_text.as_deref());
                draw_help_bar_cpu(
                    glyphs,
                    frame,
                    frame_width,
                    frame_height,
                    cell_h,
                    border_thickness,
                    help_text.as_ref(),
                    link_label.as_deref(),
                    status_label,
                );
            }
        }
    }

    if show_overlays {
        history_menu.draw_cpu(glyphs, frame, frame_width, frame_height);
        context_menu.draw_cpu(glyphs, frame, frame_width, frame_height);
        draw_cheatsheet_cpu(cheatsheet, glyphs, frame, frame_width, frame_height);
        draw_settings_panel_cpu(settings, glyphs, frame, frame_width, frame_height);
        draw_log_viewer_cpu(log_viewer, glyphs, frame, frame_width, frame_height);
    }
    if show_fps_notice {
        if let Some(text) = notice_text {
            let y_offset = overlay_offset_y(glyphs, settings.show_fps);
            draw_notice_overlay_cpu(glyphs, frame, frame_width, frame_height, text, y_offset);
        }
    }

    draw_border_cpu(
        frame,
        frame_width,
        frame_height,
        border_thickness,
        border_radius,
        start_time,
        focused,
        border_inset,
    );
}

fn text_width_px(glyphs: &mut GlyphCache, text: &str) -> i32 {
    text.chars().map(|ch| glyphs.advance_width(ch)).sum()
}

fn overlay_box_height(glyphs: &GlyphCache) -> u32 {
    glyphs.cell_height() as u32 + 12
}

fn overlay_offset_y(glyphs: &GlyphCache, show_fps: bool) -> u32 {
    let _ = (glyphs, show_fps);
    0
}

fn draw_fps_in_tab_bar_cpu(
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    fps: f32,
    renderer_used: DebugRendererUsed,
    tab_bar_height: u32,
    border_thickness: u32,
) {
    if frame_width == 0 || frame_height == 0 {
        return;
    }
    let text = format!("FPS {:.1}", fps);
    let text_w = text_width_px(glyphs, &text).max(1) as u32;
    let icon_size = (glyphs.cell_height().saturating_sub(4)).clamp(6, 12) as u32;
    let icon_gap = 6u32;
    let total_w = text_w + icon_size + icon_gap;
    let x0 = ((frame_width.saturating_sub(total_w)) / 2) as i32;
    let bar_top = border_thickness as i32;
    let bar_bottom = bar_top + tab_bar_height as i32;
    let icon_y0 = bar_top + ((tab_bar_height as i32 - icon_size as i32) / 2).max(0);
    let icon_x0 = x0.max(0);
    let icon_x1 = (icon_x0 + icon_size as i32).min(frame_width as i32);
    let icon_y1 = (icon_y0 + icon_size as i32).min(bar_bottom);
    let icon_color = match renderer_used {
        DebugRendererUsed::Gpu => [70, 200, 110, 255],
        DebugRendererUsed::Cpu => [220, 90, 90, 255],
    };
    fill_rect(
        frame,
        frame_width,
        frame_height,
        icon_x0,
        icon_y0,
        icon_x1,
        icon_y1,
        icon_color,
    );

    let text_y = bar_top
        + ((tab_bar_height as i32 - glyphs.cell_height() as i32) / 2).max(0)
        + (glyphs.cell_height() as i32 - glyphs.baseline());
    draw_text_line_clipped(
        glyphs,
        frame,
        frame_width,
        frame_height,
        icon_x0 + icon_size as i32 + icon_gap as i32,
        text_y,
        &text,
        rgba_bytes(OVERLAY_TEXT),
        (icon_x0 + total_w as i32).min(frame_width as i32),
    );
}

fn draw_notice_overlay_cpu(
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    text: &str,
    y_offset: u32,
) {
    if frame_width == 0 || frame_height == 0 {
        return;
    }
    let text_w = text_width_px(glyphs, text).max(1) as u32;
    let pad = 6u32;
    let margin = 8u32;
    let box_w = text_w + pad * 2;
    let box_h = overlay_box_height(glyphs);
    let x1 = frame_width.saturating_sub(margin);
    let x0 = x1.saturating_sub(box_w);
    let y0 = margin.saturating_add(y_offset);
    let y1 = y0.saturating_add(box_h).min(frame_height);

    fill_rect(
        frame,
        frame_width,
        frame_height,
        x0 as i32,
        y0 as i32,
        x1 as i32,
        y1 as i32,
        rgba_bytes(OVERLAY_BADGE),
    );
    draw_text_line_clipped(
        glyphs,
        frame,
        frame_width,
        frame_height,
        (x0 + pad) as i32,
        (y0 + pad) as i32,
        text,
        rgba_bytes(OVERLAY_TEXT),
        x1 as i32 - pad as i32,
    );
}

fn build_fps_in_tab_bar_gpu(
    rects: &mut Vec<term_core::gpu::RectVertex>,
    glyphs_out: &mut Vec<term_core::gpu::GlyphVertex>,
    atlas: &mut term_core::gpu::GlyphAtlas,
    queue: &pixels::wgpu::Queue,
    glyphs: &mut GlyphCache,
    frame_width: u32,
    frame_height: u32,
    fps: f32,
    renderer_used: DebugRendererUsed,
    tab_bar_height: u32,
    border_thickness: u32,
) {
    if frame_width == 0 || frame_height == 0 {
        return;
    }
    let text = format!("FPS {:.1}", fps);
    let text_w = text_width_px(glyphs, &text).max(1) as f32;
    let icon_size = (glyphs.cell_height().saturating_sub(4)).clamp(6, 12) as f32;
    let icon_gap = 6.0;
    let total_w = text_w + icon_size + icon_gap;
    let x0 = ((frame_width as f32 - total_w) / 2.0).max(0.0);
    let bar_top = border_thickness as f32;
    let bar_bottom = bar_top + tab_bar_height as f32;
    let icon_y0 = bar_top + ((tab_bar_height as f32 - icon_size) / 2.0).max(0.0);
    let icon_x0 = x0;
    let icon_x1 = (icon_x0 + icon_size).min(frame_width as f32);
    let icon_y1 = (icon_y0 + icon_size).min(bar_bottom);
    let icon_color = match renderer_used {
        DebugRendererUsed::Gpu => Rgba {
            r: 70,
            g: 200,
            b: 110,
            a: 255,
        },
        DebugRendererUsed::Cpu => Rgba {
            r: 220,
            g: 90,
            b: 90,
            a: 255,
        },
    };
    GpuRenderer::push_rect(rects, icon_x0, icon_y0, icon_x1, icon_y1, icon_color);
    let text_y = bar_top
        + ((tab_bar_height as f32 - glyphs.cell_height() as f32) / 2.0).max(0.0)
        + (glyphs.cell_height() as f32 - glyphs.baseline() as f32);
    GpuRenderer::push_text_line(
        glyphs,
        atlas,
        glyphs_out,
        &text,
        icon_x0 + icon_size + icon_gap,
        text_y,
        OVERLAY_TEXT,
        queue,
    );
}

fn build_notice_overlay_gpu(
    rects: &mut Vec<term_core::gpu::RectVertex>,
    glyphs_out: &mut Vec<term_core::gpu::GlyphVertex>,
    atlas: &mut term_core::gpu::GlyphAtlas,
    queue: &pixels::wgpu::Queue,
    glyphs: &mut GlyphCache,
    frame_width: u32,
    frame_height: u32,
    text: &str,
    y_offset: u32,
) {
    if frame_width == 0 || frame_height == 0 {
        return;
    }
    let text_w = text_width_px(glyphs, text).max(1) as f32;
    let pad = 6.0;
    let margin = 8.0;
    let box_w = text_w + pad * 2.0;
    let box_h = overlay_box_height(glyphs) as f32;
    let x1 = (frame_width as f32 - margin).max(0.0);
    let x0 = (x1 - box_w).max(0.0);
    let y0 = (margin + y_offset as f32).min(frame_height as f32);
    let y1 = (y0 + box_h).min(frame_height as f32);

    GpuRenderer::push_rect(rects, x0, y0, x1, y1, OVERLAY_BADGE);
    GpuRenderer::push_text_line(
        glyphs,
        atlas,
        glyphs_out,
        text,
        x0 + pad,
        y0 + pad,
        OVERLAY_TEXT,
        queue,
    );
}

fn draw_ghost_suggestion(
    term: &term_core::terminal::Terminal,
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    cell_w: u32,
    cell_h: u32,
    origin_x: u32,
    origin_y: u32,
) {
    let Some(text) = term.ghost_text.as_ref() else {
        return;
    };
    if text.is_empty() || term.view_offset > 0 {
        return;
    }
    if term.cursor_row >= term.rows || term.cursor_col >= term.cols {
        return;
    }
    let start_x = origin_x as i32 + term.cursor_col as i32 * cell_w as i32;
    let y = origin_y as i32 + term.cursor_row as i32 * cell_h as i32;
    let max_x = origin_x as i32 + term.cols as i32 * cell_w as i32;
    let color = [180, 210, 255, 120];
    draw_text_line_clipped(
        glyphs,
        frame,
        frame_width,
        frame_height,
        start_x,
        y,
        text,
        color,
        max_x,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_mode_reports_selection_when_active() {
        let history = HistoryMenu::new();
        let context = ContextMenu::new();
        let cheatsheet = Cheatsheet::new();
        let selection = Some(((0, 0), (1, 0)));
        let settings = SettingsPanel::new();
        let log_viewer = LogViewer::new();

        assert_eq!(
            current_ui_mode(
                selection,
                &history,
                &context,
                &cheatsheet,
                &settings,
                &log_viewer
            ),
            UiMode::Selecting
        );
    }

    #[test]
    fn ui_mode_prefers_settings_over_selection() {
        let selection = Some(((0, 0), (1, 0)));
        let history = HistoryMenu::new();
        let context = ContextMenu::new();
        let cheatsheet = Cheatsheet::new();
        let mut settings = SettingsPanel::new();
        settings.open = true;
        let log_viewer = LogViewer::new();

        assert_eq!(
            current_ui_mode(
                selection,
                &history,
                &context,
                &cheatsheet,
                &settings,
                &log_viewer
            ),
            UiMode::Settings
        );
    }
}
