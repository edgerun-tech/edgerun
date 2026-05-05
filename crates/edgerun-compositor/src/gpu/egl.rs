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

// GBM buffer usage
pub const GBM_BO_USE_SCANOUT: u32 = 1 << 0;
pub const GBM_BO_USE_RENDERING: u32 = 1 << 1;

pub type PFNEGLGETPLATFORMDISPLAYEXTPROC = unsafe extern "system" fn(
    platform: EGLenum,
    native_display: *mut c_void,
    attrib_list: *const EGLint,
) -> EGLDisplay;
pub type PFNEGLGETDISPLAYPROC =
    unsafe extern "system" fn(display_id: EGLNativeDisplayType) -> EGLDisplay;
pub type PFNEGLQUERYSTRINGPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, name: EGLint) -> *const c_char;
pub type PFNEGLGETERRORPROC = unsafe extern "system" fn() -> EGLenum;
pub type PFNEGLINITIALIZEPROC = unsafe extern "system" fn(
    dpy: EGLDisplay,
    major: *mut EGLint,
    minor: *mut EGLint,
) -> EGLBoolean;
pub type PFNEGLCHOOSECONFIGPROC = unsafe extern "system" fn(
    dpy: EGLDisplay,
    attrib_list: *const EGLint,
    configs: *mut EGLConfig,
    config_size: EGLint,
    num_config: *mut EGLint,
) -> EGLBoolean;
pub type PFNEGLCREATECONTEXTPROC = unsafe extern "system" fn(
    dpy: EGLDisplay,
    config: EGLConfig,
    share_context: EGLContext,
    attrib_list: *const EGLint,
) -> EGLContext;
pub type PFNEGLDESTROYCONTEXTPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, ctx: EGLContext) -> EGLBoolean;
pub type PFNEGLMAKECURRENTPROC = unsafe extern "system" fn(
    dpy: EGLDisplay,
    draw: EGLSurface,
    read: EGLSurface,
    ctx: EGLContext,
) -> EGLBoolean;
pub type PFNEGLBINDAPIPROC = unsafe extern "system" fn(api: EGLenum) -> EGLBoolean;
pub type PFNEGLCREATEIMAGEPROC = unsafe extern "system" fn(
    dpy: EGLDisplay,
    ctx: EGLContext,
    target: EGLenum,
    buffer: EGLClientBuffer,
    attrib_list: *const EGLint,
) -> EGLImage;
pub type PFNEGLDESTROYIMAGEPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, image: EGLImage) -> EGLBoolean;
pub type PFNEGLCREATESYNCPROC = unsafe extern "system" fn(
    dpy: EGLDisplay,
    ty: EGLenum,
    attrib_list: *const EGLint,
) -> *mut c_void;
pub type PFNEGLDESTROYSYNCPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, sync: *mut c_void) -> EGLBoolean;
pub type PFNEGLWAITSYNCPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, sync: *mut c_void, flags: EGLint) -> EGLint;
pub type PFNEGLTERMINATEPROC = unsafe extern "system" fn(dpy: EGLDisplay) -> EGLBoolean;
pub type PFNEGLDESTROYSURFACEPROC =
    unsafe extern "system" fn(dpy: EGLDisplay, surface: EGLSurface) -> EGLBoolean;

pub struct Egl {
    pub lib: *mut c_void,
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
    pub eglTerminate: PFNEGLTERMINATEPROC,
    pub eglDestroySurface: PFNEGLDESTROYSURFACEPROC,
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
        let lib = {
            let l = dlopen("libEGL.so.1");
            if !l.is_null() {
                eprintln!("[egl] Loaded libEGL.so.1");
                l
            } else {
                let l = dlopen("libEGL.so");
                if l.is_null() {
                    eprintln!("[egl] Failed to load libEGL.so.1 or libEGL.so");
                    return None;
                }
                eprintln!("[egl] Loaded libEGL.so");
                l
            }
        };

        macro_rules! resolve {
            ($name:expr) => {{
                let sym = dlsym(lib, $name);
                if sym.is_none() {
                    eprintln!("[egl] Failed to resolve symbol: {}", $name);
                }
                sym
            }};
            ($name:expr, $alt:expr) => {{
                let sym = dlsym(lib, $name).or_else(|| dlsym(lib, $alt));
                if sym.is_none() {
                    eprintln!("[egl] Failed to resolve symbols: {} or {}", $name, $alt);
                }
                sym
            }};
        }

