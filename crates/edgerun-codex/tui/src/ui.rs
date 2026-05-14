use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use edgerun_ui_core::gpu::gl::GlRenderer;
use edgerun_ui_core::gpu::{Color4, FontAtlas, GpuHit, GpuRect, GpuScene, HitKind, UiKey};

use super::{AgentEvent, provider, read_chatgpt_auth, run_agent_loop, user_item};

const SDL_INIT_VIDEO: u32 = 0x0000_0020;
const SDL_WINDOWPOS_CENTERED: c_int = 0x2fff_0000u32 as c_int;
const SDL_WINDOW_OPENGL: u32 = 0x0000_0002;
const SDL_WINDOW_SHOWN: u32 = 0x0000_0004;
const SDL_WINDOW_RESIZABLE: u32 = 0x0000_0020;
const SDL_QUIT: u32 = 0x100;
const SDL_KEYDOWN: u32 = 0x300;
const SDL_TEXTINPUT: u32 = 0x303;
const SDL_MOUSEBUTTONDOWN: u32 = 0x401;
const SDL_MOUSEBUTTONUP: u32 = 0x402;
const SDL_MOUSEWHEEL: u32 = 0x403;
const SDL_WINDOWEVENT: u32 = 0x200;
const SDL_WINDOWEVENT_RESIZED: u8 = 0x05;
const SDL_GL_CONTEXT_MAJOR_VERSION: c_int = 17;
const SDL_GL_CONTEXT_MINOR_VERSION: c_int = 18;
const SDL_GL_CONTEXT_PROFILE_MASK: c_int = 21;
const SDL_GL_CONTEXT_PROFILE_CORE: c_int = 0x0001;
const SDL_GL_DOUBLEBUFFER: c_int = 5;
const SDLK_ESCAPE: i32 = 27;
const KMOD_SHIFT: u16 = 0x0003;

const SEND_ID: u32 = 81_000;
const NEW_CHAT_ID: u32 = 81_001;
const CLEAR_ID: u32 = 81_002;

const BG: Color4 = Color4::rgba(0.035, 0.039, 0.047, 1.0);
const SIDEBAR: Color4 = Color4::rgba(0.025, 0.029, 0.035, 1.0);
const PANEL: Color4 = Color4::rgba(0.055, 0.063, 0.075, 1.0);
const PANEL_2: Color4 = Color4::rgba(0.075, 0.086, 0.102, 1.0);
const INPUT: Color4 = Color4::rgba(0.021, 0.025, 0.031, 1.0);
const BORDER: Color4 = Color4::rgba(0.20, 0.23, 0.27, 1.0);
const TEXT: Color4 = Color4::rgba(0.91, 0.92, 0.90, 1.0);
const MUTED: Color4 = Color4::rgba(0.55, 0.59, 0.63, 1.0);
const FAINT: Color4 = Color4::rgba(0.37, 0.41, 0.45, 1.0);
const ACCENT: Color4 = Color4::rgba(0.11, 0.67, 0.52, 1.0);
const ACCENT_DIM: Color4 = Color4::rgba(0.065, 0.36, 0.30, 1.0);
const USER: Color4 = Color4::rgba(0.13, 0.18, 0.22, 1.0);
const TOOL: Color4 = Color4::rgba(0.13, 0.105, 0.055, 1.0);
const ERROR: Color4 = Color4::rgba(0.24, 0.07, 0.07, 1.0);

#[repr(C)]
struct SDL_Window(c_void);

type SdlGlContext = *mut c_void;

#[repr(C)]
struct SdlEvent {
    data: [u8; 56],
}

impl SdlEvent {
    fn event_type(&self) -> u32 {
        u32::from_ne_bytes([self.data[0], self.data[1], self.data[2], self.data[3]])
    }

