use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use pixels::Pixels;

use log::info;
use term::debug::{DebugOverlay, DebugRenderMode, DebugRendererUsed};
use term::render::layout::LayoutMetrics;
use term::render::primitives::fill_rect;
use term::render::{
    GlyphCache, TabVisual, build_border_gpu, draw_background, draw_text_line_clipped,
};
use term::render::{
    build_help_bar_gpu, build_tab_bar_gpu, draw_border_cpu, draw_grid, draw_help_bar_cpu,
    draw_tab_bar_cpu,
};
use term::widgets::cheatsheet::{Cheatsheet, build_cheatsheet_gpu, draw_cheatsheet_cpu};
use term::widgets::context::ContextMenu;
use term::widgets::history::HistoryMenu;
use term::widgets::settings::{SettingsPanel, build_settings_panel_gpu, draw_settings_panel_cpu};

use crate::{BORDER_INSET, BORDER_RADIUS, BORDER_THICKNESS, HelpToggle, Tab};
use term::gpu::GpuRenderer;
use term::render::overlay;

static LOG_CURSOR_CELLS: OnceLock<bool> = OnceLock::new();
static LAST_CELL_LOG: OnceLock<Mutex<Option<CellSnapshot>>> = OnceLock::new();

const SEARCH_HILITE: [u8; 4] = [120, 200, 255, 80];

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

fn row_snapshot(term: &term::terminal::Terminal, row: usize, cols: usize) -> String {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UiMode {
    Normal,
    Search,
    Selecting,
    HistoryMenu,
    ContextMenu,
    Cheatsheet,
    Settings,
}

pub(crate) fn current_ui_mode(
    selection: Option<((usize, usize), (usize, usize))>,
    search_active: bool,
    history_menu: &HistoryMenu,
    context_menu: &ContextMenu,
    cheatsheet: &Cheatsheet,
    settings: &SettingsPanel,
) -> UiMode {
    if settings.open {
        UiMode::Settings
    } else if cheatsheet.is_open() {
        UiMode::Cheatsheet
    } else if history_menu.open {
        UiMode::HistoryMenu
    } else if context_menu.open {
        UiMode::ContextMenu
    } else if selection.is_some() {
        UiMode::Selecting
    } else if search_active {
        UiMode::Search
    } else {
        UiMode::Normal
    }
}

fn help_text_for_mode(mode: UiMode) -> &'static str {
    match mode {
        UiMode::Normal => {
            "Alt+T new tab • Alt+Q close tab • Alt+1-5 switch • Ctrl+Tab / Ctrl+Shift+Tab cycle • PgUp/PgDn scrollback • Ctrl+Shift+C/V copy/paste"
        }
        UiMode::Search => {
            "Search scrollback: type to edit query • Enter/F3 next • Shift+Enter/Shift+F3 prev • Esc exits • PgUp/PgDn scroll"
        }
        UiMode::Selecting => {
            "Drag to select • Mouse release copies • Ctrl+Shift+C copies selection • Esc cancels selection"
        }
        UiMode::HistoryMenu => {
            "↑/↓/PgUp/PgDn move • Enter run • Tab/→ insert • Esc close • Scroll to browse"
        }
        UiMode::ContextMenu => {
            "Click Copy/Paste • Right-click to close • Middle-click pastes clipboard"
        }
        UiMode::Cheatsheet => "Cheatsheet open • F2 or Esc closes • Click to dismiss",
        UiMode::Settings => {
            "Settings open • F4 or Esc closes • D download emoji • N download symbols"
        }
    }
}

pub(crate) struct RenderInputs<'a> {
    pub(crate) pixels: &'a mut Pixels,
    pub(crate) gpu_renderer: Option<&'a mut GpuRenderer>,
    pub(crate) tabs: &'a [Tab],
    pub(crate) active_tab: usize,
    pub(crate) glyphs: &'a mut GlyphCache,
    pub(crate) layout: &'a LayoutMetrics,
    pub(crate) tab_bar_height: u32,
    pub(crate) frame_width: u32,
    pub(crate) frame_height: u32,
    pub(crate) cell_w: u32,
    pub(crate) cell_h: u32,
    pub(crate) start_time: Instant,
    pub(crate) focused: bool,
    pub(crate) history_menu: &'a mut HistoryMenu,
    pub(crate) context_menu: &'a mut ContextMenu,
    pub(crate) help: &'a HelpToggle,
    pub(crate) cheatsheet: &'a Cheatsheet,
    pub(crate) settings: &'a SettingsPanel,
    pub(crate) debug_overlay: &'a mut DebugOverlay,
}

