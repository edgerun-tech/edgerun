//! Shared GPU UI scene primitives and a small native OpenGL renderer.
//!
//! The scene types are platform neutral and are intended to be consumed by both
//! native OpenGL/EGL/SDL hosts and browser WebGL hosts. The native GL renderer
//! below is only one backend for the scene.

use std::string::String;
use std::vec::Vec;

#[cfg(feature = "fontdue-text")]
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color4 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color4 {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RectMode {
    Fill,
    Shadow,
    Border,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
    pub color: Color4,
    pub mode: RectMode,
    pub shadow: f32,
}

impl GpuRect {
    pub const fn fill(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color,
            mode: RectMode::Fill,
            shadow: 0.0,
        }
    }

    pub const fn shadow(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        color: Color4,
        shadow: f32,
    ) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color,
            mode: RectMode::Shadow,
            shadow,
        }
    }

    pub const fn border(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color,
            mode: RectMode::Border,
            shadow: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GpuScene {
    pub clear: Color4,
    rects: Vec<GpuRect>,
    #[cfg(feature = "fontdue-text")]
    text_quads: Vec<TextQuad>,
}

impl GpuScene {
    pub fn new(clear: Color4) -> Self {
        Self {
            clear,
            rects: Vec::new(),
            #[cfg(feature = "fontdue-text")]
            text_quads: Vec::new(),
        }
    }

    pub fn clear_rects(&mut self) {
        self.rects.clear();
        #[cfg(feature = "fontdue-text")]
        self.text_quads.clear();
    }

    pub fn push_rect(&mut self, rect: GpuRect) {
        self.rects.push(rect);
    }

    pub fn push_text(&mut self, mut x: f32, y: f32, text: &str, scale: f32, color: Color4) {
        let cell = scale.max(1.0);
        let step = cell * 6.0;
        let start_x = x;
        for ch in text.chars() {
            match ch {
                '\n' => {
                    x = start_x;
                }
                '\r' => {}
                ' ' => x += step,
                _ => {
                    let glyph = glyph5x7(ch);
                    for (row, bits) in glyph.iter().copied().enumerate() {
                        for col in 0..5 {
                            if ((bits >> (4 - col)) & 1) == 0 {
                                continue;
                            }
                            self.push_rect(GpuRect::fill(
                                x + col as f32 * cell,
                                y + row as f32 * cell,
                                cell,
                                cell,
                                0.0,
                                color,
                            ));
                        }
                    }
                    x += step;
                }
            }
        }
    }

    pub fn rects(&self) -> &[GpuRect] {
        &self.rects
    }

    #[cfg(feature = "fontdue-text")]
    pub fn push_font_text(&mut self, atlas: &FontAtlas, x: f32, y: f32, text: &str, color: Color4) {
        atlas.layout_text(self, x, y, text, color);
    }

    #[cfg(feature = "fontdue-text")]
    pub fn text_quads(&self) -> &[TextQuad] {
        &self.text_quads
    }
}

#[cfg(feature = "fontdue-text")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextQuad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub color: Color4,
}

#[cfg(feature = "fontdue-text")]
#[derive(Clone, Copy, Debug)]
struct AtlasGlyph {
    uv: [f32; 4],
    size: [f32; 2],
    bearing: [f32; 2],
    advance: f32,
}

#[cfg(feature = "fontdue-text")]
#[derive(Clone, Debug)]
pub struct FontAtlas {
    pub width: u32,
    pub height: u32,
    pub alpha: Vec<u8>,
    glyphs: HashMap<char, AtlasGlyph>,
    px: f32,
}

#[cfg(feature = "fontdue-text")]
impl FontAtlas {
    pub fn from_font_bytes(bytes: &[u8], px: f32) -> Result<Self, String> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|_| "font parse failed".to_string())?;
        Ok(Self::build(&font, &ascii_chars(), px))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_inter(px: f32) -> Result<Self, String> {
        let path = crate::font::find_best_ui_font().ok_or_else(|| {
            "Inter font not found; set EDGE_UI_FONT=/path/to/Inter.ttf".to_string()
        })?;
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        Self::from_font_bytes(&bytes, px)
    }

    fn build(font: &fontdue::Font, chars: &[char], px: f32) -> Self {
        let width = 1024u32;
        let height = 1024u32;
        let mut alpha = vec![0u8; (width * height) as usize];
        let mut glyphs = HashMap::new();
        let mut x = 2u32;
        let mut y = 2u32;
        let mut row_h = 0u32;

        for &ch in chars {
            let (metrics, bitmap) = font.rasterize(ch, px);
            if metrics.width == 0 || metrics.height == 0 {
                glyphs.insert(
                    ch,
                    AtlasGlyph {
                        uv: [0.0; 4],
                        size: [0.0, 0.0],
                        bearing: [metrics.xmin as f32, metrics.ymin as f32],
                        advance: metrics.advance_width,
                    },
                );
                continue;
            }

            let gw = metrics.width as u32;
            let gh = metrics.height as u32;
            if x + gw + 2 >= width {
                x = 2;
                y += row_h + 2;
                row_h = 0;
            }
            if y + gh + 2 >= height {
                break;
            }

            for gy in 0..gh {
                for gx in 0..gw {
                    alpha[((y + gy) * width + x + gx) as usize] = bitmap[(gy * gw + gx) as usize];
                }
            }

            glyphs.insert(
                ch,
                AtlasGlyph {
                    uv: [
                        x as f32 / width as f32,
                        y as f32 / height as f32,
                        (x + gw) as f32 / width as f32,
                        (y + gh) as f32 / height as f32,
                    ],
                    size: [gw as f32, gh as f32],
                    bearing: [metrics.xmin as f32, metrics.ymin as f32],
                    advance: metrics.advance_width,
                },
            );
            x += gw + 2;
            row_h = row_h.max(gh);
        }

        Self {
            width,
            height,
            alpha,
            glyphs,
            px,
        }
    }

    fn layout_text(&self, scene: &mut GpuScene, mut x: f32, y: f32, text: &str, color: Color4) {
        let baseline = y + self.px * 0.82;
        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }
            let Some(glyph) = self
                .glyphs
                .get(&ch)
                .or_else(|| self.glyphs.get(&ch.to_ascii_uppercase()))
            else {
                x += self.px * 0.32;
                continue;
            };
            if glyph.size[0] > 0.0 && glyph.size[1] > 0.0 {
                scene.text_quads.push(TextQuad {
                    x: x + glyph.bearing[0],
                    y: baseline - glyph.bearing[1] - glyph.size[1],
                    w: glyph.size[0],
                    h: glyph.size[1],
                    u0: glyph.uv[0],
                    v0: glyph.uv[1],
                    u1: glyph.uv[2],
                    v1: glyph.uv[3],
                    color,
                });
            }
            x += glyph.advance.max(self.px * 0.28);
        }
    }

    pub fn text_width(&self, text: &str) -> f32 {
        text.chars()
            .map(|ch| {
                self.glyphs
                    .get(&ch)
                    .or_else(|| self.glyphs.get(&ch.to_ascii_uppercase()))
                    .map(|glyph| glyph.advance.max(self.px * 0.28))
                    .unwrap_or(self.px * 0.32)
            })
            .sum()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChatShellMetrics {
    pub sidebar_w: f32,
    pub topbar_h: f32,
    pub composer_h: f32,
    pub pad: f32,
}

impl Default for ChatShellMetrics {
    fn default() -> Self {
        Self {
            sidebar_w: 284.0,
            topbar_h: 58.0,
            composer_h: 86.0,
            pad: 18.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnifiedContactKind {
    Person,
    CodexClient,
    Node,
}

#[derive(Clone, Copy, Debug)]
pub struct UnifiedContact<'a> {
    pub name: &'a str,
    pub detail: &'a str,
    pub kind: UnifiedContactKind,
    pub unread: u16,
    pub online: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct UnifiedMessage<'a> {
    pub author: &'a str,
    pub body: &'a str,
    pub outgoing: bool,
    pub accent: Color4,
}

#[derive(Clone, Debug)]
pub struct UnifiedChatState<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub contacts: &'a [UnifiedContact<'a>],
    pub selected_contact: usize,
    pub messages: &'a [UnifiedMessage<'a>],
    pub composer_placeholder: &'a str,
    pub connected: bool,
}

