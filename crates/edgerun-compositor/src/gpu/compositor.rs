//! GL-based compositor — composites surfaces using textured quads via EGL/OpenGL.
//!
//! Architecture:
//! 1. EGL display created from DRM device fd
//! 2. EGL context (OpenGL or OpenGL ES)
//! 3. Offscreen FBO with texture attachment
//! 4. Each surface (SHM or DMA-BUF) → GL texture
//! 5. Render textured quads back-to-front
//! 6. glReadPixels back to DRM dumb buffer for scanout

use std::os::raw::c_int;

use crate::compositor::surface::{SurfaceBuffer, SurfaceTree};
use crate::compositor::shell::Shell;
use crate::drm;
use crate::drm::dumb::DumbBuffer;
use crate::gpu::egl::{self, Egl, EGLImage};
use crate::gpu::gl::{self, Gl};
use crate::render::cursor::Cursor;

/// A GL texture handle for a surface buffer.
struct GlTexture {
    id: u32,
    width: u32,
    height: u32,
    /// If created from DMA-BUF, the EGLImage that backs this texture.
    egl_image: EGLImage,
}

impl Drop for GlTexture {
    fn drop(&mut self) {
        // egl_image is owned by the compositor's texture cache, not dropped here
    }
}

/// GL compositor state.
pub struct GlCompositor {
    egl: Egl,
    gl: Gl,
    egl_display: egl::EGLDisplay,
    egl_context: egl::EGLContext,
    /// FBO for offscreen rendering
    fbo: u32,
    /// Color attachment texture for FBO
    fbo_texture: u32,
    /// Depth/stencil renderbuffer
    fbo_depth: u32,
    /// Shader program for textured quad rendering
    shader_program: u32,
    /// VBO for fullscreen quad vertices
    quad_vbo: u32,
    /// VAO for quad rendering
    quad_vao: u32,
    /// Width of the output
    width: u32,
    /// Height of the output
    height: u32,
    /// Texture cache — maps (surface_id, buffer_key) → GlTexture
    texture_cache: std::collections::HashMap<u64, GlTexture>,
    next_cache_key: u64,
}

/// Generate a unique cache key for a surface buffer.
fn buffer_cache_key(surface_id: u32, buf: &SurfaceBuffer) -> u64 {
    let mut key = surface_id as u64;
    match buf {
        SurfaceBuffer::Shm { offset, width, height, stride, format } => {
            key ^= (*offset as u64) << 16;
            key ^= (*width as u64) << 32;
            key ^= (*height as u64) << 48;
            // stride and format affect the buffer identity
            let _ = (stride, format);
        }
        SurfaceBuffer::Dumb { handle, fb_id, width, height, pitch: _ } => {
            key ^= (*handle as u64) << 16;
            key ^= (*fb_id as u64) << 32;
            let _ = (width, height);
        }
        SurfaceBuffer::DmaBuf { width, height, format, num_planes, plane_fds, offsets: _, strides: _ } => {
            key ^= (*width as u64) << 16;
            key ^= (*height as u64) << 32;
            key ^= (*format as u64) << 48;
            if let Some(&fd) = plane_fds.first() {
                key ^= (fd as u64) << 4;
            }
            let _ = num_planes;
        }
        SurfaceBuffer::Null => {}
    }
    key
}