    fn window_event(&self) -> u8 {
        self.data[8]
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

    fn key_mod(&self) -> u16 {
        u16::from_ne_bytes([self.data[24], self.data[25]])
    }

    fn mouse_x(&self) -> f32 {
        i32::from_ne_bytes([self.data[20], self.data[21], self.data[22], self.data[23]]) as f32
    }

    fn mouse_y(&self) -> f32 {
        i32::from_ne_bytes([self.data[24], self.data[25], self.data[26], self.data[27]]) as f32
    }

    fn wheel_y(&self) -> f32 {
        i32::from_ne_bytes([self.data[24], self.data[25], self.data[26], self.data[27]]) as f32
    }

    fn text_input(&self) -> Option<String> {
        let bytes = &self.data[12..44];
        let len = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        if len == 0 {
            None
        } else {
            std::str::from_utf8(&bytes[..len])
                .ok()
                .map(ToString::to_string)
        }
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
    fn SDL_PollEvent(event: *mut SdlEvent) -> c_int;
    fn SDL_Delay(ms: u32);
    fn SDL_StartTextInput();
    fn SDL_StopTextInput();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    User,
    Assistant,
    ToolRunning,
    ToolSuccess,
    ToolError,
    Error,
}

#[derive(Clone, Debug)]
struct Message {
    role: Role,
    text: String,
}

#[derive(Debug)]
struct CodexUi {
    model: String,
    input: String,
    input_cursor: usize,
    status: String,
    busy: bool,
    scroll: f32,
    pressed: Option<u32>,
    messages: Vec<Message>,
    command_tx: mpsc::Sender<WorkerCommand>,
    event_rx: mpsc::Receiver<WorkerEvent>,
}

#[derive(Debug)]
enum WorkerCommand {
    Prompt(String),
    Reset,
}

#[derive(Debug)]
enum WorkerEvent {
    Ready,
    Status(String),
    AssistantText(String),
    ToolStarted(String),
    ToolCompleted {
        name: String,
        summary: String,
        success: bool,
    },
    Error(String),
    Done,
}

pub struct UiOptions {
    pub model: String,
    pub frames: Option<u32>,
    pub dump_scene: bool,
}

pub fn run(options: UiOptions) -> Result<(), String> {
    let (command_tx, command_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    start_worker(options.model.clone(), command_rx, event_tx);

    let mut state = CodexUi {
        model: options.model,
        input: String::new(),
        input_cursor: 0,
        status: "Starting Codex worker".to_string(),
        busy: true,
        scroll: 1.0,
        pressed: None,
        messages: vec![Message {
            role: Role::Assistant,
            text: "EdgeRun Codex UI is ready for local workspace work.".to_string(),
        }],
        command_tx,
        event_rx,
    };

    if options.dump_scene {
        let atlas = FontAtlas::load_inter(18.0)?;
        let mut scene = GpuScene::new(BG);
        state.drain_events();
        build_scene(&mut scene, &atlas, &state, 1120.0, 720.0);
        println!(
            "edgerun-codex ui scene rects={} text_quads={}",
            scene.rects().len(),
            scene.text_quads().len()
        );
        return Ok(());
    }

    run_window(options.frames, state)
}

fn start_worker(
    model: String,
    command_rx: mpsc::Receiver<WorkerCommand>,
    event_tx: mpsc::Sender<WorkerEvent>,
) {
    thread::spawn(move || {
        let runtime = match edgerun_tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = event_tx.send(WorkerEvent::Error(format!("runtime: {error}")));
                return;
            }
        };
        let auth = match read_chatgpt_auth() {
            Ok(auth) => auth,
            Err(error) => {
                let _ = event_tx.send(WorkerEvent::Error(format!("auth: {error}")));
                return;
            }
        };
        let client = codex_core::ModelClient::new_native(model, provider(), auth);
        let mut history = Vec::new();
        let _ = event_tx.send(WorkerEvent::Ready);

        while let Ok(command) = command_rx.recv() {
            match command {
                WorkerCommand::Reset => {
                    history.clear();
                    let _ = event_tx.send(WorkerEvent::Status("New chat".to_string()));
                }
                WorkerCommand::Prompt(prompt) => {
                    history.push(user_item(prompt));
                    let _ = event_tx.send(WorkerEvent::Status("Thinking".to_string()));
                    let tx = event_tx.clone();
                    let mut emit = move |event: AgentEvent| match event {
                        AgentEvent::AssistantText(text) => {
                            let _ = tx.send(WorkerEvent::AssistantText(text));
                        }
                        AgentEvent::ToolStarted(name) => {
                            let _ = tx.send(WorkerEvent::ToolStarted(name));
                        }
                        AgentEvent::ToolCompleted {
                            name,
                            summary,
                            success,
                        } => {
                            let _ = tx.send(WorkerEvent::ToolCompleted {
                                name,
                                summary,
                                success,
                            });
                        }
                    };
                    match runtime.block_on(run_agent_loop(&client, history, Some(&mut emit))) {
                        Ok((_, next_history)) => {
                            history = next_history;
                            let _ = event_tx.send(WorkerEvent::Done);
                        }
                        Err(error) => {
                            history = Vec::new();
                            let _ = event_tx.send(WorkerEvent::Error(error.to_string()));
                        }
                    }
                }
            }
        }
    });
}

fn run_window(frames: Option<u32>, mut state: CodexUi) -> Result<(), String> {
    let _sdl = Sdl::init()?;
    unsafe {
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 3);
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 3);
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_CORE);
        SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);
    }

    let mut width = 1180;
    let mut height = 760;
    let title = CString::new("EdgeRun Codex").map_err(|error| error.to_string())?;
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
    let mut scene = GpuScene::new(BG);
    let mut running = true;
    let mut scene_dirty = true;
    let mut rendered_frames = 0u32;

    while running {
        state.drain_events();
        let mut event = SdlEvent { data: [0; 56] };
        while unsafe { SDL_PollEvent(&mut event) } != 0 {
            match event.event_type() {
                SDL_QUIT => running = false,
                SDL_KEYDOWN if event.key_sym() == SDLK_ESCAPE => running = false,
                SDL_KEYDOWN => {
                    state.handle_key(sdl_key(event.key_sym()), event.key_mod());
                    scene_dirty = true;
                }
                SDL_TEXTINPUT => {
                    if let Some(text) = event.text_input() {
                        state.insert_text(&text);
                        scene_dirty = true;
                    }
                }
                SDL_MOUSEBUTTONDOWN => {
                    state.pointer_down(&scene, event.mouse_x(), event.mouse_y());
                    scene_dirty = true;
                }
                SDL_MOUSEBUTTONUP => {
                    state.pointer_up(&scene, event.mouse_x(), event.mouse_y());
                    scene_dirty = true;
                }
                SDL_MOUSEWHEEL => {
                    state.scroll = (state.scroll - event.wheel_y() * 0.08).clamp(0.0, 1.0);
                    scene_dirty = true;
                }
                SDL_WINDOWEVENT if event.window_event() == SDL_WINDOWEVENT_RESIZED => {
                    width = event.data1().max(720);
                    height = event.data2().max(480);
                    scene_dirty = true;
                }
                _ => {}
            }
        }

        if scene_dirty {
            build_scene(&mut scene, &atlas, &state, width as f32, height as f32);
            renderer.render(width, height, &scene);
            unsafe {
                SDL_GL_SwapWindow(window.0);
            }
            rendered_frames = rendered_frames.saturating_add(1);
            scene_dirty = false;
        }
        if frames.is_some_and(|limit| rendered_frames >= limit) {
            running = false;
        } else if frames.is_some() || state.busy {
            scene_dirty = true;
        }
        unsafe {
            SDL_Delay(8);
        }
        thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

impl CodexUi {
    fn drain_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                WorkerEvent::Ready => {
                    self.busy = false;
                    self.status = "Ready".to_string();
                }
                WorkerEvent::Status(status) => self.status = status,
                WorkerEvent::AssistantText(text) => {
                    self.messages.push(Message {
                        role: Role::Assistant,
                        text,
                    });
                    self.scroll = 1.0;
                }
                WorkerEvent::ToolStarted(name) => {
                    self.messages.push(Message {
                        role: Role::ToolRunning,
                        text: format!("Running {name}"),
                    });
                    self.status = "Tool running".to_string();
                    self.scroll = 1.0;
                }
                WorkerEvent::ToolCompleted {
                    name,
                    summary,
                    success,
                } => {
                    self.messages.push(Message {
                        role: if success {
                            Role::ToolSuccess
                        } else {
                            Role::ToolError
                        },
                        text: format!("{name}\n{summary}"),
                    });
                    self.status = if success {
                        "Tool completed".to_string()
                    } else {
                        "Tool failed".to_string()
                    };
                    self.scroll = 1.0;
                }
                WorkerEvent::Error(error) => {
                    self.busy = false;
                    self.status = "Error".to_string();
                    self.messages.push(Message {
                        role: Role::Error,
                        text: error,
                    });
                    self.scroll = 1.0;
                }
                WorkerEvent::Done => {
                    self.busy = false;
                    self.status = "Ready".to_string();
                }
            }
        }
    }

    fn handle_key(&mut self, key: UiKey, modifiers: u16) {
        match key {
            UiKey::Backspace => {
                self.delete_before_cursor();
            }
            UiKey::Enter if modifiers & KMOD_SHIFT != 0 => self.insert_text("\n"),
            UiKey::Enter => self.submit(),
            UiKey::ArrowUp => self.scroll = (self.scroll - 0.06).clamp(0.0, 1.0),
            UiKey::ArrowDown => self.scroll = (self.scroll + 0.06).clamp(0.0, 1.0),
            UiKey::ArrowLeft => self.move_cursor_left(),
            UiKey::ArrowRight => self.move_cursor_right(),
            UiKey::Escape | UiKey::Tab | UiKey::Other(_) => {}
        }
    }

    fn insert_text(&mut self, text: &str) {
        let byte_index = self.cursor_byte_index();
        self.input.insert_str(byte_index, text);
        self.input_cursor += text.chars().count();
    }

    fn delete_before_cursor(&mut self) {
        if self.input_cursor == 0 {
            return;
        }
        let end = self.cursor_byte_index();
        self.input_cursor -= 1;
        let start = self.cursor_byte_index();
        self.input.replace_range(start..end, "");
    }

    fn move_cursor_left(&mut self) {
        self.input_cursor = self.input_cursor.saturating_sub(1);
    }

    fn move_cursor_right(&mut self) {
        self.input_cursor = (self.input_cursor + 1).min(self.input.chars().count());
    }

    fn cursor_byte_index(&self) -> usize {
        self.input
            .char_indices()
            .nth(self.input_cursor)
            .map(|(index, _)| index)
            .unwrap_or(self.input.len())
    }

    fn pointer_down(&mut self, scene: &GpuScene, x: f32, y: f32) {
        self.pressed = scene.hit_test(x, y).map(|hit| hit.id);
    }

    fn pointer_up(&mut self, scene: &GpuScene, x: f32, y: f32) {
        let released = scene.hit_test(x, y).map(|hit| hit.id);
        let pressed = self.pressed.take();
        if pressed != released {
            return;
        }
        match released {
            Some(SEND_ID) => self.submit(),
            Some(NEW_CHAT_ID) => self.new_chat(),
            Some(CLEAR_ID) => {
                self.messages.clear();
                self.status = "Transcript cleared".to_string();
            }
            _ => {}
        }
    }

    fn submit(&mut self) {
        let prompt = self.input.trim().to_string();
        if prompt.is_empty() {
            self.status = "Type a prompt before sending".to_string();
            return;
        }
        if self.busy {
            self.status = "Codex is still working".to_string();
            return;
        }
        self.messages.push(Message {
            role: Role::User,
            text: prompt.clone(),
        });
        self.input.clear();
        self.input_cursor = 0;
        self.busy = true;
        self.status = "Queued".to_string();
        self.scroll = 1.0;
        if self.command_tx.send(WorkerCommand::Prompt(prompt)).is_err() {
            self.busy = false;
            self.status = "Worker disconnected".to_string();
        }
    }

    fn new_chat(&mut self) {
        self.messages.clear();
        self.messages.push(Message {
            role: Role::Assistant,
            text: "New chat started.".to_string(),
        });
        self.scroll = 1.0;
        let _ = self.command_tx.send(WorkerCommand::Reset);
    }
}