impl<'a> UnifiedChatState<'a> {
    pub fn demo(codex_active: bool) -> Self {
        const ACTIVE_CONTACTS: &[UnifiedContact<'static>] = &[
            UnifiedContact {
                name: "Ken",
                detail: "identity contact",
                kind: UnifiedContactKind::Person,
                unread: 0,
                online: true,
            },
            UnifiedContact {
                name: "Codex native",
                detail: "local agent client",
                kind: UnifiedContactKind::CodexClient,
                unread: 2,
                online: true,
            },
            UnifiedContact {
                name: "Codex WebGL",
                detail: "browser wasm client",
                kind: UnifiedContactKind::CodexClient,
                unread: 0,
                online: true,
            },
            UnifiedContact {
                name: "nodes.edgerun.tech",
                detail: "admission and relay",
                kind: UnifiedContactKind::Node,
                unread: 0,
                online: true,
            },
            UnifiedContact {
                name: "Family admission",
                detail: "policy source",
                kind: UnifiedContactKind::Node,
                unread: 0,
                online: false,
            },
        ];
        const IDLE_CONTACTS: &[UnifiedContact<'static>] = &[
            UnifiedContact {
                name: "Ken",
                detail: "identity contact",
                kind: UnifiedContactKind::Person,
                unread: 0,
                online: true,
            },
            UnifiedContact {
                name: "Codex native",
                detail: "local agent client",
                kind: UnifiedContactKind::CodexClient,
                unread: 2,
                online: true,
            },
            UnifiedContact {
                name: "Codex WebGL",
                detail: "browser wasm client",
                kind: UnifiedContactKind::CodexClient,
                unread: 0,
                online: false,
            },
            UnifiedContact {
                name: "nodes.edgerun.tech",
                detail: "admission and relay",
                kind: UnifiedContactKind::Node,
                unread: 0,
                online: true,
            },
            UnifiedContact {
                name: "Family admission",
                detail: "policy source",
                kind: UnifiedContactKind::Node,
                unread: 0,
                online: false,
            },
        ];
        const MESSAGES: &[UnifiedMessage<'static>] = &[
            UnifiedMessage {
                author: "Ken",
                body: "Codex clients should show up as contacts, not as a separate product surface.",
                outgoing: true,
                accent: palette::ACCENT,
            },
            UnifiedMessage {
                author: "Codex native",
                body: "I can live in the same contact book as people and node instances. The thread decides what capabilities I can use.",
                outgoing: false,
                accent: palette::VIOLET,
            },
            UnifiedMessage {
                author: "nodes.edgerun.tech",
                body: "Relay route is available. Messages remain recipient encrypted before they reach transport.",
                outgoing: false,
                accent: palette::GREEN,
            },
        ];

        Self {
            title: "EdgeRun Chat",
            subtitle: "contacts, Codex clients, and nodes",
            contacts: if codex_active {
                ACTIVE_CONTACTS
            } else {
                IDLE_CONTACTS
            },
            selected_contact: 1,
            messages: MESSAGES,
            composer_placeholder: "Message Codex native...",
            connected: codex_active,
        }
    }
}

pub fn build_unified_chat_shell(scene: &mut GpuScene, width: f32, height: f32, codex_active: bool) {
    let state = UnifiedChatState::demo(codex_active);
    build_unified_chat_shell_impl(
        scene,
        width,
        height,
        &state,
        #[cfg(feature = "fontdue-text")]
        None,
    );
}

#[cfg(feature = "fontdue-text")]
pub fn build_unified_chat_shell_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    state: &UnifiedChatState<'_>,
) {
    build_unified_chat_shell_impl(scene, width, height, state, Some(atlas));
}

