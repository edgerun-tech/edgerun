//! GPU UI preview with analytic rounded shapes + font/icon atlas textures.
//!
//! This is still a preview binary. SDL creates the window/GL context only; UI
//! rendering is OpenGL: rounded rects are shader SDFs, text/icons are atlas
//! texture quads.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::time::{Duration, Instant};

#[cfg(feature = "fontdue-text")]
use edgerun_ui_core::font::FontFace;
#[cfg(feature = "fontdue-text")]
use edgerun_ui_core::tabler_font_generated::{tabler_icon, TABLER_ICON_FONT_HINT};

const SDL_INIT_VIDEO: u32 = 0x0000_0020;
const SDL_WINDOWPOS_CENTERED: c_int = 0x2fff_0000u32 as c_int;
const SDL_WINDOW_OPENGL: u32 = 0x0000_0002;
const SDL_WINDOW_SHOWN: u32 = 0x0000_0004;
const SDL_WINDOW_RESIZABLE: u32 = 0x0000_0020;
const SDL_QUIT: u32 = 0x100;
const SDL_WINDOWEVENT: u32 = 0x200;
const SDL_WINDOWEVENT_RESIZED: u8 = 0x05;
const SDL_KEYDOWN: u32 = 0x300;
const SDL_MOUSEBUTTONDOWN: u32 = 0x401;
const SDL_GL_CONTEXT_MAJOR_VERSION: c_int = 17;
const SDL_GL_CONTEXT_MINOR_VERSION: c_int = 18;
const SDL_GL_CONTEXT_PROFILE_MASK: c_int = 21;
const SDL_GL_CONTEXT_PROFILE_CORE: c_int = 0x0001;
const SDL_GL_DOUBLEBUFFER: c_int = 5;

const GL_COLOR_BUFFER_BIT: u32 = 0x00004000;
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
const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_TEXTURE0: u32 = 0x84C0;
const GL_RED: u32 = 0x1903;
const GL_R8: u32 = 0x8229;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_WRAP_S: u32 = 0x2802;
const GL_TEXTURE_WRAP_T: u32 = 0x2803;
const GL_LINEAR: i32 = 0x2601;
const GL_CLAMP_TO_EDGE: i32 = 0x812F;
const GL_UNPACK_ALIGNMENT: u32 = 0x0CF5;

#[repr(C)]
struct SDL_Window(c_void);
type SDL_GLContext = *mut c_void;

#[repr(C)]
struct SdlEvent {
    data: [u8; 56],
}

impl SdlEvent {
    fn event_type(&self) -> u32 { u32::from_ne_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]) }
    fn window_event(&self) -> u8 { self.data[8] }
    fn data1(&self) -> i32 { i32::from_ne_bytes([self.data[16], self.data[17], self.data[18], self.data[19]]) }
    fn data2(&self) -> i32 { i32::from_ne_bytes([self.data[20], self.data[21], self.data[22], self.data[23]]) }
    fn mouse_x(&self) -> i32 { i32::from_ne_bytes([self.data[20], self.data[21], self.data[22], self.data[23]]) }
    fn mouse_y(&self) -> i32 { i32::from_ne_bytes([self.data[24], self.data[25], self.data[26], self.data[27]]) }
    fn key_sym(&self) -> i32 { i32::from_ne_bytes([self.data[20], self.data[21], self.data[22], self.data[23]]) }
}

#[link(name = "SDL2")]
unsafe extern "C" {
    fn SDL_Init(flags: u32) -> c_int;
    fn SDL_Quit();
    fn SDL_GetError() -> *const c_char;
    fn SDL_GL_SetAttribute(attr: c_int, value: c_int) -> c_int;
    fn SDL_CreateWindow(title: *const c_char, x: c_int, y: c_int, w: c_int, h: c_int, flags: u32) -> *mut SDL_Window;
    fn SDL_DestroyWindow(window: *mut SDL_Window);
    fn SDL_GL_CreateContext(window: *mut SDL_Window) -> SDL_GLContext;
    fn SDL_GL_DeleteContext(context: SDL_GLContext);
    fn SDL_GL_SetSwapInterval(interval: c_int) -> c_int;
    fn SDL_GL_SwapWindow(window: *mut SDL_Window);
    fn SDL_PollEvent(event: *mut SdlEvent) -> c_int;
    fn SDL_Delay(ms: u32);
}

