//! Server-side rendering functions for the compositor.

use crate::compositor::shell::Shell;
use crate::compositor::surface::{
    buffer_layout, DamageRect, SurfaceBuffer, SurfaceSampleRect, SurfaceTree,
};
use crate::drm;
use crate::drm::device::DrmDevice;
use crate::drm::dumb::DumbBuffer;
use crate::drm::kms;
use crate::libc;
use crate::render::cursor::Cursor;
use crate::render::shm::{read_shm_buffer_with_fallback, ShmManager};
use std::io;

/// Accumulated damage regions for incremental rendering.
pub struct DamageAccumulator {
    rects: Vec<DamageRect>,
    /// Whether to render the entire screen next frame.
    pub damage_all: bool,
}

impl DamageAccumulator {
    pub fn new() -> Self {
        Self {
            rects: Vec::new(),
            damage_all: true,
        }
    }

    /// Add a damage rectangle.
    pub fn add(&mut self, rect: DamageRect) {
        self.rects.push(rect);
    }

    /// Mark the entire output as damaged.
    pub fn mark_all(&mut self) {
        self.damage_all = true;
    }

    /// Take the current damage rectangles and reset for the next frame.
    pub fn take_rects(&mut self) -> Vec<DamageRect> {
        std::mem::take(&mut self.rects)
    }

    /// Check if there is any pending damage.
    pub fn is_dirty(&self) -> bool {
        self.damage_all || !self.rects.is_empty()
    }
}

/// Render all surfaces to the scanout buffer and issue a page flip.
///
/// The current CPU renderer recomposites the full scene each frame, so the
/// scanout buffer is cleared before every pass and queued damage is consumed.
pub fn render_and_flip(
    dumb: &mut DumbBuffer,
    fb_id: u32,
    crtc_id: u32,
    drm_device: &DrmDevice,
    surfaces: &SurfaceTree,
    shell: &Shell,
    cursor: &Cursor,
    shm: &ShmManager,
    damage: &mut DamageAccumulator,
    old_fb_ids: &mut Vec<u32>,
) -> io::Result<u32> {
    // Map the dumb buffer once — DumbBuffer::map() caches the mapping internally
    let fb_slice = match dumb.map() {
        Ok(p) => p,
        Err(e) => return Err(e),
    };
    let pixels = unsafe { std::slice::from_raw_parts_mut(fb_slice.as_mut_ptr(), fb_slice.len()) };

    let width = dumb.width;
    let height = dumb.height;
    let stride = dumb.pitch;

    clear_xrgb8888(pixels, width, height, stride, [0x2e, 0x1a, 0x1a, 0xff]);
    damage.damage_all = false;
    damage.take_rects();

    // Composite layer surfaces: background + bottom layers (below toplevels)
    for ls in shell.layer_surfaces_render_order() {
        if ls.layer > 1 {
            break;
        } // only background(0) and bottom(1)
        if let Some(surface) = surfaces.get(ls.surface_id) {
            if surface.buffer.is_some() {
                if let Some(ref buf) = surface.buffer {
                    let transform = surface.buffer_transform;
                    let (logical_w, logical_h) = surface.logical_size();
                    let sample = surface.sample_rect();
                    blit_surface_buffer(
                        buf, pixels, ls.x, ls.y, logical_w, logical_h, sample, width, height,
                        stride, shm, transform,
                    );
                }
            }
        }
    }

    // Composite surfaces in z-order (toplevels + subsurfaces)
    // Always render surfaces with buffers — surfaces are composited
    // back-to-front onto the cleared framebuffer each frame.
    for toplevel in shell.toplevels_render_order() {
        let surface_id = toplevel.surface_id;
        if let Some(surface) = surfaces.get(surface_id) {
            if surface.buffer.is_none() {
                continue;
            }
            if let Some(ref buf) = surface.buffer {
                let transform = surface.buffer_transform;
                let (logical_w, logical_h) = surface.logical_size();
                let sample = surface.sample_rect();
                blit_surface_buffer(
                    buf, pixels, surface.x, surface.y, logical_w, logical_h, sample, width, height,
                    stride, shm, transform,
                );
            }
        }

        // Composite subsurfaces
        let parent_pos = surfaces
            .get(surface_id)
            .map(|surface| (surface.x, surface.y))
            .unwrap_or((0, 0));
        for sub in shell.subsurfaces.for_parent_render_order(surface_id) {
            if let Some(sub_surface) = surfaces.get(sub.surface_id) {
                if sub_surface.buffer.is_some() {
                    if let Some(ref buf) = sub_surface.buffer {
                        let transform = sub_surface.buffer_transform;
                        let (sub_x, sub_y) = sub.output_position(parent_pos.0, parent_pos.1);
                        let (logical_w, logical_h) = sub_surface.logical_size();
                        let sample = sub_surface.sample_rect();
                        blit_surface_buffer(
                            buf, pixels, sub_x, sub_y, logical_w, logical_h, sample, width, height,
                            stride, shm, transform,
                        );
                    }
                }
            }
        }
    }

    // Composite layer surfaces: top + overlay layers (above toplevels)
    for ls in shell.layer_surfaces_render_order() {
        if ls.layer < 2 {
            continue;
        } // only top(2) and overlay(3)
        if let Some(surface) = surfaces.get(ls.surface_id) {
            if surface.buffer.is_some() {
                if let Some(ref buf) = surface.buffer {
                    let transform = surface.buffer_transform;
                    let (logical_w, logical_h) = surface.logical_size();
                    let sample = surface.sample_rect();
                    blit_surface_buffer(
                        buf, pixels, ls.x, ls.y, logical_w, logical_h, sample, width, height,
                        stride, shm, transform,
                    );
                }
            }
        }
    }

    // Draw native EdgeRun system UI above Wayland surfaces and below cursor.
    crate::render::system_ui::draw_system_ui_overlay(pixels, width, height, stride);

    // Draw cursor
    cursor.draw(pixels, width, height, stride);

    // Page flip to display the rendered buffer
    let new_fb_id = match dumb.add_fb() {
        Ok(id) => id,
        Err(e) => return Err(e),
    };

    // Determine page flip flags based on topmost surface tearing hint.
    // Check the frontmost toplevel for async/tearing preference.
    let mut flip_flags = crate::drm::ioctl::page_flip::PAGE_FLIP_EVENT;
    if frontmost_visible_toplevel_allows_tearing(shell, surfaces) {
        flip_flags |= crate::drm::ioctl::page_flip::PAGE_FLIP_ASYNC;
    }

    let user_data = kms::next_flip_serial();
    if let Err(e) = kms::page_flip(
        drm_device.as_raw_fd(),
        crtc_id,
        new_fb_id,
        flip_flags,
        user_data,
    ) {
        let _ = kms::rmfb(drm_device.as_raw_fd(), new_fb_id);
        return Err(e);
    }

    // Track old FB IDs for cleanup only after the kernel accepts the new scanout.
    if fb_id != 0 {
        old_fb_ids.push(fb_id);
    }

    Ok(new_fb_id)
}

