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

#[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
use super::IconQuad;
#[cfg(feature = "fontdue-text")]
use super::TextQuad;
use super::{GpuHit, GpuRect, GpuScene, HitKind, RectMode};
use std::vec::Vec;

pub type UiWebGlSurfaceHost<A> = super::UiSurfaceHost<A>;

pub fn instantiate_webgl_app<A: super::UiSurfaceApp>(
    app: A,
    clear: super::Color4,
) -> UiWebGlSurfaceHost<A> {
    super::UiSurfaceHost::new(app, clear)
}

pub const RECT_FLOAT_STRIDE: u32 = 15;
pub const TEXT_VERTEX_FLOAT_STRIDE: u32 = 8;
pub const ICON_VERTEX_FLOAT_STRIDE: u32 = 8;
pub const HIT_FLOAT_STRIDE: u32 = 6;
pub const GPU_SCENE_ABI_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiRendererBackend {
    #[default]
    WebGl2,
    NativeGl,
    Software,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiRendererCapabilities {
    pub backend: UiRendererBackend,
    pub scene_abi_version: u32,
    pub packed_rects: bool,
    pub packed_text_vertices: bool,
    pub packed_icon_vertices: bool,
    pub packed_hits: bool,
    pub font_atlas: bool,
    pub icon_atlas: bool,
    pub fallback_bitmap_text: bool,
}

impl UiRendererCapabilities {
    pub const fn webgl2() -> Self {
        Self {
            backend: UiRendererBackend::WebGl2,
            scene_abi_version: GPU_SCENE_ABI_VERSION,
            packed_rects: true,
            packed_text_vertices: true,
            packed_icon_vertices: true,
            packed_hits: true,
            font_atlas: cfg!(feature = "fontdue-text"),
            icon_atlas: cfg!(any(
                feature = "tabler-svg-atlas",
                feature = "lucide-svg-atlas"
            )),
            fallback_bitmap_text: true,
        }
    }

    pub const fn native_gl() -> Self {
        Self {
            backend: UiRendererBackend::NativeGl,
            scene_abi_version: GPU_SCENE_ABI_VERSION,
            packed_rects: true,
            packed_text_vertices: true,
            packed_icon_vertices: true,
            packed_hits: true,
            font_atlas: cfg!(feature = "fontdue-text"),
            icon_atlas: cfg!(any(
                feature = "tabler-svg-atlas",
                feature = "lucide-svg-atlas"
            )),
            fallback_bitmap_text: true,
        }
    }

    pub const fn software() -> Self {
        Self {
            backend: UiRendererBackend::Software,
            scene_abi_version: GPU_SCENE_ABI_VERSION,
            packed_rects: false,
            packed_text_vertices: false,
            packed_icon_vertices: false,
            packed_hits: false,
            font_atlas: false,
            icon_atlas: false,
            fallback_bitmap_text: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PackedGpuSceneStats {
    pub rect_count: usize,
    pub text_vertex_count: usize,
    pub icon_vertex_count: usize,
    pub hit_count: usize,
    pub rect_float_count: usize,
    pub text_float_count: usize,
    pub icon_float_count: usize,
    pub hit_float_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PackedGpuScene {
    pub rects: Vec<f32>,
    pub text_vertices: Vec<f32>,
    pub icon_vertices: Vec<f32>,
    pub hits: Vec<f32>,
}

impl PackedGpuScene {
    pub fn pack(&mut self, scene: &GpuScene) {
        self.pack_rects(scene.rects());
        #[cfg(feature = "fontdue-text")]
        self.pack_text_quads(scene.text_quads());
        #[cfg(not(feature = "fontdue-text"))]
        self.text_vertices.clear();
        #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
        self.pack_icon_quads(scene.icon_quads());
        #[cfg(not(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas")))]
        self.icon_vertices.clear();
        self.pack_hits(scene.hits());
    }

    pub fn rect_buffer(&self) -> &[f32] {
        &self.rects
    }

    pub fn text_vertex_buffer(&self) -> &[f32] {
        &self.text_vertices
    }

    pub fn icon_vertex_buffer(&self) -> &[f32] {
        &self.icon_vertices
    }

    pub fn hit_buffer(&self) -> &[f32] {
        &self.hits
    }

    pub fn stats(&self) -> PackedGpuSceneStats {
        PackedGpuSceneStats {
            rect_count: self.rects.len() / RECT_FLOAT_STRIDE as usize,
            text_vertex_count: self.text_vertices.len() / TEXT_VERTEX_FLOAT_STRIDE as usize,
            icon_vertex_count: self.icon_vertices.len() / ICON_VERTEX_FLOAT_STRIDE as usize,
            hit_count: self.hits.len() / HIT_FLOAT_STRIDE as usize,
            rect_float_count: self.rects.len(),
            text_float_count: self.text_vertices.len(),
            icon_float_count: self.icon_vertices.len(),
            hit_float_count: self.hits.len(),
        }
    }

    pub fn clear(&mut self) {
        self.rects.clear();
        self.text_vertices.clear();
        self.icon_vertices.clear();
        self.hits.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
            && self.text_vertices.is_empty()
            && self.icon_vertices.is_empty()
            && self.hits.is_empty()
    }

    fn pack_rects(&mut self, rects: &[GpuRect]) {
        self.rects.clear();
        self.rects.reserve(rects.len() * RECT_FLOAT_STRIDE as usize);
        for rect in rects {
            push_packed_rect(&mut self.rects, rect);
        }
    }

    #[cfg(feature = "fontdue-text")]
    fn pack_text_quads(&mut self, quads: &[TextQuad]) {
        self.text_vertices.clear();
        self.text_vertices
            .reserve(quads.len() * TEXT_VERTEX_FLOAT_STRIDE as usize * 6);
        for quad in quads {
            push_packed_text_quad(&mut self.text_vertices, quad);
        }
    }

    #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
    fn pack_icon_quads(&mut self, quads: &[IconQuad]) {
        self.icon_vertices.clear();
        self.icon_vertices
            .reserve(quads.len() * ICON_VERTEX_FLOAT_STRIDE as usize * 6);
        for quad in quads {
            push_packed_icon_quad(&mut self.icon_vertices, quad);
        }
    }

    fn pack_hits(&mut self, hits: &[GpuHit]) {
        self.hits.clear();
        self.hits.reserve(hits.len() * HIT_FLOAT_STRIDE as usize);
        for hit in hits {
            self.hits.extend_from_slice(&[
                hit_kind_code(hit.kind) as f32,
                (hit.id & 0x00ff_ffff) as f32,
                hit.x,
                hit.y,
                hit.w,
                hit.h,
            ]);
        }
    }
}

fn push_packed_rect(packed: &mut Vec<f32>, rect: &GpuRect) {
    packed.extend_from_slice(&[
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        rect.radius,
        rect.shadow,
        rect.color.r,
        rect.color.g,
        rect.color.b,
        rect.color.a,
        rect.color2.r,
        rect.color2.g,
        rect.color2.b,
        rect.color2.a,
        rect_mode_code(rect.mode) as f32,
    ]);
}

#[cfg(feature = "fontdue-text")]
fn push_packed_text_quad(packed: &mut Vec<f32>, quad: &TextQuad) {
    push_textured_vertex(packed, quad.x, quad.y, quad.u0, quad.v0, quad.color);
    push_textured_vertex(
        packed,
        quad.x + quad.w,
        quad.y,
        quad.u1,
        quad.v0,
        quad.color,
    );
    push_textured_vertex(
        packed,
        quad.x + quad.w,
        quad.y + quad.h,
        quad.u1,
        quad.v1,
        quad.color,
    );
    push_textured_vertex(packed, quad.x, quad.y, quad.u0, quad.v0, quad.color);
    push_textured_vertex(
        packed,
        quad.x + quad.w,
        quad.y + quad.h,
        quad.u1,
        quad.v1,
        quad.color,
    );
    push_textured_vertex(
        packed,
        quad.x,
        quad.y + quad.h,
        quad.u0,
        quad.v1,
        quad.color,
    );
}

#[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
fn push_packed_icon_quad(packed: &mut Vec<f32>, quad: &IconQuad) {
    push_textured_vertex(packed, quad.x, quad.y, quad.u0, quad.v0, quad.color);
    push_textured_vertex(
        packed,
        quad.x + quad.w,
        quad.y,
        quad.u1,
        quad.v0,
        quad.color,
    );
    push_textured_vertex(
        packed,
        quad.x + quad.w,
        quad.y + quad.h,
        quad.u1,
        quad.v1,
        quad.color,
    );
    push_textured_vertex(packed, quad.x, quad.y, quad.u0, quad.v0, quad.color);
    push_textured_vertex(
        packed,
        quad.x + quad.w,
        quad.y + quad.h,
        quad.u1,
        quad.v1,
        quad.color,
    );
    push_textured_vertex(
        packed,
        quad.x,
        quad.y + quad.h,
        quad.u0,
        quad.v1,
        quad.color,
    );
}

#[cfg(any(
    feature = "fontdue-text",
    any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas")
))]
fn push_textured_vertex(
    packed: &mut Vec<f32>,
    x: f32,
    y: f32,
    u: f32,
    v: f32,
    color: super::Color4,
) {
    packed.extend_from_slice(&[x, y, u, v, color.r, color.g, color.b, color.a]);
}

pub const fn hit_kind_code(kind: HitKind) -> u32 {
    match kind {
        HitKind::Contact => 1,
        HitKind::Composer => 2,
        HitKind::Send => 3,
        HitKind::Button => 4,
        HitKind::Tab => 5,
        HitKind::Toggle => 6,
        HitKind::ListRow => 7,
        HitKind::Input => 8,
        HitKind::TextArea => 9,
        HitKind::Slider => 10,
        HitKind::MenuItem => 11,
        HitKind::TransactionRow => 12,
        HitKind::ScrollArea => 13,
        HitKind::Scrollbar => 24,
        HitKind::WorkspaceTab => 14,
        HitKind::WorkspaceClose => 15,
        HitKind::WorkspaceSplit => 16,
        HitKind::Checkbox => 17,
        HitKind::Radio => 18,
        HitKind::Select => 19,
        HitKind::Breadcrumb => 20,
        HitKind::TreeItem => 21,
        HitKind::AppLauncherItem => 22,
        HitKind::ShellLauncher => 23,
    }
}

pub const fn rect_mode_code(mode: RectMode) -> u32 {
    match mode {
        RectMode::Fill => 0,
        RectMode::Shadow => 1,
        RectMode::Border => 2,
        RectMode::LinearGradient => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::{GpuRect, palette};

    #[test]
    fn packed_scene_stats_report_counts_and_floats() {
        let mut scene = GpuScene::new(palette::BG);
        scene.push_rect(GpuRect::fill(0.0, 0.0, 10.0, 10.0, 0.0, palette::TEXT));
        let mut packed = PackedGpuScene::default();

        packed.pack(&scene);

        assert_eq!(
            packed.stats(),
            PackedGpuSceneStats {
                rect_count: 1,
                text_vertex_count: 0,
                icon_vertex_count: 0,
                hit_count: 0,
                rect_float_count: RECT_FLOAT_STRIDE as usize,
                text_float_count: 0,
                icon_float_count: 0,
                hit_float_count: 0,
            }
        );
        assert!(!packed.is_empty());
        packed.clear();
        assert!(packed.is_empty());
    }

    #[test]
    fn renderer_capabilities_commit_to_scene_abi_version() {
        let web = UiRendererCapabilities::webgl2();
        let native = UiRendererCapabilities::native_gl();
        let software = UiRendererCapabilities::software();

        assert_eq!(web.scene_abi_version, GPU_SCENE_ABI_VERSION);
        assert_eq!(native.scene_abi_version, GPU_SCENE_ABI_VERSION);
        assert_eq!(software.scene_abi_version, GPU_SCENE_ABI_VERSION);
        assert!(web.packed_rects);
        assert!(web.packed_icon_vertices);
        assert!(!software.packed_rects);
        assert!(!software.packed_icon_vertices);
    }

    #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
    #[test]
    fn packed_scene_includes_icon_vertices() {
        let mut scene = GpuScene::new(palette::BG);
        scene.push_icon_quad(
            crate::gpu::UiRect::new(1.0, 2.0, 12.0, 14.0),
            crate::gpu::UiIconAtlasRect {
                name: "fixture",
                x: 0,
                y: 0,
                w: 12,
                h: 14,
                u0: 0.1,
                v0: 0.2,
                u1: 0.7,
                v1: 0.8,
            },
            palette::TEXT,
        );
        let mut packed = PackedGpuScene::default();

        packed.pack(&scene);

        assert_eq!(packed.stats().icon_vertex_count, 6);
        assert_eq!(
            packed.stats().icon_float_count,
            ICON_VERTEX_FLOAT_STRIDE as usize * 6
        );
        assert_eq!(&packed.icon_vertex_buffer()[0..4], &[1.0, 2.0, 0.1, 0.2]);
    }
}
