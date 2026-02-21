use thiserror::Error;

#[derive(Debug, Error)]
pub enum TermCoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("Sixel decode error: {0}")]
    Sixel(#[from] icy_sixel::SixelError),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Clipboard error: {0}")]
    Arboard(#[from] arboard::Error),
    #[cfg(target_arch = "wasm32")]
    #[error("Clipboard is not available on wasm32")]
    ClipboardUnavailable,
    #[error("Other error: {0}")]
    Other(String),
}
use std::io::Write;
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(not(target_arch = "wasm32"))]
use arboard::Clipboard;
use base64::Engine;
use base64::engine::general_purpose;
use icy_sixel::decoder::{DcsSettings, sixel_decode_from_dcs};
use once_cell::sync::Lazy;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use vte::{Params, Perform};

pub const TAB_WIDTH: usize = 4;
pub const FG: [u8; 4] = [221, 221, 221, 255];
pub const CURSOR: [u8; 4] = [255, 255, 255, 128];
pub const SELECTION: [u8; 4] = [180, 200, 255, 96];
pub const DEFAULT_SCROLLBACK_LIMIT: usize = 10_000;
pub const ANSI_BASE: [[u8; 3]; 8] = [
    // Make “black” (ANSI 30/40) clearly visible on dark backgrounds.
    [0, 0, 0],
    [204, 4, 3],
    [25, 203, 0],
    [206, 203, 0],
    [13, 115, 204],
    [203, 30, 209],
    [13, 205, 205],
    [221, 221, 221],
];
pub const ANSI_BRIGHT: [[u8; 3]; 8] = [
    [118, 118, 118],
    [242, 32, 31],
    [35, 253, 0],
    [255, 253, 0],
    [26, 143, 255],
    [253, 40, 255],
    [20, 255, 255],
    [255, 255, 255],
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub const DEFAULT_BG: Rgba = Rgba {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

pub const DEFAULT_FG: Rgba = Rgba {
    r: FG[0],
    g: FG[1],
    b: FG[2],
    a: FG[3],
};

#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub text: String,
    pub wide: bool,
    pub wide_continuation: bool,
    pub fg: Rgba,
    pub bg: Rgba,
    pub bold: bool,
    pub faint: bool,
    pub blink: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub overline: bool,
    pub concealed: bool,
    pub hyperlink: Option<String>,
}

impl Cell {
    pub fn blank() -> Self {
        Self::blank_with(DEFAULT_FG, DEFAULT_BG)
    }

    pub fn blank_with(fg: Rgba, bg: Rgba) -> Self {
        Self {
            text: " ".to_string(),
            wide: false,
            wide_continuation: false,
            fg,
            bg,
            bold: false,
            faint: false,
            blink: false,
            italic: false,
            underline: false,
            strike: false,
            overline: false,
            concealed: false,
            hyperlink: None,
        }
    }

    pub fn with_style(fg: Rgba, bg: Rgba) -> Self {
        Self {
            text: " ".to_string(),
            wide: false,
            wide_continuation: false,
            fg,
            bg,
            bold: false,
            faint: false,
            blink: false,
            italic: false,
            underline: false,
            strike: false,
            overline: false,
            concealed: false,
            hyperlink: None,
        }
    }

    pub fn is_blank(&self) -> bool {
        !self.wide_continuation && self.text == " "
    }

    pub fn set_text<S: Into<String>>(&mut self, text: S) {
        self.text = text.into();
    }

    pub fn push_char(&mut self, ch: char) {
        self.text.push(ch);
    }
}

pub struct Terminal {
    pub cols: usize,
    pub rows: usize,
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub saved_cursor: Option<SavedCursor>,
    pub cells: Vec<Cell>,
    pub pen_fg: Rgba,
    pub pen_bg: Rgba,
    pub pen_inverse: bool,
    pub pen_bold: bool,
    pub pen_faint: bool,
    pub pen_blink: bool,
    pub pen_italic: bool,
    pub pen_underline: bool,
    pub pen_strike: bool,
    pub pen_overline: bool,
    pub pen_conceal: bool,
    pub alt_state: Option<AltState>,
    pub alt_active: bool,
    pub top_margin: usize,
    pub bottom_margin: usize,
    pub origin_mode: bool,
    pub left_margin: usize,
    pub right_margin: usize,
    pub scrollback: Vec<Vec<Cell>>,
    pub scrollback_limit: usize,
    pub view_offset: usize,
    pub bracketed_paste: bool,
    pub focus_reporting: bool,
    pub wrap_next: bool,
    pub mouse_btn_report: bool,
    pub mouse_motion_report: bool,
    pub mouse_sgr: bool,
    pub mouse_utf8: bool,
    pub mouse_pressed: Option<u8>,
    pub prompt_mark: Option<char>,
    pub prompt_status: Option<i32>,
    pub help_text: Option<String>,
    pub status_text: Option<String>,
    pub cwd_state: Option<String>,
    pub hyperlink: Option<String>,
    pub kitty_keyboard: bool,
    pub ghost_text: Option<String>,
    pub window_title: Option<String>,
    pub icon_title: Option<String>,
    pub cursor_visible: bool,
    pub cursor_shape: CursorShape,
    pub sixels: Vec<SixelSprite>,
    default_bg: Rgba,
    default_fg: Rgba,
    cursor_color: Rgba,
    palette: [Rgba; 256],
    blank_cell: Cell,
    dirty_rows: Vec<u64>,
    dirty_epoch: u64,
}

#[allow(dead_code)]
static CLIPBOARD_HOOK: Lazy<Mutex<Option<Arc<dyn Fn(&str) + Send + Sync>>>> =
    Lazy::new(|| Mutex::new(None));
static DEFAULT_PALETTE: Lazy<[Rgba; 256]> = Lazy::new(default_palette);

pub struct AltState {
    pub cells: Vec<Cell>,
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub saved_cursor: Option<SavedCursor>,
    pub left_margin: usize,
    pub right_margin: usize,
    pub pen_fg: Rgba,
    pub pen_bg: Rgba,
    pub pen_inverse: bool,
    pub pen_bold: bool,
    pub pen_faint: bool,
    pub pen_blink: bool,
    pub pen_italic: bool,
    pub pen_underline: bool,
    pub pen_strike: bool,
    pub pen_overline: bool,
    pub pen_conceal: bool,
    pub wrap_next: bool,
    pub mouse_pressed: Option<u8>,
    pub default_bg: Rgba,
    pub default_fg: Rgba,
    pub cursor_color: Rgba,
    pub cursor_visible: bool,
    pub cursor_shape: CursorShape,
    pub palette: [Rgba; 256],
    pub sixels: Vec<SixelSprite>,
}

#[derive(Clone)]
pub struct SixelSprite {
    pub col: usize,
    pub row: usize,
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>, // RGBA
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorShape {
    Block,
    Underline,
    Bar,
}

pub struct GridPerformer<'a> {
    pub grid: &'a mut Terminal,
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub app_cursor_keys: &'a mut bool,
    pub dcs_state: Option<DcsState>,
}

#[derive(Clone, Debug)]
pub struct DcsState {
    pub data: Vec<u8>,
    pub aspect_ratio: Option<u16>,
    pub zero_color: Option<u16>,
    pub grid_size: Option<u16>,
    pub col: usize,
    pub row: usize,
}

#[derive(Clone, Copy)]
pub enum MouseEventKind {
    Press(u8), // 0=left,1=middle,2=right
    Release,
    Motion,
    WheelUp,
    WheelDown,
}

#[derive(Clone, Copy)]
pub struct SavedCursor {
    pub col: usize,
    pub row: usize,
    pub fg: Rgba,
    pub bg: Rgba,
    pub inverse: bool,
    pub bold: bool,
    pub faint: bool,
    pub blink: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub overline: bool,
    pub conceal: bool,
    pub origin_mode: bool,
    pub wrap_next: bool,
}

pub fn ansi_color(idx: usize, bright: bool) -> Rgba {
    let palette = if bright { ANSI_BRIGHT } else { ANSI_BASE };
    let i = idx.min(7);
    let c = palette[i];
    Rgba {
        r: c[0],
        g: c[1],
        b: c[2],
        a: 255,
    }
}

pub fn brightened(c: Rgba) -> Rgba {
    Rgba {
        r: (c.r as u16 * 11 / 10).min(255) as u8,
        g: (c.g as u16 * 11 / 10).min(255) as u8,
        b: (c.b as u16 * 11 / 10).min(255) as u8,
        a: c.a,
    }
}

pub fn faintened(c: Rgba) -> Rgba {
    Rgba {
        r: (c.r as u16 * 2 / 3) as u8,
        g: (c.g as u16 * 2 / 3) as u8,
        b: (c.b as u16 * 2 / 3) as u8,
        a: c.a,
    }
}

fn default_palette() -> [Rgba; 256] {
    let mut p = [Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    }; 256];
    for i in 0..8 {
        let c = ANSI_BASE[i];
        p[i] = Rgba {
            r: c[0],
            g: c[1],
            b: c[2],
            a: 255,
        };
    }
    for i in 0..8 {
        let c = ANSI_BRIGHT[i];
        p[8 + i] = Rgba {
            r: c[0],
            g: c[1],
            b: c[2],
            a: 255,
        };
    }
    for idx in 16..232 {
        let i = idx - 16;
        let r = (i / 36) % 6;
        let g = (i / 6) % 6;
        let b = i % 6;
        let to_u8 = |v: u16| if v == 0 { 0 } else { 55 + v * 40 } as u8;
        p[idx] = Rgba {
            r: to_u8(r as u16),
            g: to_u8(g as u16),
            b: to_u8(b as u16),
            a: 255,
        };
    }
    for idx in 232..256 {
        let shade = 8u16.saturating_add((idx as u16 - 232) * 10).min(255) as u8;
        p[idx] = Rgba {
            r: shade,
            g: shade,
            b: shade,
            a: 255,
        };
    }
    p
}

fn parse_hex_component(component: &str) -> Option<u8> {
    match component.len() {
        1 => u8::from_str_radix(&format!("{component}{component}"), 16).ok(),
        2 => u8::from_str_radix(component, 16).ok(),
        _ => None,
    }
}

fn parse_osc_color(spec: &str) -> Option<Rgba> {
    let trimmed = spec.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        let hex = hex.as_bytes();
        let (r, g, b, a) = match hex.len() {
            3 => (
                parse_hex_component(std::str::from_utf8(&hex[0..1]).ok()?)?,
                parse_hex_component(std::str::from_utf8(&hex[1..2]).ok()?)?,
                parse_hex_component(std::str::from_utf8(&hex[2..3]).ok()?)?,
                255,
            ),
            4 => (
                parse_hex_component(std::str::from_utf8(&hex[0..1]).ok()?)?,
                parse_hex_component(std::str::from_utf8(&hex[1..2]).ok()?)?,
                parse_hex_component(std::str::from_utf8(&hex[2..3]).ok()?)?,
                parse_hex_component(std::str::from_utf8(&hex[3..4]).ok()?)?,
            ),
            6 => (
                u8::from_str_radix(std::str::from_utf8(&hex[0..2]).ok()?, 16).ok()?,
                u8::from_str_radix(std::str::from_utf8(&hex[2..4]).ok()?, 16).ok()?,
                u8::from_str_radix(std::str::from_utf8(&hex[4..6]).ok()?, 16).ok()?,
                255,
            ),
            8 => (
                u8::from_str_radix(std::str::from_utf8(&hex[0..2]).ok()?, 16).ok()?,
                u8::from_str_radix(std::str::from_utf8(&hex[2..4]).ok()?, 16).ok()?,
                u8::from_str_radix(std::str::from_utf8(&hex[4..6]).ok()?, 16).ok()?,
                u8::from_str_radix(std::str::from_utf8(&hex[6..8]).ok()?, 16).ok()?,
            ),
            _ => return None,
        };
        return Some(Rgba { r, g, b, a });
    }
    if let Some(body) = trimmed.strip_prefix("rgb:") {
        let parts: Vec<&str> = body.split('/').collect();
        if parts.len() >= 3 {
            let r = parse_hex_component(parts[0])?;
            let g = parse_hex_component(parts[1])?;
            let b = parse_hex_component(parts[2])?;
            return Some(Rgba { r, g, b, a: 255 });
        }
    }
    None
}

/// Ensure foreground remains readable against a background by bumping contrast when too close.
pub fn ensure_contrast(fg: Rgba, bg: Rgba) -> Rgba {
    let lum = |c: Rgba| -> f32 { 0.2126 * c.r as f32 + 0.7152 * c.g as f32 + 0.0722 * c.b as f32 };
    let diff = (lum(fg) - lum(bg)).abs();
    if diff < 25.0 {
        // Push toward default foreground for low-contrast pairs.
        DEFAULT_FG
    } else {
        fg
    }
}

pub fn xterm_color(code: u16) -> Rgba {
    DEFAULT_PALETTE
        .get(code.min(255) as usize)
        .copied()
        .unwrap_or(DEFAULT_FG)
}

pub fn write_bytes(writer: &Arc<Mutex<Box<dyn Write + Send>>>, bytes: &[u8]) {
    if let Ok(mut guard) = writer.lock() {
        if std::env::var("TERM_DEBUG_PTY")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false)
        {
            log::info!(
                "debug pty write: {}",
                bytes
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
        let _ = guard.write_all(bytes);
        let _ = guard.flush();
    }
}

fn debug_bg_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("TERM_DEBUG_BG")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false)
    })
}

fn debug_width_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("TERM_DEBUG_WIDTH")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false)
    })
}

fn dcs_bytes_from_payload(data: &[u8]) -> Option<&[u8]> {
    if data.is_empty() {
        return None;
    }
    Some(data)
}

pub fn copy_text_to_clipboard(text: &str) -> Result<(), TermCoreError> {
    write_clipboard_text(text)
}

#[cfg(not(target_arch = "wasm32"))]
fn write_clipboard_text(text: &str) -> Result<(), TermCoreError> {
    let cb = CLIPBOARD_HOOK
        .lock()
        .map_err(|e| TermCoreError::Other(format!("Clipboard hook lock poisoned: {e}")))?
        .clone();
    if let Some(cb) = cb {
        cb(text);
        return Ok(());
    }
    let mut clipboard = Clipboard::new().map_err(TermCoreError::Arboard)?;
    clipboard
        .set_text(text.to_string())
        .map_err(TermCoreError::Arboard)
}