fn build_scene(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    state: &CodexUi,
    width: f32,
    height: f32,
) {
    scene.clear = BG;
    scene.clear_rects();

    let w = width.max(720.0);
    let h = height.max(480.0);
    let sidebar_w = if w >= 980.0 { 260.0 } else { 0.0 };
    let main_x = sidebar_w;
    let main_w = w - sidebar_w;
    let top_h = 54.0;
    let composer_h = 154.0;
    let transcript_y = top_h;
    let transcript_h = h - top_h - composer_h;

    rect(scene, 0.0, 0.0, w, h, 0.0, BG);
    if sidebar_w > 0.0 {
        draw_sidebar(scene, atlas, state, sidebar_w, h);
    }
    draw_header(scene, atlas, state, main_x, main_w, top_h);
    draw_transcript(
        scene,
        atlas,
        state,
        main_x,
        transcript_y,
        main_w,
        transcript_h,
    );
    draw_composer(
        scene,
        atlas,
        state,
        main_x,
        h - composer_h,
        main_w,
        composer_h,
    );
}

fn draw_sidebar(scene: &mut GpuScene, atlas: &FontAtlas, state: &CodexUi, width: f32, height: f32) {
    rect(scene, 0.0, 0.0, width, height, 0.0, SIDEBAR);
    rect(scene, width - 1.0, 0.0, 1.0, height, 0.0, BORDER);
    label(scene, 22.0, 18.0, "edgerun codex", TEXT);
    small_label(scene, 22.0, 48.0, "Native SDL client", MUTED);
    button(
        scene,
        ButtonSpec::new(18.0, 86.0, 106.0, 30.0, "New", NEW_CHAT_ID, ACCENT_DIM),
    );
    button(
        scene,
        ButtonSpec::new(132.0, 86.0, 88.0, 30.0, "Clear", CLEAR_ID, PANEL_2),
    );
    small_label(scene, 22.0, 140.0, "Session", MUTED);
    side_row(scene, 18.0, 168.0, "Current workspace", true);
    side_row(scene, 18.0, 206.0, "Tools enabled", false);
    side_row(scene, 18.0, 244.0, "Proof trail pending", false);
    small_label(
        scene,
        22.0,
        height - 78.0,
        &format!("model {}", state.model),
        MUTED,
    );
    small_label(
        scene,
        22.0,
        height - 50.0,
        "shell, process, apply_patch",
        FAINT,
    );
}