fn frontmost_visible_toplevel_allows_tearing(shell: &Shell, surfaces: &SurfaceTree) -> bool {
    for &tl_id in shell.stack.iter().rev() {
        let Some(toplevel) = shell.toplevels.get(&tl_id) else {
            continue;
        };
        let Some(surface) = surfaces.get(toplevel.surface_id) else {
            continue;
        };
        if surface.buffer.is_some() {
            return surface.tearing_hint == 2;
        }
    }
    false
}

fn clear_xrgb8888(pixels: &mut [u8], width: u32, height: u32, stride: u32, bgra: [u8; 4]) {
    let row_bytes = (width as usize).saturating_mul(4);
    let stride = stride as usize;
    if row_bytes == 0 || stride < row_bytes {
        return;
    }

    let mut row = vec![0u8; row_bytes];
    for px in row.chunks_exact_mut(4) {
        px.copy_from_slice(&bgra);
    }

    for y in 0..height as usize {
        let start = y.saturating_mul(stride);
        let end = start.saturating_add(row_bytes);
        let Some(dst) = pixels.get_mut(start..end) else {
            break;
        };
        dst.copy_from_slice(&row);
    }
}

/// Blit a surface buffer onto the pixel array.
///
/// Uses cached SHM pool mappings instead of re-mmap/munmap per call.
/// Uses integer alpha blending for performance.
fn blit_surface_buffer(
    buf: &SurfaceBuffer,
    pixels: &mut [u8],
    origin_x: i32,
    origin_y: i32,
    draw_w: u32,
    draw_h: u32,
    sample: SurfaceSampleRect,
    output_width: u32,
    output_height: u32,
    output_stride: u32,
    shm: &ShmManager,
    transform: i32,
) {
    match buf {
        SurfaceBuffer::Shm {
            pool_fd,
            offset,
            width: buf_w,
            height: buf_h,
            stride: buf_stride,
            format,
        } => {
            if let Some((offset, len)) = buffer_layout(*offset, *buf_w, *buf_h, *buf_stride) {
                let Some(result) = read_shm_buffer_with_fallback(shm, *pool_fd, offset, len) else {
                    return;
                };
                blit_pixels(
                    result.as_bytes(),
                    *format,
                    *buf_w as u32,
                    *buf_h as u32,
                    *buf_stride as u32,
                    origin_x,
                    origin_y,
                    draw_w,
                    draw_h,
                    sample,
                    output_width,
                    output_height,
                    output_stride,
                    pixels,
                    transform,
                );
            }
        }
        SurfaceBuffer::DmaBuf {
            width: buf_w,
            height: buf_h,
            format,
            plane_fds,
            offsets,
            strides,
            num_planes: _,
        } => {
            if plane_fds.is_empty() || plane_fds[0] < 0 || *buf_w <= 0 || *buf_h <= 0 {
                return;
            }
            let fd = plane_fds[0];
            let buf_stride = strides
                .first()
                .copied()
                .unwrap_or((*buf_w as u32).saturating_mul(4));
            if buf_stride < (*buf_w as u32).saturating_mul(4) {
                return;
            }
            let offset = offsets.first().copied().unwrap_or(0);

            let pool_size = match drm::fd_size(fd) {
                Ok(s) => s,
                Err(_) => return,
            };
            let mapping = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    pool_size,
                    libc::PROT_READ,
                    libc::MAP_SHARED,
                    fd,
                    0,
                )
            };
            if mapping == libc::MAP_FAILED {
                return;
            }

            let buf_data = unsafe { std::slice::from_raw_parts(mapping as *const u8, pool_size) };
            let off = offset as usize;
            let Some(len) = (buf_stride as usize).checked_mul(*buf_h as usize) else {
                unsafe { libc::munmap(mapping, pool_size) };
                return;
            };
            let Some(end) = off.checked_add(len) else {
                unsafe { libc::munmap(mapping, pool_size) };
                return;
            };
            if end > buf_data.len() {
                unsafe { libc::munmap(mapping, pool_size) };
                return;
            }
            let data = &buf_data[off..end];

            blit_pixels(
                data,
                *format,
                *buf_w as u32,
                *buf_h as u32,
                buf_stride,
                origin_x,
                origin_y,
                draw_w,
                draw_h,
                sample,
                output_width,
                output_height,
                output_stride,
                pixels,
                transform,
            );

            unsafe { libc::munmap(mapping, pool_size) };
        }
        SurfaceBuffer::Dumb { .. } => {}
        SurfaceBuffer::Null => {}
    }
}

