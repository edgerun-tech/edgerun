use std::collections::{HashMap, HashSet, hash_map::Entry};
use std::env;
use std::fs;
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::OnceLock;
use std::sync::{Arc, Mutex, mpsc::TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use dirs::config_dir;
use log::{debug, error, info, warn};
use pixels::{Pixels, SurfaceTexture};
use portable_pty::CommandBuilder;
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use term_core::gpu::GpuRenderer;
use term_core::render::layout::{LayoutMetrics, compute_layout};
use term_core::render::{FONT_DATA, FONT_SIZE, GlyphCache};
use term_core::terminal::Rgba;
use term_core::terminal::{
    GridPerformer, Terminal, copy_text_to_clipboard, selection_text, write_bytes,
};
use term_ui::app_render::{RenderInputs, TabRender, render_frame};
use term_ui::debug::{DebugOverlay, DebugRenderMode};
use term_ui::input;
use term_ui::suggest::{looks_like_path, path_completion_suggestions};
use term_ui::widgets::history::{MenuColumn, MenuEntry};
use term_ui::widgets::settings::SystemFont;
use term_ui::widgets::{
    Cheatsheet, ContextAction, ContextMenu, HistoryMenu, LogFocus, LogSourceEntry, LogViewer,
    SettingsPanel,
};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, Event, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::platform::startup_notify::{
    WindowAttributesExtStartupNotify, reset_activation_token_env,
};
use winit::window::Window;

mod logging;
mod suggest;
mod tab;

use crate::suggest::{fix_command_paths, recent_dirs_for_tab};
use crate::tab::{
    CwdHistoryEntry, Tab, TabKind, preferred_cwd, refresh_cwd_for_tabs, refresh_tab_titles,
    resize_tabs_to_layout, selection_bounds, spawn_shell_tab, tab_current_dir,
};

fn handle_settings_key(
    event: KeyEvent,
    settings: &mut SettingsPanel,
    debug_overlay: &mut DebugOverlay,
    proxy: &EventLoopProxy<AppEvent>,
    tabs: &mut [Tab],
    font_load_inflight: &mut bool,
    font_load_pending: &mut Option<SystemFont>,
) {
    if event.state != ElementState::Pressed {
        return;
    }
    let mut changed = false;

    // build same lines as the render code so we can navigate and act on selection
    let lines = vec![
        "F4/Esc closes the panel".to_string(),
        " ".to_string(),
        "Performance".to_string(),
        format!(
            "  Scrollback: {} (S to toggle)",
            if settings.scrollback_enabled {
                "Enabled"
            } else {
                "Disabled"
            }
        ),
        format!(
            "  FPS overlay: {} (P to toggle)",
            if settings.show_fps { "On" } else { "Off" }
        ),
        format!(
            "  Copy notice: {} (C to toggle)",
            if settings.show_copy_notice {
                "On"
            } else {
                "Off"
            }
        ),
        format!("  Rendering: {} (G to cycle)", settings.render_mode),
        format!(
            "  Log level: {} (L to cycle)",
            settings.log_level.to_ascii_uppercase()
        ),
        " ".to_string(),
        "Fonts".to_string(),
        settings.current_font_label(),
        "  F = next font, R = refresh system list, 0 = reset embedded".to_string(),
        " ".to_string(),
        "Downloads".to_string(),
        "  D = Noto Color Emoji, N = Nerd Font Symbols".to_string(),
        "  Saved to ~/.local/share/fonts/term-emoji/ (fc-cache runs)".to_string(),
        " ".to_string(),
        format!("Status: {}", settings.status),
    ];

    match &event.logical_key {
        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::F4) => settings.close(),
        Key::Named(NamedKey::ArrowUp) => {
            if settings.selected_index == 0 {
                settings.selected_index = lines.len().saturating_sub(1);
            } else {
                settings.selected_index -= 1;
            }
        }
        Key::Named(NamedKey::ArrowDown) => {
            settings.selected_index = (settings.selected_index + 1) % lines.len();
        }
        Key::Named(NamedKey::Space) | Key::Named(NamedKey::Enter) => {
            match settings.selected_index {
                3 => {
                    settings.scrollback_enabled = !settings.scrollback_enabled;
                    changed = true;
                    for tab in tabs.iter_mut() {
                        tab.terminal
                            .set_scrollback_enabled(settings.scrollback_enabled);
                    }
                    settings.status = if settings.scrollback_enabled {
                        "Scrollback enabled".to_string()
                    } else {
                        "Scrollback disabled (no history retained)".to_string()
                    };
                }
                4 => {
                    settings.show_fps = !settings.show_fps;
                    changed = true;
                    settings.status = if settings.show_fps {
                        "FPS overlay enabled".to_string()
                    } else {
                        "FPS overlay disabled".to_string()
                    };
                }
                5 => {
                    settings.show_copy_notice = !settings.show_copy_notice;
                    changed = true;
                    settings.status = if settings.show_copy_notice {
                        "Copy notice enabled".to_string()
                    } else {
                        "Copy notice disabled".to_string()
                    };
                }
                6 => {
                    settings.render_mode = next_render_mode(settings.render_mode);
                    debug_overlay.set_render_mode(settings.render_mode);
                    changed = true;
                    settings.status = format!("Rendering mode: {}", settings.render_mode);
                }
                7 => {
                    settings.log_level = next_log_level(&settings.log_level).to_string();
                    if let Some(level) = parse_log_level(&settings.log_level) {
                        logging::set_level(level);
                    }
                    changed = true;
                    settings.status = format!(
                        "Log level set to {}",
                        settings.log_level.to_ascii_uppercase()
                    );
                }
                _ => {}
            }
        }
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
                changed = true;
                for tab in tabs.iter_mut() {
                    tab.terminal
                        .set_scrollback_enabled(settings.scrollback_enabled);
                }
                settings.status = if settings.scrollback_enabled {
                    "Scrollback enabled".to_string()
                } else {
                    "Scrollback disabled (no history retained)".to_string()
                };
            } else if lower == "p" {
                settings.show_fps = !settings.show_fps;
                changed = true;
                settings.status = if settings.show_fps {
                    "FPS overlay enabled".to_string()
                } else {
                    "FPS overlay disabled".to_string()
                };
            } else if lower == "c" {
                settings.show_copy_notice = !settings.show_copy_notice;
                changed = true;
                settings.status = if settings.show_copy_notice {
                    "Copy notice enabled".to_string()
                } else {
                    "Copy notice disabled".to_string()
                };
            } else if lower == "g" {
                settings.render_mode = next_render_mode(settings.render_mode);
                debug_overlay.set_render_mode(settings.render_mode);
                changed = true;
                settings.status = format!("Rendering mode: {}", settings.render_mode);
            } else if lower == "l" {
                settings.log_level = next_log_level(&settings.log_level).to_string();
                if let Some(level) = parse_log_level(&settings.log_level) {
                    logging::set_level(level);
                }
                changed = true;
                settings.status = format!(
                    "Log level set to {}",
                    settings.log_level.to_ascii_uppercase()
                );
            } else if lower == "r" {
                settings.refresh_system_fonts();
            } else if lower == "f" {
                if settings.system_fonts.is_empty() {
                    settings.refresh_system_fonts();
                }
                if let Some(font) = settings.cycle_font() {
                    if *font_load_inflight {
                        *font_load_pending = Some(font.clone());
                        settings.status = format!("Queued {}…", font.name);
                    } else {
                        *font_load_inflight = true;
                        settings.status = format!("Loading {}…", font.name);
                        start_load_font(font, proxy.clone());
                    }
                } else {
                    settings.status = "No system fonts found; press R to rescan".to_string();
                }
            } else if lower == "0" {
                settings.selected_font = None;
                let font = SystemFont {
                    name: "Embedded DejaVu Sans Mono".to_string(),
                    path: PathBuf::new(),
                };
                if *font_load_inflight {
                    *font_load_pending = Some(font);
                    settings.status = "Queued embedded font…".to_string();
                } else {
                    *font_load_inflight = true;
                    settings.status = "Loading embedded font…".to_string();
                    start_load_font(font, proxy.clone());
                }
            }
        }
        _ => {}
    }
    if changed {
        persist_settings_state(SettingsState::from_panel(settings));
    }
}

#[derive(Clone)]
struct LogSource {
    label: String,
    kind: LogSourceKind,
}

#[derive(Clone)]
enum LogSourceKind {
    JournalSystem,
    JournalUser,
    Dmesg,
    File(PathBuf),
}