fn draw_header(scene: &mut GpuScene, atlas: &FontAtlas, state: &CodexUi, x: f32, w: f32, height: f32) {
    rect(scene, x, 0.0, w, height, 0.0, PANEL);
    rect(scene, x, height - 1.0, w, 1.0, 0.0, BORDER);
    label(scene, atlas, x + 24.0, 18.0, "Codex", TEXT);
    small_label(scene, atlas, x + 84.0, 21.0, &state.status, MUTED);
    pill(scene, atlas, x + w - 252.0, 14.0, 92.0, &state.model);
    pill(
        scene,
        atlas,
        x + w - 150.0,
        14.0,
        126.0,
        if state.busy { "working" } else { "ready" },
    );
}

fn draw_transcript(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    state: &CodexUi,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    scene.push_clip(edgerun_ui_core::gpu::GpuClip::new(x, y, w, h));
    let blocks = message_blocks(atlas, state, w - 48.0);
    let total_h = blocks.iter().map(|block| block.height + 14.0).sum::<f32>();
    let overflow = (total_h - h + 36.0).max(0.0);
    let mut cursor_y = y + 22.0 - overflow * state.scroll;
    for block in blocks {
        draw_message(scene, atlas, x + 24.0, cursor_y, w - 48.0, &block);
        cursor_y += block.height + 14.0;
    }
    scene.pop_clip();
}

