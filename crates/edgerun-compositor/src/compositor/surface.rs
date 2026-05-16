//! Surface management — the core of Wayland compositing.

use std::collections::HashMap;

/// A Wayland surface.
#[derive(Debug)]
pub struct Surface {
    /// The wl_surface object id.
    pub id: u32,
    /// Currently committed buffer (None = no buffer).
    pub buffer: Option<SurfaceBuffer>,
    /// Pending buffer (from attach, applied on commit).
    pub pending_buffer: Option<SurfaceBuffer>,
    /// Pending damage region.
    pub pending_damage: Vec<DamageRect>,
    pub pending_x: Option<i32>,
    pub pending_y: Option<i32>,
    pub pending_buffer_scale: Option<i32>,
    pub pending_buffer_transform: Option<i32>,
    pub pending_opaque: Option<bool>,
    pub pending_opaque_region: Option<Vec<DamageRect>>,
    pub pending_input_region: Option<Option<Vec<DamageRect>>>,
    pub pending_viewport_src: Option<Option<(f64, f64, f64, f64)>>,
    pub pending_viewport_dst: Option<Option<(i32, i32)>>,
    /// Surface position relative to output.
    pub x: i32,
    pub y: i32,
    /// Buffer scale factor.
    pub buffer_scale: i32,
    /// Buffer transform.
    pub buffer_transform: i32,
    /// Whether this surface is opaque.
    pub opaque: bool,
    /// Opaque region rectangles (in surface coordinates).
    pub opaque_region: Vec<DamageRect>,
    /// Input region rectangles (in surface coordinates). None = entire surface.
    pub input_region: Option<Vec<DamageRect>>,
    /// Frame callbacks waiting to be fired.
    pub frame_callbacks: Vec<u32>, // callback object ids
    /// Width of current buffer.
    pub width: u32,
    /// Height of current buffer.
    pub height: u32,
    /// Viewport source rectangle (x, y, w, h) in buffer coordinates.
    /// None = use entire buffer.
    pub viewport_src: Option<(f64, f64, f64, f64)>,
    /// Viewport destination size (w, h). (-1, -1) = use buffer size.
    pub viewport_dst: Option<(i32, i32)>,
    /// Tearing hint from wp_tearing_control_v1.
    /// 0 = default, 1 = sync (VSync), 2 = async (allow tearing)
    pub tearing_hint: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceSampleRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A buffer attached to a surface.
#[derive(Debug, Clone)]
pub enum SurfaceBuffer {
    /// SHM buffer.
    Shm {
        /// The SHM pool fd.
        pool_fd: i32,
        /// Offset within the pool.
        offset: i32,
        /// Buffer width.
        width: i32,
        /// Buffer height.
        height: i32,
        /// Row stride.
        stride: i32,
        /// Pixel format.
        format: u32,
    },
    /// Dumb DRM buffer.
    Dumb {
        /// DRM buffer handle.
        handle: u32,
        /// Framebuffer id.
        fb_id: u32,
        width: u32,
        height: u32,
        pitch: u32,
    },
    /// DMA-BUF buffer.
    DmaBuf {
        /// Width in pixels.
        width: i32,
        /// Height in pixels.
        height: i32,
        /// DRM format.
        format: u32,
        /// Number of planes.
        num_planes: u32,
        /// Plane fds (will be closed after mmap).
        plane_fds: Vec<i32>,
        /// Offsets for each plane.
        offsets: Vec<u32>,
        /// Strides for each plane.
        strides: Vec<u32>,
    },
    /// Null buffer (detach).
    Null,
}

impl SurfaceBuffer {
    pub fn width(&self) -> u32 {
        match self {
            Self::Shm { width, .. } => (*width).max(0) as u32,
            Self::Dumb { width, .. } => *width,
            Self::DmaBuf { width, .. } => (*width).max(0) as u32,
            Self::Null => 0,
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Self::Shm { height, .. } => (*height).max(0) as u32,
            Self::Dumb { height, .. } => *height,
            Self::DmaBuf { height, .. } => (*height).max(0) as u32,
            Self::Null => 0,
        }
    }

