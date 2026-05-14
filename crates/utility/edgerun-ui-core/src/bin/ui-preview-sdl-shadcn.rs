use std::env;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

use edgerun_ui_core::gpu::{
    Color4, GpuRect, GpuScene, RectMode, UiPainter, UiRect,
    build_shadcn_component_preview_by_identifier, build_shadcn_demo_gallery, palette,
};

const SDL_INIT_VIDEO: u32 = 0x0000_0020;
const SDL_WINDOWPOS_CENTERED: c_int = 0x2fff_0000u32 as c_int;
const SDL_WINDOW_SHOWN: u32 = 0x0000_0004;
const SDL_WINDOW_RESIZABLE: u32 = 0x0000_0020;
const SDL_RENDERER_ACCELERATED: u32 = 0x0000_0002;
const SDL_RENDERER_PRESENTVSYNC: u32 = 0x0000_0004;
const SDL_TEXTUREACCESS_STREAMING: c_int = 1;
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
struct Window(*mut SDL_Window);
struct Renderer(*mut SDL_Renderer);
struct Texture(*mut SDL_Texture);

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
        eprintln!("ui-preview-sdl-shadcn: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let identifier = env::args().nth(1);
    let fb_width = 1440u32;
    let fb_height = 900u32;
    let pitch = fb_width * 4;
    let mut pixels = vec![0u8; (pitch * fb_height) as usize];
    let scene = build_scene(identifier.as_deref(), fb_width as f32, fb_height as f32)?;

    let _sdl = init_sdl()?;
    let title = match identifier.as_deref() {
        Some(identifier) => format!("EdgeRun shadcn Preview - {identifier}"),
        None => "EdgeRun shadcn Component Gallery".to_string(),
    };
    let title = CString::new(title).unwrap();
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

    rasterize_scene(&scene, &mut pixels, fb_width, fb_height);

    let mut running = true;
    while running {
        let mut event = SdlEvent { data: [0; 56] };
        while unsafe { SDL_PollEvent(&mut event as *mut SdlEvent) } != 0 {
            if event.event_type() == SDL_QUIT {
                running = false;
            }
        }

        unsafe {
            SDL_UpdateTexture(
                texture.0,
                ptr::null(),
                pixels.as_ptr() as *const c_void,
                pitch as c_int,
            );
            SDL_RenderClear(renderer.0);
            SDL_RenderCopy(renderer.0, texture.0, ptr::null(), ptr::null());
            SDL_RenderPresent(renderer.0);
            SDL_Delay(16);
        }
    }

    Ok(())
}

fn build_scene(identifier: Option<&str>, width: f32, height: f32) -> Result<GpuScene, String> {
    let root = match identifier {
        Some(identifier) if identifier != "gallery" && identifier != "all" => {
            build_shadcn_component_preview_by_identifier(identifier)
                .ok_or_else(|| format!("unknown shadcn component identifier: {identifier}"))?
        }
        _ => build_shadcn_demo_gallery(),
    };

    let mut scene = GpuScene::new(palette::BG);
    {
        let mut ui = UiPainter::new(&mut scene);
        root.render(
            &mut ui,
            UiRect {
                x: 24.0,
                y: 24.0,
                w: width - 48.0,
                h: height - 48.0,
            },
        );
    }
    Ok(scene)
}

fn init_sdl() -> Result<Sdl, String> {
    let rc = unsafe { SDL_Init(SDL_INIT_VIDEO) };
    if rc != 0 { Err(sdl_error()) } else { Ok(Sdl) }
}

fn rasterize_scene(scene: &GpuScene, pixels: &mut [u8], width: u32, height: u32) {
    fill_background(pixels, scene.clear);
    for rect in scene.rects() {
        rasterize_rect(*rect, pixels, width, height);
    }
}

