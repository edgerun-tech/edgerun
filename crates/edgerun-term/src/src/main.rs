use std::collections::{HashMap, HashSet, hash_map::Entry};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc::TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use dirs::config_dir;
use log::{error, info, warn};
use pixels::{Error as PixelsError, Pixels, SurfaceTexture, wgpu};
use portable_pty::CommandBuilder;
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use term::debug::DebugOverlay;
use term::logging;
use term::render::layout::{LayoutMetrics, compute_layout};
use term::render::{FONT_DATA, FONT_SIZE, GlyphCache};
use term::terminal::Rgba;
use term::terminal::{
    GridPerformer, Terminal, copy_text_to_clipboard, selection_text, write_bytes,
};
use term::widgets::cheatsheet::Cheatsheet;
use term::widgets::context::{ContextAction, ContextMenu};
use term::widgets::history::{HistoryMenu, MenuColumn, MenuEntry};
use term::widgets::settings::{SettingsPanel, SystemFont};
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::{EventLoopBuilder, EventLoopProxy},
    keyboard::{Key, ModifiersState, NamedKey},
    platform::startup_notify::{
        EventLoopExtStartupNotify, WindowBuilderExtStartupNotify, reset_activation_token_env,
    },
    window::WindowBuilder,
};

use term::gpu::GpuRenderer;
mod app_render;
mod input;
mod suggest;
mod tab;
#[cfg(test)]
#[path = "tests/main_tests.rs"]
mod tests;
use app_render::{RenderInputs, render_frame};
use suggest::{
    fix_command_paths, looks_like_path, path_completion_suggestions, recent_dirs_for_tab,
};
use tab::{
    CwdHistoryEntry, Tab, TabKind, preferred_cwd, refresh_cwd_for_tabs, refresh_tab_titles,
    resize_tabs_to_layout, selection_bounds, spawn_shell_tab, tab_current_dir,
};

#[derive(Debug)]
enum AppEvent {
    Wake,
    Fonts(Vec<Arc<Vec<u8>>>),
    DownloadStatus(String),
    PrimaryFont(Arc<Vec<u8>>),
}

struct HelpToggle {
    visible: bool,
}

impl HelpToggle {
    fn new(_now: Instant, _timeout: Duration) -> Self {
        let visible = load_help_visible_flag().unwrap_or(true);
        Self { visible }
    }

    fn toggle(&mut self, _now: Instant) {
        self.visible = !self.visible;
        persist_help_visible_flag(self.visible);
    }

    fn bump(&mut self, _now: Instant) {}

    fn should_show(&self, now: Instant) -> bool {
        let _ = now;
        self.visible
    }
}