fn build_unified_chat_shell_impl(
    scene: &mut GpuScene,
    width: f32,
    height: f32,
    state: &UnifiedChatState<'_>,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) {
    scene.clear = palette::BG;
    scene.clear_rects();
    let m = ChatShellMetrics::default();
    let w = width.max(360.0);
    let h = height.max(320.0);
    let sidebar_w = if w < 760.0 { 0.0 } else { m.sidebar_w + 28.0 };
    let main_x = sidebar_w;
    let main_w = w - sidebar_w;

    if sidebar_w > 0.0 {
        panel(scene, 0.0, 0.0, sidebar_w, h, 0.0, palette::SIDEBAR);
        scene.push_rect(GpuRect::fill(
            0.0,
            0.0,
            sidebar_w,
            4.0,
            0.0,
            palette::ACCENT,
        ));
        push_label(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            24.0,
            20.0,
            state.title,
            3.0,
            palette::TEXT,
        );
        push_label(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            24.0,
            50.0,
            state.subtitle,
            2.0,
            palette::MUTED,
        );
        draw_pill(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            sidebar_w - 104.0,
            22.0,
            78.0,
            if state.connected { "relay" } else { "local" },
            if state.connected {
                palette::GREEN
            } else {
                palette::AMBER
            },
        );
        scene.push_rect(GpuRect::fill(
            sidebar_w - 1.0,
            0.0,
            1.0,
            h,
            0.0,
            palette::BORDER,
        ));

        for (index, contact) in state.contacts.iter().enumerate() {
            let y = 88.0 + index as f32 * 68.0;
            let selected = index == state.selected_contact;
            draw_contact_row(
                scene,
                #[cfg(feature = "fontdue-text")]
                atlas,
                16.0,
                y,
                sidebar_w - 32.0,
                contact,
                selected,
            );
        }
    }

    panel(scene, main_x, 0.0, main_w, m.topbar_h, 0.0, palette::TOPBAR);
    let active = state
        .contacts
        .get(state.selected_contact)
        .or_else(|| state.contacts.first());
    let active_name = active.map(|contact| contact.name).unwrap_or("Unified chat");
    let active_detail = active
        .map(|contact| contact.detail)
        .unwrap_or("contact thread");
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        main_x + 20.0,
        14.0,
        active_name,
        2.0,
        palette::TEXT,
    );
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        main_x + 20.0,
        35.0,
        active_detail,
        2.0,
        palette::MUTED,
    );
    draw_pill(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        main_x + main_w - 322.0,
        15.0,
        132.0,
        "recipient sealed",
        palette::GREEN,
    );
    draw_pill(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        main_x + main_w - 178.0,
        15.0,
        154.0,
        "identity routed",
        palette::ACCENT,
    );
    scene.push_rect(GpuRect::fill(
        main_x,
        m.topbar_h - 1.0,
        main_w,
        1.0,
        0.0,
        palette::BORDER,
    ));

    let transcript_top = m.topbar_h + m.pad;
    let transcript_x = main_x + m.pad;
    let transcript_w = main_w - m.pad * 2.0;
    let rail_w = if transcript_w > 820.0 { 220.0 } else { 0.0 };
    let message_area_w = transcript_w - if rail_w > 0.0 { rail_w + 18.0 } else { 0.0 };
    let message_w = (message_area_w * 0.82).clamp(220.0, 760.0);
    let transcript_limit = h - m.composer_h - m.pad * 2.0;
    let mut message_y = transcript_top;

    for message in state.messages {
        let x = if message.outgoing {
            transcript_x + message_area_w - message_w
        } else {
            transcript_x
        };
        let fill = if message.outgoing {
            palette::USER
        } else {
            palette::ASSISTANT
        };
        let height = estimate_message_height(
            message.body,
            (message_w - 42.0).max(120.0),
            4,
            #[cfg(feature = "fontdue-text")]
            atlas,
        );
        if message_y + height > transcript_limit {
            break;
        }
        let drawn = draw_message(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            x,
            message_y,
            message_w,
            message.author,
            message.body,
            fill,
            message.accent,
        );
        message_y += drawn + 16.0;
    }

    if rail_w > 0.0 {
        let rail_x = transcript_x + transcript_w - rail_w;
        soft_card(
            scene,
            rail_x,
            transcript_top,
            rail_w,
            170.0,
            12.0,
            palette::PANEL,
        );
        push_label(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            rail_x + 16.0,
            transcript_top + 18.0,
            "Thread policy",
            2.0,
            palette::TEXT,
        );
        draw_pill(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            rail_x + 16.0,
            transcript_top + 50.0,
            132.0,
            "2 recipients",
            palette::ACCENT,
        );
        draw_pill(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            rail_x + 16.0,
            transcript_top + 84.0,
            158.0,
            "codex tools scoped",
            palette::VIOLET,
        );
        draw_pill(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            rail_x + 16.0,
            transcript_top + 118.0,
            140.0,
            "relay admitted",
            palette::GREEN,
        );
    }

    let composer = (
        main_x + m.pad,
        h - m.composer_h - m.pad,
        main_w - m.pad * 2.0,
        m.composer_h,
    );
    soft_card(
        scene,
        composer.0,
        composer.1,
        composer.2,
        composer.3,
        16.0,
        palette::COMPOSER,
    );
    scene.push_rect(GpuRect::border(
        composer.0,
        composer.1,
        composer.2,
        composer.3,
        16.0,
        if state.connected {
            palette::ACCENT
        } else {
            palette::BORDER
        },
    ));
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        composer.0 + 22.0,
        composer.1 + 24.0,
        state.composer_placeholder,
        2.0,
        palette::MUTED,
    );
    draw_pill(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        composer.0 + 20.0,
        composer.1 + composer.3 - 34.0,
        92.0,
        "encrypted",
        palette::GREEN,
    );
    draw_pill(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        composer.0 + 122.0,
        composer.1 + composer.3 - 34.0,
        88.0,
        "contact",
        palette::ACCENT,
    );
    draw_pill(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        composer.0 + 220.0,
        composer.1 + composer.3 - 34.0,
        112.0,
        "codex tools",
        palette::VIOLET,
    );
    scene.push_rect(GpuRect::fill(
        composer.0 + composer.2 - 58.0,
        composer.1 + composer.3 - 56.0,
        40.0,
        38.0,
        12.0,
        palette::ACCENT,
    ));
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        composer.0 + composer.2 - 47.0,
        composer.1 + composer.3 - 45.0,
        ">",
        3.0,
        palette::ACCENT_TEXT,
    );
}

pub fn build_codex_chat_shell(scene: &mut GpuScene, width: f32, height: f32, thinking: bool) {
    build_codex_chat_shell_impl(
        scene,
        width,
        height,
        thinking,
        #[cfg(feature = "fontdue-text")]
        None,
    );
}

#[cfg(feature = "fontdue-text")]
pub fn build_codex_chat_shell_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    thinking: bool,
) {
    build_codex_chat_shell_impl(scene, width, height, thinking, Some(atlas));
}

