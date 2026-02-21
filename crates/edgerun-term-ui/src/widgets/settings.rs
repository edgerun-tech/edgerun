use crate::debug::DebugRenderMode;
use crate::widgets::{
    MODAL_PANEL_H_FRAC, MODAL_PANEL_MIN_H, MODAL_PANEL_MIN_W, MODAL_PANEL_W_FRAC, modal_panel_rect,
};
use pixels::wgpu;
use term_core::gpu::{GlyphAtlas, GlyphVertex, GpuRenderer, RectVertex};
use term_core::render::primitives::fill_rect;
use term_core::render::{
    GlyphCache, OVERLAY_DIM, OVERLAY_PANEL, OVERLAY_PANEL_INNER, OVERLAY_TEXT, OVERLAY_TEXT_MUTED,
    draw_text_line_clipped, rgba_bytes,
};

pub struct SettingsPanel {
    pub open: bool,
    pub status: String,
    pub downloading: bool,
    pub scrollback_enabled: bool,
    pub show_fps: bool,
    pub show_copy_notice: bool,
    pub selected_index: usize,
    pub render_mode: DebugRenderMode,
    pub log_level: String,
    pub system_fonts: Vec<SystemFont>,
    pub selected_font: Option<usize>,
}

impl SettingsPanel {
    pub fn new() -> Self {
        Self {
            open: false,
            status: String::new(),
            downloading: false,
            scrollback_enabled: true,
            show_fps: false,
            show_copy_notice: false,
            selected_index: 0,
            render_mode: DebugRenderMode::Auto,
            log_level: "debug".to_string(),
            system_fonts: Vec::new(),
            selected_font: None,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
        self.downloading = false;
    }

    pub fn refresh_system_fonts(&mut self) {
        self.system_fonts = discover_system_fonts();
        self.selected_font = None;
        if self.system_fonts.is_empty() {
            self.status =
                "No system fonts found in ~/.local/share/fonts or /usr/share/fonts".to_string();
        } else {
            self.status = format!(
                "Found {} system fonts (press F to cycle)",
                self.system_fonts.len()
            );
        }
    }

    pub fn cycle_font(&mut self) -> Option<SystemFont> {
        if self.system_fonts.is_empty() {
            return None;
        }
        let idx = self
            .selected_font
            .map(|i| (i + 1) % self.system_fonts.len())
            .unwrap_or(0);
        self.selected_font = Some(idx);
        self.system_fonts.get(idx).cloned()
    }

    pub fn current_font_label(&self) -> String {
        if let Some(idx) = self.selected_font.and_then(|i| self.system_fonts.get(i)) {
            format!("Font: {}", idx.name)
        } else {
            "Font: Embedded DejaVu Sans Mono".to_string()
        }
    }
}

#[derive(Clone)]
pub struct SystemFont {
    pub name: String,
    pub path: std::path::PathBuf,
}

fn discover_system_fonts() -> Vec<SystemFont> {
    use std::collections::HashSet;
    use std::collections::VecDeque;
    use std::fs;
    use std::path::PathBuf;

    let mut dirs = VecDeque::new();
    if let Some(font_dir) = dirs::font_dir() {
        dirs.push_back(font_dir);
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push_back(home.join(".local/share/fonts"));
    }
    dirs.push_back(PathBuf::from("/usr/share/fonts"));

    let mut seen = HashSet::new();
    let mut fonts = Vec::new();

    while let Some(dir) = dirs.pop_front() {
        if !dir.exists() {
            continue;
        }
        let Ok(read_dir) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push_back(path);
                continue;
            }
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            let ext = ext.to_ascii_lowercase();
            if ext == "ttf" || ext == "otf" || ext == "ttc" {
                if seen.insert(path.clone()) {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    fonts.push(SystemFont { name, path });
                }
            }
        }
    }

    fonts.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    fonts
}

pub fn draw_settings_panel_cpu(
    settings: &SettingsPanel,
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
) {
    if !settings.open || width == 0 || height == 0 {
        return;
    }

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
        0,
        0,
        width as i32,
        height as i32,
        rgba_bytes(OVERLAY_DIM),
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

    let heading = "Settings";
    draw_text_line_clipped(
        glyphs,
        frame,
        width,
        height,
        x0 + 12,
        y0 + 12,
        heading,
        rgba_bytes(OVERLAY_TEXT),
        x1 - 12,
    );

    let line_h = glyphs.cell_height() as i32 + 6;
    let mut y = y0 + 12 + line_h;
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

    for (i, line) in lines.into_iter().enumerate() {
        // draw highlight rect for selected line
        if i == settings.selected_index {
            fill_rect(
                frame,
                width,
                height,
                x0 + 8,
                y - 2,
                x1 - 8,
                y - 2 + line_h as i32 + 4,
                rgba_bytes(OVERLAY_PANEL_INNER),
            );
        }
        let (display, color) = if i == settings.selected_index {
            (format!("> {}", line), rgba_bytes(OVERLAY_TEXT))
        } else {
            (line, rgba_bytes(OVERLAY_TEXT_MUTED))
        };
        draw_text_line_clipped(
            glyphs,
            frame,
            width,
            height,
            x0 + 12,
            y,
            &display,
            color,
            x1 - 12,
        );
        y += line_h;
    }
}

pub fn build_settings_panel_gpu(
    rects: &mut Vec<RectVertex>,
    glyphs_out: &mut Vec<GlyphVertex>,
    atlas: &mut GlyphAtlas,
    queue: &wgpu::Queue,
    settings: &SettingsPanel,
    glyphs: &mut GlyphCache,
    width: u32,
    height: u32,
) {
    if !settings.open || width == 0 || height == 0 {
        return;
    }

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

    GpuRenderer::push_rect(rects, 0.0, 0.0, width as f32, height as f32, OVERLAY_DIM);
    GpuRenderer::push_rect(rects, x0, y0, x1, y1, OVERLAY_PANEL);
    GpuRenderer::push_rect(
        rects,
        x0 + 1.0,
        y0 + 1.0,
        x1 - 1.0,
        y1 - 1.0,
        OVERLAY_PANEL_INNER,
    );

    let heading = "Settings";
    GpuRenderer::push_text_line(
        glyphs,
        atlas,
        glyphs_out,
        heading,
        x0 + 12.0,
        y0 + 12.0,
        OVERLAY_TEXT,
        queue,
    );

    let line_h = glyphs.cell_height() as f32 + 6.0;
    let mut y = y0 + 12.0 + line_h;
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

    for (i, line) in lines.into_iter().enumerate() {
        let (display, color) = if i == settings.selected_index {
            // push a background rect for GPU render path
            GpuRenderer::push_rect(
                rects,
                x0 + 8.0,
                y - 2.0,
                x1 - 8.0,
                y - 2.0 + line_h + 4.0,
                OVERLAY_PANEL_INNER,
            );
            (format!("> {}", line), OVERLAY_TEXT)
        } else {
            (line, OVERLAY_TEXT_MUTED)
        };
        GpuRenderer::push_text_line(
            glyphs,
            atlas,
            glyphs_out,
            &display,
            x0 + 12.0,
            y,
            color,
            queue,
        );
        y += line_h;
    }
}