impl GlCompositor {
    pub fn new(drm_fd: c_int, width: u32, height: u32) -> Option<Self> {
        let egl = Egl::open()?;
        let gl = Gl::open()?;

        // Try EGL_EXT_platform_device first (no GBM dependency)
        let egl_display = unsafe {
            let device_attribs = [egl::EGL_NONE];
            (egl.eglGetPlatformDisplay)(
                egl::EGL_PLATFORM_DEVICE_EXT,
                std::ptr::null_mut(),
                device_attribs.as_ptr(),
            )
        };

        let egl_display = if egl_display == egl::EGL_NO_DISPLAY {
            eprintln!("[gl-compositor] EGL_PLATFORM_DEVICE_EXT failed, trying eglGetDisplay(DEFAULT)");
            // Fall back to EGL_DEFAULT_DISPLAY (0)
            unsafe { (egl.eglGetDisplay)(std::ptr::null_mut()) }
        } else {
            egl_display
        };

        if egl_display == egl::EGL_NO_DISPLAY {
            eprintln!("[gl-compositor] All EGL display methods failed (error: {:x})",
                unsafe { egl.eglGetError() });
            return None;
        }

        // Initialize EGL
        let mut major = 0;
        let mut minor = 0;
        let init_result = unsafe { (egl.eglInitialize)(egl_display, &mut major, &mut minor) };
        if init_result == egl::EGL_FALSE {
            eprintln!("[gl-compositor] eglInitialize failed (error: {:x})", unsafe { egl.eglGetError() });
            return None;
        }
        eprintln!("[gl-compositor] EGL {}.{} initialized", major, minor);

        // Print extensions
        if let Some(extensions) = egl.query_string(egl_display, 0x3055 /* EGL_EXTENSIONS */) {
            eprintln!("[gl-compositor] EGL extensions: {}", &extensions[..extensions.len().min(200)]);
        }

        // Choose config
        let config_attribs = [
            egl::EGL_RED_SIZE, 8,
            egl::EGL_GREEN_SIZE, 8,
            egl::EGL_BLUE_SIZE, 8,
            egl::EGL_ALPHA_SIZE, 8,
            egl::EGL_RENDERABLE_TYPE, egl::EGL_OPENGL_ES2_BIT,
            egl::EGL_SURFACE_TYPE, egl::EGL_PBUFFER_BIT | egl::EGL_PIXMAP_BIT,
            egl::EGL_NONE,
        ];

        let mut config: egl::EGLConfig = std::ptr::null_mut();
        let mut num_configs = 0;
        let choose_result = unsafe {
            (egl.eglChooseConfig)(
                egl_display,
                config_attribs.as_ptr(),
                &mut config,
                1,
                &mut num_configs,
            )
        };
        if choose_result == egl::EGL_FALSE || num_configs == 0 {
            eprintln!("[gl-compositor] eglChooseConfig failed");
            return None;
        }

        // Bind OpenGL ES API
        unsafe {
            (egl.eglBindAPI)(egl::EGL_OPENGL_ES_API);
        }

        // Create context
        let context_attribs = [
            egl::EGL_CONTEXT_CLIENT_VERSION, 2, // OpenGL ES 2.0
            egl::EGL_NONE,
        ];
        let egl_context = unsafe {
            (egl.eglCreateContext)(egl_display, config, egl::EGL_NO_CONTEXT, context_attribs.as_ptr())
        };
        if egl_context == egl::EGL_NO_CONTEXT {
            eprintln!("[gl-compositor] eglCreateContext failed (error: {:x})", unsafe { egl.eglGetError() });
            return None;
        }

        // Make current with a pbuffer surface (we use FBO for actual rendering)
        let pbuffer_attribs = [
            egl::EGL_WIDTH, width as c_int,
            egl::EGL_HEIGHT, height as c_int,
            egl::EGL_NONE,
        ];
        // We'll create a minimal pbuffer surface just to make the context current
        // In a full implementation we'd use a GBM surface, but for compositing
        // we just need a context to create FBOs

        // For now, skip surface creation and just use the context
        // Make current requires a surface, so create a 1x1 pbuffer
        let tiny_attribs = [
            egl::EGL_WIDTH, 1,
            egl::EGL_HEIGHT, 1,
            egl::EGL_NONE,
        ];

        // We need eglCreatePbufferSurface but that requires a symbol
        // Let's use a simpler approach: create context without making current
        // and manually manage GL state

        // Actually, let's try a different approach: create the pbuffer
        let create_pbuffer: Option<unsafe extern "system" fn(egl::EGLDisplay, egl::EGLConfig, *const c_int) -> egl::EGLSurface> = {
            let c_name = std::ffi::CString::new("eglCreatePbufferSurface").unwrap();
            let sym = unsafe { libc::dlsym(egl.lib, c_name.as_ptr()) };
            if sym.is_null() { None } else { Some(std::mem::transmute(sym)) }
        };

        let pbuffer = if let Some(fn_create) = create_pbuffer {
            unsafe { fn_create(egl_display, config, tiny_attribs.as_ptr()) }
        } else {
            egl::EGL_NO_SURFACE
        };

        if pbuffer != egl::EGL_NO_SURFACE {
            unsafe {
                (egl.eglMakeCurrent)(egl_display, pbuffer, pbuffer, egl_context);
            }
        }

        // Load GL function pointers
        // Create FBO
        let mut fbo = 0;
        let mut fbo_texture = 0;
        let mut fbo_depth = 0;
        unsafe {
            (gl.glGenFramebuffers)(1, &mut fbo);
            (gl.glGenTextures)(1, &mut fbo_texture);
            (gl.glGenRenderbuffers)(1, &mut fbo_depth);
        }

        // Create FBO texture
        unsafe {
            (gl.glBindTexture)(gl::GL_TEXTURE_2D, fbo_texture);
            (gl.glTexImage2D)(
                gl::GL_TEXTURE_2D, 0, gl::GL_RGBA as i32,
                width as i32, height as i32, 0,
                gl::GL_RGBA, gl::GL_UNSIGNED_BYTE,
                std::ptr::null(),
            );
            (gl.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MIN_FILTER, gl::GL_LINEAR as i32);
            (gl.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MAG_FILTER, gl::GL_LINEAR as i32);
            (gl.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_S, gl::GL_CLAMP_TO_EDGE as i32);
            (gl.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_T, gl::GL_CLAMP_TO_EDGE as i32);
        }

        // Create depth renderbuffer
        unsafe {
            (gl.glBindRenderbuffer)(gl::GL_RENDERBUFFER, fbo_depth);
            (gl.glRenderbufferStorage)(
                gl::GL_RENDERBUFFER,
                0x81A5, // GL_DEPTH_COMPONENT16
                width as i32, height as i32,
            );
        }

        // Attach to FBO
        unsafe {
            (gl.glBindFramebuffer)(gl::GL_FRAMEBUFFER, fbo);
            (gl.glFramebufferTexture2D)(
                gl::GL_FRAMEBUFFER, gl::GL_COLOR_ATTACHMENT0,
                gl::GL_TEXTURE_2D, fbo_texture, 0,
            );
            (gl.glFramebufferRenderbuffer)(
                gl::GL_FRAMEBUFFER, gl::GL_DEPTH_ATTACHMENT,
                gl::GL_RENDERBUFFER, fbo_depth,
            );
        }

        // Check FBO status
        let status = unsafe { (gl.glCheckFramebufferStatus)(gl::GL_FRAMEBUFFER) };
        if status != gl::GL_FRAMEBUFFER_COMPLETE {
            eprintln!("[gl-compositor] FBO incomplete (status: {:x})", status);
            return None;
        }
        unsafe { (gl.glBindFramebuffer)(gl::GL_FRAMEBUFFER, 0); }

        // Create shader program
        let shader_program = Self::create_shader_program(&gl);

        // Create quad VBO
        let mut quad_vbo = 0;
        let mut quad_vao = 0;
        unsafe {
            (gl.glGenVertexArrays)(1, &mut quad_vao);
            (gl.glGenBuffers)(1, &mut quad_vbo);

            // Quad vertices: position (2) + texcoord (2)
            let vertices: [f32; 24] = [
                // x, y, u, v
                -1.0, -1.0, 0.0, 1.0,
                 1.0, -1.0, 1.0, 1.0,
                 1.0,  1.0, 1.0, 0.0,
                -1.0,  1.0, 0.0, 0.0,
            ];

            let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

            (gl.glBindVertexArray)(quad_vao);
            (gl.glBindBuffer)(gl::GL_ARRAY_BUFFER, quad_vbo);
            (gl.glBufferData)(
                gl::GL_ARRAY_BUFFER,
                (vertices.len() * 4) as isize,
                vertices.as_ptr() as *const _,
                gl::GL_STATIC_DRAW,
            );

            // Position attribute
            (gl.glEnableVertexAttribArray)(0);
            (gl.glVertexAttribPointer)(0, 2, gl::GL_FLOAT, gl::GL_FALSE, 16, 0 as *const _);
            // Texcoord attribute
            (gl.glEnableVertexAttribArray)(1);
            (gl.glVertexAttribPointer)(1, 2, gl::GL_FLOAT, gl::GL_FALSE, 16, 8 as *const _);

            // Index buffer
            let mut ebo = 0;
            (gl.glGenBuffers)(1, &mut ebo);
            (gl.glBindBuffer)(gl::GL_ELEMENT_ARRAY_BUFFER, ebo);
            (gl.glBufferData)(
                gl::GL_ELEMENT_ARRAY_BUFFER,
                (indices.len() * 2) as isize,
                indices.as_ptr() as *const _,
                gl::GL_STATIC_DRAW,
            );

            (gl.glBindVertexArray)(0);
        }

        eprintln!("[gl-compositor] GL compositor initialized {}x{}", width, height);

        Some(Self {
            egl,
            gl,
            egl_display,
            egl_context,
            fbo,
            fbo_texture,
            fbo_depth,
            shader_program,
            quad_vbo,
            quad_vao,
            width,
            height,
            texture_cache: std::collections::HashMap::new(),
            next_cache_key: 0,
        })
    }

