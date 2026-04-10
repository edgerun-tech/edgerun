//! GL-based compositor — composites surfaces using textured quads via EGL/OpenGL.
//!
//! Architecture:
//! 1. EGL display created from system default
//! 2. EGL context (OpenGL ES 2.0)
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
use crate::gpu::gl;
use crate::render::shm::ShmManager;
use crate::render::cursor::{self, Cursor};

/// A GL texture handle for a surface buffer.
struct GlTexture {
    id: u32,
    width: u32,
    height: u32,
    /// If created from DMA-BUF, the EGLImage that backs this texture.
    egl_image: EGLImage,
}

/// GL compositor state.
pub struct GlCompositor {
    egl: Egl,
    gl_ctx: gl::Gl,
    egl_display: egl::EGLDisplay,
    egl_context: egl::EGLContext,
    /// Offscreen pbuffer surface (needed for context current).
    #[allow(dead_code)]
    egl_surface: egl::EGLSurface,
    /// FBO for offscreen rendering
    fbo: u32,
    /// Color attachment texture for FBO
    fbo_texture: u32,
    /// Depth renderbuffer
    fbo_depth: u32,
    /// Shader program for textured quad rendering
    shader_program: u32,
    /// VAO for quad rendering
    quad_vao: u32,
    /// VBO for quad vertex data
    quad_vbo: u32,
    /// Index buffer for quad
    quad_ebo: u32,
    /// Cursor textures (24 shapes from the Adwaita set).
    cursor_textures: [u32; 24],
    /// Width of the output
    width: u32,
    /// Height of the output
    height: u32,
    /// Texture cache — maps (surface_id, buffer_key) → GlTexture
    texture_cache: std::collections::HashMap<u64, GlTexture>,
}

/// Generate a unique cache key for a surface buffer.
fn buffer_cache_key(surface_id: u32, buf: &SurfaceBuffer) -> u64 {
    let mut key = surface_id as u64;
    match buf {
        SurfaceBuffer::Shm { offset, width, height, .. } => {
            key ^= (*offset as u64) << 16;
            key ^= (*width as u64) << 32;
            key ^= (*height as u64) << 48;
        }
        SurfaceBuffer::Dumb { handle, fb_id, width, height, .. } => {
            key ^= (*handle as u64) << 8;
            key ^= (*fb_id as u64) << 24;
            key ^= (*width as u64) << 40;
            key ^= (*height as u64) << 56;
        }
        SurfaceBuffer::DmaBuf { width, height, format, plane_fds, .. } => {
            key ^= (*width as u64) << 16;
            key ^= (*height as u64) << 32;
            key ^= (*format as u64) << 48;
            if let Some(&fd) = plane_fds.first() {
                key ^= (fd as u64).wrapping_mul(0x100000007);
            }
        }
        SurfaceBuffer::Null => {}
    }
    key
}

impl GlCompositor {
    pub fn new(_drm_fd: c_int, width: u32, height: u32) -> Option<Self> {
        let egl = Egl::open()?;
        let gl_ctx = gl::Gl::open()?;

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
            unsafe { (egl.eglGetDisplay)(std::ptr::null_mut()) }
        } else {
            egl_display
        };

        if egl_display == egl::EGL_NO_DISPLAY {
            eprintln!("[gl-compositor] All EGL display methods failed (error: {:x})",
                unsafe { (egl.eglGetError)() });
            return None;
        }

        // Initialize EGL
        let mut major = 0;
        let mut minor = 0;
        let init_result = unsafe { (egl.eglInitialize)(egl_display, &mut major, &mut minor) };
        if init_result == egl::EGL_FALSE {
            eprintln!("[gl-compositor] eglInitialize failed (error: {:x})", unsafe { (egl.eglGetError)() });
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
            egl::EGL_SURFACE_TYPE, egl::EGL_PBUFFER_BIT,
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
            eprintln!("[gl-compositor] eglChooseConfig failed (error: {:x})", unsafe { (egl.eglGetError)() });
            return None;
        }