#[cfg(target_arch = "wasm32")]
fn write_clipboard_text(_text: &str) -> Result<(), TermCoreError> {
    // Clipboard support is handled in JS for the web build.
    Ok(())
}

#[cfg(test)]
pub fn set_clipboard_hook<F>(hook: F)
where
    F: Fn(&str) + Send + Sync + 'static,
{
    let mut slot = CLIPBOARD_HOOK
        .lock()
        .map_err(|p| p.into_inner())
        .unwrap_or_else(|guard| guard);
    *slot = Some(Arc::new(hook));
}

pub fn resize_cells(
    buf: &mut Vec<Cell>,
    cols: usize,
    rows: usize,
    default_fg: Rgba,
    default_bg: Rgba,
) {
    let new_len = cols.saturating_mul(rows).max(1);
    if buf.len() > new_len {
        buf.truncate(new_len);
    } else if buf.len() < new_len {
        buf.resize(new_len, Cell::blank_with(default_fg, default_bg));
    }
}

impl Terminal {
    pub fn new(cols: usize, rows: usize) -> Self {
        let default_bg = DEFAULT_BG;
        let default_fg = DEFAULT_FG;
        let cursor_color = Rgba {
            r: CURSOR[0],
            g: CURSOR[1],
            b: CURSOR[2],
            a: CURSOR[3],
        };
        let cells =
            vec![Cell::blank_with(default_fg, default_bg); cols.saturating_mul(rows).max(1)];
        let palette = *DEFAULT_PALETTE;
        let blank_cell = Cell::blank_with(default_fg, default_bg);
        Self {
            cols,
            rows,
            cursor_col: 0,
            cursor_row: 0,
            saved_cursor: None,
            cells,
            pen_fg: default_fg,
            pen_bg: default_bg,
            pen_inverse: false,
            pen_bold: false,
            pen_faint: false,
            pen_blink: false,
            pen_italic: false,
            pen_underline: false,
            pen_strike: false,
            pen_overline: false,
            pen_conceal: false,
            alt_state: None,
            alt_active: false,
            top_margin: 0,
            bottom_margin: rows.saturating_sub(1),
            left_margin: 0,
            right_margin: cols.saturating_sub(1),
            origin_mode: false,
            scrollback: Vec::new(),
            scrollback_limit: DEFAULT_SCROLLBACK_LIMIT,
            view_offset: 0,
            bracketed_paste: false,
            focus_reporting: false,
            wrap_next: false,
            mouse_btn_report: false,
            mouse_motion_report: false,
            mouse_sgr: false,
            mouse_utf8: false,
            mouse_pressed: None,
            prompt_mark: None,
            prompt_status: None,
            help_text: None,
            status_text: None,
            cwd_state: None,
            hyperlink: None,
            kitty_keyboard: false,
            ghost_text: None,
            window_title: None,
            icon_title: None,
            cursor_visible: true,
            cursor_shape: CursorShape::Block,
            sixels: Vec::new(),
            default_bg,
            default_fg,
            cursor_color,
            palette,
            blank_cell,
            dirty_rows: vec![0u64; rows.max(1)],
            dirty_epoch: 0,
        }
    }

    pub fn set_ghost_text<S: Into<String>>(&mut self, text: Option<S>) {
        self.ghost_text = text.map(Into::into);
    }

    pub fn set_window_title<S: Into<String>>(&mut self, title: Option<S>) {
        self.window_title = title.map(Into::into);
    }

    pub fn window_title(&self) -> Option<&str> {
        self.window_title.as_deref()
    }

    pub fn set_icon_title<S: Into<String>>(&mut self, title: Option<S>) {
        self.icon_title = title.map(Into::into);
    }

    pub fn icon_title(&self) -> Option<&str> {
        self.icon_title.as_deref()
    }

    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    pub fn set_cursor_shape(&mut self, shape: CursorShape) {
        self.cursor_shape = shape;
    }

    pub fn cursor_shape(&self) -> CursorShape {
        self.cursor_shape
    }

    pub fn in_alt_screen(&self) -> bool {
        self.alt_active
    }

    pub fn erase_cell(&self) -> Cell {
        let (fg, bg) = self.effective_colors();
        Cell::with_style(fg, bg)
    }

    pub fn cell(&self, col: usize, row: usize) -> Cell {
        self.cells
            .get(row.saturating_mul(self.cols).saturating_add(col))
            .cloned()
            .unwrap_or_else(|| Cell::blank_with(self.default_fg, self.default_bg))
    }

    pub fn set_cursor(&mut self, col: usize, row: usize) {
        let mut r = row.min(self.rows.saturating_sub(1));
        if self.origin_mode {
            r = (self.top_margin + row).min(self.bottom_margin);
        }
        let min_col = self.left_margin.min(self.cols.saturating_sub(1));
        let max_col = self.right_margin.min(self.cols.saturating_sub(1));
        self.cursor_col = col.clamp(min_col, max_col);
        self.cursor_row = r;
        self.wrap_next = false;
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let mut new_cells = vec![
            Cell::blank_with(self.default_fg, self.default_bg);
            cols.saturating_mul(rows).max(1)
        ];
        let min_rows = rows.min(self.rows);
        let min_cols = cols.min(self.cols);

        for r in 0..min_rows {
            for c in 0..min_cols {
                new_cells[r * cols + c] = self.cells[r * self.cols + c].clone();
            }
        }

        self.cols = cols;
        self.rows = rows;
        self.cells = new_cells;
        self.dirty_rows = vec![0u64; self.rows.max(1)];
        self.mark_all_dirty();
        let last_row = self.rows.saturating_sub(1);
        self.top_margin = self.top_margin.min(last_row);
        self.bottom_margin = self.bottom_margin.min(last_row);
        if self.top_margin > self.bottom_margin {
            self.top_margin = 0;
            self.bottom_margin = last_row;
        }
        let last_col = self.cols.saturating_sub(1);
        self.left_margin = self.left_margin.min(last_col);
        self.right_margin = self.right_margin.min(last_col);
        if self.left_margin > self.right_margin {
            self.left_margin = 0;
            self.right_margin = last_col;
        }
        self.cursor_col = self.cursor_col.min(self.right_margin).max(self.left_margin);
        self.cursor_row = self.cursor_row.min(self.rows.saturating_sub(1));
        if let Some(mut saved) = self.saved_cursor {
            saved.col = saved.col.min(self.cols.saturating_sub(1));
            saved.row = saved.row.min(self.rows.saturating_sub(1));
            self.saved_cursor = Some(saved);
        }
        if let Some(state) = &mut self.alt_state {
            resize_cells(
                &mut state.cells,
                self.cols,
                self.rows,
                self.default_fg,
                self.default_bg,
            );
            state.cursor_col = state.cursor_col.min(self.cols.saturating_sub(1));
            state.cursor_row = state.cursor_row.min(self.rows.saturating_sub(1));
            if let Some(mut saved) = state.saved_cursor {
                saved.col = saved.col.min(self.cols.saturating_sub(1));
                saved.row = saved.row.min(self.rows.saturating_sub(1));
                state.saved_cursor = Some(saved);
            }
        }
        self.top_margin = 0;
        self.bottom_margin = self.rows.saturating_sub(1);
        self.origin_mode = false;
        for line in &mut self.scrollback {
            if line.len() > self.cols {
                line.truncate(self.cols);
            } else if line.len() < self.cols {
                line.resize(
                    self.cols,
                    Cell::blank_with(self.default_fg, self.default_bg),
                );
            }
        }
        self.clamp_view_offset();
    }

    pub fn set_focus_reporting(&mut self, enabled: bool) {
        self.focus_reporting = enabled;
    }

    pub fn report_focus(&self, focused: bool, writer: &Arc<Mutex<Box<dyn Write + Send>>>) {
        if !self.focus_reporting {
            return;
        }
        let seq = if focused { b"\x1b[I" } else { b"\x1b[O" };
        write_bytes(writer, seq);
    }

    pub fn report_mouse_event(
        &mut self,
        col: usize,
        row: usize,
        kind: MouseEventKind,
        writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    ) {
        if !self.mouse_btn_report {
            return;
        }
        let mut code: u8;
        let mut motion = false;
        match kind {
            MouseEventKind::Press(btn) => {
                self.mouse_pressed = Some(btn);
                code = btn;
            }
            MouseEventKind::Release => {
                self.mouse_pressed = None;
                code = 3;
            }
            MouseEventKind::WheelUp => {
                code = 64;
            }
            MouseEventKind::WheelDown => {
                code = 65;
            }
            MouseEventKind::Motion => {
                if !self.mouse_motion_report {
                    return;
                }
                code = self.mouse_pressed.unwrap_or(3);
                motion = true;
            }
        }

        if motion {
            code |= 32;
        }

        let x = col.saturating_add(1) as u32;
        let y = row.saturating_add(1) as u32;

        if self.mouse_sgr {
            let suffix = match kind {
                MouseEventKind::Release => 'm',
                _ => 'M',
            };
            let seq = format!("\x1b[<{};{};{}{}", code, x, y, suffix);
            write_bytes(writer, seq.as_bytes());
        } else {
            let mut seq = vec![0x1b, b'[', b'M'];
            let push_val = |out: &mut Vec<u8>, val: u32| {
                let adj = val + 32;
                if self.mouse_utf8 {
                    let mut buf = [0u8; 4];
                    let len = char::from_u32(adj.min(0x10FFFF))
                        .unwrap_or('\0')
                        .encode_utf8(&mut buf)
                        .len();
                    out.extend_from_slice(&buf[..len]);
                } else {
                    out.push(adj.min(255) as u8);
                }
            };
            push_val(&mut seq, code as u32);
            push_val(&mut seq, x);
            push_val(&mut seq, y);
            write_bytes(writer, &seq);
        }
    }

