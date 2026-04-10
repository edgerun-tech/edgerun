//! Minimal EGL bindings via dynamic loading.
//! Only the subset needed for DMA-BUF import and context management.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

pub type EGLint = c_int;
pub type EGLBoolean = c_int;
pub type EGLenum = c_int;
pub type EGLConfig = *mut c_void;
pub type EGLContext = *mut c_void;
pub type EGLDisplay = *mut c_void;
pub type EGLSurface = *mut c_void;
pub type EGLImage = *mut c_void;
pub type EGLClientBuffer = *mut c_void;
pub type EGLNativeDisplayType = *mut c_void;

pub const EGL_NO_DISPLAY: EGLDisplay = ptr::null_mut();
pub const EGL_NO_CONTEXT: EGLContext = ptr::null_mut();
pub const EGL_NO_SURFACE: EGLSurface = ptr::null_mut();
pub const EGL_NO_IMAGE: EGLImage = ptr::null_mut();

pub const EGL_NO_SYNC_KHR: *mut c_void = ptr::null_mut();

// EGL enum constants
pub const EGL_FALSE: EGLBoolean = 0;
pub const EGL_TRUE: EGLBoolean = 1;

pub const EGL_RENDER_BUFFER: EGLenum = 0x3086;
pub const EGL_CONTEXT_CLIENT_VERSION: EGLenum = 0x3098;
pub const EGL_OPENGL_ES_API: EGLenum = 0x30A0;
pub const EGL_NONE: EGLenum = 0x3038;
pub const EGL_SUCCESS: EGLenum = 0x3000;

pub const EGL_WIDTH: EGLenum = 0x3057;
pub const EGL_HEIGHT: EGLenum = 0x3058;
pub const EGL_TEXTURE_FORMAT: EGLenum = 0x3080;
pub const EGL_TEXTURE_2D: EGLenum = 0x305F;

pub const EGL_BUFFER_SIZE: EGLenum = 0x3020;
pub const EGL_ALPHA_SIZE: EGLenum = 0x3021;
pub const EGL_BLUE_SIZE: EGLenum = 0x3022;
pub const EGL_GREEN_SIZE: EGLenum = 0x3023;
pub const EGL_RED_SIZE: EGLenum = 0x3024;
pub const EGL_DEPTH_SIZE: EGLenum = 0x3025;
pub const EGL_STENCIL_SIZE: EGLenum = 0x3026;
pub const EGL_SAMPLE_BUFFERS: EGLenum = 0x3032;
pub const EGL_SAMPLES: EGLenum = 0x3031;
pub const EGL_RENDERABLE_TYPE: EGLenum = 0x3040;
pub const EGL_SURFACE_TYPE: EGLenum = 0x3033;
pub const EGL_OPENGL_ES2_BIT: EGLenum = 0x0004;
pub const EGL_PIXMAP_BIT: EGLenum = 0x0002;
pub const EGL_WINDOW_BIT: EGLenum = 0x0004;
pub const EGL_PBUFFER_BIT: EGLenum = 0x0001;

// EGL_KHR_platform_gbm
pub const EGL_PLATFORM_GBM_KHR: EGLenum = 0x31D7;

// EGL_EXT_platform_base
pub const EGL_PLATFORM_DEVICE_EXT: EGLenum = 0x313F;

