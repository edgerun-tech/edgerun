use term_core::gpu::{GlyphAtlas, GlyphVertex, GpuRenderer, RectVertex};
use term_core::render::{
    GlyphCache, OVERLAY_ACCENT, OVERLAY_DIM, OVERLAY_PANEL, OVERLAY_PANEL_INNER, OVERLAY_TEXT,
    OVERLAY_TEXT_MUTED, draw_text_line_clipped, rgba_bytes,
};
use term_core::render::primitives::fill_rect;
use crate::widgets::{
    MODAL_PANEL_H_FRAC, MODAL_PANEL_MIN_H, MODAL_PANEL_MIN_W, MODAL_PANEL_W_FRAC, PanelLayout,
    list_panel_cpu, list_panel_gpu, modal_panel_rect,
};
use pixels::wgpu;

#[derive(Clone, Debug)]
pub struct LogSourceEntry {
    pub label: String,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFocus {
    Sources,
    Logs,
}

pub struct LogViewer {
    pub open: bool,
    pub sources: Vec<LogSourceEntry>,
    pub selected: usize,
    pub lines: Vec<String>,
    pub status: String,
    pub query: String,
    pub input: String,
    pub follow: bool,
    pub sudo: bool,
    pub scroll: usize,
    pub focus: LogFocus,
    pub editing: bool,
}

impl LogViewer {
    pub fn new() -> Self {
        Self {
            open: false,
            sources: Vec::new(),
            selected: 0,
            lines: Vec::new(),
            status: String::new(),
            query: String::new(),
            input: String::new(),
            follow: true,
            sudo: false,
            scroll: 0,
            focus: LogFocus::Sources,
            editing: false,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.editing = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn set_sources(&mut self, sources: Vec<LogSourceEntry>) {
        self.sources = sources;
        if self.selected >= self.sources.len() {
            self.selected = 0;
        }
    }
}

pub fn draw_log_viewer_cpu(
    viewer: &LogViewer,
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
) {
    if !viewer.open || width == 0 || height == 0 {
        return;
    }

    fill_rect(
        frame,
        width,
        height,
        0,
        0,
        width as i32,
        height as i32,
        rgba_bytes(OVERLAY_DIM),
    );

    let (x0, y0, x1, y1) = modal_panel_rect(
        width,
        height,
        MODAL_PANEL_W_FRAC,
        MODAL_PANEL_H_FRAC,
        MODAL_PANEL_MIN_W,
        MODAL_PANEL_MIN_H,
    );
    fill_rect(frame, width, height, x0, y0, x1, y1, rgba_bytes(OVERLAY_PANEL));
    fill_rect(
        frame,
        width,
        height,
        x0 + 1,
        y0 + 1,
        x1 - 1,
        y1 - 1,
        rgba_bytes(OVERLAY_PANEL_INNER),
    );

    let line_h = glyphs.cell_height() as i32 + 6;
    let heading = "Log Viewer — F5/Esc close";
    let heading_x = x0 + 16;
    let heading_y = y0 + 12;
    draw_text_line_clipped(
        glyphs,
        frame,
        width,
        height,
        heading_x,
        heading_y,
        heading,
        rgba_bytes(OVERLAY_TEXT),
        x1 - 16,
    );

    let focus_label = match viewer.focus {
        LogFocus::Sources => "Sources",
        LogFocus::Logs => "Logs",
    };
    let hint = format!(
        "Focus: {focus_label} • / search • F follow • S sudo • R refresh • Tab switch focus"
    );
    draw_text_line_clipped(
        glyphs,
        frame,
        width,
        height,
        heading_x,
        heading_y + line_h,
        &hint,
        rgba_bytes(OVERLAY_TEXT_MUTED),
        x1 - 16,
    );

    let search_text = if viewer.editing {
        format!("Search: {}_", viewer.input)
    } else if viewer.query.is_empty() {
        "Search: (none)".to_string()
    } else {
        format!("Search: {}", viewer.query)
    };
    draw_text_line_clipped(
        glyphs,
        frame,
        width,
        height,
        heading_x,
        heading_y + line_h * 2,
        &search_text,
        rgba_bytes(OVERLAY_TEXT),
        x1 - 16,
    );

    let status_text = if viewer.status.is_empty() {
        format!(
            "Follow: {} • Sudo: {} • Lines: {}",
            if viewer.follow { "on" } else { "off" },
            if viewer.sudo { "on" } else { "off" },
            viewer.lines.len()
        )
    } else {
        viewer.status.clone()
    };
    draw_text_line_clipped(
        glyphs,
        frame,
        width,
        height,
        heading_x,
        heading_y + line_h * 3,
        &status_text,
        rgba_bytes(OVERLAY_ACCENT),
        x1 - 16,
    );

    let panel_w = (x1 - x0).max(0);
    let list_w = ((panel_w as f32) * 0.28).round().max(160.0) as i32;
    let list_x0 = x0 + 16;
    let list_y0 = heading_y + line_h * 4 + 8;
    let list_x1 = (list_x0 + list_w).min(x1 - 16);
    let list_y1 = y1 - 16;

    let list_layout = PanelLayout {
        rect: (list_x0, list_y0, list_x1, list_y1),
        padding: 10,
        item_height: glyphs.cell_height() as i32 + 8,
    };
    let items: Vec<(String, bool)> = viewer
        .sources
        .iter()
        .map(|s| (s.label.clone(), s.enabled))
        .collect();
    list_panel_cpu(
        glyphs,
        frame,
        width,
        height,
        list_layout,
        rgba_bytes(OVERLAY_PANEL_INNER),
        &items,
        None,
        Some(viewer.selected),
    );

    let log_x0 = list_x1 + 12;
    let log_y0 = list_y0;
    let log_x1 = x1 - 16;
    let log_y1 = list_y1;
    fill_rect(
        frame,
        width,
        height,
        log_x0,
        log_y0,
        log_x1,
        log_y1,
        rgba_bytes(OVERLAY_PANEL_INNER),
    );

    let visible_lines = ((log_y1 - log_y0) / line_h).max(0) as usize;
    let max_start = viewer
        .lines
        .len()
        .saturating_sub(visible_lines);
    let start = if viewer.follow {
        max_start
    } else {
        viewer.scroll.min(max_start)
    };
    let mut y = log_y0 + 8;
    let line_limit = log_y1 - 8;
    if viewer.lines.is_empty() {
        draw_text_line_clipped(
            glyphs,
            frame,
            width,
            height,
            log_x0 + 12,
            y,
            "No log entries",
            rgba_bytes(OVERLAY_TEXT_MUTED),
            log_x1 - 12,
        );
    } else {
        for line in viewer.lines.iter().skip(start).take(visible_lines) {
            if y + line_h > line_limit {
                break;
            }
            draw_text_line_clipped(
                glyphs,
                frame,
                width,
                height,
                log_x0 + 12,
                y,
                line,
                rgba_bytes(OVERLAY_TEXT),
                log_x1 - 12,
            );
            y += line_h;
        }
    }
}

pub fn build_log_viewer_gpu(
    rects: &mut Vec<RectVertex>,
    glyphs_out: &mut Vec<GlyphVertex>,
    atlas: &mut GlyphAtlas,
    queue: &wgpu::Queue,
    glyphs: &mut GlyphCache,
    width: u32,
    height: u32,
    viewer: &LogViewer,
) {
    if !viewer.open || width == 0 || height == 0 {
        return;
    }

    GpuRenderer::push_rect(rects, 0.0, 0.0, width as f32, height as f32, OVERLAY_DIM);
    let (x0, y0, x1, y1) = modal_panel_rect(
        width,
        height,
        MODAL_PANEL_W_FRAC,
        MODAL_PANEL_H_FRAC,
        MODAL_PANEL_MIN_W,
        MODAL_PANEL_MIN_H,
    );
    GpuRenderer::push_rect(rects, x0 as f32, y0 as f32, x1 as f32, y1 as f32, OVERLAY_PANEL);
    GpuRenderer::push_rect(
        rects,
        (x0 + 1) as f32,
        (y0 + 1) as f32,
        (x1 - 1) as f32,
        (y1 - 1) as f32,
        OVERLAY_PANEL_INNER,
    );

    let line_h = glyphs.cell_height() as i32 + 6;
    let heading_x = x0 + 16;
    let heading_y = y0 + 12;
    GpuRenderer::push_text_line(
        glyphs,
        atlas,
        glyphs_out,
        "Log Viewer — F5/Esc close",
        heading_x as f32,
        heading_y as f32,
        OVERLAY_TEXT,
        queue,
    );

    let focus_label = match viewer.focus {
        LogFocus::Sources => "Sources",
        LogFocus::Logs => "Logs",
    };
    let hint = format!(
        "Focus: {focus_label} • / search • F follow • S sudo • R refresh • Tab switch focus"
    );
    GpuRenderer::push_text_line(
        glyphs,
        atlas,
        glyphs_out,
        &hint,
        heading_x as f32,
        (heading_y + line_h) as f32,
        OVERLAY_TEXT_MUTED,
        queue,
    );

    let search_text = if viewer.editing {
        format!("Search: {}_", viewer.input)
    } else if viewer.query.is_empty() {
        "Search: (none)".to_string()
    } else {
        format!("Search: {}", viewer.query)
    };
    GpuRenderer::push_text_line(
        glyphs,
        atlas,
        glyphs_out,
        &search_text,
        heading_x as f32,
        (heading_y + line_h * 2) as f32,
        OVERLAY_TEXT,
        queue,
    );

    let status_text = if viewer.status.is_empty() {
        format!(
            "Follow: {} • Sudo: {} • Lines: {}",
            if viewer.follow { "on" } else { "off" },
            if viewer.sudo { "on" } else { "off" },
            viewer.lines.len()
        )
    } else {
        viewer.status.clone()
    };
    GpuRenderer::push_text_line(
        glyphs,
        atlas,
        glyphs_out,
        &status_text,
        heading_x as f32,
        (heading_y + line_h * 3) as f32,
        OVERLAY_ACCENT,
        queue,
    );

    let panel_w = (x1 - x0).max(0);
    let list_w = ((panel_w as f32) * 0.28).round().max(160.0) as i32;
    let list_x0 = x0 + 16;
    let list_y0 = heading_y + line_h * 4 + 8;
    let list_x1 = (list_x0 + list_w).min(x1 - 16);
    let list_y1 = y1 - 16;
    let list_layout = PanelLayout {
        rect: (list_x0, list_y0, list_x1, list_y1),
        padding: 10,
        item_height: glyphs.cell_height() as i32 + 8,
    };
    let items: Vec<(String, bool)> = viewer
        .sources
        .iter()
        .map(|s| (s.label.clone(), s.enabled))
        .collect();
    list_panel_gpu(
        rects,
        glyphs_out,
        atlas,
        queue,
        glyphs,
        list_layout,
        OVERLAY_PANEL_INNER,
        &items,
        None,
        Some(viewer.selected),
    );

    let log_x0 = list_x1 + 12;
    let log_y0 = list_y0;
    let log_x1 = x1 - 16;
    let log_y1 = list_y1;
    GpuRenderer::push_rect(
        rects,
        log_x0 as f32,
        log_y0 as f32,
        log_x1 as f32,
        log_y1 as f32,
        OVERLAY_PANEL_INNER,
    );

    let visible_lines = ((log_y1 - log_y0) / line_h).max(0) as usize;
    let max_start = viewer
        .lines
        .len()
        .saturating_sub(visible_lines);
    let start = if viewer.follow {
        max_start
    } else {
        viewer.scroll.min(max_start)
    };
    let mut y = log_y0 + 8;
    let line_limit = log_y1 - 8;
    if viewer.lines.is_empty() {
        GpuRenderer::push_text_line(
            glyphs,
            atlas,
            glyphs_out,
            "No log entries",
            (log_x0 + 12) as f32,
            y as f32,
            OVERLAY_TEXT_MUTED,
            queue,
        );
    } else {
        for line in viewer.lines.iter().skip(start).take(visible_lines) {
            if y + line_h > line_limit {
                break;
            }
            GpuRenderer::push_text_line(
                glyphs,
                atlas,
                glyphs_out,
                line,
                (log_x0 + 12) as f32,
                y as f32,
                OVERLAY_TEXT,
                queue,
            );
            y += line_h;
        }
    }
}