/// Shared pixel blitting logic — handles format conversion and clipping.
/// Uses integer alpha blending for performance (no f32).
/// Applies buffer transform (rotation/flip) before compositing.
///
/// The format match is hoisted outside the pixel loop for performance —
/// we dispatch to a format-specific inner loop once per surface.
fn blit_pixels(
    src_data: &[u8],
    format: u32,
    buf_w: u32,
    buf_h: u32,
    buf_stride: u32,
    origin_x: i32,
    origin_y: i32,
    draw_w: u32,
    draw_h: u32,
    sample: SurfaceSampleRect,
    output_width: u32,
    output_height: u32,
    output_stride: u32,
    pixels: &mut [u8],
    transform: i32,
) {
    // Transform values match Wayland wl_output.transform
    // 0=normal, 1=90deg CW, 2=180deg, 3=270deg
    // 4=flipped, 5=flipped+90, 6=flipped+180, 7=flipped+270
    if draw_w == 0 || draw_h == 0 || sample.width == 0 || sample.height == 0 {
        return;
    }

    // Hoist format match outside the per-pixel loop
    match format {
        0x34325258 /* XRGB8888 */ => {
            blit_xrgb8888(src_data, buf_w, buf_h, buf_stride, origin_x, origin_y,
                output_width, output_height, output_stride, pixels, transform, draw_w, draw_h, sample);
        }
        0x34325241 /* ARGB8888 */ => {
            blit_argb8888(src_data, buf_w, buf_h, buf_stride, origin_x, origin_y,
                output_width, output_height, output_stride, pixels, transform, draw_w, draw_h, sample);
        }
        0x34324241 /* ABGR8888 */ => {
            blit_abgr8888(src_data, buf_w, buf_h, buf_stride, origin_x, origin_y,
                output_width, output_height, output_stride, pixels, transform, draw_w, draw_h, sample);
        }
        _ => {
            // Default: treat as XRGB8888, direct copy
            blit_xrgb8888(src_data, buf_w, buf_h, buf_stride, origin_x, origin_y,
                output_width, output_height, output_stride, pixels, transform, draw_w, draw_h, sample);
        }
    }
}