/// Centralized render entrypoint that decides GPU vs CPU and also draws the debug overlay.
pub(crate) fn render_frame(inputs: RenderInputs) -> bool {
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
        history_menu,
        context_menu,
        help,
        cheatsheet,
        settings,
        debug_overlay,
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
    let gpu_allowed = matches!(
        debug_overlay.render_mode(),
        DebugRenderMode::Auto | DebugRenderMode::GpuOnly
    );

    if gpu_allowed && !settings.open {
        if let Some(renderer) = gpu_renderer.as_mut() {
            let render_result = if overlay_active {
                pixels.render_with(|encoder, target, context| {
                    renderer.render_grid(
                        encoder,
                        target,
                        context,
                        debug_overlay.preview(),
                        debug_overlay.preview().default_bg(),
                        debug_overlay.preview().cursor_color(),
                        glyphs,
                        cell_w,
                        cell_h,
                        layout.content_x,
                        layout.content_y,
                        frame_width,
                        frame_height,
                        None,
                        |rects, glyphs_out, atlas, queue, glyphs_cache| {
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
                            build_border_gpu(
                                rects,
                                frame_width,
                                frame_height,
                                BORDER_THICKNESS,
                                BORDER_RADIUS,
                                start_time,
                                focused,
                                BORDER_INSET,
                            );
                        },
                    );
                    Ok(())
                })
            } else if let Some(tab) = tabs.get(active_tab) {
                let tab_visuals: Vec<_> =
                    tabs.iter().map(|t| TabVisual { title: &t.title }).collect();
                let selection = super::selection_bounds(tab);
                let link_label = tab
                    .link_hover
                    .and_then(|(c, r)| tab.terminal.display_cell(c, r).hyperlink);
                pixels.render_with(|encoder, target, context| {
                    let search_spans = visible_search_spans(tab, layout.rows);
                    renderer.render_grid(
                        encoder,
                        target,
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
                        |rects, glyphs_out, atlas, queue, glyphs_cache| {
                            if !search_spans.is_empty() {
                                for (row, spans) in &search_spans {
                                    let base_y =
                                        layout.content_y as f32 + *row as f32 * cell_h as f32;
                                    for (col, width) in spans {
                                        let start_col =
                                            (*col).min(tab.terminal.cols.saturating_sub(1));
                                        let span_cells = (*width)
                                            .min(tab.terminal.cols.saturating_sub(start_col));
                                        let x0 = layout.content_x as f32
                                            + start_col as f32 * cell_w as f32;
                                        let x1 = x0 + span_cells as f32 * cell_w as f32;
                                        let y0 = base_y;
                                        let y1 = y0 + cell_h as f32;
                                        GpuRenderer::push_rect(
                                            rects,
                                            x0,
                                            y0,
                                            x1,
                                            y1,
                                            term::terminal::Rgba {
                                                r: SEARCH_HILITE[0],
                                                g: SEARCH_HILITE[1],
                                                b: SEARCH_HILITE[2],
                                                a: SEARCH_HILITE[3],
                                            },
                                        );
                                    }
                                }
                            }
                            build_tab_bar_gpu(
                                rects,
                                glyphs_out,
                                atlas,
                                queue,
                                &tab_visuals,
                                active_tab,
                                glyphs_cache,
                                frame_width,
                                tab_bar_height,
                                BORDER_THICKNESS,
                                start_time,
                            );
                            let mode = current_ui_mode(
                                selection,
                                tab.search.active,
                                history_menu,
                                context_menu,
                                cheatsheet,
                                settings,
                            );
                            let search_label = tab.search.active.then(|| {
                                let mut label = format!("/{}", tab.search.query);
                                if tab.search.total_matches > 0 {
                                    let idx = tab.search.current_index.unwrap_or(0);
                                    label.push_str(&format!(
                                        " {}/{}",
                                        idx, tab.search.total_matches
                                    ));
                                } else if !tab.search.query.is_empty() {
                                    label.push_str(" 0/0");
                                }
                                label
                            });
                            let status_label = search_label.clone().or_else(|| {
                                tab.terminal
                                    .prompt_status
                                    .map(|code| format!("Exit {}", code))
                            });
                            if help.should_show(Instant::now()) && !cheatsheet.is_open() {
                                build_help_bar_gpu(
                                    rects,
                                    glyphs_out,
                                    atlas,
                                    queue,
                                    glyphs_cache,
                                    frame_width,
                                    frame_height,
                                    cell_h,
                                    BORDER_THICKNESS,
                                    help_text_for_mode(mode),
                                    search_label.as_deref().or_else(|| link_label.as_deref()),
                                    status_label,
                                );
                            }
                            build_border_gpu(
                                rects,
                                frame_width,
                                frame_height,
                                BORDER_THICKNESS,
                                BORDER_RADIUS,
                                start_time,
                                focused,
                                BORDER_INSET,
                            );
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
                        },
                    );
                    Ok(())
                })
            } else {
                Ok(())
            };

            if let Err(err) = render_result {
                if !super::handle_render_error(
                    err,
                    frame_width,
                    frame_height,
                    pixels,
                    Some(renderer),
                ) {
                    return false;
                }
            } else {
                renderer_used = Some(DebugRendererUsed::Gpu);
            }
        } else if matches!(debug_overlay.render_mode(), DebugRenderMode::GpuOnly) {
            log::warn!("GPU renderer unavailable; falling back to CPU for debug view");
        }
    }

    if renderer_used.is_none() {
        if overlay_active {
            overlay::draw_debug_scene(
                debug_overlay,
                glyphs,
                pixels.frame_mut(),
                frame_width,
                frame_height,
                cell_w,
                cell_h,
                start_time,
                focused,
                debug_overlay.last_used_renderer(),
            );
            draw_border_cpu(
                pixels.frame_mut(),
                frame_width,
                frame_height,
                BORDER_THICKNESS,
                BORDER_RADIUS,
                start_time,
                focused,
                BORDER_INSET,
            );
        } else {
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
                history_menu,
                context_menu,
                help,
                cheatsheet,
                settings,
            );
        }

        if let Err(err) = pixels.render() {
            if !super::handle_render_error(err, frame_width, frame_height, pixels, None) {
                return false;
            }
        } else {
            renderer_used = Some(DebugRendererUsed::Cpu);
        }
    }

    if let Some(used) = renderer_used {
        debug_overlay.record_renderer(used);
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn draw_scene(
    tabs: &[Tab],
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
    history_menu: &mut HistoryMenu,
    context_menu: &mut ContextMenu,
    help: &HelpToggle,
    cheatsheet: &Cheatsheet,
    settings: &SettingsPanel,
) {
    let bg = tabs
        .get(active)
        .map(|t| t.terminal.default_bg())
        .unwrap_or(term::terminal::DEFAULT_BG);
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
        BORDER_THICKNESS,
        start_time,
    );

    if let Some(tab) = tabs.get(active) {
        let selection = super::selection_bounds(tab);
        let link_label = tab
            .link_hover
            .and_then(|(c, r)| tab.terminal.display_cell(c, r).hyperlink);
        let search_label = tab.search.active.then(|| {
            let mut label = format!("/{}", tab.search.query);
            if tab.search.total_matches > 0 {
                let idx = tab.search.current_index.unwrap_or(0);
                label.push_str(&format!(" {}/{}", idx, tab.search.total_matches));
            } else if !tab.search.query.is_empty() {
                label.push_str(" 0/0");
            }
            label
        });
        let status_label = search_label.clone().or_else(|| {
            tab.terminal
                .prompt_status
                .map(|code| format!("Exit {}", code))
        });
        let search_spans = visible_search_spans(tab, layout.rows);
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
            start_time.elapsed().as_millis() / 500 % 2 == 0,
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

        if !search_spans.is_empty() {
            draw_search_highlights_cpu(
                frame,
                frame_width,
                frame_height,
                cell_w,
                cell_h,
                layout.content_x,
                layout.content_y,
                tab.terminal.cols,
                &search_spans,
            );
        }

        if tab.terminal.bell_flash_active() {
            flash_frame(frame, 30);
        }

        let mode = current_ui_mode(
            selection,
            tab.search.active,
            history_menu,
            context_menu,
            cheatsheet,
            settings,
        );
        if help.should_show(Instant::now()) && !cheatsheet.is_open() {
            draw_help_bar_cpu(
                glyphs,
                frame,
                frame_width,
                frame_height,
                cell_h,
                BORDER_THICKNESS,
                help_text_for_mode(mode),
                search_label.as_deref().or_else(|| link_label.as_deref()),
                status_label,
            );
        }
    }

    history_menu.draw_cpu(glyphs, frame, frame_width, frame_height);
    context_menu.draw_cpu(glyphs, frame, frame_width, frame_height);
    draw_cheatsheet_cpu(cheatsheet, glyphs, frame, frame_width, frame_height);
    draw_settings_panel_cpu(settings, glyphs, frame, frame_width, frame_height);

    draw_border_cpu(
        frame,
        frame_width,
        frame_height,
        BORDER_THICKNESS,
        BORDER_RADIUS,
        start_time,
        focused,
        BORDER_INSET,
    );
}