#[link(name = "GL")]
unsafe extern "C" {
    fn glViewport(x: c_int, y: c_int, width: c_int, height: c_int);
    fn glClearColor(r: f32, g: f32, b: f32, a: f32);
    fn glClear(mask: u32);
    fn glEnable(cap: u32);
    fn glBlendFunc(sfactor: u32, dfactor: u32);
    fn glCreateShader(shader_type: u32) -> u32;
    fn glShaderSource(shader: u32, count: c_int, string: *const *const c_char, length: *const c_int);
    fn glCompileShader(shader: u32);
    fn glGetShaderiv(shader: u32, pname: u32, params: *mut c_int);
    fn glGetShaderInfoLog(shader: u32, buf_size: c_int, length: *mut c_int, info_log: *mut c_char);
    fn glDeleteShader(shader: u32);
    fn glCreateProgram() -> u32;
    fn glAttachShader(program: u32, shader: u32);
    fn glLinkProgram(program: u32);
    fn glGetProgramiv(program: u32, pname: u32, params: *mut c_int);
    fn glGetProgramInfoLog(program: u32, buf_size: c_int, length: *mut c_int, info_log: *mut c_char);
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
    fn glVertexAttribPointer(index: u32, size: c_int, ty: u32, normalized: u8, stride: c_int, pointer: *const c_void);
    fn glDrawArrays(mode: u32, first: c_int, count: c_int);
    fn glDeleteBuffers(n: c_int, buffers: *const u32);
    fn glDeleteVertexArrays(n: c_int, arrays: *const u32);
    fn glDeleteProgram(program: u32);
    fn glGenTextures(n: c_int, textures: *mut u32);
    fn glBindTexture(target: u32, texture: u32);
    fn glTexParameteri(target: u32, pname: u32, param: c_int);
    fn glTexImage2D(target: u32, level: c_int, internalformat: c_int, width: c_int, height: c_int, border: c_int, format: u32, ty: u32, pixels: *const c_void);
    fn glActiveTexture(texture: u32);
    fn glPixelStorei(pname: u32, param: c_int);
    fn glDeleteTextures(n: c_int, textures: *const u32);
}

const SHAPE_VERT: &str = r#"#version 330 core
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

const SHAPE_FRAG: &str = r#"#version 330 core
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
        out_color = vec4(u_color.rgb, u_color.a * alpha * 0.22);
    } else if (u_mode == 2) {
        float inner = rounded_box(p, v_size * 0.5 - vec2(1.2), max(u_radius - 1.2, 0.0));
        float border = (1.0 - smoothstep(0.0, aa, d)) * smoothstep(0.0, aa, inner);
        out_color = vec4(u_color.rgb, u_color.a * border);
    } else {
        out_color = vec4(u_color.rgb, u_color.a * alpha);
    }
}
"#;

