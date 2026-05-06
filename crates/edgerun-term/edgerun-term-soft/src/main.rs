use std::io::{Read, Write};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use edgerun_term_core::render::layout::{LayoutMetrics, compute_layout};
use edgerun_term_core::render::{
    FONT_DATA, FONT_SIZE, GlyphCache, TabVisual, draw_background, draw_border_cpu,
    draw_cursor_overlay, draw_grid, draw_help_bar_cpu, draw_tab_bar_cpu,
};
use edgerun_term_core::terminal::{GridPerformer, Terminal, write_bytes};
use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use softbuffer::{Context, Surface};
use vte::Parser as VteParser;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::raw_window_handle::{DisplayHandle, HasDisplayHandle};
use winit::window::{Window, WindowId};

const BORDER_THICKNESS: u32 = 2;
const BORDER_RADIUS: u32 = 8;
const BORDER_INSET: u32 = 0;
const PADDING_X: u32 = 8;
const PADDING_Y: u32 = 6;

#[derive(Debug)]
enum AppEvent {
    Pty { tab_id: usize, bytes: Vec<u8> },
}

struct Tab {
    id: usize,
    terminal: Terminal,
    parser: VteParser,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    app_cursor_keys: bool,
    title: String,
}

struct App {
    context: Context<DisplayHandle<'static>>,
    window: Option<Arc<Window>>,
    surface: Option<Surface<DisplayHandle<'static>, Arc<Window>>>,
    glyphs: GlyphCache,
    tabs: Vec<Tab>,
    active: usize,
    next_tab_id: usize,
    proxy: EventLoopProxy<AppEvent>,
    modifiers: ModifiersState,
    rgba: Vec<u8>,
    start_time: Instant,
    focused: bool,
    show_help: bool,
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let context = Context::new(unsafe {
        std::mem::transmute::<DisplayHandle<'_>, DisplayHandle<'static>>(
            event_loop.display_handle()?,
        )
    })
    .map_err(|e| anyhow::anyhow!("create softbuffer context: {e}"))?;

    let cols = 100usize;
    let rows = 32usize;
    let first_tab = spawn_tab(0, cols as u16, rows as u16, 0, 0, proxy.clone())?;
    let mut app = App {
        context,
        window: None,
        surface: None,
        glyphs: GlyphCache::new(Arc::new(FONT_DATA.to_vec()), FONT_SIZE),
        tabs: vec![first_tab],
        active: 0,
        next_tab_id: 1,
        proxy,
        modifiers: ModifiersState::empty(),
        rgba: Vec::new(),
        start_time: Instant::now(),
        focused: true,
        show_help: true,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("edgerun-term-soft")
            .with_inner_size(LogicalSize::new(960.0, 540.0));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let surface = Surface::new(&self.context, window.clone()).expect("create surface");
        self.surface = Some(surface);
        self.window = Some(window);
        self.resize_to_window();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.resize_to_window();
                self.request_redraw();
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.request_redraw();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                self.handle_key(event);
                self.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                self.draw();
            }
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        let AppEvent::Pty { tab_id, bytes } = event;
        self.apply_pty_bytes(tab_id, &bytes);
        self.request_redraw();
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {}
}