    pub fn stride(&self) -> u32 {
        match self {
            Self::Shm { stride, .. } => (*stride).max(0) as u32,
            Self::Dumb { pitch, .. } => *pitch,
            Self::DmaBuf { strides, .. } => strides.first().copied().unwrap_or(0),
            Self::Null => 0,
        }
    }

    pub fn format(&self) -> u32 {
        match self {
            Self::Shm { format, .. } => *format,
            Self::Dumb { .. } => 0x34325258, // XRGB8888
            Self::DmaBuf { format, .. } => *format,
            Self::Null => 0,
        }
    }

    pub fn has_valid_layout(&self) -> bool {
        match self {
            Self::Shm {
                offset,
                width,
                height,
                stride,
                ..
            } => buffer_layout(*offset, *width, *height, *stride).is_some(),
            Self::DmaBuf {
                width,
                height,
                strides,
                ..
            } => {
                let stride = strides
                    .first()
                    .copied()
                    .unwrap_or((*width).max(0) as u32)
                    .saturating_mul(4);
                *width > 0
                    && *height > 0
                    && stride >= (*width as u32).saturating_mul(4)
                    && (stride as usize).checked_mul(*height as usize).is_some()
            }
            Self::Dumb {
                width,
                height,
                pitch,
                ..
            } => {
                *width > 0
                    && *height > 0
                    && *pitch >= width.saturating_mul(4)
                    && (*pitch as usize).checked_mul(*height as usize).is_some()
            }
            Self::Null => false,
        }
    }
}

impl Surface {
    pub fn logical_width(&self) -> u32 {
        if let Some((width, _)) = self.viewport_dst {
            if width > 0 {
                return width as u32;
            }
        }
        let (width, _) = self.transformed_sample_size();
        width / self.buffer_scale.max(1) as u32
    }

    pub fn logical_height(&self) -> u32 {
        if let Some((_, height)) = self.viewport_dst {
            if height > 0 {
                return height as u32;
            }
        }
        let (_, height) = self.transformed_sample_size();
        height / self.buffer_scale.max(1) as u32
    }

    pub fn logical_size(&self) -> (u32, u32) {
        (self.logical_width(), self.logical_height())
    }

    pub fn sample_rect(&self) -> SurfaceSampleRect {
        let full = SurfaceSampleRect {
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
        };
        let Some((x, y, width, height)) = self.viewport_src else {
            return full;
        };
        if self.width == 0 || self.height == 0 || width <= 0.0 || height <= 0.0 {
            return full;
        }

        let x = x.max(0.0).floor() as u32;
        let y = y.max(0.0).floor() as u32;
        if x >= self.width || y >= self.height {
            return SurfaceSampleRect {
                x: self.width,
                y: self.height,
                width: 0,
                height: 0,
            };
        }

        let max_width = self.width - x;
        let max_height = self.height - y;
        SurfaceSampleRect {
            x,
            y,
            width: (width.ceil() as u32).min(max_width),
            height: (height.ceil() as u32).min(max_height),
        }
    }

    fn transformed_sample_size(&self) -> (u32, u32) {
        let sample = self.sample_rect();
        match self.buffer_transform {
            1 | 3 | 5 | 7 => (sample.height, sample.width),
            _ => (sample.width, sample.height),
        }
    }
}

pub(crate) fn buffer_layout(
    offset: i32,
    width: i32,
    height: i32,
    stride: i32,
) -> Option<(usize, usize)> {
    if offset < 0 || width <= 0 || height <= 0 || stride <= 0 {
        return None;
    }

    let row_bytes = (width as usize).checked_mul(4)?;
    let stride = stride as usize;
    if stride < row_bytes {
        return None;
    }

    let len = stride.checked_mul(height as usize)?;
    Some((offset as usize, len))
}

/// Global buffer registry — maps wl_buffer object id to SHM buffer info.
#[derive(Debug, Default)]
pub struct BufferRegistry {
    buffers: HashMap<u32, ShmBufferInfo>,
}

/// SHM buffer info for a wl_buffer object.
#[derive(Debug, Clone)]
pub struct ShmBufferInfo {
    /// The SHM pool fd.
    pub pool_fd: i32,
    /// Offset within the pool.
    pub offset: i32,
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub format: u32,
}

impl BufferRegistry {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    pub fn register(&mut self, buffer_id: u32, info: ShmBufferInfo) {
        self.buffers.insert(buffer_id, info);
    }

