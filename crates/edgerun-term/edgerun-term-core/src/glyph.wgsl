struct Screen {
    size: vec2<f32>,
    flags: vec2<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;
@group(1) @binding(0) var glyph_tex: texture_2d<f32>;
@group(1) @binding(1) var glyph_sampler: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) rect: vec4<f32>,
    @location(3) uv_rect: vec4<f32>,
    @location(4) color: vec4<f32>,
    @location(5) flags: f32,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) flags: f32,
};

const MSDF_SPREAD: f32 = 4.0;

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.04045);
    let below = c / vec3<f32>(12.92);
    let above = pow((c + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    return select(below, above, c > cutoff);
}

fn smoothstep_edge(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

fn median(a: f32, b: f32, c: f32) -> f32 {
    return max(min(a, b), min(max(a, b), c));
}

@vertex
fn vs_main(input: VsIn) -> VsOut {
    let pos = mix(input.rect.xy, input.rect.zw, input.pos);
    let uv = mix(input.uv_rect.xy, input.uv_rect.zw, input.uv);
    let ndc = vec2<f32>(
        pos.x / screen.size.x * 2.0 - 1.0,
        1.0 - pos.y / screen.size.y * 2.0,
    );
    var out: VsOut;
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = uv;
    out.color = input.color;
    out.flags = input.flags;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let sample = textureSample(glyph_tex, glyph_sampler, input.uv);
    let tint = srgb_to_linear(input.color.rgb);
    // Treat the texture alpha as either coverage or an MSDF encoded in [0,1]
    // centered at 0.5. Use a smoothstep around 0.5 for smooth edges and scalable text.
    let use_msdf = input.flags <= 0.5;
    var rgb = tint;
    if !use_msdf {
        let tex_rgb = srgb_to_linear(sample.rgb);
        rgb = tex_rgb * tint;
    }
    let dist = median(sample.r, sample.g, sample.b);
    let w = min(max(fwidth(dist), screen.flags.y), 0.5 / MSDF_SPREAD);
    var alpha: f32 = sample.a;
    if use_msdf {
        alpha = smoothstep_edge(0.5 - w, 0.5 + w, dist);
    }
    return vec4<f32>(rgb, alpha * input.color.a);
}
