use std::ffi::{CStr, CString};
use std::format;
use std::os::raw::{c_char, c_int, c_void};
use std::string::ToString;

use super::gl::GlRenderer;
use super::{
    Color4, FontAtlas, GpuScene, UiAction, UiEvent, UiNode, UiPainter, UiRect, UiRuntimeState,
};

const SDL_INIT_VIDEO: u32 = 0x0000_0020;
const SDL_WINDOWPOS_CENTERED: c_int = 0x2fff_0000u32 as c_int;
const SDL_WINDOW_OPENGL: u32 = 0x0000_0002;
const SDL_WINDOW_SHOWN: u32 = 0x0000_0004;
const SDL_WINDOW_RESIZABLE: u32 = 0x0000_0020;
const SDL_QUIT: u32 = 0x100;
const SDL_WINDOWEVENT: u32 = 0x200;
const SDL_KEYDOWN: u32 = 0x300;
const SDL_MOUSEMOTION: u32 = 0x400;
const SDL_MOUSEBUTTONDOWN: u32 = 0x401;
const SDL_MOUSEBUTTONUP: u32 = 0x402;
const SDL_MOUSEWHEEL: u32 = 0x403;
const SDL_WINDOWEVENT_RESIZED: u8 = 0x05;
const SDL_WINDOWEVENT_SIZE_CHANGED: u8 = 0x06;
const SDL_GL_CONTEXT_MAJOR_VERSION: c_int = 17;
const SDL_GL_CONTEXT_MINOR_VERSION: c_int = 18;
const SDL_GL_CONTEXT_PROFILE_MASK: c_int = 21;
const SDL_GL_CONTEXT_PROFILE_CORE: c_int = 0x0001;
const SDL_GL_DOUBLEBUFFER: c_int = 5;

pub const SDL_KEY_ESCAPE: i32 = 27;

pub struct SdlDirectDrawCapability {
    _private: (),
}

impl SdlDirectDrawCapability {
    /// Grants direct access to the scene renderer.
    ///
    /// Prefer `run_sdl_gl_window` for app code. Direct scene access is intended
    /// for trusted framework surfaces, diagnostics, and render backend tests.
    pub unsafe fn new_unchecked() -> Self {
        Self { _private: () }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SdlGlWindowOptions<'a> {
    pub title: &'a str,
    pub width: i32,
    pub height: i32,
    pub min_width: i32,
    pub min_height: i32,
    pub clear: Color4,
    pub frames: Option<u32>,
}

impl<'a> SdlGlWindowOptions<'a> {
    pub const fn new(title: &'a str, width: i32, height: i32, clear: Color4) -> Self {
        Self {
            title,
            width,
            height,
            min_width: 320,
            min_height: 240,
            clear,
            frames: None,
        }
    }

    pub const fn min_size(mut self, width: i32, height: i32) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }

    pub const fn frames(mut self, frames: Option<u32>) -> Self {
        self.frames = frames;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SdlEventResult {
    pub dirty: bool,
    pub quit: bool,
}

impl SdlEventResult {
    pub const fn dirty() -> Self {
        Self {
            dirty: true,
            quit: false,
        }
    }

    pub const fn quit() -> Self {
        Self {
            dirty: false,
            quit: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SdlInputEvent {
    CloseRequested,
    KeyDown { key: i32 },
    MouseWheel { y: f32 },
    MouseMotion { x: f32, y: f32 },
    MouseDown { x: f32, y: f32 },
    MouseUp { x: f32, y: f32 },
    UiAction { action: UiAction },
    Resized { width: i32, height: i32 },
}

#[repr(C)]
struct SDL_Window(c_void);

type SdlGlContext = *mut c_void;

#[repr(C)]
struct RawSdlEvent {
    data: [u8; 56],
}

impl RawSdlEvent {
    fn event_type(&self) -> u32 {
        u32::from_ne_bytes([self.data[0], self.data[1], self.data[2], self.data[3]])
    }

    fn window_event(&self) -> u8 {
        self.data[12]
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

    fn wheel_y(&self) -> f32 {
        i32::from_ne_bytes([self.data[20], self.data[21], self.data[22], self.data[23]]) as f32
    }

    fn mouse_x(&self) -> f32 {
        i32::from_ne_bytes([self.data[20], self.data[21], self.data[22], self.data[23]]) as f32
    }

    fn mouse_y(&self) -> f32 {
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
    fn SDL_GetWindowSize(window: *mut SDL_Window, w: *mut c_int, h: *mut c_int);
    fn SDL_SetWindowMinimumSize(window: *mut SDL_Window, min_w: c_int, min_h: c_int);
    fn SDL_PollEvent(event: *mut RawSdlEvent) -> c_int;
    fn SDL_Delay(ms: u32);
}

struct Sdl;
struct Window(*mut SDL_Window);
struct GlContext(SdlGlContext);

impl Drop for Sdl {
    fn drop(&mut self) {
        unsafe { SDL_Quit() };
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { SDL_DestroyWindow(self.0) };
        }
    }
}

impl Drop for GlContext {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { SDL_GL_DeleteContext(self.0) };
        }
    }
}

pub fn run_sdl_gl_window(
    options: SdlGlWindowOptions<'_>,
    on_event: impl FnMut(SdlInputEvent) -> SdlEventResult,
    layout: impl FnMut(i32, i32) -> UiNode,
) -> Result<(), String> {
    run_sdl_gl_layout_window(options, on_event, layout)
}

pub fn run_sdl_gl_layout_window(
    options: SdlGlWindowOptions<'_>,
    on_event: impl FnMut(SdlInputEvent) -> SdlEventResult,
    mut layout: impl FnMut(i32, i32) -> UiNode,
) -> Result<(), String> {
    let direct = SdlDirectDrawCapability { _private: () };
    run_sdl_gl_window_backend(
        options,
        direct,
        on_event,
        move |scene, atlas, width, height, runtime| {
            scene.clear_rects();
            let root = layout(width, height);
            let mut ui = UiPainter::with_font(scene, atlas);
            root.render_with_state(
                &mut ui,
                UiRect::new(0.0, 0.0, width as f32, height as f32),
                runtime,
            );
        },
    )
}

pub fn run_sdl_gl_window_direct(
    options: SdlGlWindowOptions<'_>,
    _capability: SdlDirectDrawCapability,
    on_event: impl FnMut(SdlInputEvent) -> SdlEventResult,
    mut render: impl FnMut(&mut GpuScene, &FontAtlas, i32, i32),
) -> Result<(), String> {
    run_sdl_gl_window_backend(
        options,
        _capability,
        on_event,
        move |scene, atlas, width, height, _runtime| render(scene, atlas, width, height),
    )
}

fn run_sdl_gl_window_backend(
    options: SdlGlWindowOptions<'_>,
    _capability: SdlDirectDrawCapability,
    mut on_event: impl FnMut(SdlInputEvent) -> SdlEventResult,
    mut render: impl FnMut(&mut GpuScene, &FontAtlas, i32, i32, Option<&UiRuntimeState>),
) -> Result<(), String> {
    let _sdl = init_sdl()?;
    unsafe {
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 3);
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 3);
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_CORE);
        SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);
    }