    pub fn get(&self, buffer_id: u32) -> Option<&ShmBufferInfo> {
        self.buffers.get(&buffer_id)
    }

    pub fn remove(&mut self, buffer_id: u32) {
        self.buffers.remove(&buffer_id);
    }
}

/// A damage rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub(crate) fn subtract_rect_from_region(region: &mut Vec<DamageRect>, subtract: DamageRect) {
    if subtract.width <= 0 || subtract.height <= 0 {
        return;
    }

    let mut result = Vec::with_capacity(region.len());
    for rect in region.drain(..) {
        if rect.width <= 0 || rect.height <= 0 {
            continue;
        }

        let rect_right = rect.x.saturating_add(rect.width);
        let rect_bottom = rect.y.saturating_add(rect.height);
        let sub_right = subtract.x.saturating_add(subtract.width);
        let sub_bottom = subtract.y.saturating_add(subtract.height);

        let ix0 = rect.x.max(subtract.x);
        let iy0 = rect.y.max(subtract.y);
        let ix1 = rect_right.min(sub_right);
        let iy1 = rect_bottom.min(sub_bottom);

        if ix0 >= ix1 || iy0 >= iy1 {
            result.push(rect);
            continue;
        }

        if rect.y < iy0 {
            result.push(DamageRect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: iy0 - rect.y,
            });
        }
        if iy1 < rect_bottom {
            result.push(DamageRect {
                x: rect.x,
                y: iy1,
                width: rect.width,
                height: rect_bottom - iy1,
            });
        }
        if rect.x < ix0 {
            result.push(DamageRect {
                x: rect.x,
                y: iy0,
                width: ix0 - rect.x,
                height: iy1 - iy0,
            });
        }
        if ix1 < rect_right {
            result.push(DamageRect {
                x: ix1,
                y: iy0,
                width: rect_right - ix1,
                height: iy1 - iy0,
            });
        }
    }

    *region = result;
}

/// Surface state manager.
pub struct SurfaceTree {
    surfaces: HashMap<u32, Surface>,
}

impl SurfaceTree {
    pub fn new() -> Self {
        Self {
            surfaces: HashMap::new(),
        }
    }

    /// Create a new surface.
    pub fn create(&mut self, id: u32) {
        self.surfaces.insert(
            id,
            Surface {
                id,
                buffer: None,
                pending_buffer: None,
                pending_damage: Vec::new(),
                pending_x: None,
                pending_y: None,
                pending_buffer_scale: None,
                pending_buffer_transform: None,
                pending_opaque: None,
                pending_opaque_region: None,
                pending_input_region: None,
                pending_viewport_src: None,
                pending_viewport_dst: None,
                x: 0,
                y: 0,
                buffer_scale: 1,
                buffer_transform: 0,
                opaque: false,
                opaque_region: Vec::new(),
                input_region: None,
                frame_callbacks: Vec::new(),
                width: 0,
                height: 0,
                viewport_src: None,
                viewport_dst: None,
                tearing_hint: 0,
            },
        );
    }

    /// Get a surface by id.
    pub fn get(&self, id: u32) -> Option<&Surface> {
        self.surfaces.get(&id)
    }

    /// Get a mutable surface.
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Surface> {
        self.surfaces.get_mut(&id)
    }