const PADDING_X: u32 = 16;
const PADDING_Y: u32 = 12;
const BORDER_THICKNESS: u32 = 1;
const BORDER_RADIUS: u32 = 10;
const BORDER_INSET: u32 = 0;
fn main() {
    logging::init();
    if let Err(err) = run() {
        error!("error: {err}");
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    // Respect compositor-provided activation token (e.g. Hyprland binds) so the new
    // window grabs focus on the originating workspace instead of bouncing to
    // whichever workspace already has focus.
    let activation_token = event_loop.read_token_from_env();
    let mut window_builder = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(900.0, 600.0))
        .with_title("term");
    if let Some(token) = activation_token {
        window_builder = window_builder.with_activation_token(token);
    }
    let window = window_builder.build(&event_loop)?;
    reset_activation_token_env();

    let mut glyphs = GlyphCache::new(Arc::new(FONT_DATA.to_vec()), FONT_SIZE);
    spawn_font_loader(proxy.clone());
    let (mut cell_w, mut cell_h) = glyphs.cell_size();
    let mut history_menu = HistoryMenu::new();
    let mut context_menu = ContextMenu::new();
    let mut autocomplete = AutocompleteEngine::load();
    let mut cwd_history: Vec<CwdHistoryEntry> = Vec::new();
    let mut settings = SettingsPanel::new();

    let size = window.inner_size();
    let surface_texture = SurfaceTexture::new(size.width, size.height, &window);
    let mut pixels = Pixels::new(size.width, size.height, surface_texture)?;
    let gpu_enabled = env::var("TERM_GPU")
        .map(|v| {
            let v = v.to_ascii_lowercase();
            !(v == "0" || v == "false" || v == "off" || v == "no")
        })
        .unwrap_or(true);
    let mut gpu_renderer = if gpu_enabled {
        Some(GpuRenderer::new(
            pixels.context(),
            pixels.surface_texture_format(),
        ))
    } else {
        None
    };
    let mut frame_width = size.width;
    let mut frame_height = size.height;
    let start_time = Instant::now();
    // Throttle expensive /proc cwd polling.
    let mut last_cwd_poll = start_time
        .checked_sub(Duration::from_secs(1))
        .unwrap_or(start_time);
    let mut tab_bar_height = cell_h + 12;
    let mut layout = compute_layout(
        frame_width,
        frame_height,
        cell_w,
        cell_h,
        tab_bar_height,
        BORDER_THICKNESS,
        BORDER_INSET,
        PADDING_X,
        PADDING_Y,
    );
    let mut help = HelpToggle::new(start_time, Duration::from_secs(8));
    let mut cheatsheet = Cheatsheet::new();
    let mut debug_overlay = DebugOverlay::new(layout.cols, layout.rows);

    let mut tabs = Vec::new();
    tabs.push(spawn_shell_tab(
        layout.cols as u16,
        layout.rows as u16,
        layout.usable_width,
        layout.usable_height,
        proxy.clone(),
        None,
        settings.scrollback_enabled,
    )?);
    refresh_tab_titles(&mut tabs);
    let mut active_tab: usize = 0;
    let mut modifiers = ModifiersState::default();
    let mut focused = true;
    let mut last_cursor_pos: Option<(f64, f64)> = None;
    let mut needs_redraw = true;

    event_loop.run(move |event, elwt| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            for tab in &mut tabs {
                match &mut tab.kind {
                    TabKind::Shell(shell) => {
                        if let Some(child) = shell.child.as_mut() {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                    }
                }
            }
            elwt.exit();
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(new_size),
            ..
        } => {
            let (new_layout, new_w, new_h) = apply_window_resize(
                new_size,
                cell_w,
                cell_h,
                tab_bar_height,
                &mut pixels,
                &mut tabs,
            );
            layout = new_layout;
            frame_width = new_w;
            frame_height = new_h;
            debug_overlay.resize(layout.cols, layout.rows);
            if let Some(renderer) = gpu_renderer.as_mut() {
                renderer.resize(frame_width, frame_height, pixels.queue());
            }
            window.request_redraw();
            needs_redraw = true;
        }
        Event::WindowEvent {
            event:
                WindowEvent::ScaleFactorChanged {
                    scale_factor: _,
                    inner_size_writer: _,
                    ..
                },
            ..
        } => {
            let inner = window.inner_size();
            let (new_layout, new_w, new_h) = apply_window_resize(
                inner,
                cell_w,
                cell_h,
                tab_bar_height,
                &mut pixels,
                &mut tabs,
            );
            layout = new_layout;
            frame_width = new_w;
            frame_height = new_h;
            debug_overlay.resize(layout.cols, layout.rows);
            if let Some(renderer) = gpu_renderer.as_mut() {
                renderer.resize(frame_width, frame_height, pixels.queue());
            }
            window.request_redraw();
            needs_redraw = true;
        }
        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
            let mut active_window_title: Option<String> = None;
            for (idx, tab) in tabs.iter_mut().enumerate() {
                let TabKind::Shell(shell) = &mut tab.kind;
                let mut disconnected = false;
                loop {
                    match shell.rx.try_recv() {
                        Ok(chunk) => {
                            let mut performer = GridPerformer {
                                grid: &mut tab.terminal,
                                writer: shell.writer.clone(),
                                app_cursor_keys: &mut shell.app_cursor_keys,
                                dcs_state: None,
                            };
                            for byte in chunk {
                                shell.parser.advance(&mut performer, byte);
                            }
                            tab.terminal.view_offset = 0;
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => {
                            disconnected = true;
                            break;
                        }
                    }
                }
                if disconnected && !shell.exited {
                    shell.exited = true;
                    tab.selection_anchor = None;
                    tab.selection_edge = None;
                    tab.selecting = false;
                    if tab.terminal.alt_active {
                        tab.terminal.leave_alt_screen_preserve();
                    }
                    if let Some(mut child) = shell.child.take() {
                        let _ = child.wait();
                    }
                }

                if tab.pending_cursor_to_line_end
                    && !tab.terminal.in_alt_screen()
                    && tab.terminal.view_offset == 0
                {
                    tab.terminal.move_to_line_end();
                    tab.pending_cursor_to_line_end = false;
                }

                if tab.title.is_empty() {
                    tab.title = format!("Tab {}", idx + 1);
                }
                if let Some(title) = tab.terminal.window_title().map(|t| t.to_string()) {
                    if tab.title != title {
                        tab.title = title;
                    }
                }
                if idx == active_tab {
                    active_window_title = tab.terminal.window_title().map(|t| t.to_string());
                }
            }
            window.set_title(active_window_title.as_deref().unwrap_or("term"));

            let now = Instant::now();
            if now.duration_since(last_cwd_poll) >= Duration::from_secs(1) {
                refresh_cwd_for_tabs(&mut tabs, &mut cwd_history);
                last_cwd_poll = now;
            }

            let keep_running = render_frame(RenderInputs {
                pixels: &mut pixels,
                gpu_renderer: gpu_renderer.as_mut(),
                tabs: &tabs,
                active_tab,
                glyphs: &mut glyphs,
                layout: &layout,
                tab_bar_height,
                frame_width,
                frame_height,
                cell_w,
                cell_h,
                start_time,
                focused,
                history_menu: &mut history_menu,
                context_menu: &mut context_menu,
                help: &help,
                cheatsheet: &cheatsheet,
                settings: &settings,
                debug_overlay: &mut debug_overlay,
            });
            if !keep_running {
                elwt.exit();
            }
            needs_redraw = false;
        }
        Event::UserEvent(app_event) => match app_event {
            AppEvent::Wake => {
                needs_redraw = true;
                window.request_redraw();
            }
            AppEvent::Fonts(fonts) => {
                let (old_w, old_h) = glyphs.cell_size();
                glyphs.add_fonts(fonts);
                if let Some(renderer) = gpu_renderer.as_mut() {
                    renderer.clear_atlas();
                }
                let (new_w, new_h) = glyphs.cell_size();
                if new_w != old_w || new_h != old_h {
                    cell_w = new_w;
                    cell_h = new_h;
                    tab_bar_height = cell_h + 12;
                    layout = compute_layout(
                        frame_width,
                        frame_height,
                        cell_w,
                        cell_h,
                        tab_bar_height,
                        BORDER_THICKNESS,
                        BORDER_INSET,
                        PADDING_X,
                        PADDING_Y,
                    );
                    resize_tabs_to_layout(&mut tabs, &layout);
                    debug_overlay.resize(layout.cols, layout.rows);
                }
                needs_redraw = true;
                window.request_redraw();
            }
            AppEvent::PrimaryFont(font) => {
                let (old_w, old_h) = glyphs.cell_size();
                glyphs.set_primary_font(font);
                if let Some(renderer) = gpu_renderer.as_mut() {
                    renderer.clear_atlas();
                }
                let (new_w, new_h) = glyphs.cell_size();
                if new_w != old_w || new_h != old_h {
                    cell_w = new_w;
                    cell_h = new_h;
                    tab_bar_height = cell_h + 12;
                    layout = compute_layout(
                        frame_width,
                        frame_height,
                        cell_w,
                        cell_h,
                        tab_bar_height,
                        BORDER_THICKNESS,
                        BORDER_INSET,
                        PADDING_X,
                        PADDING_Y,
                    );
                    resize_tabs_to_layout(&mut tabs, &layout);
                    debug_overlay.resize(layout.cols, layout.rows);
                }
                needs_redraw = true;
                window.request_redraw();
            }
            AppEvent::DownloadStatus(msg) => {
                settings.set_status(msg);
                needs_redraw = true;
                window.request_redraw();
            }
        },
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } => {
            if std::env::var("TERM_DEBUG_CURSOR_CELLS")
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
                .unwrap_or(false)
            {
                let state = match event.state {
                    ElementState::Pressed => "down",
                    ElementState::Released => "up",
                };
                let key_label = match &event.logical_key {
                    Key::Character(txt) => format!("char({})", txt.escape_default()),
                    Key::Named(name) => format!("named({:?})", name),
                    _ => format!("{:?}", event.logical_key),
                };
                info!("debug input: key {} {}", key_label, state);
            }
            // If the user is viewing scrollback, any key press should jump back to live output
            // and not be forwarded. This avoids cursor/history arrows adding text while the
            // screen appears "stuck" on older content.
            if let Some(tab) = tabs.get_mut(active_tab) {
                if tab.terminal.view_offset > 0 {
                    tab.selection_anchor = None;
                    tab.selection_edge = None;
                    tab.selecting = false;
                    tab.terminal.view_offset = 0;
                    needs_redraw = true;
                    return;
                }
            }

            if context_menu.open {
                context_menu.close();
            }
            help.bump(Instant::now());

            if matches!(event.logical_key, Key::Named(NamedKey::F3))
                && event.state == ElementState::Pressed
            {
                cheatsheet.close();
                history_menu.close();
                context_menu.close();
                settings.close();
                debug_overlay.toggle();
                needs_redraw = true;
                return;
            }

            if matches!(event.logical_key, Key::Named(NamedKey::F4))
                && event.state == ElementState::Pressed
            {
                if debug_overlay.is_active() {
                    debug_overlay.close();
                }
                settings.toggle();
                if settings.open {
                    settings.refresh_system_fonts();
                }
                cheatsheet.close();
                history_menu.close();
                context_menu.close();
                needs_redraw = true;
                return;
            }

            if debug_overlay.is_active() {
                if event.state == ElementState::Pressed {
                    match &event.logical_key {
                        Key::Character(text) if text.eq_ignore_ascii_case("r") => {
                            debug_overlay.cycle_render_mode();
                        }
                        Key::Character(text) if text.eq_ignore_ascii_case("i") => {
                            debug_overlay.cycle_input_mode();
                        }
                        Key::Named(NamedKey::Escape) => debug_overlay.close(),
                        _ => {}
                    }
                }
                needs_redraw = true;
                return;
            }

            if matches!(event.logical_key, Key::Named(NamedKey::F2))
                && event.state == ElementState::Pressed
            {
                if cheatsheet.is_open() {
                    cheatsheet.close();
                } else {
                    history_menu.close();
                    context_menu.close();
                    settings.close();
                    cheatsheet.open();
                }
                needs_redraw = true;
                return;
            }

            if cheatsheet.is_open() {
                handle_cheatsheet_key(event, &mut cheatsheet);
                needs_redraw = true;
                return;
            }
            if settings.open {
                handle_settings_key(event, &mut settings, &proxy, &mut tabs);
                needs_redraw = true;
                return;
            }
            if history_menu.open {
                if let Some(text) = handle_history_key(
                    event,
                    &mut history_menu,
                    &mut tabs,
                    active_tab,
                    &cwd_history,
                    &mut autocomplete,
                ) {
                    autocomplete.record_accept(&text);
                }
            } else {
                handle_key(
                    event,
                    modifiers,
                    &mut tabs,
                    &mut active_tab,
                    &layout,
                    &mut history_menu,
                    &mut autocomplete,
                    &proxy,
                    &mut help,
                    &settings,
                    &cwd_history,
                );
            }
            needs_redraw = true;
        }
        Event::WindowEvent {
            event: WindowEvent::ModifiersChanged(new_mods),
            ..
        } => {
            modifiers = new_mods.state();
        }
        Event::WindowEvent {
            event: WindowEvent::MouseWheel { delta, .. },
            ..
        } => {
            if cheatsheet.is_open() {
                return;
            }
            help.bump(Instant::now());
            if context_menu.open {
                context_menu.close();
                return;
            }
            if let Some((mx, my)) = last_cursor_pos {
                if let Some(cell) = pos_to_cell(mx, my, &layout, cell_w, cell_h, active_tab, &tabs)
                {
                    if let Some(tab) = tabs.get_mut(active_tab) {
                        if tab.terminal.mouse_btn_report {
                            let kind = match delta {
                                winit::event::MouseScrollDelta::LineDelta(_, y) if y > 0.0 => {
                                    term::terminal::MouseEventKind::WheelUp
                                }
                                winit::event::MouseScrollDelta::LineDelta(_, y) if y < 0.0 => {
                                    term::terminal::MouseEventKind::WheelDown
                                }
                                winit::event::MouseScrollDelta::PixelDelta(pos) if pos.y > 0.0 => {
                                    term::terminal::MouseEventKind::WheelUp
                                }
                                winit::event::MouseScrollDelta::PixelDelta(pos) if pos.y < 0.0 => {
                                    term::terminal::MouseEventKind::WheelDown
                                }
                                _ => term::terminal::MouseEventKind::WheelUp,
                            };
                            let TabKind::Shell(shell) = &tab.kind;
                            tab.terminal
                                .report_mouse_event(cell.0, cell.1, kind, &shell.writer);
                            needs_redraw = true;
                            return;
                        }
                    }
                }
            }
            if history_menu.open {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        (pos.y / cell_h.max(1) as f64) as f32
                    }
                };
                if lines != 0.0 {
                    let step = if lines > 0.0 {
                        lines.ceil() as i32
                    } else {
                        lines.floor() as i32
                    };
                    history_menu.move_selection(step);
                }
            } else if let Some(tab) = tabs.get_mut(active_tab) {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        (pos.y / cell_h.max(1) as f64) as f32
                    }
                };
                if lines != 0.0 && !tab.terminal.in_alt_screen() {
                    let step = if lines > 0.0 {
                        lines.ceil() as i32
                    } else {
                        lines.floor() as i32
                    };
                    tab.terminal.scroll_view_offset(step);
                }
            }
            needs_redraw = true;
        }
        Event::WindowEvent {
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } => {
            last_cursor_pos = Some((position.x, position.y));
            if cheatsheet.is_open() {
                return;
            }
            help.bump(Instant::now());
            if context_menu.open {
                context_menu.update_hover(position.x, position.y);
            } else if history_menu.open {
                history_menu.update_hover(position.x, position.y);
            } else if let Some(cell) = pos_to_cell(
                position.x, position.y, &layout, cell_w, cell_h, active_tab, &tabs,
            ) {
                if let Some(tab) = tabs.get_mut(active_tab) {
                    let reporting =
                        tab.terminal.mouse_btn_report || tab.terminal.mouse_motion_report;
                    if reporting {
                        let TabKind::Shell(shell) = &tab.kind;
                        tab.terminal.report_mouse_event(
                            cell.0,
                            cell.1,
                            term::terminal::MouseEventKind::Motion,
                            &shell.writer,
                        );
                    } else if tab.selecting {
                        tab.selection_edge = Some(cell);
                        tab.link_hover = None;
                    } else {
                        let cell_data = tab.terminal.display_cell(cell.0, cell.1);
                        tab.link_hover = if cell_data.hyperlink.is_some() {
                            Some(cell)
                        } else {
                            None
                        };
                    }
                }
            } else if let Some(tab) = tabs.get_mut(active_tab) {
                tab.link_hover = None;
            }
            needs_redraw = true;
        }
        Event::WindowEvent {
            event: WindowEvent::MouseInput { state, button, .. },
            ..
        } => {
            if cheatsheet.is_open() {
                if state == ElementState::Released {
                    cheatsheet.close();
                    needs_redraw = true;
                }
                return;
            }
            if history_menu.open {
                if button == MouseButton::Left && state == ElementState::Released {
                    if let Some((mx, my)) = last_cursor_pos {
                        if let Some(sel_idx) = history_menu.click(mx, my) {
                            if let Some(text) = apply_history_selection(
                                &mut history_menu,
                                &mut tabs,
                                active_tab,
                                false,
                                &cwd_history,
                            ) {
                                autocomplete.record_accept(&text);
                            } else {
                                history_menu.selected =
                                    sel_idx.min(history_menu.entries.len().saturating_sub(1));
                            }
                        }
                    }
                } else if button == MouseButton::Right && state == ElementState::Released {
                    history_menu.close();
                }
                return;
            }
            if context_menu.open {
                help.bump(Instant::now());
                if let Some((mx, my)) = last_cursor_pos
                    && button == MouseButton::Left
                    && state == ElementState::Released
                    && let Some(action) = context_menu.click(mx, my)
                {
                    match action {
                        ContextAction::Copy => {
                            if let Some(tab) = tabs.get_mut(active_tab)
                                && let Some((a, b)) = selection_bounds(tab)
                            {
                                copy_selection_to_clipboard(&tab.terminal, a, b);
                            }
                        }
                        ContextAction::Paste => {
                            if let Some(tab) = tabs.get(active_tab) {
                                let TabKind::Shell(shell) = &tab.kind;
                                paste_clipboard(shell.writer.clone(), tab.terminal.bracketed_paste);
                            }
                        }
                    }
                }
                if state == ElementState::Released {
                    context_menu.close();
                }
                return;
            }
            match button {
                MouseButton::Left => {
                    if let Some((mx, my)) = last_cursor_pos {
                        help.bump(Instant::now());
                        let cell = pos_to_cell(mx, my, &layout, cell_w, cell_h, active_tab, &tabs);
                        match state {
                            ElementState::Pressed => {
                                if let Some(tab) = tabs.get_mut(active_tab) {
                                    let reporting = tab.terminal.mouse_btn_report;
                                    if reporting {
                                        if let Some((c, r)) = cell {
                                            let TabKind::Shell(shell) = &tab.kind;
                                            tab.terminal.report_mouse_event(
                                                c,
                                                r,
                                                term::terminal::MouseEventKind::Press(0),
                                                &shell.writer,
                                            );
                                        }
                                        tab.selecting = false;
                                    } else {
                                        tab.selecting = false;
                                        tab.selection_anchor = cell;
                                        tab.selection_edge = cell;
                                        tab.selecting = cell.is_some();
                                    }
                                }
                            }
                            ElementState::Released => {
                                if let Some(tab) = tabs.get_mut(active_tab) {
                                    let reporting = tab.terminal.mouse_btn_report;
                                    if reporting {
                                        if let Some((c, r)) = cell {
                                            let TabKind::Shell(shell) = &tab.kind;
                                            tab.terminal.report_mouse_event(
                                                c,
                                                r,
                                                term::terminal::MouseEventKind::Release,
                                                &shell.writer,
                                            );
                                        }
                                    } else if tab.selecting {
                                        tab.selecting = false;
                                        if let (Some(a), Some(b)) =
                                            (tab.selection_anchor, tab.selection_edge)
                                        {
                                            copy_selection_to_clipboard(&tab.terminal, a, b);
                                        }
                                    } else if let Some((c, r)) = cell {
                                        let cell = tab.terminal.display_cell(c, r);
                                        if let Some(link) = cell.hyperlink {
                                            copy_text_to_clipboard(&link);
                                            // If Ctrl is held, attempt to open in system handler.
                                            if modifiers.control_key() {
                                                open_link(&link);
                                            }
                                        }
                                        tab.selection_anchor = None;
                                        tab.selection_edge = None;
                                    } else {
                                        tab.selection_anchor = None;
                                        tab.selection_edge = None;
                                    }
                                }
                            }
                        }
                    }
                }
                MouseButton::Right if state == ElementState::Released => {
                    help.bump(Instant::now());
                    if let Some((mx, my)) = last_cursor_pos {
                        let can_copy = tabs.get(active_tab).and_then(selection_bounds).is_some();
                        context_menu.open(mx, my, can_copy);
                    }
                }
                MouseButton::Middle if state == ElementState::Released => {
                    help.bump(Instant::now());
                    if let Some(tab) = tabs.get(active_tab) {
                        let TabKind::Shell(shell) = &tab.kind;
                        paste_clipboard(shell.writer.clone(), tab.terminal.bracketed_paste);
                    }
                }
                _ => {}
            }
            needs_redraw = true;
        }
        Event::WindowEvent {
            event: WindowEvent::Focused(focus),
            ..
        } => {
            focused = focus;
            if let Some(tab) = tabs.get(active_tab) {
                match &tab.kind {
                    TabKind::Shell(shell) => {
                        tab.terminal.report_focus(focus, &shell.writer);
                    }
                }
            }
        }
        Event::AboutToWait => {
            if needs_redraw {
                window.request_redraw();
            }
        }
        _ => {}
    })?;

    Ok(())
}