        // Bind OpenGL ES API
        unsafe {
            (egl.eglBindAPI)(egl::EGL_OPENGL_ES_API);
        }

        // Create context
        let context_attribs = [
            egl::EGL_CONTEXT_CLIENT_VERSION, 2,
            egl::EGL_NONE,
        ];
        let egl_context = unsafe {
            (egl.eglCreateContext)(egl_display, config, egl::EGL_NO_CONTEXT, context_attribs.as_ptr())
        };
        if egl_context == egl::EGL_NO_CONTEXT {
            eprintln!("[gl-compositor] eglCreateContext failed (error: {:x})", unsafe { (egl.eglGetError)() });
            return None;
        }

        // Create a tiny pbuffer surface just to make context current
        // (we use FBO for actual rendering)
        let create_pbuffer: Option<unsafe extern "system" fn(egl::EGLDisplay, egl::EGLConfig, *const c_int) -> egl::EGLSurface> = {
            let c_name = std::ffi::CString::new("eglCreatePbufferSurface").unwrap();
            let sym = unsafe { libc::dlsym(egl.lib, c_name.as_ptr()) };
            if sym.is_null() { None } else { Some(unsafe { std::mem::transmute(sym) }) }
        };

        let tiny_attribs = [egl::EGL_WIDTH, 1, egl::EGL_HEIGHT, 1, egl::EGL_NONE];
        let egl_surface = if let Some(fn_create) = create_pbuffer {
            unsafe { fn_create(egl_display, config, tiny_attribs.as_ptr()) }
        } else {
            egl::EGL_NO_SURFACE
        };

        if egl_surface != egl::EGL_NO_SURFACE {
            unsafe {
                (egl.eglMakeCurrent)(egl_display, egl_surface, egl_surface, egl_context);
            }
        }

        // Create FBO
        let mut fbo = 0;
        let mut fbo_texture = 0;
        let mut fbo_depth = 0;
        unsafe {
            (gl_ctx.glGenFramebuffers)(1, &mut fbo);
            (gl_ctx.glGenTextures)(1, &mut fbo_texture);
            (gl_ctx.glGenRenderbuffers)(1, &mut fbo_depth);
        }

        // Create FBO texture
        unsafe {
            (gl_ctx.glBindTexture)(gl::GL_TEXTURE_2D, fbo_texture);
            (gl_ctx.glTexImage2D)(
                gl::GL_TEXTURE_2D, 0, gl::GL_RGBA as i32,
                width as i32, height as i32, 0,
                gl::GL_RGBA, gl::GL_UNSIGNED_BYTE,
                std::ptr::null(),
            );
            (gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MIN_FILTER, gl::GL_LINEAR as i32);
            (gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MAG_FILTER, gl::GL_LINEAR as i32);
            (gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_S, gl::GL_CLAMP_TO_EDGE as i32);
            (gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_T, gl::GL_CLAMP_TO_EDGE as i32);
        }

        // Create depth renderbuffer
        unsafe {
            (gl_ctx.glBindRenderbuffer)(gl::GL_RENDERBUFFER, fbo_depth);
            (gl_ctx.glRenderbufferStorage)(
                gl::GL_RENDERBUFFER,
                0x81A5, // GL_DEPTH_COMPONENT16
                width as i32, height as i32,
            );
        }

        // Attach to FBO
        unsafe {
            (gl_ctx.glBindFramebuffer)(gl::GL_FRAMEBUFFER, fbo);
            (gl_ctx.glFramebufferTexture2D)(
                gl::GL_FRAMEBUFFER, gl::GL_COLOR_ATTACHMENT0,
                gl::GL_TEXTURE_2D, fbo_texture, 0,
            );
            (gl_ctx.glFramebufferRenderbuffer)(
                gl::GL_FRAMEBUFFER, gl::GL_DEPTH_ATTACHMENT,
                gl::GL_RENDERBUFFER, fbo_depth,
            );
        }

