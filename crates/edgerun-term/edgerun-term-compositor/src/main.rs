use std::io::{self, Read, Write};
use std::ops::{Deref, DerefMut};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use edgerun_compositor::input::keymap::{Keymap, Keysym, Modifiers, SpecialKey, process_key_event};
use edgerun_compositor::protocol::{wl_compositor, wl_core, wl_seat, wl_shm, xdg_shell};
use edgerun_compositor::render::shm::SharedMemFrame;
use edgerun_compositor::wire;
use edgerun_compositor::wire::decode::{ArgCursor, DecodeError, parse_message};
use edgerun_compositor::wire::encode::{encode, encode_string, message_empty, message_uint};
use edgerun_compositor::wire::fd::{recv_with_fds, send_with_fds};
use edgerun_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use edgerun_term_core::render::layout::{LayoutMetrics, compute_layout};
use edgerun_term_core::render::{
    FONT_DATA, FONT_SIZE, GlyphCache, TabVisual, draw_background, draw_border_cpu,
    draw_cursor_overlay, draw_grid, draw_help_bar_cpu, draw_tab_bar_cpu, draw_text_line_clipped,
    fill_rect,
};
use edgerun_term_core::terminal::{GridPerformer, Terminal, write_bytes};
use edgerun_terminal_parser::Parser as VteParser;

const DEFAULT_SOCKET: &str = "/tmp/edgerun-wayland-0";
const WIDTH: u32 = 960;
const HEIGHT: u32 = 540;
const BORDER_THICKNESS: u32 = 2;
const BORDER_RADIUS: u32 = 8;
const BORDER_INSET: u32 = 0;
const PADDING_X: u32 = 8;
const PADDING_Y: u32 = 6;

const REGISTRY_ID: u32 = 2;
const SYNC_ID: u32 = 3;
const COMPOSITOR_ID: u32 = 10;
const SHM_ID: u32 = 11;
const WM_BASE_ID: u32 = 12;
const SEAT_ID: u32 = 13;
const SURFACE_ID: u32 = 20;
const XDG_SURFACE_ID: u32 = 21;
const TOPLEVEL_ID: u32 = 22;
const SHM_POOL_ID: u32 = 30;
const BUFFER_ID: u32 = 31;
const KEYBOARD_ID: u32 = 40;

type AppResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn app_error(message: impl Into<String>) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(io::Error::other(message.into()))
}

#[derive(Debug)]
enum AppEvent {
    Pty(Vec<u8>),
}

struct Tab {
    terminal: Terminal,
    parser: VteParser,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    _master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    _child: Box<dyn edgerun_pty::Child + Send + Sync>,
    app_cursor_keys: bool,
    title: String,
}

#[derive(Default)]
struct ChatInput {
    active: bool,
    buffer: String,
    status: String,
}

impl ChatInput {
    fn toggle(&mut self) {
        self.active = !self.active;
        if self.active && self.status.is_empty() {
            self.status = "Enter prompt, Esc closes".to_string();
        }
    }

    fn submit_command(&mut self) -> Option<String> {
        let prompt = self.buffer.trim();
        if prompt.is_empty() {
            self.status = "Empty prompt".to_string();
            return None;
        }
        let command = format!("zen-client {}\r", shell_quote(prompt));
        self.status = format!("Submitted: {prompt}");
        self.buffer.clear();
        self.active = false;
        Some(command)
    }

    fn handle_key(&mut self, keysym: Keysym, mods: Modifiers) -> Option<String> {
        match keysym {
            Keysym::Char(ch) => {
                if mods.ctrl {
                    match ch.to_ascii_lowercase() {
                        'u' => self.buffer.clear(),
                        'w' => trim_last_word(&mut self.buffer),
                        _ => {}
                    }
                } else {
                    self.buffer.push(ch);
                }
                None
            }
            Keysym::Special(SpecialKey::Space) => {
                if !mods.ctrl {
                    self.buffer.push(' ');
                }
                None
            }
            Keysym::Special(SpecialKey::Tab) => {
                self.buffer.push('\t');
                None
            }
            Keysym::Special(SpecialKey::Backspace) => {
                self.buffer.pop();
                None
            }
            Keysym::Special(SpecialKey::Escape) => {
                self.active = false;
                None
            }
            Keysym::Special(SpecialKey::Return) => self.submit_command(),
            _ => None,
        }
    }
}