        Some(Self {
            lib,
            eglGetPlatformDisplay: resolve!("eglGetPlatformDisplayEXT", "eglGetPlatformDisplay")?,
            eglGetDisplay: resolve!("eglGetDisplay")?,
            eglQueryString: resolve!("eglQueryString")?,
            eglGetError: resolve!("eglGetError")?,
            eglInitialize: resolve!("eglInitialize")?,
            eglChooseConfig: resolve!("eglChooseConfig")?,
            eglCreateContext: resolve!("eglCreateContext")?,
            eglDestroyContext: resolve!("eglDestroyContext")?,
            eglMakeCurrent: resolve!("eglMakeCurrent")?,
            eglBindAPI: resolve!("eglBindAPI")?,
            eglCreateImage: resolve!("eglCreateImage", "eglCreateImageKHR")?,
            eglDestroyImage: resolve!("eglDestroyImage", "eglDestroyImageKHR")?,
            eglTerminate: resolve!("eglTerminate")?,
            eglDestroySurface: resolve!("eglDestroySurface")?,
        })
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

// ─── GBM bindings ─────────────────────────────────────────────

pub type GbmDevice = c_void;
pub type GbmSurface = c_void;
pub type GbmBo = c_void;

pub type PfngbmCreateDevice = unsafe extern "system" fn(fd: c_int) -> *mut GbmDevice;
pub type PfngbmDeviceDestroy = unsafe extern "system" fn(gbm: *mut GbmDevice);
pub type PfngbmDeviceGetFd = unsafe extern "system" fn(gbm: *mut GbmDevice) -> c_int;
pub type PfngbmCreateSurface = unsafe extern "system" fn(
    gbm: *mut GbmDevice,
    width: u32,
    height: u32,
    format: u32,
    flags: u32,
) -> *mut GbmSurface;
pub type PfngbmSurfaceDestroy = unsafe extern "system" fn(gs: *mut GbmSurface);
pub type PfngbmSurfaceLockFrontBuffer =
    unsafe extern "system" fn(gs: *mut GbmSurface) -> *mut GbmBo;
pub type PfngbmBoGetHandle = unsafe extern "system" fn(bo: *mut GbmBo) -> u32;
pub type PfngbmBoGetStride = unsafe extern "system" fn(bo: *mut GbmBo) -> u32;
pub type PfngbmBoGetFormat = unsafe extern "system" fn(bo: *mut GbmBo) -> u32;
pub type PfngbmBoGetWidth = unsafe extern "system" fn(bo: *mut GbmBo) -> u32;
pub type PfngbmBoGetHeight = unsafe extern "system" fn(bo: *mut GbmBo) -> u32;
pub type PfngbmBoUnmap = unsafe extern "system" fn(bo: *mut GbmBo);
pub type PfngbmBoMap = unsafe extern "system" fn(
    bo: *mut GbmBo,
    offset: u32,
    width: u32,
    height: u32,
    flags: i32,
    stride: *mut u32,
    map_data: *mut *mut c_void,
) -> *mut c_void;
pub type PfngbmBoRelease = unsafe extern "system" fn(bo: *mut GbmBo);
pub type PfngbmSurfaceReleaseBuffer =
    unsafe extern "system" fn(gs: *mut GbmSurface, bo: *mut GbmBo);
pub type PfngbmDeviceIsFormatSupported =
    unsafe extern "system" fn(gbm: *mut GbmDevice, format: u32, width: u32, height: u32) -> i32;

pub struct Gbm {
    pub lib: *mut c_void,
    pub gbm_create_device: PfngbmCreateDevice,
    pub gbm_device_destroy: PfngbmDeviceDestroy,
    pub gbm_device_get_fd: PfngbmDeviceGetFd,
    pub gbm_create_surface: PfngbmCreateSurface,
    pub gbm_surface_destroy: PfngbmSurfaceDestroy,
    pub gbm_bo_get_handle: PfngbmBoGetHandle,
    pub gbm_bo_get_stride: PfngbmBoGetStride,
    pub gbm_bo_get_format: PfngbmBoGetFormat,
    pub gbm_bo_get_width: PfngbmBoGetWidth,
    pub gbm_bo_get_height: PfngbmBoGetHeight,
    pub gbm_bo_map: PfngbmBoMap,
    pub gbm_bo_unmap: PfngbmBoUnmap,
    pub gbm_bo_release: PfngbmBoRelease,
    pub gbm_surface_release_buffer: PfngbmSurfaceReleaseBuffer,
}

unsafe impl Send for Gbm {}
unsafe impl Sync for Gbm {}

impl Gbm {
    pub fn open() -> Option<Self> {
        let lib = {
            let l = dlopen("libgbm.so.1");
            if l.is_null() {
                let l = dlopen("libgbm.so");
                if l.is_null() {
                    eprintln!("[gbm] Failed to load libgbm.so");
                    return None;
                }
                l
            } else {
                l
            }
        };

        Some(Self {
            lib,
            gbm_create_device: dlsym(lib, "gbm_create_device")?,
            gbm_device_destroy: dlsym(lib, "gbm_device_destroy")?,
            gbm_device_get_fd: dlsym(lib, "gbm_device_get_fd")?,
            gbm_create_surface: dlsym(lib, "gbm_surface_create")?,
            gbm_surface_destroy: dlsym(lib, "gbm_surface_destroy")?,
            gbm_bo_get_handle: dlsym(lib, "gbm_bo_get_handle")?,
            gbm_bo_get_stride: dlsym(lib, "gbm_bo_get_stride")?,
            gbm_bo_get_format: dlsym(lib, "gbm_bo_get_format")?,
            gbm_bo_get_width: dlsym(lib, "gbm_bo_get_width")?,
            gbm_bo_get_height: dlsym(lib, "gbm_bo_get_height")?,
            gbm_bo_map: dlsym(lib, "gbm_bo_map")?,
            gbm_bo_unmap: dlsym(lib, "gbm_bo_unmap")?,
            gbm_bo_release: dlsym(lib, "gbm_bo_release")?,
            gbm_surface_release_buffer: dlsym(lib, "gbm_surface_release_buffer")?,
        })
    }
}

impl Drop for Gbm {
    fn drop(&mut self) {
        if !self.lib.is_null() {
            unsafe { libc::dlclose(self.lib) };
        }
    }
}
