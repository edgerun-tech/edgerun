struct Screen {
    size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;
@group(1) @binding(0) var glyph_tex: texture_2d<f32>;
@group(1) @binding(1) var glyph_sampler: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.04045);
    let below = c / vec3<f32>(12.92);
    let above = pow((c + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    return select(below, above, c > cutoff);
}

@vertex
fn vs_main(input: VsIn) -> VsOut {
    let ndc = vec2<f32>(
        input.pos.x / screen.size.x * 2.0 - 1.0,
        1.0 - input.pos.y / screen.size.y * 2.0,
    );
    var out: VsOut;
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let sample = textureSample(glyph_tex, glyph_sampler, input.uv);
    let tex_rgb = srgb_to_linear(sample.rgb);
    let tint = srgb_to_linear(input.color.rgb);
    let rgb = tex_rgb * tint;
    let alpha = sample.a * input.color.a;
    return vec4<f32>(rgb, alpha);
}
