use crate::render::GlyphCache;
use crate::render::draw_text_line_clipped;
use crate::render::primitives::fill_rect;
use crate::terminal::Rgba;
use pixels::wgpu;

pub struct SettingsPanel {
    pub open: bool,
    pub status: String,
    pub downloading: bool,
    pub scrollback_enabled: bool,
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
            "Font: Embedded Source Code Pro".to_string()
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

    let panel_w = (width as f32 * 0.7).max(320.0);
    let panel_h = (height as f32 * 0.5).max(220.0);
    let x0 = ((width as f32 - panel_w) / 2.0).max(0.0) as i32;
    let y0 = ((height as f32 - panel_h) / 2.0).max(0.0) as i32;
    let x1 = (x0 as f32 + panel_w).min(width as f32) as i32;
    let y1 = (y0 as f32 + panel_h).min(height as f32) as i32;

    fill_rect(frame, width, height, x0, y0, x1, y1, [16, 20, 28, 235]);
    fill_rect(
        frame,
        width,
        height,
        x0 + 1,
        y0 + 1,
        x1 - 1,
        y1 - 1,
        [30, 36, 46, 235],
    );

    let heading = "Settings (F4/Esc to close)";
    draw_text_line_clipped(
        glyphs,
        frame,
        width,
        height,
        x0 + 12,
        y0 + 12,
        heading,
        [220, 235, 255, 255],
        x1 - 12,
    );

    let line_h = glyphs.cell_height() as i32 + 6;
    let mut y = y0 + 12 + line_h;
    let lines = vec![
        format!(
            "Scrollback: {} (press S to toggle)",
            if settings.scrollback_enabled {
                "Enabled"
            } else {
                "Disabled"
            }
        ),
        settings.current_font_label(),
        "Fonts: F = next, R = refresh system list, 0 = reset embedded".to_string(),
        "Downloads: D = Noto Color Emoji, N = Nerd Font Symbols".to_string(),
        "Fonts saved to ~/.local/share/fonts/term-emoji/ (fc-cache runs)".to_string(),
        settings.status.clone(),
    ];

    for line in lines {
        draw_text_line_clipped(
            glyphs,
            frame,
            width,
            height,
            x0 + 12,
            y,
            &line,
            [210, 220, 235, 255],
            x1 - 12,
        );
        y += line_h;
    }
}

pub fn build_settings_panel_gpu(
    rects: &mut Vec<crate::gpu::RectVertex>,
    glyphs_out: &mut Vec<crate::gpu::GlyphVertex>,
    atlas: &mut crate::gpu::GlyphAtlas,
    queue: &wgpu::Queue,
    settings: &SettingsPanel,
    glyphs: &mut GlyphCache,
    width: u32,
    height: u32,
) {
    if !settings.open || width == 0 || height == 0 {
        return;
    }

    let panel_w = (width as f32 * 0.7).max(320.0);
    let panel_h = (height as f32 * 0.5).max(220.0);
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
            r: 16,
            g: 20,
            b: 28,
            a: 235,
        },
    );
    crate::gpu::GpuRenderer::push_rect(
        rects,
        x0 + 1.0,
        y0 + 1.0,
        x1 - 1.0,
        y1 - 1.0,
        Rgba {
            r: 30,
            g: 36,
            b: 46,
            a: 235,
        },
    );

    let heading = "Settings (F4/Esc to close)";
    crate::gpu::GpuRenderer::push_text_line_with_fallback(
        glyphs,
        atlas,
        glyphs_out,
        Some(rects),
        heading,
        x0 + 12.0,
        y0 + 12.0,
        Rgba {
            r: 220,
            g: 235,
            b: 255,
            a: 255,
        },
        queue,
    );

    let line_h = glyphs.cell_height() as f32 + 6.0;
    let mut y = y0 + 12.0 + line_h;
    let lines = vec![
        format!(
            "Scrollback: {} (press S to toggle)",
            if settings.scrollback_enabled {
                "Enabled"
            } else {
                "Disabled"
            }
        ),
        settings.current_font_label(),
        "Fonts: F = next, R = refresh system list, 0 = reset embedded".to_string(),
        "Downloads: D = Noto Color Emoji, N = Nerd Font Symbols".to_string(),
        "Fonts saved to ~/.local/share/fonts/term-emoji/ (fc-cache runs)".to_string(),
        settings.status.clone(),
    ];

    for line in lines {
        crate::gpu::GpuRenderer::push_text_line_with_fallback(
            glyphs,
            atlas,
            glyphs_out,
            Some(rects),
            &line,
            x0 + 12.0,
            y,
            Rgba {
                r: 210,
                g: 220,
                b: 235,
                a: 255,
            },
            queue,
        );
        y += line_h;
    }
}
