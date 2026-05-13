use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::thread;
use std::time::Duration;

use edgerun_ui_core::gpu::gl::GlRenderer;
use edgerun_ui_core::gpu::{
    FontAtlas, GpuScene, UiColorScheme, UiEvent, UiKey, UiRuntimeState, UnifiedChatState,
    build_unified_chat_shell_with_font_and_runtime, palette,
};

const SDL_INIT_VIDEO: u32 = 0x0000_0020;
const SDL_WINDOWPOS_CENTERED: c_int = 0x2fff_0000u32 as c_int;
const SDL_WINDOW_OPENGL: u32 = 0x0000_0002;
const SDL_WINDOW_SHOWN: u32 = 0x0000_0004;
const SDL_WINDOW_RESIZABLE: u32 = 0x0000_0020;
const SDL_QUIT: u32 = 0x100;
const SDL_KEYDOWN: u32 = 0x300;
const SDL_MOUSEMOTION: u32 = 0x400;
const SDL_MOUSEBUTTONDOWN: u32 = 0x401;
const SDL_MOUSEBUTTONUP: u32 = 0x402;
const SDL_MOUSEWHEEL: u32 = 0x403;
const SDL_WINDOWEVENT: u32 = 0x200;
const SDL_WINDOWEVENT_RESIZED: u8 = 0x05;
const SDL_GL_CONTEXT_MAJOR_VERSION: c_int = 17;
const SDL_GL_CONTEXT_MINOR_VERSION: c_int = 18;
const SDL_GL_CONTEXT_PROFILE_MASK: c_int = 21;
const SDL_GL_CONTEXT_PROFILE_CORE: c_int = 0x0001;
const SDL_GL_DOUBLEBUFFER: c_int = 5;
const SDLK_ESCAPE: i32 = 27;

#[repr(C)]
struct SDL_Window(c_void);

type SdlGlContext = *mut c_void;

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

    fn key_sym(&self) -> i32 {
        i32::from_ne_bytes([self.data[20], self.data[21], self.data[22], self.data[23]])
    }

    fn mouse_x(&self) -> f32 {
        i32::from_ne_bytes([self.data[20], self.data[21], self.data[22], self.data[23]]) as f32
    }

    fn mouse_y(&self) -> f32 {
        i32::from_ne_bytes([self.data[24], self.data[25], self.data[26], self.data[27]]) as f32
    }

    fn wheel_y(&self) -> f32 {
        i32::from_ne_bytes([self.data[24], self.data[25], self.data[26], self.data[27]]) as f32
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
    fn SDL_GL_CreateContext(window: *mut SDL_Window) -> SdlGlContext;
    fn SDL_GL_DeleteContext(context: SdlGlContext);
    fn SDL_GL_SetSwapInterval(interval: c_int) -> c_int;
    fn SDL_GL_SwapWindow(window: *mut SDL_Window);
    fn SDL_PollEvent(event: *mut SdlEvent) -> c_int;
    fn SDL_Delay(ms: u32);
}

