use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use pixels::Pixels;

use log::info;
use term_core::debug::{DebugOverlay, DebugRenderMode, DebugRendererUsed};
use term_core::render::layout::LayoutMetrics;
use term_core::render::{
    GlyphCache, TabVisual, build_border_gpu, draw_background, draw_text_line_clipped,
};
use term_core::render::{
    build_help_bar_gpu, build_tab_bar_gpu, draw_border_cpu, draw_grid, draw_help_bar_cpu,
    draw_tab_bar_cpu,
};
use term_core::widgets::cheatsheet::{Cheatsheet, build_cheatsheet_gpu, draw_cheatsheet_cpu};
use term_core::widgets::context::ContextMenu;
use term_core::widgets::history::HistoryMenu;
use term_core::widgets::settings::{SettingsPanel, build_settings_panel_gpu, draw_settings_panel_cpu};

use crate::{BORDER_INSET, BORDER_RADIUS, BORDER_THICKNESS, HelpToggle, Tab};
use term_core::gpu::GpuRenderer;
use term_core::render::overlay;

static LOG_CURSOR_CELLS: OnceLock<bool> = OnceLock::new();
static LAST_CELL_LOG: OnceLock<Mutex<Option<CellSnapshot>>> = OnceLock::new();

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UiMode {
    Normal,
    Selecting,
    HistoryMenu,
    ContextMenu,
    Cheatsheet,
    Settings,
}

pub(crate) fn current_ui_mode(
    selection: Option<((usize, usize), (usize, usize))>,
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
    } else {
        UiMode::Normal
    }
}

fn help_text_for_mode(mode: UiMode) -> &'static str {
    match mode {
        UiMode::Normal => {
            "Alt+T new tab • Alt+Q close tab • Alt+1-5 switch • Ctrl+Tab / Ctrl+Shift+Tab cycle • ˇ autocomplete • Ctrl+Shift+C/V copy/paste"
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
                pixels.render_with(|encoder, target, context| {
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
                                history_menu,
                                context_menu,
                                cheatsheet,
                                settings,
                            );
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
                                    tab.link_hover
                                        .and_then(|(c, r)| {
                                            tab.terminal.display_cell(c, r).hyperlink.clone()
                                        })
                                        .as_deref(),
                                    tab.terminal
                                        .prompt_status
                                        .map(|code| format!("Exit {}", code)),
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
        BORDER_THICKNESS,
        start_time,
    );

    if let Some(tab) = tabs.get(active) {
        let selection = super::selection_bounds(tab);
        let link_label = tab
            .link_hover
            .and_then(|(c, r)| tab.terminal.display_cell(c, r).hyperlink);
        let status_label = tab
            .terminal
            .prompt_status
            .map(|code| format!("Exit {}", code));
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

        let mode = current_ui_mode(selection, history_menu, context_menu, cheatsheet, settings);
        if help.should_show(Instant::now()) && !cheatsheet.is_open() {
            draw_help_bar_cpu(
                glyphs,
                frame,
                frame_width,
                frame_height,
                cell_h,
                BORDER_THICKNESS,
                help_text_for_mode(mode),
                link_label.as_deref(),
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

        assert_eq!(
            current_ui_mode(selection, &history, &context, &cheatsheet, &settings),
            UiMode::Selecting
        );
    }
}