    fn create_shader_program(gl: &Gl) -> u32 {
        let vertex_src = r#"
            #version 100
            attribute vec2 a_position;
            attribute vec2 a_texcoord;
            varying vec2 v_texcoord;
            void main() {
                gl_Position = vec4(a_position, 0.0, 1.0);
                v_texcoord = a_texcoord;
            }
        "#;

        let fragment_src = r#"
            #version 100
            precision mediump float;
            varying vec2 v_texcoord;
            uniform sampler2D u_texture;
            void main() {
                gl_FragColor = texture2D(u_texture, v_texcoord);
            }
        "#;

        let vertex_shader = unsafe {
            let shader = (gl.glCreateShader)(gl::GL_VERTEX_SHADER);
            let src_ptr = vertex_src.as_ptr() as *const *const std::os::raw::c_char;
            let len = vertex_src.len() as i32;
            (gl.glShaderSource)(shader, 1, src_ptr, &len);
            (gl.glCompileShader)(shader);
            let mut status = 0;
            (gl.glGetShaderiv)(shader, gl::GL_COMPILE_STATUS, &mut status);
            if status == 0 {
                let mut log = [0u8; 256];
                (gl.glGetShaderInfoLog)(shader, 256, std::ptr::null_mut(), log.as_mut_ptr() as *mut _);
                eprintln!("[gl-compositor] Vertex shader compile error: {}",
                    String::from_utf8_lossy(&log));
            }
            shader
        };

        let fragment_shader = unsafe {
            let shader = (gl.glCreateShader)(gl::GL_FRAGMENT_SHADER);
            let src_ptr = fragment_src.as_ptr() as *const *const std::os::raw::c_char;
            let len = fragment_src.len() as i32;
            (gl.glShaderSource)(shader, 1, src_ptr, &len);
            (gl.glCompileShader)(shader);
            let mut status = 0;
            (gl.glGetShaderiv)(shader, gl::GL_COMPILE_STATUS, &mut status);
            if status == 0 {
                let mut log = [0u8; 256];
                (gl.glGetShaderInfoLog)(shader, 256, std::ptr::null_mut(), log.as_mut_ptr() as *mut _);
                eprintln!("[gl-compositor] Fragment shader compile error: {}",
                    String::from_utf8_lossy(&log));
            }
            shader
        };

        let program = unsafe {
            let prog = (gl.glCreateProgram)();
            (gl.glAttachShader)(prog, vertex_shader);
            (gl.glAttachShader)(prog, fragment_shader);
            (gl.glLinkProgram)(prog);
            let mut status = 0;
            (gl.glGetProgramiv)(prog, gl::GL_LINK_STATUS, &mut status);
            if status == 0 {
                let mut log = [0u8; 256];
                (gl.glGetProgramInfoLog)(prog, 256, std::ptr::null_mut(), log.as_mut_ptr() as *mut _);
                eprintln!("[gl-compositor] Program link error: {}",
                    String::from_utf8_lossy(&log));
            }
            (gl.glDeleteShader)(vertex_shader);
            (gl.glDeleteShader)(fragment_shader);
            prog
        };

        program
    }