fn default_log_sources() -> Vec<LogSource> {
    vec![
        LogSource {
            label: "journalctl (system)".to_string(),
            kind: LogSourceKind::JournalSystem,
        },
        LogSource {
            label: "journalctl (user)".to_string(),
            kind: LogSourceKind::JournalUser,
        },
        LogSource {
            label: "dmesg".to_string(),
            kind: LogSourceKind::Dmesg,
        },
        LogSource {
            label: "/var/log/pacman.log".to_string(),
            kind: LogSourceKind::File(PathBuf::from("/var/log/pacman.log")),
        },
        LogSource {
            label: "/var/log/boot.log".to_string(),
            kind: LogSourceKind::File(PathBuf::from("/var/log/boot.log")),
        },
        LogSource {
            label: "/var/log/Xorg.0.log".to_string(),
            kind: LogSourceKind::File(PathBuf::from("/var/log/Xorg.0.log")),
        },
        LogSource {
            label: "~/.local/state/term/term.log".to_string(),
            kind: LogSourceKind::File(expand_home_path("~/.local/state/term/term.log")),
        },
    ]
}

fn expand_home_path(path: &str) -> PathBuf {
    if path == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(path));
    }
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(path)
}

#[derive(Debug, Clone)]
enum AppEvent {
    Wake,
    HyprResize(PhysicalSize<u32>),
    HyprPoll,
    Fonts(Vec<Arc<Vec<u8>>>),
    PrimaryFont(Arc<Vec<u8>>),
    FontLoadDone,
    DownloadStatus(String),
}

fn log_source_entries(sources: &[LogSource]) -> Vec<LogSourceEntry> {
    sources
        .iter()
        .map(|source| LogSourceEntry {
            label: source.label.clone(),
            enabled: true,
        })
        .collect()
}

fn build_log_command(source: &LogSource, max_lines: usize, use_sudo: bool) -> Command {
    let (prog, args): (&str, Vec<String>) = match &source.kind {
        LogSourceKind::JournalSystem => (
            "journalctl",
            vec![
                "-n".to_string(),
                max_lines.to_string(),
                "--no-pager".to_string(),
                "-o".to_string(),
                "short-iso".to_string(),
            ],
        ),
        LogSourceKind::JournalUser => (
            "journalctl",
            vec![
                "--user".to_string(),
                "-n".to_string(),
                max_lines.to_string(),
                "--no-pager".to_string(),
                "-o".to_string(),
                "short-iso".to_string(),
            ],
        ),
        LogSourceKind::Dmesg => (
            "dmesg",
            vec!["--color=never".to_string(), "--time-format=iso".to_string()],
        ),
        LogSourceKind::File(path) => (
            "tail",
            vec![
                "-n".to_string(),
                max_lines.to_string(),
                path.display().to_string(),
            ],
        ),
    };

    if use_sudo {
        let mut cmd = Command::new("sudo");
        cmd.arg("-n").arg("--").arg(prog);
        for arg in args {
            cmd.arg(arg);
        }
        cmd
    } else {
        let mut cmd = Command::new(prog);
        for arg in args {
            cmd.arg(arg);
        }
        cmd
    }
}

fn refresh_log_viewer(viewer: &mut LogViewer, sources: &[LogSource], max_lines: usize) {
    if sources.is_empty() {
        viewer.lines.clear();
        viewer.status = "No log sources configured".to_string();
        return;
    }

    let idx = viewer.selected.min(sources.len().saturating_sub(1));
    let source = &sources[idx];
    let mut cmd = build_log_command(source, max_lines, viewer.sudo);
    let output = cmd.output();
    let mut status = String::new();
    let mut lines: Vec<String> = Vec::new();

    match output {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("sudo") && stderr.contains("password") {
                    status = "sudo required; run `sudo -v` in a tab to unlock".to_string();
                } else if let Some(code) = out.status.code() {
                    status = format!("command failed (exit {code}): {}", stderr.trim());
                } else {
                    status = format!("command failed: {}", stderr.trim());
                }
            }
            let stdout = String::from_utf8_lossy(&out.stdout);
            lines = stdout.lines().map(|line| line.to_string()).collect();
        }
        Err(err) => {
            status = format!("command error: {err}");
        }
    }

    if let LogSourceKind::Dmesg = source.kind
        && lines.len() > max_lines
    {
        lines = lines.split_off(lines.len().saturating_sub(max_lines));
    }

    if !viewer.query.is_empty() {
        let needle = viewer.query.to_lowercase();
        lines.retain(|line| line.to_lowercase().contains(&needle));
    }

    viewer.lines = lines;
    if status.is_empty() {
        viewer.status = format!(
            "{} • {} lines • follow {} • sudo {}",
            source.label,
            viewer.lines.len(),
            if viewer.follow { "on" } else { "off" },
            if viewer.sudo { "on" } else { "off" }
        );
    } else {
        viewer.status = status;
    }
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
const COPY_NOTICE_MS: u64 = 1200;
const CELL_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(700);
const LAYOUT_REFRESH_INTERVAL: Duration = Duration::from_millis(750);
const BENCH_DURATION: Duration = Duration::from_secs(2);
const FONT_SIZE_MIN: f32 = 10.0;
const FONT_SIZE_MAX: f32 = 32.0;
const FONT_SIZE_STEP: f32 = 1.0;
const SELECTION_DRAG_THRESHOLD_PX: f64 = 2.0;

struct FpsCounter {
    last_tick: Instant,
    frames: u32,
    fps: f32,
}

impl FpsCounter {
    fn new(now: Instant) -> Self {
        Self {
            last_tick: now,
            frames: 0,
            fps: 0.0,
        }
    }

    fn tick(&mut self, now: Instant) -> f32 {
        self.frames = self.frames.saturating_add(1);
        let elapsed = now.duration_since(self.last_tick);
        if elapsed >= Duration::from_secs(1) {
            let secs = elapsed.as_secs_f32().max(0.001);
            self.fps = self.frames as f32 / secs;
            self.frames = 0;
            self.last_tick = now;
        }
        self.fps
    }
}

struct CopyNotice {
    text: String,
    until: Instant,
}

fn maybe_set_copy_notice(
    settings: &SettingsPanel,
    notice: &mut Option<CopyNotice>,
    text: impl Into<String>,
    now: Instant,
) {
    if settings.show_copy_notice {
        *notice = Some(CopyNotice {
            text: text.into(),
            until: now + Duration::from_millis(COPY_NOTICE_MS),
        });
    }
}
const APP_NAME: &str = "term";

fn app_version() -> &'static str {
    option_env!("TERM_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn main() {
    if std::env::args()
        .skip(1)
        .any(|arg| arg == "--version" || arg == "-V")
    {
        println!("{} {}", APP_NAME, app_version());
        return;
    }
    let initial_log_level = load_settings_state()
        .and_then(|state| parse_log_level(&state.log_level))
        .unwrap_or(log::LevelFilter::Debug);
    logging::init(initial_log_level);
    if let Err(err) = run() {
        error!("error: {err}");
    }
}

use anyhow::Result;

