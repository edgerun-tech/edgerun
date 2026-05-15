use super::{Color4, GpuRect, GpuScene, RectMode};
#[cfg(feature = "fontdue-text")]
use super::{FontAtlas, TextQuad};
#[cfg(all(feature = "fontdue-text", feature = "tabler-svg-atlas"))]
use super::{IconQuad, tabler_svg_icon_atlas};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

const GL_COLOR_BUFFER_BIT: u32 = 0x0000_4000;
const GL_ARRAY_BUFFER: u32 = 0x8892;
const GL_DYNAMIC_DRAW: u32 = 0x88E8;
const GL_FLOAT: u32 = 0x1406;
const GL_FALSE: u8 = 0;
const GL_TRIANGLES: u32 = 0x0004;
const GL_VERTEX_SHADER: u32 = 0x8B31;
const GL_FRAGMENT_SHADER: u32 = 0x8B30;
const GL_COMPILE_STATUS: u32 = 0x8B81;
const GL_LINK_STATUS: u32 = 0x8B82;
const GL_BLEND: u32 = 0x0BE2;
const GL_SRC_ALPHA: u32 = 0x0302;
const GL_ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
#[cfg(feature = "fontdue-text")]
const GL_TEXTURE_2D: u32 = 0x0DE1;
#[cfg(feature = "fontdue-text")]
const GL_TEXTURE0: u32 = 0x84C0;
#[cfg(feature = "fontdue-text")]
const GL_RGBA: u32 = 0x1908;
#[cfg(feature = "fontdue-text")]
const GL_RGBA8: u32 = 0x8058;
#[cfg(feature = "fontdue-text")]
const GL_UNSIGNED_BYTE: u32 = 0x1401;
#[cfg(feature = "fontdue-text")]
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
#[cfg(feature = "fontdue-text")]
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
#[cfg(feature = "fontdue-text")]
const GL_TEXTURE_WRAP_S: u32 = 0x2802;
#[cfg(feature = "fontdue-text")]
const GL_TEXTURE_WRAP_T: u32 = 0x2803;
#[cfg(feature = "fontdue-text")]
const GL_LINEAR: i32 = 0x2601;
#[cfg(feature = "fontdue-text")]
const GL_CLAMP_TO_EDGE: i32 = 0x812F;
#[cfg(feature = "fontdue-text")]
const GL_UNPACK_ALIGNMENT: u32 = 0x0CF5;

#[link(name = "GL")]
unsafe extern "C" {
    fn glViewport(x: c_int, y: c_int, width: c_int, height: c_int);
    fn glClearColor(r: f32, g: f32, b: f32, a: f32);
    fn glClear(mask: u32);
    fn glEnable(cap: u32);
    fn glBlendFunc(sfactor: u32, dfactor: u32);
    fn glCreateShader(shader_type: u32) -> u32;
    fn glShaderSource(
        shader: u32,
        count: c_int,
        string: *const *const c_char,
        length: *const c_int,
    );
    fn glCompileShader(shader: u32);
    fn glGetShaderiv(shader: u32, pname: u32, params: *mut c_int);
    fn glGetShaderInfoLog(shader: u32, buf_size: c_int, length: *mut c_int, info_log: *mut c_char);
    fn glDeleteShader(shader: u32);
    fn glCreateProgram() -> u32;
    fn glAttachShader(program: u32, shader: u32);
    fn glLinkProgram(program: u32);
    fn glGetProgramiv(program: u32, pname: u32, params: *mut c_int);
    fn glGetProgramInfoLog(
        program: u32,
        buf_size: c_int,
        length: *mut c_int,
        info_log: *mut c_char,
    );
    fn glUseProgram(program: u32);
    fn glGetUniformLocation(program: u32, name: *const c_char) -> c_int;
    fn glUniform2f(location: c_int, v0: f32, v1: f32);
    fn glUniform4f(location: c_int, v0: f32, v1: f32, v2: f32, v3: f32);
    fn glUniform1f(location: c_int, v0: f32);
    fn glUniform1i(location: c_int, v0: c_int);
    fn glGenVertexArrays(n: c_int, arrays: *mut u32);
    fn glBindVertexArray(array: u32);
    fn glGenBuffers(n: c_int, buffers: *mut u32);
    fn glBindBuffer(target: u32, buffer: u32);
    fn glBufferData(target: u32, size: isize, data: *const c_void, usage: u32);
    fn glEnableVertexAttribArray(index: u32);
    fn glVertexAttribPointer(
        index: u32,
        size: c_int,
        ty: u32,
        normalized: u8,
        stride: c_int,
        pointer: *const c_void,
    );
    fn glDrawArrays(mode: u32, first: c_int, count: c_int);
    fn glDeleteBuffers(n: c_int, buffers: *const u32);
    fn glDeleteVertexArrays(n: c_int, arrays: *const u32);
    fn glDeleteProgram(program: u32);
    #[cfg(feature = "fontdue-text")]
    fn glGenTextures(n: c_int, textures: *mut u32);
    #[cfg(feature = "fontdue-text")]
    fn glBindTexture(target: u32, texture: u32);
    #[cfg(feature = "fontdue-text")]
    fn glTexParameteri(target: u32, pname: u32, param: c_int);
    #[cfg(feature = "fontdue-text")]
    fn glTexImage2D(
        target: u32,
        level: c_int,
        internalformat: c_int,
        width: c_int,
        height: c_int,
        border: c_int,
        format: u32,
        ty: u32,
        pixels: *const c_void,
    );
    #[cfg(feature = "fontdue-text")]
    fn glActiveTexture(texture: u32);
    #[cfg(feature = "fontdue-text")]
    fn glPixelStorei(pname: u32, param: c_int);
    #[cfg(feature = "fontdue-text")]
    fn glDeleteTextures(n: c_int, textures: *const u32);
}

