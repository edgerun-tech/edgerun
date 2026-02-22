// SPDX-License-Identifier: Apache-2.0
use std::env;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use crate::AppEvent;
use log::info;
use portable_pty::{MasterPty, NativePtySystem, PtySize, PtySystem};
use term_core::terminal::Terminal;
use vte::Parser as VteParser;

pub struct ShellTab {
    pub parser: VteParser,
    pub rx: mpsc::Receiver<Vec<u8>>,
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    pub app_cursor_keys: bool,
    pub exited: bool,
    pub child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
}

pub enum TabKind {
    Shell(ShellTab),
}

pub struct Tab {
    pub terminal: Terminal,
    pub kind: TabKind,
    pub title: String,
    pub selection_anchor: Option<(usize, usize)>,
    pub selection_edge: Option<(usize, usize)>,
    pub selecting: bool,
    pub pending_selection: Option<(usize, usize)>,
    pub mouse_down_pos: Option<(f64, f64)>,
    pub link_hover: Option<(usize, usize)>,
    pub hover_link: Option<String>,
    pub hover_link_range: Option<(usize, usize, usize)>,
    pub link_ranges: Vec<Vec<(usize, usize)>>,
    pub pending_cursor_to_line_end: bool,
    pub last_cwd: Option<PathBuf>,
    pub cell_blink_on: bool,
    pub cursor_blink_on: bool,
    pub next_cell_blink: Instant,
    pub next_cursor_blink: Instant,
}

#[derive(Clone, Debug)]
pub struct CwdHistoryEntry {
    pub tab: usize,
    pub path: PathBuf,
}

pub fn selection_bounds(tab: &Tab) -> Option<((usize, usize), (usize, usize))> {
    tab.selection_anchor.zip(tab.selection_edge)
}

pub fn refresh_cwd_for_tabs(tabs: &mut [Tab], log: &mut Vec<CwdHistoryEntry>) {
    #[cfg(unix)]
    {
        for (idx, tab) in tabs.iter_mut().enumerate() {
            if let Some(path) = tab_proc_cwd(tab)
                && tab.last_cwd.as_ref() != Some(&path)
            {
                tab.last_cwd = Some(path.clone());
                log.push(CwdHistoryEntry { tab: idx, path });
                if log.len() > 256 {
                    let drop = log.len().saturating_sub(256);
                    log.drain(0..drop);
                }
            }
        }
    }
}

pub fn tab_proc_cwd(tab: &Tab) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        let TabKind::Shell(shell) = &tab.kind;
        let pid = shell.child.as_ref()?.process_id()?;
        let path = PathBuf::from(format!("/proc/{pid}/cwd"));
        std::fs::read_link(path).ok()
    }
    #[cfg(not(unix))]
    {
        let _ = tab;
        None
    }
}

pub fn preferred_cwd(tabs: &[Tab], history: &[CwdHistoryEntry], active: usize) -> Option<PathBuf> {
    tabs.get(active)
        .and_then(|t| t.last_cwd.clone())
        .or_else(|| {
            history
                .iter()
                .rev()
                .find(|entry| entry.tab == active)
                .map(|entry| entry.path.clone())
        })
        .or_else(|| tabs.get(active).and_then(tab_proc_cwd))
}

pub fn tab_current_dir(tab: &Tab) -> Option<PathBuf> {
    tab.last_cwd.clone().or_else(|| tab_proc_cwd(tab))
}

pub fn refresh_tab_titles(tabs: &mut [Tab]) {
    for (idx, tab) in tabs.iter_mut().enumerate() {
        if tab.terminal.window_title.is_none() {
            tab.title = format!("Tab {}", idx + 1);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_shell_tab(
    cols: u16,
    rows: u16,
    usable_width: u32,
    usable_height: u32,
    proxy: winit::event_loop::EventLoopProxy<AppEvent>,
    cwd: Option<PathBuf>,
    scrollback_enabled: bool,
    now: Instant,
    cell_blink_interval: Duration,
    cursor_blink_interval: Duration,
) -> anyhow::Result<Tab> {
    let pty_system = NativePtySystem::default();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: usable_width.min(u16::MAX as u32) as u16,
        pixel_height: usable_height.min(u16::MAX as u32) as u16,
    })?;

    let mut cmd = crate::platform::default_shell();
    cmd.env("TERM", "xterm-256color");
    cmd.env("PROMPT_EOL_MARK", "");
    if let Some(ref dir) = cwd {
        cmd.cwd(dir);
    }
    let child = pair.slave.spawn_command(cmd)?;

    let master: Arc<Mutex<Box<dyn MasterPty + Send>>> = Arc::new(Mutex::new(pair.master));
    let mut reader = master.lock().unwrap().try_clone_reader()?;
    let writer = Arc::new(Mutex::new(master.lock().unwrap().take_writer()?));

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let log_pty = env::var("TERM_DEBUG_PTY")
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false);
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if log_pty {
                        let hex = buf[..n]
                            .iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let printable = String::from_utf8_lossy(&buf[..n])
                            .escape_default()
                            .to_string();
                        info!("debug pty read: [{}] \"{}\"", hex, printable);
                    }
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                    let _ = proxy.send_event(AppEvent::Wake);
                }
                Err(_) => break,
            }
        }
    });

    Ok(Tab {
        terminal: {
            let mut t = Terminal::new(cols as usize, rows as usize);
            t.set_scrollback_enabled(scrollback_enabled);
            t
        },
        kind: TabKind::Shell(ShellTab {
            parser: VteParser::new(),
            rx,
            writer,
            master,
            app_cursor_keys: false,
            exited: false,
            child: Some(child),
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
        last_cwd: cwd.clone(),
        cell_blink_on: true,
        cursor_blink_on: true,
        next_cell_blink: now + cell_blink_interval,
        next_cursor_blink: now + cursor_blink_interval,
    })
}

pub fn resize_tabs_to_layout(tabs: &mut [Tab], layout: &term_core::render::layout::LayoutMetrics) {
    for tab in tabs {
        tab.terminal.resize(layout.cols, layout.rows);
        let TabKind::Shell(shell) = &tab.kind;
        if let Ok(master) = shell.master.lock() {
            let _ = master.resize(PtySize {
                rows: layout.rows as u16,
                cols: layout.cols as u16,
                pixel_width: layout.usable_width.min(u16::MAX as u32) as u16,
                pixel_height: layout.usable_height.min(u16::MAX as u32) as u16,
            });
        }
    }
}
