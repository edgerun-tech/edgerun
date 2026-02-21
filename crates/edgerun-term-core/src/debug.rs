use std::fmt;

use log::debug;

use crate::terminal::{DEFAULT_BG, DEFAULT_FG, Terminal};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DebugRenderMode {
    Auto,
    CpuOnly,
    GpuOnly,
}

impl fmt::Display for DebugRenderMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::CpuOnly => write!(f, "cpu"),
            Self::GpuOnly => write!(f, "gpu"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DebugInputMode {
    Normal,
    AlternateScreen,
    BracketedPaste,
}

impl fmt::Display for DebugInputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::AlternateScreen => write!(f, "alt-screen"),
            Self::BracketedPaste => write!(f, "bracketed-paste"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DebugRendererUsed {
    Cpu,
    Gpu,
}

impl fmt::Display for DebugRendererUsed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cpu => write!(f, "cpu"),
            Self::Gpu => write!(f, "gpu"),
        }
    }
}

pub struct DebugOverlay {
    active: bool,
    render_mode: DebugRenderMode,
    input_mode: DebugInputMode,
    last_used_renderer: Option<DebugRendererUsed>,
    preview: Terminal,
    size: (usize, usize),
}

impl DebugOverlay {
    pub fn new(cols: usize, rows: usize) -> Self {
        let mut overlay = Self {
            active: false,
            render_mode: DebugRenderMode::Auto,
            input_mode: DebugInputMode::Normal,
            last_used_renderer: None,
            preview: Terminal::new(cols, rows),
            size: (cols, rows),
        };
        overlay.rebuild_preview();
        overlay
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let (old_c, old_r) = self.size;
        if old_c == cols && old_r == rows {
            return;
        }
        self.size = (cols, rows);
        self.preview = Terminal::new(cols, rows);
        self.rebuild_preview();
        debug!("debug overlay resized to {}x{}", cols, rows);
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
        debug!(
            "debug overlay {}",
            if self.active { "opened" } else { "closed" }
        );
    }

    pub fn close(&mut self) {
        if self.active {
            self.active = false;
            debug!("debug overlay closed");
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn render_mode(&self) -> DebugRenderMode {
        self.render_mode
    }

    pub fn input_mode(&self) -> DebugInputMode {
        self.input_mode
    }

    pub fn cycle_render_mode(&mut self) {
        self.render_mode = match self.render_mode {
            DebugRenderMode::Auto => DebugRenderMode::CpuOnly,
            DebugRenderMode::CpuOnly => DebugRenderMode::GpuOnly,
            DebugRenderMode::GpuOnly => DebugRenderMode::Auto,
        };
        self.rebuild_preview();
        debug!("debug render mode -> {}", self.render_mode);
    }

    pub fn cycle_input_mode(&mut self) {
        self.input_mode = match self.input_mode {
            DebugInputMode::Normal => DebugInputMode::AlternateScreen,
            DebugInputMode::AlternateScreen => DebugInputMode::BracketedPaste,
            DebugInputMode::BracketedPaste => DebugInputMode::Normal,
        };
        self.rebuild_preview();
        debug!("debug input mode -> {}", self.input_mode);
    }

    pub fn record_renderer(&mut self, renderer: DebugRendererUsed) {
        self.last_used_renderer = Some(renderer);
    }

    pub fn last_used_renderer(&self) -> Option<DebugRendererUsed> {
        self.last_used_renderer
    }

    pub fn preview(&self) -> &Terminal {
        &self.preview
    }

    fn rebuild_preview(&mut self) {
        self.preview = Terminal::new(self.size.0, self.size.1);
        self.preview.pen_fg = DEFAULT_FG;
        self.preview.pen_bg = DEFAULT_BG;
    }
}
