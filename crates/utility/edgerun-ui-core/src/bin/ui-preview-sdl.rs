use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::time::{Duration, Instant};

use edgerun_ui_core::visual::demo_dashboard_polished;
use edgerun_ui_core::{DashboardState, Painter, EDGERUN_DARK};

const SDL_INIT_VIDEO: u32 = 0x0000_0020;
const SDL_WINDOWPOS_CENTERED: c_int = 0x2fff_0000u32 as c_int;
const SDL_WINDOW_SHOWN: u32 = 0x0000_0004;
const SDL_WINDOW_RESIZABLE: u32 = 0x0000_0020;
const SDL_RENDERER_ACCELERATED: u32 = 0x0000_0002;
const SDL_RENDERER_PRESENTVSYNC: u32 = 0x0000_0004;
const SDL_TEXTUREACCESS_STREAMING: c_int = 1;

// SDL_PIXELFORMAT_ARGB8888. On little-endian machines this is byte-compatible
// with the XRGB/BGRA buffers used by the compositor path: B, G, R, A.
const SDL_PIXELFORMAT_ARGB8888: u32 = 0x1636_2004;
const SDL_QUIT: u32 = 0x100;

#[repr(C)]
struct SDL_Window(c_void);
#[repr(C)]
struct SDL_Renderer(c_void);
#[repr(C)]
struct SDL_Texture(c_void);

#[link(name = "SDL2")]
unsafe extern "C" {
    fn SDL_Init(flags: u32) -> c_int;
    fn SDL_Quit();
    fn SDL_GetError() -> *const c_char;

    fn SDL_CreateWindow(
        title: *const c_char,
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
        flags: u32,
    ) -> *mut SDL_Window;
    fn SDL_DestroyWindow(window: *mut SDL_Window);

    fn SDL_CreateRenderer(window: *mut SDL_Window, index: c_int, flags: u32) -> *mut SDL_Renderer;
    fn SDL_DestroyRenderer(renderer: *mut SDL_Renderer);

    fn SDL_CreateTexture(
        renderer: *mut SDL_Renderer,
        format: u32,
        access: c_int,
        w: c_int,
        h: c_int,
    ) -> *mut SDL_Texture;
    fn SDL_DestroyTexture(texture: *mut SDL_Texture);

    fn SDL_UpdateTexture(
        texture: *mut SDL_Texture,
        rect: *const c_void,
        pixels: *const c_void,
        pitch: c_int,
    ) -> c_int;
    fn SDL_RenderClear(renderer: *mut SDL_Renderer) -> c_int;
    fn SDL_RenderCopy(
        renderer: *mut SDL_Renderer,
        texture: *mut SDL_Texture,
        srcrect: *const c_void,
        dstrect: *const c_void,
    ) -> c_int;
    fn SDL_RenderPresent(renderer: *mut SDL_Renderer);
    fn SDL_PollEvent(event: *mut SdlEvent) -> c_int;
    fn SDL_Delay(ms: u32);
}

// SDL_Event is a union. 56 bytes is the public SDL2 x86_64 size and is enough
// for the first u32 type field used here.
#[repr(C)]
struct SdlEvent {
    data: [u8; 56],
}

impl SdlEvent {
    fn event_type(&self) -> u32 {
        u32::from_ne_bytes([self.data[0], self.data[1], self.data[2], self.data[3]])
    }
}

struct Sdl;

impl Sdl {
    fn init() -> Result<Self, String> {
        let rc = unsafe { SDL_Init(SDL_INIT_VIDEO) };
        if rc != 0 {
            Err(sdl_error())
        } else {
            Ok(Self)
        }
    }
}

impl Drop for Sdl {
    fn drop(&mut self) {
        unsafe { SDL_Quit() };
    }
}

struct Window(*mut SDL_Window);
struct Renderer(*mut SDL_Renderer);
struct Texture(*mut SDL_Texture);

impl Drop for Window {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { SDL_DestroyWindow(self.0) };
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { SDL_DestroyRenderer(self.0) };
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { SDL_DestroyTexture(self.0) };
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("ui-preview-sdl: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let _sdl = Sdl::init()?;

    let fb_width = 960u32;
    let fb_height = 540u32;
    let pitch = fb_width * 4;
    let mut pixels = vec![0u8; (pitch * fb_height) as usize];

    let title = CString::new("EdgeRun UI Core SDL Preview").unwrap();
    let window = Window(unsafe {
        SDL_CreateWindow(
            title.as_ptr(),
            SDL_WINDOWPOS_CENTERED,
            SDL_WINDOWPOS_CENTERED,
            fb_width as c_int,
            fb_height as c_int,
            SDL_WINDOW_SHOWN | SDL_WINDOW_RESIZABLE,
        )
    });
    if window.0.is_null() {
        return Err(format!("SDL_CreateWindow failed: {}", sdl_error()));
    }

    let renderer = Renderer(unsafe {
        SDL_CreateRenderer(
            window.0,
            -1,
            SDL_RENDERER_ACCELERATED | SDL_RENDERER_PRESENTVSYNC,
        )
    });
    if renderer.0.is_null() {
        return Err(format!("SDL_CreateRenderer failed: {}", sdl_error()));
    }

    let texture = Texture(unsafe {
        SDL_CreateTexture(
            renderer.0,
            SDL_PIXELFORMAT_ARGB8888,
            SDL_TEXTUREACCESS_STREAMING,
            fb_width as c_int,
            fb_height as c_int,
        )
    });
    if texture.0.is_null() {
        return Err(format!("SDL_CreateTexture failed: {}", sdl_error()));
    }

    let started = Instant::now();
    let mut running = true;
    while running {
        let mut event = SdlEvent { data: [0; 56] };
        while unsafe { SDL_PollEvent(&mut event as *mut SdlEvent) } != 0 {
            if event.event_type() == SDL_QUIT {
                running = false;
            }
        }

        let elapsed_ms = started.elapsed().as_millis() as u32;
        let cpu = 80 + ((elapsed_ms / 16) % 640) as u16;
        let ram = 32 + ((elapsed_ms / 1000) % 64);

        {
            let mut painter = Painter {
                pixels: &mut pixels,
                width: fb_width,
                height: fb_height,
                pitch,
            };
            demo_dashboard_polished(
                &mut painter,
                DashboardState {
                    title: "EdgeRun",
                    subtitle: "rounded / antialiased / soft-shadow CPU UI",
                    node_status: "preview",
                    cpu_permille: cpu.min(1000),
                    memory_mb: ram,
                    network_status: "local",
                },
                EDGERUN_DARK,
            );
        }

        let rc = unsafe {
            SDL_UpdateTexture(
                texture.0,
                ptr::null(),
                pixels.as_ptr() as *const c_void,
                pitch as c_int,
            )
        };
        if rc != 0 {
            return Err(format!("SDL_UpdateTexture failed: {}", sdl_error()));
        }

        unsafe {
            SDL_RenderClear(renderer.0);
            SDL_RenderCopy(renderer.0, texture.0, ptr::null(), ptr::null());
            SDL_RenderPresent(renderer.0);
            SDL_Delay(8);
        }

        // Avoid spinning if vsync is unavailable.
        std::thread::sleep(Duration::from_millis(1));
    }

    Ok(())
}

fn sdl_error() -> String {
    let ptr = unsafe { SDL_GetError() };
    if ptr.is_null() {
        return "unknown SDL error".to_string();
    }
    unsafe { std::ffi::CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}