fn default_shell() -> CommandBuilder {
    #[cfg(windows)]
    {
        CommandBuilder::new("cmd.exe")
    }
    #[cfg(not(windows))]
    {
        let shell = env::var("TERM_FORCE_SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let mut cmd = CommandBuilder::new(shell);
        // Keep the shell clean for debugging: skip user rc/profile and force an empty BASH_ENV.
        cmd.arg("--noprofile");
        if let Some(rcfile) = ensure_term_rcfile() {
            cmd.arg("--rcfile");
            cmd.arg(rcfile);
        } else {
            cmd.arg("--norc");
            cmd.env("BASH_ENV", "/dev/null");
        }
        if env::var("TERM_KEEP_PROMPT").is_err() {
            // Self-contained prompt (time, cwd, git, last status) scoped to this app only.
            let prompt_command = "TERM_LAST=$?; TERM_PROMPT_GIT=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)";
            let ps1 = r#"\[\e[38;5;117m\]\t\[\e[0m\] \
\[\e[38;5;148m\]\w\[\e[0m\]\
\[\e[38;5;215m\]${TERM_PROMPT_GIT:+ [${TERM_PROMPT_GIT}]}\[\e[0m\] \
\[\e[38;5;246m\]✦ $TERM_LAST\[\e[0m\]\n\[$(if [ $TERM_LAST -eq 0 ]; then printf '\e[38;5;41m'; else printf '\e[38;5;196m'; fi)\]\$\[\e[0m\] "#;
            cmd.env("PROMPT_COMMAND", prompt_command);
            cmd.env("PS1", ps1);
        }
        cmd
    }
}

fn open_link(link: &str) {
    #[cfg(target_os = "macos")]
    let candidates = ["open"];
    #[cfg(target_os = "linux")]
    let candidates = ["xdg-open"];
    #[cfg(target_os = "windows")]
    let candidates = ["start"];

    for bin in candidates {
        if Command::new(bin)
            .arg(link)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
        {
            break;
        }
    }
}

fn line_input_prefix(term: &Terminal) -> (String, usize) {
    let prefix = term.current_line_prefix();
    let total_cols = prefix.chars().count();

    const PROMPT_MARKERS: &[&str] = &["$ ", "# ", "% ", "> ", "❯ ", "➜ ", "» ", "› ", "λ "];

    let mut prompt_cols = 0;
    for marker in PROMPT_MARKERS {
        if let Some(idx) = prefix.rfind(marker) {
            let end = idx + marker.len();
            prompt_cols = prefix[..end].chars().count();
            break;
        }
    }

    if prompt_cols == 0 {
        if let Some((idx, ch)) = prefix
            .char_indices()
            .rev()
            .find(|&(_, c)| matches!(c, '$' | '#' | '%' | '>' | '❯' | '➜' | '»' | '›' | 'λ'))
        {
            let mut end_idx = idx + ch.len_utf8();
            while let Some(c) = prefix[end_idx..].chars().next() {
                if c != ' ' {
                    break;
                }
                end_idx += c.len_utf8();
            }
            prompt_cols = prefix[..end_idx].chars().count();
        }
    }

    if prompt_cols == 0 {
        let input_len = prefix.trim_start().chars().count();
        prompt_cols = total_cols.saturating_sub(input_len);
    }

    let input: String = prefix.chars().skip(prompt_cols).collect();
    (input.trim_start().to_string(), prompt_cols)
}

fn smart_suggestion_shortcut(event: &KeyEvent, modifiers: ModifiersState) -> bool {
    let ctrl_space = modifiers.control_key()
        && !modifiers.alt_key()
        && !modifiers.super_key()
        && (matches!(&event.logical_key, Key::Named(NamedKey::Space))
            || matches!(&event.logical_key, Key::Character(text) if text == " "));

    let bare_backquote = !modifiers.control_key()
        && !modifiers.alt_key()
        && !modifiers.super_key()
        && matches!(
            &event.logical_key,
            Key::Character(text) if text == "`" || text == "ˇ"
        );

    ctrl_space || bare_backquote
}

#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
fn handle_key(
    event: KeyEvent,
    modifiers: ModifiersState,
    tabs: &mut Vec<Tab>,
    active_tab: &mut usize,
    layout: &LayoutMetrics,
    history_menu: &mut HistoryMenu,
    autocomplete: &mut AutocompleteEngine,
    proxy: &EventLoopProxy<AppEvent>,
    help: &mut HelpToggle,
    settings: &SettingsPanel,
    cwd_history: &[CwdHistoryEntry],
) {
    if event.state != ElementState::Pressed {
        return;
    }

    if modifiers.control_key() && matches!(event.logical_key, Key::Named(NamedKey::Space)) {
        if let Some(tab) = tabs.get(*active_tab) {
            let (prefix, _) = line_input_prefix(&tab.terminal);
            let columns = smart_history_columns(
                history_menu,
                &prefix,
                *active_tab,
                tabs,
                cwd_history,
                autocomplete,
            );
            history_menu.open(columns);
        }
        return;
    }

    if matches!(event.logical_key, Key::Named(NamedKey::F1)) {
        help.toggle(Instant::now());
        return;
    }

    if modifiers.super_key()
        && let Key::Character(text) = &event.logical_key
    {
        let lower = text.to_lowercase();
        if lower == "t" {
            let cwd = preferred_cwd(tabs, cwd_history, *active_tab);
            match spawn_shell_tab(
                layout.cols as u16,
                layout.rows as u16,
                layout.usable_width,
                layout.usable_height,
                proxy.clone(),
                cwd,
                settings.scrollback_enabled,
            ) {
                Ok(tab) => {
                    tabs.push(tab);
                    refresh_tab_titles(tabs);
                    *active_tab = tabs.len().saturating_sub(1);
                }
                Err(err) => error!("failed to create tab: {err}"),
            }
            return;
        }
    }

    if modifiers.alt_key()
        && let Key::Character(text) = &event.logical_key
    {
        let lower = text.to_lowercase();
        if lower == "t" {
            match spawn_shell_tab(
                layout.cols as u16,
                layout.rows as u16,
                layout.usable_width,
                layout.usable_height,
                proxy.clone(),
                None,
                settings.scrollback_enabled,
            ) {
                Ok(tab) => {
                    tabs.push(tab);
                    refresh_tab_titles(tabs);
                    *active_tab = tabs.len().saturating_sub(1);
                }
                Err(err) => error!("failed to create tab: {err}"),
            }
            return;
        }
        if lower == "q" {
            if tabs.len() > 1 {
                tabs.remove(*active_tab);
                if *active_tab >= tabs.len() {
                    *active_tab = tabs.len().saturating_sub(1);
                }
                refresh_tab_titles(tabs);
            }
            return;
        }
        if let Some(digit) = lower.chars().next()
            && ('1'..='9').contains(&digit)
        {
            let idx = (digit as u8 - b'1') as usize;
            if idx < tabs.len() && idx < 5 {
                *active_tab = idx;
            }
            return;
        }
    }

    match &event.logical_key {
        Key::Named(NamedKey::Tab) if modifiers.control_key() => {
            if tabs.is_empty() {
                return;
            }
            if modifiers.shift_key() {
                *active_tab = active_tab.saturating_sub(1);
                if *active_tab >= tabs.len() {
                    *active_tab = tabs.len().saturating_sub(1);
                }
            } else {
                *active_tab = (*active_tab + 1) % tabs.len();
            }
            return;
        }
        Key::Character(text) => {
            if modifiers.control_key() && modifiers.shift_key() {
                let lower = text.to_lowercase();
                if lower == "t" {
                    match spawn_shell_tab(
                        layout.cols as u16,
                        layout.rows as u16,
                        layout.usable_width,
                        layout.usable_height,
                        proxy.clone(),
                        None,
                        settings.scrollback_enabled,
                    ) {
                        Ok(tab) => {
                            tabs.push(tab);
                            refresh_tab_titles(tabs);
                            *active_tab = tabs.len().saturating_sub(1);
                        }
                        Err(err) => error!("failed to create tab: {err}"),
                    }
                    return;
                } else if lower == "w" {
                    if tabs.len() > 1 {
                        tabs.remove(*active_tab);
                        if *active_tab >= tabs.len() {
                            *active_tab = tabs.len().saturating_sub(1);
                        }
                        refresh_tab_titles(tabs);
                    }
                    return;
                }
            }
        }
        _ => {}
    }

    if tabs.is_empty() {
        return;
    }

    // No tabs? Nothing to do.
    if tabs.get(*active_tab).is_none() {
        return;
    }

    if smart_suggestion_shortcut(&event, modifiers) {
        if let Some(tab) = tabs.get(*active_tab) {
            let (prefix, _) = line_input_prefix(&tab.terminal);
            let columns = smart_history_columns(
                history_menu,
                &prefix,
                *active_tab,
                tabs,
                cwd_history,
                autocomplete,
            );
            history_menu.open(columns);
        }
        return;
    }

    let copy_request = match &event.logical_key {
        Key::Named(NamedKey::Copy) => true,
        Key::Named(NamedKey::Insert) if modifiers.control_key() => true,
        Key::Character(text)
            if (modifiers.control_key() || modifiers.super_key())
                && modifiers.shift_key()
                && text.eq_ignore_ascii_case("c")
                && !modifiers.alt_key() =>
        {
            true
        }
        _ => false,
    };

    if copy_request {
        if let Some(tab) = tabs.get_mut(*active_tab)
            && let Some((a, b)) = selection_bounds(tab)
        {
            copy_selection_to_clipboard(&tab.terminal, a, b);
        }
        return;
    }

    let paste_request = match &event.logical_key {
        Key::Named(NamedKey::Paste) => true,
        Key::Named(NamedKey::Insert) if modifiers.shift_key() => true,
        Key::Character(text)
            if (modifiers.control_key() || modifiers.super_key())
                && text.eq_ignore_ascii_case("v")
                && !modifiers.alt_key() =>
        {
            true
        }
        _ => false,
    };

    if paste_request {
        if let Some(tab) = tabs.get(*active_tab) {
            let TabKind::Shell(shell) = &tab.kind;
            paste_clipboard(shell.writer.clone(), tab.terminal.bracketed_paste);
        }
        return;
    }

    if let Some(tab) = tabs.get_mut(*active_tab) {
        let TabKind::Shell(shell) = &mut tab.kind;

        input::send_key(
            event,
            modifiers,
            &shell.writer,
            shell.app_cursor_keys,
            tab.terminal.kitty_keyboard,
        );
    }
}

#[allow(clippy::ptr_arg)]
fn handle_history_key(
    event: KeyEvent,
    history_menu: &mut HistoryMenu,
    tabs: &mut Vec<Tab>,
    active_tab: usize,
    cwd_history: &[CwdHistoryEntry],
    autocomplete: &mut AutocompleteEngine,
) -> Option<String> {
    if event.state != ElementState::Pressed {
        return None;
    }
    match &event.logical_key {
        Key::Named(NamedKey::Escape) => history_menu.close(),
        Key::Named(NamedKey::ArrowUp) => history_menu.move_dir(0, -1),
        Key::Named(NamedKey::ArrowDown) => history_menu.move_dir(0, 1),
        Key::Named(NamedKey::ArrowLeft) => history_menu.move_dir(-1, 0),
        Key::Named(NamedKey::ArrowRight) => history_menu.move_dir(1, 0),
        Key::Named(NamedKey::Tab) => history_menu.move_dir(1, 0),
        Key::Named(NamedKey::PageUp) => history_menu.move_selection(-5),
        Key::Named(NamedKey::PageDown) => history_menu.move_selection(5),
        Key::Named(NamedKey::Enter) => {
            return apply_history_selection(history_menu, tabs, active_tab, true, cwd_history);
        }
        Key::Named(NamedKey::F8) => {
            if let Some((col, row)) = history_menu.selected_cell() {
                if let Some(col_data) = history_menu.columns.get(col) {
                    if let Some(entry) = col_data.entries.get(row).cloned() {
                        let command_text = entry.command.clone();
                        if col_data.title == "Bookmarks" {
                            history_menu
                                .bookmarks
                                .retain(|b| b.command != entry.command);
                        } else if !history_menu
                            .bookmarks
                            .iter()
                            .any(|b| b.command == entry.command)
                        {
                            history_menu.bookmarks.push(entry);
                        }
                        if let Some(tab) = tabs.get(active_tab) {
                            let (prefix, _) = line_input_prefix(&tab.terminal);
                            let cols = smart_history_columns(
                                history_menu,
                                &prefix,
                                active_tab,
                                tabs,
                                cwd_history,
                                autocomplete,
                            );
                            history_menu.open(cols);
                        }
                        if command_text.starts_with("ssh ") || command_text.starts_with("sudo ssh")
                        {
                            let _ = add_ssh_host_to_config(&command_text);
                        }
                    }
                }
            }
        }
        _ => {}
    }

    None
}

fn apply_history_selection(
    history_menu: &mut HistoryMenu,
    tabs: &mut Vec<Tab>,
    active_tab: usize,
    run: bool,
    cwd_history: &[CwdHistoryEntry],
) -> Option<String> {
    let entry = history_menu.selected_entry().cloned()?;
    let text = fix_command_paths(&entry.command, active_tab, tabs, cwd_history);
    if let Some(tab) = tabs.get_mut(active_tab) {
        let TabKind::Shell(shell) = &tab.kind;
        write_bytes(&shell.writer, text.as_bytes());
        if run {
            write_bytes(&shell.writer, b"\n");
        }
        tab.pending_cursor_to_line_end = true;
    }
    history_menu.close();
    Some(entry.command)
}

fn handle_cheatsheet_key(event: KeyEvent, cheatsheet: &mut Cheatsheet) {
    if event.state != ElementState::Pressed {
        return;
    }
    match &event.logical_key {
        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::F2) => cheatsheet.close(),
        _ => {}
    }
}

fn handle_settings_key(
    event: KeyEvent,
    settings: &mut SettingsPanel,
    proxy: &EventLoopProxy<AppEvent>,
    tabs: &mut [Tab],
) {
    if event.state != ElementState::Pressed {
        return;
    }
    match &event.logical_key {
        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::F4) => settings.close(),
        Key::Character(text) => {
            let lower = text.to_ascii_lowercase();
            if lower == "d" && !settings.downloading {
                settings.downloading = true;
                settings.status = "Downloading Noto Color Emoji…".to_string();
                start_download(
                    "https://github.com/googlefonts/noto-emoji/raw/main/fonts/NotoColorEmoji.ttf",
                    "NotoColorEmoji.ttf",
                    proxy.clone(),
                );
            } else if lower == "n" && !settings.downloading {
                settings.downloading = true;
                settings.status = "Downloading Nerd Font Symbols…".to_string();
                start_download(
                    "https://github.com/ryanoasis/nerd-fonts/raw/master/patched-fonts/NerdFontsSymbolsOnly/Regular/SymbolsNerdFont-Regular.ttf",
                    "SymbolsNerdFont-Regular.ttf",
                    proxy.clone(),
                );
            } else if lower == "s" {
                settings.scrollback_enabled = !settings.scrollback_enabled;
                for tab in tabs.iter_mut() {
                    tab.terminal
                        .set_scrollback_enabled(settings.scrollback_enabled);
                }
                settings.status = if settings.scrollback_enabled {
                    "Scrollback enabled".to_string()
                } else {
                    "Scrollback disabled (no history retained)".to_string()
                };
            } else if lower == "r" {
                settings.refresh_system_fonts();
            } else if lower == "f" {
                if settings.system_fonts.is_empty() {
                    settings.refresh_system_fonts();
                }
                if let Some(font) = settings.cycle_font() {
                    settings.status = format!("Loading {}…", font.name);
                    start_load_font(font, proxy.clone());
                } else {
                    settings.status = "No system fonts found; press R to rescan".to_string();
                }
            } else if lower == "0" {
                settings.selected_font = None;
                settings.status = "Reset to embedded font".to_string();
                let _ = proxy.send_event(AppEvent::PrimaryFont(Arc::new(FONT_DATA.to_vec())));
            }
        }
        _ => {}
    }
}

fn start_download(url: &str, filename: &str, proxy: EventLoopProxy<AppEvent>) {
    let url = url.to_string();
    let filename = filename.to_string();
    thread::spawn(move || {
        let base = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".local")
            .join("share")
            .join("fonts")
            .join("term-emoji");
        let _ = fs::create_dir_all(&base);
        let target = base.join(&filename);
        let cmd = format!(
            "curl -L '{}' -o '{}' && fc-cache -f '{}'",
            url,
            target.display(),
            base.display()
        );
        let status = Command::new("sh").arg("-c").arg(cmd).status();
        let (ok, msg) = match status {
            Ok(s) if s.success() => (
                true,
                format!("Downloaded {} to {}", filename, target.display()),
            ),
            Ok(s) => (false, format!("Download failed (status {})", s)),
            Err(err) => (false, format!("Download failed: {err}")),
        };
        let _ = proxy.send_event(AppEvent::DownloadStatus(msg));
        if ok {
            if let Ok(bytes) = fs::read(&target) {
                let _ = proxy.send_event(AppEvent::Fonts(vec![Arc::new(bytes)]));
            }
        }
    });
}

fn start_load_font(font: SystemFont, proxy: EventLoopProxy<AppEvent>) {
    thread::spawn(move || {
        let result = if font.path.as_os_str().is_empty() {
            Ok(FONT_DATA.to_vec())
        } else {
            fs::read(&font.path)
        };
        match result {
            Ok(bytes) => {
                let _ = proxy.send_event(AppEvent::PrimaryFont(Arc::new(bytes)));
                let _ = proxy.send_event(AppEvent::DownloadStatus(format!(
                    "Loaded font {}",
                    font.name
                )));
            }
            Err(err) => {
                let _ = proxy.send_event(AppEvent::DownloadStatus(format!(
                    "Font load failed ({}): {err}",
                    font.name
                )));
            }
        }
    });
}

fn pos_to_cell(
    x: f64,
    y: f64,
    layout: &LayoutMetrics,
    cell_w: u32,
    cell_h: u32,
    active_idx: usize,
    tabs: &[Tab],
) -> Option<(usize, usize)> {
    if active_idx >= tabs.len() {
        return None;
    }
    if x < layout.content_x as f64
        || y < layout.content_y as f64
        || x >= (layout.content_x + layout.usable_width) as f64
        || y >= (layout.content_y + layout.usable_height) as f64
    {
        return None;
    }
    let rel_x = x - layout.content_x as f64;
    let rel_y = y - layout.content_y as f64;
    let col = (rel_x / cell_w.max(1) as f64).floor() as usize;
    let row = (rel_y / cell_h.max(1) as f64).floor() as usize;
    if row < tabs[active_idx].terminal.rows && col < tabs[active_idx].terminal.cols {
        Some((col, row))
    } else {
        None
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
            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                let _ = pixels.resize_surface(frame_width, frame_height);
                if let Some(r) = renderer {
                    r.resize(frame_width, frame_height, pixels.queue());
                    r.clear_atlas();
                }
                true
            }
            wgpu::SurfaceError::Timeout => {
                warn!("render timeout; retrying next frame");
                true
            }
            wgpu::SurfaceError::OutOfMemory => {
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

#[cfg(test)]
mod layout_tests {
    use super::*;
    use crate::tab::ShellTab;
    #[cfg(unix)]
    use libc;
    #[cfg(unix)]
    use portable_pty::unix;
    use portable_pty::{MasterPty, PtySize};
    use std::io::{Write, empty, sink};
    use std::sync::mpsc;
    use vte::Parser as VteParser;

    #[derive(Default, Debug)]
    struct DummyMasterState {
        sizes: Vec<PtySize>,
    }

    #[derive(Clone)]
    struct SharedMaster(Arc<Mutex<DummyMasterState>>);

    impl MasterPty for SharedMaster {
        fn resize(&self, size: PtySize) -> Result<(), anyhow::Error> {
            self.0.lock().unwrap().sizes.push(size);
            Ok(())
        }

        fn get_size(&self) -> Result<PtySize, anyhow::Error> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .sizes
                .last()
                .copied()
                .unwrap_or_default())
        }

        fn try_clone_reader(&self) -> Result<Box<dyn std::io::Read + Send>, anyhow::Error> {
            Ok(Box::new(empty()))
        }

        fn take_writer(&self) -> Result<Box<dyn std::io::Write + Send>, anyhow::Error> {
            Ok(Box::new(sink()))
        }

        #[cfg(unix)]
        fn process_group_leader(&self) -> Option<libc::pid_t> {
            None
        }

        #[cfg(unix)]
        fn as_raw_fd(&self) -> Option<unix::RawFd> {
            None
        }
    }

    fn make_shell_tab(cols: usize, rows: usize) -> Tab {
        let master: Box<dyn MasterPty + Send> = Box::new(SharedMaster(Arc::new(Mutex::new(
            DummyMasterState::default(),
        ))));
        let dummy_writer: Box<dyn Write + Send> = Box::new(sink());
        let (_, rx) = mpsc::channel();

        Tab {
            terminal: Terminal::new(cols, rows),
            kind: TabKind::Shell(ShellTab {
                parser: VteParser::new(),
                rx,
                writer: Arc::new(Mutex::new(dummy_writer)),
                master: Arc::new(Mutex::new(master)),
                app_cursor_keys: false,
                exited: false,
                child: None,
            }),
            title: String::new(),
            selection_anchor: None,
            selection_edge: None,
            selecting: false,
            link_hover: None,
            pending_cursor_to_line_end: false,
            last_cwd: None,
        }
    }

    #[test]
    fn layout_respects_padding_and_border() {
        let cell_w = 9;
        let cell_h = 18;
        let tab_bar_height = 24;
        let layout = compute_layout(
            200,
            120,
            cell_w,
            cell_h,
            tab_bar_height,
            BORDER_THICKNESS,
            BORDER_INSET,
            PADDING_X,
            PADDING_Y,
        );

        let expected_left = BORDER_THICKNESS + PADDING_X;
        let expected_top = BORDER_THICKNESS + tab_bar_height + PADDING_Y;
        assert_eq!(layout.content_x, expected_left);
        assert_eq!(layout.content_y, expected_top);

        assert_eq!(
            layout.cols,
            ((layout.usable_width / cell_w.max(1)) as usize).max(1)
        );
        assert_eq!(
            layout.rows,
            ((layout.usable_height / cell_h.max(1)) as usize).max(1)
        );
    }

    #[test]
    fn layout_never_drops_below_one_cell() {
        let layout = compute_layout(
            0,
            0,
            0,
            0,
            0,
            BORDER_THICKNESS,
            BORDER_INSET,
            PADDING_X,
            PADDING_Y,
        );
        assert_eq!(layout.cols, 1);
        assert_eq!(layout.rows, 1);
    }

    #[test]
    fn resize_tabs_updates_terminal_and_master() {
        let state = Arc::new(Mutex::new(DummyMasterState::default()));
        let master: Box<dyn MasterPty + Send> = Box::new(SharedMaster(state.clone()));

        let dummy_writer: Box<dyn Write + Send> = Box::new(sink());
        let (_, rx) = mpsc::channel();

        let mut tab = Tab {
            terminal: Terminal::new(2, 2),
            kind: TabKind::Shell(ShellTab {
                parser: VteParser::new(),
                rx,
                writer: Arc::new(Mutex::new(dummy_writer)),
                master: Arc::new(Mutex::new(master)),
                app_cursor_keys: false,
                exited: false,
                child: None,
            }),
            title: String::new(),
            selection_anchor: None,
            selection_edge: None,
            selecting: false,
            link_hover: None,
            pending_cursor_to_line_end: false,
            last_cwd: None,
        };

        let layout = LayoutMetrics {
            content_x: 0,
            content_y: 0,
            usable_width: 640,
            usable_height: 360,
            cols: 80,
            rows: 40,
        };

        resize_tabs_to_layout(std::slice::from_mut(&mut tab), &layout);
        assert_eq!(tab.terminal.cols, 80);
        assert_eq!(tab.terminal.rows, 40);

        let sizes = &state.lock().unwrap().sizes;
        assert_eq!(sizes.len(), 1);
        assert_eq!(sizes[0].rows, 40);
        assert_eq!(sizes[0].cols, 80);
        assert_eq!(sizes[0].pixel_width, 640u32.min(u16::MAX as u32) as u16);
        assert_eq!(sizes[0].pixel_height, 360u32.min(u16::MAX as u32) as u16);
    }

    #[test]
    fn bracketed_paste_wraps_payload() {
        let plain = compose_bracketed_paste(b"hello", false);
        assert_eq!(plain, b"hello");

        let bracketed = compose_bracketed_paste(b"hello", true);
        assert_eq!(bracketed, b"\x1b[200~hello\x1b[201~");
    }

    #[test]
    fn pos_to_cell_bounds_check() {
        let layout = LayoutMetrics {
            content_x: 10,
            content_y: 20,
            usable_width: 40,
            usable_height: 40,
            cols: 2,
            rows: 2,
        };
        let tabs = vec![make_shell_tab(2, 2)];
        let inside = pos_to_cell(12.0, 22.0, &layout, 20, 20, 0, &tabs);
        assert_eq!(inside, Some((0, 0)));

        let outside = pos_to_cell(0.0, 0.0, &layout, 20, 20, 0, &tabs);
        assert!(outside.is_none());
    }

    #[test]
    fn selection_bounds_requires_anchor_and_edge() {
        let mut tab = make_shell_tab(2, 1);
        assert!(selection_bounds(&tab).is_none());
        tab.selection_anchor = Some((0, 0));
        tab.selection_edge = Some((1, 0));
        assert_eq!(selection_bounds(&tab), Some(((0, 0), (1, 0))));
    }

    #[test]
    fn run_command_with_timeout_captures_stdout() {
        // printf exists on POSIX; if absent the test will simply fail.
        let out = run_command_with_timeout("printf", &["abc"], None, Duration::from_millis(500))
            .expect("stdout captured");
        assert_eq!(out, b"abc");
    }

    #[test]
    fn run_command_with_timeout_allows_empty_stdout() {
        #[cfg(windows)]
        let cmd = ("cmd.exe", &["/C", "exit", "0"][..]);
        #[cfg(not(windows))]
        let cmd = ("true", &[][..]);

        let out = run_command_with_timeout(cmd.0, cmd.1, None, Duration::from_millis(500))
            .expect("command success still returns buffer");
        assert!(out.is_empty());
    }
}

fn apply_window_resize(
    new_size: PhysicalSize<u32>,
    cell_w: u32,
    cell_h: u32,
    tab_bar_height: u32,
    pixels: &mut Pixels,
    tabs: &mut [Tab],
) -> (LayoutMetrics, u32, u32) {
    let frame_width = new_size.width;
    let frame_height = new_size.height;
    let layout = compute_layout(
        frame_width,
        frame_height,
        cell_w,
        cell_h,
        tab_bar_height,
        BORDER_THICKNESS,
        BORDER_INSET,
        PADDING_X,
        PADDING_Y,
    );

    pixels.resize_buffer(frame_width, frame_height).ok();
    pixels.resize_surface(frame_width, frame_height).ok();
    resize_tabs_to_layout(tabs, &layout);

    (layout, frame_width, frame_height)
}

fn path_menu_suggestions(
    active: usize,
    tabs: &[Tab],
    history: &[CwdHistoryEntry],
    limit: usize,
) -> Vec<String> {
    recent_dirs_for_tab(active, tabs, history)
        .into_iter()
        .take(limit)
        .map(|p| format!("cd {}", p.display()))
        .collect()
}

fn push_menu_entry(
    dest: &mut Vec<MenuEntry>,
    used: &mut HashSet<String>,
    label: impl Into<String>,
    command: impl Into<String>,
) {
    let command = command.into().trim().to_string();
    if command.is_empty() {
        return;
    }
    if used.insert(command.clone()) {
        let label = label.into();
        let label = if label.trim().is_empty() {
            command.clone()
        } else {
            label
        };
        dest.push(MenuEntry { label, command });
    }
}

fn push_menu_entry_local(
    dest: &mut Vec<MenuEntry>,
    seen: &mut HashSet<String>,
    label: impl Into<String>,
    command: impl Into<String>,
) {
    let command = command.into().trim().to_string();
    if command.is_empty() || !seen.insert(command.clone()) {
        return;
    }
    let label = label.into();
    let label = if label.trim().is_empty() {
        command.clone()
    } else {
        label
    };
    dest.push(MenuEntry { label, command });
}

fn command_executable(cmd: &str) -> bool {
    let mut parts = cmd.split_whitespace();
    let mut first = match parts.next() {
        Some(p) => p,
        None => return false,
    };
    if first == "sudo" {
        first = match parts.next() {
            Some(p) => p,
            None => return false,
        };
    }
    if first.starts_with('#') {
        return false;
    }
    let builtins = [
        "cd", "exit", "pwd", "true", "false", "alias", "fg", "bg", "history", "set", "unset",
    ];
    if builtins.contains(&first) {
        return true;
    }
    if first.contains('/') {
        let path = Path::new(first);
        if !path.is_file() {
            return false;
        }
        #[cfg(unix)]
        {
            return path
                .metadata()
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false);
        }
        #[cfg(not(unix))]
        {
            return true;
        }
    }
    let path_var = env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin:/usr/local/bin".into());
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(first);
        if candidate.is_file() {
            #[cfg(unix)]
            {
                if candidate
                    .metadata()
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false)
                {
                    return true;
                }
            }
            #[cfg(not(unix))]
            {
                return true;
            }
        }
    }
    false
}

fn extract_ssh_target(cmd: &str) -> Option<String> {
    let mut parts = cmd.split_whitespace();
    let mut first = parts.next()?;
    if first == "sudo" {
        first = parts.next()?;
    }
    if first != "ssh" {
        return None;
    }
    for part in parts {
        if part.starts_with('-') {
            continue;
        }
        return Some(part.to_string());
    }
    None
}

fn ssh_config_hosts() -> Vec<String> {
    let mut hosts = Vec::new();
    let mut seen = HashSet::new();
    if let Some(home) = dirs::home_dir() {
        let path = home.join(".ssh").join("config");
        if let Ok(contents) = fs::read_to_string(path) {
            for line in contents.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') || trimmed.is_empty() {
                    continue;
                }
                if let Some(rest) = trimmed.strip_prefix("Host ") {
                    for host in rest.split_whitespace() {
                        if host.contains('*') || host.contains('?') {
                            continue;
                        }
                        if seen.insert(host.to_string()) {
                            hosts.push(host.to_string());
                        }
                    }
                }
            }
        }
    }
    hosts
}