fn main() {
    if let Err(error) = run() {
        eprintln!("codex-gl-ui: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    if args.dump_scene {
        let mut scene = GpuScene::new(palette::BG);
        let atlas = FontAtlas::load_inter(18.0)?;
        let ui_state = UiRuntimeState::default();
        build_surface(&mut scene, &atlas, &ui_state, 1120.0, 720.0, args.scheme);
        println!(
            "codex-gl-ui unified-chat scene rects={} text_quads={}",
            scene.rects().len(),
            scene.text_quads().len()
        );
        return Ok(());
    }

    let _sdl = Sdl::init()?;
    unsafe {
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 3);
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 3);
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_CORE);
        SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);
    }

    let mut width = 1120;
    let mut height = 720;
    let title = CString::new("EdgeRun Unified Chat").map_err(|error| error.to_string())?;
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

    let _context = GlContext(unsafe { SDL_GL_CreateContext(window.0) });
    if _context.0.is_null() {
        return Err(format!("SDL_GL_CreateContext failed: {}", sdl_error()));
    }
    unsafe {
        SDL_GL_SetSwapInterval(1);
    }

    let atlas = FontAtlas::load_inter(18.0)?;
    let renderer = unsafe { GlRenderer::new_current_context_with_font(&atlas)? };
    let mut scene = GpuScene::new(palette::BG);
    let mut ui_state = UiRuntimeState::default();
    let mut running = true;
    let mut frames = 0u32;
    let mut scene_dirty = true;
    while running {
        let mut event = SdlEvent { data: [0; 56] };
        while unsafe { SDL_PollEvent(&mut event) } != 0 {
            match event.event_type() {
                SDL_QUIT => running = false,
                SDL_KEYDOWN if event.key_sym() == SDLK_ESCAPE => {
                    running = false;
                    let _ = ui_state.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Escape });
                }
                SDL_KEYDOWN => {
                    ui_state.handle_event(
                        &scene,
                        UiEvent::KeyDown {
                            key: sdl_key(event.key_sym()),
                        },
                    );
                    scene_dirty = true;
                }
                SDL_MOUSEBUTTONDOWN => {
                    ui_state.handle_event(
                        &scene,
                        UiEvent::PointerDown {
                            x: event.mouse_x(),
                            y: event.mouse_y(),
                        },
                    );
                    scene_dirty = true;
                }
                SDL_MOUSEMOTION => {
                    ui_state.handle_event(
                        &scene,
                        UiEvent::PointerMove {
                            x: event.mouse_x(),
                            y: event.mouse_y(),
                        },
                    );
                    scene_dirty = true;
                }
                SDL_MOUSEBUTTONUP => {
                    ui_state.handle_event(
                        &scene,
                        UiEvent::PointerUp {
                            x: event.mouse_x(),
                            y: event.mouse_y(),
                        },
                    );
                    scene_dirty = true;
                }
                SDL_MOUSEWHEEL => {
                    ui_state.handle_event(
                        &scene,
                        UiEvent::Wheel {
                            x: ui_state.hovered().map(|hit| hit.x).unwrap_or(0.0),
                            y: ui_state.hovered().map(|hit| hit.y).unwrap_or(0.0),
                            delta_y: -event.wheel_y() * 120.0,
                        },
                    );
                    scene_dirty = true;
                }
                SDL_WINDOWEVENT if event.window_event() == SDL_WINDOWEVENT_RESIZED => {
                    width = event.data1().max(360);
                    height = event.data2().max(320);
                    scene_dirty = true;
                }
                _ => {}
            }
        }

        if scene_dirty {
            build_surface(
                &mut scene,
                &atlas,
                &ui_state,
                width as f32,
                height as f32,
                args.scheme,
            );
            renderer.render(width, height, &scene);
            unsafe {
                SDL_GL_SwapWindow(window.0);
            }
            frames = frames.saturating_add(1);
            scene_dirty = false;
        }
        if args.frames.is_some_and(|limit| frames >= limit) {
            running = false;
        } else if args.frames.is_some() {
            scene_dirty = true;
        }
        unsafe {
            SDL_Delay(4);
        }
        thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

fn sdl_key(sym: i32) -> UiKey {
    match sym {
        8 => UiKey::Backspace,
        9 => UiKey::Tab,
        13 => UiKey::Enter,
        27 => UiKey::Escape,
        1073741904 => UiKey::ArrowLeft,
        1073741903 => UiKey::ArrowRight,
        1073741906 => UiKey::ArrowUp,
        1073741905 => UiKey::ArrowDown,
        other => UiKey::Other(other as u32),
    }
}

fn build_surface(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    ui_state: &UiRuntimeState,
    width: f32,
    height: f32,
    scheme: UiColorScheme,
) {
    let state = UnifiedChatState::empty();
    build_unified_chat_shell_with_font_and_runtime(scene, atlas, width, height, &state, ui_state);
    scene.apply_color_scheme(scheme);
}

#[derive(Default)]
struct Args {
    frames: Option<u32>,
    dump_scene: bool,
    scheme: UiColorScheme,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut args = std::env::args().skip(1);
        let mut parsed = Self::default();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--frames" => {
                    let value = args
                        .next()
                        .ok_or("--frames requires a frame count")?
                        .parse::<u32>()
                        .map_err(|error| format!("invalid --frames value: {error}"))?;
                    parsed.frames = Some(value);
                }
                "--dump-scene" => parsed.dump_scene = true,
                "--scheme" => {
                    let value = args
                        .next()
                        .ok_or("--scheme requires dark, light, or terminal")?;
                    parsed.scheme = parse_scheme(&value)?;
                }
                "--help" | "-h" => {
                    println!(
                        "Usage: codex-gl-ui [--frames N] [--dump-scene] [--scheme dark|light|terminal]"
                    );
                    std::process::exit(0);
                }
                other => return Err(format!("unknown argument: {other}")),
            }
        }
        Ok(parsed)
    }
}

fn parse_scheme(value: &str) -> Result<UiColorScheme, String> {
    match value {
        "dark" => Ok(UiColorScheme::Dark),
        "light" => Ok(UiColorScheme::Light),
        "terminal" => Ok(UiColorScheme::Terminal),
        _ => Err(format!("invalid --scheme value: {value}")),
    }
}

struct Sdl;

impl Sdl {
    fn init() -> Result<Self, String> {
        let rc = unsafe { SDL_Init(SDL_INIT_VIDEO) };
        if rc == 0 { Ok(Self) } else { Err(sdl_error()) }
    }
}

impl Drop for Sdl {
    fn drop(&mut self) {
        unsafe {
            SDL_Quit();
        }
    }
}

struct Window(*mut SDL_Window);

impl Drop for Window {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                SDL_DestroyWindow(self.0);
            }
        }
    }
}

struct GlContext(SdlGlContext);

impl Drop for GlContext {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                SDL_GL_DeleteContext(self.0);
            }
        }
    }
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
