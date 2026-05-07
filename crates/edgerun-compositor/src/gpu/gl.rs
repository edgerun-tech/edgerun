//! Minimal GL bindings via dynamic loading.
//! Only the subset needed for compositing (textured quads).

use crate::libc;
use std::ffi::CString;
use std::os::raw::{c_char, c_float, c_int, c_void};

pub type GLenum = c_int;
pub type GLint = c_int;
pub type GLuint = u32;
pub type GLsizei = c_int;
pub type GLboolean = u8;
pub type GLbitfield = c_int;
pub type GLvoid = c_void;
pub type GLsizeiptr = isize;
pub type GLintptr = isize;
pub type GLfloat = c_float;

// GL constants
pub const GL_TEXTURE_2D: GLenum = 0x0DE1;
pub const GL_TEXTURE0: GLenum = 0x84C0;
pub const GL_RGBA: GLenum = 0x1908;
pub const GL_RGB: GLenum = 0x1907;
pub const GL_BGRA: GLenum = 0x80E1;
pub const GL_UNSIGNED_BYTE: GLenum = 0x1401;
pub const GL_TEXTURE_MIN_FILTER: GLenum = 0x2801;
pub const GL_TEXTURE_MAG_FILTER: GLenum = 0x2800;
pub const GL_TEXTURE_WRAP_S: GLenum = 0x2802;
pub const GL_TEXTURE_WRAP_T: GLenum = 0x2803;
pub const GL_LINEAR: GLenum = 0x2601;
pub const GL_CLAMP_TO_EDGE: GLenum = 0x812F;
pub const GL_BLEND: GLenum = 0x0BE2;
pub const GL_ONE: GLenum = 1;
pub const GL_ONE_MINUS_SRC_ALPHA: GLenum = 0x0303;
pub const GL_SRC_ALPHA: GLenum = 0x0302;
pub const GL_FUNC_ADD: GLenum = 0x8006;
pub const GL_BLEND_EQUATION: GLenum = 0x8009;
pub const GL_BLEND_EQUATION_RGB: GLenum = 0x8009;
pub const GL_BLEND_EQUATION_ALPHA: GLenum = 0x883D;
pub const GL_COLOR_BUFFER_BIT: GLbitfield = 0x00004000;
pub const GL_ARRAY_BUFFER: GLenum = 0x8892;
pub const GL_ELEMENT_ARRAY_BUFFER: GLenum = 0x8893;
pub const GL_STATIC_DRAW: GLenum = 0x88E4;
pub const GL_STREAM_DRAW: GLenum = 0x88E0;
pub const GL_FLOAT: GLenum = 0x1406;
pub const GL_FALSE: GLboolean = 0;
pub const GL_TRUE: GLboolean = 1;
pub const GL_FRAGMENT_SHADER: GLenum = 0x8B30;
pub const GL_VERTEX_SHADER: GLenum = 0x8B31;
pub const GL_COMPILE_STATUS: GLenum = 0x8B81;
pub const GL_LINK_STATUS: GLenum = 0x8B82;
pub const GL_FRAMEBUFFER: GLenum = 0x8D40;
pub const GL_RENDERBUFFER: GLenum = 0x8D41;
pub const GL_COLOR_ATTACHMENT0: GLenum = 0x8CE0;
pub const GL_FRAMEBUFFER_COMPLETE: GLenum = 0x8CD5;
pub const GL_DEPTH_ATTACHMENT: GLenum = 0x8D00;
pub const GL_TRIANGLES: GLenum = 0x0004;
pub const GL_UNSIGNED_SHORT: GLenum = 0x1403;
pub const GL_TEXTURE_MAG_FILTER_OES: GLenum = 0x2600;

// GL_OES_EGL_image
pub const GL_TEXTURE_EXTERNAL_OES: GLenum = 0x8D65;
pub const GL_TEXTURE_BINDING_EXTERNAL_OES: GLenum = 0x8D67;

pub type PFNGLGENTEXTURESPROC = unsafe extern "system" fn(n: GLsizei, textures: *mut GLuint);
pub type PFNGLBINDTEXTUREPROC = unsafe extern "system" fn(target: GLenum, texture: GLuint);
pub type PFNGLTEXPARAMETERIPROC =
    unsafe extern "system" fn(target: GLenum, pname: GLenum, param: GLint);
