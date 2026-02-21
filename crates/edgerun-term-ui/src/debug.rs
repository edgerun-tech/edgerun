use std::fmt;
use std::time::{Duration, Instant};

use log::debug;

use term_core::terminal::{DEFAULT_BG, DEFAULT_FG, Terminal};

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
    benchmark: BenchmarkState,
}

#[derive(Clone, Copy, Debug)]
pub struct BenchmarkSnapshot {
    pub active: bool,
    pub elapsed: Duration,
    pub duration: Duration,
    pub frames: u32,
    pub avg_ms: f32,
    pub max_ms: f32,
    pub last_frames: u32,
    pub last_duration: Duration,
    pub last_avg_ms: f32,
    pub last_max_ms: f32,
}

#[derive(Clone, Copy, Debug)]
struct BenchmarkState {
    active: bool,
    start: Instant,
    duration: Duration,
    frames: u32,
    total_ms: f32,
    max_ms: f32,
    last_frames: u32,
    last_duration: Duration,
    last_avg_ms: f32,
    last_max_ms: f32,
}

impl BenchmarkState {
    fn new(now: Instant) -> Self {
        Self {
            active: false,
            start: now,
            duration: Duration::from_secs(0),
            frames: 0,
            total_ms: 0.0,
            max_ms: 0.0,
            last_frames: 0,
            last_duration: Duration::from_secs(0),
            last_avg_ms: 0.0,
            last_max_ms: 0.0,
        }
    }

    fn start(&mut self, now: Instant, duration: Duration) {
        self.active = true;
        self.start = now;
        self.duration = duration;
        self.frames = 0;
        self.total_ms = 0.0;
        self.max_ms = 0.0;
    }

    fn stop(&mut self, now: Instant) {
        if !self.active {
            return;
        }
        let elapsed = now.saturating_duration_since(self.start);
        let avg = if self.frames > 0 {
            self.total_ms / self.frames as f32
        } else {
            0.0
        };
        self.last_frames = self.frames;
        self.last_duration = elapsed;
        self.last_avg_ms = avg;
        self.last_max_ms = self.max_ms;
        self.active = false;
    }

    fn record(&mut self, now: Instant, render_ms: f32) -> bool {
        if !self.active {
            return false;
        }
        self.frames = self.frames.saturating_add(1);
        self.total_ms += render_ms;
        if render_ms > self.max_ms {
            self.max_ms = render_ms;
        }
        if now.duration_since(self.start) >= self.duration {
            self.stop(now);
            return true;
        }
        false
    }

    fn snapshot(&self, now: Instant) -> BenchmarkSnapshot {
        if self.active {
            let elapsed = now.saturating_duration_since(self.start);
            let avg = if self.frames > 0 {
                self.total_ms / self.frames as f32
            } else {
                0.0
            };
            BenchmarkSnapshot {
                active: true,
                elapsed,
                duration: self.duration,
                frames: self.frames,
                avg_ms: avg,
                max_ms: self.max_ms,
                last_frames: self.last_frames,
                last_duration: self.last_duration,
                last_avg_ms: self.last_avg_ms,
                last_max_ms: self.last_max_ms,
            }
        } else {
            BenchmarkSnapshot {
                active: false,
                elapsed: Duration::from_secs(0),
                duration: Duration::from_secs(0),
                frames: 0,
                avg_ms: 0.0,
                max_ms: 0.0,
                last_frames: self.last_frames,
                last_duration: self.last_duration,
                last_avg_ms: self.last_avg_ms,
                last_max_ms: self.last_max_ms,
            }
        }
    }
}

impl DebugOverlay {
    pub fn new(cols: usize, rows: usize) -> Self {
        let now = Instant::now();
        let mut overlay = Self {
            active: false,
            render_mode: DebugRenderMode::Auto,
            input_mode: DebugInputMode::Normal,
            last_used_renderer: None,
            preview: Terminal::new(cols, rows),
            size: (cols, rows),
            benchmark: BenchmarkState::new(now),
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

    pub fn set_render_mode(&mut self, mode: DebugRenderMode) {
        if self.render_mode != mode {
            self.render_mode = mode;
            self.rebuild_preview();
            debug!("debug render mode -> {}", self.render_mode);
        }
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

    pub fn benchmark_active(&self) -> bool {
        self.benchmark.active
    }

    pub fn toggle_benchmark(&mut self, now: Instant, duration: Duration) {
        if self.benchmark.active {
            self.benchmark.stop(now);
        } else {
            self.benchmark.start(now, duration);
        }
    }

    pub fn record_benchmark_frame(&mut self, now: Instant, render_ms: f32) -> bool {
        self.benchmark.record(now, render_ms)
    }

    pub fn benchmark_snapshot(&self, now: Instant) -> BenchmarkSnapshot {
        self.benchmark.snapshot(now)
    }

    fn rebuild_preview(&mut self) {
        self.preview = Terminal::new(self.size.0, self.size.1);
        self.preview.pen_fg = DEFAULT_FG;
        self.preview.pen_bg = DEFAULT_BG;
    }
}