    pub fn clear_all(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::with_style(self.pen_fg, self.pen_bg);
        }
        self.sixels.clear();
        self.cursor_col = 0;
        self.cursor_row = 0;
        self.saved_cursor = None;
        self.reset_style();
        self.set_view_offset(0);
        self.bracketed_paste = false;
        self.focus_reporting = false;
        self.cursor_visible = true;
        self.cursor_shape = CursorShape::Block;
        self.top_margin = 0;
        self.bottom_margin = self.rows.saturating_sub(1);
        self.origin_mode = false;
        self.wrap_next = false;
        self.mark_all_dirty();
    }

    pub fn clear_all_and_scrollback(&mut self) {
        let keep_alt = self.alt_active;
        if !keep_alt {
            self.alt_state = None;
        }
        self.scrollback.clear();
        self.set_view_offset(0);
        self.pen_fg = DEFAULT_FG;
        self.pen_bg = DEFAULT_BG;
        self.pen_inverse = false;
        self.pen_bold = false;
        self.pen_faint = false;
        self.pen_blink = false;
        self.pen_italic = false;
        self.pen_underline = false;
        self.wrap_next = false;
        self.focus_reporting = false;
        self.mouse_btn_report = false;
        self.mouse_motion_report = false;
        self.mouse_sgr = false;
        self.mouse_utf8 = false;
        self.mouse_pressed = None;
        self.prompt_mark = None;
        self.prompt_status = None;
        self.help_text = None;
        self.status_text = None;
        self.cwd_state = None;
        self.hyperlink = None;
        self.kitty_keyboard = false;
        self.cursor_visible = true;
        self.cursor_shape = CursorShape::Block;
        self.pen_strike = false;
        self.pen_overline = false;
        self.pen_conceal = false;
        self.set_default_fg(DEFAULT_FG);
        self.set_default_bg(DEFAULT_BG);
        self.set_cursor_color(Rgba {
            r: CURSOR[0],
            g: CURSOR[1],
            b: CURSOR[2],
            a: CURSOR[3],
        });
        self.reset_palette_indices(None);
        self.sixels.clear();
        if !keep_alt {
            self.alt_active = false;
        }
        self.clear_all();
    }

    pub fn reset_terminal(&mut self) {
        self.alt_state = None;
        self.alt_active = false;
        self.clear_all_and_scrollback();
    }

    pub fn clear_line_from_cursor(&mut self) {
        let blank = self.erase_cell();
        let start = self.cursor_row * self.cols + self.cursor_col;
        let end = self.cursor_row * self.cols + self.cols;
        if self.cursor_col > 0 && start < self.cells.len() && self.cells[start].wide_continuation {
            let lead = start - 1;
            if lead < self.cells.len() {
                self.cells[lead] = blank.clone();
            }
        }
        for cell in &mut self.cells[start..end] {
            *cell = blank.clone();
        }
        self.mark_row_dirty(self.cursor_row);
    }

    pub fn clear_line_to_cursor(&mut self) {
        if self.cursor_row >= self.rows {
            return;
        }
        let blank = self.erase_cell();
        let start = self.cursor_row * self.cols;
        let end = start + self.cursor_col.min(self.cols);
        for cell in &mut self.cells[start..end] {
            *cell = blank.clone();
        }
        self.mark_row_dirty(self.cursor_row);
    }

    pub fn clear_line_all(&mut self) {
        if self.cursor_row >= self.rows {
            return;
        }
        let blank = self.erase_cell();
        let start = self.cursor_row * self.cols;
        let end = (start + self.cols).min(self.cells.len());
        for cell in &mut self.cells[start..end] {
            *cell = blank.clone();
        }
        self.mark_row_dirty(self.cursor_row);
    }

    pub fn clear_screen_from_cursor(&mut self) {
        let blank = self.erase_cell();
        let start = self.cursor_row * self.cols + self.cursor_col;
        for cell in &mut self.cells[start..] {
            *cell = blank.clone();
        }
        self.mark_rows_dirty(self.cursor_row, self.rows.saturating_sub(1));
    }

    pub fn clear_screen_to_cursor(&mut self) {
        let blank = self.erase_cell();
        let end = self.cursor_row * self.cols + self.cursor_col;
        let end = end.min(self.cells.len());
        for cell in &mut self.cells[..end] {
            *cell = blank.clone();
        }
        self.mark_rows_dirty(0, self.cursor_row);
    }

    pub fn sanitize_line(&mut self, row: usize) {
        let start = row.saturating_mul(self.cols);
        let end = (start + self.cols).min(self.cells.len());
        let mut prev_wide = false;
        let blank = self.erase_cell();
        for idx in start..end {
            let cell = &mut self.cells[idx];
            if cell.wide_continuation {
                if !prev_wide {
                    *cell = blank.clone();
                } else {
                    prev_wide = false;
                }
            } else {
                prev_wide = cell.wide;
            }
        }
        self.mark_row_dirty(row);
    }

    pub fn delete_chars(&mut self, count: usize) {
        if count == 0 || self.cursor_col >= self.cols {
            return;
        }
        let start = self.cursor_row * self.cols + self.cursor_col;
        let end = self.cursor_row * self.cols + self.cols;
        let span = end.saturating_sub(start);
        let shift = count.min(span);
        let blank = self.erase_cell();
        for i in start..end.saturating_sub(shift) {
            self.cells[i] = self.cells[i + shift].clone();
        }
        for cell in &mut self.cells[end.saturating_sub(shift)..end] {
            *cell = blank.clone();
        }
        self.sanitize_line(self.cursor_row);
        self.mark_row_dirty(self.cursor_row);
    }

    pub fn insert_spaces(&mut self, count: usize) {
        if count == 0 || self.cursor_col >= self.cols {
            return;
        }
        let start = self.cursor_row * self.cols + self.cursor_col;
        let end = self.cursor_row * self.cols + self.cols;
        let span = end.saturating_sub(start);
        let shift = count.min(span);
        let blank = self.erase_cell();
        if shift == span {
            for cell in &mut self.cells[start..end] {
                *cell = blank.clone();
            }
        } else {
            for i in (start..end.saturating_sub(shift)).rev() {
                self.cells[i + shift] = self.cells[i].clone();
            }
            for cell in &mut self.cells[start..start + shift] {
                *cell = blank.clone();
            }
        }
        self.sanitize_line(self.cursor_row);
        self.mark_row_dirty(self.cursor_row);
    }

    pub fn set_fg(&mut self, color: Rgba) {
        self.pen_fg = color;
    }

    pub fn set_bg(&mut self, color: Rgba) {
        let mut color = color;
        if debug_bg_enabled() && color.a != 255 {
            log::debug!(
                "bg alpha clamped: pen bg rgba({}, {}, {}, {})",
                color.r,
                color.g,
                color.b,
                color.a
            );
        }
        color.a = 255;
        self.pen_bg = color;
    }

    pub fn reset_style(&mut self) {
        self.pen_fg = self.default_fg;
        self.pen_bg = self.default_bg;
        self.pen_inverse = false;
        self.pen_bold = false;
        self.pen_faint = false;
        self.pen_blink = false;
        self.pen_italic = false;
        self.pen_underline = false;
        self.pen_strike = false;
        self.pen_overline = false;
        self.pen_conceal = false;
    }

    pub fn set_default_fg(&mut self, fg: Rgba) {
        let previous = self.default_fg;
        if fg == previous {
            return;
        }
        self.default_fg = fg;
        self.refresh_blank_cell();
        if self.pen_fg == previous {
            self.pen_fg = fg;
        }
        if let Some(mut saved) = self.saved_cursor {
            if saved.fg == previous {
                saved.fg = fg;
            }
            self.saved_cursor = Some(saved);
        }
        for sprite in &mut self.sixels {
            // no-op for now; sixels are rendered as-is
            let _ = sprite;
        }
        for cell in &mut self.cells {
            if cell.fg == previous {
                cell.fg = fg;
            }
        }
        for line in &mut self.scrollback {
            for cell in line {
                if cell.fg == previous {
                    cell.fg = fg;
                }
            }
        }
        if let Some(state) = self.alt_state.as_mut() {
            for cell in &mut state.cells {
                if cell.fg == previous {
                    cell.fg = fg;
                }
            }
            if state.pen_fg == previous {
                state.pen_fg = fg;
            }
            state.default_fg = fg;
            if let Some(mut saved) = state.saved_cursor {
                if saved.fg == previous {
                    saved.fg = fg;
                }
                state.saved_cursor = Some(saved);
            }
        }
        self.mark_all_dirty();
    }

    pub fn set_default_bg(&mut self, bg: Rgba) {
        let mut bg = bg;
        if debug_bg_enabled() && bg.a != 255 {
            log::debug!(
                "bg alpha clamped: default bg rgba({}, {}, {}, {})",
                bg.r,
                bg.g,
                bg.b,
                bg.a
            );
        }
        bg.a = 255;
        let previous = self.default_bg;
        if bg == previous {
            return;
        }
        self.default_bg = bg;
        self.refresh_blank_cell();
        if self.pen_bg == previous {
            self.pen_bg = bg;
        }
        if let Some(mut saved) = self.saved_cursor {
            if saved.bg == previous {
                saved.bg = bg;
            }
            self.saved_cursor = Some(saved);
        }
        for sprite in &mut self.sixels {
            let _ = sprite;
        }
        for cell in &mut self.cells {
            if cell.bg == previous {
                cell.bg = bg;
            }
        }
        for line in &mut self.scrollback {
            for cell in line {
                if cell.bg == previous {
                    cell.bg = bg;
                }
            }
        }
        if let Some(state) = self.alt_state.as_mut() {
            for cell in &mut state.cells {
                if cell.bg == previous {
                    cell.bg = bg;
                }
            }
            if state.pen_bg == previous {
                state.pen_bg = bg;
            }
            state.default_bg = bg;
            if let Some(mut saved) = state.saved_cursor {
                if saved.bg == previous {
                    saved.bg = bg;
                }
                state.saved_cursor = Some(saved);
            }
        }
        self.mark_all_dirty();
    }

    pub fn default_bg(&self) -> Rgba {
        self.default_bg
    }

    pub fn default_fg(&self) -> Rgba {
        self.default_fg
    }

    fn refresh_blank_cell(&mut self) {
        self.blank_cell = Cell::blank_with(self.default_fg, self.default_bg);
    }

    pub fn set_cursor_color(&mut self, color: Rgba) {
        self.cursor_color = color;
    }

    pub fn cursor_color(&self) -> Rgba {
        self.cursor_color
    }

    pub fn palette_color(&self, idx: usize) -> Rgba {
        self.palette.get(idx).copied().unwrap_or(DEFAULT_FG)
    }

    pub fn set_palette_color(&mut self, idx: usize, color: Rgba) {
        if let Some(entry) = self.palette.get_mut(idx) {
            *entry = color;
        }
    }

    pub fn reset_palette_indices(&mut self, indices: Option<Vec<usize>>) {
        let defaults = *DEFAULT_PALETTE;
        match indices {
            Some(list) => {
                for idx in list {
                    if idx < self.palette.len() {
                        self.palette[idx] = defaults[idx];
                    }
                }
            }
            None => {
                self.palette = defaults;
            }
        }
    }

    pub fn push_scrollback_row(&mut self, row: usize) {
        if self.scrollback_limit == 0 {
            return;
        }
        if row >= self.rows {
            return;
        }
        let start = row * self.cols;
        let end = (start + self.cols).min(self.cells.len());
        let mut line = Vec::with_capacity(self.cols);
        line.extend_from_slice(&self.cells[start..end]);
        self.scrollback.push(line);
        if self.scrollback.len() > self.scrollback_limit {
            let overflow = self.scrollback.len() - self.scrollback_limit;
            self.scrollback.drain(0..overflow);
        }
    }

    pub fn clamp_view_offset(&mut self) {
        if self.view_offset > self.scrollback.len() {
            self.set_view_offset(self.scrollback.len());
        }
    }

    pub fn set_scrollback_enabled(&mut self, enabled: bool) {
        self.scrollback_limit = if enabled { DEFAULT_SCROLLBACK_LIMIT } else { 0 };
        if !enabled {
            self.scrollback.clear();
            self.set_view_offset(0);
        } else {
            self.clamp_view_offset();
        }
    }

    pub fn scroll_view_offset(&mut self, lines: i32) {
        if lines == 0 {
            return;
        }
        let current = self.view_offset as i32;
        let target = (current + lines).clamp(0, self.scrollback.len() as i32);
        self.set_view_offset(target as usize);
    }

    pub fn row_version(&self, row: usize) -> u64 {
        self.dirty_rows.get(row).copied().unwrap_or(0)
    }

    pub fn set_view_offset(&mut self, value: usize) {
        if self.view_offset == value {
            return;
        }
        self.view_offset = value;
        self.mark_all_dirty();
    }

    fn mark_row_dirty(&mut self, row: usize) {
        self.mark_rows_dirty(row, row);
    }

    fn mark_rows_dirty(&mut self, start: usize, end: usize) {
        if self.rows == 0 {
            return;
        }
        let start = start.min(self.rows.saturating_sub(1));
        let end = end.min(self.rows.saturating_sub(1));
        let next = self.dirty_epoch.wrapping_add(1);
        for row in start..=end {
            self.dirty_rows[row] = next;
        }
        self.dirty_epoch = next;
    }

    fn mark_all_dirty(&mut self) {
        if self.rows == 0 {
            return;
        }
        let next = self.dirty_epoch.wrapping_add(1);
        for row in &mut self.dirty_rows {
            *row = next;
        }
        self.dirty_epoch = next;
    }

    pub fn display_cell(&self, col: usize, row: usize) -> Cell {
        self.display_cell_ref(col, row).clone()
    }

    pub fn display_cell_ref(&self, col: usize, row: usize) -> &Cell {
        if col >= self.cols {
            return &self.blank_cell;
        }
        let hist_len = self.scrollback.len();
        let base = hist_len.saturating_sub(self.view_offset);
        let idx = base + row;
        if idx < hist_len {
            let line = &self.scrollback[idx];
            return line.get(col).unwrap_or(&self.blank_cell);
        }
        let screen_row = idx - hist_len;
        if screen_row >= self.rows {
            return &self.blank_cell;
        }
        let i = screen_row * self.cols + col;
        self.cells.get(i).unwrap_or(&self.blank_cell)
    }

    fn prev_leading_cell_index(&self) -> Option<usize> {
        if self.cursor_col == 0 {
            return None;
        }
        let mut col = self.cursor_col.saturating_sub(1);
        let mut idx = self
            .cursor_row
            .saturating_mul(self.cols)
            .saturating_add(col);
        if idx >= self.cells.len() {
            return None;
        }
        if self.cells[idx].wide_continuation && col > 0 {
            col = col.saturating_sub(1);
            idx = self
                .cursor_row
                .saturating_mul(self.cols)
                .saturating_add(col);
        }
        if idx < self.cells.len() {
            Some(idx)
        } else {
            None
        }
    }

    fn should_combine_with_prev(&self, ch: char, prev: Option<&Cell>) -> bool {
        if let Some(prev_cell) = prev {
            if let Some(last) = prev_cell.text.chars().last() {
                if last == '\u{200d}' {
                    return true;
                }
                if matches!(last, '\u{1f1e6}'..='\u{1f1ff}')
                    && matches!(ch, '\u{1f1e6}'..='\u{1f1ff}')
                {
                    return true;
                }
            }
        }

        // Treat zero-width codepoints, emoji modifiers, and ZWJ sequences as part of the
        // previous grapheme so they render as a single cell.
        let width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width == 0 {
            return true;
        }
        // Emoji modifiers (skin tones) and zero width joiners extend the prior glyph.
        if matches!(ch as u32, 0x1f3fb..=0x1f3ff | 0x200d) {
            return true;
        }
        false
    }

    fn recompute_cell_width(&mut self, idx: usize) {
        if idx >= self.cells.len() {
            return;
        }
        let mut cell = self.cells[idx].clone();
        let width = UnicodeWidthStr::width(cell.text.as_str()).clamp(1, 2);
        cell.wide = width > 1;
        cell.wide_continuation = false;

        let col = idx % self.cols;
        if cell.wide {
            let cont_idx = idx + 1;
            if col + 1 < self.cols && cont_idx < self.cells.len() {
                let fg = cell.fg;
                let bg = cell.bg;
                self.cells[cont_idx] = Cell {
                    text: " ".to_string(),
                    wide: false,
                    wide_continuation: true,
                    fg,
                    bg,
                    bold: cell.bold,
                    faint: cell.faint,
                    blink: cell.blink,
                    italic: cell.italic,
                    underline: cell.underline,
                    strike: cell.strike,
                    overline: cell.overline,
                    concealed: cell.concealed,
                    hyperlink: cell.hyperlink.clone(),
                };
            }
        } else if col + 1 < self.cols {
            let cont_idx = idx + 1;
            if cont_idx < self.cells.len() && self.cells[cont_idx].wide_continuation {
                self.cells[cont_idx] = Cell::blank_with(self.default_fg, self.default_bg);
            }
        }
        self.cells[idx] = cell;
        let row = idx / self.cols;
        self.mark_row_dirty(row);
    }

    pub fn put_char(&mut self, ch: char) {
        if self.wrap_next {
            self.new_line();
            self.wrap_next = false;
        }

        // If we are at column 0 and the line already contains text, wipe it before writing.
        // Shells redraw prompts in-place using carriage return; without clearing, shorter lines
        // leave remnants that look like commands are appended.
        if self.cursor_col == 0 && !self.line_is_blank(self.cursor_row) {
            self.clear_line_all();
        }

        if ch == '\n' {
            self.new_line();
            return;
        }

        let prev_idx = self.prev_leading_cell_index();
        if self.should_combine_with_prev(ch, prev_idx.map(|i| &self.cells[i])) {
            if let Some(idx) = prev_idx {
                if debug_width_enabled() {
                    let ch_dbg = ch.escape_default().to_string();
                    log::debug!(
                        "width combine: ch='{}' cursor=({}, {})",
                        ch_dbg,
                        self.cursor_col,
                        self.cursor_row
                    );
                }
                self.cells[idx].push_char(ch);
                self.recompute_cell_width(idx);
                self.mark_row_dirty(self.cursor_row);
            }
            return;
        }

        let width = UnicodeWidthChar::width(ch).unwrap_or(1).max(1).min(2);

        let line_limit = self
            .right_margin
            .min(self.cols.saturating_sub(1))
            .saturating_add(1);
        if width > 1 && self.cursor_col + width > line_limit {
            if debug_width_enabled() {
                let ch_dbg = ch.escape_default().to_string();
                log::debug!(
                    "width wrap: ch='{}' width={} cursor_col={} line_limit={}",
                    ch_dbg,
                    width,
                    self.cursor_col,
                    line_limit
                );
            }
            self.new_line();
            self.wrap_next = false;
        }

        let idx = self.cursor_row * self.cols + self.cursor_col;
        if idx < self.cells.len() {
            if debug_width_enabled() {
                let ch_dbg = ch.escape_default().to_string();
                log::debug!(
                    "width put: ch='{}' width={} cursor=({}, {}) cols={}",
                    ch_dbg,
                    width,
                    self.cursor_col,
                    self.cursor_row,
                    self.cols
                );
            }
            let (fg, bg) = self.effective_colors();
            self.cells[idx] = Cell {
                text: ch.to_string(),
                wide: width > 1,
                wide_continuation: false,
                fg,
                bg,
                bold: self.pen_bold,
                faint: self.pen_faint,
                blink: self.pen_blink,
                italic: self.pen_italic,
                underline: self.pen_underline,
                strike: self.pen_strike,
                overline: self.pen_overline,
                concealed: self.pen_conceal,
                hyperlink: self.hyperlink.clone(),
            };
            if width > 1 && idx + 1 < self.cells.len() {
                let (fg, bg) = self.effective_colors();
                self.cells[idx + 1] = Cell {
                    text: " ".to_string(),
                    wide: false,
                    wide_continuation: true,
                    fg,
                    bg,
                    bold: self.pen_bold,
                    faint: self.pen_faint,
                    blink: self.pen_blink,
                    italic: self.pen_italic,
                    underline: self.pen_underline,
                    strike: self.pen_strike,
                    overline: self.pen_overline,
                    concealed: self.pen_conceal,
                    hyperlink: self.hyperlink.clone(),
                };
            }
            self.mark_row_dirty(self.cursor_row);
        }

        self.cursor_col += width;
        let max_col = self.right_margin.min(self.cols.saturating_sub(1));
        if self.cursor_col > max_col {
            self.cursor_col = max_col;
            self.wrap_next = true;
        }
    }

    pub fn backspace(&mut self) {
        self.wrap_next = false;
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            let idx = self.cursor_row * self.cols + self.cursor_col;
            if idx < self.cells.len() {
                let blank = self.erase_cell();
                if self.cells[idx].wide_continuation && self.cursor_col > 0 {
                    let lead_idx = idx.saturating_sub(1);
                    self.cursor_col = self.cursor_col.saturating_sub(1);
                    if lead_idx < self.cells.len() {
                        self.cells[lead_idx] = blank.clone();
                    }
                    self.cells[idx] = blank.clone();
                } else {
                    if self.cells[idx].wide {
                        let cont_idx = idx + 1;
                        if cont_idx < self.cells.len() {
                            self.cells[cont_idx] = blank.clone();
                        }
                    }
                    self.cells[idx] = blank;
                }
                self.mark_row_dirty(self.cursor_row);
            }
        }
    }

    pub fn carriage_return(&mut self) {
        self.cursor_col = self.left_margin;
        self.wrap_next = false;
    }

    pub fn tab(&mut self) {
        let next = ((self.cursor_col / TAB_WIDTH) + 1) * TAB_WIDTH;
        let max_col = self.right_margin.min(self.cols.saturating_sub(1));
        self.cursor_col = next.min(max_col);
        self.wrap_next = false;
    }

    pub fn new_line(&mut self) {
        self.wrap_next = false;
        self.cursor_col = self.left_margin;
        if self.cursor_row >= self.bottom_margin {
            if !self.alt_active
                && self.top_margin == 0
                && self.bottom_margin == self.rows.saturating_sub(1)
            {
                self.push_scrollback_row(0);
            }
            self.scroll_up_region(1);
        } else {
            self.cursor_row += 1;
        }
        self.set_view_offset(0);
    }

    pub fn scroll_up_region(&mut self, count: usize) {
        if self.rows <= 1 || count == 0 {
            return;
        }
        // Clear sixels when shifting text to avoid stale overlays.
        self.sixels.clear();
        let top = self.top_margin;
        let bottom = self.bottom_margin.min(self.rows.saturating_sub(1));
        if bottom < top {
            return;
        }
        let height = bottom.saturating_sub(top).saturating_add(1);
        let shift = count.min(height);
        let row_len = self.cols;
        let blank = self.erase_cell();

        for r in top..=bottom {
            let dst_row = r;
            let src_row = r + shift;
            if src_row <= bottom {
                let dst = dst_row * row_len;
                let src = src_row * row_len;
                self.clone_within(src, dst, row_len);
            } else {
                let start = dst_row * row_len;
                let end = (start + row_len).min(self.cells.len());
                for cell in &mut self.cells[start..end] {
                    *cell = blank.clone();
                }
            }
        }

        for r in top..=bottom {
            self.sanitize_line(r);
        }
        self.cursor_row = self.cursor_row.min(bottom);
        self.set_view_offset(0);
        self.mark_rows_dirty(top, bottom);
    }

    pub fn scroll_down_region(&mut self, count: usize) {
        if self.rows <= 1 || count == 0 {
            return;
        }
        self.sixels.clear();
        let top = self.top_margin;
        let bottom = self.bottom_margin.min(self.rows.saturating_sub(1));
        if bottom < top {
            return;
        }
        let height = bottom.saturating_sub(top).saturating_add(1);
        let shift = count.min(height);
        let row_len = self.cols;
        let blank = self.erase_cell();

        for r in (top..=bottom).rev() {
            let dst_row = r;
            let src_row = r.saturating_sub(shift);
            if src_row >= top {
                let dst = dst_row * row_len;
                let src = src_row * row_len;
                self.clone_within(src, dst, row_len);
            } else {
                let start = dst_row * row_len;
                let end = (start + row_len).min(self.cells.len());
                for cell in &mut self.cells[start..end] {
                    *cell = blank.clone();
                }
            }
        }

        for r in top..=bottom {
            self.sanitize_line(r);
        }
        self.cursor_row = self.cursor_row.max(top);
        self.set_view_offset(0);
        self.mark_rows_dirty(top, bottom);
    }

    pub fn insert_lines(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.sixels.clear();
        let top = self.cursor_row;
        let bottom = self.bottom_margin.min(self.rows.saturating_sub(1));
        if top > bottom {
            return;
        }
        let height = bottom.saturating_sub(top).saturating_add(1);
        let shift = count.min(height);
        let row_len = self.cols;
        let blank = self.erase_cell();

        for r in (top..=bottom).rev() {
            let dst_row = r;
            let src_row = r.saturating_sub(shift);
            if src_row >= top {
                let dst = dst_row * row_len;
                let src = src_row * row_len;
                self.clone_within(src, dst, row_len);
            } else {
                let start = dst_row * row_len;
                let end = (start + row_len).min(self.cells.len());
                for cell in &mut self.cells[start..end] {
                    *cell = blank.clone();
                }
            }
        }

        for r in top..=bottom {
            self.sanitize_line(r);
        }
        self.cursor_row = top;
        self.set_view_offset(0);
        self.mark_rows_dirty(top, bottom);
    }

    pub fn delete_lines(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.sixels.clear();
        let top = self.cursor_row;
        let bottom = self.bottom_margin.min(self.rows.saturating_sub(1));
        if top > bottom {
            return;
        }
        let height = bottom.saturating_sub(top).saturating_add(1);
        let shift = count.min(height);
        let row_len = self.cols;
        let blank = self.erase_cell();

        for r in top..=bottom {
            let dst_row = r;
            let src_row = r + shift;
            if src_row <= bottom {
                let dst = dst_row * row_len;
                let src = src_row * row_len;
                self.clone_within(src, dst, row_len);
            } else {
                let start = dst_row * row_len;
                let end = (start + row_len).min(self.cells.len());
                for cell in &mut self.cells[start..end] {
                    *cell = blank.clone();
                }
            }
        }

        for r in top..=bottom {
            self.sanitize_line(r);
        }
        self.cursor_row = top;
        self.set_view_offset(0);
        self.mark_rows_dirty(top, bottom);
    }

    pub fn cursor_up(&mut self, count: usize) {
        self.wrap_next = false;
        let limit = if self.origin_mode { self.top_margin } else { 0 };
        self.cursor_row = self.cursor_row.saturating_sub(count).max(limit);
    }

    pub fn cursor_down(&mut self, count: usize) {
        self.wrap_next = false;
        let limit = if self.origin_mode {
            self.bottom_margin.min(self.rows.saturating_sub(1))
        } else {
            self.rows.saturating_sub(1)
        };
        self.cursor_row = (self.cursor_row + count).min(limit);
        self.set_view_offset(0);
    }

    pub fn cursor_right(&mut self, count: usize) {
        self.wrap_next = false;
        let max_col = self.right_margin.min(self.cols.saturating_sub(1));
        self.cursor_col = (self.cursor_col + count).min(max_col);
    }

    pub fn cursor_left(&mut self, count: usize) {
        self.wrap_next = false;
        let min_col = self.left_margin.min(self.cols.saturating_sub(1));
        self.cursor_col = self.cursor_col.saturating_sub(count).max(min_col);
    }

    pub fn move_to_line_end(&mut self) {
        if self.cursor_row >= self.rows {
            return;
        }
        let start = self.cursor_row * self.cols;
        let mut last = 0;
        for c in 0..self.cols {
            let cell = &self.cells[start + c];
            if !cell.is_blank() || cell.wide_continuation {
                last = c;
            }
        }
        let max_col = self.right_margin.min(self.cols.saturating_sub(1));
        self.cursor_col = last.min(max_col);
    }

    pub fn line_is_blank(&self, row: usize) -> bool {
        if row >= self.rows {
            return true;
        }
        let start = row * self.cols;
        let end = start + self.cols.min(self.cells.len().saturating_sub(start));
        self.cells[start..end]
            .iter()
            .all(|c| c.is_blank() && !c.wide_continuation)
    }

    pub fn current_line_prefix(&self) -> String {
        if self.cursor_row >= self.rows {
            return String::new();
        }
        let start = self.cursor_row * self.cols;
        let end = start + self.cursor_col.min(self.cols);
        let mut s = String::new();
        for cell in &self.cells[start..end] {
            if cell.wide_continuation {
                continue;
            }
            s.push_str(&cell.text);
        }
        s
    }

    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        self.top_margin = top.min(self.rows.saturating_sub(1));
        self.bottom_margin = bottom.min(self.rows.saturating_sub(1));
        if self.top_margin > self.bottom_margin {
            self.top_margin = 0;
            self.bottom_margin = self.rows.saturating_sub(1);
        }
        self.cursor_row = self.top_margin;
        self.cursor_col = self.left_margin;
        self.set_view_offset(0);
    }

    pub fn set_left_right_margin(&mut self, left: usize, right: usize) {
        let max_col = self.cols.saturating_sub(1);
        let left = left.min(max_col);
        let right = right.min(max_col);
        if left >= right {
            self.left_margin = 0;
            self.right_margin = max_col;
        } else {
            self.left_margin = left;
            self.right_margin = right;
        }
        self.set_cursor(self.left_margin, self.cursor_row);
    }

    pub fn save_cursor(&mut self) {
        self.saved_cursor = Some(SavedCursor {
            col: self.cursor_col,
            row: self.cursor_row,
            fg: self.pen_fg,
            bg: self.pen_bg,
            inverse: self.pen_inverse,
            bold: self.pen_bold,
            faint: self.pen_faint,
            blink: self.pen_blink,
            italic: self.pen_italic,
            underline: self.pen_underline,
            strike: self.pen_strike,
            overline: self.pen_overline,
            conceal: self.pen_conceal,
            origin_mode: self.origin_mode,
            wrap_next: self.wrap_next,
        });
    }

    pub fn restore_cursor(&mut self) {
        if let Some(saved) = self.saved_cursor {
            self.origin_mode = saved.origin_mode;
            self.set_cursor(saved.col, saved.row);
            self.pen_fg = saved.fg;
            self.pen_bg = saved.bg;
            self.pen_inverse = saved.inverse;
            self.pen_bold = saved.bold;
            self.pen_faint = saved.faint;
            self.pen_blink = saved.blink;
            self.pen_italic = saved.italic;
            self.pen_underline = saved.underline;
            self.pen_strike = saved.strike;
            self.pen_overline = saved.overline;
            self.pen_conceal = saved.conceal;
            self.wrap_next = saved.wrap_next;
        }
    }

    pub fn enter_alt_screen(&mut self) {
        if self.alt_active {
            self.clear_all();
            return;
        }
        let backup = AltState {
            cells: std::mem::take(&mut self.cells),
            cursor_col: self.cursor_col,
            cursor_row: self.cursor_row,
            saved_cursor: self.saved_cursor,
            left_margin: self.left_margin,
            right_margin: self.right_margin,
            pen_fg: self.pen_fg,
            pen_bg: self.pen_bg,
            pen_inverse: self.pen_inverse,
            pen_bold: self.pen_bold,
            pen_faint: self.pen_faint,
            pen_blink: self.pen_blink,
            pen_italic: self.pen_italic,
            pen_underline: self.pen_underline,
            pen_strike: self.pen_strike,
            pen_overline: self.pen_overline,
            pen_conceal: self.pen_conceal,
            wrap_next: self.wrap_next,
            mouse_pressed: self.mouse_pressed,
            default_bg: self.default_bg,
            default_fg: self.default_fg,
            cursor_color: self.cursor_color,
            cursor_visible: self.cursor_visible,
            cursor_shape: self.cursor_shape,
            palette: self.palette,
            sixels: self.sixels.clone(),
        };
        self.alt_state = Some(backup);
        self.cells = vec![
            Cell::blank_with(self.default_fg, self.default_bg);
            self.cols.saturating_mul(self.rows).max(1)
        ];
        self.cursor_col = 0;
        self.cursor_row = 0;
        self.saved_cursor = None;
        self.alt_active = true;
        self.set_view_offset(0);
        self.top_margin = 0;
        self.bottom_margin = self.rows.saturating_sub(1);
        self.origin_mode = false;
        self.wrap_next = false;
        self.dirty_rows = vec![0u64; self.rows.max(1)];
        self.mark_all_dirty();
    }

    pub fn leave_alt_screen(&mut self) {
        if let Some(mut state) = self.alt_state.take() {
            if state.cells.len() != self.cols.saturating_mul(self.rows) {
                resize_cells(
                    &mut state.cells,
                    self.cols,
                    self.rows,
                    self.default_fg,
                    self.default_bg,
                );
            }
            self.cells = state.cells;
            self.cursor_col = state.cursor_col.min(self.cols.saturating_sub(1));
            self.cursor_row = state.cursor_row.min(self.rows.saturating_sub(1));
            self.saved_cursor = state.saved_cursor;
            self.left_margin = state.left_margin.min(self.cols.saturating_sub(1));
            self.right_margin = state.right_margin.min(self.cols.saturating_sub(1));
            self.pen_fg = state.pen_fg;
            self.pen_bg = state.pen_bg;
            self.pen_inverse = state.pen_inverse;
            self.pen_bold = state.pen_bold;
            self.pen_faint = state.pen_faint;
            self.pen_blink = state.pen_blink;
            self.pen_italic = state.pen_italic;
            self.pen_underline = state.pen_underline;
            self.pen_strike = state.pen_strike;
            self.pen_overline = state.pen_overline;
            self.pen_conceal = state.pen_conceal;
            self.wrap_next = state.wrap_next;
            self.mouse_pressed = state.mouse_pressed;
            self.default_bg = state.default_bg;
            self.default_fg = state.default_fg;
            self.refresh_blank_cell();
            self.cursor_color = state.cursor_color;
            self.cursor_visible = state.cursor_visible;
            self.cursor_shape = state.cursor_shape;
            self.palette = state.palette;
            self.sixels = state.sixels;
        }
        self.alt_active = false;
        self.set_view_offset(0);
        self.top_margin = 0;
        self.bottom_margin = self.rows.saturating_sub(1);
        self.origin_mode = false;

        // Leave primary contents intact; programs should clear explicitly if desired.
        self.dirty_rows = vec![0u64; self.rows.max(1)];
        self.mark_all_dirty();
    }

    pub fn leave_alt_screen_preserve(&mut self) {
        if let Some(mut state) = self.alt_state.take() {
            if state.cells.len() != self.cols.saturating_mul(self.rows) {
                resize_cells(
                    &mut state.cells,
                    self.cols,
                    self.rows,
                    self.default_fg,
                    self.default_bg,
                );
            }
            self.cells = state.cells;
            self.cursor_col = state.cursor_col.min(self.cols.saturating_sub(1));
            self.cursor_row = state.cursor_row.min(self.rows.saturating_sub(1));
            self.saved_cursor = state.saved_cursor;
            self.left_margin = state.left_margin.min(self.cols.saturating_sub(1));
            self.right_margin = state.right_margin.min(self.cols.saturating_sub(1));
            self.pen_fg = state.pen_fg;
            self.pen_bg = state.pen_bg;
            self.pen_inverse = state.pen_inverse;
            self.pen_bold = state.pen_bold;
            self.pen_faint = state.pen_faint;
            self.pen_blink = state.pen_blink;
            self.pen_italic = state.pen_italic;
            self.pen_underline = state.pen_underline;
            self.pen_strike = state.pen_strike;
            self.pen_overline = state.pen_overline;
            self.wrap_next = state.wrap_next;
            self.mouse_pressed = state.mouse_pressed;
            self.default_bg = state.default_bg;
            self.default_fg = state.default_fg;
            self.refresh_blank_cell();
            self.cursor_color = state.cursor_color;
            self.cursor_visible = state.cursor_visible;
            self.cursor_shape = state.cursor_shape;
            self.palette = state.palette;
            self.sixels = state.sixels;
        }
        self.alt_active = false;
        self.set_view_offset(0);
        self.top_margin = 0;
        self.bottom_margin = self.rows.saturating_sub(1);
        self.origin_mode = false;
        self.dirty_rows = vec![0u64; self.rows.max(1)];
        self.mark_all_dirty();
    }

    pub fn effective_colors(&self) -> (Rgba, Rgba) {
        if self.pen_inverse {
            (self.pen_bg, self.pen_fg)
        } else {
            (self.pen_fg, self.pen_bg)
        }
    }

    fn clone_within(&mut self, src: usize, dst: usize, len: usize) {
        if len == 0 || src >= self.cells.len() || dst >= self.cells.len() {
            return;
        }
        let max_src = src.saturating_add(len);
        let max_dst = dst.saturating_add(len);
        if max_src > self.cells.len() || max_dst > self.cells.len() {
            return;
        }
        if dst > src {
            for offset in (0..len).rev() {
                let from = src + offset;
                let to = dst + offset;
                self.cells[to] = self.cells[from].clone();
            }
        } else {
            for offset in 0..len {
                let from = src + offset;
                let to = dst + offset;
                self.cells[to] = self.cells[from].clone();
            }
        }
    }
}

