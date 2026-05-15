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

#[cfg(feature = "fontdue-text")]
use super::TextQuad;
use super::{GpuHit, GpuRect, GpuScene, HitKind, RectMode};
use std::vec::Vec;

pub const RECT_FLOAT_STRIDE: u32 = 11;
pub const TEXT_VERTEX_FLOAT_STRIDE: u32 = 8;
pub const HIT_FLOAT_STRIDE: u32 = 6;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PackedGpuScene {
    pub rects: Vec<f32>,
    pub text_vertices: Vec<f32>,
    pub hits: Vec<f32>,
}

impl PackedGpuScene {
    pub fn pack(&mut self, scene: &GpuScene) {
        self.pack_rects(scene.rects());
        #[cfg(feature = "fontdue-text")]
        self.pack_text_quads(scene.text_quads());
        #[cfg(not(feature = "fontdue-text"))]
        self.text_vertices.clear();
        self.pack_hits(scene.hits());
    }

    pub fn rect_buffer(&self) -> &[f32] {
        &self.rects
    }

    pub fn text_vertex_buffer(&self) -> &[f32] {
        &self.text_vertices
    }

    pub fn hit_buffer(&self) -> &[f32] {
        &self.hits
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
        rect_mode_code(rect.mode) as f32,
    ]);
}

#[cfg(feature = "fontdue-text")]
fn push_packed_text_quad(packed: &mut Vec<f32>, quad: &TextQuad) {
    push_text_vertex(packed, quad.x, quad.y, quad.u0, quad.v0, quad);
    push_text_vertex(packed, quad.x + quad.w, quad.y, quad.u1, quad.v0, quad);
    push_text_vertex(
        packed,
        quad.x + quad.w,
        quad.y + quad.h,
        quad.u1,
        quad.v1,
        quad,
    );
    push_text_vertex(packed, quad.x, quad.y, quad.u0, quad.v0, quad);
    push_text_vertex(
        packed,
        quad.x + quad.w,
        quad.y + quad.h,
        quad.u1,
        quad.v1,
        quad,
    );
    push_text_vertex(packed, quad.x, quad.y + quad.h, quad.u0, quad.v1, quad);
}

#[cfg(feature = "fontdue-text")]
fn push_text_vertex(packed: &mut Vec<f32>, x: f32, y: f32, u: f32, v: f32, quad: &TextQuad) {
    packed.extend_from_slice(&[
        x,
        y,
        u,
        v,
        quad.color.r,
        quad.color.g,
        quad.color.b,
        quad.color.a,
    ]);
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
        HitKind::Scrollbar => 13,
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
    }
}
