//! Composite surfaces to the scanout framebuffer.

use crate::compositor::surface::{Surface, SurfaceBuffer, SurfaceTree};
use crate::render::fb::Framebuffer;
use crate::render::shm::ShmManager;

/// Composite all surfaces onto the scanout buffer.
///
/// Surfaces are composited back-to-front (first = bottom, last = top).
pub fn composite(
    fb: &mut Framebuffer,
    tree: &SurfaceTree,
    shm: &ShmManager,
    surface_ids: &[u32], // z-ordered list of surface ids
) {
    // Clear to a dark background
    fb.clear(0x1a, 0x1a, 0x2e, 0xff);

    for &surface_id in surface_ids {
        if let Some(surface) = tree.get(surface_id) {
            render_surface(fb, surface, shm);
        }
    }
}

/// Render a single surface onto the framebuffer.
fn render_surface(fb: &mut Framebuffer, surface: &Surface, shm: &ShmManager) {
    let buf = match &surface.buffer {
        Some(b) => b,
        None => return,
    };

    match buf {
        SurfaceBuffer::Shm { pool_fd, offset, width, height, stride, format } => {
            // Use cached SHM pool mapping instead of iterating blindly
            if let Some((_, pool)) = shm.pools().find(|(_, p)| p.fd == *pool_fd) {
                let data = match pool.read(*offset as usize, (*stride as usize) * (*height as usize)) {
                    Some(d) => d,
                    None => return,
                };
                blit_xrgb8888(fb, data, surface.x, surface.y, *width, *height, *stride, *format);
            }
        }
        SurfaceBuffer::Dumb { handle: _, fb_id: _, width, height, pitch: _ } => {
            // Dumb buffers are already in VRAM — handled by KMS page flip
            let _ = (*width, *height);
        }
        SurfaceBuffer::DmaBuf { .. } => {
            // DMA-BUF buffers are handled in the main render_and_flip loop
        }
        SurfaceBuffer::Null => {}
    }
}

/// Blit an XRGB8888 buffer onto the framebuffer.
/// Uses integer alpha blending for performance.
fn blit_xrgb8888(
    fb: &mut Framebuffer,
    src: &[u8],
    dst_x: i32,
    dst_y: i32,
    src_width: i32,
    src_height: i32,
    src_stride: i32,
    format: u32,
) {
    for sy in 0..src_height {
        for sx in 0..src_width {
            let dx = dst_x + sx;
            let dy = dst_y + sy;

            if dx < 0 || dy < 0 || dx as u32 >= fb.width || dy as u32 >= fb.height {
                continue;
            }

            let src_offset = (sy * src_stride + sx * 4) as usize;
            if src_offset + 4 > src.len() {
                continue;
            }

            let dst_offset = (dy as u32 * fb.stride + dx as u32 * 4) as usize;
            if dst_offset + 4 > fb.pixels.len() {
                continue;
            }

            // Target framebuffer is XRGB8888 (little-endian memory: [B, G, R, X])
            match format {
                0x34325258 /* XRGB8888 */ => {
                    // Same format — direct copy
                    fb.pixels[dst_offset] = src[src_offset];
                    fb.pixels[dst_offset + 1] = src[src_offset + 1];
                    fb.pixels[dst_offset + 2] = src[src_offset + 2];
                    fb.pixels[dst_offset + 3] = 0xff;
                }
                0x34325241 /* ARGB8888 */ => {
                    // ARGB8888 little-endian: [B, G, R, A] — same RGB order, needs alpha
                    let alpha = src[src_offset + 3];
                    if alpha == 0 { continue; }
                    if alpha == 255 {
                        fb.pixels[dst_offset] = src[src_offset];
                        fb.pixels[dst_offset + 1] = src[src_offset + 1];
                        fb.pixels[dst_offset + 2] = src[src_offset + 2];
                    } else {
                        // Integer alpha blending
                        let a_inv = 255u32 - alpha as u32;
                        fb.pixels[dst_offset] =
                            ((src[src_offset] as u32 * alpha as u32 + fb.pixels[dst_offset] as u32 * a_inv) / 255) as u8;
                        fb.pixels[dst_offset + 1] =
                            ((src[src_offset + 1] as u32 * alpha as u32 + fb.pixels[dst_offset + 1] as u32 * a_inv) / 255) as u8;
                        fb.pixels[dst_offset + 2] =
                            ((src[src_offset + 2] as u32 * alpha as u32 + fb.pixels[dst_offset + 2] as u32 * a_inv) / 255) as u8;
                    }
                    fb.pixels[dst_offset + 3] = 0xff;
                }
                0x34324241 /* ABGR8888 */ => {
                    // ABGR8888 little-endian: [R, G, B, A] — swap R↔B
                    let alpha = src[src_offset + 3];
                    if alpha == 0 { continue; }
                    if alpha == 255 {
                        fb.pixels[dst_offset] = src[src_offset + 2]; // B
                        fb.pixels[dst_offset + 1] = src[src_offset + 1]; // G
                        fb.pixels[dst_offset + 2] = src[src_offset]; // R
                    } else {
                        let a_inv = 255u32 - alpha as u32;
                        let src_b = src[src_offset + 2] as u32;
                        let src_g = src[src_offset + 1] as u32;
                        let src_r = src[src_offset] as u32;
                        fb.pixels[dst_offset] =
                            ((src_b * alpha as u32 + fb.pixels[dst_offset] as u32 * a_inv) / 255) as u8;
                        fb.pixels[dst_offset + 1] =
                            ((src_g * alpha as u32 + fb.pixels[dst_offset + 1] as u32 * a_inv) / 255) as u8;
                        fb.pixels[dst_offset + 2] =
                            ((src_r * alpha as u32 + fb.pixels[dst_offset + 2] as u32 * a_inv) / 255) as u8;
                    }
                    fb.pixels[dst_offset + 3] = 0xff;
                }
                _ => {
                    // Default: treat as XRGB8888
                    fb.pixels[dst_offset] = src[src_offset + 2];
                    fb.pixels[dst_offset + 1] = src[src_offset + 1];
                    fb.pixels[dst_offset + 2] = src[src_offset];
                    fb.pixels[dst_offset + 3] = 0xff;
                }
            }
        }
    }
}