fn add_ssh_host_to_config(command: &str) -> std::io::Result<()> {
    let Some(target) = extract_ssh_target(command) else {
        return Ok(());
    };
    let Some(home) = dirs::home_dir() else {
        return Ok(());
    };
    let config_path = home.join(".ssh").join("config");
    let mut existing = String::new();
    if let Ok(contents) = fs::read_to_string(&config_path) {
        existing = contents;
        if existing.contains(&format!("Host {}", target)) {
            return Ok(());
        }
    }
    let block = format!("\nHost {}\n  HostName {}\n", target, target);
    existing.push_str(&block);
    if let Some(parent) = config_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(config_path, existing)
}

fn ensure_term_rcfile() -> Option<PathBuf> {
    let base = dirs::cache_dir()?.join("term");
    let _ = fs::create_dir_all(&base);
    let rc = base.join("rc.sh");
    let contents = r#"
# term minimal rc (does not touch user shell configs)
if command -v eza >/dev/null 2>&1; then
  alias ls='eza --icons --group-directories-first -lh'
elif command -v exa >/dev/null 2>&1; then
  alias ls='exa --icons --group-directories-first -lh'
else
alias ls='ls --color=auto -lh --group-directories-first --classify'
fi
# keep grep colored for convenience
alias grep='grep --color=auto'
# readline tweaks to make completion feel closer to “just works”
bind 'set colored-completion-prefix on'
bind 'set show-all-if-ambiguous on'
bind 'set completion-ignore-case on'
bind 'set menu-complete-display-prefix on'
bind 'set completion-query-items 0'
# no other user env is modified
"#;
    if fs::write(&rc, contents).is_ok() {
        Some(rc)
    } else {
        None
    }
}