fn fill_background(pixels: &mut [u8], color: Color4) {
    let [b, g, r, a] = color_bytes(color);
    for px in pixels.chunks_exact_mut(4) {
        px[0] = b;
        px[1] = g;
        px[2] = r;
        px[3] = a;
    }
}

fn rasterize_rect(rect: GpuRect, pixels: &mut [u8], width: u32, height: u32) {
    if rect.w <= 0.0 || rect.h <= 0.0 || rect.color.a <= 0.0 {
        return;
    }
    let x0 = rect.x.floor().max(0.0) as i32;
    let y0 = rect.y.floor().max(0.0) as i32;
    let x1 = (rect.x + rect.w).ceil().min(width as f32) as i32;
    let y1 = (rect.y + rect.h).ceil().min(height as f32) as i32;
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    for y in y0..y1 {
        for x in x0..x1 {
            let alpha = coverage(rect, x as f32 + 0.5, y as f32 + 0.5);
            if alpha <= 0.0 {
                continue;
            }
            let idx = ((y as u32 * width + x as u32) * 4) as usize;
            blend_pixel(
                &mut pixels[idx..idx + 4],
                rect.color.with_alpha(rect.color.a * alpha),
            );
        }
    }
}

fn coverage(rect: GpuRect, px: f32, py: f32) -> f32 {
    match rect.mode {
        RectMode::Fill => rounded_coverage(rect, px, py),
        RectMode::Shadow => {
            let mut shadow = rect;
            shadow.x -= rect.shadow * 0.35;
            shadow.y += rect.shadow * 0.18;
            shadow.w += rect.shadow * 0.7;
            shadow.h += rect.shadow * 0.7;
            shadow.radius += rect.shadow * 0.35;
            rounded_coverage(shadow, px, py) * 0.38
        }
        RectMode::Border => {
            let outer = rounded_coverage(rect, px, py);
            if outer <= 0.0 {
                return 0.0;
            }
            let inner = GpuRect {
                x: rect.x + 1.0,
                y: rect.y + 1.0,
                w: (rect.w - 2.0).max(0.0),
                h: (rect.h - 2.0).max(0.0),
                radius: (rect.radius - 1.0).max(0.0),
                ..rect
            };
            (outer - rounded_coverage(inner, px, py)).clamp(0.0, 1.0)
        }
    }
}

fn rounded_coverage(rect: GpuRect, px: f32, py: f32) -> f32 {
    let r = rect.radius.max(0.0).min(rect.w.min(rect.h) * 0.5);
    if r <= 0.0 {
        return 1.0;
    }
    let left = rect.x + r;
    let right = rect.x + rect.w - r;
    let top = rect.y + r;
    let bottom = rect.y + rect.h - r;
    let cx = px.clamp(left, right);
    let cy = py.clamp(top, bottom);
    let dx = px - cx;
    let dy = py - cy;
    let dist = (dx * dx + dy * dy).sqrt();
    (r + 0.75 - dist).clamp(0.0, 1.0)
}

fn blend_pixel(dst: &mut [u8], color: Color4) {
    let alpha = color.a.clamp(0.0, 1.0);
    let inv = 1.0 - alpha;
    dst[0] = ((color.b.clamp(0.0, 1.0) * 255.0 * alpha) + dst[0] as f32 * inv) as u8;
    dst[1] = ((color.g.clamp(0.0, 1.0) * 255.0 * alpha) + dst[1] as f32 * inv) as u8;
    dst[2] = ((color.r.clamp(0.0, 1.0) * 255.0 * alpha) + dst[2] as f32 * inv) as u8;
    dst[3] = 255;
}

fn color_bytes(color: Color4) -> [u8; 4] {
    [
        (color.b.clamp(0.0, 1.0) * 255.0) as u8,
        (color.g.clamp(0.0, 1.0) * 255.0) as u8,
        (color.r.clamp(0.0, 1.0) * 255.0) as u8,
        (color.a.clamp(0.0, 1.0) * 255.0) as u8,
    ]
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