impl App {
    fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    fn apply_pty_bytes(&mut self, tab_id: usize, bytes: &[u8]) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        let mut performer = GridPerformer {
            grid: &mut tab.terminal,
            writer: tab.writer.clone(),
            app_cursor_keys: &mut tab.app_cursor_keys,
            dcs_state: None,
        };
        for byte in bytes {
            tab.parser.advance(&mut performer, *byte);
        }
        refresh_tab_title(tab);
    }

    fn resize_to_window(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }

        let (cell_w, cell_h) = self.glyphs.cell_size();
        let cell_w = cell_w.max(1);
        let cell_h = cell_h.max(1);
        let layout = self.layout_for(size.width, size.height, cell_w, cell_h);
        for tab in &mut self.tabs {
            if layout.cols != tab.terminal.cols || layout.rows != tab.terminal.rows {
                tab.terminal.resize(layout.cols, layout.rows);
            }
            if let Ok(master) = tab.master.lock() {
                let _ = master.resize(PtySize {
                    rows: layout.rows.min(u16::MAX as usize) as u16,
                    cols: layout.cols.min(u16::MAX as usize) as u16,
                    pixel_width: layout.usable_width.min(u16::MAX as u32) as u16,
                    pixel_height: layout.usable_height.min(u16::MAX as u32) as u16,
                });
            }
        }
    }

    fn draw(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }

        let width = size.width;
        let height = size.height;
        self.rgba.resize((width * height * 4) as usize, 0);
        let (cell_w, cell_h) = self.glyphs.cell_size();
        let cell_w = cell_w.max(1);
        let cell_h = cell_h.max(1);
        let layout = self.layout_for(width, height, cell_w, cell_h);
        let tab_bar_height = cell_h + 12;
        let bg = self
            .active_tab()
            .map(|tab| tab.terminal.default_bg())
            .unwrap_or_else(|| edgerun_term_core::terminal::DEFAULT_BG);
        draw_background(&mut self.rgba, width, height, self.start_time, bg);

        let tab_visuals: Vec<_> = self
            .tabs
            .iter()
            .map(|tab| TabVisual { title: &tab.title })
            .collect();
        draw_tab_bar_cpu(
            &tab_visuals,
            self.active,
            &mut self.glyphs,
            &mut self.rgba,
            width,
            height,
            tab_bar_height,
            BORDER_THICKNESS,
            self.start_time,
        );

        if let Some(tab) = self.tabs.get(self.active) {
            let status_label = tab
                .terminal
                .prompt_status
                .map(|code| format!("Exit {code}"));
            draw_grid(
                &tab.terminal,
                &mut self.glyphs,
                &mut self.rgba,
                width,
                height,
                cell_w,
                cell_h,
                layout.content_x,
                layout.content_y,
                None,
                None,
                self.start_time.elapsed().as_millis() / 500 % 2 == 0,
                None,
                None,
            );
            draw_cursor_overlay(
                &tab.terminal,
                &mut self.glyphs,
                &mut self.rgba,
                width,
                height,
                cell_w,
                cell_h,
                layout.content_x,
                layout.content_y,
                None,
                true,
                true,
            );
            if self.show_help {
                draw_help_bar_cpu(
                    &mut self.glyphs,
                    &mut self.rgba,
                    width,
                    height,
                    cell_h,
                    BORDER_THICKNESS,
                    "F1 help  Ctrl+Shift+T new tab  Ctrl+Shift+W close  Ctrl+Tab switch  Alt+1..9 jump",
                    None,
                    status_label,
                );
            }
        }

        draw_border_cpu(
            &mut self.rgba,
            width,
            height,
            BORDER_THICKNESS,
            BORDER_RADIUS,
            self.start_time,
            self.focused,
            BORDER_INSET,
        );

        let Some(surface) = &mut self.surface else {
            return;
        };
        surface
            .resize(
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            )
            .expect("resize surface");
        let mut buffer = surface.buffer_mut().expect("surface buffer");
        for (dst, px) in buffer.iter_mut().zip(self.rgba.chunks_exact(4)) {
            *dst = (px[0] as u32) << 16 | (px[1] as u32) << 8 | px[2] as u32;
        }
        buffer.present().expect("present surface");
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn layout_for(&self, width: u32, height: u32, cell_w: u32, cell_h: u32) -> LayoutMetrics {
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

    fn handle_key(&mut self, event: KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }

        if matches!(event.logical_key, Key::Named(NamedKey::F1)) {
            self.show_help = !self.show_help;
            return;
        }

        if self.modifiers.control_key()
            && matches!(event.logical_key, Key::Named(NamedKey::Tab))
            && !self.tabs.is_empty()
        {
            if self.modifiers.shift_key() {
                self.active = self.active.checked_sub(1).unwrap_or(self.tabs.len() - 1);
            } else {
                self.active = (self.active + 1) % self.tabs.len();
            }
            return;
        }

        if let Key::Character(text) = &event.logical_key {
            let lower = text.to_lowercase();
            if self.modifiers.control_key() && self.modifiers.shift_key() && lower == "t" {
                self.new_tab();
                return;
            }
            if self.modifiers.control_key() && self.modifiers.shift_key() && lower == "w" {
                self.close_active_tab();
                return;
            }
            if self.modifiers.alt_key() {
                if lower == "t" {
                    self.new_tab();
                    return;
                }
                if lower == "q" {
                    self.close_active_tab();
                    return;
                }
                if let Some(digit) = lower.chars().next()
                    && ('1'..='9').contains(&digit)
                {
                    let idx = (digit as u8 - b'1') as usize;
                    if idx < self.tabs.len() {
                        self.active = idx;
                    }
                    return;
                }
            }
        }

        let modifiers = self.modifiers;
        if let Some(tab) = self.active_tab() {
            send_key(
                event,
                modifiers,
                &tab.writer,
                tab.app_cursor_keys,
                tab.terminal.kitty_keyboard,
            );
        }
    }

    fn new_tab(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let size = window.inner_size();
        let (cell_w, cell_h) = self.glyphs.cell_size();
        let layout = self.layout_for(size.width, size.height, cell_w.max(1), cell_h.max(1));
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        if let Ok(tab) = spawn_tab(
            id,
            layout.cols.min(u16::MAX as usize) as u16,
            layout.rows.min(u16::MAX as usize) as u16,
            layout.usable_width,
            layout.usable_height,
            self.proxy.clone(),
        ) {
            self.tabs.push(tab);
            self.active = self.tabs.len().saturating_sub(1);
        }
    }

    fn close_active_tab(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }
        let mut tab = self.tabs.remove(self.active);
        let _ = tab._child.kill();
        let _ = tab._child.wait();
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len().saturating_sub(1);
        }
    }
}