fn scan_path_programs(prefix: &str, limit: usize) -> Vec<String> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    let path_var = env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin:/usr/local/bin".into());
    let wants = prefix.trim();
    let matcher = |name: &str| {
        if wants.is_empty() {
            return true;
        }
        name.starts_with(wants)
    };
    for dir in env::split_paths(&path_var) {
        if results.len() >= limit {
            break;
        }
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if results.len() >= limit {
                    break;
                }
                let name = match entry.file_name().into_string() {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                if !matcher(&name) {
                    continue;
                }
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                #[cfg(unix)]
                {
                    if entry
                        .metadata()
                        .map(|m| m.permissions().mode() & 0o111 == 0)
                        .unwrap_or(true)
                    {
                        continue;
                    }
                }
                if seen.insert(name.clone()) {
                    results.push(name);
                }
            }
        }
    }
    results
}

fn smart_history_columns(
    menu: &HistoryMenu,
    prefix: &str,
    active: usize,
    tabs: &[Tab],
    history: &[CwdHistoryEntry],
    autocomplete: &mut AutocompleteEngine,
) -> Vec<MenuColumn> {
    let mut used_commands = HashSet::new();

    // Column 1: paths to navigate (from history and path completions).
    let mut paths = Vec::new();
    let current_dir_buf = tabs.get(active).and_then(tab_current_dir);
    let current_dir = current_dir_buf.as_deref();
    for cmd in path_menu_suggestions(active, tabs, history, 32) {
        push_menu_entry(&mut paths, &mut used_commands, cmd.clone(), cmd);
    }
    for cmd in path_completion_suggestions(prefix, current_dir, 32) {
        push_menu_entry(&mut paths, &mut used_commands, cmd.clone(), cmd);
    }
    push_menu_entry(&mut paths, &mut used_commands, "pwd", "pwd");
    push_menu_entry(&mut paths, &mut used_commands, "ls -lah", "ls -lah");

    // Column 2: commands from history/auto-complete.
    let mut commands = Vec::new();
    for cmd in autocomplete.suggest(prefix, 64) {
        if command_executable(&cmd) {
            push_menu_entry(&mut commands, &mut used_commands, &cmd, &cmd);
        }
    }
    // include path completions inline when user is typing a path-like token
    if looks_like_path(prefix.split_whitespace().last().unwrap_or(prefix).trim()) {
        for cmd in path_completion_suggestions(prefix, current_dir, 24) {
            push_menu_entry(&mut commands, &mut used_commands, &cmd, &cmd);
        }
    }

    // Column 3: bookmarks (persistent per session).
    let mut bookmarks = Vec::new();
    let mut bookmark_seen = HashSet::new();
    for entry in &menu.bookmarks {
        push_menu_entry_local(
            &mut bookmarks,
            &mut bookmark_seen,
            &entry.label,
            &entry.command,
        );
    }

    // Column 4: SSH targets (from config and history).
    let mut ssh_entries = Vec::new();
    let mut ssh_seen = HashSet::new();
    if command_executable("ssh") {
        for host in ssh_config_hosts() {
            let cmd = format!("ssh {}", host);
            push_menu_entry(
                &mut ssh_entries,
                &mut used_commands,
                format!("ssh {}", host),
                cmd,
            );
        }
        for cmd in autocomplete.suggest(prefix, 96) {
            if let Some(target) = extract_ssh_target(&cmd) {
                let canon = format!("ssh {}", target);
                if ssh_seen.insert(canon.clone()) {
                    push_menu_entry(&mut ssh_entries, &mut used_commands, canon.clone(), canon);
                }
            }
        }
    }

    vec![
        MenuColumn {
            title: "Paths",
            accent: Rgba {
                r: 140,
                g: 210,
                b: 255,
                a: 255,
            },
            entries: paths,
        },
        MenuColumn {
            title: "Commands",
            accent: Rgba {
                r: 255,
                g: 210,
                b: 150,
                a: 255,
            },
            entries: commands,
        },
        MenuColumn {
            title: "Bookmarks",
            accent: Rgba {
                r: 205,
                g: 150,
                b: 255,
                a: 255,
            },
            entries: bookmarks,
        },
        MenuColumn {
            title: "SSH",
            accent: Rgba {
                r: 140,
                g: 255,
                b: 200,
                a: 255,
            },
            entries: ssh_entries,
        },
    ]
}

