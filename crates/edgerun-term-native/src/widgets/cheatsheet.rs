use crate::render::GlyphCache;
use crate::render::draw_text_line_clipped;
use crate::render::primitives::fill_rect;
use crate::terminal::Rgba;
use pixels::wgpu;

pub struct Cheatsheet {
    open: bool,
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
            "F1 toggle mini help • F2 open/close this cheatsheet",
            "Alt+T new tab • Alt+Q close tab",
            "Alt+1-5 jump tabs • Ctrl+Tab / Ctrl+Shift+Tab cycle",
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
        [8, 8, 12, 180],
    );

    let panel_w = (width as f32 * 0.9) as i32;
    let panel_h = (height as f32 * 0.82) as i32;
    let x0 = ((width as i32 - panel_w) / 2).max(0);
    let y0 = ((height as i32 - panel_h) / 2).max(0);
    let x1 = (x0 + panel_w).min(width as i32);
    let y1 = (y0 + panel_h).min(height as i32);

    fill_rect(frame, width, height, x0, y0, x1, y1, [18, 22, 30, 235]);

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
        [240, 240, 240, 255],
        x1 - 16,
    );

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
            [140, 200, 255, 255],
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
                [230, 230, 230, 255],
                x + col_width,
            );
            y += line_h;
        }
        y += line_h / 2;
    }
}

pub fn build_cheatsheet_gpu(
    rects: &mut Vec<crate::gpu::RectVertex>,
    glyphs_out: &mut Vec<crate::gpu::GlyphVertex>,
    atlas: &mut crate::gpu::GlyphAtlas,
    queue: &wgpu::Queue,
    cheatsheet: &Cheatsheet,
    glyphs: &mut GlyphCache,
    width: u32,
    height: u32,
) {
    if !cheatsheet.is_open() || width == 0 || height == 0 {
        return;
    }

    crate::gpu::GpuRenderer::push_rect(
        rects,
        0.0,
        0.0,
        width as f32,
        height as f32,
        Rgba {
            r: 8,
            g: 8,
            b: 12,
            a: 180,
        },
    );

    let panel_w = (width as f32 * 0.9) as f32;
    let panel_h = (height as f32 * 0.82) as f32;
    let x0 = ((width as f32 - panel_w) / 2.0).max(0.0);
    let y0 = ((height as f32 - panel_h) / 2.0).max(0.0);
    let x1 = (x0 + panel_w).min(width as f32);
    let y1 = (y0 + panel_h).min(height as f32);

    crate::gpu::GpuRenderer::push_rect(
        rects,
        x0,
        y0,
        x1,
        y1,
        Rgba {
            r: 18,
            g: 22,
            b: 30,
            a: 235,
        },
    );

    let line_h = glyphs.cell_height() as f32 + 6.0;
    let heading = "Terminal Cheatsheet — press F2 or Esc to close";
    let heading_x = x0 + 16.0;
    let heading_y = y0 + 12.0;
    crate::gpu::GpuRenderer::push_text_line_with_fallback(
        glyphs,
        atlas,
        glyphs_out,
        Some(rects),
        heading,
        heading_x,
        heading_y,
        Rgba {
            r: 240,
            g: 240,
            b: 240,
            a: 255,
        },
        queue,
    );

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

        crate::gpu::GpuRenderer::push_text_line_with_fallback(
            glyphs,
            atlas,
            glyphs_out,
            Some(rects),
            section.title,
            x,
            y,
            Rgba {
                r: 200,
                g: 220,
                b: 255,
                a: 255,
            },
            queue,
        );
        y += line_h;
        for line in section.lines {
            crate::gpu::GpuRenderer::push_text_line_with_fallback(
                glyphs,
                atlas,
                glyphs_out,
                Some(rects),
                line,
                x + 12.0,
                y,
                Rgba {
                    r: 220,
                    g: 220,
                    b: 220,
                    a: 255,
                },
                queue,
            );
            y += line_h;
        }
        y += line_h / 2.0;
    }
}
