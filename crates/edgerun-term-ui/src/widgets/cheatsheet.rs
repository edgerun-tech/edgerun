// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::too_many_arguments)]

use crate::widgets::{
    MODAL_PANEL_H_FRAC, MODAL_PANEL_MIN_H, MODAL_PANEL_MIN_W, MODAL_PANEL_W_FRAC, modal_panel_rect,
};
use pixels::wgpu;
use term_core::gpu::{GlyphAtlas, GlyphVertex, GpuRenderer, RectVertex};
use term_core::render::primitives::fill_rect;
use term_core::render::{
    GlyphCache, OVERLAY_ACCENT, OVERLAY_DIM, OVERLAY_PANEL, OVERLAY_PANEL_INNER, OVERLAY_TEXT,
    OVERLAY_TEXT_MUTED, draw_text_line_clipped, rgba_bytes,
};

pub struct Cheatsheet {
    open: bool,
}

impl Default for Cheatsheet {
    fn default() -> Self {
        Self::new()
    }
}

impl Cheatsheet {
    pub fn new() -> Self {
        Self { open: false }
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }
}

#[derive(Clone)]
struct CheatSection {
    title: &'static str,
    lines: &'static [&'static str],
}

const CHEATSHEET_SECTIONS: &[CheatSection] = &[
    CheatSection {
        title: "App shortcuts",
        lines: &[
            "F1 toggle mini help • F2 open/close this cheatsheet • F5 log viewer",
            "Alt+T new tab • Alt+Q close tab",
            "Alt+1-9 jump tabs • Ctrl+Tab / Ctrl+Shift+Tab cycle",
            "ˇ autocomplete palette",
            "Right-click menu • Middle-click paste clipboard",
        ],
    },
    CheatSection {
        title: "Line editing (readline)",
        lines: &[
            "Ctrl+A / Ctrl+E start/end of line",
            "Alt+B / Alt+F move by word",
            "Ctrl+U / Ctrl+K delete to start/end",
            "Ctrl+W delete word back • Alt+D delete word forward",
            "Ctrl+L clear screen",
        ],
    },
    CheatSection {
        title: "History",
        lines: &[
            "Ctrl+R reverse search • Enter runs selection",
            "Up/Down cycle history • !! repeat last command",
            "!str run last command starting with str",
            "history | tail -n 20 list recent entries",
        ],
    },
    CheatSection {
        title: "Files & navigation",
        lines: &[
            "pwd print directory • ls -lah list with sizes",
            "cd path change dir • pushd/popd jump stacks",
            "mkdir -p dir create nested dirs",
            "cp -r src dst copy tree • mv old new move/rename",
            "rm -rf target remove recursively (careful)",
            "cat file quick view • less file paged view",
        ],
    },
    CheatSection {
        title: "Pipes & redirection",
        lines: &[
            "cmd > out overwrite • cmd >> out append",
            "cmd1 | cmd2 pipe output into next command",
            "grep -n \"pat\" file search text",
            "tar -xzf file.tgz extract • zip -r out.zip dir",
            "curl -LO url download file",
        ],
    },
    CheatSection {
        title: "Processes",
        lines: &[
            "Ctrl+C stop foreground • Ctrl+Z suspend",
            "fg resume • bg continue in background",
            "jobs list jobs • kill -9 pid force stop",
            "ps aux | grep name find pids • top/htop monitor",
        ],
    },
];

pub fn draw_cheatsheet_cpu(
    cheatsheet: &Cheatsheet,
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
) {
    if !cheatsheet.is_open() || width == 0 || height == 0 {
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

    fill_rect(
        frame,
        width,
        height,
        x0,
        y0,
        x1,
        y1,
        rgba_bytes(OVERLAY_PANEL),
    );
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
    let heading = "Terminal Cheatsheet — press F2 or Esc to close";
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

    let panel_w = (x1 - x0).max(0);
    let columns = 2;
    let col_padding = 18;
    let col_width = ((panel_w - col_padding * (columns + 1)) / columns).max(120);
    let mut col = 0;
    let mut x = x0 + col_padding;
    let mut y = heading_y + line_h + 6;
    let bottom = y1 - col_padding;

    for section in CHEATSHEET_SECTIONS {
        let needed = ((section.lines.len() as i32 + 1) * line_h) + line_h / 2;
        if y + needed > bottom && col + 1 < columns {
            col += 1;
            x = x0 + col_padding + col * (col_width + col_padding);
            y = heading_y + line_h;
        }
        if y + line_h > bottom {
            break;
        }

        draw_text_line_clipped(
            glyphs,
            frame,
            width,
            height,
            x,
            y,
            section.title,
            rgba_bytes(OVERLAY_ACCENT),
            x + col_width,
        );
        y += line_h;

        for line in section.lines {
            if y + line_h > bottom {
                break;
            }
            draw_text_line_clipped(
                glyphs,
                frame,
                width,
                height,
                x,
                y,
                line,
                rgba_bytes(OVERLAY_TEXT_MUTED),
                x + col_width,
            );
            y += line_h;
        }
        y += line_h / 2;
    }
}

pub fn build_cheatsheet_gpu(
    rects: &mut Vec<RectVertex>,
    glyphs_out: &mut Vec<GlyphVertex>,
    atlas: &mut GlyphAtlas,
    queue: &wgpu::Queue,
    cheatsheet: &Cheatsheet,
    glyphs: &mut GlyphCache,
    width: u32,
    height: u32,
) {
    if !cheatsheet.is_open() || width == 0 || height == 0 {
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
    let x0 = x0 as f32;
    let y0 = y0 as f32;
    let x1 = x1 as f32;
    let y1 = y1 as f32;

    GpuRenderer::push_rect(rects, x0, y0, x1, y1, OVERLAY_PANEL);
    GpuRenderer::push_rect(
        rects,
        x0 + 1.0,
        y0 + 1.0,
        x1 - 1.0,
        y1 - 1.0,
        OVERLAY_PANEL_INNER,
    );

    let line_h = glyphs.cell_height() as f32 + 6.0;
    let heading = "Terminal Cheatsheet — press F2 or Esc to close";
    let heading_x = x0 + 16.0;
    let heading_y = y0 + 12.0;
    GpuRenderer::push_text_line(
        glyphs,
        atlas,
        glyphs_out,
        heading,
        heading_x,
        heading_y,
        OVERLAY_TEXT,
        queue,
    );

    let panel_w = (x1 - x0).max(0.0);
    let columns = 2;
    let col_padding = 18.0;
    let col_width = ((panel_w - col_padding * (columns as f32 + 1.0)) / columns as f32).max(120.0);
    let mut col = 0;
    let mut x = x0 + col_padding;
    let mut y = heading_y + line_h + 6.0;
    let bottom = y1 - col_padding;

    for section in CHEATSHEET_SECTIONS {
        let needed = ((section.lines.len() as f32 + 1.0) * line_h) + line_h / 2.0;
        if y + needed > bottom && col + 1 < columns {
            col += 1;
            x = x0 + col_padding + col as f32 * (col_width + col_padding);
            y = heading_y + line_h;
        }
        if y + line_h > bottom {
            break;
        }

        GpuRenderer::push_text_line(
            glyphs,
            atlas,
            glyphs_out,
            section.title,
            x,
            y,
            OVERLAY_ACCENT,
            queue,
        );
        y += line_h;
        for line in section.lines {
            GpuRenderer::push_text_line(
                glyphs,
                atlas,
                glyphs_out,
                line,
                x + 12.0,
                y,
                OVERLAY_TEXT_MUTED,
                queue,
            );
            y += line_h;
        }
        y += line_h / 2.0;
    }
}