// EGL_EXT_image_dma_buf_import
pub const EGL_LINUX_DMA_BUF_EXT: EGLenum = 0x3270;
pub const EGL_DMA_BUF_PLANE0_FD_EXT: EGLenum = 0x3272;
pub const EGL_DMA_BUF_PLANE0_OFFSET_EXT: EGLenum = 0x3273;
pub const EGL_DMA_BUF_PLANE0_PITCH_EXT: EGLenum = 0x3274;
pub const EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT: EGLenum = 0x3443;
pub const EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT: EGLenum = 0x3444;
pub const EGL_DMA_BUF_PLANE1_FD_EXT: EGLenum = 0x3283;
pub const EGL_DMA_BUF_PLANE1_OFFSET_EXT: EGLenum = 0x3284;
pub const EGL_DMA_BUF_PLANE1_PITCH_EXT: EGLenum = 0x3285;
pub const EGL_DMA_BUF_PLANE2_FD_EXT: EGLenum = 0x3286;
pub const EGL_DMA_BUF_PLANE2_OFFSET_EXT: EGLenum = 0x3287;
pub const EGL_DMA_BUF_PLANE2_PITCH_EXT: EGLenum = 0x3288;
pub const EGL_YUV_COLOR_SPACE_HINT_EXT: EGLenum = 0x3275;
pub const EGL_SAMPLE_RANGE_HINT_EXT: EGLenum = 0x3276;
pub const EGL_YUV_CHROMA_HORIZONTAL_SITING_HINT_EXT: EGLenum = 0x3277;
pub const EGL_YUV_CHROMA_VERTICAL_SITING_HINT_EXT: EGLenum = 0x3278;

// DRM format constants (matching linux_dmabuf)
pub const EGL_LINUX_DRM_FOURCC_EXT: EGLenum = 0x3271;
pub const DRM_FORMAT_ARGB8888: u32 = 0x34325241;
pub const DRM_FORMAT_XRGB8888: u32 = 0x34325258;
pub const DRM_FORMAT_ABGR8888: u32 = 0x34324241;
pub const DRM_FORMAT_RGBA8888: u32 = 0x34324152;

pub const EGL_DRM_MASTER_FD_EXT: EGLenum = 0x333C;

// EGL_DRM_BUFFER_FORMAT_MESA
pub const EGL_DRM_BUFFER_FORMAT_MESA: EGLenum = 0x31D0;
pub const EGL_DRM_BUFFER_USE_MESA: EGLenum = 0x31D1;
pub const EGL_DRM_BUFFER_FORMAT_ARGB32_MESA: EGLenum = 0x31D2;
pub const EGL_DRM_BUFFER_USE_SCANOUT_MESA: EGLenum = 0x00000001;

// EGLImage target
pub const EGL_GL_TEXTURE_2D_KHR: EGLenum = 0x30B1;

pub type PFNEGLGETPLATFORMDISPLAYEXTPROC =
    unsafe extern "system" fn(platform: EGLenum, native_display: *mut c_void, attrib_list: *const EGLint) -> EGLDisplay;
pub type PFNEGLGETDISPLAYPROC =
    unsafe extern "system" fn(display_id: EGLNativeDisplayType) -> EGLDisplay;
pub type PFNEGLQUERYSTRINGPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, name: EGLint) -> *const c_char;
pub type PFNEGLGETERRORPROC =
    unsafe extern "system" fn() -> EGLenum;
pub type PFNEGLINITIALIZEPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, major: *mut EGLint, minor: *mut EGLint) -> EGLBoolean;
pub type PFNEGLCHOOSECONFIGPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, attrib_list: *const EGLint, configs: *mut EGLConfig, config_size: EGLint, num_config: *mut EGLint) -> EGLBoolean;
pub type PFNEGLCREATECONTEXTPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, config: EGLConfig, share_context: EGLContext, attrib_list: *const EGLint) -> EGLContext;
pub type PFNEGLDESTROYCONTEXTPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, ctx: EGLContext) -> EGLBoolean;
pub type PFNEGLMAKECURRENTPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, draw: EGLSurface, read: EGLSurface, ctx: EGLContext) -> EGLBoolean;
pub type PFNEGLBINDAPIPROC =
    unsafe extern "system" fn(api: EGLenum) -> EGLBoolean;
pub type PFNEGLCREATEIMAGEPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, ctx: EGLContext, target: EGLenum, buffer: EGLClientBuffer, attrib_list: *const EGLint) -> EGLImage;
pub type PFNEGLDESTROYIMAGEPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, image: EGLImage) -> EGLBoolean;
pub type PFNEGLCREATESYNCPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, ty: EGLenum, attrib_list: *const EGLint) -> *mut c_void;
pub type PFNEGLDESTROYSYNCPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, sync: *mut c_void) -> EGLBoolean;
pub type PFNEGLWAITSYNCPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, sync: *mut c_void, flags: EGLint) -> EGLint;

