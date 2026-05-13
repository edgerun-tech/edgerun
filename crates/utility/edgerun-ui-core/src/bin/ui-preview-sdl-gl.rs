use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::time::{Duration, Instant};

const SDL_INIT_VIDEO: u32 = 0x0000_0020;
const SDL_WINDOWPOS_CENTERED: c_int = 0x2fff_0000u32 as c_int;
const SDL_WINDOW_OPENGL: u32 = 0x0000_0002;
const SDL_WINDOW_SHOWN: u32 = 0x0000_0004;
const SDL_WINDOW_RESIZABLE: u32 = 0x0000_0020;
const SDL_QUIT: u32 = 0x100;
const SDL_WINDOWEVENT: u32 = 0x200;
const SDL_WINDOWEVENT_RESIZED: u8 = 0x05;
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

#[repr(C)]
struct SDL_Window(c_void);
type SDL_GLContext = *mut c_void;

#[repr(C)]
struct SdlEvent {
    data: [u8; 56],
}

impl SdlEvent {
    fn event_type(&self) -> u32 {
        u32::from_ne_bytes([self.data[0], self.data[1], self.data[2], self.data[3]])
    }

    fn window_event(&self) -> u8 {
        self.data[8]
    }

    fn data1(&self) -> i32 {
        i32::from_ne_bytes([self.data[16], self.data[17], self.data[18], self.data[19]])
    }

    fn data2(&self) -> i32 {
        i32::from_ne_bytes([self.data[20], self.data[21], self.data[22], self.data[23]])
    }
}

#[link(name = "SDL2")]
unsafe extern "C" {
    fn SDL_Init(flags: u32) -> c_int;
    fn SDL_Quit();
    fn SDL_GetError() -> *const c_char;
    fn SDL_GL_SetAttribute(attr: c_int, value: c_int) -> c_int;
    fn SDL_CreateWindow(
        title: *const c_char,
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
        flags: u32,
    ) -> *mut SDL_Window;
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
        out_color = vec4(u_color.rgb, u_color.a * alpha * 0.38);
    } else if (u_mode == 2) {
        float inner = rounded_box(p, v_size * 0.5 - vec2(1.5), max(u_radius - 1.5, 0.0));
        float border = (1.0 - smoothstep(0.0, aa, d)) * smoothstep(0.0, aa, inner);
        out_color = vec4(u_color.rgb, u_color.a * border);
    } else {
        out_color = vec4(u_color.rgb, u_color.a * alpha);
    }
}
"#;

#[derive(Clone, Copy)]
struct GpuRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: [f32; 4],
    mode: i32,
    shadow: f32,
}

struct GpuUi {
    program: u32,
    vao: u32,
    vbo: u32,
    u_screen: c_int,
    u_rect: c_int,
    u_color: c_int,
    u_radius: c_int,
    u_mode: c_int,
    u_shadow: c_int,
}

impl GpuUi {
    fn new() -> Result<Self, String> {
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
                verts.as_ptr() as *const c_void,
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
        })
    }

    fn render(&self, width: i32, height: i32, elapsed: f32) {
        unsafe {
            glViewport(0, 0, width, height);
            glClearColor(0.008, 0.024, 0.090, 1.0);
            glClear(GL_COLOR_BUFFER_BIT);
            glEnable(GL_BLEND);
            glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
            glUseProgram(self.program);
            glBindVertexArray(self.vao);
            glUniform2f(self.u_screen, width as f32, height as f32);
        }

        let cpu = 0.12 + (elapsed * 1.8).sin().abs() * 0.55;
        let ram = 0.42;
        let margin = 28.0;
        let hero_w = (width as f32 - margin * 2.0).min(920.0);
        let hero = GpuRect::card(margin, margin, hero_w, 188.0);
        self.draw_card(hero);
        self.draw_button(GpuRect::button(
            margin + 26.0,
            margin + 128.0,
            170.0,
            40.0,
            true,
        ));
        self.draw_button(GpuRect::button(
            margin + 210.0,
            margin + 128.0,
            172.0,
            40.0,
            false,
        ));

        let stats_y = margin + 210.0;
        let gap = 16.0;
        let stat_w = (hero_w - gap * 2.0) / 3.0;
        let r1 = GpuRect::card(margin, stats_y, stat_w, 134.0);
        let r2 = GpuRect::card(margin + stat_w + gap, stats_y, stat_w, 134.0);
        let r3 = GpuRect::card(margin + (stat_w + gap) * 2.0, stats_y, stat_w, 134.0);
        self.draw_card(r1);
        self.draw_card(r2);
        self.draw_card(r3);

        self.draw_rect(GpuRect::bar(
            margin + 20.0,
            stats_y + 106.0,
            stat_w - 40.0,
            14.0,
            0.0,
        ));
        self.draw_rect(GpuRect::bar(
            margin + 20.0,
            stats_y + 106.0,
            (stat_w - 40.0) * cpu,
            14.0,
            1.0,
        ));
        self.draw_rect(GpuRect::bar(
            margin + stat_w + gap + 20.0,
            stats_y + 106.0,
            stat_w - 40.0,
            14.0,
            0.0,
        ));
        self.draw_rect(GpuRect::bar(
            margin + stat_w + gap + 20.0,
            stats_y + 106.0,
            (stat_w - 40.0) * ram,
            14.0,
            1.0,
        ));
    }

    fn draw_card(&self, rect: GpuRect) {
        let shadow = GpuRect {
            x: rect.x,
            y: rect.y + 4.0,
            w: rect.w,
            h: rect.h,
            radius: rect.radius,
            color: [0.0, 0.0, 0.0, 0.22],
            mode: 1,
            shadow: 10.0,
        };
        self.draw_rect(shadow);
        self.draw_rect(rect);
        self.draw_rect(GpuRect {
            color: [0.20, 0.26, 0.35, 0.62],
            mode: 2,
            ..rect
        });
    }

    fn draw_button(&self, rect: GpuRect) {
        if rect.color[0] > 0.03 {
            let shadow = GpuRect {
                x: rect.x,
                y: rect.y + 2.0,
                w: rect.w,
                h: rect.h,
                radius: rect.radius,
                color: [0.0, 0.60, 0.80, 0.20],
                mode: 1,
                shadow: 7.0,
            };
            self.draw_rect(shadow);
        }
        self.draw_rect(rect);
    }

    fn draw_rect(&self, r: GpuRect) {
        unsafe {
            glUniform4f(self.u_rect, r.x, r.y, r.w, r.h);
            glUniform4f(self.u_color, r.color[0], r.color[1], r.color[2], r.color[3]);
            glUniform1f(self.u_radius, r.radius);
            glUniform1i(self.u_mode, r.mode);
            glUniform1f(self.u_shadow, r.shadow);
            glDrawArrays(GL_TRIANGLES, 0, 6);
        }
    }
}