fn build_codex_chat_shell_impl(
    scene: &mut GpuScene,
    width: f32,
    height: f32,
    thinking: bool,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) {
    scene.clear = palette::BG;
    scene.clear_rects();
    let m = ChatShellMetrics::default();
    let w = width.max(360.0);
    let h = height.max(320.0);
    let sidebar_w = if w < 760.0 { 0.0 } else { m.sidebar_w };
    let main_x = sidebar_w;
    let main_w = w - sidebar_w;

    if sidebar_w > 0.0 {
        panel(scene, 0.0, 0.0, sidebar_w, h, 0.0, palette::SIDEBAR);
        scene.push_rect(GpuRect::fill(
            0.0,
            0.0,
            sidebar_w,
            4.0,
            0.0,
            palette::ACCENT,
        ));
        push_label(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            24.0,
            22.0,
            "Codex",
            3.0,
            palette::TEXT,
        );
        push_label(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            24.0,
            50.0,
            "native and web",
            2.0,
            palette::MUTED,
        );
        draw_pill(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            182.0,
            24.0,
            76.0,
            "local",
            palette::GREEN,
        );
        scene.push_rect(GpuRect::fill(
            sidebar_w - 1.0,
            0.0,
            1.0,
            h,
            0.0,
            palette::BORDER,
        ));
        let threads = [
            ("Codex GL client", "active", palette::ACCENT),
            ("WASM renderer", "web", palette::VIOLET),
            ("Native transport", "ok", palette::GREEN),
            ("Approvals", "2 pending", palette::AMBER),
            ("Settings", "local", palette::MUTED),
        ];
        for (i, (title, detail, color)) in threads.iter().enumerate() {
            let y = 86.0 + i as f32 * 64.0;
            let selected = i == 0;
            soft_card(
                scene,
                16.0,
                y,
                sidebar_w - 32.0,
                52.0,
                10.0,
                if selected {
                    palette::ACTIVE_ROW
                } else {
                    palette::ROW
                },
            );
            push_label(
                scene,
                #[cfg(feature = "fontdue-text")]
                atlas,
                32.0,
                y + 12.0,
                title,
                2.0,
                if selected {
                    palette::TEXT
                } else {
                    palette::MUTED
                },
            );
            push_label(
                scene,
                #[cfg(feature = "fontdue-text")]
                atlas,
                32.0,
                y + 33.0,
                detail,
                2.0,
                *color,
            );
        }
    }

    panel(scene, main_x, 0.0, main_w, m.topbar_h, 0.0, palette::TOPBAR);
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        main_x + 20.0,
        18.0,
        "GPT-5.5",
        2.0,
        palette::TEXT,
    );
    draw_pill(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        main_x + 94.0,
        15.0,
        86.0,
        "128k ctx",
        palette::VIOLET,
    );
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        main_x + 198.0,
        18.0,
        if thinking { "Thinking" } else { "Ready" },
        2.0,
        if thinking {
            palette::ACCENT
        } else {
            palette::MUTED
        },
    );
    draw_pill(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        main_x + main_w - 214.0,
        15.0,
        92.0,
        "tools on",
        palette::GREEN,
    );
    draw_pill(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        main_x + main_w - 112.0,
        15.0,
        88.0,
        "webgl",
        palette::ACCENT,
    );
    scene.push_rect(GpuRect::fill(
        main_x,
        m.topbar_h - 1.0,
        main_w,
        1.0,
        0.0,
        palette::BORDER,
    ));

    let transcript_top = m.topbar_h + m.pad;
    let transcript_x = main_x + m.pad;
    let transcript_w = main_w - m.pad * 2.0;

    let message_w = (transcript_w * 0.78).clamp(220.0, 820.0);
    let mut message_y = transcript_top;
    let transcript_limit = h - m.composer_h - m.pad * 2.0;
    let first_h = draw_message(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        transcript_x,
        message_y,
        message_w,
        "Codex",
        "Native GL and browser WebGL now share the same scene graph, font atlas, and layout code.",
        palette::ASSISTANT,
        palette::ACCENT,
    );
    message_y += first_h + 16.0;

    let second_h = draw_message(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        transcript_x + transcript_w - message_w,
        message_y,
        message_w,
        "You",
        "Keep improving the UX, but keep it performant and portable.",
        palette::USER,
        palette::ACCENT,
    );
    message_y += second_h + 16.0;

    let assistant_body = if thinking {
        "Streaming the next interface pass. Approvals, tool calls, and prompt editing should all live in this same shared scene."
    } else {
        "Ready for input. The next pass can wire real Codex messages and tool events into this renderer."
    };
    let third_h = estimate_message_height(
        assistant_body,
        (message_w - 42.0).max(120.0),
        4,
        #[cfg(feature = "fontdue-text")]
        atlas,
    );
    if message_y + third_h <= transcript_limit {
        draw_message(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            transcript_x,
            message_y,
            message_w,
            "Codex",
            assistant_body,
            palette::ASSISTANT,
            if thinking {
                palette::AMBER
            } else {
                palette::GREEN
            },
        );
    }

    if thinking {
        scene.push_rect(GpuRect::fill(
            transcript_x + 20.0,
            (message_y + 26.0).min(transcript_limit - 8.0),
            76.0,
            6.0,
            3.0,
            palette::ACCENT,
        ));
    }

    let rail_x = transcript_x + transcript_w - 188.0;
    if transcript_w > 720.0 {
        soft_card(
            scene,
            rail_x,
            transcript_top + 220.0,
            188.0,
            124.0,
            12.0,
            palette::PANEL,
        );
        push_label(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            rail_x + 16.0,
            transcript_top + 238.0,
            "Turn state",
            2.0,
            palette::TEXT,
        );
        draw_pill(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            rail_x + 16.0,
            transcript_top + 268.0,
            94.0,
            if thinking { "running" } else { "idle" },
            if thinking {
                palette::AMBER
            } else {
                palette::GREEN
            },
        );
        draw_pill(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            rail_x + 16.0,
            transcript_top + 302.0,
            118.0,
            "2 approvals",
            palette::VIOLET,
        );
    }

    let composer = (
        main_x + m.pad,
        h - m.composer_h - m.pad,
        main_w - m.pad * 2.0,
        m.composer_h,
    );
    soft_card(
        scene,
        composer.0,
        composer.1,
        composer.2,
        composer.3,
        16.0,
        palette::COMPOSER,
    );
    scene.push_rect(GpuRect::border(
        composer.0,
        composer.1,
        composer.2,
        composer.3,
        16.0,
        if thinking {
            palette::ACCENT
        } else {
            palette::BORDER
        },
    ));
    scene.push_rect(GpuRect::fill(
        composer.0 + composer.2 - 58.0,
        composer.1 + composer.3 - 56.0,
        40.0,
        38.0,
        12.0,
        palette::ACCENT,
    ));
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        composer.0 + 22.0,
        composer.1 + 24.0,
        "Ask Codex to change the codebase...",
        2.0,
        palette::MUTED,
    );
    draw_pill(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        composer.0 + 20.0,
        composer.1 + composer.3 - 34.0,
        78.0,
        "native",
        palette::ACCENT,
    );
    draw_pill(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        composer.0 + 108.0,
        composer.1 + composer.3 - 34.0,
        74.0,
        "wasm",
        palette::VIOLET,
    );
    draw_pill(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        composer.0 + 192.0,
        composer.1 + composer.3 - 34.0,
        92.0,
        "no logs",
        palette::GREEN,
    );
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        composer.0 + composer.2 - 47.0,
        composer.1 + composer.3 - 45.0,
        ">",
        3.0,
        palette::ACCENT_TEXT,
    );
}

fn push_label(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    text: &str,
    scale: f32,
    color: Color4,
) {
    #[cfg(feature = "fontdue-text")]
    if let Some(atlas) = atlas {
        scene.push_font_text(atlas, x, y, text, color);
        return;
    }
    scene.push_text(x, y, text, scale, color);
}

