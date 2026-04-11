/// edgerun-wgpu — WebGPU host for the generated WGSL rasterizer.
///
/// This crate provides the GPU pipeline that replaces the CPU scanline
/// rasterizer. It uploads styled rectangles as a uniform buffer and
/// renders via a generated fragment shader.
pub mod pipeline;
pub mod uniforms;
pub mod render;
pub mod layout_compute;
