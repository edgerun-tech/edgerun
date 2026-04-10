//! Demo: constructs a RenderObject tree by hand, rasterizes to DRM dumb buffer, page flips.
use std::io;
use std::os::unix::io::AsRawFd;
use std::ptr;

const DRM_IOCTL_MODE_CREATE_DUMB: u64 = 0xC01864B2;
const DRM_IOCTL_MODE_MAP_DUMB: u64 = 0xC00864B3;
const DRM_IOCTL_MODE_PAGE_FLIP: u64 = 0x401864B0;
const DRM_IOCTL_MODE_SETCRTC: u64 = 0xC04064B8;
const DRM_IOCTL_MODE_ADDFB2: u64 = 0xC09064B5;

#[repr(C)]
struct DrmModeCreateDumb {
    width: u32, height: u32, bpp: u32, flags: u32,
    handle: u32, pitch: u32, size: u64,
}
#[repr(C)]
struct DrmModeMapDumb { handle: u32, pad: u32, offset: u64 }
#[repr(C)]
struct DrmModePageFlip {
    crtc_id: u32, fb_id: u32, flags: u32,
    reserved: u32, user_data: u64,
}
#[repr(C)]
struct DrmModeModeInfo {
    clock: u32, hdisplay: u16, hsync_start: u16, hsync_end: u16,
    htotal: u16, hskew: u16, vdisplay: u16, vsync_start: u16,
    vsync_end: u16, vtotal: u16, vscan: u16,
    vrefresh: u32, flags: u32, type_: u32, name: [u8; 32],
}
#[repr(C)]
struct DrmModeCrtc {
    set_connectors: u64, count_connectors: u32,
    x: u32, y: u32, fb_id: u32,
    mode: DrmModeModeInfo, mode_valid: i32,
}

fn ioctl_raw(fd: i32, req: u64, data: *mut std::ffi::c_void) -> io::Result<()> {
    let ret = unsafe { libc::ioctl(fd, req as libc::c_ulong, data) };
    if ret < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

fn main() -> io::Result<()> {
    // 1. Open DRM device
    let drm_file = std::fs::OpenOptions::new()
        .read(true).write(true)
        .open("/dev/dri/card0")
        .or_else(|_| std::fs::OpenOptions::new().read(true).write(true).open("/dev/dri/card1"))?;
    let drm_fd = drm_file.as_raw_fd();
    println!("Opened DRM fd: {}", drm_fd);

    let width: u32 = 1920;
    let height: u32 = 1080;
    let crtc_id: u32 = 0;
    let connector_id: u32 = 0;
    println!("Display: {}x{}", width, height);

    // 2. Allocate dumb buffer
    let mut create = DrmModeCreateDumb {
        width, height, bpp: 32, flags: 0,
        handle: 0, pitch: 0, size: 0,
    };
    ioctl_raw(drm_fd, DRM_IOCTL_MODE_CREATE_DUMB, &mut create as *mut _ as _)?;

    // 3. Create framebuffer
    let mut fb_req: [u32; 18] = [0; 18];
    fb_req[0] = width;
    fb_req[1] = height;
    fb_req[2] = create.pitch;
    fb_req[3] = 0x34325258; // XRGB8888
    fb_req[4] = create.handle;
    fb_req[5] = create.pitch;
    fb_req[6] = 0x34325258;
    fb_req[7] = create.handle;
    fb_req[8] = 0;
    ioctl_raw(drm_fd, DRM_IOCTL_MODE_ADDFB2, &mut fb_req as *mut _ as _)?;
    let fb_id = fb_req[17];
    println!("Dumb buffer: pitch={}, fb_id={}", create.pitch, fb_id);

    // 4. Map dumb buffer
    let mut map_req = DrmModeMapDumb {
        handle: create.handle, pad: 0, offset: 0,
    };
    ioctl_raw(drm_fd, DRM_IOCTL_MODE_MAP_DUMB, &mut map_req as *mut _ as _)?;

    let map_len = (create.pitch * height) as usize;
    let pixels_ptr = unsafe {
        libc::mmap(ptr::null_mut(), map_len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED, drm_fd, map_req.offset as libc::off_t)
    };
    if pixels_ptr == libc::MAP_FAILED {
        return Err(io::Error::last_os_error());
    }
    let pixels: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(pixels_ptr as *mut u8, map_len) };
    println!("Mapped {} bytes", pixels.len());

    // 5. Set CRTC
    let mut crtc = DrmModeCrtc {
        set_connectors: connector_id as u64,
        count_connectors: 1,
        x: 0, y: 0, fb_id,
        mode: DrmModeModeInfo {
            clock: 148500, hdisplay: 1920, hsync_start: 2008, hsync_end: 2052,
            htotal: 2200, hskew: 0, vdisplay: 1080, vsync_start: 1084,
            vsync_end: 1089, vtotal: 1125, vscan: 0,
            vrefresh: 60, flags: 5, type_: 64, name: [0; 32],
        },
        mode_valid: 1,
    };
    ioctl_raw(drm_fd, DRM_IOCTL_MODE_SETCRTC, &mut crtc as *mut _ as _)?;
    println!("CRTC set, display active");

    // 6. Render demo scene
    println!("Rendering demo scene...");
    render_demo(pixels, width, height, create.pitch);

    // 7. Page flip
    let flip = DrmModePageFlip {
        crtc_id, fb_id, flags: 0, // synchronous flip
        reserved: 0, user_data: 0,
    };
    ioctl_raw(drm_fd, DRM_IOCTL_MODE_PAGE_FLIP, &flip as *const _ as _)?;
    println!("Page flipped! Press Enter to exit...");

    // Wait
    let _ = std::io::stdin().read_line(&mut String::new());

    // Cleanup
    unsafe { libc::munmap(pixels_ptr, map_len) };
    println!("Done.");
    Ok(())
}