#[allow(deprecated)]
fn run() -> Result<()> {
    let mut event_loop_builder = EventLoop::<AppEvent>::with_user_event();
    let event_loop = event_loop_builder.build()?;
    let proxy = event_loop.create_proxy();
    let hypr_ipc = hyprland_ipc_info();
    let mut hypr_poll = hypr_ipc.as_ref().map(|info| HyprPollState {
        info: info.clone(),
        last_check: Instant::now(),
        interval: Duration::from_millis(350),
        last_size: None,
    });
    if let Some(info) = hypr_ipc.clone() {
        spawn_hyprland_listener(proxy.clone(), info);
    }
    // Respect compositor-provided activation token (e.g. Hyprland binds) so the new
    // window grabs focus on the originating workspace instead of bouncing to
    // whichever workspace already has focus.
    let activation_token = None;
    let base_title = format!("{} {}", APP_NAME, app_version());
    let mut window_attributes = Window::default_attributes()
        .with_inner_size(LogicalSize::new(900.0, 600.0))
        .with_title(&base_title);
    if let Some(token) = activation_token {
        window_attributes = window_attributes.with_activation_token(token);
    }
    let window = Arc::new(event_loop.create_window(window_attributes)?);
    reset_activation_token_env();

    let mut scale_factor = window.scale_factor();
    let mut font_size_base = FONT_SIZE;
    let mut font_size = font_size_base * scale_factor as f32;
    let mut glyphs = GlyphCache::new(Arc::new(FONT_DATA.to_vec()), font_size);
    if let Some(font) = load_kitty_primary_font() {
        glyphs.set_primary_font(font);
    }
    spawn_font_loader(proxy.clone());
    let (mut cell_w, mut cell_h) = glyphs.cell_size();
    let mut history_menu = HistoryMenu::new();
    let mut context_menu = ContextMenu::new();
    let mut autocomplete = AutocompleteEngine::load();
    let mut cwd_history: Vec<CwdHistoryEntry> = Vec::new();
    let mut settings = SettingsPanel::new();
    if let Some(state) = load_settings_state() {
        settings.scrollback_enabled = state.scrollback_enabled;
        settings.show_fps = state.show_fps;
        settings.show_copy_notice = state.show_copy_notice;
        settings.render_mode =
            parse_render_mode(&state.render_mode).unwrap_or(settings.render_mode);
        settings.log_level = state.log_level;
    }
    if let Some(level) = parse_log_level(&settings.log_level) {
        logging::set_level(level);
    }
    let mut fps_counter = FpsCounter::new(Instant::now());
    let mut copy_notice: Option<CopyNotice> = None;

    let size = window.inner_size();
    let surface_texture = SurfaceTexture::new(size.width, size.height, window.clone());
    let mut pixels = Pixels::new(size.width, size.height, surface_texture)?;
    let gpu_enabled = env::var("TERM_GPU")
        .map(|v| {
            let v = v.to_ascii_lowercase();
            !(v == "0" || v == "false" || v == "off" || v == "no")
        })
        .unwrap_or(true);
    let blink_enabled = env::var("TERM_BLINK")
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
    let mut next_layout_refresh = start_time + LAYOUT_REFRESH_INTERVAL;
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
    let mut log_viewer = LogViewer::new();
    let log_sources = default_log_sources();
    log_viewer.set_sources(log_source_entries(&log_sources));
    let mut log_refresh_needed = false;
    let mut log_last_refresh = start_time
        .checked_sub(Duration::from_secs(1))
        .unwrap_or(start_time);
    let log_refresh_interval = Duration::from_millis(750);
    let mut debug_overlay = DebugOverlay::new(layout.cols, layout.rows);
    debug_overlay.set_render_mode(settings.render_mode);

    let mut tabs = Vec::new();
    tabs.push(spawn_shell_tab(
        layout.cols as u16,
        layout.rows as u16,
        layout.usable_width,
        layout.usable_height,
        proxy.clone(),
        None,
        settings.scrollback_enabled,
        start_time,
        CELL_BLINK_INTERVAL,
        CURSOR_BLINK_INTERVAL,
    )?);
    refresh_tab_titles(&mut tabs);
    let mut active_tab: usize = 0;
    let mut modifiers = ModifiersState::default();
    let mut focused = true;
    let mut last_cursor_pos: Option<(f64, f64)> = None;
    let mut needs_redraw = true;
    let mut cursor_only_redraw = false;
    let mut font_load_inflight = false;
    let mut font_load_pending: Option<SystemFont> = None;

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
            event: WindowEvent::Resized(_new_size),
            ..
        } => {
            let size = resolve_window_size(&window, &hypr_ipc);
            let changed = refresh_layout_for_size(
                size,
                cell_w,
                cell_h,
                tab_bar_height,
                &mut pixels,
                &mut tabs,
                &mut debug_overlay,
                &mut gpu_renderer,
                &mut layout,
                &mut frame_width,
                &mut frame_height,
            );
            if !changed {
                return;
            }
            debug!(
                "window resize: size={}x{} frame={}x{} cell={}x{} layout={}x{} usable={}x{}",
                size.width,
                size.height,
                frame_width,
                frame_height,
                cell_w,
                cell_h,
                layout.cols,
                layout.rows,
                layout.usable_width,
                layout.usable_height
            );
            window.request_redraw();
            needs_redraw = true;
        }
        Event::WindowEvent {
            event:
                WindowEvent::ScaleFactorChanged {
                    scale_factor: new_scale_factor,
                    mut inner_size_writer,
                    ..
                },
            ..
        } => {
            if (new_scale_factor - scale_factor).abs() > f64::EPSILON {
                scale_factor = new_scale_factor;
                let next_size = font_size_base * scale_factor as f32;
                if (next_size - font_size).abs() > f32::EPSILON {
                    font_size = next_size;
                    glyphs.set_size(font_size);
                    if let Some(renderer) = gpu_renderer.as_mut() {
                        renderer.clear_atlas();
                    }
                    let (new_w, new_h) = glyphs.cell_size();
                    if new_w != cell_w || new_h != cell_h {
                        cell_w = new_w;
                        cell_h = new_h;
                        tab_bar_height = cell_h + 12;
                    }
                }
            }
            let inner = window.inner_size();
            let _ = inner_size_writer.request_inner_size(inner);
            let size = resolve_window_size(&window, &hypr_ipc);
            refresh_layout_for_size(
                size,
                cell_w,
                cell_h,
                tab_bar_height,
                &mut pixels,
                &mut tabs,
                &mut debug_overlay,
                &mut gpu_renderer,
                &mut layout,
                &mut frame_width,
                &mut frame_height,
            );
            window.request_redraw();
            needs_redraw = true;
        }
        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
            let current_size = resolve_window_size(&window, &hypr_ipc);
            if current_size.width == 0 || current_size.height == 0 {
                return;
            }
            refresh_layout_for_size(
                current_size,
                cell_w,
                cell_h,
                tab_bar_height,
                &mut pixels,
                &mut tabs,
                &mut debug_overlay,
                &mut gpu_renderer,
                &mut layout,
                &mut frame_width,
                &mut frame_height,
            );

            let mut active_window_title: Option<String> = None;
            for (idx, tab) in tabs.iter_mut().enumerate() {
                let TabKind::Shell(shell) = &mut tab.kind;
                let mut disconnected = false;
                let mut saw_output = false;
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
                            tab.terminal.set_view_offset(0);
                            saw_output = true;
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => {
                            disconnected = true;
                            break;
                        }
                    }
                }
                if saw_output {
                    needs_redraw = true;
                }
                if disconnected && !shell.exited {
                    shell.exited = true;
                    tab.selection_anchor = None;
                    tab.selection_edge = None;
                    tab.selecting = false;
                    tab.pending_selection = None;
                    tab.mouse_down_pos = None;
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
                if let Some(title) = tab.terminal.window_title().map(|t| t.to_string())
                    && tab.title != title
                {
                    tab.title = title;
                }
                if idx == active_tab {
                    active_window_title = tab.terminal.window_title().map(|t| t.to_string());
                    tab.link_ranges = detect_link_ranges(&tab.terminal);
                }
            }
            let title = match active_window_title.as_deref() {
                Some(active) if !active.is_empty() => format!("{active} - {base_title}"),
                _ => base_title.clone(),
            };
            window.set_title(&title);

            let now = Instant::now();
            if now.duration_since(last_cwd_poll) >= Duration::from_secs(1) {
                refresh_cwd_for_tabs(&mut tabs, &mut cwd_history);
                last_cwd_poll = now;
            }

            let tab_renders: Vec<_> = tabs
                .iter()
                .map(|tab| TabRender {
                    title: &tab.title,
                    terminal: &tab.terminal,
                    link_hover: tab.link_hover,
                    hover_link: tab.hover_link.as_deref(),
                    hover_link_range: tab.hover_link_range,
                    link_ranges: Some(&tab.link_ranges),
                    selection: selection_bounds(tab),
                })
                .collect();
            let now = Instant::now();
            if copy_notice
                .as_ref()
                .map(|notice| notice.until <= now)
                .unwrap_or(false)
            {
                copy_notice = None;
            }
            let notice_text = copy_notice.as_ref().map(|notice| notice.text.as_str());
            let fps = fps_counter.tick(now);
            let benchmark_active = debug_overlay.benchmark_active();
            let render_start = Instant::now();
            let (cell_blink_on, cursor_blink_on) = tabs
                .get(active_tab)
                .map(|tab| (tab.cell_blink_on, tab.cursor_blink_on))
                .unwrap_or((true, true));
            let render_outcome = render_frame(RenderInputs {
                pixels: &mut pixels,
                gpu_renderer: gpu_renderer.as_mut(),
                tabs: &tab_renders,
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
                help_visible: help.should_show(Instant::now()),
                history_menu: &mut history_menu,
                context_menu: &mut context_menu,
                cheatsheet: &cheatsheet,
                settings: &settings,
                log_viewer: &log_viewer,
                debug_overlay: &mut debug_overlay,
                fps,
                notice_text,
                border_thickness: BORDER_THICKNESS,
                border_radius: BORDER_RADIUS,
                border_inset: BORDER_INSET,
                cursor_only: cursor_only_redraw && !needs_redraw && !benchmark_active,
                cell_blink_on,
                cursor_blink_on,
            });
            let render_ms = render_start.elapsed().as_secs_f32() * 1000.0;
            let mut benchmark_request_redraw = false;
            if benchmark_active {
                let finished = debug_overlay.record_benchmark_frame(Instant::now(), render_ms);
                benchmark_request_redraw = debug_overlay.benchmark_active() || finished;
                if benchmark_request_redraw {
                    window.request_redraw();
                }
            }
            if !render_outcome.keep_running {
                elwt.exit();
            }
            needs_redraw = render_outcome.needs_redraw || benchmark_request_redraw;
            cursor_only_redraw = false;
        }
        Event::UserEvent(app_event) => match app_event {
            AppEvent::Wake => {
                needs_redraw = true;
                window.request_redraw();
            }
            AppEvent::HyprResize(size) => {
                let size = if size.width > 0 && size.height > 0 {
                    size
                } else {
                    window.inner_size()
                };
                let changed = refresh_layout_for_size(
                    size,
                    cell_w,
                    cell_h,
                    tab_bar_height,
                    &mut pixels,
                    &mut tabs,
                    &mut debug_overlay,
                    &mut gpu_renderer,
                    &mut layout,
                    &mut frame_width,
                    &mut frame_height,
                );
                if !changed {
                    return;
                }
                debug!(
                    "hypr resize: size={}x{} frame={}x{} cell={}x{} layout={}x{} usable={}x{}",
                    size.width,
                    size.height,
                    frame_width,
                    frame_height,
                    cell_w,
                    cell_h,
                    layout.cols,
                    layout.rows,
                    layout.usable_width,
                    layout.usable_height
                );
                needs_redraw = true;
                window.request_redraw();
            }
            AppEvent::HyprPoll => {
                if let Some(state) = hypr_poll.as_mut()
                    && let Some(size) =
                        fetch_hyprland_size(&state.info.request_socket, state.info.pid)
                    && state.last_size != Some(size)
                {
                    state.last_size = Some(size);
                    let _ = proxy.send_event(AppEvent::HyprResize(size));
                }
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
                    refresh_layout_for_size(
                        window.inner_size(),
                        cell_w,
                        cell_h,
                        tab_bar_height,
                        &mut pixels,
                        &mut tabs,
                        &mut debug_overlay,
                        &mut gpu_renderer,
                        &mut layout,
                        &mut frame_width,
                        &mut frame_height,
                    );
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
                    refresh_layout_for_size(
                        window.inner_size(),
                        cell_w,
                        cell_h,
                        tab_bar_height,
                        &mut pixels,
                        &mut tabs,
                        &mut debug_overlay,
                        &mut gpu_renderer,
                        &mut layout,
                        &mut frame_width,
                        &mut frame_height,
                    );
                }
                needs_redraw = true;
                window.request_redraw();
            }
            AppEvent::FontLoadDone => {
                font_load_inflight = false;
                if let Some(next_font) = font_load_pending.take() {
                    font_load_inflight = true;
                    settings.status = format!("Loading {}…", next_font.name);
                    start_load_font(next_font, proxy.clone());
                }
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
            if let Some(tab) = tabs.get_mut(active_tab)
                && tab.terminal.view_offset > 0
            {
                tab.selection_anchor = None;
                tab.selection_edge = None;
                tab.selecting = false;
                tab.pending_selection = None;
                tab.mouse_down_pos = None;
                tab.terminal.set_view_offset(0);
                needs_redraw = true;
                return;
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
                log_viewer.close();
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
                log_viewer.close();
                needs_redraw = true;
                return;
            }

            if matches!(event.logical_key, Key::Named(NamedKey::F5))
                && event.state == ElementState::Pressed
            {
                log_viewer.toggle();
                if log_viewer.is_open() {
                    cheatsheet.close();
                    history_menu.close();
                    context_menu.close();
                    settings.close();
                    log_viewer.status = "Loading logs…".to_string();
                    log_viewer.lines.clear();
                    log_refresh_needed = true;
                    log_viewer.focus = LogFocus::Sources;
                }
                needs_redraw = true;
                return;
            }

            if log_viewer.is_open()
                && handle_log_viewer_key(&event, &mut log_viewer, &mut log_refresh_needed)
            {
                needs_redraw = true;
                return;
            }

            if debug_overlay.is_active() {
                if event.state == ElementState::Pressed {
                    match &event.logical_key {
                        Key::Character(text) if text.eq_ignore_ascii_case("r") => {
                            debug_overlay.cycle_render_mode();
                            settings.render_mode = debug_overlay.render_mode();
                            persist_settings_state(SettingsState::from_panel(&settings));
                        }
                        Key::Character(text) if text.eq_ignore_ascii_case("i") => {
                            debug_overlay.cycle_input_mode();
                        }
                        Key::Character(text) if text.eq_ignore_ascii_case("b") => {
                            debug_overlay.toggle_benchmark(Instant::now(), BENCH_DURATION);
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
                    log_viewer.close();
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
                handle_settings_key(
                    event,
                    &mut settings,
                    &mut debug_overlay,
                    &proxy,
                    &mut tabs,
                    &mut font_load_inflight,
                    &mut font_load_pending,
                );
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
                    &mut copy_notice,
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
            if modifiers.control_key() {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => (pos.y / 30.0) as f32,
                };
                if lines != 0.0 {
                    let step = if lines > 0.0 {
                        lines.ceil() as i32
                    } else {
                        lines.floor() as i32
                    };
                    let next_base = (font_size_base + FONT_SIZE_STEP * step as f32)
                        .clamp(FONT_SIZE_MIN, FONT_SIZE_MAX);
                    if (next_base - font_size_base).abs() > f32::EPSILON {
                        font_size_base = next_base;
                        font_size = font_size_base * scale_factor as f32;
                        glyphs.set_size(font_size);
                        if let Some(renderer) = gpu_renderer.as_mut() {
                            renderer.clear_atlas();
                        }
                        let (new_w, new_h) = glyphs.cell_size();
                        if new_w != cell_w || new_h != cell_h {
                            cell_w = new_w;
                            cell_h = new_h;
                            tab_bar_height = cell_h + 12;
                            refresh_layout_for_size(
                                window.inner_size(),
                                cell_w,
                                cell_h,
                                tab_bar_height,
                                &mut pixels,
                                &mut tabs,
                                &mut debug_overlay,
                                &mut gpu_renderer,
                                &mut layout,
                                &mut frame_width,
                                &mut frame_height,
                            );
                        }
                        needs_redraw = true;
                        window.request_redraw();
                    }
                }
                return;
            }
            if log_viewer.is_open() {
                let step = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as i32,
                    MouseScrollDelta::PixelDelta(pos) => (pos.y / 18.0) as i32,
                };
                if step != 0 {
                    log_viewer.follow = false;
                    if step > 0 {
                        log_viewer.scroll = log_viewer.scroll.saturating_sub(step as usize);
                    } else {
                        log_viewer.scroll = log_viewer
                            .scroll
                            .saturating_add(step.unsigned_abs() as usize);
                    }
                    needs_redraw = true;
                }
                return;
            }
            help.bump(Instant::now());
            if context_menu.open {
                context_menu.close();
                return;
            }
            if let Some((mx, my)) = last_cursor_pos
                && let Some(cell) = pos_to_cell(mx, my, &layout, cell_w, cell_h, active_tab, &tabs)
                && let Some(tab) = tabs.get_mut(active_tab)
                && tab.terminal.mouse_btn_report
            {
                let kind = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) if y > 0.0 => {
                        term_core::terminal::MouseEventKind::WheelUp
                    }
                    winit::event::MouseScrollDelta::LineDelta(_, y) if y < 0.0 => {
                        term_core::terminal::MouseEventKind::WheelDown
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) if pos.y > 0.0 => {
                        term_core::terminal::MouseEventKind::WheelUp
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) if pos.y < 0.0 => {
                        term_core::terminal::MouseEventKind::WheelDown
                    }
                    _ => term_core::terminal::MouseEventKind::WheelUp,
                };
                let TabKind::Shell(shell) = &tab.kind;
                tab.terminal
                    .report_mouse_event(cell.0, cell.1, kind, &shell.writer);
                needs_redraw = true;
                return;
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
                            term_core::terminal::MouseEventKind::Motion,
                            &shell.writer,
                        );
                        tab.link_hover = None;
                        tab.hover_link = None;
                        tab.hover_link_range = None;
                    } else if tab.selecting {
                        tab.selection_edge = Some(cell);
                        tab.link_hover = None;
                        tab.hover_link = None;
                        tab.hover_link_range = None;
                    } else if let Some((sx, sy)) = tab.mouse_down_pos
                        && let Some(anchor) = tab.pending_selection
                    {
                        let dx = position.x - sx;
                        let dy = position.y - sy;
                        if dx.abs() > SELECTION_DRAG_THRESHOLD_PX
                            || dy.abs() > SELECTION_DRAG_THRESHOLD_PX
                        {
                            tab.selecting = true;
                            tab.selection_anchor = Some(anchor);
                            tab.selection_edge = Some(cell);
                            tab.pending_selection = None;
                        }
                    } else {
                        let cell_data = tab.terminal.display_cell(cell.0, cell.1);
                        if let Some(link) = cell_data.hyperlink.clone() {
                            let display = parse_path_candidate(&link, tab)
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| link.clone());
                            tab.link_hover = Some(cell);
                            tab.hover_link = Some(display);
                            tab.hover_link_range = None;
                        } else if let Some(link) =
                            detect_link_text_at_cell(&tab.terminal, cell.0, cell.1)
                        {
                            let display = parse_path_candidate(&link.text, tab)
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| link.text.clone());
                            tab.link_hover = Some(cell);
                            tab.hover_link = Some(display);
                            tab.hover_link_range = Some((cell.1, link.start, link.end));
                        } else {
                            tab.link_hover = None;
                            tab.hover_link = None;
                            tab.hover_link_range = None;
                        }
                    }
                }
            } else if let Some(tab) = tabs.get_mut(active_tab) {
                tab.link_hover = None;
                tab.hover_link = None;
                tab.hover_link_range = None;
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
            if log_viewer.is_open() {
                if state == ElementState::Released && matches!(button, MouseButton::Left) {
                    log_viewer.focus = LogFocus::Logs;
                    needs_redraw = true;
                }
                return;
            }
            if history_menu.open {
                if button == MouseButton::Left && state == ElementState::Released {
                    if let Some((mx, my)) = last_cursor_pos
                        && let Some(sel_idx) = history_menu.click(mx, my)
                    {
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
                                maybe_set_copy_notice(
                                    &settings,
                                    &mut copy_notice,
                                    "Selection copied",
                                    Instant::now(),
                                );
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
                                                term_core::terminal::MouseEventKind::Press(0),
                                                &shell.writer,
                                            );
                                        }
                                        tab.selecting = false;
                                    } else {
                                        tab.selecting = false;
                                        tab.selection_anchor = None;
                                        tab.selection_edge = None;
                                        tab.pending_selection = cell;
                                        tab.mouse_down_pos = Some((mx, my));
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
                                                term_core::terminal::MouseEventKind::Release,
                                                &shell.writer,
                                            );
                                        }
                                    } else if tab.selecting {
                                        tab.selecting = false;
                                        if let (Some(a), Some(b)) =
                                            (tab.selection_anchor, tab.selection_edge)
                                        {
                                            copy_selection_to_clipboard(&tab.terminal, a, b);
                                            maybe_set_copy_notice(
                                                &settings,
                                                &mut copy_notice,
                                                "Selection copied",
                                                Instant::now(),
                                            );
                                        }
                                    } else if let Some((c, r)) = cell {
                                        let cell = tab.terminal.display_cell(c, r);
                                        let link = if let Some(link) = cell.hyperlink.clone() {
                                            Some(link)
                                        } else {
                                            detect_link_text_at_cell(&tab.terminal, c, r)
                                                .map(|link| link.text)
                                        };
                                        if let Some(link) = link {
                                            let resolved_path = resolve_existing_path(&link, tab);
                                            let copy_value = if looks_like_url(&link) {
                                                link.clone()
                                            } else if let Some(path) = resolved_path.as_ref() {
                                                path.display().to_string()
                                            } else {
                                                link.clone()
                                            };
                                            if let Err(e) = copy_text_to_clipboard(&copy_value) {
                                                error!("Failed to copy to clipboard: {e}");
                                            }
                                            maybe_set_copy_notice(
                                                &settings,
                                                &mut copy_notice,
                                                "Link copied",
                                                Instant::now(),
                                            );
                                            // If Ctrl is held, attempt to open in system handler.
                                            if modifiers.control_key() {
                                                if let Some(path) = resolved_path {
                                                    open_path(&path);
                                                } else {
                                                    open_link(&link);
                                                }
                                            }
                                        }
                                        tab.selection_anchor = None;
                                        tab.selection_edge = None;
                                        tab.pending_selection = None;
                                        tab.mouse_down_pos = None;
                                    } else {
                                        tab.selection_anchor = None;
                                        tab.selection_edge = None;
                                        tab.pending_selection = None;
                                        tab.mouse_down_pos = None;
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
            let now = Instant::now();
            for tab in tabs.iter_mut() {
                tab.cell_blink_on = true;
                tab.cursor_blink_on = true;
                tab.next_cell_blink = now + CELL_BLINK_INTERVAL;
                tab.next_cursor_blink = now + CURSOR_BLINK_INTERVAL;
            }
            if let Some(tab) = tabs.get(active_tab) {
                match &tab.kind {
                    TabKind::Shell(shell) => {
                        tab.terminal.report_focus(focus, &shell.writer);
                    }
                }
            }
            needs_redraw = true;
            window.request_redraw();
        }
        Event::AboutToWait => {
            let now = Instant::now();
            if log_viewer.is_open()
                && (log_refresh_needed
                    || (log_viewer.follow
                        && now.duration_since(log_last_refresh) >= log_refresh_interval))
            {
                refresh_log_viewer(&mut log_viewer, &log_sources, 200);
                log_last_refresh = now;
                log_refresh_needed = false;
                needs_redraw = true;
                window.request_redraw();
            }
            let mut blink_changed = false;
            if blink_enabled && focused {
                for tab in tabs.iter_mut() {
                    while now >= tab.next_cell_blink {
                        tab.cell_blink_on = !tab.cell_blink_on;
                        tab.next_cell_blink += CELL_BLINK_INTERVAL;
                        blink_changed = true;
                    }
                    while now >= tab.next_cursor_blink {
                        tab.cursor_blink_on = !tab.cursor_blink_on;
                        tab.next_cursor_blink += CURSOR_BLINK_INTERVAL;
                        blink_changed = true;
                    }
                }
            } else {
                for tab in tabs.iter_mut() {
                    if !tab.cell_blink_on || !tab.cursor_blink_on {
                        tab.cell_blink_on = true;
                        tab.cursor_blink_on = true;
                        blink_changed = true;
                    }
                }
            }
            if blink_changed {
                needs_redraw = true;
                window.request_redraw();
            }
            if now >= next_layout_refresh {
                if refresh_layout_for_size(
                    resolve_window_size(&window, &hypr_ipc),
                    cell_w,
                    cell_h,
                    tab_bar_height,
                    &mut pixels,
                    &mut tabs,
                    &mut debug_overlay,
                    &mut gpu_renderer,
                    &mut layout,
                    &mut frame_width,
                    &mut frame_height,
                ) {
                    needs_redraw = true;
                }
                next_layout_refresh = now + LAYOUT_REFRESH_INTERVAL;
            }
            if needs_redraw {
                window.request_redraw();
            }
            let mut next_wake = next_layout_refresh;
            if focused
                && let Some(next_blink) = tabs
                    .iter()
                    .map(|tab| tab.next_cell_blink.min(tab.next_cursor_blink))
                    .min()
                && next_blink < next_wake
            {
                next_wake = next_blink;
            }
            if let Some(state) = hypr_poll.as_mut() {
                if now.duration_since(state.last_check) >= state.interval {
                    state.last_check = now;
                    let _ = proxy.send_event(AppEvent::HyprPoll);
                }
                let poll_due = state.last_check + state.interval;
                if poll_due < next_wake {
                    next_wake = poll_due;
                }
            }
            elwt.set_control_flow(ControlFlow::WaitUntil(next_wake));
        }
        _ => {}
    })?;

    Ok(())
}