fn paste_clipboard(writer: Arc<Mutex<Box<dyn Write + Send>>>, bracketed: bool) {
    thread::spawn(move || paste_clipboard_sync(&writer, bracketed));
}

fn compose_bracketed_paste(data: &[u8], bracketed: bool) -> Vec<u8> {
    if bracketed {
        let mut payload = Vec::with_capacity(data.len() + 8);
        payload.extend_from_slice(b"\x1b[200~");
        payload.extend_from_slice(data);
        payload.extend_from_slice(b"\x1b[201~");
        payload
    } else {
        data.to_vec()
    }
}

fn paste_clipboard_sync(writer: &Arc<Mutex<Box<dyn Write + Send>>>, bracketed: bool) {
    let send_text = |data: &[u8]| {
        let payload = compose_bracketed_paste(data, bracketed);
        write_bytes(writer, &payload);
    };

    let wl_paths = ["/usr/bin/wl-paste", "/bin/wl-paste", "wl-paste"];
    let xclip_paths = ["/usr/bin/xclip", "/bin/xclip", "xclip"];
    let xsel_paths = ["/usr/bin/xsel", "/bin/xsel", "xsel"];
    let pbpaste_paths = ["/usr/bin/pbpaste", "/bin/pbpaste", "pbpaste"];
    let powershell_paths = [
        "/usr/bin/powershell",
        "/bin/powershell",
        "powershell",
        "powershell.exe",
    ];

    // Try Wayland/X11 tools first so cliphist/portals stay in sync.
    let mut candidates: Vec<(&str, &[&str])> = wl_paths
        .iter()
        .map(|p| {
            (
                *p,
                &[
                    "--no-newline",
                    "--type",
                    "text/plain",
                    "--selection",
                    "clipboard",
                ][..],
            )
        })
        .chain(wl_paths.iter().map(|p| {
            (
                *p,
                &[
                    "--no-newline",
                    "--type",
                    "text/plain",
                    "--selection",
                    "primary",
                ][..],
            )
        }))
        .collect();
    candidates.extend(
        xclip_paths
            .iter()
            .map(|p| (*p, &["-o", "-selection", "clipboard"][..])),
    );
    candidates.extend(
        xclip_paths
            .iter()
            .map(|p| (*p, &["-o", "-selection", "primary"][..])),
    );
    candidates.extend(xsel_paths.iter().map(|p| (*p, &["-o", "-b"][..])));
    candidates.extend(xsel_paths.iter().map(|p| (*p, &["-o", "-p"][..])));
    candidates.extend(pbpaste_paths.iter().map(|p| (*p, &[][..])));
    candidates.extend(
        powershell_paths
            .iter()
            .map(|p| (*p, &["-NoProfile", "-Command", "Get-Clipboard"][..])),
    );

    for (bin, args) in candidates {
        let exe = Path::new(bin);
        if !exe.exists() && bin.contains('/') {
            continue;
        }
        if let Some(out) = run_command_with_timeout(bin, args, None, Duration::from_millis(500)) {
            if out.is_empty() {
                continue;
            }
            send_text(&out);
            return;
        }
    }

    // Fallback to arboard if shell tools fail or are unavailable.
    if let Ok(mut clipboard) = Clipboard::new()
        && let Ok(text) = clipboard.get_text()
        && !text.is_empty()
    {
        send_text(text.as_bytes());
        return;
    }

    warn!("paste failed: no clipboard provider returned data");
}

