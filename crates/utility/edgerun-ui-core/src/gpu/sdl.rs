use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::format;
use std::os::raw::{c_char, c_int, c_void};
use std::rc::Rc;
use std::string::String;
use std::string::ToString;

use super::gl::GlRenderer;
use super::{ui_key_from_sdl_key_sym, UiHostSession};
use super::{
    Color4, FontAtlas, GpuScene, HitKind, UiAction, UiEvent, UiKey, UiKeyModifiers, UiNode,
    UiPainter, UiRect, UiRuntimeState,
};

const SDL_INIT_VIDEO: u32 = 0x0000_0020;
const SDL_WINDOWPOS_CENTERED: c_int = 0x2fff_0000u32 as c_int;
const SDL_WINDOW_OPENGL: u32 = 0x0000_0002;
const SDL_WINDOW_SHOWN: u32 = 0x0000_0004;
const SDL_WINDOW_RESIZABLE: u32 = 0x0000_0020;
const SDL_QUIT: u32 = 0x100;
const SDL_WINDOWEVENT: u32 = 0x200;
const SDL_KEYDOWN: u32 = 0x300;
const SDL_TEXTINPUT: u32 = 0x303;
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
const SDL_SYSTEM_CURSOR_ARROW: c_int = 0;
const SDL_SYSTEM_CURSOR_IBEAM: c_int = 1;
const SDL_SYSTEM_CURSOR_SIZEWE: c_int = 7;
const SDL_SYSTEM_CURSOR_SIZENS: c_int = 8;
const SDL_SYSTEM_CURSOR_HAND: c_int = 11;
const KMOD_SHIFT: u16 = 0x0003;
const KMOD_CTRL: u16 = 0x00c0;

pub const SDL_KEY_ESCAPE: i32 = 27;
pub const SDL_KEY_BACKSPACE: i32 = 8;
pub const SDL_KEY_TAB: i32 = 9;
pub const SDL_KEY_ENTER: i32 = 13;

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
    Tick,
    KeyDown { key: i32, modifiers: UiKeyModifiers },
    TextInput { text: String },
    MouseWheel { y: f32 },
    MouseMotion { x: f32, y: f32 },
    MouseDown { x: f32, y: f32 },
    MouseUp { x: f32, y: f32 },
    UiAction { action: UiAction },
    Resized { width: i32, height: i32 },
}

#[repr(C)]
struct SDL_Window(c_void);
#[repr(C)]
struct SDL_Cursor(c_void);

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

    fn key_modifiers(&self) -> UiKeyModifiers {
        let raw = u16::from_ne_bytes([self.data[24], self.data[25]]);
        UiKeyModifiers {
            shift: raw & KMOD_SHIFT != 0,
            ctrl: raw & KMOD_CTRL != 0,
            ..UiKeyModifiers::default()
        }
    }

    fn text_input(&self) -> Option<String> {
        let bytes = &self.data[12..44];
        let len = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        if len == 0 {
            return None;
        }
        Some(String::from_utf8_lossy(&bytes[..len]).into_owned())
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
    fn SDL_CreateSystemCursor(id: c_int) -> *mut SDL_Cursor;
    fn SDL_SetCursor(cursor: *mut SDL_Cursor);
    fn SDL_FreeCursor(cursor: *mut SDL_Cursor);
    fn SDL_StartTextInput();
    fn SDL_StopTextInput();
    fn SDL_Delay(ms: u32);
}

struct Sdl;
struct Window(*mut SDL_Window);
struct GlContext(SdlGlContext);
struct Cursor(*mut SDL_Cursor);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SdlCursorKind {
    Arrow,
    Hand,
    Text,
    ResizeX,
    ResizeY,
}

struct CursorSet {
    arrow: Cursor,
    hand: Cursor,
    text: Cursor,
    resize_x: Cursor,
    resize_y: Cursor,
    active: SdlCursorKind,
}

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