// ============================================================
// RENDER DEMO
// ============================================================
use edgerun_rasterizer::scanline::{self, RasterCommand, cmd_fill, cmd_text};
use edgerun_rasterizer::gradient::GradientStop;
use edgerun_rasterizer::framebuffer::Framebuffer;

fn render_demo(pixels: &mut [u8], width: u32, height: u32, pitch: u32) {
    let mut fb = Framebuffer::new(pixels, width, height);
    fb.clear();

    let mut cmds: Vec<RasterCommand> = Vec::new();

    // Background gradient
    cmds.push(RasterCommand::LinearGradient {
        x: 0, y: 0, w: width, h: height,
        angle: std::f64::consts::PI / 2.0,
        stops: vec![
            GradientStop { r: 0x1a, g: 0x1a, b: 0x2e, a: 255, position: 0.0 },
            GradientStop { r: 0x2d, g: 0x1b, b: 0x4e, a: 255, position: 1.0 },
        ],
    });

    // Title bar
    cmds.push(cmd_fill(0, 0, width, 60, 0xFF, 0x4D, 0x00));
    cmds.push(cmd_text(20, 16, "EDGERUN DEMO", 0xFF, 0xFF, 0xFF));
    cmds.push(cmd_text(20, 80, "Software renderer generated from proto data", 0xAA, 0xAA, 0xCC));

    // Color swatches from the LUT
    let colors = [(1, "White"), (2, "Red"), (3, "Green"), (4, "Blue"),
                  (5, "Yellow"), (6, "Cyan"), (7, "Magenta"), (8, "Orange")];
    for (i, (idx, name)) in colors.iter().enumerate() {
        let x = 20 + i as u32 * 120;
        let y = 120;
        let (r, g, b, _) = edgerun_rasterizer::color_lut::named_color(*idx);
        cmds.push(cmd_fill(x, y, 100, 100, r, g, b));
        cmds.push(cmd_text(x + 10, y + 110, name, 0xCC, 0xCC, 0xCC));
    }

    // Border style demo
    let styles = [(1, "solid"), (2, "dashed"), (3, "dotted"), (4, "double"),
                  (5, "groove"), (6, "ridge"), (7, "inset"), (8, "outset")];
    for (i, (style, label)) in styles.iter().enumerate() {
        let x = 20 + i as u32 * 120;
        let y = 280;
        cmds.push(RasterCommand::StrokeRect {
            x, y, w: 100, h: 80, r: 0xFF, g: 0xAA, b: 0x55,
            style: *style, thickness: 3,
        });
        cmds.push(cmd_text(x + 10, y + 90, label, 0xAA, 0x88, 0x55));
    }

    // Gradient demos
    cmds.push(RasterCommand::LinearGradient {
        x: 20, y: 420, w: 200, h: 100, angle: 0.0,
        stops: vec![
            GradientStop { r: 0xFF, g: 0x00, b: 0x00, a: 255, position: 0.0 },
            GradientStop { r: 0x00, g: 0xFF, b: 0x00, a: 255, position: 0.5 },
            GradientStop { r: 0x00, g: 0x00, b: 0xFF, a: 255, position: 1.0 },
        ],
    });
    cmds.push(cmd_text(20, 530, "linear gradient", 0xAA, 0xAA, 0xAA));

    cmds.push(RasterCommand::RadialGradient {
        x: 260, y: 420, w: 200, h: 100, cx: 0.5, cy: 0.5,
        stops: vec![
            GradientStop { r: 0xFF, g: 0xFF, b: 0xFF, a: 255, position: 0.0 },
            GradientStop { r: 0x44, g: 0x44, b: 0xFF, a: 255, position: 1.0 },
        ],
    });
    cmds.push(cmd_text(260, 530, "radial gradient", 0xAA, 0xAA, 0xAA));

    cmds.push(RasterCommand::ConicGradient {
        x: 500, y: 420, w: 200, h: 100, from_angle: 0.0, cx: 0.5, cy: 0.5,
        stops: vec![
            GradientStop { r: 0xFF, g: 0x00, b: 0x80, a: 255, position: 0.0 },
            GradientStop { r: 0x00, g: 0xFF, b: 0x80, a: 255, position: 0.33 },
            GradientStop { r: 0x80, g: 0x00, b: 0xFF, a: 255, position: 0.66 },
            GradientStop { r: 0xFF, g: 0x00, b: 0x80, a: 255, position: 1.0 },
        ],
    });
    cmds.push(cmd_text(500, 530, "conic gradient", 0xAA, 0xAA, 0xAA));

    // Text sample
    cmds.push(cmd_text(20, 580, "The quick brown fox jumps over the lazy dog", 0xDD, 0xDD, 0xDD));
    cmds.push(cmd_text(20, 596, "ABCDEFGHIJKLMNOPQRSTUVWXYZ", 0xFF, 0xCC, 0x88));
    cmds.push(cmd_text(20, 612, "abcdefghijklmnopqrstuvwxyz", 0x88, 0xCC, 0xFF));
    cmds.push(cmd_text(20, 628, "0123456789 !@#$%", 0xAA, 0xFF, 0xAA));

    // Footer
    cmds.push(cmd_fill(0, height - 40, width, 40, 0x0D, 0x0D, 0x1A));
    cmds.push(cmd_text(20, height - 30, "proto -> buf generate -> Rust -> pixels", 0x66, 0x66, 0x88));

    scanline::rasterize(&mut fb, &cmds);
    println!("Rendered {} commands", cmds.len());
}