fn load_shell_history(limit: usize) -> Vec<String> {
    let mut entries: Vec<String> = Vec::new();
    if let Some(home) = env::var_os("HOME") {
        let home = std::path::PathBuf::from(home);
        let files = [".zsh_history", ".bash_history"];
        'outer: for file in files {
            let path = home.join(file);
            if let Ok(contents) = fs::read_to_string(&path) {
                for line in contents.lines().rev() {
                    let trimmed = if line.starts_with(':') {
                        line.split_once(';')
                            .map(|x| x.1)
                            .unwrap_or("")
                            .trim()
                            .to_string()
                    } else {
                        line.trim().to_string()
                    };
                    if trimmed.is_empty() {
                        continue;
                    }
                    entries.push(trimmed);
                    if entries.len() >= limit * 2 {
                        break 'outer;
                    }
                }
            }
        }
    }

    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for cmd in entries {
        if seen.insert(cmd.clone()) {
            deduped.push(cmd);
            if deduped.len() >= limit {
                break;
            }
        }
    }
    deduped
}

fn read_clipboard_suggestions(limit: usize) -> Vec<String> {
    if let Ok(mut clipboard) = Clipboard::new()
        && let Ok(text) = clipboard.get_text()
    {
        let mut lines = Vec::new();
        for line in text.lines().rev() {
            let trimmed = line.trim();
            if trimmed.len() < 2 {
                continue;
            }
            lines.push(trimmed.to_string());
            if lines.len() >= limit {
                break;
            }
        }
        return lines;
    }
    Vec::new()
}