const TEXT_VERT: &str = r#"#version 330 core
layout(location = 0) in vec4 a_data; // x,y,u,v
uniform vec2 u_screen;
out vec2 v_uv;
void main() {
    vec2 px = a_data.xy;
    vec2 ndc = vec2(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
    v_uv = a_data.zw;
}
"#;

const TEXT_FRAG: &str = r#"#version 330 core
in vec2 v_uv;
out vec4 out_color;
uniform sampler2D u_tex;
uniform vec4 u_color;
void main() {
    float a = texture(u_tex, v_uv).r;
    out_color = vec4(u_color.rgb, u_color.a * a);
}
"#;

#[derive(Clone, Copy)]
struct Color4([f32; 4]);

const BG: Color4 = Color4([0.008, 0.024, 0.090, 1.0]);
const PANEL: Color4 = Color4([0.059, 0.090, 0.165, 0.94]);
const PANEL_2: Color4 = Color4([0.118, 0.161, 0.231, 0.90]);
const BORDER: Color4 = Color4([0.200, 0.255, 0.333, 0.58]);
const TEXT: Color4 = Color4([0.973, 0.980, 0.988, 1.0]);
const MUTED: Color4 = Color4([0.580, 0.640, 0.720, 1.0]);
const EMERALD: Color4 = Color4([0.314, 0.980, 0.482, 1.0]);
const CYAN: Color4 = Color4([0.545, 0.914, 0.992, 1.0]);
const PALETTE: [Color4; 7] = [
    Color4([0.055, 0.624, 0.820, 1.0]), // sky/cyan
    Color4([0.235, 0.510, 0.965, 1.0]), // blue
    Color4([0.545, 0.361, 0.965, 1.0]), // violet
    Color4([0.925, 0.282, 0.600, 1.0]), // pink
    Color4([0.059, 0.725, 0.506, 1.0]), // emerald
    Color4([0.918, 0.635, 0.071, 1.0]), // amber
    Color4([0.886, 0.114, 0.318, 1.0]), // rose
];

#[derive(Clone, Copy)]
struct Rect { x: f32, y: f32, w: f32, h: f32 }

#[derive(Clone, Copy)]
struct Shape { rect: Rect, radius: f32, color: Color4, mode: i32, shadow: f32 }

struct ShapeRenderer { program: u32, vao: u32, vbo: u32, u_screen: c_int, u_rect: c_int, u_color: c_int, u_radius: c_int, u_mode: c_int, u_shadow: c_int }

impl ShapeRenderer {
    fn new() -> Result<Self, String> {
        let program = link_program(SHAPE_VERT, SHAPE_FRAG)?;
        let (vao, vbo) = unit_quad_vao(2)?;
        Ok(Self {
            program, vao, vbo,
            u_screen: uniform(program, "u_screen"),
            u_rect: uniform(program, "u_rect"),
            u_color: uniform(program, "u_color"),
            u_radius: uniform(program, "u_radius"),
            u_mode: uniform(program, "u_mode"),
            u_shadow: uniform(program, "u_shadow"),
        })
    }
    fn begin(&self, width: i32, height: i32) { unsafe { glUseProgram(self.program); glBindVertexArray(self.vao); glUniform2f(self.u_screen, width as f32, height as f32); } }
    fn draw(&self, s: Shape) {
        unsafe {
            glUniform4f(self.u_rect, s.rect.x, s.rect.y, s.rect.w, s.rect.h);
            glUniform4f(self.u_color, s.color.0[0], s.color.0[1], s.color.0[2], s.color.0[3]);
            glUniform1f(self.u_radius, s.radius);
            glUniform1i(self.u_mode, s.mode);
            glUniform1f(self.u_shadow, s.shadow);
            glDrawArrays(GL_TRIANGLES, 0, 6);
        }
    }
    fn card(&self, rect: Rect) {
        self.draw(Shape { rect: Rect { y: rect.y + 3.0, ..rect }, radius: 18.0, color: Color4([0.0,0.0,0.0,0.18]), mode: 1, shadow: 8.0 });
        self.draw(Shape { rect, radius: 18.0, color: PANEL, mode: 0, shadow: 0.0 });
        self.draw(Shape { rect, radius: 18.0, color: BORDER, mode: 2, shadow: 0.0 });
    }
    fn button(&self, rect: Rect, accent: Color4, active: bool) {
        let radius = 11.0; // reduced from pill radius
        if active { self.draw(Shape { rect: Rect { y: rect.y + 2.0, ..rect }, radius, color: Color4([accent.0[0], accent.0[1], accent.0[2], 0.16]), mode: 1, shadow: 5.0 }); }
        self.draw(Shape { rect, radius, color: if active { accent } else { PANEL_2 }, mode: 0, shadow: 0.0 });
        self.draw(Shape { rect, radius, color: if active { Color4([accent.0[0], accent.0[1], accent.0[2], 0.9]) } else { BORDER }, mode: 2, shadow: 0.0 });
    }
    fn swatch(&self, rect: Rect, color: Color4, selected: bool) {
        self.draw(Shape { rect, radius: 9.0, color, mode: 0, shadow: 0.0 });
        if selected { self.draw(Shape { rect: Rect { x: rect.x - 2.0, y: rect.y - 2.0, w: rect.w + 4.0, h: rect.h + 4.0 }, radius: 11.0, color: TEXT, mode: 2, shadow: 0.0 }); }
    }
    fn progress(&self, rect: Rect, value: f32, accent: Color4) {
        self.draw(Shape { rect, radius: rect.h * 0.5, color: PANEL_2, mode: 0, shadow: 0.0 });
        self.draw(Shape { rect: Rect { w: rect.w * value.clamp(0.0, 1.0), ..rect }, radius: rect.h * 0.5, color: accent, mode: 0, shadow: 0.0 });
    }
}

impl Drop for ShapeRenderer { fn drop(&mut self) { unsafe { glDeleteBuffers(1, &self.vbo); glDeleteVertexArrays(1, &self.vao); glDeleteProgram(self.program); } } }

#[cfg(feature = "fontdue-text")]
#[derive(Clone, Copy)]
struct Glyph { uv: [f32; 4], size: [f32; 2], bearing: [f32; 2], advance: f32 }

#[cfg(feature = "fontdue-text")]
struct Atlas { tex: u32, glyphs: HashMap<char, Glyph>, w: u32, h: u32 }

#[cfg(feature = "fontdue-text")]
impl Atlas {
    fn build(font: &FontFace, chars: &[char], px: f32) -> Self {
        let w = 1024u32;
        let h = 1024u32;
        let mut bitmap = vec![0u8; (w*h) as usize];
        let mut glyphs = HashMap::new();
        let mut x = 2u32;
        let mut y = 2u32;
        let mut row_h = 0u32;
        for &ch in chars {
            let (m, data) = font.rasterize(ch, px);
            if m.width == 0 || m.height == 0 { glyphs.insert(ch, Glyph { uv:[0.0;4], size:[0.0,0.0], bearing:[m.xmin as f32, m.ymin as f32], advance:m.advance_width }); continue; }
            if x + m.width as u32 + 2 >= w { x = 2; y += row_h + 2; row_h = 0; }
            if y + m.height as u32 + 2 >= h { break; }
            for gy in 0..m.height as u32 { for gx in 0..m.width as u32 { bitmap[((y+gy)*w + x+gx) as usize] = data[(gy*m.width as u32 + gx) as usize]; } }
            glyphs.insert(ch, Glyph { uv:[x as f32/w as f32, y as f32/h as f32, (x+m.width as u32) as f32/w as f32, (y+m.height as u32) as f32/h as f32], size:[m.width as f32, m.height as f32], bearing:[m.xmin as f32, m.ymin as f32], advance:m.advance_width });
            x += m.width as u32 + 2;
            row_h = row_h.max(m.height as u32);
        }
        let mut tex = 0;
        unsafe { glGenTextures(1, &mut tex); glBindTexture(GL_TEXTURE_2D, tex); glPixelStorei(GL_UNPACK_ALIGNMENT, 1); glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR); glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR); glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE); glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE); glTexImage2D(GL_TEXTURE_2D, 0, GL_R8 as i32, w as i32, h as i32, 0, GL_RED, GL_UNSIGNED_BYTE, bitmap.as_ptr() as *const c_void); }
        Self { tex, glyphs, w, h }
    }
}
#[cfg(feature = "fontdue-text")]
impl Drop for Atlas { fn drop(&mut self) { unsafe { glDeleteTextures(1, &self.tex); } } }