pub type PFNGLTEXIMAGE2DPROC = unsafe extern "system" fn(
    target: GLenum,
    level: GLint,
    internalformat: GLint,
    width: GLsizei,
    height: GLsizei,
    border: GLint,
    format: GLenum,
    ty: GLenum,
    pixels: *const GLvoid,
);
pub type PFNGLDELETETEXTURESPROC = unsafe extern "system" fn(n: GLsizei, textures: *const GLuint);
pub type PFNGLENABLEPROC = unsafe extern "system" fn(cap: GLenum);
pub type PFNGLDISABLEPROC = unsafe extern "system" fn(cap: GLenum);
pub type PFNGLBLENDFUNCPROC = unsafe extern "system" fn(sfactor: GLenum, dfactor: GLenum);
pub type PFNGLBLENDEQUATIONPROC = unsafe extern "system" fn(mode: GLenum);
pub type PFNGLCLEARCOLORPROC =
    unsafe extern "system" fn(r: GLfloat, g: GLfloat, b: GLfloat, a: GLfloat);
pub type PFNGLCLEARPROC = unsafe extern "system" fn(mask: GLbitfield);
pub type PFNGLDRAWARRAYSPROC =
    unsafe extern "system" fn(mode: GLenum, first: GLint, count: GLsizei);
pub type PFNGLDRAWELEMENTSPROC =
    unsafe extern "system" fn(mode: GLenum, count: GLsizei, ty: GLenum, indices: *const GLvoid);
pub type PFNGLACTIVETEXTUREPROC = unsafe extern "system" fn(texture: GLenum);
pub type PFNGLVIEWPORTPROC =
    unsafe extern "system" fn(x: GLint, y: GLint, width: GLsizei, height: GLsizei);
pub type PFNGLFINISHPROC = unsafe extern "system" fn();
pub type PFNGLFLUSHPROC = unsafe extern "system" fn();
pub type PFNGLREADPIXELSPROC = unsafe extern "system" fn(
    x: GLint,
    y: GLint,
    width: GLsizei,
    height: GLsizei,
    format: GLenum,
    ty: GLenum,
    pixels: *mut GLvoid,
);

// OpenGL 2.0+ (for shaders)
pub type PFNGLCREATESHADERPROC = unsafe extern "system" fn(shaderType: GLenum) -> GLuint;
pub type PFNGLSHADERSOURCEPROC = unsafe extern "system" fn(
    shader: GLuint,
    count: GLsizei,
    string: *const *const c_char,
    length: *const GLint,
);
pub type PFNGLCOMPILESHADERPROC = unsafe extern "system" fn(shader: GLuint);
pub type PFNGLGETSHADERIVPROC =
    unsafe extern "system" fn(shader: GLuint, pname: GLenum, params: *mut GLint);
pub type PFNGLGETSHADERINFOLOGPROC = unsafe extern "system" fn(
    shader: GLuint,
    bufSize: GLsizei,
    length: *mut GLsizei,
    infoLog: *mut c_char,
);
pub type PFNGLCREATEPROGRAMPROC = unsafe extern "system" fn() -> GLuint;
pub type PFNGLATTACHSHADERPROC = unsafe extern "system" fn(program: GLuint, shader: GLuint);
pub type PFNGLLINKPROGRAMPROC = unsafe extern "system" fn(program: GLuint);
pub type PFNGLGETPROGRAMIVPROC =
    unsafe extern "system" fn(program: GLuint, pname: GLenum, params: *mut GLint);
pub type PFNGLGETPROGRAMINFOLOGPROC = unsafe extern "system" fn(
    program: GLuint,
    bufSize: GLsizei,
    length: *mut GLsizei,
    infoLog: *mut c_char,
);
pub type PFNGLUSEPROGRAMPROC = unsafe extern "system" fn(program: GLuint);
pub type PFNGLGETATTRIBLOCATIONPROC =
    unsafe extern "system" fn(program: GLuint, name: *const c_char) -> GLint;
pub type PFNGLGETUNIFORMLOCATIONPROC =
    unsafe extern "system" fn(program: GLuint, name: *const c_char) -> GLint;