pub struct Egl {
    lib: *mut c_void,
    pub eglGetPlatformDisplay: PFNEGLGETPLATFORMDISPLAYEXTPROC,
    pub eglGetDisplay: PFNEGLGETDISPLAYPROC,
    pub eglQueryString: PFNEGLQUERYSTRINGPROC,
    pub eglGetError: PFNEGLGETERRORPROC,
    pub eglInitialize: PFNEGLINITIALIZEPROC,
    pub eglChooseConfig: PFNEGLCHOOSECONFIGPROC,
    pub eglCreateContext: PFNEGLCREATECONTEXTPROC,
    pub eglDestroyContext: PFNEGLDESTROYCONTEXTPROC,
    pub eglMakeCurrent: PFNEGLMAKECURRENTPROC,
    pub eglBindAPI: PFNEGLBINDAPIPROC,
    pub eglCreateImage: PFNEGLCREATEIMAGEPROC,
    pub eglDestroyImage: PFNEGLDESTROYIMAGEPROC,
}

unsafe impl Send for Egl {}
unsafe impl Sync for Egl {}

fn dlopen(name: &str) -> *mut c_void {
    let c_name = CString::new(name).unwrap();
    let handle = unsafe { libc::dlopen(c_name.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL) };
    if handle.is_null() {
        return ptr::null_mut();
    }
    handle
}

fn dlsym<T>(lib: *mut c_void, name: &str) -> Option<T> {
    let c_name = CString::new(name).unwrap();
    let sym_ptr = unsafe { libc::dlsym(lib, c_name.as_ptr()) };
    if sym_ptr.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute_copy(&sym_ptr) })
    }
}

impl Egl {
    pub fn open() -> Option<Self> {
        // Try libEGL.so.1 (Mesa), then libEGL.so
        let lib = dlopen("libEGL.so.1")
            .then_some(())
            .map(|_| unsafe { &mut *(dlopen("libEGL.so.1") as *mut c_void) })
            .unwrap_or_else(|| {
                let l = dlopen("libEGL.so");
                if l.is_null() { return ptr::null_mut(); }
                l
            });

        if lib.is_null() {
            eprintln!("[egl] Failed to load libEGL.so.1 or libEGL.so");
            return None;
        }

        let egl: Self = Self {
            lib,
            eglGetPlatformDisplay: dlsym(lib, "eglGetPlatformDisplayEXT")
                .or_else(|| dlsym(lib, "eglGetPlatformDisplay"))?,
            eglGetDisplay: dlsym(lib, "eglGetDisplay")?,
            eglQueryString: dlsym(lib, "eglQueryString")?,
            eglGetError: dlsym(lib, "eglGetError")?,
            eglInitialize: dlsym(lib, "eglInitialize")?,
            eglChooseConfig: dlsym(lib, "eglChooseConfig")?,
            eglCreateContext: dlsym(lib, "eglCreateContext")?,
            eglDestroyContext: dlsym(lib, "eglDestroyContext")?,
            eglMakeCurrent: dlsym(lib, "eglMakeCurrent")?,
            eglBindAPI: dlsym(lib, "eglBindAPI")?,
            eglCreateImage: dlsym(lib, "eglCreateImage")
                .or_else(|| dlsym(lib, "eglCreateImageKHR"))?,
            eglDestroyImage: dlsym(lib, "eglDestroyImage")
                .or_else(|| dlsym(lib, "eglDestroyImageKHR"))?,
        };

        Some(egl)
    }

    pub fn get_error(&self) -> EGLenum {
        unsafe { (self.eglGetError)() }
    }

    pub fn query_string(&self, dpy: EGLDisplay, name: EGLenum) -> Option<String> {
        let ptr = unsafe { (self.eglQueryString)(dpy, name as EGLint) };
        if ptr.is_null() {
            return None;
        }
        let cstr = unsafe { CStr::from_ptr(ptr) };
        Some(cstr.to_string_lossy().into_owned())
    }
}

impl Drop for Egl {
    fn drop(&mut self) {
        if !self.lib.is_null() {
            unsafe { libc::dlclose(self.lib) };
        }
    }
}
