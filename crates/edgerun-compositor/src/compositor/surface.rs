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
    /// Surface position relative to output.
    pub x: i32,
    pub y: i32,
    /// Buffer scale factor.
    pub buffer_scale: i32,
    /// Buffer transform.
    pub buffer_transform: i32,
    /// Whether this surface is opaque.
    pub opaque: bool,
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
            Self::Shm { width, .. } => *width as u32,
            Self::Dumb { width, .. } => *width,
            Self::DmaBuf { width, .. } => *width as u32,
            Self::Null => 0,
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Self::Shm { height, .. } => *height as u32,
            Self::Dumb { height, .. } => *height,
            Self::DmaBuf { height, .. } => *height as u32,
            Self::Null => 0,
        }
    }

    pub fn stride(&self) -> u32 {
        match self {
            Self::Shm { stride, .. } => *stride as u32,
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
#[derive(Debug, Clone, Copy)]
pub struct DamageRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
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
        self.surfaces.insert(id, Surface {
            id,
            buffer: None,
            pending_buffer: None,
            pending_damage: Vec::new(),
            x: 0,
            y: 0,
            buffer_scale: 1,
            buffer_transform: 0,
            opaque: false,
            frame_callbacks: Vec::new(),
            width: 0,
            height: 0,
            viewport_src: None,
            viewport_dst: None,
            tearing_hint: 0,
        });
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
            s.x = x;
            s.y = y;
        }
    }

    /// Mark damage on a surface.
    pub fn damage(&mut self, id: u32, x: i32, y: i32, w: i32, h: i32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.pending_damage.push(DamageRect { x, y, width: w, height: h });
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
            s.buffer_scale = scale.max(1);
        }
    }

    /// Set buffer transform (rotation/flip).
    pub fn set_buffer_transform(&mut self, id: u32, transform: i32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.buffer_transform = transform;
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
            s.viewport_src = Some((x, y, w, h));
        }
    }

    /// Set viewport destination size.
    pub fn set_viewport_destination(&mut self, id: u32, w: i32, h: i32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            s.viewport_dst = Some((w, h));
        }
    }

    /// Commit pending state for a surface.
    pub fn commit(&mut self, id: u32) {
        if let Some(s) = self.surfaces.get_mut(&id) {
            // Apply pending buffer
            if let Some(buf) = s.pending_buffer.take() {
                s.width = buf.width();
                s.height = buf.height();
                s.buffer = Some(buf);
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
        tree.attach(5, SurfaceBuffer::Null, 10, 20);
        assert!(tree.get(5).unwrap().pending_buffer.is_some());
        tree.commit(5);
        assert!(tree.get(5).unwrap().buffer.is_some());
        assert_eq!(tree.get(5).unwrap().x, 10);
        assert_eq!(tree.get(5).unwrap().y, 20);
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
    fn test_surface_scale() {
        let mut tree = SurfaceTree::new();
        tree.create(5);
        assert_eq!(tree.get(5).unwrap().buffer_scale, 1);
        tree.set_buffer_scale(5, 2);
        assert_eq!(tree.get(5).unwrap().buffer_scale, 2);
        tree.set_buffer_scale(5, 0); // should clamp to 1
        assert_eq!(tree.get(5).unwrap().buffer_scale, 1);
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