fn draw_composer(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    state: &CodexUi,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    rect(scene, x, y, w, h, 0.0, PANEL);
    rect(scene, x, y, w, 1.0, 0.0, BORDER);
    let input_x = x + 24.0;
    let input_y = y + 18.0;
    let input_w = w - 48.0;
    let input_h = h - 66.0;
    rect(scene, input_x, input_y, input_w, input_h, 8.0, INPUT);
    scene.push_rect(GpuRect::border(
        input_x, input_y, input_w, input_h, 8.0, BORDER,
    ));
    let input = if state.input.is_empty() {
        "Ask Codex to inspect, edit, run commands, or patch files..."
    } else {
        state.input.as_str()
    };
    let color = if state.input.is_empty() { MUTED } else { TEXT };
    scene.push_clip(edgerun_ui_core::gpu::GpuClip::new(
        input_x + 12.0,
        input_y + 10.0,
        input_w - 24.0,
        input_h - 20.0,
    ));
    let composer_text = if state.input.is_empty() {
        input.to_string()
    } else {
        input_with_cursor(&state.input, state.input_cursor)
    };
    let lines = wrap_lines(atlas, &composer_text, input_w - 32.0);
    let visible_count = ((input_h - 18.0) / 22.0).floor().max(1.0) as usize;
    let start = lines.len().saturating_sub(visible_count);
    let mut text_y = input_y + 18.0;
    for line in lines.iter().skip(start) {
        label(scene, atlas, input_x + 16.0, text_y, line, color);
        text_y += 22.0;
    }
    scene.pop_clip();
    button(
        scene,
        atlas,
        ButtonSpec::new(
            x + w - 102.0,
            y + h - 40.0,
            78.0,
            28.0,
            "Send",
            SEND_ID,
            if state.busy { ACCENT_DIM } else { ACCENT },
        ),
    );
    small_label(
        scene,
        atlas,
        x + 24.0,
        y + h - 32.0,
        "Enter sends. Wheel or arrow keys scroll.",
        FAINT,
    );
    small_label(
        scene,
        atlas,
        x + 250.0,
        y + h - 32.0,
        "Shift+Enter inserts a newline. Left/right move the cursor.",
        FAINT,
    );
}