    /// Create or update a GL texture from a surface buffer.
    fn get_or_create_texture(&mut self, surface_id: u32, buf: &SurfaceBuffer) -> Option<&GlTexture> {
        let cache_key = buffer_cache_key(surface_id, buf);

        if self.texture_cache.contains_key(&cache_key) {
            return self.texture_cache.get(&cache_key);
        }

        // Create new texture
        let mut tex_id = 0;
        unsafe { (self.gl.glGenTextures)(1, &mut tex_id); }

        let (width, height) = match buf {
            SurfaceBuffer::Shm { width, height, .. } => (*width as u32, *height as u32),
            SurfaceBuffer::DmaBuf { width, height, .. } => (*width as u32, *height as u32),
            SurfaceBuffer::Dumb { width, height, .. } => (*width, *height),
            SurfaceBuffer::Null => return None,
        };

        match buf {
            SurfaceBuffer::Shm { pool_fd, offset, stride, format, .. } => {
                // Get pool data via cached mapping
                // We need to map the fd to get the pixel data
                // This is the SHM upload path — not ideal but works
                let pool_size = drm::fd_size(*pool_fd).ok()?;
                let mapping = unsafe {
                    libc::mmap(std::ptr::null_mut(), pool_size, libc::PROT_READ,
                               libc::MAP_SHARED, *pool_fd, 0)
                };
                if mapping == libc::MAP_FAILED { return None; }
                let pool_data = unsafe { std::slice::from_raw_parts(mapping as *const u8, pool_size) };
                let off = *offset as usize;
                let len = (*stride as usize) * (*height as usize);
                if off + len > pool_data.len() {
                    unsafe { libc::munmap(mapping, pool_size) };
                    return None;
                }
                let data = &pool_data[off..off + len];

                unsafe { (self.gl.glBindTexture)(gl::GL_TEXTURE_2D, tex_id) };
                unsafe { (self.gl.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MIN_FILTER, gl::GL_LINEAR as i32) };
                unsafe { (self.gl.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MAG_FILTER, gl::GL_LINEAR as i32) };
                unsafe { (self.gl.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_S, gl::GL_CLAMP_TO_EDGE as i32) };
                unsafe { (self.gl.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_T, gl::GL_CLAMP_TO_EDGE as i32) };

                // Upload based on format
                match format {
                    0x34325258 => { // XRGB8888
                        unsafe {
                            (self.gl.glTexImage2D)(
                                gl::GL_TEXTURE_2D, 0, gl::GL_RGBA as i32,
                                *width as i32, *height as i32, 0,
                                gl::GL_BGRA, gl::GL_UNSIGNED_BYTE, // XRGB = BGRA in LE
                                data.as_ptr() as *const _,
                            );
                        }
                    }
                    0x34325241 => { // ARGB8888
                        unsafe {
                            (self.gl.glTexImage2D)(
                                gl::GL_TEXTURE_2D, 0, gl::GL_RGBA as i32,
                                *width as i32, *height as i32, 0,
                                gl::GL_BGRA, gl::GL_UNSIGNED_BYTE,
                                data.as_ptr() as *const _,
                            );
                        }
                    }
                    _ => {
                        unsafe {
                            (self.gl.glTexImage2D)(
                                gl::GL_TEXTURE_2D, 0, gl::GL_RGBA as i32,
                                *width as i32, *height as i32, 0,
                                gl::GL_BGRA, gl::GL_UNSIGNED_BYTE,
                                data.as_ptr() as *const _,
                            );
                        }
                    }
                }

                unsafe { libc::munmap(mapping, pool_size) };

                let tex = GlTexture { id: tex_id, width, height, egl_image: egl::EGL_NO_IMAGE };
                self.texture_cache.insert(cache_key, tex);
            }
            SurfaceBuffer::DmaBuf { width, height, format, plane_fds, offsets, strides, num_planes } => {
                if plane_fds.is_empty() || plane_fds[0] < 0 { return None; }

                // Import DMA-BUF as EGLImage
                let fd = plane_fds[0];
                let buf_offset = offsets.first().copied().unwrap_or(0);
                let buf_stride = strides.first().copied().unwrap_or(*width as u32 * 4);
                let modifier = 0u64; // LINEAR

                let attribs = [
                    egl::EGL_WIDTH, *width as c_int,
                    egl::EGL_HEIGHT, *height as c_int,
                    egl::EGL_LINUX_DRM_FOURCC_EXT, *format as c_int,
                    egl::EGL_DMA_BUF_PLANE0_FD_EXT, fd,
                    egl::EGL_DMA_BUF_PLANE0_OFFSET_EXT, buf_offset as c_int,
                    egl::EGL_DMA_BUF_PLANE0_PITCH_EXT, buf_stride as c_int,
                    egl::EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT, (modifier & 0xFFFFFFFF) as c_int,
                    egl::EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT, ((modifier >> 32) & 0xFFFFFFFF) as c_int,
                    egl::EGL_NONE,
                ];

                let egl_image = unsafe {
                    (self.egl.eglCreateImage)(
                        self.egl_display,
                        self.egl_context,
                        egl::EGL_LINUX_DMA_BUF_EXT,
                        std::ptr::null_mut(),
                        attribs.as_ptr(),
                    )
                };

                if egl_image == egl::EGL_NO_IMAGE {
                    eprintln!("[gl-compositor] eglCreateImage failed for dmabuf (error: {:x})",
                        unsafe { self.egl.eglGetError() });
                    return None;
                }

                // Create texture from EGLImage
                unsafe {
                    (self.gl.glBindTexture)(gl::GL_TEXTURE_2D, tex_id);
                    (self.gl.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MIN_FILTER, gl::GL_LINEAR as i32);
                    (self.gl.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MAG_FILTER, gl::GL_LINEAR as i32);
                    (self.gl.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_S, gl::GL_CLAMP_TO_EDGE as i32);
                    (self.gl.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_T, gl::GL_CLAMP_TO_EDGE as i32);
                    (self.gl.glEGLImageTargetTexture2DOES)(gl::GL_TEXTURE_2D, egl_image);
                }

                let tex = GlTexture { id: tex_id, width: *width as u32, height: *height as u32, egl_image };
                self.texture_cache.insert(cache_key, tex);
            }
            SurfaceBuffer::Dumb { handle: _, fb_id: _, width, height, pitch: _ } => {
                // Dumb buffers — we can read back via mapping and upload
                // This is a fallback path
                let _ = (*width, *height);
            }
            SurfaceBuffer::Null => return None,
        }

        self.texture_cache.get(&cache_key)
    }