fn autocomplete_store_path() -> Option<PathBuf> {
    if let Some(dir) = config_dir() {
        return Some(dir.join("term").join("autocomplete.json"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config").join("term").join("autocomplete.json"))
}

fn help_state_path() -> Option<PathBuf> {
    if let Some(dir) = config_dir() {
        return Some(dir.join("term").join("help.json"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config").join("term").join("help.json"))
}

fn load_help_visible_flag() -> Option<bool> {
    let path = help_state_path()?;
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice::<bool>(&bytes).ok()
}

fn persist_help_visible_flag(flag: bool) {
    if let Some(path) = help_state_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec(&flag) {
            let _ = fs::write(&path, bytes);
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct LearnedStore {
    counts: HashMap<String, u32>,
    path: Option<PathBuf>,
}

impl LearnedStore {
    fn load() -> Self {
        let path = autocomplete_store_path();
        if let Some(p) = path.clone()
            && let Ok(bytes) = fs::read(&p)
            && let Ok(counts) = serde_json::from_slice::<HashMap<String, u32>>(&bytes)
        {
            return Self { counts, path };
        }
        Self {
            counts: HashMap::new(),
            path,
        }
    }

    fn record(&mut self, entry: &str) {
        let clean = entry.trim();
        if clean.is_empty() {
            return;
        }
        let count = self.counts.entry(clean.to_string()).or_insert(0);
        *count = count.saturating_add(1);
        self.persist();
    }

    fn persist(&self) {
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(&self.counts) {
                let _ = fs::write(path, json);
            }
        }
    }
}

struct AutocompleteEngine {
    learned: LearnedStore,
}

impl AutocompleteEngine {
    fn load() -> Self {
        Self {
            learned: LearnedStore::load(),
        }
    }

    fn prefix_score(text: &str, prefix: &str) -> i32 {
        if prefix.is_empty() {
            return 0;
        }
        if text.starts_with(prefix) {
            5_000
        } else if text.contains(prefix) {
            2_000
        } else {
            0
        }
    }

    fn suggest(&mut self, prefix: &str, limit: usize) -> Vec<String> {
        let prefix = prefix.trim();
        let mut scores: HashMap<String, i32> = HashMap::new();
        let mut add = |text: String, base: i32, freq: u32| {
            let clean = text.trim();
            if clean.len() < 2 {
                return;
            }
            if !prefix.is_empty() && !clean.starts_with(prefix) && !clean.contains(prefix) {
                return;
            }
            let score = base + Self::prefix_score(clean, prefix) + (freq as i32 * 25);
            match scores.entry(clean.to_string()) {
                Entry::Vacant(v) => {
                    v.insert(score);
                }
                Entry::Occupied(mut o) => {
                    if score > *o.get() {
                        o.insert(score);
                    }
                }
            }
        };

        for (idx, cmd) in load_shell_history(1200).into_iter().enumerate() {
            let base = 2_000 - idx as i32;
            add(cmd, base, 1);
        }

        for (text, freq) in self.learned.counts.clone() {
            add(text, 4_000, freq);
        }

        for (idx, clip) in read_clipboard_suggestions(64).into_iter().enumerate() {
            let base = 800 - idx as i32;
            add(clip, base, 1);
        }

        for (idx, prog) in scan_path_programs(prefix, 256).into_iter().enumerate() {
            let base = 1200 - idx as i32;
            add(prog, base, 1);
        }

        let mut entries: Vec<(String, i32)> = scores.into_iter().collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.truncate(limit);
        entries.into_iter().map(|(t, _)| t).collect()
    }

    fn record_accept(&mut self, entry: &str) {
        self.learned.record(entry);
    }
}

fn copy_selection_to_clipboard(term: &Terminal, a: (usize, usize), b: (usize, usize)) {
    let text = selection_text(term, a, b);
    if text.is_empty() {
        return;
    }

    use std::path::Path;

    let wl_paths = ["/usr/bin/wl-copy", "/bin/wl-copy", "wl-copy"];
    let xclip_paths = ["/usr/bin/xclip", "/bin/xclip", "xclip"];
    let xsel_paths = ["/usr/bin/xsel", "/bin/xsel", "xsel"];
    let pbcopy_paths = ["/usr/bin/pbcopy", "/bin/pbcopy", "pbcopy"];

    // Prefer wl-copy/xclip so cliphist sees updates even if arboard succeeds.
    let mut attempts: Vec<(&str, &[&str])> = wl_paths
        .iter()
        .map(|p| (*p, &["--trim-newline"][..]))
        .chain(
            wl_paths
                .iter()
                .map(|p| (*p, &["--primary", "--trim-newline"][..])),
        )
        .collect();

    attempts.extend(
        xclip_paths
            .iter()
            .map(|p| (*p, &["-selection", "clipboard"][..])),
    );
    attempts.extend(
        xclip_paths
            .iter()
            .map(|p| (*p, &["-selection", "primary"][..])),
    );
    attempts.extend(xsel_paths.iter().map(|p| (*p, &["-b"][..])));
    attempts.extend(xsel_paths.iter().map(|p| (*p, &["-p"][..])));
    attempts.extend(pbcopy_paths.iter().map(|p| (*p, &[][..])));

    let mut copied = false;
    for (bin, args) in &attempts {
        let exe = Path::new(bin);
        if !exe.exists() && bin.contains('/') {
            continue;
        }
        if run_command_with_timeout(bin, args, Some(text.as_bytes()), Duration::from_millis(500))
            .is_some()
        {
            copied = true;
        }
    }

    // Also push to arboard for completeness.
    if let Ok(mut clipboard) = Clipboard::new() {
        let _ = clipboard.set_text(text.clone());
    }

    if !copied {
        warn!("clipboard write failed: no provider accepted data");
    }
}

fn run_command_with_timeout(
    bin: &str,
    args: &[&str],
    stdin_data: Option<&[u8]>,
    timeout: Duration,
) -> Option<Vec<u8>> {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(if stdin_data.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    if let Some(input) = stdin_data
        && let Some(stdin) = child.stdin.as_mut()
    {
        let _ = stdin.write_all(input);
    }

    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().ok()? {
            if status.success() {
                let mut buf = Vec::new();
                if let Some(mut stdout) = child.stdout.take() {
                    let _ = stdout.read_to_end(&mut buf);
                }
                return Some(buf);
            }
            return None;
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn spawn_font_loader(proxy: EventLoopProxy<AppEvent>) {
    thread::spawn(move || {
        let fonts = GlyphCache::load_fallback_fonts();
        if !fonts.is_empty() {
            let _ = proxy.send_event(AppEvent::Fonts(fonts));
        }
    });
}