#[derive(Debug)]
struct MessageBlock {
    role: Role,
    lines: Vec<String>,
    height: f32,
}

fn message_blocks(atlas: &FontAtlas, state: &CodexUi, width: f32) -> Vec<MessageBlock> {
    state
        .messages
        .iter()
        .map(|message| {
            let lines = wrap_lines(&message.text, width - 32.0);
            MessageBlock {
                role: message.role,
                height: 36.0 + lines.len() as f32 * 22.0,
                lines,
            }
        })
        .collect()
}

fn draw_message(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    x: f32,
    y: f32,
    w: f32,
    block: &MessageBlock,
) {
    let (role, fill) = match block.role {
        Role::User => ("user", USER),
        Role::Assistant => ("assistant", PANEL_2),
        Role::ToolRunning => ("tool running", TOOL),
        Role::ToolSuccess => ("tool ok", Color4::rgba(0.06, 0.16, 0.12, 1.0)),
        Role::ToolError => ("tool failed", ERROR),
        Role::Error => ("error", ERROR),
    };
    rect(scene, x, y, w, block.height, 8.0, fill);
    scene.push_rect(GpuRect::border(x, y, w, block.height, 8.0, BORDER));
    small_label(scene, atlas, x + 16.0, y + 12.0, role, MUTED);
    let mut text_y = y + 36.0;
    for line in &block.lines {
        label(scene, atlas, x + 16.0, text_y, line, TEXT);
        text_y += 22.0;
    }
}

fn side_row(scene: &mut GpuScene, atlas: &FontAtlas, x: f32, y: f32, text: &str, active: bool) {
    rect(
        scene,
        x,
        y,
        222.0,
        30.0,
        7.0,
        if active {
            PANEL_2
        } else {
            Color4::rgba(0.0, 0.0, 0.0, 0.0)
        },
    );
    small_label(
        scene,
        atlas,
        x + 12.0,
        y + 9.0,
        text,
        if active { TEXT } else { MUTED },
    );
}

