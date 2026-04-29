//! Edgerun unikernel - bare shell with networking

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(target_arch = "xtensa", feature(asm_experimental_arch))]
#![cfg_attr(not(target_os = "none"), allow(dead_code, unused_imports))]

extern crate alloc;
extern crate edgerun_dhcp;
extern crate edgerun_http;
extern crate edgerun_oci;
extern crate edgerun_platform;
extern crate edgerun_rt as rt;
#[cfg(target_arch = "x86_64")]
extern crate edgerun_rtl8125;
extern crate edgerun_tftp;
extern crate edgerun_tpm;
#[cfg(target_arch = "x86_64")]
extern crate edgerun_virtio;

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
mod edgerun_layout {
    #[derive(Clone, Copy)]
    pub struct Color {
        pub r: u8,
        pub g: u8,
        pub b: u8,
        pub a: u8,
    }

    impl Color {
        pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
            Self { r, g, b, a: 255 }
        }
    }

    #[derive(Clone, Copy)]
    pub enum EmbeddedCommand<'a> {
        FillRect {
            x: u32,
            y: u32,
            w: u32,
            h: u32,
            color: Color,
        },
        RoundedRect {
            x: u32,
            y: u32,
            w: u32,
            h: u32,
            color: Color,
            radius: u32,
        },
        Text {
            x: u32,
            y: u32,
            text: &'a str,
            color: Color,
            font_size: u32,
        },
    }

    #[allow(dead_code)]
    pub enum UiRenderCommand<'a> {
        FillRect {
            x: u32,
            y: u32,
            w: u32,
            h: u32,
            color: Color,
        },
        StrokeRect {
            x: u32,
            y: u32,
            w: u32,
            h: u32,
            color: Color,
            style: u32,
            thickness: u32,
        },
        RoundedRect {
            x: u32,
            y: u32,
            w: u32,
            h: u32,
            color: Color,
            radius: u32,
        },
        Text {
            x: u32,
            y: u32,
            text: &'a str,
            color: Color,
            font_size: f32,
        },
    }
}
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    any(feature = "esp32s3-wifi-blob", feature = "esp32s3-wifi-mmio")
))]
extern crate edgerun_wifi;

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
const DISPLAY_CONSOLE_LINES: usize = 8;
#[cfg(all(target_arch = "xtensa", target_os = "none"))]
const DISPLAY_CONSOLE_COLS: usize = 38;

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
struct DisplayConsole {
    lines: [[u8; DISPLAY_CONSOLE_COLS]; DISPLAY_CONSOLE_LINES],
    lens: [usize; DISPLAY_CONSOLE_LINES],
    next: usize,
    count: usize,
    dirty: bool,
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl DisplayConsole {
    const fn new() -> Self {
        Self {
            lines: [[0; DISPLAY_CONSOLE_COLS]; DISPLAY_CONSOLE_LINES],
            lens: [0; DISPLAY_CONSOLE_LINES],
            next: 0,
            count: 0,
            dirty: true,
        }
    }

    fn push(&mut self, message: &str) {
        let bytes = message.as_bytes();
        let mut offset = 0;
        while offset < bytes.len() {
            let end = next_line_end(bytes, offset);
            self.push_bytes(&bytes[offset..end]);
            offset = end;
            while offset < bytes.len() && (bytes[offset] == b'\n' || bytes[offset] == b'\r') {
                offset += 1;
            }
        }
        if bytes.is_empty() {
            self.push_bytes(b"");
        }
    }

    fn push_bytes(&mut self, bytes: &[u8]) {
        let line = &mut self.lines[self.next];
        let len = bytes.len().min(DISPLAY_CONSOLE_COLS);
        let mut i = 0;
        while i < len {
            line[i] = sanitize_console_byte(bytes[i]);
            i += 1;
        }
        while i < DISPLAY_CONSOLE_COLS {
            line[i] = 0;
            i += 1;
        }
        self.lens[self.next] = len;
        self.next = (self.next + 1) % DISPLAY_CONSOLE_LINES;
        self.count = (self.count + 1).min(DISPLAY_CONSOLE_LINES);
        self.dirty = true;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
struct DisplayConsoleCell(core::cell::UnsafeCell<DisplayConsole>);

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
unsafe impl Sync for DisplayConsoleCell {}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl DisplayConsoleCell {
    const fn new() -> Self {
        Self(core::cell::UnsafeCell::new(DisplayConsole::new()))
    }

    fn with<R>(&self, f: impl FnOnce(&mut DisplayConsole) -> R) -> R {
        unsafe { f(&mut *self.0.get()) }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
static DISPLAY_CONSOLE: DisplayConsoleCell = DisplayConsoleCell::new();

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
#[derive(Clone, Copy)]
struct TouchFrameStats {
    frames: u32,
    total_us: u64,
    max_us: u32,
    last_us: u32,
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl TouchFrameStats {
    const fn new() -> Self {
        Self {
            frames: 0,
            total_us: 0,
            max_us: 0,
            last_us: 0,
        }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
struct TouchFrameStatsCell(core::cell::UnsafeCell<TouchFrameStats>);

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
unsafe impl Sync for TouchFrameStatsCell {}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl TouchFrameStatsCell {
    const fn new() -> Self {
        Self(core::cell::UnsafeCell::new(TouchFrameStats::new()))
    }

    fn with<R>(&self, f: impl FnOnce(&mut TouchFrameStats) -> R) -> R {
        unsafe { f(&mut *self.0.get()) }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
static TOUCH_FRAME_STATS: TouchFrameStatsCell = TouchFrameStatsCell::new();

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn display_console_log(message: &str) {
    DISPLAY_CONSOLE.with(|console| console.push(message));
}

#[cfg(not(all(target_arch = "xtensa", target_os = "none")))]
fn display_console_log(_message: &str) {}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn next_line_end(bytes: &[u8], offset: usize) -> usize {
    let mut end = offset;
    while end < bytes.len() && end - offset < DISPLAY_CONSOLE_COLS {
        if bytes[end] == b'\n' || bytes[end] == b'\r' {
            break;
        }
        end += 1;
    }
    end
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn sanitize_console_byte(byte: u8) -> u8 {
    if (0x20..=0x7e).contains(&byte) {
        byte
    } else {
        b'?'
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
#[repr(C, packed)]
struct EspAppDesc {
    magic_word: u32,
    secure_version: u32,
    reserv1: [u32; 2],
    version: [u8; 32],
    project_name: [u8; 32],
    time: [u8; 16],
    date: [u8; 16],
    idf_ver: [u8; 32],
    app_elf_sha256: [u8; 32],
    min_efuse_blk_rev_full: u16,
    max_efuse_blk_rev_full: u16,
    mmu_page_size: u8,
    reserv3: [u8; 3],
    reserv2: [u32; 18],
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
const UI_HTML: &str = r#"
<main class="remote">
  <section class="status">
    <div class="label">TCL AC</div>
    <div class="temp">24</div>
    <div class="mode">Cool - Auto fan</div>
  </section>
  <section class="controls">
    <button class="power">Power</button>
    <button>Mode</button>
    <button>Fan</button>
    <button>Swing</button>
  </section>
</main>
"#;

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
const UI_CSS: &str = r#"
main { display: block; width: 320px; min-height: 480px; background-color: #f8fafc; color: #101828; padding: 16px; }
.status { display: block; background-color: #0f766e; color: white; border-radius: 14px; padding: 18px; margin-bottom: 14px; }
.label { display: block; font-size: 16px; margin-bottom: 8px; }
.temp { display: block; font-size: 72px; line-height: 1.0; margin-bottom: 8px; }
.mode { display: block; font-size: 18px; }
.controls { display: flex; flex-wrap: wrap; }
button { display: block; width: 132px; height: 72px; margin-right: 8px; margin-bottom: 10px; background-color: #e2e8f0; color: #0f172a; border-radius: 10px; padding: 18px; font-size: 18px; }
.power { background-color: #dc2626; color: white; }
"#;

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn render_initial_ui() {
    #[cfg(feature = "html-ui")]
    render_html_ui(None);
    #[cfg(not(feature = "html-ui"))]
    render_debug_pattern(None);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn render_console_if_dirty(touch: Option<(u16, u16)>) {
    let dirty = DISPLAY_CONSOLE.with(|console| {
        let dirty = console.dirty;
        console.dirty = false;
        dirty
    });
    if dirty {
        match touch {
            Some(touch) => render_touch_ui(touch),
            None => render_initial_ui(),
        }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn render_touch_ui(touch: (u16, u16)) {
    #[cfg(feature = "html-ui")]
    render_html_ui(Some(touch));
    #[cfg(not(feature = "html-ui"))]
    render_debug_pattern(Some(touch));
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
#[derive(Clone, Copy)]
struct TouchOverlay {
    center: (u16, u16),
    alpha: u8,
    fade_started_at: Option<u64>,
    last_frame_at: u64,
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
#[derive(Clone, Copy)]
struct TouchTracker {
    active: Option<(u16, u16)>,
    last_seen_at: u64,
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl TouchTracker {
    const fn new() -> Self {
        Self {
            active: None,
            last_seen_at: 0,
        }
    }

    fn update(&mut self, sample: Option<(u16, u16)>, now: u64) -> Option<(u16, u16)> {
        if let Some(sample) = sample {
            self.active = Some(sample);
            self.last_seen_at = now;
            return self.active;
        }

        if self.active.is_some()
            && now.saturating_sub(self.last_seen_at) < TOUCH_RELEASE_GRACE_TICKS
        {
            return self.active;
        }

        self.active = None;
        None
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl TouchOverlay {
    fn pressed(center: (u16, u16), now: u64) -> Self {
        Self {
            center,
            alpha: TOUCH_OVERLAY_ALPHA,
            fade_started_at: None,
            last_frame_at: now,
        }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
const TOUCH_OVERLAY_RADIUS: u16 = 22;
#[cfg(all(target_arch = "xtensa", target_os = "none"))]
const TOUCH_OVERLAY_ALPHA: u8 = 96;
#[cfg(all(target_arch = "xtensa", target_os = "none"))]
const TOUCH_FADE_TICKS: u64 = 160_000_000;
#[cfg(all(target_arch = "xtensa", target_os = "none"))]
const TOUCH_FRAME_TICKS: u64 = 2_666_666;
#[cfg(all(target_arch = "xtensa", target_os = "none"))]
const TOUCH_RELEASE_GRACE_TICKS: u64 = 12_800_000;

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn render_touch_overlay_step(
    overlay: &mut Option<TouchOverlay>,
    touch: Option<(u16, u16)>,
    now: u64,
) {
    match (touch, *overlay) {
        (Some(center), Some(mut current)) => {
            if current.center != center || current.fade_started_at.is_some() {
                let previous_center = current.center;
                current.center = center;
                current.alpha = TOUCH_OVERLAY_ALPHA;
                current.fade_started_at = None;
                if now.saturating_sub(current.last_frame_at) >= TOUCH_FRAME_TICKS {
                    current.last_frame_at = now;
                    render_touch_overlay_band(Some(previous_center), current.center, current.alpha);
                }
                *overlay = Some(current);
            }
        }
        (Some(center), None) => {
            let current = TouchOverlay::pressed(center, now);
            render_touch_overlay_band(None, current.center, current.alpha);
            *overlay = Some(current);
        }
        (None, Some(mut current)) => {
            let fade_started_at = match current.fade_started_at {
                Some(start) => start,
                None => {
                    current.fade_started_at = Some(now);
                    current.last_frame_at = now.saturating_sub(TOUCH_FRAME_TICKS);
                    now
                }
            };

            if now.saturating_sub(current.last_frame_at) < TOUCH_FRAME_TICKS {
                *overlay = Some(current);
                return;
            }

            let elapsed = now.saturating_sub(fade_started_at);
            current.last_frame_at = now;
            if elapsed >= TOUCH_FADE_TICKS {
                render_touch_overlay_band(None, current.center, 0);
                *overlay = None;
                return;
            }

            let remaining = TOUCH_FADE_TICKS - elapsed;
            current.alpha = ((TOUCH_OVERLAY_ALPHA as u64 * remaining) / TOUCH_FADE_TICKS) as u8;
            render_touch_overlay_band(None, current.center, current.alpha);
            *overlay = Some(current);
        }
        (None, None) => {}
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn render_touch_overlay_band(previous: Option<(u16, u16)>, center: (u16, u16), alpha: u8) {
    let _ = previous;
    let started = edgerun_platform::timer::timer_ticks();
    #[cfg(feature = "html-ui")]
    {
        render_html_ui_with_overlay(Some(center), alpha);
    }
    #[cfg(not(feature = "html-ui"))]
    {
        unsafe {
            edgerun_platform::esp32s3::Jc3248w535Display::draw_rgb565_with(320, 480, |x, y| {
                touch_overlay_pixel(x, y, center, alpha)
            });
        }
    }
    let elapsed_us = edgerun_platform::timer::ticks_to_us(
        edgerun_platform::timer::timer_ticks().saturating_sub(started),
    ) as u32;
    TOUCH_FRAME_STATS.with(|stats| {
        stats.frames = stats.frames.saturating_add(1);
        stats.total_us = stats.total_us.saturating_add(elapsed_us as u64);
        stats.last_us = elapsed_us;
        if elapsed_us > stats.max_us {
            stats.max_us = elapsed_us;
        }
    });
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn touch_overlay_pixel(x: u16, y: u16, center: (u16, u16), alpha: u8) -> u16 {
    let base = ui_base_pixel(x, y);
    touch_overlay_over_base(base, x, y, center, alpha)
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn touch_overlay_over_base(base: u16, x: u16, y: u16, center: (u16, u16), alpha: u8) -> u16 {
    if alpha == 0 {
        return base;
    }

    let dx = x.abs_diff(center.0) as u32;
    let dy = y.abs_diff(center.1) as u32;
    let radius = TOUCH_OVERLAY_RADIUS as u32;
    let distance2 = dx * dx + dy * dy;
    let radius2 = radius * radius;
    if distance2 > radius2 {
        return base;
    }

    let edge = radius2 - distance2;
    let local_alpha = ((alpha as u32 * edge) / radius2) as u8;
    blend_rgb565(base, rgb565(255, 40, 32), local_alpha)
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn blend_rgb565(base: u16, overlay: u16, alpha: u8) -> u16 {
    let inv = 255u16 - alpha as u16;
    let br = (base >> 11) & 0x1f;
    let bg = (base >> 5) & 0x3f;
    let bb = base & 0x1f;
    let or = (overlay >> 11) & 0x1f;
    let og = (overlay >> 5) & 0x3f;
    let ob = overlay & 0x1f;
    let r = (br * inv + or * alpha as u16) / 255;
    let g = (bg * inv + og * alpha as u16) / 255;
    let b = (bb * inv + ob * alpha as u16) / 255;
    (r << 11) | (g << 5) | b
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
struct Jc3248w535TouchCapability {
    was_down: bool,
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl Jc3248w535TouchCapability {
    const fn new() -> Self {
        Self { was_down: false }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl edgerun_capabilities::CapabilityProvider for Jc3248w535TouchCapability {
    fn descriptor(&self) -> edgerun_capabilities::CapabilityDescriptor {
        edgerun_input::default_input_descriptor("jc3248w535", "touch")
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl edgerun_input::InputDevice for Jc3248w535TouchCapability {
    fn input_info(
        &self,
    ) -> Result<edgerun_input::InputDeviceInfo, edgerun_capabilities::CapabilityError> {
        Ok(edgerun_input::InputDeviceInfo {
            provider: alloc::string::String::from("jc3248w535"),
            instance_id: alloc::string::String::from("touch"),
            display_name: alloc::string::String::from("JC3248W535 capacitive touch"),
            kind: edgerun_input::InputDeviceKind::Touch,
            event_node: alloc::string::String::from("i2c://axs15231b-touch"),
            physical_path: Some(alloc::string::String::from("esp32s3/i2c0")),
            unique_id: None,
        })
    }

    fn read_events(
        &mut self,
        max_events: usize,
    ) -> Result<
        alloc::vec::Vec<edgerun_input::InputEventRecord>,
        edgerun_capabilities::CapabilityError,
    > {
        edgerun_input::validate_event_read_request(max_events)?;
        let now = edgerun_platform::timer::ticks_to_us(edgerun_platform::timer::timer_ticks());
        let timestamp_sec = (now / 1_000_000) as i64;
        let timestamp_usec = (now % 1_000_000) as i64;
        let mut events = alloc::vec::Vec::new();
        let point = unsafe { edgerun_platform::esp32s3::Jc3248w535Touch::read_point() };
        if let Some(point) = point {
            if !self.was_down && events.len() < max_events {
                events.push(input_event(
                    timestamp_sec,
                    timestamp_usec,
                    edgerun_input::InputEventKind::Key,
                    330,
                    1,
                ));
            }
            if events.len() < max_events {
                events.push(input_event(
                    timestamp_sec,
                    timestamp_usec,
                    edgerun_input::InputEventKind::AbsoluteMotion,
                    0,
                    point.x as i32,
                ));
            }
            if events.len() < max_events {
                events.push(input_event(
                    timestamp_sec,
                    timestamp_usec,
                    edgerun_input::InputEventKind::AbsoluteMotion,
                    1,
                    point.y as i32,
                ));
            }
            self.was_down = true;
        } else if self.was_down {
            events.push(input_event(
                timestamp_sec,
                timestamp_usec,
                edgerun_input::InputEventKind::Key,
                330,
                0,
            ));
            self.was_down = false;
        }
        Ok(events)
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn input_event(
    timestamp_sec: i64,
    timestamp_usec: i64,
    kind: edgerun_input::InputEventKind,
    code: u16,
    value: i32,
) -> edgerun_input::InputEventRecord {
    edgerun_input::InputEventRecord {
        timestamp_sec,
        timestamp_usec,
        kind,
        code,
        value,
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
struct Jc3248w535DisplayCapability;

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl edgerun_capabilities::CapabilityProvider for Jc3248w535DisplayCapability {
    fn descriptor(&self) -> edgerun_capabilities::CapabilityDescriptor {
        edgerun_display::default_display_descriptor("jc3248w535", "display")
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl edgerun_display::DisplayDevice for Jc3248w535DisplayCapability {
    fn display_info(
        &self,
    ) -> Result<edgerun_display::DisplayInfo, edgerun_capabilities::CapabilityError> {
        let mode = edgerun_display::DisplayMode {
            width: 320,
            height: 480,
            refresh_millihz: 60_000,
        };
        Ok(edgerun_display::DisplayInfo {
            provider: alloc::string::String::from("jc3248w535"),
            display_name: alloc::string::String::from("JC3248W535 LCD"),
            instance_id: alloc::string::String::from("display"),
            built_in: true,
            primary: true,
            current_mode: mode,
            modes: alloc::vec![mode],
            hdr_capable: false,
            touch_capable: true,
        })
    }

    fn present(
        &mut self,
        request: &edgerun_display::DisplayUpdateRequest,
    ) -> Result<(), edgerun_capabilities::CapabilityError> {
        edgerun_display::validate_display_update_request(request)?;
        render_initial_ui();
        Ok(())
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_board_capabilities(seq: u16) {
    display_console_log("ctl caps");
    rt::serial_mux::write_with_seq(
        rt::serial_mux::CHANNEL_CONTROL,
        seq,
        b"cap input provider=jc3248w535 instance=touch role=1\ncap display provider=jc3248w535 instance=display role=2\n",
    );
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_touch_frame_stats(seq: u16) {
    let stats = TOUCH_FRAME_STATS.with(|stats| *stats);
    let avg_us = if stats.frames == 0 {
        0
    } else {
        (stats.total_us / stats.frames as u64) as u32
    };
    let fps_x100 = if avg_us == 0 {
        0
    } else {
        100_000_000u32 / avg_us
    };
    let mut buf = [0u8; 96];
    let mut len = 0;
    append_bytes(&mut buf, &mut len, b"frames=");
    append_u32_dec(&mut buf, &mut len, stats.frames);
    append_bytes(&mut buf, &mut len, b" last_us=");
    append_u32_dec(&mut buf, &mut len, stats.last_us);
    append_bytes(&mut buf, &mut len, b" avg_us=");
    append_u32_dec(&mut buf, &mut len, avg_us);
    append_bytes(&mut buf, &mut len, b" max_us=");
    append_u32_dec(&mut buf, &mut len, stats.max_us);
    append_bytes(&mut buf, &mut len, b" fps_x100=");
    append_u32_dec(&mut buf, &mut len, fps_x100);
    append_bytes(&mut buf, &mut len, b"\n");
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn run_render_animation_demo() {
    const SIZE: u16 = 28;
    let mut previous: Option<(u16, u16)> = None;
    let mut frame = 0u16;
    while frame < 120 {
        let phase = frame % 112;
        let x = if phase < 56 {
            18 + phase * 4
        } else {
            18 + (111 - phase) * 4
        };
        let y = 82 + ((frame / 2) % 64);

        if let Some((px, py)) = previous {
            render_animation_rect(px, py, SIZE, false, frame);
        }
        render_animation_rect(x, y, SIZE, true, frame);
        previous = Some((x, y));

        let mut i = 0;
        while i < 350_000 {
            unsafe {
                core::arch::asm!("nop");
            }
            i += 1;
        }
        frame += 1;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn render_animation_rect(x: u16, y: u16, size: u16, active: bool, frame: u16) {
    unsafe {
        edgerun_platform::esp32s3::Jc3248w535Display::draw_rgb565_rect_with(
            x,
            y,
            size,
            size,
            |px, py| {
                if !active {
                    return ui_base_pixel(px, py);
                }

                let lx = px.saturating_sub(x);
                let ly = py.saturating_sub(y);
                if lx == 0 || ly == 0 || lx + 1 == size || ly + 1 == size {
                    return 0xffff;
                }
                rgb565(
                    (32 + ((frame * 3 + lx) & 0x7f)) as u8,
                    (160 + ((ly * 3) & 0x3f)) as u8,
                    (220 + ((frame + lx + ly) & 0x1f)) as u8,
                )
            },
        );
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn poll_serial_control(rx: &mut rt::serial_mux::Receiver<256>, last_touch: Option<(u16, u16)>) {
    let mut processed = 0;
    while processed < 8 {
        let frame = match rx.poll() {
            Ok(Some(frame)) => frame,
            Ok(None) => break,
            Err(_) => {
                rt::serial_mux::write(rt::serial_mux::CHANNEL_CONTROL, b"err decode\n");
                break;
            }
        };
        processed += 1;
        if frame.channel != rt::serial_mux::CHANNEL_CONTROL {
            continue;
        }
        match frame.payload() {
            b"ping" | b"ping\n" => {
                display_console_log("ctl ping");
                rt::serial_mux::write_with_seq(
                    rt::serial_mux::CHANNEL_CONTROL,
                    frame.seq,
                    b"pong\n",
                );
            }
            b"status" | b"status\n" => {
                display_console_log("ctl status");
                #[cfg(feature = "esp32s3-headless")]
                let status = b"ok board=jc3248w535 display=skipped touch=skipped transport=usb-serial-jtag\n";
                #[cfg(not(feature = "esp32s3-headless"))]
                let status = b"ok board=jc3248w535 display=up touch=up transport=usb-serial-jtag\n";
                rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, frame.seq, status);
                if let Some((x, y)) = last_touch {
                    write_touch_status(frame.seq, x, y);
                }
            }
            b"repaint" | b"repaint\n" => {
                display_console_log("ctl repaint");
                if let Some(touch) = last_touch {
                    render_touch_ui(touch);
                } else {
                    render_initial_ui();
                }
                rt::serial_mux::write_with_seq(
                    rt::serial_mux::CHANNEL_CONTROL,
                    frame.seq,
                    b"ok repaint\n",
                );
            }
            b"anim" | b"anim\n" => {
                display_console_log("ctl anim");
                run_render_animation_demo();
                rt::serial_mux::write_with_seq(
                    rt::serial_mux::CHANNEL_CONTROL,
                    frame.seq,
                    b"ok anim\n",
                );
            }
            b"caps" | b"caps\n" | b"capabilities" | b"capabilities\n" => {
                write_board_capabilities(frame.seq);
            }
            b"fps" | b"fps\n" => write_touch_frame_stats(frame.seq),
            b"bt" | b"bt\n" | b"ble" | b"ble\n" | b"bt start" | b"bt start\n" => {
                display_console_log("ctl bt start");
                write_bt_start(frame.seq);
            }
            b"btadv" | b"btadv\n" | b"ble adv" | b"ble adv\n" => {
                display_console_log("ctl bt adv");
                write_bt_adv(frame.seq);
            }
            b"btstats" | b"btstats\n" | b"ble stats" | b"ble stats\n" => {
                write_bt_stats(frame.seq);
            }
            b"wifistats" | b"wifistats\n" | b"wifi stats" | b"wifi stats\n" => {
                write_wifi_ap_stats(frame.seq)
            }
            b"wifi" | b"wifi\n" | b"wifi start" | b"wifi start\n" => {
                display_console_log("ctl wifi start");
                if try_start_esp32s3_wifi_ap() {
                    rt::serial_mux::write_with_seq(
                        rt::serial_mux::CHANNEL_CONTROL,
                        frame.seq,
                        b"ok wifi-start\n",
                    );
                } else {
                    rt::serial_mux::write_with_seq(
                        rt::serial_mux::CHANNEL_CONTROL,
                        frame.seq,
                        b"err wifi-start\n",
                    );
                }
            }
            b"wifiinit" | b"wifiinit\n" => write_wifi_init(frame.seq),
            b"wifi1" | b"wifi1\n" => write_wifi_debug_step(frame.seq, 1),
            b"wifi0" | b"wifi0\n" => write_wifi_debug_step(frame.seq, 0),
            b"wifi2" | b"wifi2\n" => write_wifi_debug_step(frame.seq, 2),
            b"wifi3" | b"wifi3\n" => write_wifi_debug_step(frame.seq, 3),
            b"wifi4" | b"wifi4\n" => write_wifi_debug_step(frame.seq, 4),
            b"wifi5" | b"wifi5\n" => write_wifi_debug_step(frame.seq, 5),
            b"wifi6" | b"wifi6\n" => write_wifi_debug_step(frame.seq, 6),
            b"wifi7" | b"wifi7\n" => write_wifi_debug_step(frame.seq, 7),
            b"wifi8" | b"wifi8\n" => write_wifi_debug_step(frame.seq, 8),
            b"wifi9" | b"wifi9\n" => write_wifi_debug_step(frame.seq, 9),
            b"wifi10" | b"wifi10\n" => write_wifi_debug_step(frame.seq, 10),
            b"wifi11" | b"wifi11\n" => write_wifi_debug_step(frame.seq, 11),
            b"wifi12" | b"wifi12\n" => write_wifi_debug_step(frame.seq, 12),
            b"wifi13" | b"wifi13\n" => write_wifi_debug_step(frame.seq, 13),
            b"wifi14" | b"wifi14\n" => write_wifi_debug_step(frame.seq, 14),
            b"wifi15" | b"wifi15\n" => write_wifi_debug_step(frame.seq, 15),
            b"wifi16" | b"wifi16\n" => write_wifi_debug_step(frame.seq, 16),
            b"wifi17" | b"wifi17\n" => write_wifi_debug_step(frame.seq, 17),
            b"wifi18" | b"wifi18\n" => write_wifi_debug_step(frame.seq, 18),
            b"wifi19" | b"wifi19\n" => write_wifi_debug_step(frame.seq, 19),
            b"wifi20" | b"wifi20\n" => write_wifi_debug_step(frame.seq, 20),
            b"wifi21" | b"wifi21\n" => write_wifi_debug_step(frame.seq, 21),
            b"wifi22" | b"wifi22\n" => write_wifi_debug_step(frame.seq, 22),
            b"wifi23" | b"wifi23\n" => write_wifi_debug_step(frame.seq, 23),
            b"wifi24" | b"wifi24\n" => write_wifi_debug_step(frame.seq, 24),
            b"wifi25" | b"wifi25\n" => write_wifi_debug_step(frame.seq, 25),
            b"wifi26" | b"wifi26\n" => write_wifi_debug_step(frame.seq, 26),
            b"wifi27" | b"wifi27\n" => write_wifi_debug_step(frame.seq, 27),
            b"wifi28" | b"wifi28\n" => write_wifi_debug_step(frame.seq, 28),
            b"wifi29" | b"wifi29\n" => write_wifi_debug_step(frame.seq, 29),
            b"wifi30" | b"wifi30\n" => write_wifi_debug_step(frame.seq, 30),
            b"wifi31" | b"wifi31\n" => write_wifi_debug_step(frame.seq, 31),
            b"wifi32" | b"wifi32\n" => write_wifi_debug_step(frame.seq, 32),
            b"wifi33" | b"wifi33\n" => write_wifi_debug_step(frame.seq, 33),
            b"wifi34" | b"wifi34\n" => write_wifi_debug_step(frame.seq, 34),
            b"wifi35" | b"wifi35\n" => write_wifi_debug_step(frame.seq, 35),
            b"wifi36" | b"wifi36\n" => write_wifi_debug_step(frame.seq, 36),
            b"wifi37" | b"wifi37\n" => write_wifi_debug_step(frame.seq, 37),
            b"wifi38" | b"wifi38\n" => write_wifi_debug_step(frame.seq, 38),
            b"wifi39" | b"wifi39\n" => write_wifi_debug_step(frame.seq, 39),
            b"wifi40" | b"wifi40\n" => write_wifi_debug_step(frame.seq, 40),
            b"wifirftest" | b"wifirftest\n" => write_wifi_debug_step(frame.seq, 40),
            b"wifi41" | b"wifi41\n" => write_wifi_debug_step(frame.seq, 41),
            b"wifirxgate" | b"wifirxgate\n" => write_wifi_debug_step(frame.seq, 41),
            b"wifi42" | b"wifi42\n" => write_wifi_debug_step(frame.seq, 42),
            b"wifirxbuf" | b"wifirxbuf\n" => write_wifi_debug_step(frame.seq, 42),
            b"wifi43" | b"wifi43\n" => write_wifi_debug_step(frame.seq, 43),
            b"wifirxaccept" | b"wifirxaccept\n" => write_wifi_debug_step(frame.seq, 43),
            b"wifi44" | b"wifi44\n" => write_wifi_debug_step(frame.seq, 44),
            b"wifirx2440" | b"wifirx2440\n" => write_wifi_debug_step(frame.seq, 44),
            b"wifi45" | b"wifi45\n" => write_wifi_debug_step(frame.seq, 45),
            b"wifirxeof" | b"wifirxeof\n" => write_wifi_debug_step(frame.seq, 45),
            b"wifi46" | b"wifi46\n" => write_wifi_debug_step(frame.seq, 46),
            b"wifirxwdev" | b"wifirxwdev\n" => write_wifi_debug_step(frame.seq, 46),
            b"wifi47" | b"wifi47\n" => write_wifi_debug_step(frame.seq, 47),
            b"wifimacflt" | b"wifimacflt\n" => write_wifi_debug_step(frame.seq, 47),
            b"wifi48" | b"wifi48\n" => write_wifi_debug_step(frame.seq, 48),
            b"wifirxper" | b"wifirxper\n" => write_wifi_debug_step(frame.seq, 48),
            b"wifi49" | b"wifi49\n" => write_wifi_debug_step(frame.seq, 49),
            b"wifirxpbus0" | b"wifirxpbus0\n" => write_wifi_debug_step(frame.seq, 49),
            b"wifi50" | b"wifi50\n" => write_wifi_debug_step(frame.seq, 50),
            b"wifirxpbus" | b"wifirxpbus\n" => write_wifi_debug_step(frame.seq, 50),
            b"wifi51" | b"wifi51\n" => write_wifi_debug_step(frame.seq, 51),
            b"wifiphyrx" | b"wifiphyrx\n" => write_wifi_debug_step(frame.seq, 51),
            b"wifi52" | b"wifi52\n" => write_wifi_debug_step(frame.seq, 52),
            b"wifipbusdbg" | b"wifipbusdbg\n" => write_wifi_debug_step(frame.seq, 52),
            b"wifi53" | b"wifi53\n" => write_wifi_debug_step(frame.seq, 53),
            b"wifiromrx" | b"wifiromrx\n" => write_wifi_debug_step(frame.seq, 53),
            b"wifi54" | b"wifi54\n" => write_wifi_debug_step(frame.seq, 54),
            b"wifiromphyrx" | b"wifiromphyrx\n" => write_wifi_debug_step(frame.seq, 54),
            b"wifi55" | b"wifi55\n" => write_wifi_debug_step(frame.seq, 55),
            b"wifidmaromrx" | b"wifidmaromrx\n" => write_wifi_debug_step(frame.seq, 55),
            b"wifi56" | b"wifi56\n" => write_wifi_debug_step(frame.seq, 56),
            b"wifidmaromphyrx" | b"wifidmaromphyrx\n" => write_wifi_debug_step(frame.seq, 56),
            b"wifi57" | b"wifi57\n" => write_wifi_debug_step(frame.seq, 57),
            b"wifirfchsave" | b"wifirfchsave\n" => write_wifi_debug_step(frame.seq, 57),
            b"wifi58" | b"wifi58\n" => write_wifi_debug_step(frame.seq, 58),
            b"wifirfchpre" | b"wifirfchpre\n" => write_wifi_debug_step(frame.seq, 58),
            b"wifi59" | b"wifi59\n" => write_wifi_debug_step(frame.seq, 59),
            b"wifirfchmode" | b"wifirfchmode\n" => write_wifi_debug_step(frame.seq, 59),
            b"wifi60" | b"wifi60\n" => write_wifi_debug_step(frame.seq, 60),
            b"wifirfchgainpre" | b"wifirfchgainpre\n" => write_wifi_debug_step(frame.seq, 60),
            b"wifi61" | b"wifi61\n" => write_wifi_debug_step(frame.seq, 61),
            b"wifirfchgainch" | b"wifirfchgainch\n" => write_wifi_debug_step(frame.seq, 61),
            b"wifi62" | b"wifi62\n" => write_wifi_debug_step(frame.seq, 62),
            b"wifirfchpost" | b"wifirfchpost\n" => write_wifi_debug_step(frame.seq, 62),
            b"wifi63" | b"wifi63\n" => write_wifi_debug_step(frame.seq, 63),
            b"wifirfchrestore" | b"wifirfchrestore\n" => write_wifi_debug_step(frame.seq, 63),
            b"wifi64" | b"wifi64\n" => write_wifi_debug_step(frame.seq, 64),
            b"wifirfchan6" | b"wifirfchan6\n" => write_wifi_debug_step(frame.seq, 64),
            b"wifi65" | b"wifi65\n" => write_wifi_debug_step(frame.seq, 65),
            b"wifiphyparam" | b"wifiphyparam\n" => write_wifi_debug_step(frame.seq, 65),
            b"wifi66" | b"wifi66\n" => write_wifi_debug_step(frame.seq, 66),
            b"wifirfchreg0" | b"wifirfchreg0\n" => write_wifi_debug_step(frame.seq, 66),
            b"wifi67" | b"wifi67\n" => write_wifi_debug_step(frame.seq, 67),
            b"wifitxgain0" | b"wifitxgain0\n" => write_wifi_debug_step(frame.seq, 67),
            b"wifi68" | b"wifi68\n" => write_wifi_debug_step(frame.seq, 68),
            b"wifigainwrite0" | b"wifigainwrite0\n" => write_wifi_debug_step(frame.seq, 68),
            b"wifi69" | b"wifi69\n" => write_wifi_debug_step(frame.seq, 69),
            b"wifigainflat" | b"wifigainflat\n" => write_wifi_debug_step(frame.seq, 69),
            b"wifi70" | b"wifi70\n" => write_wifi_debug_step(frame.seq, 70),
            b"wifirfsub06c" | b"wifirfsub06c\n" => write_wifi_debug_step(frame.seq, 70),
            b"wifi71" | b"wifi71\n" => write_wifi_debug_step(frame.seq, 71),
            b"wifirfsub054" | b"wifirfsub054\n" => write_wifi_debug_step(frame.seq, 71),
            b"wifi72" | b"wifi72\n" => write_wifi_debug_step(frame.seq, 72),
            b"wifirfsub0c4" | b"wifirfsub0c4\n" => write_wifi_debug_step(frame.seq, 72),
            b"wifi73" | b"wifi73\n" => write_wifi_debug_step(frame.seq, 73),
            b"wifirfsub080" | b"wifirfsub080\n" => write_wifi_debug_step(frame.seq, 73),
            b"wifi74" | b"wifi74\n" => write_wifi_debug_step(frame.seq, 74),
            b"wifirfchclone" | b"wifirfchclone\n" => write_wifi_debug_step(frame.seq, 74),
            b"wifich1" | b"wifich1\n" => write_wifi_debug_step(frame.seq, 23),
            b"wifich6" | b"wifich6\n" => write_wifi_debug_step(frame.seq, 24),
            b"wifich11" | b"wifich11\n" => write_wifi_debug_step(frame.seq, 25),
            b"wifistart" | b"wifistart\n" => write_wifi_debug_step(frame.seq, 26),
            b"wifistart1" | b"wifistart1\n" => write_wifi_debug_step(frame.seq, 34),
            b"wifistart6" | b"wifistart6\n" => write_wifi_debug_step(frame.seq, 35),
            b"wifistart11" | b"wifistart11\n" => write_wifi_debug_step(frame.seq, 36),
            b"wifirxenable" | b"wifirxenable\n" => write_wifi_debug_step(frame.seq, 37),
            b"wifirfon" | b"wifirfon\n" => write_wifi_debug_step(frame.seq, 38),
            b"wifimmio" | b"wifimmio\n" => write_wifi_mmio_regs(frame.seq),
            b"wifirx" | b"wifirx\n" => write_wifi_rx_scratch_regs(frame.seq),
            b"wifitx" | b"wifitx\n" => write_wifi_tx_regs(frame.seq),
            b"wifirate" | b"wifirate\n" => write_wifi_rate_regs(frame.seq),
            b"wificrypto" | b"wificrypto\n" => write_wifi_crypto_regs(frame.seq),
            b"wifiant" | b"wifiant\n" => write_wifi_antenna_regs(frame.seq),
            b"wifiphy" | b"wifiphy\n" => write_wifi_phy_regs(frame.seq),
            b"wifiregs" | b"wifiregs\n" => write_wifi_debug_regs(frame.seq),
            b"wififuns" | b"wififuns\n" => write_wifi_phy_fun_slots(frame.seq),
            _ => {
                display_console_log("ctl unknown");
                rt::serial_mux::write_with_seq(
                    rt::serial_mux::CHANNEL_CONTROL,
                    frame.seq,
                    b"err unknown-command\n",
                );
            }
        }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_wifi_debug_regs(seq: u16) {
    display_console_log("ctl wifiregs");
    let mut buf = [0u8; 160];
    let mut len = 0;
    append_wifi_debug_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_wifi_init(seq: u16) {
    display_console_log("ctl wifiinit");
    let ok = try_wifi_init_known_good();
    let mut buf = [0u8; 48];
    let mut len = 0;
    if ok {
        append_bytes(&mut buf, &mut len, b"ok wifi-init status=");
    } else {
        append_bytes(&mut buf, &mut len, b"err wifi-init status=");
    }
    append_i32(&mut buf, &mut len, wifi_debug_status());
    append_bytes(&mut buf, &mut len, b"\n");
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_wifi_phy_fun_slots(seq: u16) {
    display_console_log("ctl wififuns");
    let mut buf = [0u8; 640];
    let mut len = 0;
    append_wifi_phy_fun_slots(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_wifi_mmio_regs(seq: u16) {
    display_console_log("ctl wifimmio");
    let mut buf = [0u8; 640];
    let mut len = 0;
    append_wifi_mmio_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

fn write_wifi_tx_regs(seq: u16) {
    display_console_log("ctl wifitx");
    let mut buf = [0u8; 256];
    let mut len = 0;
    append_wifi_tx_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

fn write_wifi_rx_scratch_regs(seq: u16) {
    display_console_log("ctl wifirx");
    let mut buf = [0u8; 1024];
    let mut len = 0;
    append_wifi_rx_scratch_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

fn write_wifi_rate_regs(seq: u16) {
    display_console_log("ctl wifirate");
    let mut buf = [0u8; 160];
    let mut len = 0;
    append_wifi_rate_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

fn write_wifi_crypto_regs(seq: u16) {
    display_console_log("ctl wificrypto");
    let mut buf = [0u8; 192];
    let mut len = 0;
    append_wifi_crypto_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

fn write_wifi_antenna_regs(seq: u16) {
    display_console_log("ctl wifiant");
    let mut buf = [0u8; 128];
    let mut len = 0;
    append_wifi_antenna_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

fn write_wifi_phy_regs(seq: u16) {
    display_console_log("ctl wifiphy");
    let mut buf = [0u8; 704];
    let mut len = 0;
    append_wifi_phy_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_wifi_debug_step(seq: u16, step: u8) {
    display_console_log("ctl wifi step");
    let mut buf = [0u8; 48];
    let mut len = 0;
    if try_wifi_debug_step(step) {
        append_bytes(&mut buf, &mut len, b"ok wifi-step status=");
    } else {
        append_bytes(&mut buf, &mut len, b"err wifi-step status=");
    }
    append_u16(&mut buf, &mut len, wifi_debug_status() as u16);
    append_bytes(&mut buf, &mut len, b"\n");
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_wifi_ap_stats_inner(seq: u16, stats: edgerun_platform::esp32s3_wifi::Esp32s3WifiStats) {
    let mut buf = [0u8; 128];
    let mut len = 0;
    append_bytes(&mut buf, &mut len, b"wifi ap raw_rx=");
    append_u32_dec(&mut buf, &mut len, stats.raw_rx);
    append_bytes(&mut buf, &mut len, b" raw_tx=");
    append_u32_dec(&mut buf, &mut len, stats.raw_tx);
    append_bytes(&mut buf, &mut len, b" eth_rx=");
    append_u32_dec(&mut buf, &mut len, stats.eth_rx);
    append_bytes(&mut buf, &mut len, b" eth_tx=");
    append_u32_dec(&mut buf, &mut len, stats.eth_tx);
    append_bytes(&mut buf, &mut len, b" dropped_eth_rx=");
    append_u32_dec(&mut buf, &mut len, stats.dropped_eth_rx);
    append_bytes(&mut buf, &mut len, b" status=");
    append_i32(&mut buf, &mut len, wifi_debug_status());
    append_bytes(&mut buf, &mut len, b"\n");
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless"
))]
fn poll_headless_raw_control(buf: &mut [u8; 64], len: &mut usize) {
    while let Some(byte) =
        unsafe { edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_read_byte() }
    {
        if byte == b'\r' || byte == b'\n' {
            if *len != 0 {
                handle_headless_raw_command(&buf[..*len]);
                *len = 0;
            }
            continue;
        }
        if *len < buf.len() {
            buf[*len] = byte;
            *len += 1;
        } else {
            *len = 0;
            headless_raw_write(b"err line-too-long\n");
        }
    }
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless"
))]
fn handle_headless_raw_command(command: &[u8]) {
    match command {
        b"ping" => headless_raw_write(b"pong\n"),
        b"status" => {
            headless_raw_write(
                b"ok board=jc3248w535 display=skipped touch=skipped transport=usb-serial-jtag\n",
            );
        }
        b"wifi" | b"wifi start" => {
            if try_start_esp32s3_wifi_ap() {
                headless_raw_write(b"ok wifi-start\n");
            } else {
                headless_raw_write(b"err wifi-start\n");
            }
        }
        b"bt" | b"ble" | b"bt start" => write_headless_bt_start_raw(),
        b"btadv" | b"ble adv" => write_headless_bt_adv_raw(),
        b"btstats" | b"ble stats" => write_headless_bt_stats_raw(),
        b"wifistats" | b"wifi stats" => write_headless_wifi_stats_raw(),
        b"wifiinit" => write_headless_wifi_init_raw(),
        b"wifirx" => write_headless_wifi_rx_raw(),
        _ => {
            if let Some(step) = parse_headless_wifi_step(command) {
                write_headless_wifi_step_raw(step);
            } else {
                headless_raw_write(b"err unknown\n");
            }
        }
    }
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless"
))]
fn parse_headless_wifi_step(command: &[u8]) -> Option<u8> {
    let digits = match command {
        b"wifirftest" => return Some(40),
        b"wifirxgate" => return Some(41),
        b"wifirxbuf" => return Some(42),
        b"wifirxaccept" => return Some(43),
        b"wifirx2440" => return Some(44),
        b"wifirxeof" => return Some(45),
        b"wifirxwdev" => return Some(46),
        b"wifimacflt" => return Some(47),
        b"wifirxper" => return Some(48),
        b"wifirxpbus0" => return Some(49),
        b"wifirxpbus" => return Some(50),
        b"wifiphyrx" => return Some(51),
        b"wifipbusdbg" => return Some(52),
        b"wifiromrx" => return Some(53),
        b"wifiromphyrx" => return Some(54),
        b"wifidmaromrx" => return Some(55),
        b"wifidmaromphyrx" => return Some(56),
        b"wifirfchsave" => return Some(57),
        b"wifirfchpre" => return Some(58),
        b"wifirfchmode" => return Some(59),
        b"wifirfchgainpre" => return Some(60),
        b"wifirfchgainch" => return Some(61),
        b"wifirfchpost" => return Some(62),
        b"wifirfchrestore" => return Some(63),
        b"wifirfchan6" => return Some(64),
        b"wifiphyparam" => return Some(65),
        b"wifirfchreg0" => return Some(66),
        b"wifitxgain0" => return Some(67),
        b"wifigainwrite0" => return Some(68),
        b"wifigainflat" => return Some(69),
        b"wifirfsub06c" => return Some(70),
        b"wifirfsub054" => return Some(71),
        b"wifirfsub0c4" => return Some(72),
        b"wifirfsub080" => return Some(73),
        b"wifirfchclone" => return Some(74),
        b"wifich1" => return Some(23),
        b"wifich6" => return Some(24),
        b"wifich11" => return Some(25),
        b"wifistart" => return Some(26),
        b"wifistart1" => return Some(34),
        b"wifistart6" => return Some(35),
        b"wifistart11" => return Some(36),
        b"wifirxenable" => return Some(37),
        b"wifirfon" => return Some(38),
        _ => command.strip_prefix(b"wifi")?,
    };
    if digits.is_empty() {
        return None;
    }
    let mut value = 0u16;
    for byte in digits {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value
            .saturating_mul(10)
            .saturating_add((byte - b'0') as u16);
    }
    u8::try_from(value).ok()
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless"
))]
fn write_headless_wifi_init_raw() {
    if try_wifi_init_known_good() {
        headless_raw_write(b"ok wifi-init status=");
    } else {
        headless_raw_write(b"err wifi-init status=");
    }
    write_headless_status_suffix();
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless"
))]
fn write_headless_wifi_step_raw(step: u8) {
    if try_wifi_debug_step(step) {
        headless_raw_write(b"ok wifi-step ");
    } else {
        headless_raw_write(b"err wifi-step ");
    }
    let mut buf = [0u8; 48];
    let mut len = 0;
    append_bytes(&mut buf, &mut len, b"step=");
    append_u32_dec(&mut buf, &mut len, step as u32);
    append_bytes(&mut buf, &mut len, b" status=");
    append_i32(&mut buf, &mut len, wifi_debug_status());
    append_bytes(&mut buf, &mut len, b"\n");
    headless_raw_write(&buf[..len]);
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless"
))]
fn write_headless_wifi_rx_raw() {
    let mut buf = [0u8; 1024];
    let mut len = 0;
    append_wifi_rx_scratch_regs(&mut buf, &mut len);
    headless_raw_write(&buf[..len]);
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless"
))]
fn write_headless_status_suffix() {
    let mut buf = [0u8; 24];
    let mut len = 0;
    append_i32(&mut buf, &mut len, wifi_debug_status());
    append_bytes(&mut buf, &mut len, b"\n");
    headless_raw_write(&buf[..len]);
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless",
    feature = "esp32s3-wifi-blob"
))]
fn write_headless_wifi_stats_raw() {
    let mut buf = [0u8; 96];
    let mut len = 0;
    append_bytes(&mut buf, &mut len, b"wifi vendor ap ");
    if ESP32S3_VENDOR_WIFI_AP_STARTED.load(core::sync::atomic::Ordering::Acquire) {
        append_bytes(&mut buf, &mut len, b"started");
    } else {
        append_bytes(&mut buf, &mut len, b"stopped");
    }
    append_bytes(&mut buf, &mut len, b" status=");
    append_i32(&mut buf, &mut len, wifi_debug_status());
    append_bytes(&mut buf, &mut len, b"\n");
    headless_raw_write(&buf[..len]);
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn write_headless_wifi_stats_raw() {
    let mut buf = [0u8; 128];
    let mut len = 0;
    append_bytes(&mut buf, &mut len, b"wifi ap ");
    if !ESP32S3_WIFI_MMIO_AP_STARTED.load(core::sync::atomic::Ordering::Acquire) {
        append_bytes(&mut buf, &mut len, b"stopped");
    } else {
        let stats = unsafe { ESP32S3_WIFI_MMIO_AP.get().stats() };
        append_bytes(&mut buf, &mut len, b"raw_rx=");
        append_u32_dec(&mut buf, &mut len, stats.raw_rx);
        append_bytes(&mut buf, &mut len, b" raw_tx=");
        append_u32_dec(&mut buf, &mut len, stats.raw_tx);
        append_bytes(&mut buf, &mut len, b" eth_rx=");
        append_u32_dec(&mut buf, &mut len, stats.eth_rx);
        append_bytes(&mut buf, &mut len, b" eth_tx=");
        append_u32_dec(&mut buf, &mut len, stats.eth_tx);
        append_bytes(&mut buf, &mut len, b" dropped_eth_rx=");
        append_u32_dec(&mut buf, &mut len, stats.dropped_eth_rx);
    }
    append_bytes(&mut buf, &mut len, b" status=");
    append_i32(&mut buf, &mut len, wifi_debug_status());
    append_bytes(&mut buf, &mut len, b"\n");
    headless_raw_write(&buf[..len]);
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless",
    any(feature = "esp32s3-wifi-blob", feature = "esp32s3-wifi-mmio")
)))]
fn write_headless_wifi_stats_raw() {
    headless_raw_write(b"wifi ap backend disabled\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless"
))]
fn headless_raw_write(bytes: &[u8]) {
    unsafe { edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(bytes) };
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless"
)))]
fn headless_raw_write(_bytes: &[u8]) {}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
fn append_wifi_debug_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_blob::EspressifPromiscRadio::debug_regs();
    append_bytes(out, len, b"wifi regs pwc=0x");
    append_hex_u32(out, len, regs.rtc_dig_pwc);
    append_bytes(out, len, b" iso=0x");
    append_hex_u32(out, len, regs.rtc_dig_iso);
    append_bytes(out, len, b" clk=0x");
    append_hex_u32(out, len, regs.wifi_clk_en);
    append_bytes(out, len, b" rst=0x");
    append_hex_u32(out, len, regs.wifi_rst_en);
    append_bytes(out, len, b" mac=0x");
    append_hex_u32(out, len, regs.mac_reset_ctrl);
    append_bytes(out, len, b" funs=0x");
    append_hex_u32(out, len, regs.phy_funs);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_mmio_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_mac_regs();
    append_bytes(out, len, b"wifi mmio mac=0x");
    append_hex_u32(out, len, regs.mac_reset_ctrl);
    append_bytes(out, len, b" dma=0x");
    append_hex_u32(out, len, regs.dma_ctrl);
    append_bytes(out, len, b" rxp0=0x");
    append_hex_u32(out, len, regs.rx_policy0);
    append_bytes(out, len, b" rxp1=0x");
    append_hex_u32(out, len, regs.rx_policy1);
    append_bytes(out, len, b" rxp2=0x");
    append_hex_u32(out, len, regs.rx_policy2);
    append_bytes(out, len, b" rxp3=0x");
    append_hex_u32(out, len, regs.rx_policy3);
    append_bytes(out, len, b" c00=0x");
    append_hex_u32(out, len, regs.ctrl_33c00);
    append_bytes(out, len, b" c34=0x");
    append_hex_u32(out, len, regs.ctrl_33c34);
    append_bytes(out, len, b" c40=0x");
    append_hex_u32(out, len, regs.ctrl_33c40);
    append_bytes(out, len, b" c74=0x");
    append_hex_u32(out, len, regs.ctrl_33c74);
    append_bytes(out, len, b" r0=0x");
    append_hex_u32(out, len, regs.rx_ctrl0);
    append_bytes(out, len, b" r1=0x");
    append_hex_u32(out, len, regs.rx_ctrl1);
    append_bytes(out, len, b" r2=0x");
    append_hex_u32(out, len, regs.rx_ctrl2);
    append_bytes(out, len, b" r3=0x");
    append_hex_u32(out, len, regs.rx_ctrl3);
    append_bytes(out, len, b" rg=0x");
    append_hex_u32(out, len, regs.rx_global);
    append_bytes(out, len, b" ra0=0x");
    append_hex_u32(out, len, regs.rx_addr0);
    append_bytes(out, len, b" ra1=0x");
    append_hex_u32(out, len, regs.rx_addr1);
    append_bytes(out, len, b" ram0=0x");
    append_hex_u32(out, len, regs.rx_addr_mask0);
    append_bytes(out, len, b" ram1=0x");
    append_hex_u32(out, len, regs.rx_addr_mask1);
    append_bytes(out, len, b" cfg0=0x");
    append_hex_u32(out, len, regs.rx_cfg0);
    append_bytes(out, len, b" cfg1=0x");
    append_hex_u32(out, len, regs.rx_cfg1);
    append_bytes(out, len, b" cfg2=0x");
    append_hex_u32(out, len, regs.rx_cfg2);
    append_bytes(out, len, b" cfg3=0x");
    append_hex_u32(out, len, regs.rx_cfg3);
    append_bytes(out, len, b" base=0x");
    append_hex_u32(out, len, regs.rx_base);
    append_bytes(out, len, b" flt=0x");
    append_hex_u32(out, len, regs.rx_filter_count);
    append_bytes(out, len, b" f0=0x");
    append_hex_u32(out, len, regs.rx_filter_ctrl0);
    append_bytes(out, len, b" f5=0x");
    append_hex_u32(out, len, regs.rx_filter_ctrl5);
    append_bytes(out, len, b" p0=0x");
    append_hex_u32(out, len, regs.rx_filter_pattern0);
    append_bytes(out, len, b" p5=0x");
    append_hex_u32(out, len, regs.rx_filter_pattern5);
    append_bytes(out, len, b" m0=0x");
    append_hex_u32(out, len, regs.rx_filter_mask0);
    append_bytes(out, len, b" m5=0x");
    append_hex_u32(out, len, regs.rx_filter_mask5);
    append_bytes(out, len, b" c114=0x");
    append_hex_u32(out, len, regs.ctrl_33114);
    append_bytes(out, len, b" c118=0x");
    append_hex_u32(out, len, regs.ctrl_33118);
    append_bytes(out, len, b" sn=0x");
    append_hex_u32(out, len, regs.sniffer_ctrl);
    append_bytes(out, len, b" sm0=0x");
    append_hex_u32(out, len, regs.sniffer_misc0);
    append_bytes(out, len, b" sm1=0x");
    append_hex_u32(out, len, regs.sniffer_misc1);
    append_bytes(out, len, b" coex=0x");
    append_hex_u32(out, len, regs.coex_ctrl);
    append_bytes(out, len, b" pti=0x");
    append_hex_u32(out, len, regs.coex_pti);
    append_bytes(out, len, b" dpti=0x");
    append_hex_u32(out, len, regs.coex_default_pti);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_tx_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_tx_regs();
    append_bytes(out, len, b"wifi tx c118=0x");
    append_hex_u32(out, len, regs.ctrl_33118);
    append_bytes(out, len, b" c78=0x");
    append_hex_u32(out, len, regs.ctrl_33c78);
    append_bytes(out, len, b" t10=0x");
    append_hex_u32(out, len, regs.tx_ctrl_33c10);
    append_bytes(out, len, b" t14=0x");
    append_hex_u32(out, len, regs.tx_ctrl_33c14);
    append_bytes(out, len, b" t18=0x");
    append_hex_u32(out, len, regs.tx_ctrl_33c18);
    append_bytes(out, len, b" t54=0x");
    append_hex_u32(out, len, regs.tx_ctrl_33c54);
    append_bytes(out, len, b" t88=0x");
    append_hex_u32(out, len, regs.tx_ctrl_33c88);
    append_bytes(out, len, b" t94=0x");
    append_hex_u32(out, len, regs.tx_ctrl_33c94);
    append_bytes(out, len, b" b8=0x");
    append_hex_u32(out, len, regs.ctrl_332b8);
    append_bytes(out, len, b" c84=0x");
    append_hex_u32(out, len, regs.ctrl_33084);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_rx_scratch_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_rx_scratch_regs();
    append_bytes(out, len, b"wifi rx base=0x");
    append_hex_u32(out, len, regs.base);
    append_bytes(out, len, b" next=0x");
    append_hex_u32(out, len, regs.next);
    append_bytes(out, len, b" last=0x");
    append_hex_u32(out, len, regs.last);
    append_bytes(out, len, b" a0=0x");
    append_hex_u32(out, len, regs.aux0);
    append_bytes(out, len, b" a1=0x");
    append_hex_u32(out, len, regs.aux1);
    append_bytes(out, len, b" c00=0x");
    append_hex_u32(out, len, regs.ctrl_33c00);
    append_bytes(out, len, b" reload=0x");
    append_hex_u32(out, len, regs.reload);
    append_bytes(out, len, b" irq=0x");
    append_hex_u32(out, len, regs.interrupt_status);
    append_bytes(out, len, b" clr=0x");
    append_hex_u32(out, len, regs.interrupt_clear);
    append_bytes(out, len, b" dma=0x");
    append_hex_u32(out, len, regs.dma_state);
    append_bytes(out, len, b" end0=0x");
    append_hex_u32(out, len, regs.rx_end0);
    append_bytes(out, len, b" end1=0x");
    append_hex_u32(out, len, regs.rx_end1);
    append_bytes(out, len, b" est=0x");
    append_hex_u32(out, len, regs.rx_end_state);
    append_bytes(out, len, b" st0=0x");
    append_hex_u32(out, len, regs.rx_state0);
    append_bytes(out, len, b" st1=0x");
    append_hex_u32(out, len, regs.rx_state1);
    append_bytes(out, len, b" ri0=0x");
    append_hex_u32(out, len, regs.rx_info0);
    append_bytes(out, len, b" ri1=0x");
    append_hex_u32(out, len, regs.rx_info1);
    append_bytes(out, len, b" ri2=0x");
    append_hex_u32(out, len, regs.rx_info2);
    append_bytes(out, len, b" ri3=0x");
    append_hex_u32(out, len, regs.rx_info3);
    append_bytes(out, len, b" noise=0x");
    append_hex_u32(out, len, regs.phy_noise_status);
    append_bytes(out, len, b" t0=0x");
    append_hex_u32(out, len, regs.systimer_value);
    append_bytes(out, len, b" t1=0x");
    append_hex_u32(out, len, regs.systimer_aux);
    append_bytes(out, len, b" romptr");
    for word in regs.rom_wifi_ptrs {
        append_bytes(out, len, b" ");
        append_hex_u32(out, len, word);
    }
    append_bytes(out, len, b" ctrl");
    for word in regs.ctrl_words {
        append_bytes(out, len, b" ");
        append_hex_u32(out, len, word);
    }
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_rate_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_rate_regs();
    append_bytes(out, len, b"wifi rate r404=0x");
    append_hex_u32(out, len, regs.rate_33404);
    append_bytes(out, len, b" r408=0x");
    append_hex_u32(out, len, regs.rate_33408);
    append_bytes(out, len, b" r40c=0x");
    append_hex_u32(out, len, regs.rate_3340c);
    append_bytes(out, len, b" r410=0x");
    append_hex_u32(out, len, regs.rate_33410);
    append_bytes(out, len, b" r414=0x");
    append_hex_u32(out, len, regs.rate_33414);
    append_bytes(out, len, b" r418=0x");
    append_hex_u32(out, len, regs.rate_33418);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_crypto_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_crypto_regs();
    append_bytes(out, len, b"wifi crypto c800=0x");
    append_hex_u32(out, len, regs.crypto_33800);
    append_bytes(out, len, b" c804=0x");
    append_hex_u32(out, len, regs.crypto_33804);
    append_bytes(out, len, b" c808=0x");
    append_hex_u32(out, len, regs.crypto_33808);
    append_bytes(out, len, b" c80c=0x");
    append_hex_u32(out, len, regs.crypto_3380c);
    append_bytes(out, len, b" c810=0x");
    append_hex_u32(out, len, regs.crypto_33810);
    append_bytes(out, len, b" c840=0x");
    append_hex_u32(out, len, regs.crypto_33840);
    append_bytes(out, len, b"\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_crypto_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi crypto unavailable\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_antenna_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_antenna_regs();
    append_bytes(out, len, b"wifi ant a0=0x");
    append_hex_u32(out, len, regs.ant0);
    append_bytes(out, len, b" a1=0x");
    append_hex_u32(out, len, regs.ant1);
    append_bytes(out, len, b" a7=0x");
    append_hex_u32(out, len, regs.ant7);
    append_bytes(out, len, b" aux=0x");
    append_hex_u32(out, len, regs.ant_aux);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_phy_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_phy_regs();
    append_bytes(out, len, b"wifi phy lr0=0x");
    append_hex_u32(out, len, regs.low_rate_ctrl0);
    append_bytes(out, len, b" lr1=0x");
    append_hex_u32(out, len, regs.low_rate_ctrl1);
    append_bytes(out, len, b" seed=0x");
    append_hex_u32(out, len, regs.tx_seed);
    append_bytes(out, len, b" b0=0x");
    append_hex_u32(out, len, regs.rx_11b_ctrl0);
    append_bytes(out, len, b" b1=0x");
    append_hex_u32(out, len, regs.rx_11b_ctrl1);
    append_bytes(out, len, b" b2=0x");
    append_hex_u32(out, len, regs.rx_11b_ctrl2);
    append_bytes(out, len, b" b3=0x");
    append_hex_u32(out, len, regs.rx_11b_ctrl3);
    append_bytes(out, len, b" r2440=0x");
    append_hex_u32(out, len, regs.rx_2440m_ctrl);
    append_bytes(out, len, b" bb=0x");
    append_hex_u32(out, len, regs.bb_ctrl_1cc48);
    append_bytes(out, len, b" en=0x");
    append_hex_u32(out, len, regs.modem_wifi_enable);
    append_bytes(out, len, b" m=0x");
    append_hex_u32(out, len, regs.modem_ctrl_26010);
    append_bytes(out, len, b" pa=0x");
    append_hex_u32(out, len, regs.pbus_addr_ctrl);
    append_bytes(out, len, b" pd=0x");
    append_hex_u32(out, len, regs.pbus_data);
    append_bytes(out, len, b" p0=0x");
    append_hex_u32(out, len, regs.pbus_bank0);
    append_bytes(out, len, b" p1=0x");
    append_hex_u32(out, len, regs.pbus_bank1);
    append_bytes(out, len, b" p2=0x");
    append_hex_u32(out, len, regs.pbus_bank2);
    append_bytes(out, len, b" p3=0x");
    append_hex_u32(out, len, regs.pbus_bank3);
    append_bytes(out, len, b" p4=0x");
    append_hex_u32(out, len, regs.pbus_bank4);
    append_bytes(out, len, b" p5=0x");
    append_hex_u32(out, len, regs.pbus_bank5);
    append_bytes(out, len, b" tr0=0x");
    append_hex_u32(out, len, regs.txrate_power0);
    append_bytes(out, len, b" tr15=0x");
    append_hex_u32(out, len, regs.txrate_power15);
    append_bytes(out, len, b" ix0=0x");
    append_hex_u32(out, len, regs.i2c_xpd_ctrl0);
    append_bytes(out, len, b" ix1=0x");
    append_hex_u32(out, len, regs.i2c_xpd_ctrl1);
    append_bytes(out, len, b" rf=0x");
    append_hex_u32(out, len, regs.rf_ctrl);
    append_bytes(out, len, b" txrx=0x");
    append_hex_u32(out, len, regs.txrx_ctrl);
    append_bytes(out, len, b" fpll=0x");
    append_hex_u32(out, len, regs.freq_pll_cap);
    append_bytes(out, len, b" fh=0x");
    append_hex_u32(out, len, regs.freq_hw_ctrl);
    append_bytes(out, len, b" fmem=0x");
    append_hex_u32(out, len, regs.freq_mem_ctrl);
    append_bytes(out, len, b" fsw=0x");
    append_hex_u32(out, len, regs.freq_sw_ctrl);
    append_bytes(out, len, b" fbusy=0x");
    append_hex_u32(out, len, regs.freq_busy);
    append_bytes(out, len, b" fstat=0x");
    append_hex_u32(out, len, regs.freq_status);
    append_bytes(out, len, b" fm=0x");
    append_hex_u32(out, len, regs.freq_mode_ctrl);
    append_bytes(out, len, b"\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_phy_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi phy unavailable\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_antenna_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi ant unavailable\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_rate_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi rate unavailable\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_tx_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi tx unavailable\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_rx_scratch_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi rx unavailable\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_mmio_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi mmio unavailable\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_debug_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_regs();
    append_bytes(out, len, b"wifi mmio regs pwc=0x");
    append_hex_u32(out, len, regs.rtc_dig_pwc);
    append_bytes(out, len, b" iso=0x");
    append_hex_u32(out, len, regs.rtc_dig_iso);
    append_bytes(out, len, b" clk=0x");
    append_hex_u32(out, len, regs.wifi_clk_en);
    append_bytes(out, len, b" rst=0x");
    append_hex_u32(out, len, regs.wifi_rst_en);
    append_bytes(out, len, b" mac=0x");
    append_hex_u32(out, len, regs.mac_reset_ctrl);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
fn append_wifi_phy_fun_slots(out: &mut [u8], len: &mut usize) {
    let slots = edgerun_platform::esp32s3_wifi_blob::EspressifPromiscRadio::debug_phy_fun_slots();
    append_bytes(out, len, b"wifi funs 008=0x");
    append_hex_u32(out, len, slots.slot_008);
    append_bytes(out, len, b" 00c=0x");
    append_hex_u32(out, len, slots.slot_00c);
    append_bytes(out, len, b" 05c=0x");
    append_hex_u32(out, len, slots.slot_05c);
    append_bytes(out, len, b" 06c=0x");
    append_hex_u32(out, len, slots.slot_06c);
    append_bytes(out, len, b" 110=0x");
    append_hex_u32(out, len, slots.slot_110);
    append_bytes(out, len, b" 148=0x");
    append_hex_u32(out, len, slots.slot_148);
    append_bytes(out, len, b" 160=0x");
    append_hex_u32(out, len, slots.slot_160);
    append_bytes(out, len, b" 164=0x");
    append_hex_u32(out, len, slots.slot_164);
    append_bytes(out, len, b" 190=0x");
    append_hex_u32(out, len, slots.slot_190);
    append_bytes(out, len, b" 1a8=0x");
    append_hex_u32(out, len, slots.slot_1a8);
    append_bytes(out, len, b" 1d4=0x");
    append_hex_u32(out, len, slots.slot_1d4);
    append_bytes(out, len, b" 200=0x");
    append_hex_u32(out, len, slots.slot_200);
    append_bytes(out, len, b" 204=0x");
    append_hex_u32(out, len, slots.slot_204);
    append_bytes(out, len, b" 208=0x");
    append_hex_u32(out, len, slots.slot_208);
    append_bytes(out, len, b" 224=0x");
    append_hex_u32(out, len, slots.slot_224);
    append_bytes(out, len, b" 234=0x");
    append_hex_u32(out, len, slots.slot_234);
    append_bytes(out, len, b"\n");
}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-mmio",
        not(feature = "esp32s3-wifi-blob")
    )
)))]
fn append_wifi_phy_fun_slots(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi funs unavailable\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_phy_fun_slots(out: &mut [u8], len: &mut usize) {
    let slots = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_phy_fun_slots();
    append_bytes(out, len, b"wifi funs table=0x");
    append_hex_u32(out, len, slots.table);
    append_bytes(out, len, b" 02c=0x");
    append_hex_u32(out, len, slots.slot_02c);
    append_bytes(out, len, b" 008=0x");
    append_hex_u32(out, len, slots.slot_008);
    append_bytes(out, len, b" 00c=0x");
    append_hex_u32(out, len, slots.slot_00c);
    append_bytes(out, len, b" 054=0x");
    append_hex_u32(out, len, slots.slot_054);
    append_bytes(out, len, b" 05c=0x");
    append_hex_u32(out, len, slots.slot_05c);
    append_bytes(out, len, b" 06c=0x");
    append_hex_u32(out, len, slots.slot_06c);
    append_bytes(out, len, b" 078=0x");
    append_hex_u32(out, len, slots.slot_078);
    append_bytes(out, len, b" 080=0x");
    append_hex_u32(out, len, slots.slot_080);
    append_bytes(out, len, b" 088=0x");
    append_hex_u32(out, len, slots.slot_088);
    append_bytes(out, len, b" 0c4=0x");
    append_hex_u32(out, len, slots.slot_0c4);
    append_bytes(out, len, b" 0c8=0x");
    append_hex_u32(out, len, slots.slot_0c8);
    append_bytes(out, len, b" 0d0=0x");
    append_hex_u32(out, len, slots.slot_0d0);
    append_bytes(out, len, b" 0fc=0x");
    append_hex_u32(out, len, slots.slot_0fc);
    append_bytes(out, len, b" 100=0x");
    append_hex_u32(out, len, slots.slot_100);
    append_bytes(out, len, b" 110=0x");
    append_hex_u32(out, len, slots.slot_110);
    append_bytes(out, len, b" 148=0x");
    append_hex_u32(out, len, slots.slot_148);
    append_bytes(out, len, b" 160=0x");
    append_hex_u32(out, len, slots.slot_160);
    append_bytes(out, len, b" 164=0x");
    append_hex_u32(out, len, slots.slot_164);
    append_bytes(out, len, b" 190=0x");
    append_hex_u32(out, len, slots.slot_190);
    append_bytes(out, len, b" 198=0x");
    append_hex_u32(out, len, slots.slot_198);
    append_bytes(out, len, b" 1a8=0x");
    append_hex_u32(out, len, slots.slot_1a8);
    append_bytes(out, len, b" 1b4=0x");
    append_hex_u32(out, len, slots.slot_1b4);
    append_bytes(out, len, b" 1d4=0x");
    append_hex_u32(out, len, slots.slot_1d4);
    append_bytes(out, len, b" 200=0x");
    append_hex_u32(out, len, slots.slot_200);
    append_bytes(out, len, b" 204=0x");
    append_hex_u32(out, len, slots.slot_204);
    append_bytes(out, len, b" 208=0x");
    append_hex_u32(out, len, slots.slot_208);
    append_bytes(out, len, b" 20c=0x");
    append_hex_u32(out, len, slots.slot_20c);
    append_bytes(out, len, b" 224=0x");
    append_hex_u32(out, len, slots.slot_224);
    append_bytes(out, len, b" 22c=0x");
    append_hex_u32(out, len, slots.slot_22c);
    append_bytes(out, len, b" 234=0x");
    append_hex_u32(out, len, slots.slot_234);
    append_bytes(out, len, b" 254=0x");
    append_hex_u32(out, len, slots.slot_254);
    append_bytes(out, len, b" 264=0x");
    append_hex_u32(out, len, slots.slot_264);
    append_bytes(out, len, b" 268=0x");
    append_hex_u32(out, len, slots.slot_268);
    append_bytes(out, len, b" 288=0x");
    append_hex_u32(out, len, slots.slot_288);
    append_bytes(out, len, b" 28c=0x");
    append_hex_u32(out, len, slots.slot_28c);
    append_bytes(out, len, b"\n");
}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-mmio",
        not(feature = "esp32s3-wifi-blob")
    )
)))]
fn append_wifi_debug_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi regs unavailable\n");
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_touch_status(seq: u16, x: u16, y: u16) {
    let mut buf = [0u8; 32];
    let mut len = 0;
    append_bytes(&mut buf, &mut len, b"touch x=");
    append_u16(&mut buf, &mut len, x);
    append_bytes(&mut buf, &mut len, b" y=");
    append_u16(&mut buf, &mut len, y);
    append_bytes(&mut buf, &mut len, b"\n");
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

fn append_bytes(out: &mut [u8], len: &mut usize, bytes: &[u8]) {
    for &byte in bytes {
        if *len == out.len() {
            return;
        }
        out[*len] = byte;
        *len += 1;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn append_u16(out: &mut [u8], len: &mut usize, mut value: u16) {
    let mut digits = [0u8; 5];
    let mut count = 0;
    loop {
        digits[count] = b'0' + (value % 10) as u8;
        count += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    while count > 0 {
        count -= 1;
        append_bytes(out, len, &digits[count..count + 1]);
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn append_u32_dec(out: &mut [u8], len: &mut usize, mut value: u32) {
    let mut digits = [0u8; 10];
    let mut count = 0;
    loop {
        digits[count] = b'0' + (value % 10) as u8;
        count += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    while count > 0 {
        count -= 1;
        append_bytes(out, len, &digits[count..count + 1]);
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn append_i32_dec(out: &mut [u8], len: &mut usize, value: i32) {
    if value < 0 {
        append_bytes(out, len, b"-");
        append_u32_dec(out, len, value.saturating_abs() as u32);
    } else {
        append_u32_dec(out, len, value as u32);
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn append_i32(out: &mut [u8], len: &mut usize, value: i32) {
    if value < 0 {
        append_bytes(out, len, b"-");
        append_u16(out, len, value.saturating_abs() as u16);
    } else {
        append_u16(out, len, value as u16);
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn append_hex_u32(out: &mut [u8], len: &mut usize, value: u32) {
    let mut shift = 28;
    loop {
        let nibble = ((value >> shift) & 0x0f) as u8;
        append_hex_nibble(out, len, nibble);
        if shift == 0 {
            break;
        }
        shift -= 4;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn append_hex_nibble(out: &mut [u8], len: &mut usize, nibble: u8) {
    let nibble = nibble & 0x0f;
    let byte = if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
    };
    append_bytes(out, len, &[byte]);
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-stub",
    not(feature = "esp32s3-ble-blob")
))]
static ESP32S3_BT_STUB_STATE: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-stub",
    not(feature = "esp32s3-ble-blob")
))]
static ESP32S3_BT_STUB_TX_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-stub",
    not(feature = "esp32s3-ble-blob")
))]
fn try_start_esp32s3_bt() -> bool {
    use edgerun_platform::esp32s3_ble::Esp32s3BlePeripheral;
    use edgerun_platform::esp32s3_ble_stub::NoBleRadio;

    let radio = NoBleRadio::<8, 8>::new();
    let mut ble = Esp32s3BlePeripheral::new(radio);
    let ok = ble.init().is_ok()
        && ble.set_adv_data(b"\x08edgerun").is_ok()
        && ble.start_advertising().is_ok();
    let tx_count = ble.raw_transport_mut().tx_count();
    ESP32S3_BT_STUB_TX_COUNT.store(tx_count, core::sync::atomic::Ordering::Release);
    ESP32S3_BT_STUB_STATE.store(
        if ok { 1 } else { 2 },
        core::sync::atomic::Ordering::Release,
    );
    ok
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-stub",
    not(feature = "esp32s3-ble-blob")
))]
fn append_bt_stats(out: &mut [u8], len: &mut usize) {
    let state = ESP32S3_BT_STUB_STATE.load(core::sync::atomic::Ordering::Acquire);
    let tx_count = ESP32S3_BT_STUB_TX_COUNT.load(core::sync::atomic::Ordering::Acquire);
    append_bytes(out, len, b"bt stub ");
    match state {
        0 => append_bytes(out, len, b"idle"),
        1 => append_bytes(out, len, b"advertising"),
        _ => append_bytes(out, len, b"error"),
    }
    append_bytes(out, len, b" tx=");
    append_u32_dec(out, len, tx_count);
    append_bytes(out, len, b" controller=missing\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
))]
static ESP32S3_BT_BLOB_STATE: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
))]
static mut ESP32S3_BT_BLOB_VERSION: [u8; edgerun_platform::esp32s3_ble_blob::VERSION_MAX] =
    [0; edgerun_platform::esp32s3_ble_blob::VERSION_MAX];
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
))]
static ESP32S3_BT_BLOB_VERSION_LEN: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
))]
static ESP32S3_BT_BLOB_OSI_RC: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(i32::MIN);
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
))]
static ESP32S3_BT_BLOB_INIT_RC: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(i32::MIN);
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
))]
static ESP32S3_BT_BLOB_ENABLE_RC: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(i32::MIN);
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
))]
static ESP32S3_BT_BLOB_VHCI_RC: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(i32::MIN);
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
))]
static ESP32S3_BT_BLOB_HCI_TX: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
))]
fn try_start_esp32s3_bt() -> bool {
    if ESP32S3_BT_BLOB_STATE.load(core::sync::atomic::Ordering::Acquire) == 1 {
        return ESP32S3_BT_BLOB_OSI_RC.load(core::sync::atomic::Ordering::Acquire) == 0
            && ESP32S3_BT_BLOB_INIT_RC.load(core::sync::atomic::Ordering::Acquire) == 0
            && ESP32S3_BT_BLOB_ENABLE_RC.load(core::sync::atomic::Ordering::Acquire) == 0
            && ESP32S3_BT_BLOB_VHCI_RC.load(core::sync::atomic::Ordering::Acquire) == 0;
    }
    match edgerun_platform::esp32s3_ble_blob::probe() {
        Ok(status) => {
            unsafe {
                let version = core::ptr::addr_of_mut!(ESP32S3_BT_BLOB_VERSION);
                (&mut (*version))[..status.version_len].copy_from_slice(status.version_bytes());
            }
            ESP32S3_BT_BLOB_VERSION_LEN.store(
                status.version_len as u32,
                core::sync::atomic::Ordering::Release,
            );
            ESP32S3_BT_BLOB_OSI_RC.store(status.osi_rc, core::sync::atomic::Ordering::Release);
            ESP32S3_BT_BLOB_INIT_RC.store(status.init_rc, core::sync::atomic::Ordering::Release);
            ESP32S3_BT_BLOB_ENABLE_RC
                .store(status.enable_rc, core::sync::atomic::Ordering::Release);
            ESP32S3_BT_BLOB_VHCI_RC.store(status.vhci_rc, core::sync::atomic::Ordering::Release);
            ESP32S3_BT_BLOB_HCI_TX.store(status.hci_tx, core::sync::atomic::Ordering::Release);
            ESP32S3_BT_BLOB_STATE.store(1, core::sync::atomic::Ordering::Release);
            status.osi_rc == 0
                && status.init_rc == 0
                && status.enable_rc == 0
                && status.vhci_rc == 0
        }
        Err(_) => {
            ESP32S3_BT_BLOB_STATE.store(2, core::sync::atomic::Ordering::Release);
            false
        }
    }
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
))]
fn append_bt_stats(out: &mut [u8], len: &mut usize) {
    let state = ESP32S3_BT_BLOB_STATE.load(core::sync::atomic::Ordering::Acquire);
    append_bytes(out, len, b"bt blob ");
    match state {
        0 => append_bytes(out, len, b"idle"),
        1 => append_bytes(out, len, b"linked"),
        _ => append_bytes(out, len, b"probe-error"),
    }
    append_bytes(out, len, b" controller=vendor");
    let version_len = ESP32S3_BT_BLOB_VERSION_LEN.load(core::sync::atomic::Ordering::Acquire);
    if version_len != 0 {
        append_bytes(out, len, b" version=");
        let version_len =
            (version_len as usize).min(edgerun_platform::esp32s3_ble_blob::VERSION_MAX);
        let version = unsafe { &*core::ptr::addr_of!(ESP32S3_BT_BLOB_VERSION) };
        append_bytes(out, len, &version[..version_len]);
    }
    append_bytes(out, len, b" osi=");
    append_i32_dec(
        out,
        len,
        ESP32S3_BT_BLOB_OSI_RC.load(core::sync::atomic::Ordering::Acquire),
    );
    append_bytes(out, len, b" init=");
    append_i32_dec(
        out,
        len,
        ESP32S3_BT_BLOB_INIT_RC.load(core::sync::atomic::Ordering::Acquire),
    );
    append_bytes(out, len, b" enable=");
    append_i32_dec(
        out,
        len,
        ESP32S3_BT_BLOB_ENABLE_RC.load(core::sync::atomic::Ordering::Acquire),
    );
    append_bytes(out, len, b" vhci=");
    append_i32_dec(
        out,
        len,
        ESP32S3_BT_BLOB_VHCI_RC.load(core::sync::atomic::Ordering::Acquire),
    );
    append_bytes(out, len, b" hci_tx=");
    append_u32_dec(
        out,
        len,
        ESP32S3_BT_BLOB_HCI_TX.load(core::sync::atomic::Ordering::Acquire),
    );
    let hci = edgerun_platform::esp32s3_ble_blob::hci_stats();
    append_bytes(out, len, b" hci_rx=");
    append_u32_dec(out, len, hci.rx);
    append_bytes(out, len, b" ready=");
    append_u32_dec(out, len, hci.send_available);
    append_bytes(out, len, b" evt=0x");
    append_hex_nibble(out, len, hci.last_event >> 4);
    append_hex_nibble(out, len, hci.last_event);
    append_bytes(out, len, b" st=0x");
    append_hex_nibble(out, len, hci.last_status >> 4);
    append_hex_nibble(out, len, hci.last_status);
    append_bytes(out, len, b" op=0x");
    append_u16(out, len, hci.last_opcode);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
))]
fn try_send_esp32s3_bt_adv() -> bool {
    let tx = edgerun_platform::esp32s3_ble_blob::send_advertising();
    ESP32S3_BT_BLOB_HCI_TX.store(tx, core::sync::atomic::Ordering::Release);
    tx >= 4
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
)))]
fn try_send_esp32s3_bt_adv() -> bool {
    false
}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-ble-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-ble-stub",
        not(feature = "esp32s3-ble-blob")
    )
)))]
fn try_start_esp32s3_bt() -> bool {
    false
}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-ble-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-ble-stub",
        not(feature = "esp32s3-ble-blob")
    )
)))]
fn append_bt_stats(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"bt backend disabled\n");
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_bt_start(seq: u16) {
    let mut buf = [0u8; 160];
    let mut len = 0usize;
    if try_start_esp32s3_bt() {
        append_bytes(&mut buf, &mut len, b"ok bt-start ");
    } else {
        append_bytes(&mut buf, &mut len, b"err bt-start ");
    }
    append_bt_stats(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_bt_stats(seq: u16) {
    let mut buf = [0u8; 160];
    let mut len = 0usize;
    append_bt_stats(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_bt_adv(seq: u16) {
    let mut buf = [0u8; 160];
    let mut len = 0usize;
    if try_send_esp32s3_bt_adv() {
        append_bytes(&mut buf, &mut len, b"ok bt-adv ");
    } else {
        append_bytes(&mut buf, &mut len, b"err bt-adv ");
    }
    append_bt_stats(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless"
))]
fn write_headless_bt_start_raw() {
    if try_start_esp32s3_bt() {
        headless_raw_write(b"ok bt-start ");
    } else {
        headless_raw_write(b"err bt-start ");
    }
    write_headless_bt_stats_raw();
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless"
))]
fn write_headless_bt_adv_raw() {
    if try_send_esp32s3_bt_adv() {
        headless_raw_write(b"ok bt-adv ");
    } else {
        headless_raw_write(b"err bt-adv ");
    }
    write_headless_bt_stats_raw();
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-headless"
))]
fn write_headless_bt_stats_raw() {
    let mut buf = [0u8; 160];
    let mut len = 0usize;
    append_bt_stats(&mut buf, &mut len);
    headless_raw_write(&buf[..len]);
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
static ESP32S3_VENDOR_WIFI_AP_STARTED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
type Esp32s3WifiMmioAp = edgerun_platform::esp32s3_wifi::Esp32s3WifiOpenAp<
    edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio,
    4,
>;

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
struct Esp32s3WifiMmioApCell(core::cell::UnsafeCell<core::mem::MaybeUninit<Esp32s3WifiMmioAp>>);

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
unsafe impl Sync for Esp32s3WifiMmioApCell {}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
impl Esp32s3WifiMmioApCell {
    const fn new() -> Self {
        Self(core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit()))
    }

    unsafe fn init(&self, ap: Esp32s3WifiMmioAp) -> &'static mut Esp32s3WifiMmioAp {
        unsafe { (&mut *self.0.get()).write(ap) }
    }

    unsafe fn get(&self) -> &'static mut Esp32s3WifiMmioAp {
        unsafe { (&mut *self.0.get()).assume_init_mut() }
    }
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
static ESP32S3_WIFI_MMIO_AP: Esp32s3WifiMmioApCell = Esp32s3WifiMmioApCell::new();
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
static ESP32S3_WIFI_MMIO_AP_STARTED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
#[inline(never)]
fn try_start_esp32s3_wifi_ap() -> bool {
    use edgerun_platform::esp32s3_wifi_blob::EspressifPromiscRadio;

    rt::log::log(1, "ESP32-S3 vendor WiFi AP start begin");
    if EspressifPromiscRadio::start_vendor_open_ap(b"edgerun-ac", 6) {
        ESP32S3_VENDOR_WIFI_AP_STARTED.store(true, core::sync::atomic::Ordering::Release);
        rt::log::log(1, "ESP32-S3 vendor WiFi AP started");
        return true;
    }
    match EspressifPromiscRadio::last_start_status() {
        10_000..=19_999 => rt::log::log(3, "ESP32-S3 vendor WiFi AP failed: init stage"),
        20_000..=29_999 => rt::log::log(3, "ESP32-S3 vendor WiFi AP failed: mode stage"),
        30_000..=39_999 => rt::log::log(3, "ESP32-S3 vendor WiFi AP failed: config stage"),
        40_000..=49_999 => rt::log::log(3, "ESP32-S3 vendor WiFi AP failed: start stage"),
        _ => rt::log::log(3, "ESP32-S3 vendor WiFi AP failed"),
    }
    false
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
fn try_wifi_debug_step(step: u8) -> bool {
    edgerun_platform::esp32s3_wifi_blob::EspressifPromiscRadio::debug_step(step)
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
fn try_wifi_init_known_good() -> bool {
    try_start_esp32s3_wifi_ap()
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
fn wifi_debug_status() -> i32 {
    edgerun_platform::esp32s3_wifi_blob::EspressifPromiscRadio::last_start_status()
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
fn poll_esp32s3_wifi_ap() {}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
fn write_wifi_ap_stats(seq: u16) {
    use edgerun_platform::esp32s3_wifi_blob::EspressifPromiscRadio;

    if !ESP32S3_VENDOR_WIFI_AP_STARTED.load(core::sync::atomic::Ordering::Acquire) {
        rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, b"wifi ap stopped\n");
        return;
    }
    let mut out = [0u8; 96];
    let mut len = 0usize;
    append_bytes(&mut out, &mut len, b"wifi vendor ap started status=");
    append_i32(
        &mut out,
        &mut len,
        EspressifPromiscRadio::last_start_status(),
    );
    append_bytes(&mut out, &mut len, b"\n");
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &out[..len]);
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn try_wifi_debug_step(step: u8) -> bool {
    edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_step(step)
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn try_wifi_init_known_good() -> bool {
    edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::init_known_good()
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn wifi_debug_status() -> i32 {
    edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::last_status()
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
#[inline(never)]
fn try_start_esp32s3_wifi_ap() -> bool {
    use edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio;
    use edgerun_wifi::ieee80211::{MacAddr, OpenApConfig};

    #[cfg(not(feature = "esp32s3-headless"))]
    rt::log::log(1, "ESP32-S3 MMIO WiFi AP RX start begin");
    let config = match OpenApConfig::new(
        MacAddr::new([0x02, 0xed, 0x67, 0x75, 0x6e, 0x01]),
        b"edgerun-ac",
        6,
    ) {
        Ok(config) => config,
        Err(_) => {
            #[cfg(not(feature = "esp32s3-headless"))]
            rt::log::log(3, "ESP32-S3 MMIO WiFi AP config failed");
            return false;
        }
    };

    let ap = unsafe {
        ESP32S3_WIFI_MMIO_AP.init(Esp32s3WifiMmioAp::new(Esp32s3WifiMmio::new(), config))
    };
    match ap.start() {
        Ok(()) => {
            ESP32S3_WIFI_MMIO_AP_STARTED.store(true, core::sync::atomic::Ordering::Release);
            #[cfg(not(feature = "esp32s3-headless"))]
            rt::log::log(1, "ESP32-S3 MMIO WiFi RX path armed");
            true
        }
        Err(_) => {
            #[cfg(not(feature = "esp32s3-headless"))]
            rt::log::log(3, "ESP32-S3 MMIO WiFi AP start failed");
            false
        }
    }
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn poll_esp32s3_wifi_ap() {
    if ESP32S3_WIFI_MMIO_AP_STARTED.load(core::sync::atomic::Ordering::Acquire) {
        unsafe { ESP32S3_WIFI_MMIO_AP.get().poll() };
    }
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn write_wifi_ap_stats(seq: u16) {
    if !ESP32S3_WIFI_MMIO_AP_STARTED.load(core::sync::atomic::Ordering::Acquire) {
        rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, b"wifi ap stopped\n");
        return;
    }
    let stats = unsafe { ESP32S3_WIFI_MMIO_AP.get().stats() };
    write_wifi_ap_stats_inner(seq, stats);
}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-mmio",
        not(feature = "esp32s3-wifi-blob")
    )
)))]
fn try_wifi_init_known_good() -> bool {
    false
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
)))]
#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn try_start_esp32s3_wifi_ap() -> bool {
    rt::log::log(3, "ESP32-S3 WiFi AP backend disabled");
    false
}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-mmio",
        not(feature = "esp32s3-wifi-blob")
    )
)))]
fn poll_esp32s3_wifi_ap() {}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-mmio",
        not(feature = "esp32s3-wifi-blob")
    )
)))]
fn write_wifi_ap_stats(seq: u16) {
    rt::serial_mux::write_with_seq(
        rt::serial_mux::CHANNEL_CONTROL,
        seq,
        b"wifi ap backend disabled\n",
    );
}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-mmio",
        not(feature = "esp32s3-wifi-blob")
    )
)))]
fn try_wifi_debug_step(_step: u8) -> bool {
    rt::log::log(3, "ESP32-S3 WiFi debug backend disabled");
    false
}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-mmio",
        not(feature = "esp32s3-wifi-blob")
    )
)))]
fn wifi_debug_status() -> i32 {
    0
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
const ESP_UI_HTML: &str = r#"
<div class="screen">
  <div class="hero">
    <div class="eyebrow">TCL AC</div>
    <div class="temp">24</div>
    <div class="status">Cool - Auto fan</div>
  </div>
  <div class="actions">
    <div class="button power">Power</div>
    <div class="button">Mode</div>
    <div class="button">Fan</div>
    <div class="button">Swing</div>
  </div>
</div>
"#;

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
const ESP_UI_CSS: &str = r#"
.screen { background-color: #050b0f; color: #e5edf2; width: 320px; min-height: 366px; padding: 16px; }
.hero { background-color: #08252d; color: white; width: 288px; height: 174px; padding: 18px; margin-bottom: 20px; border-radius: 14px; }
.eyebrow { color: #9cf3ea; font-size: 16px; margin-bottom: 26px; }
.temp { color: white; font-size: 72px; line-height: 1.0; margin-bottom: 2px; }
.status { color: #9cf3ea; font-size: 18px; }
.actions { width: 288px; flex-wrap: wrap; }
.button { background-color: #111f26; color: #e5edf2; width: 128px; height: 68px; padding: 18px; margin-right: 16px; margin-bottom: 14px; border-radius: 10px; font-size: 18px; }
.power { background-color: #ad2430; color: white; }
"#;

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
#[derive(Clone, Copy)]
struct EspUiScene<'a> {
    commands: [Option<edgerun_layout::EmbeddedCommand<'a>>; 48],
    len: usize,
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
impl<'a> EspUiScene<'a> {
    const fn new() -> Self {
        Self {
            commands: [None; 48],
            len: 0,
        }
    }

    fn push(&mut self, command: edgerun_layout::EmbeddedCommand<'a>) {
        if self.len < self.commands.len() {
            self.commands[self.len] = Some(command);
            self.len += 1;
        }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn render_html_ui(touch: Option<(u16, u16)>) {
    render_html_ui_with_overlay(touch, TOUCH_OVERLAY_ALPHA);
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn render_html_ui_with_overlay(touch: Option<(u16, u16)>, touch_alpha: u8) {
    let scene = render_embedded_ui_scene();
    unsafe {
        edgerun_platform::esp32s3::Jc3248w535Display::draw_rgb565_with(320, 480, |x, y| {
            let mut color = embedded_ui_scene_pixel(&scene, x, y);
            if let Some(console) = console_pixel(x, y) {
                color = console;
            }
            if let Some(center) = touch {
                color = touch_overlay_over_base(color, x, y, center, touch_alpha);
            }
            color
        });
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn ui_base_pixel(x: u16, y: u16) -> u16 {
    if let Some(color) = console_pixel(x, y) {
        color
    } else {
        let scene = render_embedded_ui_scene();
        embedded_ui_scene_pixel(&scene, x, y)
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", not(feature = "html-ui")))]
fn ui_base_pixel(x: u16, y: u16) -> u16 {
    if let Some(color) = console_pixel(x, y) {
        color
    } else {
        debug_pixel(x, y)
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn console_pixel(x: u16, y: u16) -> Option<u16> {
    let x = x as u32;
    let y = y as u32;
    if y < 366 {
        return None;
    }
    if y == 366 {
        return Some(rgb565(20, 184, 166));
    }
    if y < 480 {
        if console_text_pixel(x, y) {
            return Some(rgb565(226, 232, 240));
        }
        let shade = if ((x / 12) + (y / 12)) & 1 == 0 { 0 } else { 3 };
        return Some(rgb565(3 + shade, 7 + shade, 18 + shade));
    }
    None
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn console_text_pixel(px: u32, py: u32) -> bool {
    const LEFT: u32 = 8;
    const TOP: u32 = 374;
    const SCALE: u32 = 1;
    const GLYPH_W: u32 = 5;
    const GLYPH_H: u32 = 7;
    const ROW_H: u32 = 12;
    if px < LEFT || py < TOP {
        return false;
    }
    let row = ((py - TOP) / ROW_H) as usize;
    if row >= DISPLAY_CONSOLE_LINES {
        return false;
    }
    let local_y = (py - TOP) % ROW_H;
    if local_y >= GLYPH_H {
        return false;
    }
    let col = ((px - LEFT) / (GLYPH_W + SCALE)) as usize;
    if col >= DISPLAY_CONSOLE_COLS {
        return false;
    }
    let local_x = (px - LEFT) % (GLYPH_W + SCALE);
    if local_x >= GLYPH_W {
        return false;
    }

    DISPLAY_CONSOLE.with(|console| {
        if row >= console.count {
            return false;
        }
        let line_index =
            (console.next + DISPLAY_CONSOLE_LINES - console.count + row) % DISPLAY_CONSOLE_LINES;
        if col >= console.lens[line_index] {
            return false;
        }
        let glyph = glyph_5x7(console.lines[line_index][col]);
        (glyph[local_y as usize] & (1 << (4 - local_x))) != 0
    })
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn render_embedded_ui_scene() -> EspUiScene<'static> {
    let mut scene = EspUiScene::new();
    scene.push(edgerun_layout::EmbeddedCommand::FillRect {
        x: 0,
        y: 0,
        w: 320,
        h: 480,
        color: edgerun_layout::Color::rgb(5, 11, 15),
    });
    scene.push(edgerun_layout::EmbeddedCommand::RoundedRect {
        x: 16,
        y: 16,
        w: 288,
        h: 174,
        color: edgerun_layout::Color::rgb(8, 37, 45),
        radius: 14,
    });
    scene.push(edgerun_layout::EmbeddedCommand::Text {
        x: 34,
        y: 50,
        text: "TCL AC",
        color: edgerun_layout::Color::rgb(156, 243, 234),
        font_size: 16,
    });
    scene.push(edgerun_layout::EmbeddedCommand::Text {
        x: 34,
        y: 128,
        text: "24",
        color: edgerun_layout::Color::rgb(255, 255, 255),
        font_size: 72,
    });
    scene.push(edgerun_layout::EmbeddedCommand::Text {
        x: 34,
        y: 166,
        text: "Cool - Auto fan",
        color: edgerun_layout::Color::rgb(156, 243, 234),
        font_size: 18,
    });
    push_embedded_button(
        &mut scene,
        16,
        210,
        "Power",
        edgerun_layout::Color::rgb(173, 36, 48),
        edgerun_layout::Color::rgb(255, 255, 255),
    );
    push_embedded_button(
        &mut scene,
        160,
        210,
        "Mode",
        edgerun_layout::Color::rgb(17, 31, 38),
        edgerun_layout::Color::rgb(229, 237, 242),
    );
    push_embedded_button(
        &mut scene,
        16,
        292,
        "Fan",
        edgerun_layout::Color::rgb(17, 31, 38),
        edgerun_layout::Color::rgb(229, 237, 242),
    );
    push_embedded_button(
        &mut scene,
        160,
        292,
        "Swing",
        edgerun_layout::Color::rgb(17, 31, 38),
        edgerun_layout::Color::rgb(229, 237, 242),
    );
    scene
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn push_embedded_button(
    scene: &mut EspUiScene<'static>,
    x: u32,
    y: u32,
    text: &'static str,
    background: edgerun_layout::Color,
    color: edgerun_layout::Color,
) {
    scene.push(edgerun_layout::EmbeddedCommand::RoundedRect {
        x,
        y,
        w: 128,
        h: 68,
        color: background,
        radius: 10,
    });
    scene.push(edgerun_layout::EmbeddedCommand::Text {
        x: x.saturating_add(18),
        y: y.saturating_add(40),
        text,
        color,
        font_size: 18,
    });
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn embedded_ui_scene_pixel(scene: &EspUiScene<'_>, x: u16, y: u16) -> u16 {
    let mut color = rgb565(5, 11, 15);
    let px = x as u32;
    let py = y as u32;
    let mut index = 0;
    while index < scene.len {
        let Some(command) = scene.commands[index] else {
            index += 1;
            continue;
        };
        match command {
            edgerun_layout::EmbeddedCommand::FillRect {
                x,
                y,
                w,
                h,
                color: command_color,
            } => {
                if px >= x && py >= y && px < x.saturating_add(w) && py < y.saturating_add(h) {
                    color = apply_layout_color(color, command_color);
                }
            }
            edgerun_layout::EmbeddedCommand::RoundedRect {
                x,
                y,
                w,
                h,
                color: command_color,
                radius,
            } => {
                if in_rounded_rect(px, py, x, y, w, h, radius) {
                    color = apply_layout_color(color, command_color);
                }
            }
            edgerun_layout::EmbeddedCommand::Text {
                x,
                y,
                text,
                color: command_color,
                font_size,
            } => {
                if text_pixel(px, py, x, y, font_size, text) {
                    color = apply_layout_color(color, command_color);
                }
            }
        }
        index += 1;
    }
    color
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn apply_layout_color(base: u16, color: edgerun_layout::Color) -> u16 {
    let overlay = rgb565(color.r, color.g, color.b);
    if color.a == 255 {
        overlay
    } else {
        blend_rgb565(base, overlay, color.a)
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn in_rounded_rect(px: u32, py: u32, x: u32, y: u32, w: u32, h: u32, radius: u32) -> bool {
    if px < x || py < y || px >= x.saturating_add(w) || py >= y.saturating_add(h) {
        return false;
    }
    let r = radius.min(w / 2).min(h / 2);
    let in_corner = (px < x + r || px >= x + w - r) && (py < y + r || py >= y + h - r);
    if !in_corner {
        return true;
    }
    let cx = if px < x + r { x + r } else { x + w - r - 1 };
    let cy = if py < y + r { y + r } else { y + h - r - 1 };
    let dx = px as i32 - cx as i32;
    let dy = py as i32 - cy as i32;
    dx * dx + dy * dy <= (r * r) as i32
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn text_pixel(px: u32, py: u32, x: u32, baseline_y: u32, font_size: u32, text: &str) -> bool {
    let scale = (font_size / 8).max(1);
    let glyph_w = 5 * scale;
    let glyph_h = 7 * scale;
    let top = baseline_y.saturating_sub(glyph_h);
    if py < top || py >= baseline_y {
        return false;
    }

    let mut cursor_x = x;
    for byte in text.bytes() {
        if px >= cursor_x && px < cursor_x.saturating_add(glyph_w) {
            let gx = (px - cursor_x) / scale;
            let gy = (py - top) / scale;
            let glyph = glyph_5x7(byte);
            return (glyph[gy as usize] & (1 << (4 - gx))) != 0;
        }
        cursor_x = cursor_x.saturating_add(glyph_w + scale);
    }
    false
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn paint_tile_commands(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    tile_y: u32,
    commands: &[edgerun_layout::UiRenderCommand],
) {
    for command in commands {
        match command {
            edgerun_layout::UiRenderCommand::FillRect { x, y, w, h, color } => {
                fill_rect_bgra(pixels, width, height, tile_y, *x, *y, *w, *h, *color);
            }
            edgerun_layout::UiRenderCommand::StrokeRect {
                x,
                y,
                w,
                h,
                color,
                style,
                thickness,
            } => {
                let t = (*thickness).max(1);
                fill_rect_bgra(pixels, width, height, tile_y, *x, *y, *w, t, *color);
                fill_rect_bgra(
                    pixels,
                    width,
                    height,
                    tile_y,
                    *x,
                    y + h.saturating_sub(t),
                    *w,
                    t,
                    *color,
                );
                fill_rect_bgra(pixels, width, height, tile_y, *x, *y, t, *h, *color);
                fill_rect_bgra(
                    pixels,
                    width,
                    height,
                    tile_y,
                    x + w.saturating_sub(t),
                    *y,
                    t,
                    *h,
                    *color,
                );
            }
            edgerun_layout::UiRenderCommand::Text {
                x,
                y,
                text,
                color,
                font_size,
                ..
            } => {
                draw_text_bgra(
                    pixels, width, height, tile_y, *x, *y, text, *color, *font_size,
                );
            }
            edgerun_layout::UiRenderCommand::RoundedRect {
                x,
                y,
                w,
                h,
                color,
                radius,
            } => {
                fill_rounded_rect_bgra(
                    pixels, width, height, tile_y, *x, *y, *w, *h, *radius, *color,
                );
            }
            _ => {}
        }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn fill_rect_bgra(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    tile_y: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    color: edgerun_layout::Color,
) {
    if w == 0 || h == 0 || color.a == 0 {
        return;
    }
    let x0 = x.min(width);
    let x1 = x.saturating_add(w).min(width);
    let y0 = y.max(tile_y);
    let y1 = y.saturating_add(h).min(tile_y.saturating_add(height));
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let mut py = y0;
    while py < y1 {
        let local_y = py - tile_y;
        let row = local_y as usize * width as usize * 4;
        let mut px = x0;
        while px < x1 {
            set_pixel_bgra(pixels, row + px as usize * 4, color);
            px += 1;
        }
        py += 1;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn fill_rounded_rect_bgra(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    tile_y: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    radius: u32,
    color: edgerun_layout::Color,
) {
    if radius == 0 {
        fill_rect_bgra(pixels, width, height, tile_y, x, y, w, h, color);
        return;
    }

    let x0 = x.min(width);
    let x1 = x.saturating_add(w).min(width);
    let y0 = y.max(tile_y);
    let y1 = y.saturating_add(h).min(tile_y.saturating_add(height));
    let r = radius.min(w / 2).min(h / 2);
    let r2 = (r * r) as i32;

    let mut py = y0;
    while py < y1 {
        let local_y = py - tile_y;
        let row = local_y as usize * width as usize * 4;
        let mut px = x0;
        while px < x1 {
            let in_corner = (px < x + r || px >= x + w - r) && (py < y + r || py >= y + h - r);
            let draw = if !in_corner {
                true
            } else {
                let cx = if px < x + r { x + r } else { x + w - r - 1 };
                let cy = if py < y + r { y + r } else { y + h - r - 1 };
                let dx = px as i32 - cx as i32;
                let dy = py as i32 - cy as i32;
                dx * dx + dy * dy <= r2
            };
            if draw {
                set_pixel_bgra(pixels, row + px as usize * 4, color);
            }
            px += 1;
        }
        py += 1;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn draw_text_bgra(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    tile_y: u32,
    x: u32,
    baseline_y: u32,
    text: &str,
    color: edgerun_layout::Color,
    font_size: f32,
) {
    let scale = ((font_size as u32) / 8).max(1);
    let glyph_w = 5 * scale;
    let glyph_h = 7 * scale;
    let mut cursor_x = x;
    let top = baseline_y.saturating_sub(glyph_h);
    for byte in text.bytes() {
        draw_glyph_bgra(
            pixels, width, height, tile_y, cursor_x, top, scale, byte, color,
        );
        cursor_x = cursor_x.saturating_add(glyph_w + scale);
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn draw_glyph_bgra(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    tile_y: u32,
    x: u32,
    y: u32,
    scale: u32,
    byte: u8,
    color: edgerun_layout::Color,
) {
    let glyph = glyph_5x7(byte);
    let mut gy = 0u32;
    while gy < 7 {
        let bits = glyph[gy as usize];
        let mut gx = 0u32;
        while gx < 5 {
            if (bits & (1 << (4 - gx))) != 0 {
                fill_rect_bgra(
                    pixels,
                    width,
                    height,
                    tile_y,
                    x + gx * scale,
                    y + gy * scale,
                    scale,
                    scale,
                    color,
                );
            }
            gx += 1;
        }
        gy += 1;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn glyph_5x7(byte: u8) -> [u8; 7] {
    match byte {
        b'0' => [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e],
        b'1' => [0x04, 0x0c, 0x04, 0x04, 0x04, 0x04, 0x0e],
        b'2' => [0x0e, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1f],
        b'3' => [0x1e, 0x01, 0x01, 0x0e, 0x01, 0x01, 0x1e],
        b'4' => [0x02, 0x06, 0x0a, 0x12, 0x1f, 0x02, 0x02],
        b'5' => [0x1f, 0x10, 0x10, 0x1e, 0x01, 0x01, 0x1e],
        b'6' => [0x0e, 0x10, 0x10, 0x1e, 0x11, 0x11, 0x0e],
        b'7' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        b'8' => [0x0e, 0x11, 0x11, 0x0e, 0x11, 0x11, 0x0e],
        b'9' => [0x0e, 0x11, 0x11, 0x0f, 0x01, 0x01, 0x0e],
        b'A' | b'a' => [0x0e, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        b'B' | b'b' => [0x1e, 0x11, 0x11, 0x1e, 0x11, 0x11, 0x1e],
        b'C' | b'c' => [0x0f, 0x10, 0x10, 0x10, 0x10, 0x10, 0x0f],
        b'D' | b'd' => [0x1e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1e],
        b'E' | b'e' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x1f],
        b'F' | b'f' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x10],
        b'G' | b'g' => [0x0f, 0x10, 0x10, 0x13, 0x11, 0x11, 0x0f],
        b'H' | b'h' => [0x11, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        b'I' | b'i' => [0x0e, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0e],
        b'J' | b'j' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0e],
        b'K' | b'k' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        b'L' | b'l' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f],
        b'M' | b'm' => [0x11, 0x1b, 0x15, 0x15, 0x11, 0x11, 0x11],
        b'N' | b'n' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        b'O' | b'o' => [0x0e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        b'P' | b'p' => [0x1e, 0x11, 0x11, 0x1e, 0x10, 0x10, 0x10],
        b'Q' | b'q' => [0x0e, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0d],
        b'R' | b'r' => [0x1e, 0x11, 0x11, 0x1e, 0x14, 0x12, 0x11],
        b'S' | b's' => [0x0f, 0x10, 0x10, 0x0e, 0x01, 0x01, 0x1e],
        b'T' | b't' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'U' | b'u' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        b'V' | b'v' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0a, 0x04],
        b'W' | b'w' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1b, 0x11],
        b'X' | b'x' => [0x11, 0x11, 0x0a, 0x04, 0x0a, 0x11, 0x11],
        b'Y' | b'y' => [0x11, 0x11, 0x0a, 0x04, 0x04, 0x04, 0x04],
        b'Z' | b'z' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1f],
        b'-' => [0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00],
        b':' => [0x00, 0x04, 0x04, 0x00, 0x04, 0x04, 0x00],
        b'=' => [0x00, 0x00, 0x1f, 0x00, 0x1f, 0x00, 0x00],
        b'.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x0c],
        b'/' => [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10],
        b'_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1f],
        b'[' => [0x0e, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0e],
        b']' => [0x0e, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0e],
        b' ' => [0x00; 7],
        _ => [0x1f, 0x11, 0x15, 0x15, 0x15, 0x11, 0x1f],
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn set_pixel_bgra(pixels: &mut [u8], i: usize, color: edgerun_layout::Color) {
    if i + 3 >= pixels.len() {
        return;
    }
    if color.a == 255 {
        pixels[i] = color.b;
        pixels[i + 1] = color.g;
        pixels[i + 2] = color.r;
        pixels[i + 3] = color.a;
        return;
    }
    let a = color.a as u32;
    let inv_a = 255 - a;
    pixels[i] = ((color.b as u32 * a + pixels[i] as u32 * inv_a) / 255) as u8;
    pixels[i + 1] = ((color.g as u32 * a + pixels[i + 1] as u32 * inv_a) / 255) as u8;
    pixels[i + 2] = ((color.r as u32 * a + pixels[i + 2] as u32 * inv_a) / 255) as u8;
    pixels[i + 3] = 255;
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn paint_touch_marker(pixels: &mut [u8], width: u32, height: u32, tile_y: u32, x: u32, y: u32) {
    if y.saturating_add(6) < tile_y || y > tile_y.saturating_add(height).saturating_add(6) {
        return;
    }

    let local_y = y.saturating_sub(tile_y);
    let x0 = x.saturating_sub(5);
    let y0 = local_y.saturating_sub(5);
    let x1 = x.saturating_add(6).min(width);
    let y1 = local_y.saturating_add(6).min(height);
    let mut py = y0;
    while py < y1 {
        let row = py as usize * width as usize * 4;
        let mut px = x0;
        while px < x1 {
            let i = row + px as usize * 4;
            pixels[i] = 0x00;
            pixels[i + 1] = 0x00;
            pixels[i + 2] = 0xff;
            pixels[i + 3] = 0xff;
            px += 1;
        }
        py += 1;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn render_debug_pattern(touch: Option<(u16, u16)>) {
    unsafe {
        edgerun_platform::esp32s3::Jc3248w535Display::draw_rgb565_with(320, 480, |x, y| {
            if let Some(center) = touch {
                return touch_overlay_pixel(x, y, center, TOUCH_OVERLAY_ALPHA);
            }
            if let Some(color) = console_pixel(x, y) {
                return color;
            }
            debug_pixel(x, y)
        });
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    (((r as u16) & 0xf8) << 8) | (((g as u16) & 0xfc) << 3) | (b as u16 >> 3)
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn min_u16(a: u16, b: u16) -> u16 {
    if a < b {
        a
    } else {
        b
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn debug_pixel(x: u16, y: u16) -> u16 {
    if x == 0 || y == 0 || x == 319 || y == 479 || x == 159 || y == 239 {
        return 0x0000;
    }

    if y < 80 {
        return match x / 40 {
            0 => 0xf800,
            1 => 0x07e0,
            2 => 0x001f,
            3 => 0xffe0,
            4 => 0xf81f,
            5 => 0x07ff,
            6 => 0xffff,
            _ => 0x0000,
        };
    }

    if y < 160 {
        return rgb565(
            (x as u32 * 255 / 319) as u8,
            32,
            255u8.saturating_sub((x as u32 * 255 / 319) as u8),
        );
    }

    if y < 240 {
        return rgb565(32, (x as u32 * 255 / 319) as u8, 32);
    }

    if y < 320 {
        let shade = (((x / 16) + (y / 16)) & 1) as u8;
        return if shade == 0 { 0xffff } else { 0x8410 };
    }

    if y < 400 {
        return rgb565(
            (y as u32 * 255 / 479) as u8,
            (x as u32 * 255 / 319) as u8,
            96,
        );
    }

    if x < 80 {
        0x001f
    } else if x < 160 {
        0x07e0
    } else if x < 240 {
        0xf800
    } else {
        0xffff
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
const fn fixed_cstr<const N: usize>(bytes: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    let mut i = 0;
    while i < bytes.len() && i + 1 < N {
        out[i] = bytes[i];
        i += 1;
    }
    out
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
#[used]
#[no_mangle]
#[link_section = ".flash.appdesc"]
static esp_app_desc: EspAppDesc = EspAppDesc {
    magic_word: 0xABCD_5432,
    secure_version: 0,
    reserv1: [0; 2],
    version: fixed_cstr(b"0.1.0"),
    project_name: fixed_cstr(b"edgerun-unikernel"),
    time: fixed_cstr(b"00:00:00"),
    date: fixed_cstr(b"2026-04-28"),
    idf_ver: fixed_cstr(b"edgerun-bare"),
    app_elf_sha256: [0; 32],
    min_efuse_blk_rev_full: 0,
    max_efuse_blk_rev_full: u16::MAX,
    mmu_page_size: 16,
    reserv3: [0; 3],
    reserv2: [0; 18],
};

#[cfg(target_arch = "x86_64")]
use edgerun_dhcp::message::{DHCP_CLIENT_PORT, DHCP_SERVER_PORT};
#[cfg(target_arch = "x86_64")]
use edgerun_dhcp::{DhcpMessage, DhcpMessageType};
#[cfg(target_arch = "x86_64")]
use edgerun_tftp::message::{TftpMessage, TFTP_PORT};
#[cfg(target_arch = "x86_64")]
use rt::ip::{ParsedPacket, ARP_OP_REQUEST, ICMP_ECHO_REQUEST};
#[cfg(target_arch = "x86_64")]
use rt::{block_on, crc32, IpAddr, IpStack, Network, RingBuffer, Rng, TcpSocket};

#[cfg(target_arch = "x86_64")]
use core::future::Future;
#[cfg(target_arch = "x86_64")]
use core::pin::Pin;
#[cfg(target_arch = "x86_64")]
use core::sync::atomic::{AtomicPtr, Ordering};
#[cfg(target_arch = "x86_64")]
use core::task::{Context, Poll};

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
#[allow(dead_code)]
mod oci_syscall {
    use super::rt;
    use edgerun_oci::prelude::String;
    use edgerun_oci::rootfs_access::OciRootfs;
    use edgerun_oci::{
        dispatch_x86_64_linux_syscall_frame, prepare_and_load_oci_elf_program_with_load_bias,
        OciElfError, OciElfLoadBias, OciElfUnsafeIdentityMapper, OciPreparedLaunchState,
        OciSyscallAction, OciSyscallError, OciSyscallMemory, OciSyscallSink, OciX86_64SyscallFrame,
    };
    use edgerun_platform::arch::x86_64::{
        self, SyscallFrame, KERNEL_CODE_SELECTOR, USER_COMPAT_CODE_SELECTOR,
    };

    struct DirectMemory;

    impl OciSyscallMemory for DirectMemory {
        fn read_bytes(&self, addr: u64, len: usize, out: &mut [u8]) -> Result<(), OciSyscallError> {
            let ptr = usize::try_from(addr).map_err(|_| OciSyscallError::BadAddress)? as *const u8;
            let Some(out) = out.get_mut(..len) else {
                return Err(OciSyscallError::BadAddress);
            };
            unsafe {
                core::ptr::copy_nonoverlapping(ptr, out.as_mut_ptr(), len);
            }
            Ok(())
        }
    }

    struct LogSink;

    impl OciSyscallSink for LogSink {
        fn write_fd(&mut self, fd: u64, bytes: &[u8]) -> Result<usize, OciSyscallError> {
            if fd == 1 || fd == 2 {
                if let Ok(text) = core::str::from_utf8(bytes) {
                    rt::log::log(1, text.trim_end_matches('\n'));
                } else {
                    rt::log::log(1, "container wrote non-UTF8 bytes");
                }
            }
            Ok(bytes.len())
        }
    }

    pub unsafe fn install(kernel_code_selector: u16, user_code_selector: u16) {
        x86_64::set_syscall_handler(handle_syscall);
        unsafe {
            x86_64::enable_syscall_entry(kernel_code_selector, user_code_selector);
        }
    }

    pub unsafe fn install_flat_gdt() {
        unsafe {
            x86_64::load_flat_gdt();
            install(KERNEL_CODE_SELECTOR, USER_COMPAT_CODE_SELECTOR);
        }
    }

    pub unsafe fn enter_launch_state(launch: &OciPreparedLaunchState) -> ! {
        unsafe {
            install_flat_gdt();
            x86_64::enter_user64(launch.entry_point, launch.stack_pointer);
        }
    }

    pub unsafe fn launch_rootfs<R: OciRootfs>(
        rootfs: &R,
        args: &[String],
        env: &[String],
        cwd: &str,
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciElfError> {
        let mut mapper = OciElfUnsafeIdentityMapper::new();
        let Some(launch) = prepare_and_load_oci_elf_program_with_load_bias(
            rootfs,
            args,
            env,
            cwd,
            &mut mapper,
            scratch,
            stack_base,
            stack_top,
            stack,
            load_bias,
        )?
        else {
            return Err(OciElfError::NotFound(
                args.first().cloned().unwrap_or_default(),
            ));
        };

        unsafe {
            enter_launch_state(&launch);
        }
    }

    extern "C" fn handle_syscall(frame: &mut SyscallFrame) {
        let mut oci_frame = OciX86_64SyscallFrame {
            rax: frame.rax,
            rdi: frame.rdi,
            rsi: frame.rsi,
            rdx: frame.rdx,
            r10: frame.r10,
            r8: frame.r8,
            r9: frame.r9,
        };
        let memory = DirectMemory;
        let mut sink = LogSink;
        let mut scratch = [0u8; 256];

        match dispatch_x86_64_linux_syscall_frame(&memory, &mut sink, &mut scratch, &mut oci_frame)
        {
            Ok(OciSyscallAction::Return(_)) => {
                frame.rax = oci_frame.rax;
            }
            Ok(OciSyscallAction::Exit(code)) => {
                let _ = code;
                rt::log::log(1, "container exited");
                loop {
                    unsafe {
                        core::arch::asm!("hlt");
                    }
                }
            }
            Err(OciSyscallError::Unsupported(_)) => {
                frame.rax = (-38i64) as u64;
            }
            Err(OciSyscallError::BadAddress) => {
                frame.rax = (-14i64) as u64;
            }
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
#[allow(dead_code)]
mod oci_image_boot {
    use super::boot_config::BootConfig;
    use super::disk_boot::{self, DiskBootError, OpenedEdgeFs};
    use super::oci_syscall;
    use edgerun_edgefs::EdgeFs;
    use edgerun_oci::prelude::String;
    use edgerun_oci::{
        EdgeFsImagePullReport, ImageRef, OciElfError, OciElfLoadBias, RegistryClient, RegistryError,
    };
    use edgerun_storage::{BlockStorage, PartitionBlockDevice};

    #[derive(Debug)]
    pub enum OciImageBootError {
        InvalidImageRef(String),
        Registry(RegistryError),
        Elf(OciElfError),
    }

    #[derive(Debug)]
    pub enum OciDiskImageBootError {
        Disk(DiskBootError),
        Image(OciImageBootError),
    }

    pub enum PulledConfiguredEdgeFs<S: BlockStorage> {
        Partition {
            fs: EdgeFs<PartitionBlockDevice<S>>,
            report: EdgeFsImagePullReport,
            config: BootConfig,
        },
        WholeDisk {
            fs: EdgeFs<S>,
            report: EdgeFsImagePullReport,
            config: BootConfig,
        },
    }

    impl From<RegistryError> for OciImageBootError {
        fn from(error: RegistryError) -> Self {
            Self::Registry(error)
        }
    }

    impl From<OciElfError> for OciImageBootError {
        fn from(error: OciElfError) -> Self {
            Self::Elf(error)
        }
    }

    pub fn image_boot_error_label(error: &OciImageBootError) -> &'static str {
        match error {
            OciImageBootError::InvalidImageRef(_) => "invalid image ref",
            OciImageBootError::Registry(RegistryError::HttpStatus(_)) => "registry http status",
            OciImageBootError::Registry(RegistryError::HttpError(_)) => "registry http error",
            OciImageBootError::Registry(RegistryError::AuthError(_)) => "registry auth error",
            OciImageBootError::Registry(RegistryError::TrustPolicy(_)) => "registry trust policy",
            OciImageBootError::Registry(RegistryError::ManifestNotFound(_)) => {
                "registry manifest not found"
            }
            OciImageBootError::Registry(RegistryError::NoManifests) => "registry no manifests",
            OciImageBootError::Registry(RegistryError::DigestMismatch { .. }) => {
                "registry digest mismatch"
            }
            OciImageBootError::Registry(RegistryError::DescriptorSizeMismatch { .. }) => {
                "registry descriptor size mismatch"
            }
            OciImageBootError::Registry(RegistryError::ParseError(_)) => "registry parse error",
            OciImageBootError::Elf(_) => "elf error",
        }
    }

    pub fn image_boot_error_detail(error: &OciImageBootError) -> Option<&str> {
        match error {
            OciImageBootError::InvalidImageRef(detail) => Some(detail.as_str()),
            OciImageBootError::Registry(RegistryError::HttpError(detail)) => Some(detail.as_str()),
            OciImageBootError::Registry(RegistryError::AuthError(detail)) => Some(detail.as_str()),
            OciImageBootError::Registry(RegistryError::TrustPolicy(detail)) => {
                Some(detail.as_str())
            }
            OciImageBootError::Registry(RegistryError::ManifestNotFound(detail)) => {
                Some(detail.as_str())
            }
            OciImageBootError::Registry(RegistryError::ParseError(detail)) => Some(detail.as_str()),
            _ => None,
        }
    }

    impl From<DiskBootError> for OciDiskImageBootError {
        fn from(error: DiskBootError) -> Self {
            Self::Disk(error)
        }
    }

    impl From<OciImageBootError> for OciDiskImageBootError {
        fn from(error: OciImageBootError) -> Self {
            Self::Image(error)
        }
    }

    pub async unsafe fn pull_image_into_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        image_ref: &str,
        rootfs_path: &str,
        registry_insecure_http: bool,
    ) -> Result<edgerun_oci::EdgeFsImagePullReport, OciImageBootError> {
        let image: ImageRef = image_ref
            .parse()
            .map_err(OciImageBootError::InvalidImageRef)?;
        let mut client = RegistryClient::new();
        if registry_insecure_http {
            client = client.insecure_http();
        }
        client
            .pull_into_edgefs(&image, rootfs_path, fs)
            .await
            .map_err(Into::into)
    }

    pub async unsafe fn pull_configured_image_into_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        config: &BootConfig,
    ) -> Result<edgerun_oci::EdgeFsImagePullReport, OciImageBootError> {
        unsafe {
            pull_image_into_edgefs(
                fs,
                &config.image,
                &config.rootfs_path,
                config.registry_insecure_http,
            )
            .await
        }
    }

    pub async unsafe fn pull_configured_disk_image<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<PulledConfiguredEdgeFs<S>, OciDiskImageBootError> {
        let mut device = device;
        let config = disk_boot::read_default_fat_boot_config_mut(&mut device)?;
        let opened = disk_boot::open_configured_edgefs(device, &config, key, fs_id)?;

        match opened {
            OpenedEdgeFs::Partition(mut fs) => {
                let report = unsafe { pull_configured_image_into_edgefs(&mut fs, &config).await? };
                Ok(PulledConfiguredEdgeFs::Partition { fs, report, config })
            }
            OpenedEdgeFs::WholeDisk(mut fs) => {
                let report = unsafe { pull_configured_image_into_edgefs(&mut fs, &config).await? };
                Ok(PulledConfiguredEdgeFs::WholeDisk { fs, report, config })
            }
        }
    }

    pub unsafe fn launch_pulled_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        report: &EdgeFsImagePullReport,
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciImageBootError> {
        let runtime = &report.plan.runtime;
        unsafe {
            oci_syscall::launch_rootfs(
                fs,
                &runtime.args,
                &runtime.env,
                runtime.cwd.as_str(),
                scratch,
                stack_base,
                stack_top,
                stack,
                load_bias,
            )
            .map_err(Into::into)
        }
    }

    pub async unsafe fn pull_and_launch_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        image_ref: &str,
        rootfs_path: &str,
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciImageBootError> {
        let report = unsafe { pull_image_into_edgefs(fs, image_ref, rootfs_path, false).await? };
        unsafe {
            launch_pulled_edgefs(
                fs, &report, scratch, stack_base, stack_top, stack, load_bias,
            )
        }
    }

    pub async unsafe fn pull_and_launch_configured_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        config: &BootConfig,
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciImageBootError> {
        unsafe {
            pull_and_launch_edgefs(
                fs,
                &config.image,
                &config.rootfs_path,
                scratch,
                stack_base,
                stack_top,
                stack,
                load_bias,
            )
            .await
        }
    }

    pub async unsafe fn pull_and_launch_configured_disk_image<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciDiskImageBootError> {
        let pulled = unsafe { pull_configured_disk_image(device, key, fs_id).await? };
        match pulled {
            PulledConfiguredEdgeFs::Partition { mut fs, report, .. } => unsafe {
                launch_pulled_edgefs(
                    &mut fs, &report, scratch, stack_base, stack_top, stack, load_bias,
                )
                .map_err(Into::into)
            },
            PulledConfiguredEdgeFs::WholeDisk { mut fs, report, .. } => unsafe {
                launch_pulled_edgefs(
                    &mut fs, &report, scratch, stack_base, stack_top, stack, load_bias,
                )
                .map_err(Into::into)
            },
        }
    }
}

#[cfg(any(target_os = "none", test))]
#[allow(dead_code)]
mod boot_config {
    use alloc::string::{String, ToString};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EdgeFsBootTarget {
        ExistingPartition,
        FormatFirstPartition,
        FormatDataPartition,
        FormatWholeDisk,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct BootConfig {
        pub image: String,
        pub edgefs: EdgeFsBootTarget,
        pub rootfs_path: String,
        pub http_smoke: Option<String>,
        pub oci_pull: bool,
        pub registry_insecure_http: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum BootConfigError {
        InvalidUtf8,
        InvalidLine(usize),
        UnknownKey(String),
        InvalidEdgeFsTarget(String),
        InvalidRootfsPath(String),
        MissingImage,
    }

    impl core::fmt::Display for BootConfigError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::InvalidUtf8 => f.write_str("boot config is not valid UTF-8"),
                Self::InvalidLine(line) => write!(f, "invalid boot config line {line}"),
                Self::UnknownKey(key) => write!(f, "unknown boot config key: {key}"),
                Self::InvalidEdgeFsTarget(target) => {
                    write!(f, "invalid edgefs boot target: {target}")
                }
                Self::InvalidRootfsPath(path) => write!(f, "invalid rootfs path: {path}"),
                Self::MissingImage => f.write_str("boot config is missing image"),
            }
        }
    }

    impl core::error::Error for BootConfigError {}

    impl BootConfig {
        pub fn parse(bytes: &[u8]) -> Result<Self, BootConfigError> {
            let text = core::str::from_utf8(bytes).map_err(|_| BootConfigError::InvalidUtf8)?;
            let mut image = None;
            let mut edgefs = EdgeFsBootTarget::ExistingPartition;
            let mut rootfs_path = "/".to_string();
            let mut http_smoke = None;
            let mut oci_pull = false;
            let mut registry_insecure_http = false;

            for (index, raw_line) in text.lines().enumerate() {
                let line_no = index + 1;
                let line = raw_line
                    .split_once('#')
                    .map_or(raw_line, |(line, _)| line)
                    .trim();
                if line.is_empty() {
                    continue;
                }

                let Some((key, value)) = line.split_once('=') else {
                    return Err(BootConfigError::InvalidLine(line_no));
                };
                let key = key.trim();
                let value = value.trim();
                if value.is_empty() {
                    return Err(BootConfigError::InvalidLine(line_no));
                }

                match key {
                    "image" | "oci_image" | "ref" => image = Some(value.to_string()),
                    "edgefs" => edgefs = parse_edgefs_target(value)?,
                    "rootfs" | "rootfs_path" => rootfs_path = parse_rootfs_path(value)?,
                    "http_smoke" | "http_smoke_url" => http_smoke = Some(value.to_string()),
                    "oci_pull" | "pull_image" => oci_pull = parse_bool(value, line_no)?,
                    "registry_insecure_http" | "plain_http_registry" => {
                        registry_insecure_http = parse_bool(value, line_no)?
                    }
                    other => return Err(BootConfigError::UnknownKey(other.to_string())),
                }
            }

            let image = image.ok_or(BootConfigError::MissingImage)?;
            Ok(Self {
                image,
                edgefs,
                rootfs_path,
                http_smoke,
                oci_pull,
                registry_insecure_http,
            })
        }
    }

    fn parse_bool(value: &str, line_no: usize) -> Result<bool, BootConfigError> {
        match value {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(BootConfigError::InvalidLine(line_no)),
        }
    }

    fn parse_edgefs_target(value: &str) -> Result<EdgeFsBootTarget, BootConfigError> {
        match value {
            "partition" | "existing-partition" | "existing_partition" => {
                Ok(EdgeFsBootTarget::ExistingPartition)
            }
            "format-partition" | "format_partition" | "first-partition" | "first_partition" => {
                Ok(EdgeFsBootTarget::FormatFirstPartition)
            }
            "format-data-partition"
            | "format_data_partition"
            | "data-partition"
            | "data_partition"
            | "first-data-partition"
            | "first_data_partition" => Ok(EdgeFsBootTarget::FormatDataPartition),
            "whole-disk" | "whole_disk" | "format-whole-disk" | "format_whole_disk" => {
                Ok(EdgeFsBootTarget::FormatWholeDisk)
            }
            other => Err(BootConfigError::InvalidEdgeFsTarget(other.to_string())),
        }
    }

    fn parse_rootfs_path(value: &str) -> Result<String, BootConfigError> {
        if !value.starts_with('/') || value.as_bytes().contains(&0) {
            return Err(BootConfigError::InvalidRootfsPath(value.to_string()));
        }

        let without_root = &value[1..];
        if without_root.is_empty() {
            return Ok(value.to_string());
        }

        for component in without_root.split('/') {
            if component.is_empty() || component == "." || component == ".." {
                return Err(BootConfigError::InvalidRootfsPath(value.to_string()));
            }
        }

        Ok(value.to_string())
    }

    #[cfg(test)]
    mod tests {
        use super::{BootConfig, EdgeFsBootTarget};

        #[test]
        fn parses_required_image_and_defaults() {
            let config = BootConfig::parse(b"# edgeOS\nimage=registry.local/app:latest\n").unwrap();
            assert_eq!(config.image, "registry.local/app:latest");
            assert_eq!(config.edgefs, EdgeFsBootTarget::ExistingPartition);
            assert_eq!(config.rootfs_path, "/");
            assert_eq!(config.http_smoke, None);
            assert!(!config.oci_pull);
            assert!(!config.registry_insecure_http);
        }

        #[test]
        fn parses_edgefs_and_rootfs_options() {
            let config = BootConfig::parse(
                b"oci_image = example.com/ns/app:v1\nedgefs = whole-disk\nrootfs_path = /apps/app\n",
            )
            .unwrap();
            assert_eq!(config.image, "example.com/ns/app:v1");
            assert_eq!(config.edgefs, EdgeFsBootTarget::FormatWholeDisk);
            assert_eq!(config.rootfs_path, "/apps/app");
        }

        #[test]
        fn parses_data_partition_target() {
            let config =
                BootConfig::parse(b"image=registry.local/app:v1\nedgefs=format-data-partition\n")
                    .unwrap();

            assert_eq!(config.edgefs, EdgeFsBootTarget::FormatDataPartition);
        }

        #[test]
        fn parses_http_smoke_url() {
            let config = BootConfig::parse(
                b"image=registry.local/app:v1\nhttp_smoke=http://10.0.2.2:18080/edgerun-smoke\n",
            )
            .unwrap();

            assert_eq!(
                config.http_smoke.as_deref(),
                Some("http://10.0.2.2:18080/edgerun-smoke")
            );
        }

        #[test]
        fn parses_oci_pull_options() {
            let config = BootConfig::parse(
                b"image=10.0.2.2:18080/edge/app:v1\noci_pull=true\nregistry_insecure_http=yes\n",
            )
            .unwrap();

            assert!(config.oci_pull);
            assert!(config.registry_insecure_http);
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
#[allow(dead_code)]
mod disk_boot {
    use super::boot_config::{BootConfig, BootConfigError, EdgeFsBootTarget};
    use edgerun_edgefs::{EdgeFs, EdgeFsError, EdgeFsInfo};
    use edgerun_storage::{
        detect_partitions, probe_filesystem, BlockStorage, FatError, FatReadOnly, FileSystemKind,
        FileSystemProbe, FileSystemProbeError, PartitionBlockDevice, PartitionEntry,
        PartitionError, PartitionTable, StorageError,
    };

    pub const DEFAULT_BOOT_CONFIG_PATHS: &[&str] =
        &["/edgerun/boot.cfg", "/EDGERUN/BOOT.CFG", "/boot.cfg"];

    #[derive(Debug)]
    pub enum DiskBootError {
        Partition(PartitionError),
        Probe(FileSystemProbeError),
        Fat(FatError),
        BootConfig(BootConfigError),
        EdgeFs(EdgeFsError),
        NoEdgeFsPartition,
        NoFatPartition,
        NoBootConfig,
        NoPartition,
    }

    impl From<PartitionError> for DiskBootError {
        fn from(error: PartitionError) -> Self {
            Self::Partition(error)
        }
    }

    impl From<EdgeFsError> for DiskBootError {
        fn from(error: EdgeFsError) -> Self {
            Self::EdgeFs(error)
        }
    }

    impl From<FileSystemProbeError> for DiskBootError {
        fn from(error: FileSystemProbeError) -> Self {
            Self::Probe(error)
        }
    }

    impl From<FatError> for DiskBootError {
        fn from(error: FatError) -> Self {
            Self::Fat(error)
        }
    }

    impl From<BootConfigError> for DiskBootError {
        fn from(error: BootConfigError) -> Self {
            Self::BootConfig(error)
        }
    }

    #[derive(Debug, Clone)]
    pub struct DiskPartitionProbe {
        pub partition: PartitionEntry,
        pub filesystem: FileSystemProbe,
    }

    #[derive(Debug, Clone)]
    pub struct EdgeFsPartition {
        pub partition: PartitionEntry,
        pub info: EdgeFsInfo,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MountPlanKind {
        EdgeFsReadWrite,
        ForeignReadOnly,
        FutureNative,
        Unknown,
    }

    #[derive(Debug, Clone)]
    pub struct MountPlanEntry {
        pub partition: PartitionEntry,
        pub filesystem: FileSystemProbe,
        pub edgefs: Option<EdgeFsInfo>,
        pub kind: MountPlanKind,
    }

    #[derive(Debug, Clone)]
    pub struct DiskMountPlan {
        pub table: PartitionTable,
        pub entries: alloc::vec::Vec<MountPlanEntry>,
    }

    pub enum OpenedEdgeFs<S: BlockStorage> {
        Partition(EdgeFs<PartitionBlockDevice<S>>),
        WholeDisk(EdgeFs<S>),
    }

    pub struct RtBlockDeviceStorage<T> {
        device: T,
    }

    impl<T> RtBlockDeviceStorage<T> {
        pub fn new(device: T) -> Self {
            Self { device }
        }

        pub fn into_inner(self) -> T {
            self.device
        }
    }

    impl BlockStorage for RtBlockDeviceStorage<edgerun_virtio::VirtBlk> {
        fn sector_size(&self) -> usize {
            edgerun_virtio::SECTOR_SIZE
        }

        fn sectors(&self) -> u64 {
            self.device.sectors()
        }

        fn read_sector(
            &mut self,
            sector: u64,
            buf: &mut [u8],
        ) -> core::result::Result<(), StorageError> {
            if buf.len() != self.sector_size() {
                return Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::InvalidInput,
                    "sector buffer size mismatch",
                )));
            }
            if self.device.read_sector(sector, buf) {
                Ok(())
            } else {
                Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::Other,
                    "bare block read failed",
                )))
            }
        }

        fn write_sector(
            &mut self,
            sector: u64,
            buf: &[u8],
        ) -> core::result::Result<(), StorageError> {
            if buf.len() != self.sector_size() {
                return Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::InvalidInput,
                    "sector buffer size mismatch",
                )));
            }
            if self.device.write_sector(sector, buf) {
                Ok(())
            } else {
                Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::Other,
                    "bare block write failed",
                )))
            }
        }
    }

    pub struct RtPartitionStorage<'a, S: BlockStorage> {
        parent: &'a mut S,
        start_lba: u64,
        sectors: u64,
    }

    impl<'a, S: BlockStorage> RtPartitionStorage<'a, S> {
        pub fn new(parent: &'a mut S, start_lba: u64, sectors: u64) -> Self {
            Self {
                parent,
                start_lba,
                sectors,
            }
        }
    }

    impl<S: BlockStorage> BlockStorage for RtPartitionStorage<'_, S> {
        fn sector_size(&self) -> usize {
            BlockStorage::sector_size(&*self.parent)
        }

        fn sectors(&self) -> u64 {
            self.sectors
        }

        fn read_sector(
            &mut self,
            sector: u64,
            buf: &mut [u8],
        ) -> core::result::Result<(), StorageError> {
            if sector >= self.sectors {
                return Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::UnexpectedEof,
                    "partition sector index out of range",
                )));
            }
            let Some(parent_sector) = self.start_lba.checked_add(sector) else {
                return Err(StorageError::Io(edgerun_storage::io::Error::other(
                    "partition sector overflow",
                )));
            };
            BlockStorage::read_sector(&mut *self.parent, parent_sector, buf)
        }

        fn write_sector(
            &mut self,
            sector: u64,
            buf: &[u8],
        ) -> core::result::Result<(), StorageError> {
            if sector >= self.sectors {
                return Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::UnexpectedEof,
                    "partition sector index out of range",
                )));
            }
            let Some(parent_sector) = self.start_lba.checked_add(sector) else {
                return Err(StorageError::Io(edgerun_storage::io::Error::other(
                    "partition sector overflow",
                )));
            };
            BlockStorage::write_sector(&mut *self.parent, parent_sector, buf)
        }

        fn sync(&mut self) -> core::result::Result<(), StorageError> {
            BlockStorage::sync(&mut *self.parent)
        }
    }

    pub fn scan_partition_filesystems<S: BlockStorage>(
        device: &mut S,
    ) -> Result<(PartitionTable, alloc::vec::Vec<DiskPartitionProbe>), DiskBootError> {
        let table = detect_partitions(device)?;
        let mut probes = alloc::vec::Vec::new();
        for partition in &table.partitions {
            let mut slice =
                PartitionBlockDevice::new(&mut *device, partition.start_lba, partition.sectors)?;
            probes.push(DiskPartitionProbe {
                partition: partition.clone(),
                filesystem: probe_filesystem(&mut slice)?,
            });
        }
        Ok((table, probes))
    }

    pub fn plan_partition_mounts<S: BlockStorage>(
        device: &mut S,
        key: [u8; 32],
    ) -> Result<DiskMountPlan, DiskBootError> {
        let table = detect_partitions(device)?;
        let mut entries = alloc::vec::Vec::new();
        for partition in &table.partitions {
            let mut slice =
                PartitionBlockDevice::new(&mut *device, partition.start_lba, partition.sectors)?;
            let filesystem = probe_filesystem(&mut slice)?;

            let edgefs = if filesystem.kind == FileSystemKind::EdgeFs {
                let slice = PartitionBlockDevice::new(
                    &mut *device,
                    partition.start_lba,
                    partition.sectors,
                )?;
                EdgeFs::probe(slice, key).ok()
            } else {
                None
            };

            let kind = match (filesystem.kind, edgefs.is_some()) {
                (FileSystemKind::EdgeFs, true) => MountPlanKind::EdgeFsReadWrite,
                (FileSystemKind::Fat12 | FileSystemKind::Fat16 | FileSystemKind::Fat32, _) => {
                    MountPlanKind::ForeignReadOnly
                }
                (FileSystemKind::ExFat | FileSystemKind::Iso9660, _) => {
                    MountPlanKind::ForeignReadOnly
                }
                (FileSystemKind::Ext, _) => MountPlanKind::FutureNative,
                _ => MountPlanKind::Unknown,
            };

            entries.push(MountPlanEntry {
                partition: partition.clone(),
                filesystem,
                edgefs,
                kind,
            });
        }

        Ok(DiskMountPlan { table, entries })
    }

    pub fn scan_edgefs_partitions<S: BlockStorage>(
        device: &mut S,
        key: [u8; 32],
    ) -> Result<(PartitionTable, alloc::vec::Vec<EdgeFsPartition>), DiskBootError> {
        let table = detect_partitions(device)?;
        let mut matches = alloc::vec::Vec::new();
        for partition in &table.partitions {
            let slice =
                PartitionBlockDevice::new(&mut *device, partition.start_lba, partition.sectors)?;
            if let Ok(info) = EdgeFs::probe(slice, key) {
                matches.push(EdgeFsPartition {
                    partition: partition.clone(),
                    info,
                });
            }
        }
        Ok((table, matches))
    }

    pub fn open_first_edgefs_partition<S: BlockStorage>(
        device: S,
        key: [u8; 32],
    ) -> Result<EdgeFs<PartitionBlockDevice<S>>, DiskBootError> {
        let mut scan_device = device;
        let (_, matches) = scan_edgefs_partitions(&mut scan_device, key)?;
        let Some(first) = matches.first() else {
            return Err(DiskBootError::NoEdgeFsPartition);
        };
        let partition = PartitionBlockDevice::new(
            scan_device,
            first.partition.start_lba,
            first.partition.sectors,
        )?;
        EdgeFs::open(partition, key).map_err(Into::into)
    }

    pub fn open_whole_disk_edgefs<S: BlockStorage>(
        device: S,
        key: [u8; 32],
    ) -> Result<EdgeFs<S>, DiskBootError> {
        EdgeFs::open(device, key).map_err(Into::into)
    }

    pub fn open_first_fat_partition_readonly<S: BlockStorage>(
        device: S,
    ) -> Result<FatReadOnly<PartitionBlockDevice<S>>, DiskBootError> {
        let mut scan_device = device;
        let (_, probes) = scan_partition_filesystems(&mut scan_device)?;
        let Some(first) = probes.iter().find(|probe| {
            matches!(
                probe.filesystem.kind,
                FileSystemKind::Fat12 | FileSystemKind::Fat16 | FileSystemKind::Fat32
            )
        }) else {
            return Err(DiskBootError::NoFatPartition);
        };
        let partition = PartitionBlockDevice::new(
            scan_device,
            first.partition.start_lba,
            first.partition.sectors,
        )?;
        FatReadOnly::open(partition).map_err(Into::into)
    }

    pub fn read_first_fat_file_8_3<S: BlockStorage>(
        device: S,
        path: &str,
    ) -> Result<alloc::vec::Vec<u8>, DiskBootError> {
        read_first_fat_file(device, path)
    }

    pub fn read_first_fat_file<S: BlockStorage>(
        device: S,
        path: &str,
    ) -> Result<alloc::vec::Vec<u8>, DiskBootError> {
        let mut fat = open_first_fat_partition_readonly(device)?;
        fat.read_file(path).map_err(Into::into)
    }

    pub fn read_first_fat_boot_config<S: BlockStorage>(
        device: S,
        path: &str,
    ) -> Result<BootConfig, DiskBootError> {
        let bytes = read_first_fat_file_8_3(device, path)?;
        BootConfig::parse(&bytes).map_err(Into::into)
    }

    pub fn read_default_fat_boot_config<S: BlockStorage>(
        device: S,
    ) -> Result<BootConfig, DiskBootError> {
        let mut fat = open_first_fat_partition_readonly(device)?;
        for path in DEFAULT_BOOT_CONFIG_PATHS {
            match fat.read_file(path) {
                Ok(bytes) => return BootConfig::parse(&bytes).map_err(Into::into),
                Err(FatError::NotFound(_)) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(DiskBootError::NoBootConfig)
    }

    pub fn read_default_fat_boot_config_mut<S: BlockStorage>(
        device: &mut S,
    ) -> Result<BootConfig, DiskBootError> {
        let mut fat = open_first_fat_partition_readonly(&mut *device)?;
        for path in DEFAULT_BOOT_CONFIG_PATHS {
            match fat.read_file(path) {
                Ok(bytes) => return BootConfig::parse(&bytes).map_err(Into::into),
                Err(FatError::NotFound(_)) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(DiskBootError::NoBootConfig)
    }

    pub fn open_configured_edgefs<S: BlockStorage>(
        device: S,
        config: &BootConfig,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<OpenedEdgeFs<S>, DiskBootError> {
        match config.edgefs {
            EdgeFsBootTarget::ExistingPartition => {
                open_first_edgefs_partition(device, key).map(OpenedEdgeFs::Partition)
            }
            EdgeFsBootTarget::FormatFirstPartition => {
                format_first_partition_as_edgefs(device, key, fs_id).map(OpenedEdgeFs::Partition)
            }
            EdgeFsBootTarget::FormatDataPartition => {
                format_first_data_partition_as_edgefs(device, key, fs_id)
                    .map(OpenedEdgeFs::Partition)
            }
            EdgeFsBootTarget::FormatWholeDisk => {
                format_whole_disk_edgefs(device, key, fs_id).map(OpenedEdgeFs::WholeDisk)
            }
        }
    }

    pub fn format_partition_as_edgefs<S: BlockStorage>(
        device: S,
        partition: &PartitionEntry,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<EdgeFs<PartitionBlockDevice<S>>, DiskBootError> {
        let partition = PartitionBlockDevice::new(device, partition.start_lba, partition.sectors)?;
        EdgeFs::format_with_id(partition, key, fs_id).map_err(Into::into)
    }

    pub fn format_first_partition_as_edgefs<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<EdgeFs<PartitionBlockDevice<S>>, DiskBootError> {
        let mut scan_device = device;
        let table = detect_partitions(&mut scan_device)?;
        let Some(first) = table.partitions.first() else {
            return Err(DiskBootError::NoPartition);
        };
        format_partition_as_edgefs(scan_device, first, key, fs_id)
    }

    pub fn format_first_data_partition_as_edgefs<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<EdgeFs<PartitionBlockDevice<S>>, DiskBootError> {
        let mut scan_device = device;
        let (_, probes) = scan_partition_filesystems(&mut scan_device)?;
        let Some(first) = probes.iter().find(|probe| {
            !matches!(
                probe.filesystem.kind,
                FileSystemKind::Fat12
                    | FileSystemKind::Fat16
                    | FileSystemKind::Fat32
                    | FileSystemKind::ExFat
                    | FileSystemKind::Iso9660
            )
        }) else {
            return Err(DiskBootError::NoPartition);
        };
        format_partition_as_edgefs(scan_device, &first.partition, key, fs_id)
    }

    pub fn format_whole_disk_edgefs<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<EdgeFs<S>, DiskBootError> {
        EdgeFs::format_with_id(device, key, fs_id).map_err(Into::into)
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
core::arch::global_asm!(
    r#"
    .section .text.entry,"ax"
    .global _start
_start:
    lea rsp, [rip + _stack]
    xor rbp, rbp

    lea rdi, [rip + _bss_start]
    lea rcx, [rip + _bss_end]
    sub rcx, rdi
    xor eax, eax
    rep stosb

    call kernel_main

1:
    hlt
    jmp 1b
"#
);

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
core::arch::global_asm!(
    r#"
    .section .text.entry,"ax"
    .align 4
.Lstack_ptr:
    .word _stack
.Lbss_start_ptr:
    .word _bss_start
.Lbss_end_ptr:
    .word _bss_end
.Luart0_fifo_ptr:
    .word 0x60000000
.Lusb_ep1_ptr:
    .word 0x60038000
.Lusb_ep1_conf_ptr:
    .word 0x60038004
.Lusb_conf0_ptr:
    .word 0x60038018
.Lsystem_perip_clk_en1_ptr:
    .word 0x600c001c
.Lsystem_perip_rst_en1_ptr:
    .word 0x600c0024
.Lusb_conf0_default:
    .word 0x4200
.Lkernel_main_ptr:
    .word kernel_main

    .global _start
_start:
    rsil a0, 15
    l32r a1, .Lstack_ptr
    l32r a2, .Lbss_start_ptr
    l32r a3, .Lbss_end_ptr
    sub a3, a3, a2
    srli a3, a3, 2
    movi a4, 0
1:
    beqz a3, 2f
    s32i a4, a2, 0
    addi a2, a2, 4
    addi a3, a3, -1
    j 1b
2:
    l32r a5, .Lsystem_perip_clk_en1_ptr
    l32i a6, a5, 0
    movi a7, 1
    slli a7, a7, 10
    or a6, a6, a7
    s32i a6, a5, 0
    l32r a5, .Lsystem_perip_rst_en1_ptr
    l32i a6, a5, 0
    movi a7, 1
    slli a7, a7, 10
    movi a8, -1
    xor a7, a7, a8
    and a6, a6, a7
    s32i a6, a5, 0
    l32r a5, .Lusb_conf0_ptr
    l32r a6, .Lusb_conf0_default
    s32i a6, a5, 0

    l32r a5, .Luart0_fifo_ptr
    movi a6, 69
    s32i a6, a5, 0
    movi a6, 82
    s32i a6, a5, 0
    movi a6, 10
    s32i a6, a5, 0

    l32r a5, .Lusb_ep1_ptr
    l32r a7, .Lusb_ep1_conf_ptr
    movi a6, 69
    s32i a6, a5, 0
    movi a6, 85
    s32i a6, a5, 0
    movi a6, 10
    s32i a6, a5, 0
    l32i a6, a7, 0
    movi a8, 1
    or a6, a6, a8
    s32i a6, a7, 0
    movi a6, 100
4:
    addi a6, a6, -1
    bnez a6, 4b

    call8 kernel_main
3:
    waiti 0
    j 3b
"#
);

#[cfg(target_arch = "x86_64")]
struct PumpStats {
    arp_replies: u32,
    icmp_replies: u32,
}

#[cfg(target_arch = "x86_64")]
enum BareNic {
    Rtl8125(edgerun_rtl8125::Rtl8125),
    Virtio(edgerun_virtio::VirtNet),
}

#[cfg(target_arch = "x86_64")]
struct BareNicStats {
    tx_completed: u32,
    rx_received: u32,
}

#[cfg(target_arch = "x86_64")]
impl BareNic {
    fn init(&mut self) -> bool {
        match self {
            Self::Rtl8125(net) => net.init(),
            Self::Virtio(net) => net.init(),
        }
    }

    fn send(&mut self, data: &[u8]) -> bool {
        match self {
            Self::Rtl8125(net) => net.send(data),
            Self::Virtio(net) => net.send(data),
        }
    }

    fn recv(&mut self, out: &mut [u8]) -> Option<usize> {
        match self {
            Self::Rtl8125(net) => net.recv(out),
            Self::Virtio(net) => net.recv(out),
        }
    }

    fn get_mac(&self) -> [u8; 6] {
        match self {
            Self::Rtl8125(net) => net.get_mac(),
            Self::Virtio(net) => net.get_mac(),
        }
    }

    fn is_link_up(&self) -> bool {
        match self {
            Self::Rtl8125(net) => net.is_link_up(),
            Self::Virtio(net) => net.is_link_up(),
        }
    }

    fn stats(&mut self) -> BareNicStats {
        match self {
            Self::Rtl8125(net) => {
                let stats = net.stats();
                BareNicStats {
                    tx_completed: stats.tx_completed,
                    rx_received: stats.rx_received,
                }
            }
            Self::Virtio(net) => {
                let stats = net.stats();
                BareNicStats {
                    tx_completed: stats.tx_completed,
                    rx_received: stats.rx_received,
                }
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
struct NetPump<'net, 'stack> {
    net: &'net mut BareNic,
    network: Network<'stack>,
    rx_buf: [u8; 1514],
    logged_start: bool,
    logged_arp: bool,
    logged_icmp: bool,
}

#[cfg(target_arch = "x86_64")]
async fn run_http_smoke(url: &str) {
    let client = edgerun_http::HttpClient::new()
        .version(edgerun_http::HttpVersion::Http1)
        .with_connect_timeout(edgerun_http::runtime::time::Duration::from_secs(3))
        .with_read_timeout(edgerun_http::runtime::time::Duration::from_secs(3));
    match client.get(url).await {
        Ok(response) if response.status().as_u16() == 200 => {
            rt::log::log(1, "Bare HTTP smoke GET ok")
        }
        Ok(_) => rt::log::log(1, "Bare HTTP smoke GET bad status"),
        Err(_) => rt::log::log(1, "Bare HTTP smoke GET failed"),
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
async fn run_configured_oci_pull(config: &boot_config::BootConfig) {
    if !config.oci_pull {
        return;
    }

    rt::log::log(1, "Bare OCI pull into EdgeFS start");
    let Some(mut block) = edgerun_virtio::find_virtio_blk() else {
        rt::log::log(1, "Bare OCI pull no VirtIO block device");
        return;
    };
    if !block.init() {
        rt::log::log(1, "Bare OCI pull VirtIO block init failed");
        return;
    }

    let mut storage = disk_boot::RtBlockDeviceStorage::new(block);
    let edgefs_key = [0x42u8; 32];
    let edgefs_fs_id = [0x24u8; 16];
    match disk_boot::open_configured_edgefs(&mut storage, config, edgefs_key, edgefs_fs_id) {
        Ok(disk_boot::OpenedEdgeFs::Partition(mut fs)) => {
            match unsafe {
                oci_image_boot::pull_configured_image_into_edgefs(&mut fs, config).await
            } {
                Ok(_) => rt::log::log(1, "Bare OCI pull into EdgeFS ok"),
                Err(error) => {
                    rt::log::log(1, "Bare OCI pull into EdgeFS failed");
                    rt::log::log(1, oci_image_boot::image_boot_error_label(&error));
                    if let Some(detail) = oci_image_boot::image_boot_error_detail(&error) {
                        rt::log::log(1, detail);
                    }
                }
            }
        }
        Ok(disk_boot::OpenedEdgeFs::WholeDisk(mut fs)) => {
            match unsafe {
                oci_image_boot::pull_configured_image_into_edgefs(&mut fs, config).await
            } {
                Ok(_) => rt::log::log(1, "Bare OCI pull into EdgeFS ok"),
                Err(error) => {
                    rt::log::log(1, "Bare OCI pull into EdgeFS failed");
                    rt::log::log(1, oci_image_boot::image_boot_error_label(&error));
                    if let Some(detail) = oci_image_boot::image_boot_error_detail(&error) {
                        rt::log::log(1, detail);
                    }
                }
            }
        }
        Err(_) => rt::log::log(1, "Bare OCI pull EdgeFS open failed"),
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
struct KernelBareNetDriver {
    net: AtomicPtr<BareNic>,
    stack: AtomicPtr<IpStack>,
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
unsafe impl Sync for KernelBareNetDriver {}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
impl KernelBareNetDriver {
    const fn empty() -> Self {
        Self {
            net: AtomicPtr::new(core::ptr::null_mut()),
            stack: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    fn install(&self, net: *mut BareNic, stack: *mut IpStack) {
        self.net.store(net, Ordering::Release);
        self.stack.store(stack, Ordering::Release);
    }

    fn net(&self) -> *mut BareNic {
        self.net.load(Ordering::Acquire)
    }

    fn stack(&self) -> *mut IpStack {
        self.stack.load(Ordering::Acquire)
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
impl rt::BareNetDriver for KernelBareNetDriver {
    fn local_ipv4(&self) -> [u8; 4] {
        unsafe { *(*self.stack()).ip.as_bytes() }
    }

    fn gateway_ipv4(&self) -> [u8; 4] {
        unsafe { *(*self.stack()).gateway.as_bytes() }
    }

    fn local_mac(&self) -> [u8; 6] {
        unsafe { (*self.stack()).mac }
    }

    fn lookup_arp(&self, ipv4: [u8; 4]) -> Option<[u8; 6]> {
        unsafe { (*self.stack()).arp.lookup(IpAddr::from_slice(&ipv4)) }
    }

    fn send_frame(&self, frame: &[u8]) -> bool {
        unsafe { (*self.net()).send(frame) }
    }

    fn recv_frame(&self, out: &mut [u8]) -> Option<usize> {
        let len = unsafe { (*self.net()).recv(out)? };
        if let Some(ParsedPacket::Arp { header, .. }) =
            Network::new(unsafe { &mut *self.stack() }).recv(&out[..len])
        {
            if header.oper == ARP_OP_REQUEST
                && header.tpa == unsafe { *(*self.stack()).ip.as_bytes() }
            {
                let mut network = Network::new(unsafe { &mut *self.stack() });
                if let Some(reply) = network.send_arp_reply(&header) {
                    let _ = unsafe { (*self.net()).send(reply) };
                }
            }
        }
        Some(len)
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
static BARE_NET_DRIVER: KernelBareNetDriver = KernelBareNetDriver::empty();

#[cfg(target_arch = "x86_64")]
fn poll_network(
    net: &mut BareNic,
    network: &mut Network<'_>,
    rx_buf: &mut [u8; 1514],
) -> PumpStats {
    let mut stats = PumpStats {
        arp_replies: 0,
        icmp_replies: 0,
    };

    while let Some(len) = net.recv(rx_buf) {
        match network.recv(&rx_buf[..len]) {
            Some(ParsedPacket::Arp { header, .. }) => {
                if header.oper == ARP_OP_REQUEST
                    && header.tpa == *network.stack.ip.as_bytes()
                    && network
                        .send_arp_reply(&header)
                        .map(|packet| net.send(packet))
                        .unwrap_or(false)
                {
                    stats.arp_replies = stats.arp_replies.wrapping_add(1);
                }
            }
            Some(ParsedPacket::Icmp {
                eth,
                ip,
                header,
                payload,
            }) => {
                if header.icmp_type == ICMP_ECHO_REQUEST
                    && ip.dst == *network.stack.ip.as_bytes()
                    && network
                        .send_icmp_echo_reply(
                            eth.src,
                            IpAddr::from_slice(&ip.src),
                            &header,
                            payload,
                        )
                        .map(|packet| net.send(packet))
                        .unwrap_or(false)
                {
                    stats.icmp_replies = stats.icmp_replies.wrapping_add(1);
                }
            }
            _ => {}
        }
    }

    stats
}

#[cfg(target_arch = "x86_64")]
fn dhcp_ipv4_to_rt(ip: edgerun_dhcp::Ipv4Addr) -> IpAddr {
    let octets = ip.octets();
    IpAddr::new(octets[0], octets[1], octets[2], octets[3])
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
unsafe fn probe_tpm2() -> Option<[u8; 32]> {
    rt::log::log(1, "Looking for TPM2 ACPI table...");
    match unsafe { edgerun_tpm::CrbTpmTransport::discover_acpi() } {
        Ok(Some(transport)) => {
            rt::log::log(1, "TPM2 CRB transport discovered");
            return probe_tpm2_transport(transport.with_timeout_polls(10));
        }
        Ok(None) => {
            rt::log::log(1, "No TPM2 ACPI table found");
        }
        Err(_) => {
            rt::log::log(1, "TPM2 ACPI discovery failed");
        }
    }

    rt::log::log(1, "Trying TPM2 TIS transport...");
    let transport = unsafe { edgerun_tpm::TisTpmTransport::new_default_x86() };
    probe_tpm2_transport(transport.with_timeout_polls(100_000))
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
fn fill_bare_random_source(out: &mut [u8]) -> edgerun_crypto::error::Result<()> {
    if out.is_empty() {
        return Ok(());
    }

    if let Some(mut rng) = edgerun_virtio::find_virtio_rng() {
        if rng.init() && rng.fill_bytes(out) {
            return Ok(());
        }
    }

    fill_tpm2_random_source(out)
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
fn fill_tpm2_random_source(out: &mut [u8]) -> edgerun_crypto::error::Result<()> {
    if out.is_empty() {
        return Ok(());
    }

    if let Ok(Some(transport)) = unsafe { edgerun_tpm::CrbTpmTransport::discover_acpi() } {
        if fill_tpm2_random_transport(transport.with_timeout_polls(10), out) {
            return Ok(());
        }
    }

    let transport = unsafe { edgerun_tpm::TisTpmTransport::new_default_x86() };
    if fill_tpm2_random_transport(transport.with_timeout_polls(100_000), out) {
        Ok(())
    } else {
        Err(edgerun_crypto::error::CryptoError::TpmUnavailable)
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
fn fill_tpm2_random_transport<T>(transport: T, out: &mut [u8]) -> bool
where
    T: edgerun_tpm::FixedTpmTransport + edgerun_tpm::TpmTransport,
{
    let mut device = edgerun_tpm::TpmDevice::new(transport);
    let startup_code = device.startup_response_code(edgerun_tpm::TPM_SU_CLEAR);
    if startup_code != edgerun_tpm::TPM_RC_SUCCESS && startup_code != 0x100 && startup_code != 0x120
    {
        return false;
    }

    let mut offset = 0usize;
    while offset < out.len() {
        let end = core::cmp::min(offset + 2048, out.len());
        let n = device.get_random_into(&mut out[offset..end]);
        if n == 0 || offset + n > end {
            return false;
        }
        offset += n;
    }
    true
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
fn probe_tpm2_transport<T>(transport: T) -> Option<[u8; 32]>
where
    T: edgerun_tpm::FixedTpmTransport + edgerun_tpm::TpmTransport,
{
    let mut device = edgerun_tpm::TpmDevice::new(transport);
    let startup_code = device.startup_response_code(edgerun_tpm::TPM_SU_CLEAR);
    if startup_code == edgerun_tpm::TPM_RC_SUCCESS || startup_code == 0x100 || startup_code == 0x120
    {
        rt::log::log(1, "TPM2 startup ok");
    } else if startup_code == 0x144 {
        rt::log::log(1, "TPM2 startup failed: command size");
        return None;
    } else if startup_code == 0xffff_fffb {
        rt::log::log(1, "TPM2 startup failed: transport");
        return None;
    } else if startup_code == 0xffff_fffc {
        rt::log::log(1, "TPM2 startup failed: malformed response");
        return None;
    } else {
        rt::log::log(1, "TPM2 startup failed");
        return None;
    }

    let mut tpm_random = [0u8; 32];
    let random_len = device.get_random_into(&mut tpm_random);
    let entropy = if random_len != 0 {
        rt::log::log(1, "TPM2 random ok");
        edgerun_crypto::rng::mix_entropy(&tpm_random[..random_len]);
        Some(tpm_random)
    } else {
        rt::log::log(1, "TPM2 random failed");
        None
    };

    let Some(entropy) = entropy else {
        return None;
    };

    let read_public_code = device.read_public_response_code(edgerun_tpm::TpmHandle(0x8100_0001));
    if read_public_code == edgerun_tpm::TPM_RC_SUCCESS {
        rt::log::log(1, "TPM2 persistent key readable");
    } else {
        rt::log::log(1, "TPM2 persistent key not readable");
        return Some(entropy);
    }

    let digest = [0u8; 32];
    let mut signature = [0u8; 64];
    if device
        .sign_p256_sha256_into(edgerun_tpm::TpmHandle(0x8100_0001), &digest, &mut signature)
        .is_ok()
    {
        rt::log::log(1, "TPM2 sign ok");
    } else {
        rt::log::log(1, "TPM2 sign failed");
    }

    Some(entropy)
}

#[cfg(target_arch = "x86_64")]
impl Future for NetPump<'_, '_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if !this.logged_start {
            rt::log::log(1, "Net pump started");
            this.logged_start = true;
        }

        let stats = poll_network(this.net, &mut this.network, &mut this.rx_buf);
        if stats.arp_replies != 0 && !this.logged_arp {
            rt::log::log(1, "ARP reply sent");
            this.logged_arp = true;
        }
        if stats.icmp_replies != 0 && !this.logged_icmp {
            rt::log::log(1, "ICMP echo reply sent");
            this.logged_icmp = true;
        }

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
fn find_initialized_bare_nic() -> Option<BareNic> {
    if let Some(rtl8125) = edgerun_rtl8125::find_rtl8125() {
        rt::log::log(1, "RTL8125 found");
        let mut net = BareNic::Rtl8125(rtl8125);
        if net.init() {
            rt::log::log(1, "RTL8125 init ok");
            return Some(net);
        }
        rt::log::log(1, "RTL8125 init failed");
    } else {
        rt::log::log(1, "No RTL8125 found");
    }

    if let Some(virtio) = edgerun_virtio::find_virtio_net() {
        rt::log::log(1, "VirtIO net found");
        let mut net = BareNic::Virtio(virtio);
        if net.init() {
            rt::log::log(1, "VirtIO net init ok");
            return Some(net);
        }
        rt::log::log(1, "VirtIO net init failed");
    } else {
        rt::log::log(1, "No VirtIO net found");
    }

    None
}

#[cfg(target_os = "none")]
#[panic_handler]
unsafe fn panic(_info: &core::panic::PanicInfo) -> ! {
    #[cfg(target_arch = "xtensa")]
    {
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"PANIC\n");
    }
    loop {
        #[cfg(target_arch = "x86_64")]
        core::arch::asm!("hlt");
        #[cfg(target_arch = "xtensa")]
        core::arch::asm!("waiti 0");
        #[cfg(not(any(target_arch = "x86_64", target_arch = "xtensa")))]
        core::hint::spin_loop();
    }
}

#[used]
#[cfg(all(target_arch = "x86_64", target_os = "none"))]
#[link_section = ".multiboot"]
static MULTIBOOT_HEADER: [u32; 8] = [
    0x1BADB002, 0x00010000, 0xE4514FFE, 0x100000, 0x100000, 0, 0, 0,
];

#[no_mangle]
#[cfg(all(target_arch = "xtensa", target_os = "none"))]
pub unsafe extern "C" fn kernel_main() -> ! {
    unsafe {
        edgerun_platform::arch::xtensa::esp32s3_disable_watchdogs();
        #[cfg(all(
            feature = "esp32s3-wifi-mmio",
            feature = "esp32s3-headless",
            not(feature = "esp32s3-wifi-blob")
        ))]
        edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::quiesce_after_soft_reset();
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_init();
    }
    edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KM0\n");
    #[cfg(all(feature = "esp32s3-ble-blob", not(feature = "esp32s3-headless")))]
    {
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KMB0\n");
        let _ = try_start_esp32s3_bt();
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KMB1\n");
    }
    edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KM1\n");
    rt::timer::set_now(0);
    edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KM1A\n");
    rt::log::set_mirror_logger(display_console_log);
    edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KM1B\n");
    #[cfg(not(feature = "esp32s3-headless"))]
    {
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KM1C\n");
    }
    #[cfg(feature = "esp32s3-headless")]
    edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KMH\n");
    #[cfg(not(feature = "esp32s3-headless"))]
    {
        unsafe {
            edgerun_platform::esp32s3::Jc3248w535Display::init();
        }
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KM2\n");
        rt::log::log(1, "Starting edgerun unikernel on Xtensa");
        rt::log::log(1, "Xtensa serial mux online");
        rt::log::log(1, "JC3248W535 display init complete");
        rt::log::log(1, "Rendering display UI");
        render_initial_ui();
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KM3\n");
        rt::log::log(1, "Display UI rendered");
        unsafe {
            edgerun_platform::esp32s3::Jc3248w535Touch::init();
        }
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KM4\n");
    }
    #[cfg(feature = "esp32s3-headless")]
    {
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KMW\n");
        headless_raw_write(b"ready headless\n");
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KMX\n");
    }
    #[cfg(not(feature = "esp32s3-headless"))]
    rt::log::log(1, "ESP32-S3 WiFi AP command ready");
    #[cfg(not(feature = "esp32s3-headless"))]
    rt::log::log(1, "JC3248W535 touch polling enabled");

    let mut last_touch: Option<(u16, u16)> = None;
    #[cfg(not(feature = "esp32s3-headless"))]
    let mut touch_overlay: Option<TouchOverlay> = None;
    #[cfg(not(feature = "esp32s3-headless"))]
    let mut touch_tracker = TouchTracker::new();
    let mut serial_rx = rt::serial_mux::Receiver::<256>::new();
    #[cfg(feature = "esp32s3-headless")]
    let mut headless_raw_rx = [0u8; 64];
    #[cfg(feature = "esp32s3-headless")]
    let mut headless_raw_len = 0usize;
    loop {
        #[cfg(not(feature = "esp32s3-headless"))]
        poll_serial_control(&mut serial_rx, last_touch);
        #[cfg(feature = "esp32s3-headless")]
        poll_headless_raw_control(&mut headless_raw_rx, &mut headless_raw_len);
        poll_esp32s3_wifi_ap();
        #[cfg(not(feature = "esp32s3-headless"))]
        {
            let touch_sample = unsafe {
                edgerun_platform::esp32s3::Jc3248w535Touch::read_point()
                    .map(|point| (point.x, point.y))
            };
            let now = edgerun_platform::timer::timer_ticks();
            let touch = touch_tracker.update(touch_sample, now);
            render_touch_overlay_step(&mut touch_overlay, touch, now);
            if touch.is_some() {
                last_touch = touch;
            }
            render_console_if_dirty(last_touch);
        }
    }
}

#[no_mangle]
#[cfg(all(target_arch = "x86_64", target_os = "none"))]
pub unsafe extern "C" fn kernel_main() -> ! {
    rt::timer::set_now(0);
    rt::log::log(1, "Starting edgerun unikernel");
    let mut active_boot_config = None;

    edgerun_crypto::rng::register_random_source(fill_bare_random_source);

    let mut rng = Rng::new_from_entropy();
    if let Some(mut virtio_rng) = edgerun_virtio::find_virtio_rng() {
        if virtio_rng.init() {
            let mut virtio_entropy = [0u8; 32];
            if virtio_rng.fill_bytes(&mut virtio_entropy) {
                rng.mix_entropy(&virtio_entropy);
                edgerun_crypto::rng::mix_entropy(&virtio_entropy);
                rt::log::log(1, "RNG mixed VirtIO entropy");
            } else {
                rt::log::log(1, "VirtIO RNG read failed");
            }
        } else {
            rt::log::log(1, "VirtIO RNG init failed");
        }
    } else {
        rt::log::log(1, "No VirtIO RNG found");
    }

    if let Some(tpm_entropy) = unsafe { probe_tpm2() } {
        rng.mix_entropy(&tpm_entropy);
        rt::log::log(1, "RNG mixed TPM entropy");
    }

    let test_crc = crc32(b"hello");
    let _ = test_crc;

    let mut rx_buf = RingBuffer::new(1024);
    rx_buf.push_slice(b"test packet");
    let _ = rx_buf.len();

    rt::log::log(1, "Looking for VirtIO...");

    if let Some(mut console) = edgerun_virtio::find_virtio_console() {
        if console.init() {
            let _ = console.write_all(b"edgerun: virtio-console online\n");
            rt::log::log(1, "VirtIO console init ok");
        } else {
            rt::log::log(1, "VirtIO console init failed");
        }
    } else {
        rt::log::log(1, "No VirtIO console found");
    }

    if let Some(mut block) = edgerun_virtio::find_virtio_blk() {
        rt::log::log(1, "VirtIO block device found");
        if block.init() {
            rt::log::log(1, "VirtIO block init ok");
            let mut first_sector = [0u8; 512];
            if block.read_sector(0, &mut first_sector) {
                rt::log::log(1, "VirtIO block first sector read ok");
            } else {
                rt::log::log(1, "VirtIO block first sector read failed");
            }
            if block.read_sector(0, &mut first_sector) {
                rt::log::log(1, "VirtIO block second sector read ok");
            } else {
                rt::log::log(1, "VirtIO block second sector read failed");
            }
            if first_sector[510] == 0x55 && first_sector[511] == 0xaa {
                let mut partition_sector = [0u8; 512];
                if block.read_sector(2048, &mut partition_sector) {
                    rt::log::log(1, "VirtIO block partition sector read ok");
                } else {
                    rt::log::log(1, "VirtIO block partition sector read failed");
                }
                if block.read_sector(2112, &mut partition_sector) {
                    rt::log::log(1, "VirtIO block probe sector read ok");
                } else {
                    rt::log::log(1, "VirtIO block probe sector read failed");
                }
                if block.read_sector(2308, &mut partition_sector) {
                    rt::log::log(1, "VirtIO block FAT root sector read ok");
                } else {
                    rt::log::log(1, "VirtIO block FAT root sector read failed");
                }
            }
            rt::log::log(1, "VirtIO block scanning partitions");
            let mut storage = disk_boot::RtBlockDeviceStorage::new(block);
            match edgerun_storage::BlockStorage::read_sector(&mut storage, 0, &mut first_sector) {
                Ok(()) => rt::log::log(1, "VirtIO storage adapter read ok"),
                Err(_) => rt::log::log(1, "VirtIO storage adapter read failed"),
            }
            if first_sector[510] != 0x55 || first_sector[511] != 0xaa {
                rt::log::log(1, "VirtIO block has no partitions");
            } else {
                match edgerun_storage::detect_partitions(&mut storage) {
                    Ok(table) if !table.partitions.is_empty() => {
                        rt::log::log(1, "VirtIO block partition table detected");
                        let mut handled_boot_config = false;
                        for partition in &table.partitions {
                            if handled_boot_config {
                                break;
                            }
                            let mut partition_boot_sector = [0u8; 512];
                            match edgerun_storage::BlockStorage::read_sector(
                                &mut storage,
                                partition.start_lba,
                                &mut partition_boot_sector,
                            ) {
                                Ok(()) if partition_boot_sector.iter().all(|byte| *byte == 0) => {
                                    rt::log::log(1, "VirtIO partition is blank")
                                }
                                Ok(()) => {
                                    rt::log::log(1, "VirtIO block partitions detected");
                                    let mut partition_storage = disk_boot::RtPartitionStorage::new(
                                        &mut storage,
                                        partition.start_lba,
                                        partition.sectors,
                                    );
                                    let mut fat_root_sector = [0u8; 512];
                                    match edgerun_storage::BlockStorage::read_sector(
                                        &mut partition_storage,
                                        260,
                                        &mut fat_root_sector,
                                    ) {
                                        Ok(()) => {
                                            rt::log::log(1, "VirtIO FAT relative root read ok")
                                        }
                                        Err(_) => {
                                            rt::log::log(1, "VirtIO FAT relative root read failed")
                                        }
                                    }
                                    rt::log::log(1, "VirtIO opening FAT boot partition");
                                    let mut boot_config = None;
                                    match edgerun_storage::FatReadOnly::open(partition_storage) {
                                        Ok(mut fat) => {
                                            rt::log::log(1, "VirtIO FAT boot partition open ok");
                                            match fat.root_entries() {
                                                Ok(_) => rt::log::log(
                                                    1,
                                                    "VirtIO FAT root directory read ok",
                                                ),
                                                Err(_) => rt::log::log(
                                                    1,
                                                    "VirtIO FAT root directory read failed",
                                                ),
                                            }
                                            rt::log::log(1, "VirtIO FAT boot config read start");
                                            match fat.read_file("/edgerun/boot.cfg") {
                                                Ok(bytes) => {
                                                    rt::log::log(
                                                        1,
                                                        "VirtIO FAT boot config bytes read ok",
                                                    );
                                                    match boot_config::BootConfig::parse(&bytes) {
                                                        Ok(config) => {
                                                            rt::log::log(
                                                                1,
                                                                "VirtIO FAT boot config read ok",
                                                            );
                                                            active_boot_config =
                                                                Some(config.clone());
                                                            boot_config = Some(config);
                                                        }
                                                        Err(_) => rt::log::log(
                                                            1,
                                                            "VirtIO FAT boot config parse failed",
                                                        ),
                                                    }
                                                }
                                                Err(_) => rt::log::log(
                                                    1,
                                                    "VirtIO FAT boot config missing",
                                                ),
                                            }
                                        }
                                        Err(_) => rt::log::log(1, "VirtIO partition is not FAT"),
                                    }
                                    if let Some(config) = boot_config.as_ref() {
                                        let edgefs_key = [0x42u8; 32];
                                        let edgefs_fs_id = [0x24u8; 16];
                                        match disk_boot::open_configured_edgefs(
                                            &mut storage,
                                            config,
                                            edgefs_key,
                                            edgefs_fs_id,
                                        ) {
                                            Ok(_) => {
                                                rt::log::log(1, "VirtIO EdgeFS boot target open ok")
                                            }
                                            Err(_) => rt::log::log(
                                                1,
                                                "VirtIO EdgeFS boot target open failed",
                                            ),
                                        }
                                        handled_boot_config = true;
                                    }
                                }
                                Err(_) => rt::log::log(1, "VirtIO partition read failed"),
                            }
                        }
                    }
                    Ok(_) => {
                        rt::log::log(1, "VirtIO block has no partitions");
                    }
                    Err(_) => {
                        rt::log::log(1, "VirtIO block partition scan failed");
                    }
                }
            }
        } else {
            rt::log::log(1, "VirtIO block init failed");
        }
    } else {
        rt::log::log(1, "No VirtIO block device found");
    }

    let mut net = match find_initialized_bare_nic() {
        Some(net) => net,
        None => {
            rt::log::log(1, "No usable NIC found");
            loop {
                core::arch::asm!("hlt");
            }
        }
    };
    let mac = net.get_mac();

    let mut stack = IpStack::new();
    stack.configure(
        IpAddr::new(0, 0, 0, 0),
        IpAddr::new(255, 255, 255, 0),
        IpAddr::zero(),
        mac,
    );

    let dhcp_xid = 0x12345678;
    let mut dhcp_ip = IpAddr::zero();
    let mut dhcp_netmask = IpAddr::new(255, 255, 255, 0);
    let mut dhcp_gateway = IpAddr::zero();
    let mut offered_ip = None;
    let mut offered_netmask = None;
    let mut offered_gateway = None;
    let mut requested_lease = false;
    let mut network = Network::new(&mut stack);

    let discover = DhcpMessage::discover(dhcp_xid, mac).to_wire();
    rt::log::log(1, "Sending DHCP discover");
    if let Some(pkt) = network.send_udp(
        IpAddr::new(255, 255, 255, 255),
        DHCP_CLIENT_PORT,
        DHCP_SERVER_PORT,
        &discover,
    ) {
        if net.send(pkt) {
            rt::log::log(1, "DHCP discover queued");
        } else {
            rt::log::log(1, "DHCP discover send failed");
        }
    }

    let mut rx_buf = [0u8; 1514];
    let mut logged_rx = false;
    for _ in 0..1000 {
        if let Some(len) = net.recv(&mut rx_buf) {
            if !logged_rx {
                rt::log::log(1, "NIC RX packet observed");
                logged_rx = true;
            }
            if let Some(ParsedPacket::Udp {
                header, payload, ..
            }) = network.recv(&rx_buf[..len])
            {
                if header.src_port == DHCP_SERVER_PORT && header.dst_port == DHCP_CLIENT_PORT {
                    if let Ok(message) = DhcpMessage::from_wire(payload) {
                        if message.xid != dhcp_xid || message.chaddr[..6] != mac {
                            continue;
                        }

                        match message.options.message_type {
                            Some(DhcpMessageType::Offer) if !requested_lease => {
                                let Some(server_id) = message.options.server_id else {
                                    continue;
                                };
                                offered_ip = Some(message.yiaddr);
                                offered_netmask = message.options.subnet_mask;
                                offered_gateway = message.options.router;

                                let request =
                                    DhcpMessage::request(dhcp_xid, mac, message.yiaddr, server_id)
                                        .to_wire();
                                if let Some(pkt) = network.send_udp(
                                    IpAddr::new(255, 255, 255, 255),
                                    DHCP_CLIENT_PORT,
                                    DHCP_SERVER_PORT,
                                    &request,
                                ) {
                                    if net.send(pkt) {
                                        requested_lease = true;
                                        rt::log::log(1, "DHCP request queued");
                                    } else {
                                        rt::log::log(1, "DHCP request send failed");
                                    }
                                }
                            }
                            Some(DhcpMessageType::Ack) if requested_lease => {
                                dhcp_ip = dhcp_ipv4_to_rt(message.yiaddr);
                                if dhcp_ip == IpAddr::zero() {
                                    if let Some(ip) = offered_ip {
                                        dhcp_ip = dhcp_ipv4_to_rt(ip);
                                    }
                                }
                                if let Some(netmask) =
                                    message.options.subnet_mask.or(offered_netmask)
                                {
                                    dhcp_netmask = dhcp_ipv4_to_rt(netmask);
                                }
                                if let Some(gateway) = message.options.router.or(offered_gateway) {
                                    dhcp_gateway = dhcp_ipv4_to_rt(gateway);
                                }
                                rt::log::log(1, "DHCP lease accepted");
                                break;
                            }
                            Some(DhcpMessageType::Nak) => {
                                rt::log::log(1, "DHCP lease rejected");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let net_stats = net.stats();
    if net_stats.tx_completed != 0 {
        rt::log::log(1, "NIC TX completed");
    } else {
        rt::log::log(1, "NIC TX pending");
    }
    if net_stats.rx_received == 0 {
        rt::log::log(1, "NIC RX no packets");
    }

    drop(network);

    if dhcp_ip != IpAddr::zero() {
        stack.ip = dhcp_ip;
        stack.netmask = dhcp_netmask;
        stack.gateway = dhcp_gateway;
    } else {
        rt::log::log(1, "Using static fallback IP");
        stack.ip = IpAddr::new(192, 168, 1, 12);
    }

    BARE_NET_DRIVER.install(&mut net as *mut _, &mut stack as *mut _);
    rt::install_bare_net_driver(&BARE_NET_DRIVER);
    rt::log::log(1, "Bare async TCP driver installed");
    if let Some(url) = active_boot_config
        .as_ref()
        .and_then(|config| config.http_smoke.as_deref())
    {
        block_on(run_http_smoke(url));
    }
    if let Some(config) = active_boot_config.as_ref() {
        block_on(run_configured_oci_pull(config));
    }

    let mut network = Network::new(&mut stack);
    let tftp_server = if network.stack.gateway != IpAddr::zero() {
        network.stack.gateway
    } else {
        IpAddr::new(192, 168, 1, 1)
    };
    let rrq = TftpMessage::rrq("edgerun.bin").to_wire();
    rt::log::log(1, "Sending TFTP RRQ");
    if let Some(pkt) = network.send_udp(tftp_server, 2070, TFTP_PORT, &rrq) {
        if net.send(pkt) {
            rt::log::log(1, "TFTP RRQ queued");
        } else {
            rt::log::log(1, "TFTP RRQ send failed");
        }
    }
    let mut tcp = TcpSocket::new();

    let addr = rt::SocketAddr::new(0xC0A8010C, 8080);
    let _ = tcp.bind(addr);
    let _ = tcp.listen(10);
    let _ = rng.next();

    if net.is_link_up() {}

    block_on(NetPump {
        net: &mut net,
        network,
        rx_buf: [0; 1514],
        logged_start: false,
        logged_arp: false,
        logged_icmp: false,
    });

    loop {
        core::arch::asm!("hlt");
    }
}

#[cfg(not(target_os = "none"))]
fn main() {}