        let status = unsafe { (gl_ctx.glCheckFramebufferStatus)(gl::GL_FRAMEBUFFER) };
        if status != gl::GL_FRAMEBUFFER_COMPLETE {
            eprintln!("[gl-compositor] FBO incomplete (status: {:x})", status);
            return None;
        }
        unsafe { (gl_ctx.glBindFramebuffer)(gl::GL_FRAMEBUFFER, 0); }

        // Create shader program
        let shader_program = Self::create_shader_program(&gl_ctx);

        // Create quad VAO/VBO/EBO
        let mut quad_vao = 0;
        let mut quad_vbo = 0;
        let mut quad_ebo = 0;
        unsafe {
            (gl_ctx.glGenVertexArrays)(1, &mut quad_vao);
            (gl_ctx.glGenBuffers)(1, &mut quad_vbo);
            (gl_ctx.glGenBuffers)(1, &mut quad_ebo);

            // Index buffer (static)
            let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
            (gl_ctx.glBindBuffer)(gl::GL_ELEMENT_ARRAY_BUFFER, quad_ebo);
            (gl_ctx.glBufferData)(
                gl::GL_ELEMENT_ARRAY_BUFFER,
                (indices.len() * 2) as isize,
                indices.as_ptr() as *const _,
                gl::GL_STATIC_DRAW,
            );

            (gl_ctx.glBindVertexArray)(0);
        }

        // Create cursor textures from Adwaita bitmaps (all 24 shapes)
        let mut cursor_textures = [0u32; 24];
        unsafe {
            (gl_ctx.glGenTextures)(24, cursor_textures.as_mut_ptr());
            for idx in 0..24 {
                let tex = cursor_textures[idx];
                (gl_ctx.glBindTexture)(gl::GL_TEXTURE_2D, tex);
                (gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MIN_FILTER, gl::GL_LINEAR as i32);
                (gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MAG_FILTER, gl::GL_LINEAR as i32);
                (gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_S, gl::GL_CLAMP_TO_EDGE as i32);
                (gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_T, gl::GL_CLAMP_TO_EDGE as i32);
                (gl_ctx.glTexImage2D)(
                    gl::GL_TEXTURE_2D, 0, gl::GL_RGBA as i32,
                    cursor::CURSOR_SIZE as i32, cursor::CURSOR_SIZE as i32, 0,
                    gl::GL_RGBA, gl::GL_UNSIGNED_BYTE,
                    cursor::CURSOR_BITMAPS[idx].as_ptr() as *const _,
                );
            }
        }

        eprintln!("[gl-compositor] GL compositor initialized {}x{}", width, height);