fn transformed_draw_size(sample: SurfaceSampleRect, transform: i32) -> (u32, u32) {
    match transform {
        1 | 3 | 5 | 7 => (sample.height, sample.width),
        _ => (sample.width, sample.height),
    }
}

fn transform_sample_coord(
    tx: u32,
    ty: u32,
    sample: SurfaceSampleRect,
    transform: i32,
) -> (u32, u32) {
    match transform {
        0 => (sample.x + tx, sample.y + ty),
        1 => (sample.x + sample.width - 1 - ty, sample.y + tx),
        2 => (
            sample.x + sample.width - 1 - tx,
            sample.y + sample.height - 1 - ty,
        ),
        3 => (sample.x + ty, sample.y + sample.height - 1 - tx),
        4 => (sample.x + sample.width - 1 - tx, sample.y + ty),
        5 => (
            sample.x + sample.width - 1 - ty,
            sample.y + sample.height - 1 - tx,
        ),
        6 => (sample.x + tx, sample.y + sample.height - 1 - ty),
        7 => (sample.x + ty, sample.y + tx),
        _ => (sample.x + tx, sample.y + ty),
    }
}

fn scale_coord(dst: u32, dst_len: u32, src_len: u32) -> u32 {
    if dst_len == 0 || src_len == 0 {
        0
    } else {
        ((dst as u64 * src_len as u64) / dst_len as u64).min(src_len as u64 - 1) as u32
    }
}

/// Inner loop for XRGB8888 — same format as framebuffer, direct copy.
#[inline]
fn blit_xrgb8888(
    src_data: &[u8],
    _buf_w: u32,
    _buf_h: u32,
    buf_stride: u32,
    origin_x: i32,
    origin_y: i32,
    output_width: u32,
    output_height: u32,
    output_stride: u32,
    pixels: &mut [u8],
    transform: i32,
    draw_w: u32,
    draw_h: u32,
    sample: SurfaceSampleRect,
) {
    let (src_draw_w, src_draw_h) = transformed_draw_size(sample, transform);
    for dy in 0..draw_h {
        for dx_local in 0..draw_w {
            let tx = scale_coord(dx_local, draw_w, src_draw_w);
            let ty = scale_coord(dy, draw_h, src_draw_h);
            let (sx, sy) = transform_sample_coord(tx, ty, sample, transform);

            let out_x = origin_x + dx_local as i32;
            let out_y = origin_y + dy as i32;
            if out_x < 0
                || out_y < 0
                || out_x as u32 >= output_width
                || out_y as u32 >= output_height
            {
                continue;
            }
            let src_off = sy as usize * buf_stride as usize + sx as usize * 4;
            if src_off + 4 > src_data.len() {
                continue;
            }
            let dst_off = (out_y as u32 * output_stride + out_x as u32 * 4) as usize;
            if dst_off + 4 > pixels.len() {
                continue;
            }

            pixels[dst_off] = src_data[src_off];
            pixels[dst_off + 1] = src_data[src_off + 1];
            pixels[dst_off + 2] = src_data[src_off + 2];
            pixels[dst_off + 3] = 0xff;
        }
    }
}