fn draw_pill(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    color: Color4,
) {
    scene.push_rect(GpuRect::fill(x, y, w, 28.0, 14.0, color.with_alpha(0.14)));
    scene.push_rect(GpuRect::border(x, y, w, 28.0, 14.0, color.with_alpha(0.46)));
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 12.0,
        y + 7.0,
        label,
        2.0,
        color,
    );
}

fn draw_contact_row(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    w: f32,
    contact: &UnifiedContact<'_>,
    selected: bool,
) {
    soft_card(
        scene,
        x,
        y,
        w,
        56.0,
        10.0,
        if selected {
            palette::ACTIVE_ROW
        } else {
            palette::ROW
        },
    );
    let accent = match contact.kind {
        UnifiedContactKind::Person => palette::ACCENT,
        UnifiedContactKind::CodexClient => palette::VIOLET,
        UnifiedContactKind::Node => palette::GREEN,
    };
    scene.push_rect(GpuRect::fill(
        x + 14.0,
        y + 13.0,
        30.0,
        30.0,
        15.0,
        accent.with_alpha(0.22),
    ));
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 23.0,
        y + 19.0,
        contact_initial(contact.name),
        2.0,
        accent,
    );
    scene.push_rect(GpuRect::fill(
        x + 38.0,
        y + 36.0,
        8.0,
        8.0,
        4.0,
        if contact.online {
            palette::GREEN
        } else {
            palette::MUTED
        },
    ));
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 56.0,
        y + 11.0,
        contact.name,
        2.0,
        if selected {
            palette::TEXT
        } else {
            palette::MUTED
        },
    );
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 56.0,
        y + 32.0,
        contact.detail,
        2.0,
        accent,
    );
    if contact.unread > 0 {
        let label = if contact.unread > 9 { "9+" } else { "new" };
        draw_pill(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            x + w - 58.0,
            y + 14.0,
            42.0,
            label,
            palette::AMBER,
        );
    }
}

fn contact_initial(name: &str) -> &str {
    name.get(0..1).unwrap_or("?")
}

fn draw_message(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    w: f32,
    role: &str,
    body: &str,
    fill: Color4,
    accent: Color4,
) -> f32 {
    let body_w = (w - 42.0).max(120.0);
    let lines = wrap_lines(
        body,
        body_w,
        4,
        #[cfg(feature = "fontdue-text")]
        atlas,
    );
    let h = 56.0 + lines.len() as f32 * 22.0;
    soft_card(scene, x, y, w, h, 14.0, fill);
    scene.push_rect(GpuRect::fill(x, y, 4.0, h, 2.0, accent));
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 22.0,
        y + 16.0,
        role,
        2.0,
        accent,
    );
    for (index, line) in lines.iter().enumerate() {
        push_label(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            x + 22.0,
            y + 42.0 + index as f32 * 22.0,
            line,
            2.0,
            palette::TEXT,
        );
    }
    h
}

fn estimate_message_height(
    text: &str,
    max_width: f32,
    max_lines: usize,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> f32 {
    56.0 + wrap_lines(
        text,
        max_width,
        max_lines,
        #[cfg(feature = "fontdue-text")]
        atlas,
    )
    .len() as f32
        * 22.0
}

fn wrap_lines(
    text: &str,
    max_width: f32,
    max_lines: usize,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if measure_label_width(
            &candidate,
            2.0,
            #[cfg(feature = "fontdue-text")]
            atlas,
        ) <= max_width
        {
            current = candidate;
            continue;
        }

        if !current.is_empty() {
            lines.push(current);
        }
        current = word.to_string();

        if lines.len() + 1 >= max_lines {
            break;
        }
    }

    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    let consumed_words = lines
        .iter()
        .map(|line| line.split_whitespace().count())
        .sum::<usize>();
    let total_words = text.split_whitespace().count();
    if consumed_words < total_words {
        if let Some(last) = lines.last_mut() {
            while !last.is_empty()
                && measure_label_width(
                    &format!("{last}..."),
                    2.0,
                    #[cfg(feature = "fontdue-text")]
                    atlas,
                ) > max_width
            {
                last.pop();
            }
            last.push_str("...");
        }
    }

    lines
}