    /// Attach a buffer to a surface (pending until commit).
    pub fn attach(&mut self, id: u32, buffer: SurfaceBuffer, x: i32, y: i32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.pending_buffer = Some(buffer);
            s.pending_x = Some(x);
            s.pending_y = Some(y);
        }
    }

    /// Mark damage on a surface.
    pub fn damage(&mut self, id: u32, x: i32, y: i32, w: i32, h: i32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.pending_damage.push(DamageRect {
                x,
                y,
                width: w,
                height: h,
            });
        }
    }

    /// Add a frame callback.
    pub fn add_frame_callback(&mut self, id: u32, callback_id: u32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.frame_callbacks.push(callback_id);
        }
    }

    /// Set buffer scale.
    pub fn set_buffer_scale(&mut self, id: u32, scale: i32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.pending_buffer_scale = Some(scale.max(1));
        }
    }

    /// Set buffer transform (rotation/flip).
    pub fn set_buffer_transform(&mut self, id: u32, transform: i32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.pending_buffer_transform = Some(transform);
        }
    }

    /// Set tearing control hint.
    pub fn set_tearing_hint(&mut self, id: u32, hint: u32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.tearing_hint = hint;
        }
    }

    /// Set viewport source rectangle.
    pub fn set_viewport_source(&mut self, id: u32, x: f64, y: f64, w: f64, h: f64) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.pending_viewport_src = Some(if w > 0.0 && h > 0.0 {
                Some((x, y, w, h))
            } else {
                None
            });
        }
    }

    /// Set viewport destination size.
    pub fn set_viewport_destination(&mut self, id: u32, w: i32, h: i32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.pending_viewport_dst = Some(if w > 0 && h > 0 { Some((w, h)) } else { None });
        }
    }

    /// Set the opaque region for a surface.
    /// Replaces the entire opaque region with the given rectangles.
    pub fn set_opaque_region(&mut self, id: u32, rects: Vec<DamageRect>) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.pending_opaque = Some(!rects.is_empty());
            s.pending_opaque_region = Some(rects);
        }
    }

    /// Add a rectangle to the opaque region.
    pub fn add_opaque_region_rect(&mut self, id: u32, rect: DamageRect) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            let region = s
                .pending_opaque_region
                .get_or_insert_with(|| s.opaque_region.clone());
            if rect.width > 0 && rect.height > 0 {
                region.push(rect);
                s.pending_opaque = Some(true);
            }
        }
    }

    /// Subtract a rectangle from the opaque region.
    pub fn subtract_opaque_region_rect(&mut self, id: u32, rect: DamageRect) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            let region = s
                .pending_opaque_region
                .get_or_insert_with(|| s.opaque_region.clone());
            subtract_rect_from_region(region, rect);
            s.pending_opaque = Some(!region.is_empty());
        }
    }

    /// Set the input region for a surface.
    /// None = entire surface accepts input.
    pub fn set_input_region(&mut self, id: u32, rects: Option<Vec<DamageRect>>) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.pending_input_region = Some(rects);
        }
    }

    /// Add a rectangle to the input region.
    pub fn add_input_region_rect(&mut self, id: u32, rect: DamageRect) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            let region = s
                .pending_input_region
                .get_or_insert_with(|| s.input_region.clone())
                .get_or_insert_with(Vec::new);
            if rect.width > 0 && rect.height > 0 {
                region.push(rect);
            }
        }
    }

    /// Subtract a rectangle from the input region.
    pub fn subtract_input_region_rect(&mut self, id: u32, rect: DamageRect) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            let pending_region = s
                .pending_input_region
                .get_or_insert_with(|| s.input_region.clone());
            if let Some(ref mut region) = pending_region {
                subtract_rect_from_region(region, rect);
            }
        }
    }

    pub fn offset(&mut self, id: u32, x: i32, y: i32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.pending_x = Some(x);
            s.pending_y = Some(y);
        }
    }

    /// Commit pending state for a surface.
    pub fn commit(&mut self, id: u32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            if let Some(x) = s.pending_x.take() {
                s.x = x;
            }
            if let Some(y) = s.pending_y.take() {
                s.y = y;
            }
            if let Some(scale) = s.pending_buffer_scale.take() {
                s.buffer_scale = scale;
            }
            if let Some(transform) = s.pending_buffer_transform.take() {
                s.buffer_transform = transform;
            }
            if let Some(opaque) = s.pending_opaque.take() {
                s.opaque = opaque;
            }
            if let Some(region) = s.pending_opaque_region.take() {
                s.opaque_region = region;
            }
            if let Some(region) = s.pending_input_region.take() {
                s.input_region = region;
            }
            if let Some(viewport_src) = s.pending_viewport_src.take() {
                s.viewport_src = viewport_src;
            }
            if let Some(viewport_dst) = s.pending_viewport_dst.take() {
                s.viewport_dst = viewport_dst;
            }

            // Apply pending buffer
            if let Some(buf) = s.pending_buffer.take() {
                if matches!(buf, SurfaceBuffer::Null) {
                    s.width = 0;
                    s.height = 0;
                    s.buffer = None;
                } else if buf.has_valid_layout() {
                    s.width = buf.width();
                    s.height = buf.height();
                    s.buffer = Some(buf);
                } else {
                    s.width = 0;
                    s.height = 0;
                    s.buffer = None;
                }
            }
            // Damage is cleared on commit
            s.pending_damage.clear();
        }
    }

    /// Destroy a surface.
    pub fn destroy(&mut self, id: u32) -> Option<Surface> {
        self.surfaces.remove(&id)
    }

    /// Iterate over all surfaces.
    pub fn surfaces(&self) -> impl Iterator<Item = &Surface> {
        self.surfaces.values()
    }

    /// Iterate mutably.
    pub fn surfaces_mut(&mut self) -> impl Iterator<Item = &mut Surface> {
        self.surfaces.values_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_surface() {
        let mut tree = SurfaceTree::new();
        tree.create(5);
        let surface = tree.get(5).unwrap();
        assert_eq!(surface.id, 5);
        assert!(surface.buffer.is_none());
        assert_eq!(surface.x, 0);
        assert_eq!(surface.y, 0);
    }

    #[test]
    fn test_surface_attach_commit() {
        let mut tree = SurfaceTree::new();
        tree.create(5);
        tree.attach(
            5,
            SurfaceBuffer::Shm {
                pool_fd: 0,
                offset: 0,
                width: 800,
                height: 600,
                stride: 3200,
                format: 0x34325258,
            },
            10,
            20,
        );
        assert!(tree.get(5).unwrap().pending_buffer.is_some());
        tree.commit(5);
        assert!(tree.get(5).unwrap().buffer.is_some());
        assert_eq!(tree.get(5).unwrap().width, 800);
        assert_eq!(tree.get(5).unwrap().height, 600);
        assert_eq!(tree.get(5).unwrap().x, 10);
        assert_eq!(tree.get(5).unwrap().y, 20);
    }

    #[test]
    fn test_surface_null_attach_detaches_on_commit() {
        let mut tree = SurfaceTree::new();
        tree.create(5);
        tree.attach(
            5,
            SurfaceBuffer::Shm {
                pool_fd: 0,
                offset: 0,
                width: 800,
                height: 600,
                stride: 3200,
                format: 0x34325258,
            },
            0,
            0,
        );
        tree.commit(5);
        assert!(tree.get(5).unwrap().buffer.is_some());

        tree.attach(5, SurfaceBuffer::Null, 10, 20);
        tree.commit(5);
        let surface = tree.get(5).unwrap();
        assert!(surface.buffer.is_none());
        assert_eq!(surface.width, 0);
        assert_eq!(surface.height, 0);
        assert_eq!(surface.x, 10);
        assert_eq!(surface.y, 20);
    }

    #[test]
    fn test_invalid_shm_buffer_does_not_commit_live_surface() {
        let mut tree = SurfaceTree::new();
        tree.create(5);
        tree.attach(
            5,
            SurfaceBuffer::Shm {
                pool_fd: 0,
                offset: 0,
                width: -1,
                height: 600,
                stride: 3200,
                format: 0x34325258,
            },
            0,
            0,
        );
        tree.commit(5);

        let surface = tree.get(5).unwrap();
        assert!(surface.buffer.is_none());
        assert_eq!(surface.width, 0);
        assert_eq!(surface.height, 0);
    }

    #[test]
    fn test_invalid_shm_stride_is_rejected() {
        let buf = SurfaceBuffer::Shm {
            pool_fd: 0,
            offset: 0,
            width: 10,
            height: 10,
            stride: 4,
            format: 0x34325258,
        };
        assert!(!buf.has_valid_layout());
        assert_eq!(buffer_layout(0, 10, 10, 4), None);
    }

    #[test]
    fn test_surface_damage_tracking() {
        let mut tree = SurfaceTree::new();
        tree.create(5);
        tree.damage(5, 0, 0, 100, 100);
        assert_eq!(tree.get(5).unwrap().pending_damage.len(), 1);
        tree.commit(5); // clears damage on commit
        assert_eq!(tree.get(5).unwrap().pending_damage.len(), 0);
    }

    #[test]
    fn test_region_subtract_splits_rect_around_hole() {
        let mut region = vec![DamageRect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        }];

        subtract_rect_from_region(
            &mut region,
            DamageRect {
                x: 20,
                y: 30,
                width: 40,
                height: 50,
            },
        );

        assert_eq!(region.len(), 4);
        assert!(region.contains(&DamageRect {
            x: 0,
            y: 0,
            width: 100,
            height: 30,
        }));
        assert!(region.contains(&DamageRect {
            x: 0,
            y: 80,
            width: 100,
            height: 20,
        }));
        assert!(region.contains(&DamageRect {
            x: 0,
            y: 30,
            width: 20,
            height: 50,
        }));
        assert!(region.contains(&DamageRect {
            x: 60,
            y: 30,
            width: 40,
            height: 50,
        }));
    }

    #[test]
    fn test_surface_scale() {
        let mut tree = SurfaceTree::new();
        tree.create(5);
        assert_eq!(tree.get(5).unwrap().buffer_scale, 1);
        tree.set_buffer_scale(5, 2);
        assert_eq!(tree.get(5).unwrap().buffer_scale, 1);
        tree.commit(5);
        assert_eq!(tree.get(5).unwrap().buffer_scale, 2);
        tree.set_buffer_scale(5, 0); // should clamp to 1
        assert_eq!(tree.get(5).unwrap().buffer_scale, 2);
        tree.commit(5);
        assert_eq!(tree.get(5).unwrap().buffer_scale, 1);
    }

    #[test]
    fn test_surface_logical_size_uses_scale_and_viewport_destination() {
        let mut tree = SurfaceTree::new();
        tree.create(5);
        tree.attach(
            5,
            SurfaceBuffer::Shm {
                pool_fd: 0,
                offset: 0,
                width: 200,
                height: 100,
                stride: 800,
                format: 0x34325258,
            },
            0,
            0,
        );
        tree.set_buffer_scale(5, 2);
        tree.commit(5);

        let surface = tree.get(5).unwrap();
        assert_eq!(surface.logical_width(), 100);
        assert_eq!(surface.logical_height(), 50);

        tree.set_viewport_destination(5, 80, 40);
        let surface = tree.get(5).unwrap();
        assert_eq!(surface.logical_width(), 100);
        assert_eq!(surface.logical_height(), 50);
        tree.commit(5);
        let surface = tree.get(5).unwrap();
        assert_eq!(surface.logical_width(), 80);
        assert_eq!(surface.logical_height(), 40);

        tree.set_viewport_destination(5, -1, -1);
        tree.commit(5);
        let surface = tree.get(5).unwrap();
        assert_eq!(surface.logical_width(), 100);
        assert_eq!(surface.logical_height(), 50);

        tree.set_viewport_source(5, 20.0, 10.0, 60.0, 20.0);
        tree.commit(5);
        let surface = tree.get(5).unwrap();
        assert_eq!(
            surface.sample_rect(),
            SurfaceSampleRect {
                x: 20,
                y: 10,
                width: 60,
                height: 20,
            }
        );
        assert_eq!(surface.logical_width(), 30);
        assert_eq!(surface.logical_height(), 10);
    }

    #[test]
    fn test_surface_destroy() {
        let mut tree = SurfaceTree::new();
        tree.create(5);
        assert!(tree.get(5).is_some());
        let surface = tree.destroy(5);
        assert!(surface.is_some());
        assert!(tree.get(5).is_none());
    }

    #[test]
    fn test_frame_callbacks() {
        let mut tree = SurfaceTree::new();
        tree.create(5);
        tree.add_frame_callback(5, 100);
        tree.add_frame_callback(5, 101);
        assert_eq!(tree.get(5).unwrap().frame_callbacks.len(), 2);
        tree.commit(5); // commit doesn't clear callbacks
        assert_eq!(tree.get(5).unwrap().frame_callbacks.len(), 2);
    }

    #[test]
    fn test_surface_buffer_dimensions() {
        let buf = SurfaceBuffer::Shm {
            pool_fd: 0,
            offset: 0,
            width: 800,
            height: 600,
            stride: 3200,
            format: 0x34325258,
        };
        assert_eq!(buf.width(), 800);
        assert_eq!(buf.height(), 600);
        assert_eq!(buf.stride(), 3200);
        assert_eq!(buf.format(), 0x34325258);
    }
}