/// Inner loop for ARGB8888 — needs alpha blending.
#[inline]
fn blit_argb8888(
    src_data: &[u8],
    _buf_w: u32,
    _buf_h: u32,
    buf_stride: u32,
    origin_x: i32,
    origin_y: i32,
    output_width: u32,
    output_height: u32,
    output_stride: u32,
    pixels: &mut [u8],
    transform: i32,
    draw_w: u32,
    draw_h: u32,
    sample: SurfaceSampleRect,
) {
    let (src_draw_w, src_draw_h) = transformed_draw_size(sample, transform);
    for dy in 0..draw_h {
        for dx_local in 0..draw_w {
            let tx = scale_coord(dx_local, draw_w, src_draw_w);
            let ty = scale_coord(dy, draw_h, src_draw_h);
            let (sx, sy) = transform_sample_coord(tx, ty, sample, transform);

            let out_x = origin_x + dx_local as i32;
            let out_y = origin_y + dy as i32;
            if out_x < 0
                || out_y < 0
                || out_x as u32 >= output_width
                || out_y as u32 >= output_height
            {
                continue;
            }
            let src_off = sy as usize * buf_stride as usize + sx as usize * 4;
            if src_off + 4 > src_data.len() {
                continue;
            }
            let dst_off = (out_y as u32 * output_stride + out_x as u32 * 4) as usize;
            if dst_off + 4 > pixels.len() {
                continue;
            }

            let alpha = src_data[src_off + 3];
            if alpha == 0 {
                continue;
            }
            if alpha == 255 {
                pixels[dst_off] = src_data[src_off];
                pixels[dst_off + 1] = src_data[src_off + 1];
                pixels[dst_off + 2] = src_data[src_off + 2];
            } else {
                let a_inv = 255u32 - alpha as u32;
                pixels[dst_off] = ((src_data[src_off] as u32 * alpha as u32
                    + pixels[dst_off] as u32 * a_inv)
                    / 255) as u8;
                pixels[dst_off + 1] = ((src_data[src_off + 1] as u32 * alpha as u32
                    + pixels[dst_off + 1] as u32 * a_inv)
                    / 255) as u8;
                pixels[dst_off + 2] = ((src_data[src_off + 2] as u32 * alpha as u32
                    + pixels[dst_off + 2] as u32 * a_inv)
                    / 255) as u8;
            }
            pixels[dst_off + 3] = 0xff;
        }
    }
}