pub type PFNGLENABLEVERTEXATTRIBARRAYPROC = unsafe extern "system" fn(index: GLuint);
pub type PFNGLVERTEXATTRIBPOINTERPROC = unsafe extern "system" fn(
    index: GLuint,
    size: GLint,
    ty: GLenum,
    normalized: GLboolean,
    stride: GLsizei,
    pointer: *const GLvoid,
);
pub type PFNGLUNIFORM1IPROC = unsafe extern "system" fn(location: GLint, v0: GLint);
pub type PFNGLDELETESHADERPROC = unsafe extern "system" fn(shader: GLuint);
pub type PFNGLDELETEPROGRAMPROC = unsafe extern "system" fn(program: GLuint);

// OpenGL ES 3.0 / ARB_framebuffer_object
pub type PFNGLGENFRAMEBUFFERSPROC =
    unsafe extern "system" fn(n: GLsizei, framebuffers: *mut GLuint);
pub type PFNGLBINDFRAMEBUFFERPROC = unsafe extern "system" fn(target: GLenum, framebuffer: GLuint);
pub type PFNGLFRAMEBUFFERTEXTURE2DPROC = unsafe extern "system" fn(
    target: GLenum,
    attachment: GLenum,
    textarget: GLenum,
    texture: GLuint,
    level: GLint,
);
pub type PFNGLCHECKFRAMEBUFFERSTATUSPROC = unsafe extern "system" fn(target: GLenum) -> GLenum;
pub type PFNGLDELETEFRAMEBUFFERSPROC =
    unsafe extern "system" fn(n: GLsizei, framebuffers: *const GLuint);
pub type PFNGLGENRENDERBUFFERSPROC =
    unsafe extern "system" fn(n: GLsizei, renderbuffers: *mut GLuint);
pub type PFNGLBINDRENDERBUFFERPROC =
    unsafe extern "system" fn(target: GLenum, renderbuffer: GLuint);
pub type PFNGLRENDERBUFFERSTORAGEPROC = unsafe extern "system" fn(
    target: GLenum,
    internalformat: GLenum,
    width: GLsizei,
    height: GLsizei,
);
pub type PFNGLFRAMEBUFFERRENDERBUFFERPROC = unsafe extern "system" fn(
    target: GLenum,
    attachment: GLenum,
    renderbuffertarget: GLenum,
    renderbuffer: GLuint,
);
pub type PFNGLDELETERENDERBUFFERSPROC =
    unsafe extern "system" fn(n: GLsizei, renderbuffers: *const GLuint);

// GL_OES_EGL_image_external
pub type PFNGLEGLIMAGETARGETTEXTURE2DOESPROC =
    unsafe extern "system" fn(target: GLenum, image: *mut c_void);

// VAO/VBO (GL 3.0+)
pub type PFNGLGENVERTEXARRAYSPROC = unsafe extern "system" fn(n: GLsizei, arrays: *mut GLuint);
pub type PFNGLBINDVERTEXARRAYPROC = unsafe extern "system" fn(array: GLuint);
pub type PFNGLGENBUFFERSPROC = unsafe extern "system" fn(n: GLsizei, buffers: *mut GLuint);
pub type PFNGLBINDBUFFERPROC = unsafe extern "system" fn(target: GLenum, buffer: GLuint);
pub type PFNGLBUFFERDATAPROC =
    unsafe extern "system" fn(target: GLenum, size: GLsizeiptr, data: *const GLvoid, usage: GLenum);
pub type PFNGLDELETEBUFFERSPROC = unsafe extern "system" fn(n: GLsizei, buffers: *const GLuint);
pub type PFNGLDELETEVERTEXARRAYSPROC = unsafe extern "system" fn(n: GLsizei, arrays: *const GLuint);

