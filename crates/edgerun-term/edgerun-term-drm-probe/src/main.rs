use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsFd, BorrowedFd};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use drm::Device;
use drm::buffer::DrmFourcc;
use drm::control::{Device as ControlDevice, connector, crtc};
use portable_pty::{CommandBuilder, PtySize};

struct Card(File);

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Device for Card {}
impl ControlDevice for Card {}

impl Card {
    fn open(path: &str) -> Result<Self> {
        Ok(Self(
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .with_context(|| format!("open {path}"))?,
        ))
    }
}

#[derive(Default)]
struct ParserSink {
    printable: usize,
    controls: usize,
}

impl vte::Perform for ParserSink {
    fn print(&mut self, _c: char) {
        self.printable += 1;
    }

    fn execute(&mut self, _byte: u8) {
        self.controls += 1;
    }
}

fn main() -> Result<()> {
    let card = Card::open(
        std::env::var("TERM_DRM_CARD")
            .as_deref()
            .unwrap_or("/dev/dri/card0"),
    )?;
    let resources = card.resource_handles().context("drm resources")?;
    let connectors: Vec<connector::Info> = resources
        .connectors()
        .iter()
        .filter_map(|connector| card.get_connector(*connector, true).ok())
        .collect();
    let crtcs: Vec<crtc::Info> = resources
        .crtcs()
        .iter()
        .filter_map(|crtc| card.get_crtc(*crtc).ok())
        .collect();
    let connector = connectors
        .iter()
        .find(|info| info.state() == connector::State::Connected)
        .context("no connected connector")?;
    let mode = *connector
        .modes()
        .first()
        .context("connected connector has no mode")?;
    let crtc = crtcs.first().context("no crtc")?;
    let (width, height) = mode.size();

    let mut buffer = card
        .create_dumb_buffer(
            (u32::from(width), u32::from(height)),
            DrmFourcc::Xrgb8888,
            32,
        )
        .context("create dumb buffer")?;
    {
        let mut map = card
            .map_dumb_buffer(&mut buffer)
            .context("map dumb buffer")?;
        paint_probe_frame(map.as_mut(), u32::from(width), u32::from(height));
    }
    let framebuffer = card
        .add_framebuffer(&buffer, 24, 32)
        .context("add framebuffer")?;

    let _pty_worker = thread::spawn(|| {
        let _ = pty_probe();
    });

    card.set_crtc(
        crtc.handle(),
        Some(framebuffer),
        (0, 0),
        &[connector.handle()],
        Some(mode),
    )
    .context("set crtc")?;
    thread::sleep(Duration::from_millis(
        std::env::var("TERM_DRM_PROBE_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(250),
    ));
    card.destroy_framebuffer(framebuffer).ok();
    card.destroy_dumb_buffer(buffer).ok();
    Ok(())
}

fn paint_probe_frame(frame: &mut [u8], width: u32, height: u32) {
    let width = width as usize;
    let height = height as usize;
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) * 4;
            frame[i] = (x & 0xff) as u8;
            frame[i + 1] = (y & 0xff) as u8;
            frame[i + 2] = 0x20;
            frame[i + 3] = 0;
        }
    }
}

fn pty_probe() -> Result<()> {
    let system = portable_pty::native_pty_system();
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
    let mut parser = vte::Parser::new();
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