fn measure_label_width(
    text: &str,
    scale: f32,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> f32 {
    #[cfg(feature = "fontdue-text")]
    if let Some(atlas) = atlas {
        return atlas.text_width(text);
    }
    text.chars().count() as f32 * scale.max(1.0) * 6.0
}

#[cfg(feature = "fontdue-text")]
fn ascii_chars() -> Vec<char> {
    (32u8..=126u8).map(char::from).collect()
}

fn soft_card(scene: &mut GpuScene, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
    scene.push_rect(GpuRect::shadow(
        x,
        y + 4.0,
        w,
        h,
        radius,
        Color4::rgba(0.0, 0.0, 0.0, 0.24),
        10.0,
    ));
    panel(scene, x, y, w, h, radius, color);
}

fn panel(scene: &mut GpuScene, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
    scene.push_rect(GpuRect::fill(x, y, w, h, radius, color));
    scene.push_rect(GpuRect::border(x, y, w, h, radius, palette::BORDER));
}

pub mod palette {
    use super::Color4;

    pub const BG: Color4 = Color4::rgba(0.035, 0.043, 0.055, 1.0);
    pub const SIDEBAR: Color4 = Color4::rgba(0.070, 0.078, 0.090, 0.98);
    pub const TOPBAR: Color4 = Color4::rgba(0.055, 0.063, 0.078, 0.96);
    pub const ROW: Color4 = Color4::rgba(0.104, 0.118, 0.140, 0.74);
    pub const ACTIVE_ROW: Color4 = Color4::rgba(0.110, 0.230, 0.255, 0.42);
    pub const PANEL: Color4 = Color4::rgba(0.074, 0.083, 0.101, 0.94);
    pub const ASSISTANT: Color4 = Color4::rgba(0.090, 0.105, 0.125, 0.96);
    pub const USER: Color4 = Color4::rgba(0.080, 0.160, 0.180, 0.86);
    pub const COMPOSER: Color4 = Color4::rgba(0.075, 0.086, 0.105, 0.98);
    pub const BORDER: Color4 = Color4::rgba(0.220, 0.245, 0.275, 0.48);
    pub const ACCENT: Color4 = Color4::rgba(0.055, 0.624, 0.820, 1.0);
    pub const GREEN: Color4 = Color4::rgba(0.180, 0.760, 0.500, 1.0);
    pub const VIOLET: Color4 = Color4::rgba(0.560, 0.500, 0.940, 1.0);
    pub const AMBER: Color4 = Color4::rgba(0.930, 0.650, 0.220, 1.0);
    pub const ACCENT_TEXT: Color4 = Color4::rgba(0.940, 0.980, 1.000, 1.0);
    pub const TEXT: Color4 = Color4::rgba(0.930, 0.950, 0.970, 1.0);
    pub const MUTED: Color4 = Color4::rgba(0.560, 0.620, 0.700, 1.0);
}

fn glyph5x7(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        ':' => [
            0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        '>' => [
            0b10000, 0b01000, 0b00100, 0b00010, 0b00100, 0b01000, 0b10000,
        ],
        _ => [
            0b11111, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100,
        ],
    }
}

pub mod webgl2 {
    pub const VERT: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
uniform vec2 u_screen;
uniform vec4 u_rect;
out vec2 v_local;
out vec2 v_size;
void main() {
    vec2 px = u_rect.xy + a_pos * u_rect.zw;
    vec2 ndc = vec2(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
    v_local = a_pos * u_rect.zw;
    v_size = u_rect.zw;
}
"#;

    pub const FRAG: &str = r#"#version 300 es
precision highp float;
in vec2 v_local;
in vec2 v_size;
out vec4 out_color;
uniform vec4 u_color;
uniform float u_radius;
uniform float u_shadow;
uniform int u_mode;
float rounded_box(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + vec2(r);
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}
void main() {
    vec2 p = v_local - v_size * 0.5;
    float d = rounded_box(p, v_size * 0.5, u_radius);
    float aa = max(fwidth(d), 0.75);
    float alpha = 1.0 - smoothstep(0.0, aa, d);
    if (u_mode == 1) {
        float sd = rounded_box(p - vec2(0.0, -u_shadow * 0.18), v_size * 0.5, u_radius + u_shadow * 0.35);
        float blur = max(u_shadow, 1.0);
        alpha = 1.0 - smoothstep(-blur, blur, sd);
        out_color = vec4(u_color.rgb, u_color.a * alpha * 0.28);
    } else if (u_mode == 2) {
        float inner = rounded_box(p, v_size * 0.5 - vec2(1.25), max(u_radius - 1.25, 0.0));
        float border = (1.0 - smoothstep(0.0, aa, d)) * smoothstep(0.0, aa, inner);
        out_color = vec4(u_color.rgb, u_color.a * border);
    } else {
        out_color = vec4(u_color.rgb, u_color.a * alpha);
    }
}
"#;
}

#[cfg(all(feature = "gpu-gl", not(target_arch = "wasm32")))]
pub mod gl {
    use super::{Color4, GpuRect, GpuScene, RectMode};
    #[cfg(feature = "fontdue-text")]
    use super::{FontAtlas, TextQuad};
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_int, c_void};
    use std::ptr;

    const GL_COLOR_BUFFER_BIT: u32 = 0x0000_4000;
    const GL_ARRAY_BUFFER: u32 = 0x8892;
    const GL_DYNAMIC_DRAW: u32 = 0x88E8;
    const GL_FLOAT: u32 = 0x1406;
    const GL_FALSE: u8 = 0;
    const GL_TRIANGLES: u32 = 0x0004;
    const GL_VERTEX_SHADER: u32 = 0x8B31;
    const GL_FRAGMENT_SHADER: u32 = 0x8B30;
    const GL_COMPILE_STATUS: u32 = 0x8B81;
    const GL_LINK_STATUS: u32 = 0x8B82;
    const GL_BLEND: u32 = 0x0BE2;
    const GL_SRC_ALPHA: u32 = 0x0302;
    const GL_ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
    #[cfg(feature = "fontdue-text")]
    const GL_TEXTURE_2D: u32 = 0x0DE1;
    #[cfg(feature = "fontdue-text")]
    const GL_TEXTURE0: u32 = 0x84C0;
    #[cfg(feature = "fontdue-text")]
    const GL_RED: u32 = 0x1903;
    #[cfg(feature = "fontdue-text")]
    const GL_R8: u32 = 0x8229;
    #[cfg(feature = "fontdue-text")]
    const GL_UNSIGNED_BYTE: u32 = 0x1401;
    #[cfg(feature = "fontdue-text")]
    const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
    #[cfg(feature = "fontdue-text")]
    const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
    #[cfg(feature = "fontdue-text")]
    const GL_TEXTURE_WRAP_S: u32 = 0x2802;
    #[cfg(feature = "fontdue-text")]
    const GL_TEXTURE_WRAP_T: u32 = 0x2803;
    #[cfg(feature = "fontdue-text")]
    const GL_LINEAR: i32 = 0x2601;
    #[cfg(feature = "fontdue-text")]
    const GL_CLAMP_TO_EDGE: i32 = 0x812F;
    #[cfg(feature = "fontdue-text")]
    const GL_UNPACK_ALIGNMENT: u32 = 0x0CF5;

    #[link(name = "GL")]
    unsafe extern "C" {
        fn glViewport(x: c_int, y: c_int, width: c_int, height: c_int);
        fn glClearColor(r: f32, g: f32, b: f32, a: f32);
        fn glClear(mask: u32);
        fn glEnable(cap: u32);
        fn glBlendFunc(sfactor: u32, dfactor: u32);
        fn glCreateShader(shader_type: u32) -> u32;
        fn glShaderSource(
            shader: u32,
            count: c_int,
            string: *const *const c_char,
            length: *const c_int,
        );
        fn glCompileShader(shader: u32);
        fn glGetShaderiv(shader: u32, pname: u32, params: *mut c_int);
        fn glGetShaderInfoLog(
            shader: u32,
            buf_size: c_int,
            length: *mut c_int,
            info_log: *mut c_char,
        );
        fn glDeleteShader(shader: u32);
        fn glCreateProgram() -> u32;
        fn glAttachShader(program: u32, shader: u32);
        fn glLinkProgram(program: u32);
        fn glGetProgramiv(program: u32, pname: u32, params: *mut c_int);
        fn glGetProgramInfoLog(
            program: u32,
            buf_size: c_int,
            length: *mut c_int,
            info_log: *mut c_char,
        );
        fn glUseProgram(program: u32);
        fn glGetUniformLocation(program: u32, name: *const c_char) -> c_int;
        fn glUniform2f(location: c_int, v0: f32, v1: f32);
        fn glUniform4f(location: c_int, v0: f32, v1: f32, v2: f32, v3: f32);
        fn glUniform1f(location: c_int, v0: f32);
        fn glUniform1i(location: c_int, v0: c_int);
        fn glGenVertexArrays(n: c_int, arrays: *mut u32);
        fn glBindVertexArray(array: u32);
        fn glGenBuffers(n: c_int, buffers: *mut u32);
        fn glBindBuffer(target: u32, buffer: u32);
        fn glBufferData(target: u32, size: isize, data: *const c_void, usage: u32);
        fn glEnableVertexAttribArray(index: u32);
        fn glVertexAttribPointer(
            index: u32,
            size: c_int,
            ty: u32,
            normalized: u8,
            stride: c_int,
            pointer: *const c_void,
        );
        fn glDrawArrays(mode: u32, first: c_int, count: c_int);
        fn glDeleteBuffers(n: c_int, buffers: *const u32);
        fn glDeleteVertexArrays(n: c_int, arrays: *const u32);
        fn glDeleteProgram(program: u32);
        #[cfg(feature = "fontdue-text")]
        fn glGenTextures(n: c_int, textures: *mut u32);
        #[cfg(feature = "fontdue-text")]
        fn glBindTexture(target: u32, texture: u32);
        #[cfg(feature = "fontdue-text")]
        fn glTexParameteri(target: u32, pname: u32, param: c_int);
        #[cfg(feature = "fontdue-text")]
        fn glTexImage2D(
            target: u32,
            level: c_int,
            internalformat: c_int,
            width: c_int,
            height: c_int,
            border: c_int,
            format: u32,
            ty: u32,
            pixels: *const c_void,
        );
        #[cfg(feature = "fontdue-text")]
        fn glActiveTexture(texture: u32);
        #[cfg(feature = "fontdue-text")]
        fn glPixelStorei(pname: u32, param: c_int);
        #[cfg(feature = "fontdue-text")]
        fn glDeleteTextures(n: c_int, textures: *const u32);
    }

    const VERT: &str = r#"#version 330 core
layout(location = 0) in vec2 a_pos;
uniform vec2 u_screen;
uniform vec4 u_rect;
out vec2 v_local;
out vec2 v_size;
void main() {
    vec2 px = u_rect.xy + a_pos * u_rect.zw;
    vec2 ndc = vec2(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
    v_local = a_pos * u_rect.zw;
    v_size = u_rect.zw;
}
"#;

    #[cfg(feature = "fontdue-text")]
    const TEXT_VERT: &str = r#"#version 330 core
layout(location = 0) in vec4 a_data;
uniform vec2 u_screen;
out vec2 v_uv;
void main() {
    vec2 px = a_data.xy;
    vec2 ndc = vec2(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
    v_uv = a_data.zw;
}
"#;

    #[cfg(feature = "fontdue-text")]
    const TEXT_FRAG: &str = r#"#version 330 core
in vec2 v_uv;
out vec4 out_color;
uniform sampler2D u_tex;
uniform vec4 u_color;
void main() {
    float a = texture(u_tex, v_uv).r;
    out_color = vec4(u_color.rgb, u_color.a * a);
}
"#;

    const FRAG: &str = r#"#version 330 core
in vec2 v_local;
in vec2 v_size;
out vec4 out_color;
uniform vec4 u_color;
uniform float u_radius;
uniform float u_shadow;
uniform int u_mode;
float rounded_box(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + vec2(r);
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}
void main() {
    vec2 p = v_local - v_size * 0.5;
    float d = rounded_box(p, v_size * 0.5, u_radius);
    float aa = max(fwidth(d), 0.75);
    float alpha = 1.0 - smoothstep(0.0, aa, d);
    if (u_mode == 1) {
        float sd = rounded_box(p - vec2(0.0, -u_shadow * 0.18), v_size * 0.5, u_radius + u_shadow * 0.35);
        float blur = max(u_shadow, 1.0);
        alpha = 1.0 - smoothstep(-blur, blur, sd);
        out_color = vec4(u_color.rgb, u_color.a * alpha * 0.28);
    } else if (u_mode == 2) {
        float inner = rounded_box(p, v_size * 0.5 - vec2(1.25), max(u_radius - 1.25, 0.0));
        float border = (1.0 - smoothstep(0.0, aa, d)) * smoothstep(0.0, aa, inner);
        out_color = vec4(u_color.rgb, u_color.a * border);
    } else {
        out_color = vec4(u_color.rgb, u_color.a * alpha);
    }
}
"#;

    pub struct GlRenderer {
        program: u32,
        vao: u32,
        vbo: u32,
        u_screen: c_int,
        u_rect: c_int,
        u_color: c_int,
        u_radius: c_int,
        u_mode: c_int,
        u_shadow: c_int,
        #[cfg(feature = "fontdue-text")]
        text: Option<TextRenderer>,
    }

    impl GlRenderer {
        /// Create a renderer for the current OpenGL context.
        ///
        /// The caller must create and make current the GL context before calling
        /// this constructor.
        pub unsafe fn new_current_context() -> Result<Self, String> {
            let program = unsafe { glCreateProgram() };
            let vs = compile_shader(GL_VERTEX_SHADER, VERT)?;
            let fs = compile_shader(GL_FRAGMENT_SHADER, FRAG)?;
            unsafe {
                glAttachShader(program, vs);
                glAttachShader(program, fs);
                glLinkProgram(program);
                glDeleteShader(vs);
                glDeleteShader(fs);
            }
            check_program(program)?;

            let mut vao = 0;
            let mut vbo = 0;
            let verts: [f32; 12] = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0];
            unsafe {
                glGenVertexArrays(1, &mut vao);
                glBindVertexArray(vao);
                glGenBuffers(1, &mut vbo);
                glBindBuffer(GL_ARRAY_BUFFER, vbo);
                glBufferData(
                    GL_ARRAY_BUFFER,
                    (verts.len() * 4) as isize,
                    verts.as_ptr().cast(),
                    GL_DYNAMIC_DRAW,
                );
                glEnableVertexAttribArray(0);
                glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, 2 * 4, ptr::null());
            }

            Ok(Self {
                program,
                vao,
                vbo,
                u_screen: uniform(program, "u_screen"),
                u_rect: uniform(program, "u_rect"),
                u_color: uniform(program, "u_color"),
                u_radius: uniform(program, "u_radius"),
                u_mode: uniform(program, "u_mode"),
                u_shadow: uniform(program, "u_shadow"),
                #[cfg(feature = "fontdue-text")]
                text: None,
            })
        }

        #[cfg(feature = "fontdue-text")]
        pub unsafe fn new_current_context_with_font(atlas: &FontAtlas) -> Result<Self, String> {
            let mut renderer = unsafe { Self::new_current_context()? };
            renderer.text = Some(TextRenderer::new(atlas)?);
            Ok(renderer)
        }

        pub fn render(&self, width: i32, height: i32, scene: &GpuScene) {
            let clear = scene.clear;
            unsafe {
                glViewport(0, 0, width, height);
                glClearColor(clear.r, clear.g, clear.b, clear.a);
                glClear(GL_COLOR_BUFFER_BIT);
                glEnable(GL_BLEND);
                glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
                glUseProgram(self.program);
                glBindVertexArray(self.vao);
                glUniform2f(self.u_screen, width as f32, height as f32);
            }
            for rect in scene.rects() {
                self.draw_rect(*rect);
            }
            #[cfg(feature = "fontdue-text")]
            if let Some(text) = &self.text {
                text.render(width, height, scene.text_quads());
            }
        }

        fn draw_rect(&self, rect: GpuRect) {
            let mode = match rect.mode {
                RectMode::Fill => 0,
                RectMode::Shadow => 1,
                RectMode::Border => 2,
            };
            let Color4 { r, g, b, a } = rect.color;
            unsafe {
                glUniform4f(self.u_rect, rect.x, rect.y, rect.w, rect.h);
                glUniform4f(self.u_color, r, g, b, a);
                glUniform1f(self.u_radius, rect.radius);
                glUniform1i(self.u_mode, mode);
                glUniform1f(self.u_shadow, rect.shadow);
                glDrawArrays(GL_TRIANGLES, 0, 6);
            }
        }
    }

    #[cfg(feature = "fontdue-text")]
    struct TextRenderer {
        program: u32,
        vao: u32,
        vbo: u32,
        texture: u32,
        u_screen: c_int,
        u_color: c_int,
        u_tex: c_int,
    }

    #[cfg(feature = "fontdue-text")]
    impl TextRenderer {
        fn new(atlas: &FontAtlas) -> Result<Self, String> {
            let program = unsafe { glCreateProgram() };
            let vs = compile_shader(GL_VERTEX_SHADER, TEXT_VERT)?;
            let fs = compile_shader(GL_FRAGMENT_SHADER, TEXT_FRAG)?;
            unsafe {
                glAttachShader(program, vs);
                glAttachShader(program, fs);
                glLinkProgram(program);
                glDeleteShader(vs);
                glDeleteShader(fs);
            }
            check_program(program)?;

            let mut vao = 0;
            let mut vbo = 0;
            unsafe {
                glGenVertexArrays(1, &mut vao);
                glBindVertexArray(vao);
                glGenBuffers(1, &mut vbo);
                glBindBuffer(GL_ARRAY_BUFFER, vbo);
                glEnableVertexAttribArray(0);
                glVertexAttribPointer(0, 4, GL_FLOAT, GL_FALSE, 4 * 4, ptr::null());
            }

            let mut texture = 0;
            unsafe {
                glGenTextures(1, &mut texture);
                glBindTexture(GL_TEXTURE_2D, texture);
                glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
                glTexImage2D(
                    GL_TEXTURE_2D,
                    0,
                    GL_R8 as i32,
                    atlas.width as i32,
                    atlas.height as i32,
                    0,
                    GL_RED,
                    GL_UNSIGNED_BYTE,
                    atlas.alpha.as_ptr().cast(),
                );
            }

            Ok(Self {
                program,
                vao,
                vbo,
                texture,
                u_screen: uniform(program, "u_screen"),
                u_color: uniform(program, "u_color"),
                u_tex: uniform(program, "u_tex"),
            })
        }

        fn render(&self, width: i32, height: i32, quads: &[TextQuad]) {
            unsafe {
                glUseProgram(self.program);
                glBindVertexArray(self.vao);
                glActiveTexture(GL_TEXTURE0);
                glBindTexture(GL_TEXTURE_2D, self.texture);
                glUniform1i(self.u_tex, 0);
                glUniform2f(self.u_screen, width as f32, height as f32);
            }
            for quad in quads {
                self.draw_quad(*quad);
            }
        }

        fn draw_quad(&self, q: TextQuad) {
            let verts: [f32; 24] = [
                q.x,
                q.y,
                q.u0,
                q.v0,
                q.x + q.w,
                q.y,
                q.u1,
                q.v0,
                q.x + q.w,
                q.y + q.h,
                q.u1,
                q.v1,
                q.x,
                q.y,
                q.u0,
                q.v0,
                q.x + q.w,
                q.y + q.h,
                q.u1,
                q.v1,
                q.x,
                q.y + q.h,
                q.u0,
                q.v1,
            ];
            let Color4 { r, g, b, a } = q.color;
            unsafe {
                glUniform4f(self.u_color, r, g, b, a);
                glBindBuffer(GL_ARRAY_BUFFER, self.vbo);
                glBufferData(
                    GL_ARRAY_BUFFER,
                    (verts.len() * 4) as isize,
                    verts.as_ptr().cast(),
                    GL_DYNAMIC_DRAW,
                );
                glDrawArrays(GL_TRIANGLES, 0, 6);
            }
        }
    }

    #[cfg(feature = "fontdue-text")]
    impl Drop for TextRenderer {
        fn drop(&mut self) {
            unsafe {
                glDeleteTextures(1, &self.texture);
                glDeleteBuffers(1, &self.vbo);
                glDeleteVertexArrays(1, &self.vao);
                glDeleteProgram(self.program);
            }
        }
    }

    impl Drop for GlRenderer {
        fn drop(&mut self) {
            unsafe {
                glDeleteBuffers(1, &self.vbo);
                glDeleteVertexArrays(1, &self.vao);
                glDeleteProgram(self.program);
            }
        }
    }

    fn compile_shader(kind: u32, source: &str) -> Result<u32, String> {
        let shader = unsafe { glCreateShader(kind) };
        let c_src = CString::new(source).map_err(|error| error.to_string())?;
        let ptr = c_src.as_ptr();
        unsafe {
            glShaderSource(shader, 1, &ptr, ptr::null());
            glCompileShader(shader);
        }
        let mut ok = 0;
        unsafe {
            glGetShaderiv(shader, GL_COMPILE_STATUS, &mut ok);
        }
        if ok == 0 {
            let log = shader_log(shader);
            unsafe {
                glDeleteShader(shader);
            }
            Err(log)
        } else {
            Ok(shader)
        }
    }

    fn check_program(program: u32) -> Result<(), String> {
        let mut ok = 0;
        unsafe {
            glGetProgramiv(program, GL_LINK_STATUS, &mut ok);
        }
        if ok == 0 {
            Err(program_log(program))
        } else {
            Ok(())
        }
    }

    fn shader_log(shader: u32) -> String {
        let mut buf = vec![0i8; 2048];
        let mut len = 0;
        unsafe {
            glGetShaderInfoLog(shader, buf.len() as i32, &mut len, buf.as_mut_ptr());
            CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
        }
    }

    fn program_log(program: u32) -> String {
        let mut buf = vec![0i8; 2048];
        let mut len = 0;
        unsafe {
            glGetProgramInfoLog(program, buf.len() as i32, &mut len, buf.as_mut_ptr());
            CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
        }
    }

    fn uniform(program: u32, name: &str) -> i32 {
        let Ok(c) = CString::new(name) else {
            return -1;
        };
        unsafe { glGetUniformLocation(program, c.as_ptr()) }
    }
}