#[cfg(feature = "fontdue-text")]
struct TextRenderer { program: u32, vao: u32, vbo: u32, u_screen: c_int, u_color: c_int, u_tex: c_int }
#[cfg(feature = "fontdue-text")]
impl TextRenderer {
    fn new() -> Result<Self, String> { let program = link_program(TEXT_VERT, TEXT_FRAG)?; let (vao, vbo) = unit_quad_vao(4)?; Ok(Self{program,vao,vbo,u_screen:uniform(program,"u_screen"),u_color:uniform(program,"u_color"),u_tex:uniform(program,"u_tex")}) }
    fn draw_text(&self, atlas: &Atlas, width:i32, height:i32, mut x:f32, y:f32, text:&str, color:Color4, px:f32) {
        unsafe { glUseProgram(self.program); glBindVertexArray(self.vao); glActiveTexture(GL_TEXTURE0); glBindTexture(GL_TEXTURE_2D, atlas.tex); glUniform1i(self.u_tex,0); glUniform2f(self.u_screen,width as f32,height as f32); glUniform4f(self.u_color,color.0[0],color.0[1],color.0[2],color.0[3]); }
        let baseline = y + px * 0.84;
        for ch in text.chars() { if let Some(g)=atlas.glyphs.get(&ch) { if g.size[0]>0.0 { let gx=x+g.bearing[0]; let gy=baseline-g.bearing[1]-g.size[1]; self.draw_glyph(gx,gy,g); } x += g.advance.max(px*0.32); } }
    }
    fn draw_glyph(&self, x:f32, y:f32, g:&Glyph) { let (u0,v0,u1,v1)=(g.uv[0],g.uv[1],g.uv[2],g.uv[3]); let (w,h)=(g.size[0],g.size[1]); let verts:[f32;24]=[x,y,u0,v0, x+w,y,u1,v0, x+w,y+h,u1,v1, x,y,u0,v0, x+w,y+h,u1,v1, x,y+h,u0,v1]; unsafe { glBindBuffer(GL_ARRAY_BUFFER,self.vbo); glBufferData(GL_ARRAY_BUFFER,(verts.len()*4) as isize,verts.as_ptr() as *const c_void,GL_DYNAMIC_DRAW); glDrawArrays(GL_TRIANGLES,0,6); } }
}
#[cfg(feature = "fontdue-text")]
impl Drop for TextRenderer { fn drop(&mut self){ unsafe{ glDeleteBuffers(1,&self.vbo); glDeleteVertexArrays(1,&self.vao); glDeleteProgram(self.program); } } }