impl Drop for Cursor {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { SDL_FreeCursor(self.0) };
        }
    }
}

impl CursorSet {
    fn new() -> Result<Self, String> {
        let arrow = create_system_cursor(SDL_SYSTEM_CURSOR_ARROW, "arrow")?;
        let hand = create_system_cursor(SDL_SYSTEM_CURSOR_HAND, "hand")?;
        let text = create_system_cursor(SDL_SYSTEM_CURSOR_IBEAM, "text")?;
        let resize_x = create_system_cursor(SDL_SYSTEM_CURSOR_SIZEWE, "resize-x")?;
        let resize_y = create_system_cursor(SDL_SYSTEM_CURSOR_SIZENS, "resize-y")?;
        unsafe { SDL_SetCursor(arrow.0) };
        Ok(Self {
            arrow,
            hand,
            text,
            resize_x,
            resize_y,
            active: SdlCursorKind::Arrow,
        })
    }

    fn set(&mut self, kind: SdlCursorKind) {
        if self.active == kind {
            return;
        }
        let cursor = match kind {
            SdlCursorKind::Arrow => self.arrow.0,
            SdlCursorKind::Hand => self.hand.0,
            SdlCursorKind::Text => self.text.0,
            SdlCursorKind::ResizeX => self.resize_x.0,
            SdlCursorKind::ResizeY => self.resize_y.0,
        };
        unsafe { SDL_SetCursor(cursor) };
        self.active = kind;
    }
}

fn create_system_cursor(id: c_int, label: &str) -> Result<Cursor, String> {
    let cursor = unsafe { SDL_CreateSystemCursor(id) };
    if cursor.is_null() {
        Err(format!(
            "SDL_CreateSystemCursor({label}) failed: {}",
            sdl_error()
        ))
    } else {
        Ok(Cursor(cursor))
    }
}

pub fn run_sdl_gl_window(
    options: SdlGlWindowOptions<'_>,
    on_event: impl FnMut(SdlInputEvent) -> SdlEventResult,
    layout: impl FnMut(i32, i32) -> UiNode,
) -> Result<(), String> {
    run_sdl_gl_layout_window(options, on_event, layout)
}

pub const fn map_sdl_key(key: i32) -> UiKey {
    ui_key_from_sdl_key_sym(key)
}

pub fn run_edgerun_shell_sdl_window(
    options: SdlGlWindowOptions<'_>,
    session: UiHostSession,
) -> Result<(), String> {
    let direct = unsafe { SdlDirectDrawCapability::new_unchecked() };
    let session = Rc::new(RefCell::new(session));
    let event_session = Rc::clone(&session);
    let render_session = Rc::clone(&session);
    run_sdl_gl_window_direct(
        options,
        direct,
        move |event| match event {
            SdlInputEvent::CloseRequested => SdlEventResult::quit(),
            SdlInputEvent::KeyDown { key, .. } if key == SDL_KEY_ESCAPE => SdlEventResult::quit(),
            SdlInputEvent::KeyDown { key, .. } => SdlEventResult {
                dirty: event_session
                    .borrow_mut()
                    .handle_combined_event(UiEvent::KeyDown {
                        key: map_sdl_key(key),
                    }),
                quit: false,
            },
            SdlInputEvent::TextInput { text } => SdlEventResult {
                dirty: event_session
                    .borrow_mut()
                    .handle_combined_event(UiEvent::TextInput(text)),
                quit: false,
            },
            SdlInputEvent::MouseWheel { y } => {
                let hover = {
                    let session = event_session.borrow();
                    session
                        .workspace
                        .focused_app
                        .and_then(|id| session.workspace.app(id))
                        .and_then(|app| app.runtime.hovered())
                };
                SdlEventResult {
                    dirty: event_session
                        .borrow_mut()
                        .handle_combined_event(UiEvent::Wheel {
                            x: hover.map(|hit| hit.x).unwrap_or(0.0),
                            y: hover.map(|hit| hit.y).unwrap_or(0.0),
                            delta_y: -y * 120.0,
                        }),
                    quit: false,
                }
            }
            SdlInputEvent::MouseMotion { x, y } => SdlEventResult {
                dirty: event_session
                    .borrow_mut()
                    .handle_combined_event(UiEvent::PointerMove { x, y }),
                quit: false,
            },
            SdlInputEvent::MouseDown { x, y } => SdlEventResult {
                dirty: event_session
                    .borrow_mut()
                    .handle_combined_event(UiEvent::PointerDown { x, y }),
                quit: false,
            },
            SdlInputEvent::MouseUp { x, y } => SdlEventResult {
                dirty: event_session
                    .borrow_mut()
                    .handle_combined_event(UiEvent::PointerUp { x, y }),
                quit: false,
            },
            SdlInputEvent::Resized { .. } => SdlEventResult::dirty(),
            SdlInputEvent::Tick | SdlInputEvent::UiAction { .. } => SdlEventResult::default(),
        },
        move |scene, atlas, width, height| {
            let mut session = render_session.borrow_mut();
            session.build_combined_frame(atlas, width as f32, height as f32);
            *scene = session.scene.clone();
        },
    )
}