/// Inner loop for ABGR8888 — needs R↔B swap + alpha blending.
#[inline]
fn blit_abgr8888(
    src_data: &[u8],
    _buf_w: u32,
    _buf_h: u32,
    buf_stride: u32,
    origin_x: i32,
    origin_y: i32,
    output_width: u32,
    output_height: u32,
    output_stride: u32,
    pixels: &mut [u8],
    transform: i32,
    draw_w: u32,
    draw_h: u32,
    sample: SurfaceSampleRect,
) {
    let (src_draw_w, src_draw_h) = transformed_draw_size(sample, transform);
    for dy in 0..draw_h {
        for dx_local in 0..draw_w {
            let tx = scale_coord(dx_local, draw_w, src_draw_w);
            let ty = scale_coord(dy, draw_h, src_draw_h);
            let (sx, sy) = transform_sample_coord(tx, ty, sample, transform);

            let out_x = origin_x + dx_local as i32;
            let out_y = origin_y + dy as i32;
            if out_x < 0
                || out_y < 0
                || out_x as u32 >= output_width
                || out_y as u32 >= output_height
            {
                continue;
            }
            let src_off = sy as usize * buf_stride as usize + sx as usize * 4;
            if src_off + 4 > src_data.len() {
                continue;
            }
            let dst_off = (out_y as u32 * output_stride + out_x as u32 * 4) as usize;
            if dst_off + 4 > pixels.len() {
                continue;
            }

            let alpha = src_data[src_off + 3];
            if alpha == 0 {
                continue;
            }
            if alpha == 255 {
                pixels[dst_off] = src_data[src_off + 2]; // B <- source B (byte 2)
                pixels[dst_off + 1] = src_data[src_off + 1]; // G <- source G (byte 1)
                pixels[dst_off + 2] = src_data[src_off]; // R <- source R (byte 0)
            } else {
                let a_inv = 255u32 - alpha as u32;
                let src_b = src_data[src_off + 2] as u32;
                let src_g = src_data[src_off + 1] as u32;
                let src_r = src_data[src_off] as u32;
                pixels[dst_off] =
                    ((src_b * alpha as u32 + pixels[dst_off] as u32 * a_inv) / 255) as u8;
                pixels[dst_off + 1] =
                    ((src_g * alpha as u32 + pixels[dst_off + 1] as u32 * a_inv) / 255) as u8;
                pixels[dst_off + 2] =
                    ((src_r * alpha as u32 + pixels[dst_off + 2] as u32 * a_inv) / 255) as u8;
            }
            pixels[dst_off + 3] = 0xff;
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod blit_tests {
    use super::*;

    fn sample(width: u32, height: u32) -> SurfaceSampleRect {
        SurfaceSampleRect {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    #[test]
    fn test_clear_xrgb8888_respects_pitch() {
        let mut pixels = vec![0u8; 24];
        clear_xrgb8888(&mut pixels, 2, 2, 12, [0x2e, 0x1a, 0x1a, 0xff]);

        assert_eq!(
            &pixels[0..12],
            &[0x2e, 0x1a, 0x1a, 0xff, 0x2e, 0x1a, 0x1a, 0xff, 0, 0, 0, 0]
        );
        assert_eq!(
            &pixels[12..24],
            &[0x2e, 0x1a, 0x1a, 0xff, 0x2e, 0x1a, 0x1a, 0xff, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_damage_accumulator_take_rects_clears_dirty_rects() {
        let mut damage = DamageAccumulator::new();
        damage.damage_all = false;
        damage.add(DamageRect {
            x: 1,
            y: 2,
            width: 3,
            height: 4,
        });

        assert!(damage.is_dirty());
        let rects = damage.take_rects();
        assert_eq!(rects.len(), 1);
        assert!(!damage.is_dirty());
    }

    #[test]
    fn test_frontmost_visible_toplevel_controls_tearing() {
        let mut shell = Shell::new(100);
        shell.create_toplevel(1, 10);
        shell.create_toplevel(2, 20);

        let mut surfaces = SurfaceTree::new();
        surfaces.create(10);
        surfaces.create(20);
        surfaces.attach(
            10,
            SurfaceBuffer::Shm {
                pool_fd: 0,
                offset: 0,
                width: 1,
                height: 1,
                stride: 4,
                format: 0x34325258,
            },
            0,
            0,
        );
        surfaces.attach(
            20,
            SurfaceBuffer::Shm {
                pool_fd: 0,
                offset: 0,
                width: 1,
                height: 1,
                stride: 4,
                format: 0x34325258,
            },
            0,
            0,
        );
        surfaces.commit(10);
        surfaces.commit(20);

        surfaces.set_tearing_hint(10, 2);
        surfaces.set_tearing_hint(20, 1);
        assert!(!frontmost_visible_toplevel_allows_tearing(
            &shell, &surfaces
        ));

        surfaces.set_tearing_hint(20, 2);
        assert!(frontmost_visible_toplevel_allows_tearing(&shell, &surfaces));
    }

    #[test]
    fn test_detached_frontmost_toplevel_does_not_mask_tearing_fallback() {
        let mut shell = Shell::new(100);
        shell.create_toplevel(1, 10);
        shell.create_toplevel(2, 20);

        let mut surfaces = SurfaceTree::new();
        surfaces.create(10);
        surfaces.create(20);
        surfaces.attach(
            10,
            SurfaceBuffer::Shm {
                pool_fd: 0,
                offset: 0,
                width: 1,
                height: 1,
                stride: 4,
                format: 0x34325258,
            },
            0,
            0,
        );
        surfaces.commit(10);
        surfaces.set_tearing_hint(10, 2);

        assert!(frontmost_visible_toplevel_allows_tearing(&shell, &surfaces));
    }

    #[test]
    fn test_blit_rejects_negative_shm_geometry() {
        let shm = ShmManager::new();
        let mut pixels = vec![0x55u8; 16];
        let buf = SurfaceBuffer::Shm {
            pool_fd: 0,
            offset: 0,
            width: -1,
            height: 1,
            stride: 4,
            format: 0x34325258,
        };

        blit_surface_buffer(
            &buf,
            &mut pixels,
            0,
            0,
            0,
            0,
            sample(0, 0),
            2,
            2,
            8,
            &shm,
            0,
        );
        assert_eq!(pixels, vec![0x55u8; 16]);
    }

    #[test]
    fn test_blit_xrgb_direct_copy() {
        let src = vec![
            0x00, 0x00, 0xff, 0xff, 0x00, 0xff, 0x00, 0xff, 0xff, 0x00, 0x00, 0xff, 0xff, 0xff,
            0x00, 0xff,
        ];
        let mut dst = vec![0u8; 16];
        blit_xrgb8888(
            &src,
            2,
            2,
            8,
            0,
            0,
            2,
            2,
            8,
            &mut dst,
            0,
            2,
            2,
            sample(2, 2),
        );
        assert_eq!(&dst[0..4], &[0x00, 0x00, 0xff, 0xff]);
        assert_eq!(&dst[8..12], &[0xff, 0x00, 0x00, 0xff]);
    }

    #[test]
    fn test_blit_xrgb_scales_to_logical_size() {
        let src = vec![
            0x01, 0x00, 0x00, 0xff, 0x02, 0x00, 0x00, 0xff, 0x03, 0x00, 0x00, 0xff, 0x04, 0x00,
            0x00, 0xff,
        ];
        let mut dst = vec![0u8; 16];

        blit_xrgb8888(
            &src,
            4,
            1,
            16,
            0,
            0,
            4,
            1,
            16,
            &mut dst,
            0,
            2,
            1,
            sample(4, 1),
        );

        assert_eq!(dst[0], 0x01);
        assert_eq!(dst[4], 0x03);
        assert_eq!(dst[8], 0x00);
        assert_eq!(dst[12], 0x00);
    }

    #[test]
    fn test_blit_argb_alpha_blend() {
        let src = vec![0x00, 0x00, 0xff, 0x80];
        let mut dst = vec![0x00, 0x00, 0x00, 0xff];
        blit_argb8888(
            &src,
            1,
            1,
            4,
            0,
            0,
            1,
            1,
            4,
            &mut dst,
            0,
            1,
            1,
            sample(1, 1),
        );
        assert_eq!(dst[2], 128);
        assert_eq!(dst[3], 0xff);
    }

    #[test]
    fn test_blit_argb_opaque_skip() {
        let src = vec![0x00, 0x00, 0xff, 0x00];
        let mut dst = vec![0x00, 0x00, 0x00, 0xff];
        blit_argb8888(
            &src,
            1,
            1,
            4,
            0,
            0,
            1,
            1,
            4,
            &mut dst,
            0,
            1,
            1,
            sample(1, 1),
        );
        assert_eq!(&dst[..], &[0x00, 0x00, 0x00, 0xff]);
    }

    #[test]
    fn test_blit_abgr_r_b_swap() {
        let src = vec![0xff, 0x80, 0x40, 0xff];
        let mut dst = vec![0u8; 4];
        blit_abgr8888(
            &src,
            1,
            1,
            4,
            0,
            0,
            1,
            1,
            4,
            &mut dst,
            0,
            1,
            1,
            sample(1, 1),
        );
        assert_eq!(dst[0], 0x40);
        assert_eq!(dst[1], 0x80);
        assert_eq!(dst[2], 0xff);
        assert_eq!(dst[3], 0xff);
    }

    #[test]
    fn test_blit_transform_180() {
        let src = vec![
            0x01, 0x00, 0x00, 0xff, 0x02, 0x00, 0x00, 0xff, 0x03, 0x00, 0x00, 0xff, 0x04, 0x00,
            0x00, 0xff,
        ];
        let mut dst = vec![0u8; 16];
        blit_xrgb8888(
            &src,
            2,
            2,
            8,
            0,
            0,
            2,
            2,
            8,
            &mut dst,
            2,
            2,
            2,
            sample(2, 2),
        );
        assert_eq!(dst[0], 0x04);
        assert_eq!(dst[4], 0x03);
    }

    #[test]
    fn test_blit_xrgb_samples_viewport_source() {
        let src = vec![
            0x01, 0x00, 0x00, 0xff, 0x02, 0x00, 0x00, 0xff, 0x03, 0x00, 0x00, 0xff, 0x04, 0x00,
            0x00, 0xff,
        ];
        let mut dst = vec![0u8; 8];

        blit_xrgb8888(
            &src,
            4,
            1,
            16,
            0,
            0,
            2,
            1,
            8,
            &mut dst,
            0,
            2,
            1,
            SurfaceSampleRect {
                x: 1,
                y: 0,
                width: 2,
                height: 1,
            },
        );

        assert_eq!(dst[0], 0x02);
        assert_eq!(dst[4], 0x03);
    }
}