    /// Render all surfaces to the FBO, then read back to the dumb buffer.
    pub fn composite(
        &mut self,
        surfaces: &SurfaceTree,
        shell: &Shell,
        cursor: &Cursor,
        dumb: &mut DumbBuffer,
    ) {
        let width = self.width;
        let height = self.height;

        // Bind FBO
        unsafe {
            (self.gl.glBindFramebuffer)(gl::GL_FRAMEBUFFER, self.fbo);
            (self.gl.glViewport)(0, 0, width as i32, height as i32);

            // Clear to dark blue-gray
            (self.gl.glClearColor)(0x1a / 255.0, 0x1a / 255.0, 0x2e / 255.0, 1.0);
            (self.gl.glClear)(gl::GL_COLOR_BUFFER_BIT);

            // Use shader program
            (self.gl.glUseProgram)(self.shader_program);
            let tex_loc = (self.gl.glGetUniformLocation)(self.shader_program, b"u_texture\0".as_ptr() as *const _);
            (self.gl.glUniform1i)(tex_loc, 0);

            // Enable blending
            (self.gl.glEnable)(gl::GL_BLEND);
            (self.gl.glBlendFunc)(gl::GL_ONE, gl::GL_ONE_MINUS_SRC_ALPHA);

            // Bind quad VAO
            (self.gl.glBindVertexArray)(self.quad_vao);
        }

        // Render surfaces back-to-front
        for toplevel in shell.toplevels_z_order() {
            let surface_id = toplevel.surface_id;
            if let Some(surface) = surfaces.get(surface_id) {
                if let Some(ref buf) = surface.buffer {
                    self.render_surface(buf, surface.x, surface.y, width, height);
                }
            }

            // Subsurfaces
            for sub in shell.subsurfaces.for_parent(surface_id) {
                if let Some(sub_surface) = surfaces.get(sub.surface_id) {
                    if let Some(ref buf) = sub_surface.buffer {
                        self.render_surface(buf, sub.x, sub.y, width, height);
                    }
                }
            }
        }

        // TODO: Render cursor
        let _ = cursor;

        unsafe {
            (self.gl.glDisable)(gl::GL_BLEND);
            (self.gl.glBindVertexArray)(0);
            (self.gl.glUseProgram)(0);
            (self.gl.glFinish)();

            // Read back to dumb buffer
            let fb_slice = match dumb.map() {
                Ok(p) => p,
                Err(_) => return,
            };
            let pixels = unsafe { std::slice::from_raw_parts_mut(fb_slice.as_mut_ptr(), fb_slice.len()) };

            // GL uses top-left origin, framebuffer uses bottom-left
            // We need to flip Y or read with proper orientation
            (self.gl.glReadPixels)(
                0, 0,
                width as i32, height as i32,
                gl::GL_BGRA, gl::GL_UNSIGNED_BYTE, // XRGB8888 = BGRA in LE
                pixels.as_mut_ptr() as *mut _,
            );

            // Unbind FBO
            (self.gl.glBindFramebuffer)(gl::GL_FRAMEBUFFER, 0);
        }
    }