fn cursor_for_scene_position(scene: &GpuScene, x: f32, y: f32) -> SdlCursorKind {
    scene
        .hit_test(x, y)
        .map(cursor_for_hit_kind)
        .unwrap_or(SdlCursorKind::Arrow)
}

fn cursor_for_hit_kind(hit: super::runtime::GpuHit) -> SdlCursorKind {
    match hit.kind {
        HitKind::Input | HitKind::TextArea | HitKind::Composer => SdlCursorKind::Text,
        HitKind::Slider => SdlCursorKind::ResizeX,
        HitKind::Scrollbar => SdlCursorKind::ResizeY,
        HitKind::Button
        | HitKind::Tab
        | HitKind::Toggle
        | HitKind::ListRow
        | HitKind::Checkbox
        | HitKind::Radio
        | HitKind::Select
        | HitKind::Breadcrumb
        | HitKind::TreeItem
        | HitKind::MenuItem
        | HitKind::TransactionRow
        | HitKind::Send
        | HitKind::WorkspaceTab
        | HitKind::WorkspaceClose
        | HitKind::WorkspaceSplit
        | HitKind::ShellLauncher
        | HitKind::AppLauncherItem
        | HitKind::Contact => SdlCursorKind::Hand,
    }
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
        SDL_StartTextInput();
    }
    let atlas = FontAtlas::load_inter(18.0)?;
    let renderer = unsafe { GlRenderer::new_current_context_with_font(&atlas)? };
    let mut cursors = CursorSet::new()?;
    let mut scene = GpuScene::new(options.clear);
    let mut running = true;
    let mut rendered_frames = 0u32;
    let mut runtime = UiRuntimeState::default();
    let mut pointer_x = 0.0_f32;
    let mut pointer_y = 0.0_f32;

    sync_window_size(
        window.0,
        options.min_width,
        options.min_height,
        &mut width,
        &mut height,
    );
    render(&mut scene, &atlas, width, height, Some(&runtime));
    renderer.render(width, height, &scene);
    unsafe {
        SDL_GL_SwapWindow(window.0);
    }
    rendered_frames = rendered_frames.saturating_add(1);
    let mut dirty = false;

    while running {
        let mut event = RawSdlEvent { data: [0; 56] };
        while unsafe { SDL_PollEvent(&mut event) } != 0 {
            let mapped = match event.event_type() {
                SDL_QUIT => Some(SdlInputEvent::CloseRequested),
                SDL_KEYDOWN => Some(SdlInputEvent::KeyDown {
                    key: event.key_sym(),
                    modifiers: event.key_modifiers(),
                })
                .inspect(|mapped| {
                    if let SdlInputEvent::KeyDown { key, .. } = mapped {
                        let action = runtime.handle_event(
                            &scene,
                            UiEvent::KeyDown {
                                key: map_sdl_key(*key),
                            },
                        );
                        if action != UiAction::None {
                            dirty = true;
                            let result = on_event(SdlInputEvent::UiAction { action });
                            dirty |= result.dirty;
                            running &= !result.quit;
                        }
                    }
                }),
                SDL_TEXTINPUT => {
                    if let Some(text) = event.text_input() {
                        let action = runtime.handle_event(&scene, UiEvent::TextInput(text.clone()));
                        if action != UiAction::None {
                            dirty = true;
                            let result = on_event(SdlInputEvent::UiAction { action });
                            dirty |= result.dirty;
                            running &= !result.quit;
                        }
                        Some(SdlInputEvent::TextInput { text })
                    } else {
                        None
                    }
                }
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
                    cursors.set(cursor_for_scene_position(&scene, pointer_x, pointer_y));
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

        if sync_window_size(
            window.0,
            options.min_width,
            options.min_height,
            &mut width,
            &mut height,
        ) {
            dirty = true;
            let result = on_event(SdlInputEvent::Resized { width, height });
            dirty |= result.dirty;
            running &= !result.quit;
        }

        let result = on_event(SdlInputEvent::Tick);
        dirty |= result.dirty;
        running &= !result.quit;

        if dirty {
            render(&mut scene, &atlas, width, height, Some(&runtime));
            cursors.set(cursor_for_scene_position(&scene, pointer_x, pointer_y));
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

    unsafe {
        SDL_StopTextInput();
    }

    Ok(())
}

fn sync_window_size(
    window: *mut SDL_Window,
    min_width: i32,
    min_height: i32,
    width: &mut i32,
    height: &mut i32,
) -> bool {
    let mut current_w = 0;
    let mut current_h = 0;
    unsafe {
        SDL_GetWindowSize(window, &mut current_w, &mut current_h);
    }
    if current_w <= 0 || current_h <= 0 {
        return false;
    }
    let next_w = current_w.max(min_width);
    let next_h = current_h.max(min_height);
    if next_w == *width && next_h == *height {
        return false;
    }
    *width = next_w;
    *height = next_h;
    true
}

fn init_sdl() -> Result<Sdl, String> {
    let rc = unsafe { SDL_Init(SDL_INIT_VIDEO) };
    if rc == 0 {
        Ok(Sdl)
    } else {
        Err(sdl_error())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::runtime::GpuHit;

    #[test]
    fn sdl_cursor_kind_follows_hit_semantics() {
        assert_eq!(
            cursor_for_hit_kind(GpuHit::new(HitKind::Input, 1, 0.0, 0.0, 10.0, 10.0)),
            SdlCursorKind::Text
        );
        assert_eq!(
            cursor_for_hit_kind(GpuHit::new(HitKind::TextArea, 1, 0.0, 0.0, 10.0, 10.0)),
            SdlCursorKind::Text
        );
        assert_eq!(
            cursor_for_hit_kind(GpuHit::new(HitKind::Slider, 1, 0.0, 0.0, 10.0, 10.0)),
            SdlCursorKind::ResizeX
        );
        assert_eq!(
            cursor_for_hit_kind(GpuHit::new(HitKind::Scrollbar, 1, 0.0, 0.0, 10.0, 10.0)),
            SdlCursorKind::ResizeY
        );
        assert_eq!(
            cursor_for_hit_kind(GpuHit::new(HitKind::Button, 1, 0.0, 0.0, 10.0, 10.0)),
            SdlCursorKind::Hand
        );
    }

    #[test]
    fn sdl_cursor_resets_to_arrow_without_hit() {
        let scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        assert_eq!(
            cursor_for_scene_position(&scene, 40.0, 40.0),
            SdlCursorKind::Arrow
        );
    }
}