fn flash_frame(frame: &mut [u8], amount: u8) {
    for px in frame.chunks_exact_mut(4) {
        px[0] = px[0].saturating_add(amount);
        px[1] = px[1].saturating_add(amount);
        px[2] = px[2].saturating_add(amount);
    }
}

fn visible_search_spans(tab: &Tab, rows: usize) -> Vec<(usize, Vec<(usize, usize)>)> {
    if !tab.search.active || tab.search.query.is_empty() || tab.terminal.in_alt_screen() {
        return Vec::new();
    }
    let needle = tab.search.query.to_lowercase();
    let step = needle.len().max(1);
    let needle_width = tab.search.query.chars().count().max(1);
    let hist_len = tab.terminal.scrollback.len();
    let base_line = hist_len.saturating_sub(tab.terminal.view_offset);
    let mut out = Vec::new();
    for row in 0..rows.min(tab.terminal.rows) {
        let line_idx = base_line + row;
        if let Some(text) = tab.terminal.line_text(line_idx) {
            let lower = text.to_lowercase();
            let mut offset = 0;
            let mut spans = Vec::new();
            while let Some(pos) = lower[offset..].find(&needle) {
                let abs = offset + pos;
                let col = lower[..abs].chars().count();
                spans.push((col, needle_width));
                offset = abs + step;
            }
            if !spans.is_empty() {
                out.push((row, spans));
            }
        }
    }
    out
}

fn draw_search_highlights_cpu(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    cell_w: u32,
    cell_h: u32,
    origin_x: u32,
    origin_y: u32,
    cols: usize,
    spans: &[(usize, Vec<(usize, usize)>)],
) {
    for (row, spans_for_row) in spans {
        let base_y = origin_y as i32 + *row as i32 * cell_h as i32;
        for (col, width) in spans_for_row {
            if cols == 0 {
                continue;
            }
            let start_col = (*col).min(cols.saturating_sub(1));
            let span_cells = (*width).min(cols.saturating_sub(start_col));
            let x0 = origin_x as i32 + start_col as i32 * cell_w as i32;
            let x1 = x0 + span_cells as i32 * cell_w as i32;
            let y0 = base_y;
            let y1 = y0 + cell_h as i32;
            fill_rect(
                frame,
                frame_width,
                frame_height,
                x0,
                y0,
                x1,
                y1,
                SEARCH_HILITE,
            );
        }
    }
}

fn draw_ghost_suggestion(
    term: &term::terminal::Terminal,
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

        assert_eq!(
            current_ui_mode(selection, false, &history, &context, &cheatsheet, &settings),
            UiMode::Selecting
        );
    }
}