struct HyprPollState {
    info: HyprIpcInfo,
    last_check: Instant,
    interval: Duration,
    last_size: Option<PhysicalSize<u32>>,
}

#[derive(Clone)]
struct HyprIpcInfo {
    event_socket: String,
    request_socket: String,
    pid: u32,
}

fn hyprland_ipc_info() -> Option<HyprIpcInfo> {
    #[cfg(unix)]
    {
        let sig = match env::var("HYPRLAND_INSTANCE_SIGNATURE") {
            Ok(sig) if !sig.trim().is_empty() => sig,
            _ => return None,
        };
        let runtime_dir = match env::var("XDG_RUNTIME_DIR") {
            Ok(dir) if !dir.trim().is_empty() => dir,
            _ => {
                warn!("HYPRLAND_INSTANCE_SIGNATURE set but XDG_RUNTIME_DIR missing");
                return None;
            }
        };
        Some(HyprIpcInfo {
            event_socket: format!("{}/hypr/{}/.socket2.sock", runtime_dir, sig),
            request_socket: format!("{}/hypr/{}/.socket.sock", runtime_dir, sig),
            pid: std::process::id(),
        })
    }
    #[cfg(not(unix))]
    {
        None
    }
}

fn spawn_hyprland_listener(proxy: EventLoopProxy<AppEvent>, info: HyprIpcInfo) {
    #[cfg(unix)]
    {
        let HyprIpcInfo {
            event_socket,
            request_socket,
            pid,
        } = info;
        thread::spawn(move || {
            loop {
                let mut last_size: Option<PhysicalSize<u32>> = None;
                match UnixStream::connect(&event_socket) {
                    Ok(stream) => {
                        let mut reader = std::io::BufReader::new(stream);
                        let mut line = String::new();
                        loop {
                            line.clear();
                            match reader.read_line(&mut line) {
                                Ok(0) => break,
                                Ok(_) => {
                                    if line.trim().is_empty() {
                                        continue;
                                    }
                                    if maybe_send_hypr_resize(
                                        &proxy,
                                        &request_socket,
                                        pid,
                                        &mut last_size,
                                    ) {
                                        thread::sleep(Duration::from_millis(50));
                                        let _ = maybe_send_hypr_resize(
                                            &proxy,
                                            &request_socket,
                                            pid,
                                            &mut last_size,
                                        );
                                    } else {
                                        let _ = proxy.send_event(AppEvent::Wake);
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    }
                    Err(err) => {
                        warn!("Hyprland IPC connect failed: {err}");
                    }
                }
                thread::sleep(Duration::from_millis(500));
            }
        });
    }
}

fn maybe_send_hypr_resize(
    proxy: &EventLoopProxy<AppEvent>,
    socket_path: &str,
    pid: u32,
    last_size: &mut Option<PhysicalSize<u32>>,
) -> bool {
    if let Some(size) = fetch_hyprland_size(socket_path, pid) {
        if *last_size != Some(size) {
            *last_size = Some(size);
            let _ = proxy.send_event(AppEvent::HyprResize(size));
        }
        return true;
    }
    false
}

fn fetch_hyprland_size(socket_path: &str, pid: u32) -> Option<PhysicalSize<u32>> {
    #[cfg(unix)]
    {
        let mut stream = UnixStream::connect(socket_path).ok()?;
        stream.write_all(b"j/clients\n").ok()?;
        let mut buf = String::new();
        stream.read_to_string(&mut buf).ok()?;
        let value: serde_json::Value = serde_json::from_str(&buf).ok()?;
        let clients = value.as_array()?;
        for client in clients {
            if client.get("pid").and_then(|v| v.as_u64()) != Some(pid as u64) {
                continue;
            }
            let size = client.get("size").and_then(|v| v.as_array())?;
            let width = size.first()?.as_u64()? as u32;
            let height = size.get(1)?.as_u64()? as u32;
            return Some(PhysicalSize::new(width, height));
        }
        None
    }
    #[cfg(not(unix))]
    {
        let _ = socket_path;
        let _ = pid;
        None
    }
}

fn resolve_window_size(
    window: &winit::window::Window,
    hypr_ipc: &Option<HyprIpcInfo>,
) -> PhysicalSize<u32> {
    let window_size = window.inner_size();
    if let Some(info) = hypr_ipc.as_ref()
        && let Some(size) = fetch_hyprland_size(&info.request_socket, info.pid)
        && size.width > 0
        && size.height > 0
    {
        return size;
    }
    window_size
}

fn default_shell() -> CommandBuilder {
    #[cfg(windows)]
    {
        CommandBuilder::new("cmd.exe")
    }
    #[cfg(not(windows))]
    {
        let shell = env::var("TERM_FORCE_SHELL")
            .or_else(|_| env::var("SHELL"))
            .unwrap_or_else(|_| "/bin/bash".to_string());
        let shell_name = Path::new(&shell)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(shell.as_str());
        let mut cmd = CommandBuilder::new(&shell);
        let clean_shell = env::var("TERM_CLEAN_SHELL").is_ok();
        if clean_shell {
            // Keep the shell clean for debugging: skip user rc/profile when requested.
            match shell_name {
                "bash" | "sh" => {
                    cmd.arg("--noprofile");
                    if let Some(rcfile) = ensure_term_rcfile() {
                        cmd.arg("--rcfile");
                        cmd.arg(rcfile);
                    } else {
                        cmd.arg("--norc");
                        cmd.env("BASH_ENV", "/dev/null");
                    }
                }
                "zsh" => {
                    cmd.arg("-f");
                }
                _ => {}
            }
        }
        if clean_shell && env::var("TERM_KEEP_PROMPT").is_err() {
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

fn is_link_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(
            ch,
            '-' | '_'
                | '.'
                | '/'
                | ':'
                | '?'
                | '#'
                | '&'
                | '='
                | '%'
                | '+'
                | '~'
                | '@'
                | '!'
                | '$'
                | '\''
                | '('
                | ')'
                | '*'
                | ','
                | ';'
        )
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LinkMatch {
    text: String,
    start: usize,
    end: usize,
}

fn is_trim_start(ch: char) -> bool {
    matches!(ch, '(' | '[' | '{' | '<' | '"' | '\'')
}

fn is_trim_end(ch: char) -> bool {
    matches!(
        ch,
        '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '>' | '"' | '\''
    )
}

fn trim_link_token(token: &str) -> String {
    let mut trimmed = token.trim();
    trimmed = trimmed.trim_start_matches(is_trim_start);
    trimmed = trimmed.trim_end_matches(is_trim_end);
    trimmed.to_string()
}

fn looks_like_url(token: &str) -> bool {
    token.starts_with("http://") || token.starts_with("https://") || token.starts_with("file://")
}

fn looks_like_file_path(token: &str) -> bool {
    if token.contains("://") {
        return false;
    }
    token.starts_with('/')
        || token.starts_with("./")
        || token.starts_with("../")
        || token.starts_with("~/")
        || token.contains('/')
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = bytes[i + 1];
            let lo = bytes[i + 2];
            let decode = |b| match b {
                b'0'..=b'9' => Some(b - b'0'),
                b'a'..=b'f' => Some(b - b'a' + 10),
                b'A'..=b'F' => Some(b - b'A' + 10),
                _ => None,
            };
            if let (Some(hi), Some(lo)) = (decode(hi), decode(lo)) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn parse_path_candidate(candidate: &str, tab: &Tab) -> Option<PathBuf> {
    if let Some(rest) = candidate.strip_prefix("file://") {
        let rest = rest.strip_prefix("localhost/").unwrap_or(rest);
        let decoded = percent_decode(rest);
        if decoded.starts_with('/') {
            return Some(PathBuf::from(decoded));
        }
        return None;
    }

    if let Some(rest) = candidate.strip_prefix("~/") {
        let home = dirs::home_dir()?;
        return Some(home.join(rest));
    }

    if candidate.starts_with('/') {
        return Some(PathBuf::from(candidate));
    }

    if candidate.starts_with("./") || candidate.starts_with("../") || candidate.contains('/') {
        let base = tab_current_dir(tab)?;
        return Some(base.join(candidate));
    }

    None
}

fn resolve_existing_path(candidate: &str, tab: &Tab) -> Option<PathBuf> {
    let path = parse_path_candidate(candidate, tab)?;
    if path.exists() { Some(path) } else { None }
}

fn detect_link_text_at_cell(term: &Terminal, col: usize, row: usize) -> Option<LinkMatch> {
    if row >= term.rows || col >= term.cols {
        return None;
    }
    let mut chars = Vec::with_capacity(term.cols);
    for c in 0..term.cols {
        let cell = term.display_cell(c, row);
        let ch = if cell.wide_continuation || cell.text.is_empty() {
            ' '
        } else {
            cell.text.chars().next().unwrap_or(' ')
        };
        chars.push(ch);
    }
    let ch = *chars.get(col)?;
    if ch.is_whitespace() || !is_link_char(ch) {
        return None;
    }
    let mut start = col;
    while start > 0 && is_link_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end + 1 < chars.len() && is_link_char(chars[end + 1]) {
        end += 1;
    }
    let token: String = chars[start..=end].iter().collect();
    let token = trim_link_token(&token);
    if token.is_empty() {
        return None;
    }
    if looks_like_url(&token) || looks_like_file_path(&token) {
        Some(LinkMatch {
            text: token,
            start,
            end,
        })
    } else {
        None
    }
}

fn detect_link_ranges(term: &Terminal) -> Vec<Vec<(usize, usize)>> {
    let mut ranges = Vec::with_capacity(term.rows);
    for row in 0..term.rows {
        let mut row_ranges = Vec::new();
        let mut chars = Vec::with_capacity(term.cols);
        for col in 0..term.cols {
            let cell = term.display_cell(col, row);
            let ch = if cell.wide_continuation || cell.text.is_empty() {
                ' '
            } else {
                cell.text.chars().next().unwrap_or(' ')
            };
            chars.push(ch);
        }

        let mut col = 0;
        while col < chars.len() {
            if !is_link_char(chars[col]) {
                col += 1;
                continue;
            }
            let start = col;
            while col + 1 < chars.len() && is_link_char(chars[col + 1]) {
                col += 1;
            }
            let end = col;
            let mut trimmed_start = start;
            let mut trimmed_end = end;
            while trimmed_start <= trimmed_end && is_trim_start(chars[trimmed_start]) {
                trimmed_start += 1;
            }
            while trimmed_end >= trimmed_start && is_trim_end(chars[trimmed_end]) {
                if trimmed_end == 0 {
                    break;
                }
                trimmed_end -= 1;
            }
            if trimmed_start <= trimmed_end {
                let token: String = chars[trimmed_start..=trimmed_end].iter().collect();
                if looks_like_url(&token) || looks_like_file_path(&token) {
                    row_ranges.push((trimmed_start, trimmed_end));
                }
            }
            col += 1;
        }
        ranges.push(row_ranges);
    }
    ranges
}

fn is_text_file(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 4096];
    let Ok(read_len) = file.read(&mut buf) else {
        return false;
    };
    if buf[..read_len].contains(&0) {
        return false;
    }
    std::str::from_utf8(&buf[..read_len]).is_ok()
}

fn open_path(path: &Path) {
    #[cfg(target_os = "linux")]
    {
        let bin = if path.is_dir() {
            "nautilus"
        } else if is_text_file(path) {
            "geany"
        } else {
            "nautilus"
        };
        let _ = Command::new(bin)
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
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

    if prompt_cols == 0
        && let Some((idx, ch)) = prefix
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
    copy_notice: &mut Option<CopyNotice>,
    cwd_history: &[CwdHistoryEntry],
) {
    if event.state != ElementState::Pressed {
        return;
    }

    if matches!(event.logical_key, Key::Named(NamedKey::Escape))
        && let Some(tab) = tabs.get_mut(*active_tab)
        && (tab.selection_anchor.is_some() || tab.selection_edge.is_some())
    {
        tab.selection_anchor = None;
        tab.selection_edge = None;
        tab.selecting = false;
        tab.pending_selection = None;
        tab.mouse_down_pos = None;
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
                Instant::now(),
                CELL_BLINK_INTERVAL,
                CURSOR_BLINK_INTERVAL,
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
                Instant::now(),
                CELL_BLINK_INTERVAL,
                CURSOR_BLINK_INTERVAL,
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
            if idx < tabs.len() {
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
                        Instant::now(),
                        CELL_BLINK_INTERVAL,
                        CURSOR_BLINK_INTERVAL,
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

    let is_macos = cfg!(target_os = "macos");
    let copy_request = match &event.logical_key {
        Key::Named(NamedKey::Copy) => true,
        Key::Named(NamedKey::Insert) if modifiers.control_key() => true,
        Key::Character(text)
            if text.eq_ignore_ascii_case("c")
                && !modifiers.alt_key()
                && ((modifiers.control_key() && modifiers.shift_key())
                    || (is_macos && modifiers.super_key())) =>
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
            maybe_set_copy_notice(settings, copy_notice, "Selection copied", Instant::now());
        }
        return;
    }

    let paste_request = match &event.logical_key {
        Key::Named(NamedKey::Paste) => true,
        Key::Named(NamedKey::Insert) if modifiers.shift_key() => true,
        Key::Character(text)
            if text.eq_ignore_ascii_case("v")
                && !modifiers.alt_key()
                && ((modifiers.control_key() && modifiers.shift_key())
                    || (is_macos && modifiers.super_key())) =>
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
            if let Some((col, row)) = history_menu.selected_cell()
                && let Some(col_data) = history_menu.columns.get(col)
                && let Some(entry) = col_data.entries.get(row).cloned()
            {
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
                if command_text.starts_with("ssh ") || command_text.starts_with("sudo ssh") {
                    let _ = add_ssh_host_to_config(&command_text);
                }
            }
        }
        _ => {}
    }

    None
}

fn apply_history_selection(
    history_menu: &mut HistoryMenu,
    tabs: &mut [Tab],
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

fn handle_log_viewer_key(
    event: &KeyEvent,
    log_viewer: &mut LogViewer,
    log_refresh_needed: &mut bool,
) -> bool {
    if event.state != ElementState::Pressed {
        return false;
    }

    match &event.logical_key {
        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::F5) => {
            log_viewer.close();
            return true;
        }
        _ => {}
    }

    if log_viewer.editing {
        match &event.logical_key {
            Key::Named(NamedKey::Enter) => {
                log_viewer.query = log_viewer.input.trim().to_string();
                log_viewer.editing = false;
                *log_refresh_needed = true;
            }
            Key::Named(NamedKey::Backspace) => {
                log_viewer.input.pop();
            }
            Key::Character(text) => {
                log_viewer.input.push_str(text);
            }
            _ => {}
        }
        return true;
    }

    match &event.logical_key {
        Key::Character(text) if text == "/" => {
            log_viewer.editing = true;
            log_viewer.input = log_viewer.query.clone();
        }
        Key::Character(text) if text.eq_ignore_ascii_case("f") => {
            log_viewer.follow = !log_viewer.follow;
            *log_refresh_needed = true;
        }
        Key::Character(text) if text.eq_ignore_ascii_case("s") => {
            log_viewer.sudo = !log_viewer.sudo;
            *log_refresh_needed = true;
        }
        Key::Character(text) if text.eq_ignore_ascii_case("r") => {
            *log_refresh_needed = true;
        }
        Key::Named(NamedKey::Tab) => {
            log_viewer.focus = match log_viewer.focus {
                LogFocus::Sources => LogFocus::Logs,
                LogFocus::Logs => LogFocus::Sources,
            };
        }
        Key::Named(NamedKey::ArrowUp) => {
            if log_viewer.focus == LogFocus::Sources {
                if log_viewer.selected > 0 {
                    log_viewer.selected -= 1;
                    *log_refresh_needed = true;
                }
            } else {
                log_viewer.follow = false;
                log_viewer.scroll = log_viewer.scroll.saturating_sub(1);
            }
        }
        Key::Named(NamedKey::ArrowDown) => {
            if log_viewer.focus == LogFocus::Sources {
                if log_viewer.selected + 1 < log_viewer.sources.len() {
                    log_viewer.selected += 1;
                    *log_refresh_needed = true;
                }
            } else {
                log_viewer.follow = false;
                log_viewer.scroll = log_viewer.scroll.saturating_add(1);
            }
        }
        Key::Named(NamedKey::PageUp) => {
            log_viewer.follow = false;
            log_viewer.scroll = log_viewer.scroll.saturating_sub(10);
        }
        Key::Named(NamedKey::PageDown) => {
            log_viewer.follow = false;
            log_viewer.scroll = log_viewer.scroll.saturating_add(10);
        }
        Key::Named(NamedKey::Home) => {
            log_viewer.follow = false;
            log_viewer.scroll = 0;
        }
        Key::Named(NamedKey::End) => {
            log_viewer.follow = true;
        }
        _ => {}
    }
    true
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
        if ok && let Ok(bytes) = fs::read(&target) {
            let _ = proxy.send_event(AppEvent::Fonts(vec![Arc::new(bytes)]));
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
        let _ = proxy.send_event(AppEvent::FontLoadDone);
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

#[cfg(test)]
mod layout_tests {
    use super::*;
    use crate::tab::ShellTab;
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
        let now = Instant::now();

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
            pending_selection: None,
            mouse_down_pos: None,
            link_hover: None,
            hover_link: None,
            hover_link_range: None,
            link_ranges: Vec::new(),
            pending_cursor_to_line_end: false,
            last_cwd: None,
            cell_blink_on: true,
            cursor_blink_on: true,
            next_cell_blink: now,
            next_cursor_blink: now,
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
        let now = Instant::now();

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
            pending_selection: None,
            mouse_down_pos: None,
            link_hover: None,
            hover_link: None,
            hover_link_range: None,
            link_ranges: Vec::new(),
            pending_cursor_to_line_end: false,
            last_cwd: None,
            cell_blink_on: true,
            cursor_blink_on: true,
            next_cell_blink: now,
            next_cursor_blink: now,
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
    fn resolve_existing_path_prefers_tab_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("note.txt");
        fs::write(&file, b"hello").unwrap();

        let mut tab = make_shell_tab(2, 2);
        tab.last_cwd = Some(dir.path().to_path_buf());

        let resolved = resolve_existing_path("./note.txt", &tab);
        assert_eq!(resolved.as_deref(), Some(file.as_path()));
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

#[allow(clippy::too_many_arguments)]
fn refresh_layout_for_size(
    new_size: PhysicalSize<u32>,
    cell_w: u32,
    cell_h: u32,
    tab_bar_height: u32,
    pixels: &mut Pixels,
    tabs: &mut [Tab],
    debug_overlay: &mut DebugOverlay,
    gpu_renderer: &mut Option<GpuRenderer>,
    layout: &mut LayoutMetrics,
    frame_width: &mut u32,
    frame_height: &mut u32,
) -> bool {
    if new_size.width == 0 || new_size.height == 0 {
        return false;
    }
    let next_layout = compute_layout(
        new_size.width,
        new_size.height,
        cell_w,
        cell_h,
        tab_bar_height,
        BORDER_THICKNESS,
        BORDER_INSET,
        PADDING_X,
        PADDING_Y,
    );
    let size_changed = new_size.width != *frame_width || new_size.height != *frame_height;
    let layout_changed = next_layout.content_x != layout.content_x
        || next_layout.content_y != layout.content_y
        || next_layout.usable_width != layout.usable_width
        || next_layout.usable_height != layout.usable_height
        || next_layout.cols != layout.cols
        || next_layout.rows != layout.rows;

    if size_changed {
        pixels.resize_buffer(new_size.width, new_size.height).ok();
        pixels.resize_surface(new_size.width, new_size.height).ok();
        *frame_width = new_size.width;
        *frame_height = new_size.height;
    }

    if size_changed || layout_changed {
        *layout = next_layout;
        resize_tabs_to_layout(tabs, layout);
        debug_overlay.resize(layout.cols, layout.rows);
        if let Some(renderer) = gpu_renderer.as_mut() {
            if size_changed {
                renderer.resize(*frame_width, *frame_height, pixels.queue());
            }
            renderer.invalidate_base();
        }
        return true;
    }
    false
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
    #[cfg(test)]
    if let Some(path) = HELP_STATE_OVERRIDE.get() {
        return Some(path.clone());
    }
    if let Some(dir) = env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(dir).join("term").join("help.json"));
    }
    if let Some(dir) = config_dir() {
        return Some(dir.join("term").join("help.json"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config").join("term").join("help.json"))
}

#[cfg(test)]
static HELP_STATE_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

fn settings_state_path() -> Option<PathBuf> {
    if let Some(dir) = env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(dir).join("term").join("settings.json"));
    }
    if let Some(dir) = config_dir() {
        return Some(dir.join("term").join("settings.json"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config").join("term").join("settings.json"))
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct SettingsState {
    scrollback_enabled: bool,
    show_fps: bool,
    show_copy_notice: bool,
    render_mode: String,
    log_level: String,
}

impl SettingsState {
    fn from_panel(settings: &SettingsPanel) -> Self {
        Self {
            scrollback_enabled: settings.scrollback_enabled,
            show_fps: settings.show_fps,
            show_copy_notice: settings.show_copy_notice,
            render_mode: settings.render_mode.to_string(),
            log_level: settings.log_level.clone(),
        }
    }
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            scrollback_enabled: true,
            show_fps: false,
            show_copy_notice: false,
            render_mode: "auto".to_string(),
            log_level: "debug".to_string(),
        }
    }
}

fn load_settings_state() -> Option<SettingsState> {
    let path = settings_state_path()?;
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice::<SettingsState>(&bytes).ok()
}

fn persist_settings_state(state: SettingsState) {
    if let Some(path) = settings_state_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&state) {
            let _ = fs::write(&path, bytes);
        }
    }
}

fn parse_render_mode(value: &str) -> Option<DebugRenderMode> {
    match value.to_ascii_lowercase().as_str() {
        "auto" => Some(DebugRenderMode::Auto),
        "cpu" => Some(DebugRenderMode::CpuOnly),
        "gpu" => Some(DebugRenderMode::GpuOnly),
        _ => None,
    }
}

fn next_render_mode(current: DebugRenderMode) -> DebugRenderMode {
    match current {
        DebugRenderMode::Auto => DebugRenderMode::GpuOnly,
        DebugRenderMode::GpuOnly => DebugRenderMode::CpuOnly,
        DebugRenderMode::CpuOnly => DebugRenderMode::Auto,
    }
}

fn parse_log_level(value: &str) -> Option<log::LevelFilter> {
    match value.to_ascii_lowercase().as_str() {
        "error" => Some(log::LevelFilter::Error),
        "warn" | "warning" => Some(log::LevelFilter::Warn),
        "info" => Some(log::LevelFilter::Info),
        "debug" => Some(log::LevelFilter::Debug),
        "trace" => Some(log::LevelFilter::Trace),
        _ => None,
    }
}

fn next_log_level(current: &str) -> &'static str {
    match current.to_ascii_lowercase().as_str() {
        "error" => "warn",
        "warn" | "warning" => "info",
        "info" => "debug",
        "debug" => "trace",
        "trace" => "error",
        _ => "debug",
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
    thread::spawn(move || copy_text_to_clipboard_sync(text));
}

fn copy_text_to_clipboard_sync(text: String) {
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
        // Close stdin so clipboard helpers don't block waiting for EOF.
        drop(child.stdin.take());
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

fn kitty_font_family() -> String {
    if let Some(config) = run_command_with_timeout(
        "kitty",
        &["@", "get-config"],
        None,
        Duration::from_millis(500),
    ) && let Some(family) = parse_kitty_font_family(&String::from_utf8_lossy(&config))
    {
        return family;
    }

    if let Some(path) = kitty_config_path()
        && let Ok(contents) = fs::read_to_string(path)
        && let Some(family) = parse_kitty_font_family(&contents)
    {
        return family;
    }

    "monospace".to_string()
}

fn kitty_config_path() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|base| base.join("kitty").join("kitty.conf"))
}

fn parse_kitty_font_family(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if !line.starts_with("font_family") {
            continue;
        }
        let rest = line.trim_start_matches("font_family").trim();
        if rest.is_empty() {
            continue;
        }
        return Some(rest.trim_matches('"').trim_matches('\'').to_string());
    }
    None
}

fn load_kitty_primary_font() -> Option<Arc<Vec<u8>>> {
    let family = kitty_font_family();
    let output = run_command_with_timeout(
        "fc-match",
        &["-f", "%{file}", &family],
        None,
        Duration::from_millis(500),
    )?;
    let path = String::from_utf8_lossy(&output).trim().to_string();
    if path.is_empty() {
        return None;
    }
    fs::read(path).ok().map(Arc::new)
}

fn spawn_font_loader(proxy: EventLoopProxy<AppEvent>) {
    thread::spawn(move || {
        let fonts = GlyphCache::load_fallback_fonts();
        if !fonts.is_empty() {
            let _ = proxy.send_event(AppEvent::Fonts(fonts));
        }
    });
}

#[cfg(test)]
mod ui_tests {
    use super::*;

    fn terminal_from_line(text: &str) -> Terminal {
        let cols = text.chars().count().max(1);
        let mut term = Terminal::new(cols, 1);
        for (idx, ch) in text.chars().enumerate() {
            if let Some(cell) = term.cells.get_mut(idx) {
                cell.set_text(ch.to_string());
            }
        }
        term
    }

    fn col_for(text: &str, needle: &str) -> usize {
        let byte_idx = text.find(needle).expect("needle present in text");
        text[..byte_idx].chars().count()
    }

    #[test]
    fn help_toggle_stays_visible_until_toggled() {
        let now = Instant::now();
        let mut help = HelpToggle { visible: true };
        assert!(help.should_show(now));
        let later = now.checked_add(Duration::from_secs(60)).unwrap();
        assert!(help.should_show(later), "should not auto-hide");
        help.toggle(later);
        assert!(!help.should_show(later), "toggling hides the bar");
        help.toggle(later);
        assert!(help.should_show(later), "toggling again shows it");
    }

    #[test]
    fn help_visibility_persists_to_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = HELP_STATE_OVERRIDE
            .get_or_init(|| dir.path().join("term").join("help.json"))
            .clone();
        let _ = fs::remove_file(&path);

        persist_help_visible_flag(false);
        assert_eq!(load_help_visible_flag(), Some(false));

        persist_help_visible_flag(true);
        assert_eq!(load_help_visible_flag(), Some(true));
    }

    #[test]
    fn bracketed_paste_wraps_payload() {
        let data = b"echo hi";
        let wrapped = compose_bracketed_paste(data, true);
        assert_eq!(wrapped, b"\x1b[200~echo hi\x1b[201~");
    }

    #[test]
    fn non_bracketed_paste_is_passthrough() {
        let data = b"abc123";
        let wrapped = compose_bracketed_paste(data, false);
        assert_eq!(wrapped, data);
    }

    #[test]
    fn detects_http_url_under_cursor() {
        let text = "curl https://example.com/test,";
        let term = terminal_from_line(text);
        let col = col_for(text, "https");
        let found = detect_link_text_at_cell(&term, col, 0).expect("link match");
        assert_eq!(found.text, "https://example.com/test");
        assert!(found.start <= col && found.end >= col);
    }

    #[test]
    fn detects_file_path_and_trims_punctuation() {
        let text = "open (/tmp/foo.txt)";
        let term = terminal_from_line(text);
        let col = col_for(text, "/tmp");
        let found = detect_link_text_at_cell(&term, col, 0).expect("link match");
        assert_eq!(found.text, "/tmp/foo.txt");
    }

    #[test]
    fn ignores_non_links() {
        let text = "hello world";
        let term = terminal_from_line(text);
        let col = col_for(text, "world");
        let found = detect_link_text_at_cell(&term, col, 0);
        assert!(found.is_none());
    }
}