const VERT: &str = r#"#version 330 core
layout(location = 0) in vec2 a_pos;
uniform vec2 u_screen;
uniform vec4 u_rect;
out vec2 v_local;
out vec2 v_size;
void main() {
    vec2 px = u_rect.xy + a_pos * u_rect.zw;
    vec2 ndc = vec2(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
    v_local = a_pos * u_rect.zw;
    v_size = u_rect.zw;
}
"#;

#[cfg(feature = "fontdue-text")]
const TEXT_VERT: &str = r#"#version 330 core
layout(location = 0) in vec4 a_data;
uniform vec2 u_screen;
out vec2 v_uv;
void main() {
    vec2 px = a_data.xy;
    vec2 ndc = vec2(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
    v_uv = a_data.zw;
}
"#;

#[cfg(feature = "fontdue-text")]
const TEXT_FRAG: &str = r#"#version 330 core
in vec2 v_uv;
out vec4 out_color;
uniform sampler2D u_tex;
uniform vec4 u_color;
void main() {
    float a = texture(u_tex, v_uv).a;
    out_color = vec4(u_color.rgb, u_color.a * a);
}
"#;

const FRAG: &str = r#"#version 330 core
in vec2 v_local;
in vec2 v_size;
out vec4 out_color;
uniform vec4 u_color;
uniform float u_radius;
uniform float u_shadow;
uniform int u_mode;
float rounded_box(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + vec2(r);
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}
void main() {
    vec2 p = v_local - v_size * 0.5;
    float d = rounded_box(p, v_size * 0.5, u_radius);
    float aa = max(fwidth(d), 0.75);
    float alpha = 1.0 - smoothstep(0.0, aa, d);
    if (u_mode == 1) {
        float sd = rounded_box(p - vec2(0.0, -u_shadow * 0.18), v_size * 0.5, u_radius + u_shadow * 0.35);
        float blur = max(u_shadow, 1.0);
        alpha = 1.0 - smoothstep(-blur, blur, sd);
        out_color = vec4(u_color.rgb, u_color.a * alpha * 0.28);
    } else if (u_mode == 2) {
        float inner = rounded_box(p, v_size * 0.5 - vec2(1.25), max(u_radius - 1.25, 0.0));
        float border = (1.0 - smoothstep(0.0, aa, d)) * smoothstep(0.0, aa, inner);
        out_color = vec4(u_color.rgb, u_color.a * border);
    } else {
        out_color = vec4(u_color.rgb, u_color.a * alpha);
    }
}
"#;

pub struct GlRenderer {
    program: u32,
    vao: u32,
    vbo: u32,
    u_screen: c_int,
    u_rect: c_int,
    u_color: c_int,
    u_radius: c_int,
    u_mode: c_int,
    u_shadow: c_int,
    #[cfg(all(feature = "fontdue-text", feature = "tabler-svg-atlas"))]
    icons: Option<IconRenderer>,
    #[cfg(feature = "fontdue-text")]
    text: Option<TextRenderer>,
}

impl GlRenderer {
    /// Create a renderer for the current OpenGL context.
    ///
    /// The caller must create and make current the GL context before calling
    /// this constructor.
    pub unsafe fn new_current_context() -> Result<Self, String> {
        let program = unsafe { glCreateProgram() };
        let vs = compile_shader(GL_VERTEX_SHADER, VERT)?;
        let fs = compile_shader(GL_FRAGMENT_SHADER, FRAG)?;
        unsafe {
            glAttachShader(program, vs);
            glAttachShader(program, fs);
            glLinkProgram(program);
            glDeleteShader(vs);
            glDeleteShader(fs);
        }
        check_program(program)?;

        let mut vao = 0;
        let mut vbo = 0;
        let verts: [f32; 12] = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        unsafe {
            glGenVertexArrays(1, &mut vao);
            glBindVertexArray(vao);
            glGenBuffers(1, &mut vbo);
            glBindBuffer(GL_ARRAY_BUFFER, vbo);
            glBufferData(
                GL_ARRAY_BUFFER,
                (verts.len() * 4) as isize,
                verts.as_ptr().cast(),
                GL_DYNAMIC_DRAW,
            );
            glEnableVertexAttribArray(0);
            glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, 2 * 4, ptr::null());
        }

        Ok(Self {
            program,
            vao,
            vbo,
            u_screen: uniform(program, "u_screen"),
            u_rect: uniform(program, "u_rect"),
            u_color: uniform(program, "u_color"),
            u_radius: uniform(program, "u_radius"),
            u_mode: uniform(program, "u_mode"),
            u_shadow: uniform(program, "u_shadow"),
            #[cfg(all(feature = "fontdue-text", feature = "tabler-svg-atlas"))]
            icons: Some(IconRenderer::new()?),
            #[cfg(feature = "fontdue-text")]
            text: None,
        })
    }

    #[cfg(feature = "fontdue-text")]
    pub unsafe fn new_current_context_with_font(atlas: &FontAtlas) -> Result<Self, String> {
        let mut renderer = unsafe { Self::new_current_context()? };
        renderer.text = Some(TextRenderer::new(atlas)?);
        Ok(renderer)
    }

    pub fn render(&self, width: i32, height: i32, scene: &GpuScene) {
        let clear = scene.clear;
        unsafe {
            glViewport(0, 0, width, height);
            glClearColor(clear.r, clear.g, clear.b, clear.a);
            glClear(GL_COLOR_BUFFER_BIT);
            glEnable(GL_BLEND);
            glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
            glUseProgram(self.program);
            glBindVertexArray(self.vao);
            glUniform2f(self.u_screen, width as f32, height as f32);
        }
        for rect in scene.rects() {
            self.draw_rect(*rect);
        }
        #[cfg(all(feature = "fontdue-text", feature = "tabler-svg-atlas"))]
        if let Some(icons) = &self.icons {
            icons.render(width, height, scene.icon_quads());
        }
        #[cfg(feature = "fontdue-text")]
        if let Some(text) = &self.text {
            text.render(width, height, scene.text_quads());
        }
    }

    fn draw_rect(&self, rect: GpuRect) {
        let mode = match rect.mode {
            RectMode::Fill => 0,
            RectMode::Shadow => 1,
            RectMode::Border => 2,
        };
        let Color4 { r, g, b, a } = rect.color;
        unsafe {
            glUniform4f(self.u_rect, rect.x, rect.y, rect.w, rect.h);
            glUniform4f(self.u_color, r, g, b, a);
            glUniform1f(self.u_radius, rect.radius);
            glUniform1i(self.u_mode, mode);
            glUniform1f(self.u_shadow, rect.shadow);
            glDrawArrays(GL_TRIANGLES, 0, 6);
        }
    }
}

#[cfg(all(feature = "fontdue-text", feature = "tabler-svg-atlas"))]
struct IconRenderer {
    program: u32,
    vao: u32,
    vbo: u32,
    texture: u32,
    u_screen: c_int,
    u_color: c_int,
    u_tex: c_int,
}

#[cfg(all(feature = "fontdue-text", feature = "tabler-svg-atlas"))]
impl IconRenderer {
    fn new() -> Result<Self, String> {
        let atlas = tabler_svg_icon_atlas();
        let program = unsafe { glCreateProgram() };
        let vs = compile_shader(GL_VERTEX_SHADER, TEXT_VERT)?;
        let fs = compile_shader(GL_FRAGMENT_SHADER, TEXT_FRAG)?;
        unsafe {
            glAttachShader(program, vs);
            glAttachShader(program, fs);
            glLinkProgram(program);
            glDeleteShader(vs);
            glDeleteShader(fs);
        }
        check_program(program)?;

        let mut vao = 0;
        let mut vbo = 0;
        unsafe {
            glGenVertexArrays(1, &mut vao);
            glBindVertexArray(vao);
            glGenBuffers(1, &mut vbo);
            glBindBuffer(GL_ARRAY_BUFFER, vbo);
            glEnableVertexAttribArray(0);
            glVertexAttribPointer(0, 4, GL_FLOAT, GL_FALSE, 4 * 4, ptr::null());
        }

        let mut texture = 0;
        let mut rgba = Vec::with_capacity(atlas.alpha.len() * 4);
        for alpha in atlas.alpha {
            rgba.extend_from_slice(&[255, 255, 255, *alpha]);
        }
        unsafe {
            glGenTextures(1, &mut texture);
            glBindTexture(GL_TEXTURE_2D, texture);
            glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
            glTexImage2D(
                GL_TEXTURE_2D,
                0,
                GL_RGBA8 as i32,
                atlas.width as i32,
                atlas.height as i32,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                rgba.as_ptr().cast(),
            );
        }

        Ok(Self {
            program,
            vao,
            vbo,
            texture,
            u_screen: uniform(program, "u_screen"),
            u_color: uniform(program, "u_color"),
            u_tex: uniform(program, "u_tex"),
        })
    }

    fn render(&self, width: i32, height: i32, quads: &[IconQuad]) {
        unsafe {
            glUseProgram(self.program);
            glBindVertexArray(self.vao);
            glActiveTexture(GL_TEXTURE0);
            glBindTexture(GL_TEXTURE_2D, self.texture);
            glUniform1i(self.u_tex, 0);
            glUniform2f(self.u_screen, width as f32, height as f32);
        }
        for quad in quads {
            self.draw_quad(*quad);
        }
    }

    fn draw_quad(&self, q: IconQuad) {
        let verts: [f32; 24] = [
            q.x,
            q.y,
            q.u0,
            q.v0,
            q.x + q.w,
            q.y,
            q.u1,
            q.v0,
            q.x + q.w,
            q.y + q.h,
            q.u1,
            q.v1,
            q.x,
            q.y,
            q.u0,
            q.v0,
            q.x + q.w,
            q.y + q.h,
            q.u1,
            q.v1,
            q.x,
            q.y + q.h,
            q.u0,
            q.v1,
        ];
        let Color4 { r, g, b, a } = q.color;
        unsafe {
            glUniform4f(self.u_color, r, g, b, a);
            glBindBuffer(GL_ARRAY_BUFFER, self.vbo);
            glBufferData(
                GL_ARRAY_BUFFER,
                (verts.len() * 4) as isize,
                verts.as_ptr().cast(),
                GL_DYNAMIC_DRAW,
            );
            glDrawArrays(GL_TRIANGLES, 0, 6);
        }
    }
}

#[cfg(all(feature = "fontdue-text", feature = "tabler-svg-atlas"))]
impl Drop for IconRenderer {
    fn drop(&mut self) {
        unsafe {
            glDeleteTextures(1, &self.texture);
            glDeleteBuffers(1, &self.vbo);
            glDeleteVertexArrays(1, &self.vao);
            glDeleteProgram(self.program);
        }
    }
}

#[cfg(feature = "fontdue-text")]
struct TextRenderer {
    program: u32,
    vao: u32,
    vbo: u32,
    texture: u32,
    u_screen: c_int,
    u_color: c_int,
    u_tex: c_int,
}

#[cfg(feature = "fontdue-text")]
impl TextRenderer {
    fn new(atlas: &FontAtlas) -> Result<Self, String> {
        let program = unsafe { glCreateProgram() };
        let vs = compile_shader(GL_VERTEX_SHADER, TEXT_VERT)?;
        let fs = compile_shader(GL_FRAGMENT_SHADER, TEXT_FRAG)?;
        unsafe {
            glAttachShader(program, vs);
            glAttachShader(program, fs);
            glLinkProgram(program);
            glDeleteShader(vs);
            glDeleteShader(fs);
        }
        check_program(program)?;

        let mut vao = 0;
        let mut vbo = 0;
        unsafe {
            glGenVertexArrays(1, &mut vao);
            glBindVertexArray(vao);
            glGenBuffers(1, &mut vbo);
            glBindBuffer(GL_ARRAY_BUFFER, vbo);
            glEnableVertexAttribArray(0);
            glVertexAttribPointer(0, 4, GL_FLOAT, GL_FALSE, 4 * 4, ptr::null());
        }

        let mut texture = 0;
        let mut rgba = Vec::with_capacity(atlas.alpha.len() * 4);
        for alpha in &atlas.alpha {
            rgba.extend_from_slice(&[255, 255, 255, *alpha]);
        }
        unsafe {
            glGenTextures(1, &mut texture);
            glBindTexture(GL_TEXTURE_2D, texture);
            glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
            glTexImage2D(
                GL_TEXTURE_2D,
                0,
                GL_RGBA8 as i32,
                atlas.width as i32,
                atlas.height as i32,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                rgba.as_ptr().cast(),
            );
        }

        Ok(Self {
            program,
            vao,
            vbo,
            texture,
            u_screen: uniform(program, "u_screen"),
            u_color: uniform(program, "u_color"),
            u_tex: uniform(program, "u_tex"),
        })
    }

    fn render(&self, width: i32, height: i32, quads: &[TextQuad]) {
        unsafe {
            glUseProgram(self.program);
            glBindVertexArray(self.vao);
            glActiveTexture(GL_TEXTURE0);
            glBindTexture(GL_TEXTURE_2D, self.texture);
            glUniform1i(self.u_tex, 0);
            glUniform2f(self.u_screen, width as f32, height as f32);
        }
        for quad in quads {
            self.draw_quad(*quad);
        }
    }

    fn draw_quad(&self, q: TextQuad) {
        let verts: [f32; 24] = [
            q.x,
            q.y,
            q.u0,
            q.v0,
            q.x + q.w,
            q.y,
            q.u1,
            q.v0,
            q.x + q.w,
            q.y + q.h,
            q.u1,
            q.v1,
            q.x,
            q.y,
            q.u0,
            q.v0,
            q.x + q.w,
            q.y + q.h,
            q.u1,
            q.v1,
            q.x,
            q.y + q.h,
            q.u0,
            q.v1,
        ];
        let Color4 { r, g, b, a } = q.color;
        unsafe {
            glUniform4f(self.u_color, r, g, b, a);
            glBindBuffer(GL_ARRAY_BUFFER, self.vbo);
            glBufferData(
                GL_ARRAY_BUFFER,
                (verts.len() * 4) as isize,
                verts.as_ptr().cast(),
                GL_DYNAMIC_DRAW,
            );
            glDrawArrays(GL_TRIANGLES, 0, 6);
        }
    }
}

#[cfg(feature = "fontdue-text")]
impl Drop for TextRenderer {
    fn drop(&mut self) {
        unsafe {
            glDeleteTextures(1, &self.texture);
            glDeleteBuffers(1, &self.vbo);
            glDeleteVertexArrays(1, &self.vao);
            glDeleteProgram(self.program);
        }
    }
}

impl Drop for GlRenderer {
    fn drop(&mut self) {
        unsafe {
            glDeleteBuffers(1, &self.vbo);
            glDeleteVertexArrays(1, &self.vao);
            glDeleteProgram(self.program);
        }
    }
}

fn compile_shader(kind: u32, source: &str) -> Result<u32, String> {
    let shader = unsafe { glCreateShader(kind) };
    let c_src = CString::new(source).map_err(|error| error.to_string())?;
    let ptr = c_src.as_ptr();
    unsafe {
        glShaderSource(shader, 1, &ptr, ptr::null());
        glCompileShader(shader);
    }
    let mut ok = 0;
    unsafe {
        glGetShaderiv(shader, GL_COMPILE_STATUS, &mut ok);
    }
    if ok == 0 {
        let log = shader_log(shader);
        unsafe {
            glDeleteShader(shader);
        }
        Err(log)
    } else {
        Ok(shader)
    }
}

fn check_program(program: u32) -> Result<(), String> {
    let mut ok = 0;
    unsafe {
        glGetProgramiv(program, GL_LINK_STATUS, &mut ok);
    }
    if ok == 0 {
        Err(program_log(program))
    } else {
        Ok(())
    }
}

fn shader_log(shader: u32) -> String {
    let mut buf = vec![0i8; 2048];
    let mut len = 0;
    unsafe {
        glGetShaderInfoLog(shader, buf.len() as i32, &mut len, buf.as_mut_ptr());
        CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
    }
}

fn program_log(program: u32) -> String {
    let mut buf = vec![0i8; 2048];
    let mut len = 0;
    unsafe {
        glGetProgramInfoLog(program, buf.len() as i32, &mut len, buf.as_mut_ptr());
        CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
    }
}

fn uniform(program: u32, name: &str) -> i32 {
    let Ok(c) = CString::new(name) else {
        return -1;
    };
    unsafe { glGetUniformLocation(program, c.as_ptr()) }
}