    let mut width = options.width.max(options.min_width);
    let mut height = options.height.max(options.min_height);
    let title = CString::new(options.title).map_err(|error| error.to_string())?;
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
    unsafe {
        SDL_SetWindowMinimumSize(window.0, options.min_width, options.min_height);
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
    let mut scene = GpuScene::new(options.clear);
    let mut running = true;
    let mut dirty = true;
    let mut rendered_frames = 0u32;
    let mut runtime = UiRuntimeState::default();
    let mut pointer_x = 0.0_f32;
    let mut pointer_y = 0.0_f32;

    while running {
        let mut event = RawSdlEvent { data: [0; 56] };
        while unsafe { SDL_PollEvent(&mut event) } != 0 {
            let mapped = match event.event_type() {
                SDL_QUIT => Some(SdlInputEvent::CloseRequested),
                SDL_KEYDOWN => Some(SdlInputEvent::KeyDown {
                    key: event.key_sym(),
                }),
                SDL_MOUSEWHEEL => {
                    let action = runtime.handle_event(
                        &scene,
                        UiEvent::Wheel {
                            x: pointer_x,
                            y: pointer_y,
                            delta_y: -event.wheel_y() * 72.0,
                        },
                    );
                    if action != UiAction::None {
                        dirty = true;
                        let result = on_event(SdlInputEvent::UiAction { action });
                        dirty |= result.dirty;
                        running &= !result.quit;
                    }
                    Some(SdlInputEvent::MouseWheel { y: event.wheel_y() })
                }
                SDL_MOUSEMOTION => {
                    pointer_x = event.mouse_x();
                    pointer_y = event.mouse_y();
                    let action = runtime.handle_event(
                        &scene,
                        UiEvent::PointerMove {
                            x: pointer_x,
                            y: pointer_y,
                        },
                    );
                    if action != UiAction::None {
                        dirty = true;
                        let result = on_event(SdlInputEvent::UiAction { action });
                        dirty |= result.dirty;
                        running &= !result.quit;
                    }
                    Some(SdlInputEvent::MouseMotion {
                        x: pointer_x,
                        y: pointer_y,
                    })
                }
                SDL_MOUSEBUTTONDOWN => {
                    pointer_x = event.mouse_x();
                    pointer_y = event.mouse_y();
                    let action = runtime.handle_event(
                        &scene,
                        UiEvent::PointerDown {
                            x: pointer_x,
                            y: pointer_y,
                        },
                    );
                    if action != UiAction::None {
                        dirty = true;
                        let result = on_event(SdlInputEvent::UiAction { action });
                        dirty |= result.dirty;
                        running &= !result.quit;
                    }
                    Some(SdlInputEvent::MouseDown {
                        x: pointer_x,
                        y: pointer_y,
                    })
                }
                SDL_MOUSEBUTTONUP => {
                    pointer_x = event.mouse_x();
                    pointer_y = event.mouse_y();
                    let action = runtime.handle_event(
                        &scene,
                        UiEvent::PointerUp {
                            x: pointer_x,
                            y: pointer_y,
                        },
                    );
                    if action != UiAction::None {
                        dirty = true;
                        let result = on_event(SdlInputEvent::UiAction { action });
                        dirty |= result.dirty;
                        running &= !result.quit;
                    }
                    Some(SdlInputEvent::MouseUp {
                        x: pointer_x,
                        y: pointer_y,
                    })
                }
                SDL_WINDOWEVENT
                    if matches!(
                        event.window_event(),
                        SDL_WINDOWEVENT_RESIZED | SDL_WINDOWEVENT_SIZE_CHANGED
                    ) =>
                {
                    width = event.data1().max(options.min_width);
                    height = event.data2().max(options.min_height);
                    dirty = true;
                    Some(SdlInputEvent::Resized { width, height })
                }
                _ => None,
            };
            if let Some(mapped) = mapped {
                let result = on_event(mapped);
                dirty |= result.dirty;
                running &= !result.quit;
            }
        }

        let mut current_w = 0;
        let mut current_h = 0;
        unsafe {
            SDL_GetWindowSize(window.0, &mut current_w, &mut current_h);
        }
        if current_w > 0 && current_h > 0 && (current_w != width || current_h != height) {
            width = current_w.max(options.min_width);
            height = current_h.max(options.min_height);
            dirty = true;
            let result = on_event(SdlInputEvent::Resized { width, height });
            dirty |= result.dirty;
            running &= !result.quit;
        }

        if dirty {
            render(&mut scene, &atlas, width, height, Some(&runtime));
            renderer.render(width, height, &scene);
            unsafe {
                SDL_GL_SwapWindow(window.0);
            }
            rendered_frames = rendered_frames.saturating_add(1);
            dirty = false;
        }
        if options.frames.is_some_and(|limit| rendered_frames >= limit) {
            running = false;
        } else if options.frames.is_some() {
            dirty = true;
        }
        unsafe {
            SDL_Delay(16);
        }
    }

    Ok(())
}

fn init_sdl() -> Result<Sdl, String> {
    let rc = unsafe { SDL_Init(SDL_INIT_VIDEO) };
    if rc == 0 { Ok(Sdl) } else { Err(sdl_error()) }
}

fn sdl_error() -> String {
    let ptr = unsafe { SDL_GetError() };
    if ptr.is_null() {
        return "unknown SDL error".to_string();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}