pub struct Gl {
    pub lib: *mut c_void,
    pub glGenTextures: PFNGLGENTEXTURESPROC,
    pub glBindTexture: PFNGLBINDTEXTUREPROC,
    pub glTexParameteri: PFNGLTEXPARAMETERIPROC,
    pub glTexImage2D: PFNGLTEXIMAGE2DPROC,
    pub glDeleteTextures: PFNGLDELETETEXTURESPROC,
    pub glEnable: PFNGLENABLEPROC,
    pub glDisable: PFNGLDISABLEPROC,
    pub glBlendFunc: PFNGLBLENDFUNCPROC,
    pub glBlendEquation: PFNGLBLENDEQUATIONPROC,
    pub glClearColor: PFNGLCLEARCOLORPROC,
    pub glClear: PFNGLCLEARPROC,
    pub glDrawArrays: PFNGLDRAWARRAYSPROC,
    pub glDrawElements: PFNGLDRAWELEMENTSPROC,
    pub glActiveTexture: PFNGLACTIVETEXTUREPROC,
    pub glViewport: PFNGLVIEWPORTPROC,
    pub glFinish: PFNGLFINISHPROC,
    pub glFlush: PFNGLFLUSHPROC,
    pub glReadPixels: PFNGLREADPIXELSPROC,
    // Shader support
    pub glCreateShader: PFNGLCREATESHADERPROC,
    pub glShaderSource: PFNGLSHADERSOURCEPROC,
    pub glCompileShader: PFNGLCOMPILESHADERPROC,
    pub glGetShaderiv: PFNGLGETSHADERIVPROC,
    pub glGetShaderInfoLog: PFNGLGETSHADERINFOLOGPROC,
    pub glCreateProgram: PFNGLCREATEPROGRAMPROC,
    pub glAttachShader: PFNGLATTACHSHADERPROC,
    pub glLinkProgram: PFNGLLINKPROGRAMPROC,
    pub glGetProgramiv: PFNGLGETPROGRAMIVPROC,
    pub glGetProgramInfoLog: PFNGLGETPROGRAMINFOLOGPROC,
    pub glUseProgram: PFNGLUSEPROGRAMPROC,
    pub glGetAttribLocation: PFNGLGETATTRIBLOCATIONPROC,
    pub glGetUniformLocation: PFNGLGETUNIFORMLOCATIONPROC,
    pub glEnableVertexAttribArray: PFNGLENABLEVERTEXATTRIBARRAYPROC,
    pub glVertexAttribPointer: PFNGLVERTEXATTRIBPOINTERPROC,
    pub glUniform1i: PFNGLUNIFORM1IPROC,
    pub glDeleteShader: PFNGLDELETESHADERPROC,
    pub glDeleteProgram: PFNGLDELETEPROGRAMPROC,
    // FBO
    pub glGenFramebuffers: PFNGLGENFRAMEBUFFERSPROC,
    pub glBindFramebuffer: PFNGLBINDFRAMEBUFFERPROC,
    pub glFramebufferTexture2D: PFNGLFRAMEBUFFERTEXTURE2DPROC,
    pub glCheckFramebufferStatus: PFNGLCHECKFRAMEBUFFERSTATUSPROC,
    pub glDeleteFramebuffers: PFNGLDELETEFRAMEBUFFERSPROC,
    pub glGenRenderbuffers: PFNGLGENRENDERBUFFERSPROC,
    pub glBindRenderbuffer: PFNGLBINDRENDERBUFFERPROC,
    pub glRenderbufferStorage: PFNGLRENDERBUFFERSTORAGEPROC,
    pub glFramebufferRenderbuffer: PFNGLFRAMEBUFFERRENDERBUFFERPROC,
    pub glDeleteRenderbuffers: PFNGLDELETERENDERBUFFERSPROC,
    // EGLImage target
    pub glEGLImageTargetTexture2DOES: PFNGLEGLIMAGETARGETTEXTURE2DOESPROC,
    // VAO/VBO (GL 3.0+)
    pub glGenVertexArrays: PFNGLGENVERTEXARRAYSPROC,
    pub glBindVertexArray: PFNGLBINDVERTEXARRAYPROC,
    pub glGenBuffers: PFNGLGENBUFFERSPROC,
    pub glBindBuffer: PFNGLBINDBUFFERPROC,
    pub glBufferData: PFNGLBUFFERDATAPROC,
    pub glDeleteBuffers: PFNGLDELETEBUFFERSPROC,
    pub glDeleteVertexArrays: PFNGLDELETEVERTEXARRAYSPROC,
}

unsafe impl Send for Gl {}
unsafe impl Sync for Gl {}