fn spawn_tab(
    id: usize,
    cols: u16,
    rows: u16,
    usable_width: u32,
    usable_height: u32,
    proxy: EventLoopProxy<AppEvent>,
) -> anyhow::Result<Tab> {
    let pty_system = NativePtySystem::default();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: usable_width.min(u16::MAX as u32) as u16,
        pixel_height: usable_height.min(u16::MAX as u32) as u16,
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
                Ok(0) => break,
                Ok(n) => {
                    let _ = proxy.send_event(AppEvent::Pty {
                        tab_id: id,
                        bytes: buf[..n].to_vec(),
                    });
                }
                Err(_) => break,
            }
        }
    });

    Ok(Tab {
        id,
        terminal: Terminal::new(cols as usize, rows as usize),
        parser: VteParser::new(),
        writer,
        master,
        _child: child,
        app_cursor_keys: false,
        title: format!("Tab {}", id + 1),
    })
}

fn default_shell() -> CommandBuilder {
    if let Ok(shell) = std::env::var("SHELL") {
        return CommandBuilder::new(shell);
    }
    CommandBuilder::new("/bin/sh")
}

fn refresh_tab_title(tab: &mut Tab) {
    tab.title = tab
        .terminal
        .window_title()
        .filter(|title| !title.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Tab {}", tab.id + 1));
}

fn encode_modifiers(mods: ModifiersState) -> u8 {
    1 + (mods.shift_key() as u8) + (mods.alt_key() as u8) * 2 + (mods.control_key() as u8) * 4
}

fn send_cursor_key(
    writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    app_cursor_keys: bool,
    code: u8,
    mods: ModifiersState,
    kitty_keyboard: bool,
) {
    let modifier = encode_modifiers(mods);
    if modifier == 1 && !kitty_keyboard {
        if app_cursor_keys {
            write_bytes(writer, &[0x1b, b'O', code]);
        } else {
            write_bytes(writer, &[0x1b, b'[', code]);
        }
        return;
    }

    if kitty_keyboard {
        let seq = format!("\x1b[{};{}u", code, modifier);
        write_bytes(writer, seq.as_bytes());
    } else {
        let prefix = if app_cursor_keys { "\x1bO" } else { "\x1b[" };
        let seq = format!("{}1;{}{}", prefix, modifier, code as char);
        write_bytes(writer, seq.as_bytes());
    }
}

fn function_key_sequence(key: NamedKey, mods: ModifiersState) -> Option<String> {
    let modifier = encode_modifiers(mods);
    match key {
        NamedKey::F1 => Some(if modifier == 1 {
            "\x1bOP".to_string()
        } else {
            format!("\x1b[1;{}P", modifier)
        }),
        NamedKey::F2 => Some(if modifier == 1 {
            "\x1bOQ".to_string()
        } else {
            format!("\x1b[1;{}Q", modifier)
        }),
        NamedKey::F3 => Some(if modifier == 1 {
            "\x1bOR".to_string()
        } else {
            format!("\x1b[1;{}R", modifier)
        }),
        NamedKey::F4 => Some(if modifier == 1 {
            "\x1bOS".to_string()
        } else {
            format!("\x1b[1;{}S", modifier)
        }),
        NamedKey::F5 => Some(format!("\x1b[15;{}~", modifier)),
        NamedKey::F6 => Some(format!("\x1b[17;{}~", modifier)),
        NamedKey::F7 => Some(format!("\x1b[18;{}~", modifier)),
        NamedKey::F8 => Some(format!("\x1b[19;{}~", modifier)),
        NamedKey::F9 => Some(format!("\x1b[20;{}~", modifier)),
        NamedKey::F10 => Some(format!("\x1b[21;{}~", modifier)),
        NamedKey::F11 => Some(format!("\x1b[23;{}~", modifier)),
        NamedKey::F12 => Some(format!("\x1b[24;{}~", modifier)),
        _ => None,
    }
}

fn send_key(
    event: KeyEvent,
    modifiers: ModifiersState,
    writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    app_cursor_keys: bool,
    kitty_keyboard: bool,
) {
    use NamedKey::*;

    match event.logical_key {
        Key::Named(NamedKey::Enter) => write_bytes(writer, b"\r"),
        Key::Named(NamedKey::Backspace) => write_bytes(writer, b"\x7f"),
        Key::Named(NamedKey::Delete) => write_bytes(writer, b"\x1b[3~"),
        Key::Named(NamedKey::Tab) => write_bytes(writer, b"\t"),
        Key::Named(NamedKey::Escape) => write_bytes(writer, b"\x1b"),
        Key::Named(NamedKey::ArrowUp) => {
            send_cursor_key(writer, app_cursor_keys, b'A', modifiers, kitty_keyboard);
        }
        Key::Named(NamedKey::ArrowDown) => {
            send_cursor_key(writer, app_cursor_keys, b'B', modifiers, kitty_keyboard);
        }
        Key::Named(NamedKey::ArrowRight) => {
            send_cursor_key(writer, app_cursor_keys, b'C', modifiers, kitty_keyboard);
        }
        Key::Named(NamedKey::ArrowLeft) => {
            send_cursor_key(writer, app_cursor_keys, b'D', modifiers, kitty_keyboard);
        }
        Key::Named(nk)
            if matches!(
                nk,
                F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 | F9 | F10 | F11 | F12
            ) =>
        {
            if let Some(seq) = function_key_sequence(nk, modifiers) {
                write_bytes(writer, seq.as_bytes());
            }
        }
        Key::Named(NamedKey::Space) => write_bytes(writer, b" "),
        Key::Named(_) => {}
        Key::Character(text) => {
            if text.is_empty() {
                return;
            }
            if kitty_keyboard
                && let Some(ch) = event.text.as_deref().and_then(|text| text.chars().next())
            {
                let seq = format!("\x1b[{};{}u", ch as u32, encode_modifiers(modifiers));
                write_bytes(writer, seq.as_bytes());
                return;
            }
            if modifiers.control_key() && !modifiers.shift_key() {
                if let Some(ch) = text.chars().next() {
                    let ctrl = (ch.to_ascii_uppercase() as u8) & 0x1f;
                    write_bytes(writer, &[ctrl]);
                }
            } else if let Some(text) = event.text {
                write_bytes(writer, text.as_bytes());
            } else {
                write_bytes(writer, text.as_bytes());
            }
        }
        Key::Unidentified(_) | Key::Dead(_) => {}
    }
}
