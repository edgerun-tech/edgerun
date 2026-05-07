use std::io::{self, Read, Write};
use std::thread;
use std::time::Duration;

use edgerun_compositor::drm::device::{DrmConnector, DrmDevice};
use edgerun_compositor::drm::dumb::DumbBuffer;
use edgerun_compositor::drm::kms;
use edgerun_pty::{CommandBuilder, PtySize};

type ProbeResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Default)]
struct ParserSink {
    printable: usize,
    controls: usize,
}

impl edgerun_terminal_parser::Perform for ParserSink {
    fn print(&mut self, _c: char) {
        self.printable += 1;
    }

    fn execute(&mut self, _byte: u8) {
        self.controls += 1;
    }
}

fn main() -> ProbeResult<()> {
    let card_path = std::env::var("TERM_DRM_CARD").unwrap_or_else(|_| "/dev/dri/card0".into());
    let card = DrmDevice::open(&card_path)
        .map_err(|err| io::Error::other(format!("open {card_path}: {err}")))?;
    let resources = card
        .get_resources()
        .map_err(|err| io::Error::other(format!("drm resources: {err}")))?;
    let connectors: Vec<DrmConnector> = resources
        .connectors
        .iter()
        .filter_map(|connector| card.get_connector(*connector).ok())
        .collect();
    let connector = connectors
        .iter()
        .find(|info| info.is_connected())
        .ok_or_else(|| io::Error::other("no connected connector"))?;
    let mode = connector
        .modes
        .first()
        .ok_or_else(|| io::Error::other("connected connector has no mode"))?;
    let crtc = resources
        .crtcs
        .first()
        .copied()
        .ok_or_else(|| io::Error::other("no crtc"))?;
    let width = u32::from(mode.hdisplay);
    let height = u32::from(mode.vdisplay);
    let fd = card.as_raw_fd();

    let mut buffer = DumbBuffer::create(fd, width, height, 32)
        .map_err(|err| io::Error::other(format!("create dumb buffer: {err}")))?;
    {
        let pitch = buffer.pitch;
        let map = buffer
            .map()
            .map_err(|err| io::Error::other(format!("map dumb buffer: {err}")))?;
        paint_probe_frame(map, width, height, pitch);
    }
    let framebuffer = buffer
        .add_fb()
        .map_err(|err| io::Error::other(format!("add framebuffer: {err}")))?;

    let _pty_worker = thread::spawn(|| {
        let _ = pty_probe();
    });

    kms::set_crtc(fd, crtc, framebuffer, connector.connector_id, mode)
        .map_err(|err| io::Error::other(format!("set crtc: {err}")))?;
    thread::sleep(Duration::from_millis(
        std::env::var("TERM_DRM_PROBE_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(250),
    ));
    kms::rmfb(fd, framebuffer).ok();
    buffer.destroy().ok();
    Ok(())
}

fn paint_probe_frame(frame: &mut [u8], width: u32, height: u32, pitch: u32) {
    let width = width as usize;
    let height = height as usize;
    let pitch = pitch as usize;
    for y in 0..height {
        for x in 0..width {
            let i = y * pitch + x * 4;
            frame[i] = (x & 0xff) as u8;
            frame[i + 1] = (y & 0xff) as u8;
            frame[i + 2] = 0x20;
            frame[i + 3] = 0;
        }
    }
}

fn pty_probe() -> ProbeResult<()> {
    let system = edgerun_pty::native_pty_system();
    let pair = system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut child = pair.slave.spawn_command(CommandBuilder::new(shell))?;
    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;
    writer.write_all(b"printf drm-probe\\nexit\\n")?;
    let mut parser = edgerun_terminal_parser::Parser::new();
    let mut sink = ParserSink::default();
    let mut buf = [0; 4096];
    while let Ok(n) = reader.read(&mut buf) {
        if n == 0 {
            break;
        }
        for byte in &buf[..n] {
            parser.advance(&mut sink, *byte);
        }
    }
    let _ = child.wait();
    Ok(())
}