struct ShmFrame(SharedMemFrame);

impl ShmFrame {
    fn new(width: u32, height: u32) -> AppResult<Self> {
        SharedMemFrame::new("edgerun-term-frame", width, height)
            .map(Self)
            .map_err(|err| app_error(format!("create shm frame: {err}")))
    }

    fn fd(&self) -> RawFd {
        self.0.fd()
    }

    fn as_mut_bytes(&mut self) -> &mut [u8] {
        self.0.as_mut_bytes()
    }
}

impl Deref for ShmFrame {
    type Target = SharedMemFrame;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ShmFrame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

struct WaylandClient {
    stream: UnixStream,
    recv_buf: Vec<u8>,
    pending_fds: Vec<i32>,
}

impl WaylandClient {
    fn connect(path: &str) -> AppResult<Self> {
        let stream = UnixStream::connect(path)
            .map_err(|err| app_error(format!("connect edgerun compositor socket {path}: {err}")))?;
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            recv_buf: Vec::new(),
            pending_fds: Vec::new(),
        })
    }

    fn fd(&self) -> RawFd {
        self.stream.as_raw_fd()
    }

    fn send(&mut self, msg: wire::Message) -> AppResult<()> {
        let data = encode(&msg);
        let fds: Vec<RawFd> = msg.fds.iter().copied().collect();
        let sent = send_with_fds(self.fd(), &data, &fds)?;
        if sent != data.len() {
            return Err(app_error("short send to compositor"));
        }
        Ok(())
    }

    fn recv_available(&mut self) -> AppResult<Vec<wire::Message>> {
        loop {
            let mut buf = [0u8; 8192];
            match recv_with_fds(self.fd(), &mut buf) {
                Ok((0, _)) => return Err(app_error("compositor disconnected")),
                Ok((n, fds)) => {
                    self.recv_buf.extend_from_slice(&buf[..n]);
                    self.pending_fds.extend(fds);
                    if n < buf.len() {
                        break;
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(err) => return Err(app_error(format!("receive compositor events: {err}"))),
            }
        }

        let mut out = Vec::new();
        loop {
            let fds = std::mem::take(&mut self.pending_fds);
            match parse_message(&self.recv_buf, fds) {
                Ok(Some((msg, used))) => {
                    self.recv_buf.drain(..used);
                    out.push(msg);
                }
                Ok(None) => break,
                Err(err) => return Err(app_error(format!("parse compositor message: {err:?}"))),
            }
        }
        Ok(out)
    }
}

fn decode_arg<T>(value: std::result::Result<T, DecodeError>) -> AppResult<T> {
    value.map_err(|err| app_error(format!("decode compositor message: {err:?}")))
}

#[derive(Default)]
struct Globals {
    compositor: Option<u32>,
    shm: Option<u32>,
    wm_base: Option<u32>,
    seat: Option<u32>,
}

fn main() -> AppResult<()> {
    let socket = std::env::var("EDGERUN_COMPOSITOR_SOCKET")
        .or_else(|_| std::env::var("WAYLAND_DISPLAY").map(|name| format!("/tmp/{name}")))
        .unwrap_or_else(|_| DEFAULT_SOCKET.to_string());

    let mut wl = WaylandClient::connect(&socket)?;
    let globals = init_registry(&mut wl)?;
    bind_globals(&mut wl, &globals)?;
    create_surface(&mut wl)?;

    let mut frame = ShmFrame::new(WIDTH, HEIGHT)?;
    create_shm_buffer(&mut wl, &frame)?;

    let (tx, rx) = mpsc::channel();
    let mut tab = spawn_tab(tx, WIDTH, HEIGHT)?;
    let mut glyphs = GlyphCache::new(Arc::new(FONT_DATA.to_vec()), FONT_SIZE);
    let mut rgba = vec![0u8; frame.len];
    let keymap = Keymap::us_qwerty();
    let mut modifiers = Modifiers::default();
    let mut focused = false;
    let mut chat = ChatInput::default();
    let start_time = Instant::now();
    let mut show_help = true;
    let mut configured = false;
    let mut next_redraw = Instant::now();

    loop {
        for event in rx.try_iter() {
            let AppEvent::Pty(bytes) = event;
            apply_pty_bytes(&mut tab, &bytes);
            next_redraw = Instant::now();
        }

        for msg in wl.recv_available()? {
            handle_event(
                &mut wl,
                msg,
                &keymap,
                &mut modifiers,
                &mut focused,
                &mut show_help,
                &mut chat,
                &mut configured,
                &tab.writer,
                tab.app_cursor_keys,
                tab.terminal.kitty_keyboard,
            )?;
            next_redraw = Instant::now();
        }

        if configured && Instant::now() >= next_redraw {
            draw_and_present(
                &mut wl,
                &mut frame,
                &mut rgba,
                &mut glyphs,
                &tab,
                start_time,
                show_help,
                &chat,
                focused,
            )?;
            next_redraw = Instant::now() + Duration::from_millis(16);
        }

        std::thread::sleep(Duration::from_millis(4));
    }
}

fn init_registry(wl: &mut WaylandClient) -> AppResult<Globals> {
    wl.send(message_uint(
        1,
        wl_core::display_request::GET_REGISTRY,
        REGISTRY_ID,
    ))?;
    wl.send(message_uint(1, wl_core::display_request::SYNC, SYNC_ID))?;

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut globals = Globals::default();
    while Instant::now() < deadline {
        for msg in wl.recv_available()? {
            if msg.sender_id == REGISTRY_ID && msg.opcode == wl_core::registry_event::GLOBAL {
                let mut c = ArgCursor::from_message(&msg);
                let name = decode_arg(c.uint())?;
                let interface = decode_arg(c.string())?.map(|s| s.0).unwrap_or_default();
                let _version = decode_arg(c.uint())?;
                match interface.as_str() {
                    wl_compositor::WL_COMPOSITOR => globals.compositor = Some(name),
                    wl_shm::WL_SHM => globals.shm = Some(name),
                    xdg_shell::XDG_WM_BASE => globals.wm_base = Some(name),
                    wl_seat::WL_SEAT => globals.seat = Some(name),
                    _ => {}
                }
            } else if msg.sender_id == SYNC_ID && msg.opcode == wl_core::callback_event::DONE {
                return Ok(globals);
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Err(app_error("timed out waiting for compositor registry"))
}

fn bind_globals(wl: &mut WaylandClient, globals: &Globals) -> AppResult<()> {
    bind(
        wl,
        globals
            .compositor
            .ok_or_else(|| app_error("missing wl_compositor"))?,
        wl_compositor::WL_COMPOSITOR,
        4,
        COMPOSITOR_ID,
    )?;
    bind(
        wl,
        globals.shm.ok_or_else(|| app_error("missing wl_shm"))?,
        wl_shm::WL_SHM,
        1,
        SHM_ID,
    )?;
    bind(
        wl,
        globals
            .wm_base
            .ok_or_else(|| app_error("missing xdg_wm_base"))?,
        xdg_shell::XDG_WM_BASE,
        6,
        WM_BASE_ID,
    )?;
    if let Some(seat) = globals.seat {
        bind(wl, seat, wl_seat::WL_SEAT, 7, SEAT_ID)?;
        wl.send(message_uint(
            SEAT_ID,
            wl_seat::seat_request::GET_KEYBOARD,
            KEYBOARD_ID,
        ))?;
    }
    Ok(())
}

fn bind(
    wl: &mut WaylandClient,
    name: u32,
    interface: &str,
    version: u32,
    id: u32,
) -> AppResult<()> {
    let mut args = Vec::new();
    args.extend_from_slice(&name.to_le_bytes());
    encode_string(&mut args, interface);
    args.extend_from_slice(&version.to_le_bytes());
    args.extend_from_slice(&id.to_le_bytes());
    wl.send(wire::Message {
        sender_id: REGISTRY_ID,
        opcode: wl_core::registry_request::BIND,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    })
}

fn create_surface(wl: &mut WaylandClient) -> AppResult<()> {
    wl.send(message_uint(
        COMPOSITOR_ID,
        wl_compositor::compositor_request::CREATE_SURFACE,
        SURFACE_ID,
    ))?;
    let mut args = Vec::new();
    args.extend_from_slice(&XDG_SURFACE_ID.to_le_bytes());
    args.extend_from_slice(&SURFACE_ID.to_le_bytes());
    wl.send(wire::Message {
        sender_id: WM_BASE_ID,
        opcode: xdg_shell::xdg_wm_base_request::GET_XDG_SURFACE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    })?;
    wl.send(message_uint(
        XDG_SURFACE_ID,
        xdg_shell::xdg_surface_request::GET_TOPLEVEL,
        TOPLEVEL_ID,
    ))?;
    send_string(
        wl,
        TOPLEVEL_ID,
        xdg_shell::xdg_toplevel_request::SET_TITLE,
        "edgerun-term",
    )?;
    send_string(
        wl,
        TOPLEVEL_ID,
        xdg_shell::xdg_toplevel_request::SET_APP_ID,
        "edgerun-term",
    )?;
    wl.send(message_empty(
        SURFACE_ID,
        wl_compositor::surface_request::COMMIT,
    ))
}

fn create_shm_buffer(wl: &mut WaylandClient, frame: &ShmFrame) -> AppResult<()> {
    let mut args = Vec::new();
    args.extend_from_slice(&SHM_POOL_ID.to_le_bytes());
    args.extend_from_slice(&[0, 0, 0, 0]);
    args.extend_from_slice(&(frame.len as i32).to_le_bytes());
    wl.send(wire::Message {
        sender_id: SHM_ID,
        opcode: wl_shm::shm_request::CREATE_POOL,
        size: (8 + args.len()) as u16,
        args,
        fds: vec![frame.fd()],
    })?;

    let mut args = Vec::new();
    args.extend_from_slice(&BUFFER_ID.to_le_bytes());
    args.extend_from_slice(&0i32.to_le_bytes());
    args.extend_from_slice(&(frame.width as i32).to_le_bytes());
    args.extend_from_slice(&(frame.height as i32).to_le_bytes());
    args.extend_from_slice(&(frame.stride as i32).to_le_bytes());
    args.extend_from_slice(&wl_shm::format::XRGB8888.to_le_bytes());
    wl.send(wire::Message {
        sender_id: SHM_POOL_ID,
        opcode: wl_shm::shm_pool_request::CREATE_BUFFER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    })
}

fn send_string(wl: &mut WaylandClient, sender_id: u32, opcode: u16, value: &str) -> AppResult<()> {
    let mut args = Vec::new();
    encode_string(&mut args, value);
    wl.send(wire::Message {
        sender_id,
        opcode,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    })
}

fn handle_event(
    wl: &mut WaylandClient,
    msg: wire::Message,
    keymap: &Keymap,
    modifiers: &mut Modifiers,
    focused: &mut bool,
    show_help: &mut bool,
    chat: &mut ChatInput,
    configured: &mut bool,
    writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    app_cursor_keys: bool,
    kitty_keyboard: bool,
) -> AppResult<()> {
    if msg.sender_id == WM_BASE_ID && msg.opcode == xdg_shell::xdg_wm_base_event::PING {
        let mut c = ArgCursor::from_message(&msg);
        wl.send(message_uint(
            WM_BASE_ID,
            xdg_shell::xdg_wm_base_request::PONG,
            decode_arg(c.uint())?,
        ))?;
        return Ok(());
    }
    if msg.sender_id == XDG_SURFACE_ID && msg.opcode == xdg_shell::xdg_surface_event::CONFIGURE {
        let mut c = ArgCursor::from_message(&msg);
        let serial = decode_arg(c.uint())?;
        wl.send(message_uint(
            XDG_SURFACE_ID,
            xdg_shell::xdg_surface_request::ACK_CONFIGURE,
            serial,
        ))?;
        *configured = true;
        return Ok(());
    }
    if msg.sender_id == KEYBOARD_ID {
        match msg.opcode {
            wl_seat::keyboard_event::ENTER => *focused = true,
            wl_seat::keyboard_event::LEAVE => *focused = false,
            wl_seat::keyboard_event::KEY => {
                let mut c = ArgCursor::from_message(&msg);
                let _serial = decode_arg(c.uint())?;
                let _time = decode_arg(c.uint())?;
                let key = decode_arg(c.uint())? as u16;
                let state = decode_arg(c.uint())?;
                let pressed = state == wl_seat::key_state::PRESSED;
                process_key_event(key, pressed, modifiers);
                if pressed {
                    handle_key(
                        keymap,
                        key,
                        *modifiers,
                        writer,
                        app_cursor_keys,
                        kitty_keyboard,
                        show_help,
                        chat,
                    );
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn draw_and_present(
    wl: &mut WaylandClient,
    frame: &mut ShmFrame,
    rgba: &mut Vec<u8>,
    glyphs: &mut GlyphCache,
    tab: &Tab,
    start_time: Instant,
    show_help: bool,
    chat: &ChatInput,
    focused: bool,
) -> AppResult<()> {
    rgba.resize(frame.len, 0);
    let (cell_w, cell_h) = glyphs.cell_size();
    let chat_h = cell_h.saturating_add(12);
    let terminal_height = frame.height.saturating_sub(chat_h);
    let layout = layout_for(frame.width, terminal_height, cell_w.max(1), cell_h.max(1));
    draw_background(
        rgba,
        frame.width,
        frame.height,
        start_time,
        tab.terminal.default_bg(),
    );
    draw_tab_bar_cpu(
        &[TabVisual { title: &tab.title }],
        0,
        glyphs,
        rgba,
        frame.width,
        frame.height,
        cell_h + 12,
        BORDER_THICKNESS,
        start_time,
    );
    draw_grid(
        &tab.terminal,
        glyphs,
        rgba,
        frame.width,
        frame.height,
        cell_w,
        cell_h,
        layout.content_x,
        layout.content_y,
        None,
        None,
        start_time.elapsed().as_millis() / 500 % 2 == 0,
        None,
        None,
    );
    draw_cursor_overlay(
        &tab.terminal,
        glyphs,
        rgba,
        frame.width,
        frame.height,
        cell_w,
        cell_h,
        layout.content_x,
        layout.content_y,
        None,
        true,
        true,
    );
    if show_help {
        draw_help_bar_cpu(
            glyphs,
            rgba,
            frame.width,
            terminal_height,
            cell_h,
            BORDER_THICKNESS,
            "F1 help  F8/Ctrl+Space AI chat  Ctrl+C/Ctrl+D from shell",
            None,
            tab.terminal
                .prompt_status
                .map(|code| format!("Exit {code}")),
        );
    }
    draw_chat_input_cpu(glyphs, rgba, frame.width, frame.height, cell_h, chat);
    draw_border_cpu(
        rgba,
        frame.width,
        frame.height,
        BORDER_THICKNESS,
        BORDER_RADIUS,
        start_time,
        focused,
        BORDER_INSET,
    );

    for (dst, src) in frame
        .as_mut_bytes()
        .chunks_exact_mut(4)
        .zip(rgba.chunks_exact(4))
    {
        dst[0] = src[2];
        dst[1] = src[1];
        dst[2] = src[0];
        dst[3] = 0xff;
    }

    attach_damage_commit(wl, frame.width, frame.height)
}

fn attach_damage_commit(wl: &mut WaylandClient, width: u32, height: u32) -> AppResult<()> {
    let mut args = Vec::new();
    args.extend_from_slice(&BUFFER_ID.to_le_bytes());
    args.extend_from_slice(&0i32.to_le_bytes());
    args.extend_from_slice(&0i32.to_le_bytes());
    wl.send(wire::Message {
        sender_id: SURFACE_ID,
        opcode: wl_compositor::surface_request::ATTACH,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    })?;
    let mut args = Vec::new();
    args.extend_from_slice(&0i32.to_le_bytes());
    args.extend_from_slice(&0i32.to_le_bytes());
    args.extend_from_slice(&(width as i32).to_le_bytes());
    args.extend_from_slice(&(height as i32).to_le_bytes());
    wl.send(wire::Message {
        sender_id: SURFACE_ID,
        opcode: wl_compositor::surface_request::DAMAGE_BUFFER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    })?;
    wl.send(message_empty(
        SURFACE_ID,
        wl_compositor::surface_request::COMMIT,
    ))
}

fn draw_chat_input_cpu(
    glyphs: &mut GlyphCache,
    rgba: &mut [u8],
    width: u32,
    height: u32,
    cell_h: u32,
    chat: &ChatInput,
) {
    if width == 0 || height == 0 {
        return;
    }

    let bar_h = cell_h.saturating_add(12).min(height);
    let y0 = height.saturating_sub(bar_h) as i32;
    let y1 = height as i32;
    let bg = if chat.active {
        [22, 34, 44, 238]
    } else {
        [18, 20, 24, 220]
    };
    fill_rect(rgba, width, height, 0, y0, width as i32, y1, bg);
    fill_rect(
        rgba,
        width,
        height,
        0,
        y0,
        width as i32,
        y0 + 1,
        [90, 118, 138, 220],
    );

    let label = if chat.active { "AI > " } else { "F8 AI " };
    let body = if chat.active {
        chat.buffer.as_str()
    } else if chat.status.is_empty() {
        "chat input"
    } else {
        chat.status.as_str()
    };
    let text = format!("{label}{body}");
    let color = if chat.active {
        [235, 242, 248, 255]
    } else {
        [190, 200, 208, 235]
    };
    draw_text_line_clipped(
        glyphs,
        rgba,
        width,
        height,
        10,
        y0 + 6,
        &text,
        color,
        width.saturating_sub(10) as i32,
    );
}

fn layout_for(width: u32, height: u32, cell_w: u32, cell_h: u32) -> LayoutMetrics {
    compute_layout(
        width,
        height,
        cell_w,
        cell_h,
        cell_h + 12,
        BORDER_THICKNESS,
        BORDER_INSET,
        PADDING_X,
        PADDING_Y,
    )
}

fn spawn_tab(tx: mpsc::Sender<AppEvent>, width: u32, height: u32) -> AppResult<Tab> {
    let cols = 100u16;
    let rows = 32u16;
    let pty_system = NativePtySystem::default();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: width.min(u16::MAX as u32) as u16,
        pixel_height: height.min(u16::MAX as u32) as u16,
    })?;
    let mut cmd = default_shell();
    cmd.env("TERM", "xterm-256color");
    cmd.env("PROMPT_EOL_MARK", "");
    let child = pair.slave.spawn_command(cmd)?;

    let master: Arc<Mutex<Box<dyn MasterPty + Send>>> = Arc::new(Mutex::new(pair.master));
    let mut reader = master.lock().unwrap().try_clone_reader()?;
    let writer = Arc::new(Mutex::new(master.lock().unwrap().take_writer()?));
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let _ = tx.send(AppEvent::Pty(buf[..n].to_vec()));
                }
            }
        }
    });

    Ok(Tab {
        terminal: Terminal::new(cols as usize, rows as usize),
        parser: VteParser::new(),
        writer,
        _master: master,
        _child: child,
        app_cursor_keys: false,
        title: "edgerun-term".to_string(),
    })
}

fn default_shell() -> CommandBuilder {
    std::env::var("SHELL")
        .map(CommandBuilder::new)
        .unwrap_or_else(|_| CommandBuilder::new("/bin/sh"))
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    let mut out = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn trim_last_word(text: &mut String) {
    while text.ends_with(char::is_whitespace) {
        text.pop();
    }
    while text
        .chars()
        .last()
        .map(|ch| !ch.is_whitespace())
        .unwrap_or(false)
    {
        text.pop();
    }
}

fn apply_pty_bytes(tab: &mut Tab, bytes: &[u8]) {
    let mut performer = GridPerformer {
        grid: &mut tab.terminal,
        writer: tab.writer.clone(),
        app_cursor_keys: &mut tab.app_cursor_keys,
        dcs_state: None,
    };
    for byte in bytes {
        tab.parser.advance(&mut performer, *byte);
    }
    tab.title = tab
        .terminal
        .window_title()
        .filter(|title| !title.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "edgerun-term".to_string());
}

fn handle_key(
    keymap: &Keymap,
    scancode: u16,
    mods: Modifiers,
    writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    app_cursor_keys: bool,
    kitty_keyboard: bool,
    show_help: &mut bool,
    chat: &mut ChatInput,
) {
    let keysym = keymap.translate(scancode, &mods);

    if matches!(keysym, Keysym::Special(SpecialKey::F(8)))
        || (mods.ctrl && matches!(keysym, Keysym::Special(SpecialKey::Space)))
    {
        chat.toggle();
        return;
    }

    if chat.active {
        if let Some(command) = chat.handle_key(keysym, mods) {
            write_bytes(writer, command.as_bytes());
        }
        return;
    }

    match keysym {
        Keysym::Char(ch) => {
            if mods.ctrl {
                let lower = ch.to_ascii_lowercase();
                if lower == 'c' {
                    write_bytes(writer, b"\x03");
                    return;
                }
                if lower == 'd' {
                    write_bytes(writer, b"\x04");
                    return;
                }
                if lower == 'l' {
                    write_bytes(writer, b"\x0c");
                    return;
                }
                if ('a'..='z').contains(&lower) {
                    write_bytes(writer, &[(lower as u8) & 0x1f]);
                    return;
                }
            }
            let mut text = String::new();
            text.push(ch);
            if mods.alt {
                write_bytes(writer, b"\x1b");
            }
            write_bytes(writer, text.as_bytes());
        }
        Keysym::Special(SpecialKey::F(1)) => *show_help = !*show_help,
        Keysym::Special(SpecialKey::Return) => write_bytes(writer, b"\r"),
        Keysym::Special(SpecialKey::Backspace) => write_bytes(writer, b"\x7f"),
        Keysym::Special(SpecialKey::Tab) => write_bytes(writer, b"\t"),
        Keysym::Special(SpecialKey::Escape) => write_bytes(writer, b"\x1b"),
        Keysym::Special(SpecialKey::Delete) => write_bytes(writer, b"\x1b[3~"),
        Keysym::Special(SpecialKey::Home) => write_bytes(writer, b"\x1b[H"),
        Keysym::Special(SpecialKey::End) => write_bytes(writer, b"\x1b[F"),
        Keysym::Special(SpecialKey::PageUp) => write_bytes(writer, b"\x1b[5~"),
        Keysym::Special(SpecialKey::PageDown) => write_bytes(writer, b"\x1b[6~"),
        Keysym::Special(SpecialKey::Up) => {
            cursor(writer, app_cursor_keys, b'A', mods, kitty_keyboard)
        }
        Keysym::Special(SpecialKey::Down) => {
            cursor(writer, app_cursor_keys, b'B', mods, kitty_keyboard)
        }
        Keysym::Special(SpecialKey::Right) => {
            cursor(writer, app_cursor_keys, b'C', mods, kitty_keyboard)
        }
        Keysym::Special(SpecialKey::Left) => {
            cursor(writer, app_cursor_keys, b'D', mods, kitty_keyboard)
        }
        _ => {}
    }
}

fn cursor(
    writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    app_cursor_keys: bool,
    code: u8,
    mods: Modifiers,
    kitty_keyboard: bool,
) {
    let modifier = 1 + mods.shift as u8 + (mods.alt as u8) * 2 + (mods.ctrl as u8) * 4;
    if kitty_keyboard || modifier != 1 {
        write_bytes(
            writer,
            format!("\x1b[1;{}{}", modifier, code as char).as_bytes(),
        );
    } else if app_cursor_keys {
        write_bytes(writer, &[0x1b, b'O', code]);
    } else {
        write_bytes(writer, &[0x1b, b'[', code]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_handles_spaces_and_quotes() {
        assert_eq!(shell_quote("hello world"), "'hello world'");
        assert_eq!(shell_quote("can't fail"), "'can'\\''t fail'");
    }

    #[test]
    fn chat_submit_builds_zen_client_command() {
        let mut chat = ChatInput {
            active: true,
            buffer: "explain src/main.rs".to_string(),
            status: String::new(),
        };

        let command = chat.submit_command().expect("command");

        assert_eq!(command, "zen-client 'explain src/main.rs'\r");
        assert!(!chat.active);
        assert!(chat.buffer.is_empty());
        assert!(chat.status.contains("Submitted"));
    }

    #[test]
    fn chat_submit_ignores_empty_prompt() {
        let mut chat = ChatInput {
            active: true,
            buffer: "   ".to_string(),
            status: String::new(),
        };

        assert!(chat.submit_command().is_none());
        assert!(chat.active);
        assert_eq!(chat.status, "Empty prompt");
    }
}