        Some(Self {
            egl,
            gl_ctx,
            egl_display,
            egl_context,
            egl_surface,
            fbo,
            fbo_texture,
            fbo_depth,
            shader_program,
            quad_vao,
            quad_vbo,
            quad_ebo,
            cursor_textures,
            width,
            height,
            texture_cache: std::collections::HashMap::new(),
        })
    }

    fn create_shader_program(gl: &gl::Gl) -> u32 {
        let vertex_src = b"#version 100\nattribute vec2 a_position;\nattribute vec2 a_texcoord;\nvarying vec2 v_texcoord;\nvoid main() {\ngl_Position = vec4(a_position, 0.0, 1.0);\nv_texcoord = a_texcoord;\n}\n\0";
        let fragment_src = b"#version 100\nprecision mediump float;\nvarying vec2 v_texcoord;\nuniform sampler2D u_texture;\nvoid main() {\ngl_FragColor = texture2D(u_texture, v_texcoord);\n}\n\0";

        let vertex_shader = unsafe {
            let shader = (gl.glCreateShader)(gl::GL_VERTEX_SHADER);
            let src_ptr = vertex_src.as_ptr() as *const *const std::os::raw::c_char;
            let len = vertex_src.len() as i32 - 1; // exclude null terminator
            (gl.glShaderSource)(shader, 1, src_ptr, &len);
            (gl.glCompileShader)(shader);
            shader
        };

        let fragment_shader = unsafe {
            let shader = (gl.glCreateShader)(gl::GL_FRAGMENT_SHADER);
            let src_ptr = fragment_src.as_ptr() as *const *const std::os::raw::c_char;
            let len = fragment_src.len() as i32 - 1;
            (gl.glShaderSource)(shader, 1, src_ptr, &len);
            (gl.glCompileShader)(shader);
            shader
        };

        let program = unsafe {
            let prog = (gl.glCreateProgram)();
            (gl.glAttachShader)(prog, vertex_shader);
            (gl.glAttachShader)(prog, fragment_shader);
            (gl.glLinkProgram)(prog);
            (gl.glDeleteShader)(vertex_shader);
            (gl.glDeleteShader)(fragment_shader);
            prog
        };

        program
    }

    /// Create or update a GL texture from a surface buffer.
    fn get_or_create_texture(&mut self, shm: &ShmManager, surface_id: u32, buf: &SurfaceBuffer) -> Option<(u32, u32, u32)> {
        let cache_key = buffer_cache_key(surface_id, buf);

        if let Some(tex) = self.texture_cache.get(&cache_key) {
            return Some((tex.id, tex.width, tex.height));
        }

        let mut tex_id = 0;
        unsafe { (self.gl_ctx.glGenTextures)(1, &mut tex_id); }

        let (width, height) = match buf {
            SurfaceBuffer::Shm { width, height, .. } => (*width as u32, *height as u32),
            SurfaceBuffer::DmaBuf { width, height, .. } => (*width as u32, *height as u32),
            SurfaceBuffer::Dumb { width, height, .. } => (*width, *height),
            SurfaceBuffer::Null => return None,
        };

        match buf {
            SurfaceBuffer::Shm { pool_fd, offset, stride, format: _, width: w, height: h } => {
                // Use cached SHM pool mapping via O(1) fd lookup
                let data: &[u8] = if let Some(pool) = shm.get_pool_by_fd(*pool_fd) {
                    let len = (*stride as usize) * (*h as usize);
                    match pool.read(*offset as usize, len) {
                        Some(d) => d,
                        None => return None,
                    }
                } else {
                    // Fallback: mmap temporarily if pool not in manager
                    let pool_size = drm::fd_size(*pool_fd).ok()?;
                    let mapping = unsafe {
                        libc::mmap(std::ptr::null_mut(), pool_size, libc::PROT_READ,
                                   libc::MAP_SHARED, *pool_fd, 0)
                    };
                    if mapping == libc::MAP_FAILED { return None; }
                    let pool_data = unsafe { std::slice::from_raw_parts(mapping as *const u8, pool_size) };
                    let off = *offset as usize;
                    let len = (*stride as usize) * (*h as usize);
                    if off + len > pool_data.len() {
                        unsafe { libc::munmap(mapping, pool_size) };
                        return None;
                    }
                    let data = &pool_data[off..off + len];
                    // Upload texture from fallback mapping
                    unsafe {
                        (self.gl_ctx.glBindTexture)(gl::GL_TEXTURE_2D, tex_id);
                        (self.gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MIN_FILTER, gl::GL_LINEAR as i32);
                        (self.gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MAG_FILTER, gl::GL_LINEAR as i32);
                        (self.gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_S, gl::GL_CLAMP_TO_EDGE as i32);
                        (self.gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_T, gl::GL_CLAMP_TO_EDGE as i32);
                        (self.gl_ctx.glTexImage2D)(
                            gl::GL_TEXTURE_2D, 0, gl::GL_RGBA as i32,
                            *w as i32, *h as i32, 0,
                            gl::GL_BGRA, gl::GL_UNSIGNED_BYTE,
                            data.as_ptr() as *const _,
                        );
                    }
                    unsafe { libc::munmap(mapping, pool_size) };
                    let tex = GlTexture { id: tex_id, width: *w as u32, height: *h as u32, egl_image: egl::EGL_NO_IMAGE };
                    self.texture_cache.insert(cache_key, tex);
                    return Some((tex_id, *w as u32, *h as u32));
                };

                unsafe {
                    (self.gl_ctx.glBindTexture)(gl::GL_TEXTURE_2D, tex_id);
                    (self.gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MIN_FILTER, gl::GL_LINEAR as i32);
                    (self.gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MAG_FILTER, gl::GL_LINEAR as i32);
                    (self.gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_S, gl::GL_CLAMP_TO_EDGE as i32);
                    (self.gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_T, gl::GL_CLAMP_TO_EDGE as i32);
                    // XRGB8888/ARGB8888 in little-endian = BGRA
                    (self.gl_ctx.glTexImage2D)(
                        gl::GL_TEXTURE_2D, 0, gl::GL_RGBA as i32,
                        *w as i32, *h as i32, 0,
                        gl::GL_BGRA, gl::GL_UNSIGNED_BYTE,
                        data.as_ptr() as *const _,
                    );
                }

                let tex = GlTexture { id: tex_id, width: *w as u32, height: *h as u32, egl_image: egl::EGL_NO_IMAGE };
                self.texture_cache.insert(cache_key, tex);
            }
            SurfaceBuffer::DmaBuf { width, height, format, plane_fds, offsets, strides, .. } => {
                if plane_fds.is_empty() || plane_fds[0] < 0 { return None; }

                let fd = plane_fds[0];
                let buf_offset = offsets.first().copied().unwrap_or(0);
                let buf_stride = strides.first().copied().unwrap_or(*width as u32 * 4);
                let modifier = 0u64;

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
                    eprintln!("[gl-compositor] eglCreateImage failed (error: {:x})",
                        unsafe { (self.egl.eglGetError)() });
                    return None;
                }

                unsafe {
                    (self.gl_ctx.glBindTexture)(gl::GL_TEXTURE_2D, tex_id);
                    (self.gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MIN_FILTER, gl::GL_LINEAR as i32);
                    (self.gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MAG_FILTER, gl::GL_LINEAR as i32);
                    (self.gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_S, gl::GL_CLAMP_TO_EDGE as i32);
                    (self.gl_ctx.glTexParameteri)(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_WRAP_T, gl::GL_CLAMP_TO_EDGE as i32);
                    (self.gl_ctx.glEGLImageTargetTexture2DOES)(gl::GL_TEXTURE_2D, egl_image);
                }

                let tex = GlTexture { id: tex_id, width: *width as u32, height: *height as u32, egl_image };
                self.texture_cache.insert(cache_key, tex);
            }
            SurfaceBuffer::Dumb { .. } | SurfaceBuffer::Null => return None,
        }

        Some((tex_id, width, height))
    }

    /// Render all surfaces to the FBO, then read back to the dumb buffer.
    pub fn composite(
        &mut self,
        shm: &ShmManager,
        surfaces: &SurfaceTree,
        shell: &Shell,
        cursor: &Cursor,
        dumb: &mut DumbBuffer,
    ) {
        let width = self.width;
        let height = self.height;

        // Bind FBO
        unsafe {
            (self.gl_ctx.glBindFramebuffer)(gl::GL_FRAMEBUFFER, self.fbo);
            (self.gl_ctx.glViewport)(0, 0, width as i32, height as i32);
            (self.gl_ctx.glClearColor)(0x1a as f32 / 255.0, 0x1a as f32 / 255.0, 0x2e as f32 / 255.0, 1.0);
            (self.gl_ctx.glClear)(gl::GL_COLOR_BUFFER_BIT);
            (self.gl_ctx.glUseProgram)(self.shader_program);
            let tex_loc = (self.gl_ctx.glGetUniformLocation)(self.shader_program, b"u_texture\0".as_ptr() as *const _);
            (self.gl_ctx.glUniform1i)(tex_loc, 0);
            (self.gl_ctx.glEnable)(gl::GL_BLEND);
            (self.gl_ctx.glBlendFunc)(gl::GL_ONE, gl::GL_ONE_MINUS_SRC_ALPHA);
        }

        // Render surfaces back-to-front
        for toplevel in shell.toplevels_z_order() {
            let surface_id = toplevel.surface_id;
            if let Some(surface) = surfaces.get(surface_id) {
                if let Some(ref buf) = surface.buffer {
                    self.render_surface(shm, surface_id, buf, surface.x, surface.y, width, height);
                }
            }
            for sub in shell.subsurfaces.for_parent(surface_id) {
                if let Some(sub_surface) = surfaces.get(sub.surface_id) {
                    if let Some(ref buf) = sub_surface.buffer {
                        self.render_surface(shm, sub.surface_id, buf, sub.x, sub.y, width, height);
                    }
                }
            }
        }

        // Render cursor
        self.render_cursor(cursor, width, height);

        unsafe {
            (self.gl_ctx.glDisable)(gl::GL_BLEND);
            (self.gl_ctx.glUseProgram)(0);
            (self.gl_ctx.glFinish)();

            // Read back to dumb buffer
            let fb_slice = match dumb.map() {
                Ok(p) => p,
                Err(_) => return,
            };
            let pixels = std::slice::from_raw_parts_mut(fb_slice.as_mut_ptr(), fb_slice.len());

            (self.gl_ctx.glReadPixels)(
                0, 0,
                width as i32, height as i32,
                gl::GL_BGRA, gl::GL_UNSIGNED_BYTE,
                pixels.as_mut_ptr() as *mut _,
            );

            (self.gl_ctx.glBindFramebuffer)(gl::GL_FRAMEBUFFER, 0);
        }
    }

    fn render_surface(&mut self, shm: &ShmManager, surface_id: u32, buf: &SurfaceBuffer, x: i32, y: i32, output_width: u32, output_height: u32) {
        let (tex, surf_w, surf_h) = match self.get_or_create_texture(shm, surface_id, buf) {
            Some(t) => t,
            None => return,
        };

        let x0 = (x as f32) / (output_width as f32) * 2.0 - 1.0;
        let y0 = -((y as f32) / (output_height as f32) * 2.0 - 1.0);
        let w = (surf_w as f32) / (output_width as f32) * 2.0;
        let h = (surf_h as f32) / (output_height as f32) * 2.0;

        let vertices: [f32; 16] = [
            x0, y0, 0.0, 1.0,
            x0 + w, y0, 1.0, 1.0,
            x0 + w, y0 + h, 1.0, 0.0,
            x0, y0 + h, 0.0, 0.0,
        ];

        unsafe {
            (self.gl_ctx.glActiveTexture)(gl::GL_TEXTURE0);
            (self.gl_ctx.glBindTexture)(gl::GL_TEXTURE_2D, tex);
            (self.gl_ctx.glBindBuffer)(gl::GL_ARRAY_BUFFER, self.quad_vbo);
            (self.gl_ctx.glBufferData)(
                gl::GL_ARRAY_BUFFER,
                (vertices.len() * 4) as isize,
                vertices.as_ptr() as *const _,
                gl::GL_STREAM_DRAW,
            );

            (self.gl_ctx.glBindVertexArray)(self.quad_vao);
            (self.gl_ctx.glEnableVertexAttribArray)(0);
            (self.gl_ctx.glVertexAttribPointer)(0, 2, gl::GL_FLOAT, gl::GL_FALSE, 16, 0 as *const _);
            (self.gl_ctx.glEnableVertexAttribArray)(1);
            (self.gl_ctx.glVertexAttribPointer)(1, 2, gl::GL_FLOAT, gl::GL_FALSE, 16, 8 as *const _);
            (self.gl_ctx.glBindBuffer)(gl::GL_ELEMENT_ARRAY_BUFFER, self.quad_ebo);

            (self.gl_ctx.glDrawElements)(
                gl::GL_TRIANGLES, 6, gl::GL_UNSIGNED_SHORT, 0 as *const _,
            );
        }
    }

    /// Render the cursor as a textured quad using the current shape.
    fn render_cursor(&mut self, cursor: &Cursor, output_width: u32, output_height: u32) {
        if !cursor.visible {
            return;
        }

        let cursor_w = cursor::CURSOR_SIZE as f32;
        let cursor_h = cursor::CURSOR_SIZE as f32;
        let (hx, hy) = cursor::CURSOR_HOTSPOTS[cursor.shape_index];

        let x0 = (cursor.x as f32 - hx as f32) / (output_width as f32) * 2.0 - 1.0;
        let y0 = -((cursor.y as f32 - hy as f32) / (output_height as f32) * 2.0 - 1.0);
        let w = cursor_w / (output_width as f32) * 2.0;
        let h = cursor_h / (output_height as f32) * 2.0;

        let vertices: [f32; 16] = [
            x0, y0, 0.0, 1.0,
            x0 + w, y0, 1.0, 1.0,
            x0 + w, y0 + h, 1.0, 0.0,
            x0, y0 + h, 0.0, 0.0,
        ];

        unsafe {
            (self.gl_ctx.glActiveTexture)(gl::GL_TEXTURE0);
            (self.gl_ctx.glBindTexture)(gl::GL_TEXTURE_2D, self.cursor_textures[cursor.shape_index]);
            (self.gl_ctx.glEnable)(gl::GL_BLEND);
            (self.gl_ctx.glBlendFunc)(gl::GL_ONE, gl::GL_ONE_MINUS_SRC_ALPHA);
            (self.gl_ctx.glUseProgram)(self.shader_program);
            let tex_loc = (self.gl_ctx.glGetUniformLocation)(self.shader_program, b"u_texture\0".as_ptr() as *const _);
            (self.gl_ctx.glUniform1i)(tex_loc, 0);

            (self.gl_ctx.glBindBuffer)(gl::GL_ARRAY_BUFFER, self.quad_vbo);
            (self.gl_ctx.glBufferData)(
                gl::GL_ARRAY_BUFFER,
                (vertices.len() * 4) as isize,
                vertices.as_ptr() as *const _,
                gl::GL_STREAM_DRAW,
            );

            (self.gl_ctx.glBindVertexArray)(self.quad_vao);
            (self.gl_ctx.glEnableVertexAttribArray)(0);
            (self.gl_ctx.glVertexAttribPointer)(0, 2, gl::GL_FLOAT, gl::GL_FALSE, 16, 0 as *const _);
            (self.gl_ctx.glEnableVertexAttribArray)(1);
            (self.gl_ctx.glVertexAttribPointer)(1, 2, gl::GL_FLOAT, gl::GL_FALSE, 16, 8 as *const _);
            (self.gl_ctx.glBindBuffer)(gl::GL_ELEMENT_ARRAY_BUFFER, self.quad_ebo);

            (self.gl_ctx.glDrawElements)(
                gl::GL_TRIANGLES, 6, gl::GL_UNSIGNED_SHORT, 0 as *const _,
            );
        }
    }
}

impl Drop for GlCompositor {
    fn drop(&mut self) {
        unsafe {
            (self.gl_ctx.glDeleteTextures)(1, &self.fbo_texture);
            (self.gl_ctx.glDeleteFramebuffers)(1, &self.fbo);
            (self.gl_ctx.glDeleteRenderbuffers)(1, &self.fbo_depth);
            (self.gl_ctx.glDeleteBuffers)(1, &self.quad_vbo);
            (self.gl_ctx.glDeleteBuffers)(1, &self.quad_ebo);
            (self.gl_ctx.glDeleteTextures)(24, self.cursor_textures.as_ptr());
            (self.gl_ctx.glDeleteVertexArrays)(1, &self.quad_vao);
            (self.gl_ctx.glDeleteProgram)(self.shader_program);
        }

        for (_, tex) in self.texture_cache.drain() {
            if tex.egl_image != egl::EGL_NO_IMAGE {
                unsafe {
                    (self.egl.eglDestroyImage)(self.egl_display, tex.egl_image);
                }
            }
            unsafe {
                (self.gl_ctx.glDeleteTextures)(1, &tex.id);
            }
        }
    }
}