fn dlopen(name: &str) -> *mut c_void {
    let c_name = CString::new(name).unwrap();
    unsafe { libc::dlopen(c_name.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL) }
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

impl Gl {
    pub fn open() -> Option<Self> {
        let lib = dlopen("libGL.so.1");
        if lib.is_null() {
            eprintln!("[gl] Failed to load libGL.so.1");
            return None;
        }

        Some(Self {
            lib,
            glGenTextures: dlsym(lib, "glGenTextures")?,
            glBindTexture: dlsym(lib, "glBindTexture")?,
            glTexParameteri: dlsym(lib, "glTexParameteri")?,
            glTexImage2D: dlsym(lib, "glTexImage2D")?,
            glDeleteTextures: dlsym(lib, "glDeleteTextures")?,
            glEnable: dlsym(lib, "glEnable")?,
            glDisable: dlsym(lib, "glDisable")?,
            glBlendFunc: dlsym(lib, "glBlendFunc")?,
            glBlendEquation: dlsym(lib, "glBlendEquation")?,
            glClearColor: dlsym(lib, "glClearColor")?,
            glClear: dlsym(lib, "glClear")?,
            glDrawArrays: dlsym(lib, "glDrawArrays")?,
            glDrawElements: dlsym(lib, "glDrawElements")?,
            glActiveTexture: dlsym(lib, "glActiveTexture")?,
            glViewport: dlsym(lib, "glViewport")?,
            glFinish: dlsym(lib, "glFinish")?,
            glFlush: dlsym(lib, "glFlush")?,
            glReadPixels: dlsym(lib, "glReadPixels")?,
            glCreateShader: dlsym(lib, "glCreateShader")?,
            glShaderSource: dlsym(lib, "glShaderSource")?,
            glCompileShader: dlsym(lib, "glCompileShader")?,
            glGetShaderiv: dlsym(lib, "glGetShaderiv")?,
            glGetShaderInfoLog: dlsym(lib, "glGetShaderInfoLog")?,
            glCreateProgram: dlsym(lib, "glCreateProgram")?,
            glAttachShader: dlsym(lib, "glAttachShader")?,
            glLinkProgram: dlsym(lib, "glLinkProgram")?,
            glGetProgramiv: dlsym(lib, "glGetProgramiv")?,
            glGetProgramInfoLog: dlsym(lib, "glGetProgramInfoLog")?,
            glUseProgram: dlsym(lib, "glUseProgram")?,
            glGetAttribLocation: dlsym(lib, "glGetAttribLocation")?,
            glGetUniformLocation: dlsym(lib, "glGetUniformLocation")?,
            glEnableVertexAttribArray: dlsym(lib, "glEnableVertexAttribArray")?,
            glVertexAttribPointer: dlsym(lib, "glVertexAttribPointer")?,
            glUniform1i: dlsym(lib, "glUniform1i")?,
            glDeleteShader: dlsym(lib, "glDeleteShader")?,
            glDeleteProgram: dlsym(lib, "glDeleteProgram")?,
            glGenFramebuffers: dlsym(lib, "glGenFramebuffers")?,
            glBindFramebuffer: dlsym(lib, "glBindFramebuffer")?,
            glFramebufferTexture2D: dlsym(lib, "glFramebufferTexture2D")?,
            glCheckFramebufferStatus: dlsym(lib, "glCheckFramebufferStatus")?,
            glDeleteFramebuffers: dlsym(lib, "glDeleteFramebuffers")?,
            glGenRenderbuffers: dlsym(lib, "glGenRenderbuffers")?,
            glBindRenderbuffer: dlsym(lib, "glBindRenderbuffer")?,
            glRenderbufferStorage: dlsym(lib, "glRenderbufferStorage")?,
            glFramebufferRenderbuffer: dlsym(lib, "glFramebufferRenderbuffer")?,
            glDeleteRenderbuffers: dlsym(lib, "glDeleteRenderbuffers")?,
            glEGLImageTargetTexture2DOES: dlsym(lib, "glEGLImageTargetTexture2DOES")?,
            glGenVertexArrays: dlsym(lib, "glGenVertexArrays")?,
            glBindVertexArray: dlsym(lib, "glBindVertexArray")?,
            glGenBuffers: dlsym(lib, "glGenBuffers")?,
            glBindBuffer: dlsym(lib, "glBindBuffer")?,
            glBufferData: dlsym(lib, "glBufferData")?,
            glDeleteBuffers: dlsym(lib, "glDeleteBuffers")?,
            glDeleteVertexArrays: dlsym(lib, "glDeleteVertexArrays")?,
        })
    }
}

impl Drop for Gl {
    fn drop(&mut self) {
        if !self.lib.is_null() {
            unsafe { libc::dlclose(self.lib) };
        }
    }
}
