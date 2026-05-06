struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.pos = vec4<f32>(input.pos, 0.0, 1.0);
    out.uv = input.uv;
    return out;
}

@group(0) @binding(0) var base_tex: texture_2d<f32>;
@group(0) @binding(1) var base_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(base_tex, base_sampler, input.uv);
    return vec4<f32>(color.rgb, 1.0);
}