impl Drop for GpuUi {
    fn drop(&mut self) {
        unsafe {
            glDeleteBuffers(1, &self.vbo);
            glDeleteVertexArrays(1, &self.vao);
            glDeleteProgram(self.program);
        }
    }
}

impl GpuRect {
    fn card(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius: 18.0,
            color: [0.059, 0.090, 0.165, 0.94],
            mode: 0,
            shadow: 0.0,
        }
    }

    fn button(x: f32, y: f32, w: f32, h: f32, active: bool) -> Self {
        let color = if active {
            [0.055, 0.624, 0.820, 1.0]
        } else {
            [0.118, 0.161, 0.231, 0.88]
        };
        Self {
            x,
            y,
            w,
            h,
            radius: h * 0.5,
            color,
            mode: 0,
            shadow: 0.0,
        }
    }

    fn bar(x: f32, y: f32, w: f32, h: f32, active: f32) -> Self {
        let color = if active > 0.5 {
            [0.055, 0.624, 0.820, 1.0]
        } else {
            [0.118, 0.161, 0.231, 0.92]
        };
        Self {
            x,
            y,
            w,
            h,
            radius: h * 0.5,
            color,
            mode: 0,
            shadow: 0.0,
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("ui-preview-sdl-gl: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let _sdl = Sdl::init()?;
    unsafe {
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 3);
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 3);
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_CORE);
        SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);
    }

    let mut width = 960;
    let mut height = 540;
    let title = CString::new("EdgeRun GPU UI Preview").unwrap();
    let window = Window(unsafe {
        SDL_CreateWindow(
            title.as_ptr(),
            SDL_WINDOWPOS_CENTERED,
            SDL_WINDOWPOS_CENTERED,
            width,
            height,
            SDL_WINDOW_OPENGL | SDL_WINDOW_SHOWN | SDL_WINDOW_RESIZABLE,
        )
    });
    if window.0.is_null() {
        return Err(format!("SDL_CreateWindow failed: {}", sdl_error()));
    }

    let gl_ctx = GlContext(unsafe { SDL_GL_CreateContext(window.0) });
    if gl_ctx.0.is_null() {
        return Err(format!("SDL_GL_CreateContext failed: {}", sdl_error()));
    }
    unsafe {
        SDL_GL_SetSwapInterval(1);
    }

    let gpu = GpuUi::new()?;
    let started = Instant::now();
    let mut running = true;
    while running {
        let mut event = SdlEvent { data: [0; 56] };
        while unsafe { SDL_PollEvent(&mut event) } != 0 {
            match event.event_type() {
                SDL_QUIT => running = false,
                SDL_WINDOWEVENT if event.window_event() == SDL_WINDOWEVENT_RESIZED => {
                    width = event.data1().max(320);
                    height = event.data2().max(240);
                }
                _ => {}
            }
        }
        gpu.render(width, height, started.elapsed().as_secs_f32());
        unsafe {
            SDL_GL_SwapWindow(window.0);
            SDL_Delay(1);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

struct Sdl;
impl Sdl {
    fn init() -> Result<Self, String> {
        let rc = unsafe { SDL_Init(SDL_INIT_VIDEO) };
        if rc != 0 { Err(sdl_error()) } else { Ok(Self) }
    }
}
impl Drop for Sdl {
    fn drop(&mut self) {
        unsafe { SDL_Quit() };
    }
}

struct Window(*mut SDL_Window);
impl Drop for Window {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { SDL_DestroyWindow(self.0) };
        }
    }
}

struct GlContext(SDL_GLContext);
impl Drop for GlContext {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { SDL_GL_DeleteContext(self.0) };
        }
    }
}

fn compile_shader(kind: u32, source: &str) -> Result<u32, String> {
    let shader = unsafe { glCreateShader(kind) };
    let c_src = CString::new(source).unwrap();
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
    }
    unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

fn program_log(program: u32) -> String {
    let mut buf = vec![0i8; 2048];
    let mut len = 0;
    unsafe {
        glGetProgramInfoLog(program, buf.len() as i32, &mut len, buf.as_mut_ptr());
    }
    unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

fn uniform(program: u32, name: &str) -> i32 {
    let c = CString::new(name).unwrap();
    unsafe { glGetUniformLocation(program, c.as_ptr()) }
}

fn sdl_error() -> String {
    let ptr = unsafe { SDL_GetError() };
    if ptr.is_null() {
        "unknown SDL error".to_string()
    } else {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}