#[derive(Clone, Copy)]
struct ButtonSpec<'a> {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    text: &'a str,
    id: u32,
    fill: Color4,
}

impl<'a> ButtonSpec<'a> {
    const fn new(x: f32, y: f32, w: f32, h: f32, text: &'a str, id: u32, fill: Color4) -> Self {
        Self {
            x,
            y,
            w,
            h,
            text,
            id,
            fill,
        }
    }
}

fn button(scene: &mut GpuScene, atlas: &FontAtlas, spec: ButtonSpec<'_>) {
    rect(scene, spec.x, spec.y, spec.w, spec.h, 7.0, spec.fill);
    scene.push_rect(GpuRect::border(spec.x, spec.y, spec.w, spec.h, 7.0, BORDER));
    scene.push_hit(GpuHit::new(
        HitKind::Button,
        spec.id,
        spec.x,
        spec.y,
        spec.w,
        spec.h,
    ));
    small_label(scene, atlas, spec.x + 16.0, spec.y + 8.0, spec.text, TEXT);
}

fn pill(scene: &mut GpuScene, atlas: &FontAtlas, x: f32, y: f32, w: f32, text: &str) {
    rect(scene, x, y, w, 26.0, 13.0, PANEL_2);
    scene.push_rect(GpuRect::border(x, y, w, 26.0, 13.0, BORDER));
    small_label(scene, atlas, x + 12.0, y + 8.0, text, MUTED);
}

fn wrap_lines(atlas: &FontAtlas, text: &str, max_w: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in text.lines() {
        let mut current = String::new();
        for word in raw_line.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if atlas.text_width(&candidate) <= max_w || current.is_empty() {
                current = candidate;
            } else {
                lines.push(current);
                current = word.to_string();
            }
        }
        if current.is_empty() {
            lines.push(String::new());
        } else {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn input_with_cursor(input: &str, cursor: usize) -> String {
    let byte_index = input
        .char_indices()
        .nth(cursor)
        .map(|(index, _)| index)
        .unwrap_or(input.len());
    let mut text = String::with_capacity(input.len() + 1);
    text.push_str(&input[..byte_index]);
    text.push('|');
    text.push_str(&input[byte_index..]);
    text
}

fn label(scene: &mut GpuScene, atlas: &FontAtlas, x: f32, y: f32, text: &str, color: Color4) {
    scene.push_font_text(atlas, x, y, text, color);
}

fn small_label(scene: &mut GpuScene, atlas: &FontAtlas, x: f32, y: f32, text: &str, color: Color4) {
    scene.push_font_text(atlas, x, y, text, color);
}

fn rect(scene: &mut GpuScene, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
    scene.push_rect(GpuRect::fill(x, y, w, h, radius, color));
}

fn sdl_key(sym: i32) -> UiKey {
    match sym {
        8 => UiKey::Backspace,
        9 => UiKey::Tab,
        13 => UiKey::Enter,
        27 => UiKey::Escape,
        1073741904 => UiKey::ArrowLeft,
        1073741903 => UiKey::ArrowRight,
        1073741906 => UiKey::ArrowUp,
        1073741905 => UiKey::ArrowDown,
        other => UiKey::Other(other as u32),
    }
}

struct Sdl;

impl Sdl {
    fn init() -> Result<Self, String> {
        let rc = unsafe { SDL_Init(SDL_INIT_VIDEO) };
        if rc == 0 { Ok(Self) } else { Err(sdl_error()) }
    }
}

impl Drop for Sdl {
    fn drop(&mut self) {
        unsafe {
            SDL_StopTextInput();
            SDL_Quit();
        }
    }
}

struct Window(*mut SDL_Window);

impl Drop for Window {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                SDL_DestroyWindow(self.0);
            }
        }
    }
}

struct GlContext(SdlGlContext);

impl Drop for GlContext {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                SDL_GL_DeleteContext(self.0);
            }
        }
    }
}

fn sdl_error() -> String {
    let ptr = unsafe { SDL_GetError() };
    if ptr.is_null() {
        "unknown SDL error".to_string()
    } else {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}
