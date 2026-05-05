//! GPU compositing via EGL + OpenGL.
//!
//! Imports DMA-BUF surfaces as EGLImages, renders textured quads via GL.

pub mod compositor;
pub mod egl;
pub mod gl;