struct State { accent: usize }

fn main(){ if let Err(err)=run(){ eprintln!("ui-preview-sdl-gl-atlas: {err}"); std::process::exit(1); } }
fn run()->Result<(),String>{
    let _sdl=Sdl::init()?; unsafe{ SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION,3); SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION,3); SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK,SDL_GL_CONTEXT_PROFILE_CORE); SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER,1); }
    let mut width=960; let mut height=540; let title=CString::new("EdgeRun GPU Atlas UI Preview").unwrap();
    let window=Window(unsafe{SDL_CreateWindow(title.as_ptr(),SDL_WINDOWPOS_CENTERED,SDL_WINDOWPOS_CENTERED,width,height,SDL_WINDOW_OPENGL|SDL_WINDOW_SHOWN|SDL_WINDOW_RESIZABLE)}); if window.0.is_null(){return Err(format!("SDL_CreateWindow failed: {}",sdl_error()));}
    let ctx=GlContext(unsafe{SDL_GL_CreateContext(window.0)}); if ctx.0.is_null(){return Err(format!("SDL_GL_CreateContext failed: {}",sdl_error()));} unsafe{SDL_GL_SetSwapInterval(1); glEnable(GL_BLEND); glBlendFunc(GL_SRC_ALPHA,GL_ONE_MINUS_SRC_ALPHA);}
    let shapes=ShapeRenderer::new()?;
    #[cfg(feature="fontdue-text")]
    let text=TextRenderer::new()?;
    #[cfg(feature="fontdue-text")]
    let ui_font=FontFace::load_best_ui_font().ok();
    #[cfg(feature="fontdue-text")]
    let icon_font=load_icon_font();
    #[cfg(feature="fontdue-text")]
    let ui_atlas=ui_font.as_ref().map(|f| Atlas::build(f,&ascii_chars(),38.0));
    #[cfg(feature="fontdue-text")]
    let icon_atlas=icon_font.as_ref().map(|f| Atlas::build(f,&icon_chars(),34.0));
    let started=Instant::now(); let mut running=true; let mut state=State{accent:0};
    while running { let mut event=SdlEvent{data:[0;56]}; while unsafe{SDL_PollEvent(&mut event)}!=0{ match event.event_type(){ SDL_QUIT=>running=false, SDL_WINDOWEVENT if event.window_event()==SDL_WINDOWEVENT_RESIZED=>{width=event.data1().max(320);height=event.data2().max(240);}, SDL_MOUSEBUTTONDOWN=>pick_accent(&mut state,event.mouse_x(),event.mouse_y()), SDL_KEYDOWN=>{ let k=event.key_sym(); if (49..=55).contains(&k){state.accent=(k-49) as usize;} }, _=>{} }}
        render_frame(width,height,started.elapsed().as_secs_f32(),state.accent,&shapes,#[cfg(feature="fontdue-text")] &text,#[cfg(feature="fontdue-text")] ui_atlas.as_ref(),#[cfg(feature="fontdue-text")] icon_atlas.as_ref());
        unsafe{SDL_GL_SwapWindow(window.0);SDL_Delay(1);} std::thread::sleep(Duration::from_millis(1)); }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_frame(width:i32,height:i32,elapsed:f32,accent_i:usize,shapes:&ShapeRenderer,#[cfg(feature="fontdue-text")] text:&TextRenderer,#[cfg(feature="fontdue-text")] ui_atlas:Option<&Atlas>,#[cfg(feature="fontdue-text")] icon_atlas:Option<&Atlas>){
    let accent=PALETTE[accent_i.min(PALETTE.len()-1)]; unsafe{glViewport(0,0,width,height);glClearColor(BG.0[0],BG.0[1],BG.0[2],1.0);glClear(GL_COLOR_BUFFER_BIT);} shapes.begin(width,height);
    let cpu=0.12+(elapsed*1.8).sin().abs()*0.55; let ram=0.42; let margin=28.0; let hero_w=(width as f32-margin*2.0).min(920.0); let hero=Rect{x:margin,y:margin,w:hero_w,h:188.0}; shapes.card(hero); shapes.button(Rect{x:margin+26.0,y:margin+128.0,w:170.0,h:40.0},accent,true); shapes.button(Rect{x:margin+210.0,y:margin+128.0,w:172.0,h:40.0},accent,false);
    let stats_y=margin+210.0; let gap=16.0; let stat_w=(hero_w-gap*2.0)/3.0; let r1=Rect{x:margin,y:stats_y,w:stat_w,h:134.0}; let r2=Rect{x:margin+stat_w+gap,y:stats_y,w:stat_w,h:134.0}; let r3=Rect{x:margin+(stat_w+gap)*2.0,y:stats_y,w:stat_w,h:134.0}; shapes.card(r1); shapes.card(r2); shapes.card(r3); shapes.progress(Rect{x:margin+20.0,y:stats_y+106.0,w:stat_w-40.0,h:14.0},cpu,accent); shapes.progress(Rect{x:margin+stat_w+gap+20.0,y:stats_y+106.0,w:stat_w-40.0,h:14.0},ram,accent);
    for (i,c) in PALETTE.iter().enumerate(){ shapes.swatch(Rect{x:margin+i as f32*34.0,y:height as f32-54.0,w:24.0,h:24.0},*c,i==accent_i); }
    #[cfg(feature="fontdue-text")]
    if let Some(a)=ui_atlas{ text.draw_text(a,width,height,122.0,52.0,"EdgeRun",TEXT,38.0); text.draw_text(a,width,height,124.0,98.0,"GPU atlas text / Tabler icons / accent picker",MUTED,38.0); text.draw_text(a,width,height,96.0,276.0,"CPU",MUTED,38.0); text.draw_text(a,width,height,374.0,276.0,"RAM",MUTED,38.0); text.draw_text(a,width,height,652.0,276.0,"NET",MUTED,38.0); text.draw_text(a,width,height,28.0,height as f32-74.0,"accent",MUTED,38.0); }
    #[cfg(feature="fontdue-text")]
    if let Some(a)=icon_atlas{ draw_icon(text,a,width,height,"sparkles",40.0,52.0,accent); draw_icon(text,a,width,height,"activity",50.0,270.0,accent); draw_icon(text,a,width,height,"server",328.0,270.0,CYAN); draw_icon(text,a,width,height,"network",606.0,270.0,EMERALD); }
}
fn pick_accent(state:&mut State,x:i32,y:i32){ if y < 486 {return;} let start=28; for i in 0..PALETTE.len(){ let sx=start+i as i32*34; if x>=sx && x<=sx+24 { state.accent=i; } } }

#[cfg(feature="fontdue-text")]
fn ascii_chars()->Vec<char>{ (32u8..=126).map(char::from).collect() }
#[cfg(feature="fontdue-text")]
fn icon_chars()->Vec<char>{ ["sparkles","activity","server","network","check","shield-check","wallet","key","lock"].into_iter().filter_map(tabler_icon).collect() }
#[cfg(feature="fontdue-text")]
fn load_icon_font()->Option<FontFace>{ let path=std::env::var("EDGE_TABLER_FONT").unwrap_or_else(|_|TABLER_ICON_FONT_HINT.to_string()); FontFace::from_bytes(std::fs::read(path).ok()?).ok() }
#[cfg(feature="fontdue-text")]
fn draw_icon(text:&TextRenderer,atlas:&Atlas,width:i32,height:i32,name:&str,x:f32,y:f32,color:Color4){ if let Some(ch)=tabler_icon(name){ let mut b=[0u8;4]; let s=ch.encode_utf8(&mut b); text.draw_text(atlas,width,height,x,y,s,color,34.0); } }

fn unit_quad_vao(stride_floats: i32)->Result<(u32,u32),String>{ let mut vao=0; let mut vbo=0; let verts:[f32;24]=[0.0,0.0,0.0,0.0, 1.0,0.0,1.0,0.0, 1.0,1.0,1.0,1.0, 0.0,0.0,0.0,0.0, 1.0,1.0,1.0,1.0, 0.0,1.0,0.0,1.0]; let count=if stride_floats==2{12}else{24}; unsafe{glGenVertexArrays(1,&mut vao);glBindVertexArray(vao);glGenBuffers(1,&mut vbo);glBindBuffer(GL_ARRAY_BUFFER,vbo);glBufferData(GL_ARRAY_BUFFER,(count*4) as isize,verts.as_ptr() as *const c_void,GL_DYNAMIC_DRAW);glEnableVertexAttribArray(0);glVertexAttribPointer(0,stride_floats,GL_FLOAT,GL_FALSE,stride_floats*4,ptr::null());} Ok((vao,vbo)) }
fn link_program(v:&str,f:&str)->Result<u32,String>{ let p=unsafe{glCreateProgram()}; let vs=compile_shader(GL_VERTEX_SHADER,v)?; let fs=compile_shader(GL_FRAGMENT_SHADER,f)?; unsafe{glAttachShader(p,vs);glAttachShader(p,fs);glLinkProgram(p);glDeleteShader(vs);glDeleteShader(fs);} check_program(p)?; Ok(p) }
fn compile_shader(kind:u32,source:&str)->Result<u32,String>{ let sh=unsafe{glCreateShader(kind)}; let c=CString::new(source).unwrap(); let ptr=c.as_ptr(); unsafe{glShaderSource(sh,1,&ptr,ptr::null());glCompileShader(sh);} let mut ok=0; unsafe{glGetShaderiv(sh,GL_COMPILE_STATUS,&mut ok);} if ok==0{let log=shader_log(sh);unsafe{glDeleteShader(sh);}Err(log)}else{Ok(sh)} }
fn check_program(p:u32)->Result<(),String>{ let mut ok=0; unsafe{glGetProgramiv(p,GL_LINK_STATUS,&mut ok);} if ok==0{Err(program_log(p))}else{Ok(())} }
fn shader_log(s:u32)->String{ let mut b=vec![0i8;2048]; let mut l=0; unsafe{glGetShaderInfoLog(s,b.len() as i32,&mut l,b.as_mut_ptr()); CStr::from_ptr(b.as_ptr()).to_string_lossy().into_owned()} }
fn program_log(p:u32)->String{ let mut b=vec![0i8;2048]; let mut l=0; unsafe{glGetProgramInfoLog(p,b.len() as i32,&mut l,b.as_mut_ptr()); CStr::from_ptr(b.as_ptr()).to_string_lossy().into_owned()} }
fn uniform(p:u32,n:&str)->i32{ let c=CString::new(n).unwrap(); unsafe{glGetUniformLocation(p,c.as_ptr())} }
struct Sdl; impl Sdl{fn init()->Result<Self,String>{let rc=unsafe{SDL_Init(SDL_INIT_VIDEO)}; if rc!=0{Err(sdl_error())}else{Ok(Self)}}} impl Drop for Sdl{fn drop(&mut self){unsafe{SDL_Quit()};}}
struct Window(*mut SDL_Window); impl Drop for Window{fn drop(&mut self){if !self.0.is_null(){unsafe{SDL_DestroyWindow(self.0)};}}}
struct GlContext(SDL_GLContext); impl Drop for GlContext{fn drop(&mut self){if !self.0.is_null(){unsafe{SDL_GL_DeleteContext(self.0)};}}}
fn sdl_error()->String{let p=unsafe{SDL_GetError()}; if p.is_null(){"unknown SDL error".into()}else{unsafe{CStr::from_ptr(p)}.to_string_lossy().into_owned()}}