    fn render_surface(&mut self, buf: &SurfaceBuffer, x: i32, y: i32, output_width: u32, output_height: u32) {
        let (tex, surf_w, surf_h) = match self.get_or_create_texture(0, buf) {
            Some(t) => (t.id, t.width, t.height),
            None => return,
        };

        // Compute normalized coordinates for the quad
        let x0 = (x as f32) / (output_width as f32) * 2.0 - 1.0;
        let y0 = -((y as f32) / (output_height as f32) * 2.0 - 1.0);
        let w = (surf_w as f32) / (output_width as f32) * 2.0;
        let h = (surf_h as f32) / (output_height as f32) * 2.0;

        // Upload dynamic quad vertices (we'd normally use instanced rendering)
        let vertices: [f32; 24] = [
            x0, y0, 0.0, 1.0,
            x0 + w, y0, 1.0, 1.0,
            x0 + w, y0 + h, 1.0, 0.0,
            x0, y0 + h, 0.0, 0.0,
        ];

        unsafe {
            (self.gl.glActiveTexture)(gl::GL_TEXTURE0);
            (self.gl.glBindTexture)(gl::GL_TEXTURE_2D, tex);

            // Update VBO with new quad
            (self.gl.glBindBuffer)(gl::GL_ARRAY_BUFFER, self.quad_vbo);
            (self.gl.glBufferData)(
                gl::GL_ARRAY_BUFFER,
                (vertices.len() * 4) as isize,
                vertices.as_ptr() as *const _,
                gl::GL_STREAM_DRAW,
            );

            (self.gl.glBindVertexArray)(self.quad_vao);
            (self.gl.glDrawElements)(
                gl::GL_TRIANGLES, 6, gl::GL_UNSIGNED_SHORT, 0 as *const _,
            );
        }
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
}

impl Drop for GlCompositor {
    fn drop(&mut self) {
        unsafe {
            (self.gl.glDeleteTextures)(1, &self.fbo_texture);
            (self.gl.glDeleteFramebuffers)(1, &self.fbo);
            (self.gl.glDeleteRenderbuffers)(1, &self.fbo_depth);
            (self.gl.glDeleteBuffers)(1, &self.quad_vbo);
            (self.gl.glDeleteVertexArrays)(1, &self.quad_vao);
            (self.gl.glDeleteProgram)(self.shader_program);
        }

        // Clean up EGL images in texture cache
        for (_, tex) in self.texture_cache.drain() {
            if tex.egl_image != egl::EGL_NO_IMAGE {
                unsafe {
                    (self.egl.eglDestroyImage)(self.egl_display, tex.egl_image);
                }
            }
            unsafe {
                (self.gl.glDeleteTextures)(1, &tex.id);
            }
        }
    }
}