impl Perform for GridPerformer<'_> {
    fn hook(&mut self, params: &Params, _intermediates: &[u8], _ignored: bool, action: char) {
        if action != 'q' {
            return;
        }
        let param_at =
            |idx: usize| -> Option<u16> { params.iter().nth(idx).and_then(|p| p.get(0)).copied() };
        self.dcs_state = Some(DcsState {
            data: Vec::new(),
            aspect_ratio: param_at(0),
            zero_color: param_at(1),
            grid_size: param_at(2),
            col: self.grid.cursor_col,
            row: self.grid.cursor_row,
        });
    }

    fn put(&mut self, byte: u8) {
        if let Some(state) = &mut self.dcs_state {
            state.data.push(byte);
        }
    }

    fn unhook(&mut self) {
        let Some(state) = self.dcs_state.take() else {
            return;
        };
        let Some(payload) = dcs_bytes_from_payload(&state.data) else {
            return;
        };
        if payload.is_empty() {
            return;
        }
        let settings = DcsSettings::new(state.aspect_ratio, state.zero_color, state.grid_size);
        if let Ok(image) = sixel_decode_from_dcs(payload, settings) {
            let (width, height) = image.corrected_dimensions();
            self.grid.sixels.push(SixelSprite {
                col: state.col,
                row: state.row,
                width,
                height,
                pixels: image.pixels,
            });
        }
    }
    fn print(&mut self, c: char) {
        self.grid.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.grid.new_line(),
            b'\r' => self.grid.carriage_return(),
            8 => self.grid.cursor_left(1), // BS is non-destructive cursor move
            127 => self.grid.backspace(),  // DEL deletes previous char
            b'\t' => self.grid.tab(),
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignored: bool, byte: u8) {
        match byte {
            b'c' => self.grid.reset_terminal(),
            b'7' => self.grid.save_cursor(),
            b'8' => self.grid.restore_cursor(),
            _ => {}
        };
    }

    fn csi_dispatch(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        _ignored: bool,
        action: char,
    ) {
        let first = |default| {
            params
                .iter()
                .next()
                .and_then(|p| p.get(0).copied())
                .unwrap_or(default)
        };

        match action {
            'A' => self.grid.cursor_up(first(1) as usize),
            'B' => self.grid.cursor_down(first(1) as usize),
            'C' => self.grid.cursor_right(first(1) as usize),
            'D' => self.grid.cursor_left(first(1) as usize),
            'E' => {
                let count = first(1) as usize;
                self.grid.cursor_row = self.grid.cursor_row
                    + count.min(
                        self.grid
                            .rows
                            .saturating_sub(1)
                            .saturating_sub(self.grid.cursor_row),
                    );
                self.grid.cursor_col = 0;
            }
            'F' => {
                let count = first(1) as usize;
                self.grid.cursor_row = self.grid.cursor_row.saturating_sub(count);
                self.grid.cursor_col = 0;
            }
            'G' => {
                let col = first(1).saturating_sub(1) as usize;
                let row = self.grid.cursor_row;
                self.grid.set_cursor(col, row);
            }
            'd' => {
                let row = first(1).saturating_sub(1) as usize;
                let col = self.grid.cursor_col;
                self.grid.set_cursor(col, row);
            }
            'n' => {
                if first(0) == 6 {
                    let row = self.grid.cursor_row.saturating_add(1);
                    let col = self.grid.cursor_col.saturating_add(1);
                    let resp = format!("\x1b[{};{}R", row, col);
                    write_bytes(&self.writer, resp.as_bytes());
                } else if first(0) == 5 {
                    write_bytes(&self.writer, b"\x1b[0n");
                }
            }
            'h' | 'l' => {
                if intermediates == [b'?'] {
                    match first(0) {
                        1 => *self.app_cursor_keys = action == 'h',
                        6 => self.grid.origin_mode = action == 'h',
                        2004 => self.grid.bracketed_paste = action == 'h',
                        1004 => self.grid.set_focus_reporting(action == 'h'),
                        9 | 1000 => {
                            self.grid.mouse_btn_report = action == 'h';
                            self.grid.mouse_motion_report = false;
                        }
                        1002 | 1003 => {
                            self.grid.mouse_btn_report = action == 'h';
                            self.grid.mouse_motion_report = action == 'h';
                        }
                        1006 => self.grid.mouse_sgr = action == 'h',
                        1005 => self.grid.mouse_utf8 = action == 'h',
                        25 => self.grid.set_cursor_visible(action == 'h'),
                        1049 | 1047 => {
                            if action == 'h' {
                                self.grid.enter_alt_screen();
                            } else {
                                self.grid.leave_alt_screen();
                            }
                        }
                        1048 => {
                            if action == 'h' {
                                self.grid.save_cursor();
                            } else {
                                self.grid.restore_cursor();
                            }
                        }
                        _ => {}
                    }
                } else if intermediates.is_empty()
                    && first(0) != 0
                    && params.len() >= 2
                    && matches!(action, 's' | 't')
                {
                    // DECSLRM: CSI Pl ; Pr s/t
                    let left = params
                        .iter()
                        .next()
                        .and_then(|p| p.get(0))
                        .copied()
                        .unwrap_or(1)
                        .saturating_sub(1) as usize;
                    let right = params
                        .iter()
                        .nth(1)
                        .and_then(|p| p.get(0))
                        .copied()
                        .unwrap_or(self.grid.cols as u16) as usize;
                    self.grid
                        .set_left_right_margin(left, right.saturating_sub(1));
                }
            }
            'P' => self.grid.delete_chars(first(1) as usize),
            '@' => self.grid.insert_spaces(first(1) as usize),
            'S' => self.grid.scroll_up_region(first(1) as usize),
            'T' => self.grid.scroll_down_region(first(1) as usize),
            'L' => self.grid.insert_lines(first(1) as usize),
            'M' => self.grid.delete_lines(first(1) as usize),
            'c' => {
                write_bytes(&self.writer, b"\x1b[?1;2c");
            }
            'p' if intermediates == [b'!'] => {
                self.grid.clear_all_and_scrollback();
            }
            'q' if intermediates == [b' '] => {
                let shape = match first(1) {
                    0 | 1 | 2 => Some(CursorShape::Block),
                    3 | 4 => Some(CursorShape::Underline),
                    5 | 6 => Some(CursorShape::Bar),
                    _ => None,
                };
                if let Some(shape) = shape {
                    self.grid.set_cursor_shape(shape);
                }
            }
            'm' => {
                let mut flat_params: Vec<u16> =
                    params.iter().flat_map(|p| p.iter().copied()).collect();
                if flat_params.is_empty() {
                    flat_params.push(0); // ESC[m resets style per spec
                }

                let mut idx = 0;
                while idx < flat_params.len() {
                    let p = flat_params[idx];
                    match p {
                        0 => self.grid.reset_style(),
                        1 => {
                            let fg = brightened(self.grid.pen_fg);
                            self.grid.set_fg(fg);
                            self.grid.pen_bold = true;
                        }
                        2 => {
                            self.grid.pen_faint = true;
                        }
                        5 => self.grid.pen_blink = true,
                        3 => self.grid.pen_italic = true,
                        4 => self.grid.pen_underline = true,
                        9 => self.grid.pen_strike = true,
                        7 => self.grid.pen_inverse = true,
                        21 | 22 => {
                            self.grid.pen_bold = false;
                            self.grid.pen_faint = false;
                            self.grid.pen_blink = false;
                            self.grid.set_fg(self.grid.pen_fg);
                        }
                        23 => self.grid.pen_italic = false,
                        24 => self.grid.pen_underline = false,
                        29 => self.grid.pen_strike = false,
                        27 => self.grid.pen_inverse = false,
                        53 => self.grid.pen_overline = true,
                        55 => self.grid.pen_overline = false,
                        25 => self.grid.pen_blink = false,
                        30..=37 => self.grid.set_fg(self.grid.palette_color((p - 30) as usize)),
                        90..=97 => self
                            .grid
                            .set_fg(self.grid.palette_color(8 + (p - 90) as usize)),
                        39 => self.grid.set_fg(DEFAULT_FG),
                        40..=47 => self.grid.set_bg(self.grid.palette_color((p - 40) as usize)),
                        100..=107 => self
                            .grid
                            .set_bg(self.grid.palette_color(8 + (p - 100) as usize)),
                        49 => self.grid.set_bg(DEFAULT_BG),
                        38 => {
                            if let Some(mode) = flat_params.get(idx + 1).copied() {
                                if mode == 2 {
                                    let r = flat_params.get(idx + 2).copied().unwrap_or(255) as u8;
                                    let g = flat_params.get(idx + 3).copied().unwrap_or(255) as u8;
                                    let b = flat_params.get(idx + 4).copied().unwrap_or(255) as u8;
                                    self.grid.set_fg(Rgba { r, g, b, a: 255 });
                                    idx += 5;
                                    continue;
                                } else if mode == 5 {
                                    if let Some(code) = flat_params.get(idx + 2).copied() {
                                        self.grid.set_fg(self.grid.palette_color(code as usize));
                                    }
                                    idx += 3;
                                    continue;
                                }
                            }
                        }
                        48 => {
                            if let Some(mode) = flat_params.get(idx + 1).copied() {
                                if mode == 2 {
                                    let r = flat_params.get(idx + 2).copied().unwrap_or(0) as u8;
                                    let g = flat_params.get(idx + 3).copied().unwrap_or(0) as u8;
                                    let b = flat_params.get(idx + 4).copied().unwrap_or(0) as u8;
                                    self.grid.set_bg(Rgba { r, g, b, a: 255 });
                                    idx += 5;
                                    continue;
                                } else if mode == 5 {
                                    if let Some(code) = flat_params.get(idx + 2).copied() {
                                        self.grid.set_bg(self.grid.palette_color(code as usize));
                                    }
                                    idx += 3;
                                    continue;
                                }
                            }
                        }
                        _ => {}
                    }
                    idx += 1;
                }
            }
            'H' | 'f' => {
                let row = first(1).saturating_sub(1) as usize;
                let col = params
                    .iter()
                    .nth(1)
                    .and_then(|p| p.get(0).copied())
                    .unwrap_or(1)
                    .saturating_sub(1) as usize;
                self.grid.set_cursor(col, row);
            }
            'J' => {
                match first(0) {
                    0 => self.grid.clear_screen_from_cursor(),
                    1 => self.grid.clear_screen_to_cursor(),
                    2 => self.grid.clear_all(),
                    _ => {}
                };
            }
            'K' => match first(0) {
                0 => self.grid.clear_line_from_cursor(),
                1 => self.grid.clear_line_to_cursor(),
                2 => self.grid.clear_line_all(),
                _ => {}
            },
            's' => self.grid.save_cursor(),
            'u' => self.grid.restore_cursor(),
            'r' => {
                let top = first(1).saturating_sub(1) as usize;
                let bottom = params
                    .iter()
                    .nth(1)
                    .and_then(|p| p.get(0).copied())
                    .unwrap_or(self.grid.rows as u16)
                    .saturating_sub(1) as usize;
                self.grid.set_scroll_region(top, bottom);
            }
            _ => {}
        }
    }
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if let Some(mut tag) = params.first().copied() {
            // Handle combined "8;...;..." packed into the first param.
            let mut extras: Vec<&[u8]> = Vec::new();
            if let Some(idx) = tag.iter().position(|b| *b == b';') {
                let (t, rest) = tag.split_at(idx);
                tag = t;
                let rest = &rest[1..];
                if !rest.is_empty() {
                    extras.extend(rest.split(|b| *b == b';'));
                }
            }
            let mut merged: Vec<&[u8]> = Vec::new();
            merged.push(tag);
            merged.extend(extras);
            merged.extend_from_slice(&params.get(1..).unwrap_or_default());
            let tag = merged[0];
            let decode_title = |idx: usize| -> Option<String> {
                merged
                    .get(idx)
                    .and_then(|p| std::str::from_utf8(p).ok())
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
            };
            if tag == b"0" {
                if merged.get(1).map(|s| *s) == Some(b"?") {
                    if let Some(title) = self.grid.window_title() {
                        let resp = format!("\x1b]0;{}\u{07}", title);
                        write_bytes(&self.writer, resp.as_bytes());
                    }
                } else {
                    let title = decode_title(1);
                    self.grid.set_window_title(title.clone());
                    self.grid.set_icon_title(title);
                }
            } else if tag == b"1" {
                if merged.get(1).map(|s| *s) == Some(b"?") {
                    if let Some(title) = self.grid.icon_title() {
                        let resp = format!("\x1b]1;{}\u{07}", title);
                        write_bytes(&self.writer, resp.as_bytes());
                    }
                } else if merged.get(1).map(|p| *p) == Some(b"2") {
                    self.grid.kitty_keyboard = true;
                } else if merged.get(1).map(|p| *p) == Some(b"0") {
                    self.grid.kitty_keyboard = false;
                } else {
                    let title = decode_title(1);
                    self.grid.set_icon_title(title);
                }
            } else if tag == b"2" {
                if merged.get(1).map(|s| *s) == Some(b"?") {
                    if let Some(title) = self.grid.window_title() {
                        let resp = format!("\x1b]2;{}\u{07}", title);
                        write_bytes(&self.writer, resp.as_bytes());
                    }
                } else {
                    let title = decode_title(1);
                    self.grid.set_window_title(title);
                }
            } else if tag == b"52" {
                if let Some(encoded) = merged.get(2).or_else(|| merged.get(1)).copied() {
                    if let Ok(decoded) = general_purpose::STANDARD.decode(encoded) {
                        if let Ok(text) = String::from_utf8(decoded) {
                            if let Err(e) = write_clipboard_text(&text) {
                                log::error!("Failed to write clipboard text: {e}");
                            }
                        }
                    }
                }
            } else if tag == b"7" {
                if let Some(body) = merged.get(1).copied() {
                    if let Ok(text) = std::str::from_utf8(body) {
                        self.grid.cwd_state = Some(text.to_string());
                    }
                }
            } else if tag == b"8" || tag.starts_with(b"8;") {
                // OSC 8 ; params ; URI ST
                let uri_param = merged.get(2).or_else(|| merged.get(1)).copied();
                if let Some(uri_bytes) = uri_param {
                    if uri_bytes.is_empty() {
                        self.grid.hyperlink = None;
                    } else if let Ok(uri) = std::str::from_utf8(uri_bytes) {
                        self.grid.hyperlink = Some(uri.to_string());
                    }
                }
            } else if tag == b"133" {
                if let Some(body) = merged.get(1).copied() {
                    if let Some(marker) = body.first().copied() {
                        if let Some(ch) = char::from_u32(marker as u32) {
                            self.grid.prompt_mark = Some(ch);
                        }
                    }
                    // Look for status in subsequent params, e.g., C;0 or D;0.
                    if body.starts_with(b"C") || body.starts_with(b"D") {
                        if let Some(status_bytes) = merged.get(2).or_else(|| merged.get(1)) {
                            if let Ok(text) = std::str::from_utf8(status_bytes) {
                                if let Ok(code) = text.parse::<i32>() {
                                    self.grid.prompt_status = Some(code);
                                }
                            }
                        }
                    }
                }
            } else if tag == b"1337" {
                // Custom term UI helpers: OSC 1337;term;help=...;status=... ST
                if merged.get(1).map(|p| *p) == Some(b"term") {
                    for param in merged.iter().skip(2) {
                        if param.is_empty() {
                            continue;
                        }
                        if *param == b"clear" {
                            self.grid.help_text = None;
                            self.grid.status_text = None;
                            continue;
                        }
                        let Some(eq_idx) = param.iter().position(|b| *b == b'=') else {
                            continue;
                        };
                        let (key, value) = param.split_at(eq_idx);
                        let value = &value[1..];
                        let text = std::str::from_utf8(value).ok().unwrap_or("").to_string();
                        let text = if text.trim().is_empty() {
                            None
                        } else {
                            Some(text)
                        };
                        match key {
                            b"help" => self.grid.help_text = text,
                            b"status" => self.grid.status_text = text,
                            _ => {}
                        }
                    }
                }
            } else if tag == b"10" {
                if let Some(body) = merged.get(1).copied() {
                    if body == b"?" {
                        let fg = self.grid.default_fg();
                        let resp =
                            format!("\u{1b}]10;rgb:{:02x}/{:02x}/{:02x}\u{07}", fg.r, fg.g, fg.b);
                        write_bytes(&self.writer, resp.as_bytes());
                    } else if let Ok(text) = std::str::from_utf8(body) {
                        if let Some(fg) = parse_osc_color(text) {
                            self.grid.set_default_fg(fg);
                        }
                    }
                }
            } else if tag == b"11" {
                if let Some(body) = merged.get(1).copied() {
                    if body == b"?" {
                        let bg = self.grid.default_bg();
                        let resp =
                            format!("\u{1b}]11;rgb:{:02x}/{:02x}/{:02x}\u{07}", bg.r, bg.g, bg.b);
                        write_bytes(&self.writer, resp.as_bytes());
                    } else if let Ok(text) = std::str::from_utf8(body) {
                        if let Some(bg) = parse_osc_color(text) {
                            self.grid.set_default_bg(bg);
                        }
                    }
                }
            } else if tag == b"12" {
                if let Some(body) = merged.get(1).copied() {
                    if body == b"?" {
                        let cur = self.grid.cursor_color();
                        let resp = format!(
                            "\u{1b}]12;rgb:{:02x}/{:02x}/{:02x}\u{07}",
                            cur.r, cur.g, cur.b
                        );
                        write_bytes(&self.writer, resp.as_bytes());
                    } else if let Ok(text) = std::str::from_utf8(body) {
                        if let Some(cur) = parse_osc_color(text) {
                            self.grid.set_cursor_color(cur);
                        }
                    }
                }
            } else if tag == b"110" {
                self.grid.set_default_fg(DEFAULT_FG);
            } else if tag == b"111" {
                self.grid.set_default_bg(DEFAULT_BG);
            } else if tag == b"112" {
                self.grid.set_cursor_color(Rgba {
                    r: CURSOR[0],
                    g: CURSOR[1],
                    b: CURSOR[2],
                    a: CURSOR[3],
                });
            } else if tag == b"4" {
                let mut idx = 1;
                while idx < merged.len() {
                    if let Some(color_idx) = std::str::from_utf8(merged[idx])
                        .ok()
                        .and_then(|s| s.parse::<usize>().ok())
                    {
                        if let Some(spec) = merged.get(idx + 1) {
                            if let Ok(text) = std::str::from_utf8(spec) {
                                if let Some(color) = parse_osc_color(text) {
                                    self.grid.set_palette_color(color_idx, color);
                                } else if text.is_empty() {
                                    self.grid.reset_palette_indices(Some(vec![color_idx]));
                                }
                            }
                            idx += 2;
                            continue;
                        } else {
                            self.grid.reset_palette_indices(Some(vec![color_idx]));
                            idx += 1;
                            continue;
                        }
                    }
                    idx += 1;
                }
            } else if tag == b"104" {
                if merged.len() == 1 {
                    self.grid.reset_palette_indices(None);
                } else {
                    let mut indices = Vec::new();
                    for entry in merged.iter().skip(1) {
                        if let Some(idx) =
                            std::str::from_utf8(entry).ok().and_then(|s| s.parse().ok())
                        {
                            indices.push(idx);
                        }
                    }
                    self.grid.reset_palette_indices(Some(indices));
                }
            } else if tag == b"52" {
                // OSC 52 ; clipboard ; data ST
                let encoded = merged.get(2).copied().or_else(|| merged.get(1).copied());
                if let Some(encoded) = encoded {
                    if encoded.is_empty() {
                        if let Err(e) = write_clipboard_text("") {
                            log::error!("Failed to write clipboard text: {e}");
                        }
                    } else if let Ok(decoded) = general_purpose::STANDARD.decode(encoded) {
                        if let Ok(text) = String::from_utf8(decoded) {
                            if let Err(e) = write_clipboard_text(&text) {
                                log::error!("Failed to write clipboard text: {e}");
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn selection_text(term: &Terminal, a: (usize, usize), b: (usize, usize)) -> String {
    let (mut c0, mut r0) = a;
    let (mut c1, mut r1) = b;
    if r0 > r1 {
        std::mem::swap(&mut r0, &mut r1);
    }
    if c0 > c1 {
        std::mem::swap(&mut c0, &mut c1);
    }

    if r0 >= term.rows || r1 >= term.rows {
        return String::new();
    }

    let mut lines = Vec::new();
    for row in r0..=r1 {
        let mut line = String::new();
        for col in c0..=c1 {
            let cell = term.display_cell(col, row);
            if cell.wide_continuation {
                continue;
            }
            line.push_str(&cell.text);
        }
        while line.ends_with(' ') {
            line.pop();
        }
        lines.push(line);
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose;
    use once_cell::sync::Lazy;
    use std::io::Write;
    use std::sync::Mutex as StdMutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Clone, Default)]
    struct LockedBuf(Arc<Mutex<Vec<u8>>>);

    impl Write for LockedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn capture_writer() -> (Arc<Mutex<Vec<u8>>>, Arc<Mutex<Box<dyn Write + Send>>>) {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer: Box<dyn Write + Send> = Box::new(LockedBuf(buf.clone()));
        (buf, Arc::new(Mutex::new(writer)))
    }

    static CLIPBOARD_TEST_LOCK: Lazy<StdMutex<()>> = Lazy::new(|| StdMutex::new(()));

    #[test]
    fn insert_spaces_shifts_content() {
        let mut term = Terminal::new(5, 1);
        for ch in ['a', 'b', 'c'] {
            term.put_char(ch);
        }
        term.set_cursor(1, 0); // between a and b
        term.insert_spaces(2);

        assert_eq!(term.display_cell(0, 0).text, "a");
        assert_eq!(term.display_cell(1, 0).text, " ");
        assert_eq!(term.display_cell(2, 0).text, " ");
        assert_eq!(term.display_cell(3, 0).text, "b");
        assert_eq!(term.display_cell(4, 0).text, "c");
    }

    #[test]
    fn insert_spaces_clears_line_when_shift_exceeds_span() {
        let mut term = Terminal::new(3, 1);
        for ch in ['a', 'b', 'c'] {
            term.put_char(ch);
        }
        term.set_cursor(0, 0);
        term.insert_spaces(3);

        assert_eq!(term.display_cell(0, 0).text, " ");
        assert_eq!(term.display_cell(1, 0).text, " ");
        assert_eq!(term.display_cell(2, 0).text, " ");
    }

    #[test]
    fn delete_chars_sanitizes_wide_glyphs() {
        let mut term = Terminal::new(4, 1);
        term.put_char('界'); // wide -> occupies col 0 & 1
        term.put_char('x');
        term.put_char('y');
        term.set_cursor(0, 0);

        term.delete_chars(1);

        assert_eq!(term.display_cell(0, 0).text, " ");
        assert_eq!(term.display_cell(1, 0).text, "x");
        assert_eq!(term.display_cell(2, 0).text, "y");
        assert_eq!(term.display_cell(3, 0).text, " ");
    }

    #[test]
    fn clear_line_from_cursor_removes_wide_lead() {
        let mut term = Terminal::new(4, 1);
        term.put_char('A');
        term.put_char('界'); // occupies 1 & 2
        term.set_cursor(2, 0); // on wide continuation

        term.clear_line_from_cursor();

        assert_eq!(term.display_cell(0, 0).text, "A");
        assert_eq!(term.display_cell(1, 0).text, " ");
        assert_eq!(term.display_cell(2, 0).text, " ");
        assert_eq!(term.display_cell(3, 0).text, " ");
    }

    #[test]
    fn scroll_view_offset_clamps_to_scrollback_len() {
        let mut term = Terminal::new(2, 1);
        term.scrollback =
            vec![vec![Cell::blank_with(term.default_fg(), term.default_bg()); term.cols]; 3];
        term.set_view_offset(0);

        term.scroll_view_offset(10);
        assert_eq!(term.view_offset, 3);

        term.scroll_view_offset(-10);
        assert_eq!(term.view_offset, 0);
    }

    #[test]
    fn insert_lines_respects_margins() {
        let mut term = Terminal::new(3, 4);
        term.top_margin = 1;
        term.bottom_margin = 2;

        let mut fill_row = |r: usize, ch: char| {
            term.set_cursor(0, r);
            for _ in 0..term.cols {
                term.put_char(ch);
            }
        };

        fill_row(0, 'a');
        fill_row(1, 'b');
        fill_row(2, 'c');
        fill_row(3, 'd');

        term.set_cursor(1, 1);
        term.insert_lines(1);

        // Row 0 untouched, row 1 blank, row 2 shifted old row 1, row 3 untouched.
        assert_eq!(term.display_cell(0, 0).text, "a");
        assert_eq!(term.display_cell(0, 1).text, " ");
        assert_eq!(term.display_cell(1, 1).text, " ");
        assert_eq!(term.display_cell(2, 1).text, " ");

        assert_eq!(term.display_cell(0, 2).text, "b");
        assert_eq!(term.display_cell(1, 2).text, "b");
        assert_eq!(term.display_cell(2, 2).text, "b");

        assert_eq!(term.display_cell(0, 3).text, "d");
    }

    #[test]
    fn delete_lines_respects_margins() {
        let mut term = Terminal::new(3, 4);
        term.top_margin = 1;
        term.bottom_margin = 2;

        let mut fill_row = |r: usize, ch: char| {
            term.set_cursor(0, r);
            for _ in 0..term.cols {
                term.put_char(ch);
            }
        };

        fill_row(0, 'a');
        fill_row(1, 'b');
        fill_row(2, 'c');
        fill_row(3, 'd');

        term.set_cursor(1, 1);
        term.delete_lines(1);

        // Row 0 untouched, row 1 becomes old row 2, row 2 blanked, row 3 untouched.
        assert_eq!(term.display_cell(0, 0).text, "a");

        assert_eq!(term.display_cell(0, 1).text, "c");
        assert_eq!(term.display_cell(1, 1).text, "c");
        assert_eq!(term.display_cell(2, 1).text, "c");

        assert_eq!(term.display_cell(0, 2).text, " ");
        assert_eq!(term.display_cell(1, 2).text, " ");
        assert_eq!(term.display_cell(2, 2).text, " ");

        assert_eq!(term.display_cell(0, 3).text, "d");
    }

    #[test]
    fn selection_text_spans_multiple_rows() {
        let mut term = Terminal::new(4, 2);
        term.put_char('h');
        term.put_char('i');
        term.new_line();
        term.put_char('x');
        term.put_char('y');

        let text = selection_text(&term, (0, 0), (3, 1));
        assert_eq!(text, "hi\nxy");
    }

    #[test]
    fn selection_text_reads_scrollback_when_view_offset_set() {
        let mut term = Terminal::new(2, 2);
        // First line will end up in scrollback after overflow.
        term.put_char('a');
        term.new_line();
        term.put_char('b'); // pushes first line into scrollback

        // Pretend user scrolled up to view the first line.
        term.set_view_offset(1);

        let text = selection_text(&term, (0, 0), (0, 0));
        assert_eq!(
            text, "a",
            "selection should reflect visible scrollback contents"
        );
    }

    #[test]
    fn scrollback_preserves_all_emitted_lines() {
        // Writing sequential numbered lines should leave every line visible somewhere in
        // the combined scrollback + screen history.
        let mut term = Terminal::new(6, 3);
        let (buf, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        let mut parser = vte::Parser::new();

        let line_count = 12;
        for i in 0..line_count {
            let line = format!("L{i:02}\n");
            for b in line.as_bytes() {
                parser.advance(&mut performer, *b);
            }
        }

        // Stitch scrollback + visible rows into a single ordered transcript.
        let mut transcript: Vec<String> = term
            .scrollback
            .iter()
            .map(|row| {
                row.iter()
                    .filter(|c| !c.wide_continuation)
                    .map(|c| c.text.clone())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect();
        for r in 0..term.rows {
            let mut s = String::new();
            for c in 0..term.cols {
                let cell = term.cell(c, r);
                if cell.wide_continuation {
                    continue;
                }
                s.push_str(&cell.text);
            }
            transcript.push(s.trim_end().to_string());
        }

        let expected: Vec<String> = (0..line_count).map(|i| format!("L{i:02}")).collect();
        assert!(
            transcript.starts_with(&expected),
            "history should retain every emitted line; got {:?}",
            transcript
        );
        assert!(
            buf.lock().unwrap().is_empty(),
            "should not echo any replies"
        );
    }

    #[test]
    fn dcs_sixel_sequence_creates_sprite_at_cursor() {
        let mut term = Terminal::new(10, 6);
        term.set_cursor(3, 4);
        let (_buf, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        let mut parser = vte::Parser::new();
        // Simple two-column sixel image from icy_sixel tests.
        let sixel = b"\x1bPq\"1;1;2;2#0;2;0;0;0#0~~\x1b\\";
        for b in sixel {
            parser.advance(&mut performer, *b);
        }

        let sprite = term.sixels.first().expect("sixel sprite present");
        assert_eq!(sprite.col, 3);
        assert_eq!(sprite.row, 4);
        assert_eq!(sprite.width, 2);
        assert!(
            sprite.height >= 6,
            "expected at least one sixel row, got {}",
            sprite.height
        );
        assert_eq!(
            sprite.pixels.len(),
            sprite.width * sprite.height * 4,
            "pixels should be RGBA"
        );
    }

    #[test]
    fn sixels_clear_on_scroll_changes() {
        let mut term = Terminal::new(5, 5);
        let (_buf, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        let mut parser = vte::Parser::new();
        let sixel = b"\x1bPq#0!5~\x1b\\";
        for b in sixel {
            parser.advance(&mut performer, *b);
        }
        assert_eq!(term.sixels.len(), 1, "sixel decoded");

        term.scroll_up_region(1);
        assert!(
            term.sixels.is_empty(),
            "scrolling should clear sixel overlays for now"
        );
    }

    #[test]
    fn bracketed_mode_newlines_still_scroll() {
        // Bracketed paste mode is often left on by shells; hitting the bottom row must
        // still push lines into scrollback instead of discarding output.
        let mut term = Terminal::new(3, 2);
        term.bracketed_paste = true;

        term.put_char('A');
        term.new_line();
        term.put_char('B');
        term.new_line(); // hits bottom row and should scroll like a normal newline

        assert_eq!(
            term.scrollback.len(),
            1,
            "first line should move to scrollback"
        );
        assert_eq!(term.scrollback[0][0].text, "A");
        assert_eq!(
            term.display_cell(0, 0).text,
            "B",
            "next line should remain visible"
        );
    }

    #[test]
    fn carriage_return_clears_line_for_rewrite() {
        let mut term = Terminal::new(10, 1);
        for ch in "previous".chars() {
            term.put_char(ch);
        }
        term.carriage_return();
        for ch in "hi".chars() {
            term.put_char(ch);
        }

        let mut line = String::new();
        for c in 0..term.cols {
            line.push_str(&term.display_cell(c, 0).text);
        }
        assert!(line.starts_with("hi"));
        assert!(line[2..].chars().all(|c| c == ' '));
    }

    #[test]
    fn history_rewrite_replaces_content_multiple_times() {
        let mut term = Terminal::new(16, 1);

        // Initial line
        for ch in "first-command".chars() {
            term.put_char(ch);
        }

        // Shell rewrites line with previous history entry.
        term.carriage_return();
        for ch in "second".chars() {
            term.put_char(ch);
        }

        // And again with another entry.
        term.carriage_return();
        for ch in "third".chars() {
            term.put_char(ch);
        }

        // Final line should only contain the last rewrite ("third") plus blanks.
        let mut line = String::new();
        for c in 0..term.cols {
            line.push_str(&term.display_cell(c, 0).text);
        }
        assert!(line.starts_with("third"));
        assert!(line[5..].chars().all(|c| c == ' '));
    }

    #[test]
    fn bracketed_prompt_newlines_push_scrollback() {
        let mut term = Terminal::new(8, 2);
        term.bracketed_paste = true;
        term.cursor_row = term.rows.saturating_sub(1);
        term.cursor_col = 0;

        term.new_line();

        assert_eq!(
            term.scrollback.len(),
            1,
            "should still push scrollback even when bracketed paste is enabled"
        );
        assert_eq!(term.cursor_row, term.rows.saturating_sub(1));
    }

    #[test]
    fn bash_reverse_search_output_does_not_scroll() {
        let mut term = Terminal::new(80, 4);
        let mut parser = vte::Parser::new();
        let mut app_cursor_keys = false;
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(std::io::sink())));
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };

        // Bytes captured from bash reverse-i-search prompt (Ctrl+R then typing "a").
        let seq = b"\x1b[?2004hbash-5.3$ \r(reverse-i-search)`': \x08\x08\x08a': : 1765388540:0;cle\x1b[7ma\x1b[27mr\x08\x08\r\x1b[13Pbash-5.3$ : 1765388540:0;clea\x08";
        for &b in seq {
            parser.advance(&mut performer, b);
        }

        assert_eq!(
            term.scrollback.len(),
            0,
            "reverse-i-search updates should not push scrollback on each key"
        );
    }

    #[test]
    fn focus_reporting_sends_sequences_when_enabled() {
        let mut term = Terminal::new(2, 1);
        let (buf, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer: writer.clone(),
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        let mut parser = vte::Parser::new();

        for byte in b"\x1b[?1004h" {
            parser.advance(&mut performer, *byte);
        }

        term.report_focus(true, &writer);
        term.report_focus(false, &writer);

        let data = buf.lock().unwrap().clone();
        assert_eq!(data, b"\x1b[I\x1b[O");
    }

    #[test]
    fn focus_reporting_is_quiet_when_disabled() {
        let term = Terminal::new(2, 1);
        let (buf, writer) = capture_writer();

        term.report_focus(true, &writer);
        term.report_focus(false, &writer);

        assert!(buf.lock().unwrap().is_empty());
    }

    #[test]
    fn osc_2_updates_window_title() {
        let mut term = Terminal::new(3, 1);
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(std::io::sink())));
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        let mut parser = vte::Parser::new();

        for b in b"\x1b]2;hello world\x07" {
            parser.advance(&mut performer, *b);
        }

        assert_eq!(term.window_title(), Some("hello world"));
    }

    #[test]
    fn osc_1_updates_icon_title_only() {
        let mut term = Terminal::new(3, 1);
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(std::io::sink())));
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        let mut parser = vte::Parser::new();

        for b in b"\x1b]1;iconic\x07" {
            parser.advance(&mut performer, *b);
        }

        assert_eq!(term.icon_title(), Some("iconic"));
        assert_eq!(term.window_title(), None);
    }

    #[test]
    fn osc_0_sets_both_titles() {
        let mut term = Terminal::new(3, 1);
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(std::io::sink())));
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        let mut parser = vte::Parser::new();

        for b in b"\x1b]0;combo\x07" {
            parser.advance(&mut performer, *b);
        }

        assert_eq!(term.window_title(), Some("combo"));
        assert_eq!(term.icon_title(), Some("combo"));
    }

    #[test]
    fn dectcem_toggles_cursor_visibility() {
        let mut term = Terminal::new(2, 1);
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(std::io::sink())));
        let mut app_cursor_keys = false;
        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer: writer.clone(),
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            let mut parser = vte::Parser::new();
            for b in b"\x1b[?25l" {
                parser.advance(&mut performer, *b);
            }
        }
        assert!(!term.cursor_visible());

        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer: writer.clone(),
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            let mut parser = vte::Parser::new();
            for b in b"\x1b[?25h" {
                parser.advance(&mut performer, *b);
            }
        }
        assert!(term.cursor_visible());
    }

    #[test]
    fn decscusr_sets_cursor_shapes() {
        let mut term = Terminal::new(2, 1);
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(std::io::sink())));
        let mut app_cursor_keys = false;
        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer: writer.clone(),
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            let mut parser = vte::Parser::new();
            for b in b"\x1b[0 q" {
                parser.advance(&mut performer, *b);
            }
        }
        assert_eq!(term.cursor_shape(), CursorShape::Block);

        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer: writer.clone(),
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            let mut parser = vte::Parser::new();
            for b in b"\x1b[4 q" {
                parser.advance(&mut performer, *b);
            }
        }
        assert_eq!(term.cursor_shape(), CursorShape::Underline);

        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer: writer.clone(),
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            let mut parser = vte::Parser::new();
            for b in b"\x1b[6 q" {
                parser.advance(&mut performer, *b);
            }
        }
        assert_eq!(term.cursor_shape(), CursorShape::Bar);
    }

    #[test]
    fn osc52_sets_clipboard_from_base64() {
        let _guard = CLIPBOARD_TEST_LOCK.lock().unwrap();
        let captured = Arc::new(Mutex::new(String::new()));
        set_clipboard_hook({
            let captured = captured.clone();
            move |text| {
                *captured.lock().unwrap() = text.to_string();
            }
        });

        let mut term = Terminal::new(4, 1);
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(std::io::sink())));
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        let mut parser = vte::Parser::new();

        let payload = general_purpose::STANDARD.encode("hello");
        let seq = format!("\x1b]52;c;{}\x07", payload);
        for b in seq.as_bytes() {
            parser.advance(&mut performer, *b);
        }

        assert_eq!(captured.lock().unwrap().as_str(), "hello");
    }

    #[test]
    fn sgr_faint_and_strike_and_overline_toggle() {
        let mut term = Terminal::new(4, 1);
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(std::io::sink())));
        let mut app_cursor_keys = false;
        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer: writer.clone(),
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            let mut parser = vte::Parser::new();
            for b in b"\x1b[2;9;53mX" {
                parser.advance(&mut performer, *b);
            }
        }
        let cell = term.display_cell(0, 0);
        assert!(cell.faint);
        assert!(cell.strike);
        assert!(cell.overline);

        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer,
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            let mut parser = vte::Parser::new();
            for b in b"\x1b[22;29;55mY" {
                parser.advance(&mut performer, *b);
            }
        }
        let cell = term.display_cell(1, 0);
        assert!(!cell.faint);
        assert!(!cell.strike);
        assert!(!cell.overline);
    }

    #[test]
    fn osc52_ignores_invalid_base64() {
        let _guard = CLIPBOARD_TEST_LOCK.lock().unwrap();
        let captured = Arc::new(Mutex::new(String::new()));
        set_clipboard_hook({
            let captured = captured.clone();
            move |text| {
                *captured.lock().unwrap() = text.to_string();
            }
        });

        let mut term = Terminal::new(4, 1);
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(std::io::sink())));
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        let mut parser = vte::Parser::new();

        let seq = "\x1b]52;c;@@@\x07";
        for b in seq.as_bytes() {
            parser.advance(&mut performer, *b);
        }

        assert_eq!(captured.lock().unwrap().as_str(), "");
    }

    #[test]
    fn origin_mode_offsets_home_row_inside_scroll_region() {
        let mut term = Terminal::new(5, 5);
        let (buf, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer: writer.clone(),
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        let mut parser = vte::Parser::new();

        // Set scroll region to rows 1..3, enable origin mode, then home.
        for b in b"\x1b[2;4r\x1b[?6h\x1b[H" {
            parser.advance(&mut performer, *b);
        }

        assert_eq!(term.top_margin, 1);
        assert_eq!(term.bottom_margin, 3);
        assert_eq!(
            term.cursor_row, 1,
            "home should land at top margin with DECOM"
        );
        assert_eq!(term.cursor_col, 0);
        assert!(buf.lock().unwrap().is_empty());
    }

    #[test]
    fn scroll_region_scrolls_only_inside_margins() {
        let mut term = Terminal::new(3, 3);
        // Preload visible content.
        term.set_cursor(0, 0);
        term.put_char('A'); // row 0
        term.set_cursor(0, 1);
        term.put_char('B'); // row 1 (inside region)
        term.set_cursor(0, 2);
        term.put_char('C'); // row 2 (inside region)

        term.set_scroll_region(1, 2);
        term.set_cursor(0, 2); // bottom of region

        term.new_line(); // should scroll region up by one

        assert_eq!(
            term.display_cell(0, 0).text,
            "A",
            "row 0 should stay untouched"
        );
        assert_eq!(
            term.display_cell(0, 1).text,
            "C",
            "row 1 should now contain previous bottom row"
        );
        assert_eq!(
            term.display_cell(0, 2).text,
            " ",
            "bottom of region should be cleared after scroll"
        );
    }

    #[test]
    fn save_restore_cursor_recovers_position_and_attributes() {
        let mut term = Terminal::new(4, 2);
        term.set_cursor(2, 1);
        term.set_fg(Rgba {
            r: 10,
            g: 20,
            b: 30,
            a: 255,
        });
        term.set_bg(Rgba {
            r: 5,
            g: 6,
            b: 7,
            a: 255,
        });
        term.pen_inverse = true;
        term.pen_bold = true;
        term.pen_italic = true;
        term.pen_underline = true;
        term.origin_mode = true;
        term.wrap_next = true;

        term.save_cursor();

        // Mutate state.
        term.set_cursor(0, 0);
        term.reset_style();
        term.origin_mode = false;
        term.wrap_next = false;

        term.restore_cursor();

        assert_eq!(term.cursor_col, 2);
        assert_eq!(term.cursor_row, 1);
        assert_eq!(
            term.pen_fg,
            Rgba {
                r: 10,
                g: 20,
                b: 30,
                a: 255
            }
        );
        assert_eq!(
            term.pen_bg,
            Rgba {
                r: 5,
                g: 6,
                b: 7,
                a: 255
            }
        );
        assert!(term.pen_inverse);
        assert!(term.pen_bold);
        assert!(term.pen_italic);
        assert!(term.pen_underline);
        assert!(term.origin_mode);
        assert!(term.wrap_next);
    }

    #[test]
    fn mouse_mode_flags_toggle_with_decset() {
        let mut term = Terminal::new(2, 1);
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(std::io::sink())));
        let mut app_cursor_keys = false;
        let mut parser = vte::Parser::new();

        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer,
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };

            for b in b"\x1b[?1000h\x1b[?1006h\x1b[?1002h" {
                parser.advance(&mut performer, *b);
            }
        }

        assert!(term.mouse_btn_report);
        assert!(term.mouse_motion_report);
        assert!(term.mouse_sgr, "1006h should enable SGR mode");

        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer: Arc::new(Mutex::new(Box::new(std::io::sink()))),
                app_cursor_keys: &mut false,
                dcs_state: None,
            };
            for b in b"\x1b[?1000l\x1b[?1002l\x1b[?1006l" {
                parser.advance(&mut performer, *b);
            }
        }
        assert!(!term.mouse_btn_report);
        assert!(!term.mouse_motion_report);
        assert!(!term.mouse_sgr);
    }

    #[test]
    fn mouse_report_sgr_press_release() {
        let mut term = Terminal::new(10, 10);
        let (buf, writer) = capture_writer();
        term.mouse_btn_report = true;
        term.mouse_sgr = true;

        term.report_mouse_event(4, 5, MouseEventKind::Press(0), &writer);
        term.report_mouse_event(4, 5, MouseEventKind::Release, &writer);

        let output = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert_eq!(output, "\x1b[<0;5;6M\x1b[<3;5;6m");
    }

    #[test]
    fn mouse_report_x10_basic_press() {
        let mut term = Terminal::new(10, 10);
        let (buf, writer) = capture_writer();
        term.mouse_btn_report = true;
        term.mouse_sgr = false;

        term.report_mouse_event(1, 1, MouseEventKind::Press(0), &writer);

        let data = buf.lock().unwrap().clone();
        assert_eq!(data, vec![0x1b, b'[', b'M', 32, 34, 34]);
    }

    #[test]
    fn mouse_report_utf8_encoding() {
        let mut term = Terminal::new(400, 400);
        let (buf, writer) = capture_writer();
        term.mouse_btn_report = true;
        term.mouse_sgr = false;
        term.mouse_utf8 = true;

        // Position large enough to require multi-byte UTF-8 after +32.
        term.report_mouse_event(300, 300, MouseEventKind::Press(0), &writer);

        let data = buf.lock().unwrap().clone();
        assert!(data.starts_with(&[0x1b, b'[', b'M']));
        // Decode the UTF-8 pieces for code, x, y.
        let tail = &data[3..];
        let decoded = std::str::from_utf8(tail).unwrap();
        // We expect three Unicode scalars; capture their codepoints.
        let scalars: Vec<u32> = decoded.chars().map(|c| c as u32).collect();
        assert_eq!(scalars.len(), 3);
        // Values should be 32+code, 32+x+1, 32+y+1
        assert_eq!(scalars[0], 32);
        assert_eq!(scalars[1], 32 + 301);
        assert_eq!(scalars[2], 32 + 301);
    }

    #[test]
    fn kitty_keyboard_enable_toggles_flag() {
        let mut term = Terminal::new(2, 1);
        let (_, writer) = capture_writer();
        let mut app_cursor_keys = false;
        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer: writer.clone(),
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            performer.osc_dispatch(&[b"1", b"2"], false);
        }
        assert!(term.kitty_keyboard);
        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer,
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            performer.osc_dispatch(&[b"1", b"0"], false);
        }
        assert!(!term.kitty_keyboard);
    }

    #[test]
    fn osc8_hyperlink_is_ignored_safely() {
        let mut term = Terminal::new(2, 1);
        let (buf, writer) = capture_writer();
        let mut app_cursor_keys = false;
        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer: writer.clone(),
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            performer.osc_dispatch(&[b"8", b"", b"https://example.com"], false);
        }

        // No output should be emitted, and grid unchanged except text cells.
        assert!(buf.lock().unwrap().is_empty());
        assert_eq!(
            term.hyperlink.as_deref(),
            Some("https://example.com"),
            "hyperlink should be tracked while active"
        );

        // Closing tag clears hyperlink.
        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer,
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            performer.osc_dispatch(&[b"8", b"", b""], false);
        }
        assert!(term.hyperlink.is_none());
    }

    #[test]
    fn osc8_applies_hyperlink_to_subsequent_text() {
        let mut term = Terminal::new(2, 1);
        let (_, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        performer.osc_dispatch(&[b"8", b"", b"https://example.com"], false);
        performer.print('X');

        let cell = term.display_cell(0, 0);
        assert_eq!(cell.text, "X");
        assert_eq!(cell.hyperlink.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn osc7_sets_cwd_state() {
        let mut term = Terminal::new(2, 1);
        let (_, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        let mut parser = vte::Parser::new();

        let seq = b"\x1b]7;file:///home/test\x07";
        for b in seq {
            parser.advance(&mut performer, *b);
        }

        assert_eq!(term.cwd_state, Some("file:///home/test".to_string()));
    }

    #[test]
    fn osc133_markers_toggle_state() {
        let mut term = Terminal::new(2, 1);
        let (_, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut parser = vte::Parser::new();

        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer: writer.clone(),
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            for b in b"\x1b]133;A\x07" {
                parser.advance(&mut performer, *b);
            }
        }
        assert_eq!(term.prompt_mark, Some('A'));

        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer,
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            for b in b"\x1b]133;D;0\x07" {
                parser.advance(&mut performer, *b);
            }
        }
        assert_eq!(term.prompt_mark, Some('D'));
        assert_eq!(term.prompt_status, Some(0));
    }

    #[test]
    fn osc11_updates_default_background_and_cells() {
        let mut term = Terminal::new(4, 2);
        term.put_char('a');
        term.new_line();
        term.put_char('b');

        let osc = b"\x1b]11;#112233\x07";
        let (_, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut parser = vte::Parser::new();
        {
            let mut performer = GridPerformer {
                grid: &mut term,
                writer,
                app_cursor_keys: &mut app_cursor_keys,
                dcs_state: None,
            };
            for b in osc {
                parser.advance(&mut performer, *b);
            }
        }

        let bg = term.default_bg();
        assert_eq!(
            bg,
            Rgba {
                r: 0x11,
                g: 0x22,
                b: 0x33,
                a: 255
            }
        );
        assert_eq!(term.cell(0, 0).bg, bg);
        assert_eq!(term.cell(0, 1).bg, bg);

        term.new_line();
        term.put_char('c');
        assert_eq!(term.cell(0, term.cursor_row).bg, bg);
    }

    #[test]
    fn osc11_query_reports_current_background() {
        let mut term = Terminal::new(2, 1);
        let (buf, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut parser = vte::Parser::new();
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };

        for b in b"\x1b]11;#010203\x07" {
            parser.advance(&mut performer, *b);
        }
        buf.lock().unwrap().clear();

        for b in b"\x1b]11;?\x07" {
            parser.advance(&mut performer, *b);
        }

        let out = buf.lock().unwrap().clone();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "\u{1b}]11;rgb:01/02/03\u{07}"
        );
    }

    #[test]
    fn osc10_and_110_manage_foreground() {
        let mut term = Terminal::new(2, 1);
        let (_, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut parser = vte::Parser::new();
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };

        for b in b"\x1b]10;#0a0b0c\x07" {
            parser.advance(&mut performer, *b);
        }
        assert_eq!(
            performer.grid.default_fg(),
            Rgba {
                r: 0x0a,
                g: 0x0b,
                b: 0x0c,
                a: 255
            }
        );

        for b in b"\x1b]110\x07" {
            parser.advance(&mut performer, *b);
        }
        assert_eq!(performer.grid.default_fg(), DEFAULT_FG);
    }

    #[test]
    fn osc111_resets_background_to_default() {
        let mut term = Terminal::new(2, 1);
        let (_, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut parser = vte::Parser::new();
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };

        for b in b"\x1b]11;#112233\x07" {
            parser.advance(&mut performer, *b);
        }
        assert_eq!(
            performer.grid.default_bg(),
            Rgba {
                r: 0x11,
                g: 0x22,
                b: 0x33,
                a: 255
            }
        );

        for b in b"\x1b]111\x07" {
            parser.advance(&mut performer, *b);
        }
        assert_eq!(term.default_bg(), DEFAULT_BG);
    }

    #[test]
    fn osc12_and_112_manage_cursor_color() {
        let mut term = Terminal::new(2, 1);
        let (_, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut parser = vte::Parser::new();
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };

        for b in b"\x1b]12;#0f0e0d\x07" {
            parser.advance(&mut performer, *b);
        }
        assert_eq!(
            performer.grid.cursor_color(),
            Rgba {
                r: 0x0f,
                g: 0x0e,
                b: 0x0d,
                a: 255
            }
        );

        for b in b"\x1b]112\x07" {
            parser.advance(&mut performer, *b);
        }
        assert_eq!(
            performer.grid.cursor_color(),
            Rgba {
                r: CURSOR[0],
                g: CURSOR[1],
                b: CURSOR[2],
                a: CURSOR[3],
            }
        );
    }

    #[test]
    fn osc52_writes_clipboard() {
        let _guard = CLIPBOARD_TEST_LOCK.lock().unwrap();
        static HIT: AtomicBool = AtomicBool::new(false);
        let last: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let last_hook = last.clone();
        set_clipboard_hook(move |text| {
            HIT.store(true, Ordering::Relaxed);
            *last_hook.lock().unwrap() = Some(text.to_string());
        });

        let mut term = Terminal::new(2, 1);
        let (_, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };

        let payload = general_purpose::STANDARD.encode("copy me");
        // Prefix a different payload first to ensure later data wins.
        performer.osc_dispatch(&[b"52", b"c", b"aGVsbG8="], false);
        performer.osc_dispatch(&[b"52", b"c", payload.as_bytes()], false);

        assert!(HIT.load(Ordering::Relaxed));
        let final_text = last.lock().unwrap().clone().unwrap();
        assert_eq!(final_text, "copy me");
    }

    #[test]
    fn osc4_sets_palette_and_sgr_uses_it() {
        let mut term = Terminal::new(2, 1);
        let (_, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut parser = vte::Parser::new();
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };

        for b in b"\x1b]4;1;#010203\x07" {
            parser.advance(&mut performer, *b);
        }
        for b in b"\x1b[31m" {
            parser.advance(&mut performer, *b);
        }
        assert_eq!(
            performer.grid.pen_fg,
            Rgba {
                r: 0x01,
                g: 0x02,
                b: 0x03,
                a: 255
            }
        );
    }

    #[test]
    fn osc104_resets_palette_indices() {
        let mut term = Terminal::new(2, 1);
        let (_, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut parser = vte::Parser::new();
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };

        for b in b"\x1b]4;1;#010203\x07" {
            parser.advance(&mut performer, *b);
        }
        for b in b"\x1b]104;1\x07" {
            parser.advance(&mut performer, *b);
        }
        for b in b"\x1b[31m" {
            parser.advance(&mut performer, *b);
        }
        assert_eq!(performer.grid.pen_fg, xterm_color(1));
    }

    #[test]
    fn device_attributes_report() {
        let mut term = Terminal::new(2, 1);
        let (buf, writer) = capture_writer();
        let mut app_cursor_keys = false;
        let mut parser = vte::Parser::new();
        let mut performer = GridPerformer {
            grid: &mut term,
            writer,
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };

        for b in b"\x1b[c" {
            parser.advance(&mut performer, *b);
        }
        assert_eq!(buf.lock().unwrap().as_slice(), b"\x1b[?1;2c");
    }
}
