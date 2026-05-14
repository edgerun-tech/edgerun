use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use edgerun_ui_core::gpu::gl::GlRenderer;
use edgerun_ui_core::gpu::{
    Color4, FontAtlas, GpuScene, UiAction, UiEvent, UiIcon, UiKey, UiKeyModifiers, UiPainter,
    UiRect, UiRuntimeState, UiShadcnActivity, UiShadcnButtonVariant, UiShadcnChatClientAction,
    UiShadcnChatClientSpec, UiShadcnChatRole, UiShadcnConversationMessage, UiShadcnSessionRow,
    UiShadcnStatusTone, UiTextBuffer, UiTextBufferAction, shadcn_chat_client,
    shadcn_chat_message_height,
};

use super::{AgentEvent, codex_home, provider, read_chatgpt_auth, run_agent_loop, user_item};
use codex_core::TokenUsage;

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
const KMOD_CTRL: u16 = 0x00c0;

const SEND_ID: u32 = 81_000;
const NEW_CHAT_ID: u32 = 81_001;
const CLEAR_ID: u32 = 81_002;
const TRANSCRIPT_SCROLL_ID: u32 = 81_003;
const MAX_PROMPT_HISTORY: usize = 100;
const MAX_DIFF_DISPLAY_BYTES: usize = 12 * 1024;

const BG: Color4 = Color4::rgba(0.035, 0.039, 0.047, 1.0);

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
    Reasoning,
    Diff,
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
    input: UiTextBuffer,
    status: String,
    busy: bool,
    animation_tick: u32,
    runtime: UiRuntimeState,
    turns: usize,
    tools_run: usize,
    failures: usize,
    usage: UsageState,
    active_tool: Option<String>,
    prompt_history: Vec<String>,
    prompt_history_index: Option<usize>,
    prompt_history_path: Option<PathBuf>,
    messages: Vec<Message>,
    assistant_streaming: bool,
    reasoning_streaming: bool,
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
    AssistantTextDelta(String),
    ReasoningDelta(String),
    ToolDiff(String),
    ToolInputDelta(String),
    ToolStarted(String),
    ToolCompleted {
        name: String,
        summary: String,
        success: bool,
    },
    Error(String),
    Done,
    Usage(TokenUsage),
}

#[derive(Clone, Debug, Default)]
struct UsageState {
    total: TokenUsage,
    last: Option<TokenUsage>,
}

impl UsageState {
    fn record(&mut self, usage: TokenUsage) {
        self.total.add_assign(&usage);
        self.last = Some(usage);
    }

    fn total_label(&self) -> String {
        if self.total.total_tokens <= 0 {
            "usage pending".to_string()
        } else {
            format!("{} tokens", compact_i64(self.total.blended_total()))
        }
    }

    fn detail_label(&self) -> String {
        self.last
            .as_ref()
            .map(|usage| {
                format!(
                    "last {} / reasoning {}",
                    compact_i64(usage.blended_total()),
                    compact_i64(usage.reasoning_output_tokens)
                )
            })
            .unwrap_or_else(|| "awaiting usage".to_string())
    }
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

    let mut runtime = UiRuntimeState::default();
    runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 1.0);
    let prompt_history_path = prompt_history_path().ok();
    let prompt_history = prompt_history_path
        .as_deref()
        .map(load_prompt_history)
        .unwrap_or_default();

    let mut state = CodexUi {
        model: options.model,
        input: UiTextBuffer::new(),
        status: "Starting Codex worker".to_string(),
        busy: true,
        animation_tick: 0,
        runtime,
        turns: 0,
        tools_run: 0,
        failures: 0,
        usage: UsageState::default(),
        active_tool: None,
        prompt_history,
        prompt_history_index: None,
        prompt_history_path,
        messages: vec![Message {
            role: Role::Assistant,
            text: "EdgeRun Codex UI is ready for local workspace work.".to_string(),
        }],
        assistant_streaming: false,
        reasoning_streaming: false,
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
                        AgentEvent::AssistantTextDelta(text) => {
                            let _ = tx.send(WorkerEvent::AssistantTextDelta(text));
                        }
                        AgentEvent::ReasoningDelta(text) => {
                            let _ = tx.send(WorkerEvent::ReasoningDelta(text));
                        }
                        AgentEvent::ToolDiff(diff) => {
                            let _ = tx.send(WorkerEvent::ToolDiff(diff));
                        }
                        AgentEvent::ToolInputDelta(delta) => {
                            let _ = tx.send(WorkerEvent::ToolInputDelta(delta));
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
                        AgentEvent::Usage(usage) => {
                            let _ = tx.send(WorkerEvent::Usage(usage));
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
        if state.drain_events() {
            scene_dirty = true;
        }
        let mut event = SdlEvent { data: [0; 56] };
        while unsafe { SDL_PollEvent(&mut event) } != 0 {
            match event.event_type() {
                SDL_QUIT => running = false,
                SDL_KEYDOWN if event.key_sym() == SDLK_ESCAPE => running = false,
                SDL_KEYDOWN => {
                    let key = sdl_key(event.key_sym());
                    let modifiers = UiKeyModifiers {
                        shift: event.key_mod() & KMOD_SHIFT != 0,
                        ctrl: event.key_mod() & KMOD_CTRL != 0,
                        ..UiKeyModifiers::default()
                    };
                    if !state.handle_history_key(key, modifiers) {
                        let action = state.input.handle_key_with_modifiers(key, modifiers);
                        state.handle_text_action(action);
                        state.handle_navigation_key(key);
                    }
                    scene_dirty = true;
                }
                SDL_TEXTINPUT => {
                    if let Some(text) = event.text_input() {
                        let action = state.input.handle_text_input(&text);
                        state.handle_text_action(action);
                        scene_dirty = true;
                    }
                }
                SDL_MOUSEBUTTONDOWN => {
                    let action = state.runtime.handle_event(
                        &scene,
                        UiEvent::PointerDown {
                            x: event.mouse_x(),
                            y: event.mouse_y(),
                        },
                    );
                    state.handle_ui_action(action);
                    scene_dirty = true;
                }
                SDL_MOUSEBUTTONUP => {
                    let action = state.runtime.handle_event(
                        &scene,
                        UiEvent::PointerUp {
                            x: event.mouse_x(),
                            y: event.mouse_y(),
                        },
                    );
                    state.handle_ui_action(action);
                    scene_dirty = true;
                }
                SDL_MOUSEWHEEL => {
                    let action = state.runtime.handle_event(
                        &scene,
                        UiEvent::Wheel {
                            x: 0.0,
                            y: 0.0,
                            delta_y: -event.wheel_y() * 72.0,
                        },
                    );
                    if matches!(action, UiAction::None) {
                        state.scroll_transcript_by(-event.wheel_y() * 0.08);
                    } else {
                        state.handle_ui_action(action);
                    }
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
            if state.busy {
                state.animation_tick = state.animation_tick.wrapping_add(1);
            }
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
    fn drain_events(&mut self) -> bool {
        let mut changed = false;
        while let Ok(event) = self.event_rx.try_recv() {
            changed = true;
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
                    self.assistant_streaming = false;
                    self.reasoning_streaming = false;
                    self.scroll_transcript_to_bottom();
                }
                WorkerEvent::AssistantTextDelta(text) => {
                    self.append_streaming_message(Role::Assistant, text);
                    self.status = "Responding".to_string();
                }
                WorkerEvent::ReasoningDelta(text) => {
                    self.append_streaming_message(Role::Reasoning, text);
                    self.status = "Reasoning".to_string();
                    self.scroll_transcript_to_bottom();
                }
                WorkerEvent::ToolDiff(diff) => {
                    self.assistant_streaming = false;
                    self.reasoning_streaming = false;
                    self.append_diff_message(diff);
                    self.status = "Reviewing patch".to_string();
                }
                WorkerEvent::ToolInputDelta(delta) => {
                    self.assistant_streaming = false;
                    self.reasoning_streaming = false;
                    self.append_diff_message(delta);
                    self.status = "Drafting patch".to_string();
                }
                WorkerEvent::ToolStarted(name) => {
                    self.assistant_streaming = false;
                    self.reasoning_streaming = false;
                    self.active_tool = Some(name.clone());
                    self.messages.push(Message {
                        role: Role::ToolRunning,
                        text: format!("Running {name}"),
                    });
                    self.status = "Tool running".to_string();
                    self.scroll_transcript_to_bottom();
                }
                WorkerEvent::ToolCompleted {
                    name,
                    summary,
                    success,
                } => {
                    self.assistant_streaming = false;
                    self.reasoning_streaming = false;
                    self.tools_run += 1;
                    if !success {
                        self.failures += 1;
                    }
                    self.active_tool = None;
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
                    self.scroll_transcript_to_bottom();
                }
                WorkerEvent::Error(error) => {
                    self.busy = false;
                    self.failures += 1;
                    self.active_tool = None;
                    self.assistant_streaming = false;
                    self.reasoning_streaming = false;
                    self.status = "Error".to_string();
                    self.messages.push(Message {
                        role: Role::Error,
                        text: normalize_error_message(&error),
                    });
                    self.scroll_transcript_to_bottom();
                }
                WorkerEvent::Done => {
                    self.busy = false;
                    self.active_tool = None;
                    self.assistant_streaming = false;
                    self.reasoning_streaming = false;
                    self.status = "Ready".to_string();
                }
                WorkerEvent::Usage(usage) => {
                    self.usage.record(usage);
                }
            }
        }
        changed
    }

    fn handle_text_action(&mut self, action: UiTextBufferAction) {
        match action {
            UiTextBufferAction::Submit => self.submit(),
            UiTextBufferAction::Changed | UiTextBufferAction::None => {}
        }
    }

    fn handle_history_key(&mut self, key: UiKey, modifiers: UiKeyModifiers) -> bool {
        if !modifiers.ctrl {
            return false;
        }
        match key {
            UiKey::Other(code) if code == b'p' as u32 || code == b'P' as u32 => {
                self.previous_prompt();
                true
            }
            UiKey::Other(code) if code == b'n' as u32 || code == b'N' as u32 => {
                self.next_prompt();
                true
            }
            _ => false,
        }
    }

    fn previous_prompt(&mut self) {
        if self.prompt_history.is_empty() {
            self.status = "No prompt history".to_string();
            return;
        }
        let next = self
            .prompt_history_index
            .map(|index| index.saturating_sub(1))
            .unwrap_or_else(|| self.prompt_history.len() - 1);
        self.prompt_history_index = Some(next);
        self.input.set_text(self.prompt_history[next].clone());
        self.status = "Prompt history".to_string();
    }

    fn next_prompt(&mut self) {
        let Some(index) = self.prompt_history_index else {
            return;
        };
        if index + 1 < self.prompt_history.len() {
            let next = index + 1;
            self.prompt_history_index = Some(next);
            self.input.set_text(self.prompt_history[next].clone());
        } else {
            self.prompt_history_index = None;
            self.input.clear();
        }
        self.status = "Prompt history".to_string();
    }

    fn handle_navigation_key(&mut self, key: UiKey) {
        match key {
            UiKey::ArrowUp => self.scroll_transcript_by(-0.06),
            UiKey::ArrowDown => self.scroll_transcript_by(0.06),
            UiKey::PageUp => self.scroll_transcript_by(-0.42),
            UiKey::PageDown => self.scroll_transcript_by(0.42),
            _ => {}
        }
    }

    fn handle_ui_action(&mut self, action: UiAction) {
        match action {
            UiAction::Activated(hit) if hit.id == SEND_ID => self.submit(),
            UiAction::Submitted { id } if id == 0 => self.submit(),
            UiAction::Activated(hit) if hit.id == NEW_CHAT_ID => self.new_chat(),
            UiAction::Activated(hit) if hit.id == CLEAR_ID => {
                self.clear_transcript();
            }
            UiAction::ScrollChanged { id, offset } if id == TRANSCRIPT_SCROLL_ID => {
                self.runtime.set_scroll_offset(id, offset);
            }
            _ => {}
        }
    }

    fn scroll_transcript_to_bottom(&mut self) {
        self.runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 1.0);
    }

    fn scroll_transcript_by(&mut self, delta: f32) {
        let next = self.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID) + delta;
        self.runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, next);
    }

    fn append_streaming_message(&mut self, role: Role, text: String) {
        if text.is_empty() {
            return;
        }
        let streaming = match role {
            Role::Assistant => &mut self.assistant_streaming,
            Role::Reasoning => &mut self.reasoning_streaming,
            _ => return,
        };
        if *streaming
            && let Some(message) = self.messages.last_mut()
            && message.role == role
        {
            message.text.push_str(&text);
        } else {
            self.messages.push(Message { role, text });
            *streaming = true;
        }
        match role {
            Role::Assistant => self.reasoning_streaming = false,
            Role::Reasoning => self.assistant_streaming = false,
            _ => {}
        }
        self.scroll_transcript_to_bottom();
    }

    fn append_diff_message(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        if let Some(message) = self.messages.last_mut()
            && message.role == Role::Diff
        {
            append_capped(&mut message.text, &text, MAX_DIFF_DISPLAY_BYTES);
        } else {
            let mut text = text;
            if text.len() > MAX_DIFF_DISPLAY_BYTES {
                text.truncate(char_boundary_at_or_before(&text, MAX_DIFF_DISPLAY_BYTES));
                text.push_str("\n[diff preview truncated]");
            }
            self.messages.push(Message {
                role: Role::Diff,
                text,
            });
        }
        self.scroll_transcript_to_bottom();
    }

    fn submit(&mut self) {
        if self.busy {
            self.status = "Codex is still working".to_string();
            return;
        }
        let prompt = self.input.as_str().trim().to_string();
        if prompt.is_empty() {
            self.status = "Type a prompt before sending".to_string();
            return;
        }
        self.messages.push(Message {
            role: Role::User,
            text: prompt.clone(),
        });
        self.remember_prompt(prompt.clone());
        self.turns += 1;
        self.input.clear();
        self.busy = true;
        self.assistant_streaming = false;
        self.reasoning_streaming = false;
        self.status = "Queued".to_string();
        self.scroll_transcript_to_bottom();
        if self.command_tx.send(WorkerCommand::Prompt(prompt)).is_err() {
            self.busy = false;
            self.status = "Worker disconnected".to_string();
        }
    }

    fn new_chat(&mut self) {
        if !self.ensure_idle("Wait for Codex to finish before starting a new chat") {
            return;
        }
        self.reset_visible_session("New chat started.");
        self.prompt_history_index = None;
        let _ = self.command_tx.send(WorkerCommand::Reset);
    }

    fn clear_transcript(&mut self) {
        if !self.ensure_idle("Wait for Codex to finish before clearing the transcript") {
            return;
        }
        self.reset_visible_session("Transcript cleared.");
        self.status = "Transcript cleared".to_string();
    }

    fn ensure_idle(&mut self, message: &str) -> bool {
        if self.busy {
            self.status = message.to_string();
            false
        } else {
            true
        }
    }

    fn reset_visible_session(&mut self, message: &str) {
        self.messages.clear();
        self.messages.push(Message {
            role: Role::Assistant,
            text: message.to_string(),
        });
        self.turns = 0;
        self.tools_run = 0;
        self.failures = 0;
        self.usage = UsageState::default();
        self.active_tool = None;
        self.assistant_streaming = false;
        self.reasoning_streaming = false;
        self.scroll_transcript_to_bottom();
    }

    fn remember_prompt(&mut self, prompt: String) {
        if let Some(index) = self
            .prompt_history
            .iter()
            .position(|saved| saved == &prompt)
        {
            self.prompt_history.remove(index);
        }
        self.prompt_history.push(prompt);
        trim_prompt_history(&mut self.prompt_history);
        if let Some(path) = self.prompt_history_path.as_deref() {
            let _ = save_prompt_history(path, &self.prompt_history);
        }
        self.prompt_history_index = None;
    }
}

fn build_scene(scene: &mut GpuScene, atlas: &FontAtlas, state: &CodexUi, width: f32, height: f32) {
    scene.clear = BG;
    scene.clear_rects();

    let w = width.max(720.0);
    let h = height.max(480.0);
    let turns = format!("{} turns", state.turns);
    let tools = format!("{} tools", state.tools_run);
    let issues = format!("{} issues", state.failures);
    let usage = state.usage.total_label();
    let usage_detail = state.usage.detail_label();
    let model = format!("model {}", state.model);
    let workspace = workspace_label();
    let availability = if state.busy { "working" } else { "ready" };
    let tone = header_tone(state);
    let (activity_title, activity_detail, activity_icon) = sidebar_activity(state);
    let main_width = if w >= 980.0 { w - 260.0 } else { w };
    let messages = conversation_messages(atlas, state, main_width - 48.0);
    let composer_text = state.input.display_value(
        "Ask Codex to inspect, edit, run commands, or patch files...",
        '|',
    );

    let actions = [
        UiShadcnChatClientAction::new("New", NEW_CHAT_ID, UiShadcnButtonVariant::Default),
        UiShadcnChatClientAction::new("Clear", CLEAR_ID, UiShadcnButtonVariant::Secondary),
    ];
    let session_rows = [
        UiShadcnSessionRow::new(turns.as_str(), "selected", true),
        UiShadcnSessionRow::new(tools.as_str(), "", false),
        UiShadcnSessionRow::new(issues.as_str(), "", state.failures > 0),
        UiShadcnSessionRow::new(usage.as_str(), usage_detail.as_str(), false),
    ];
    let footer_lines = [model.as_str(), "shell, process, patch"];
    let header_badges = [state.model.as_str(), availability];
    let hints = [
        "Enter sends. Wheel, arrows, PgUp/PgDn scroll.",
        "Shift+Enter newline. Ctrl+P/N history. Ctrl+A/E/U/K/W edit.",
    ];

    render_node(
        scene,
        atlas,
        UiRect::new(0.0, 0.0, w, h),
        &state.runtime,
        shadcn_chat_client(UiShadcnChatClientSpec {
            show_sidebar: w >= 980.0,
            sidebar_title: "edgerun codex",
            sidebar_detail: workspace.as_str(),
            sidebar_actions: &actions,
            session_rows: &session_rows,
            activity: UiShadcnActivity::new(activity_title, activity_detail, activity_icon),
            footer_lines: &footer_lines,
            header_title: "Codex",
            header_status: &state.status,
            header_badges: &header_badges,
            header_tone: tone,
            activity_phase: (state.animation_tick / 8) as u8,
            messages: &messages,
            scroll_offset: state.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID),
            scroll_id: TRANSCRIPT_SCROLL_ID,
            input_label: "Prompt",
            input_value: &composer_text,
            input_id: 0,
            send_label: "Send",
            send_id: SEND_ID,
            busy: state.busy,
            hints: &hints,
        }),
    );
}

fn header_tone(state: &CodexUi) -> UiShadcnStatusTone {
    if state.failures > 0 {
        UiShadcnStatusTone::Error
    } else if state.busy {
        UiShadcnStatusTone::Active
    } else {
        UiShadcnStatusTone::Success
    }
}

fn conversation_messages<'a>(
    atlas: &FontAtlas,
    state: &'a CodexUi,
    width: f32,
) -> Vec<UiShadcnConversationMessage<'a>> {
    state
        .messages
        .iter()
        .map(|message| {
            UiShadcnConversationMessage::new(
                chat_role(message.role),
                &message.text,
                shadcn_chat_message_height(atlas, &message.text, width),
            )
        })
        .collect()
}

fn chat_role(role: Role) -> UiShadcnChatRole {
    match role {
        Role::User => UiShadcnChatRole::User,
        Role::Assistant => UiShadcnChatRole::Assistant,
        Role::Reasoning => UiShadcnChatRole::Reasoning,
        Role::Diff => UiShadcnChatRole::Diff,
        Role::ToolRunning => UiShadcnChatRole::ToolRunning,
        Role::ToolSuccess => UiShadcnChatRole::ToolSuccess,
        Role::ToolError => UiShadcnChatRole::ToolError,
        Role::Error => UiShadcnChatRole::Error,
    }
}

fn sidebar_activity(state: &CodexUi) -> (&str, &str, UiIcon) {
    if let Some(tool) = state.active_tool.as_deref() {
        ("Running", tool, UiIcon::Terminal)
    } else if state.busy {
        ("Thinking", state.status.as_str(), UiIcon::Code)
    } else {
        ("Ready", state.status.as_str(), UiIcon::Check)
    }
}

fn workspace_label() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "workspace".to_string())
}

fn prompt_history_path() -> Result<PathBuf, String> {
    Ok(codex_home()?.join("edgerun-codex-history.json"))
}

fn load_prompt_history(path: &Path) -> Vec<String> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let Ok(value) = edgerun_json::from_slice(&bytes) else {
        return Vec::new();
    };
    let Some(values) = value.as_array() else {
        return Vec::new();
    };
    let mut history = values
        .iter()
        .filter_map(edgerun_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    trim_prompt_history(&mut history);
    history
}

fn save_prompt_history(path: &Path, history: &[String]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let value = edgerun_json::JsonValue::Array(
        history
            .iter()
            .map(|prompt| edgerun_json::JsonValue::String(prompt.clone()))
            .collect(),
    );
    let text = edgerun_json::to_string(&value).map_err(|error| error.to_string())?;
    std::fs::write(path, text).map_err(|error| error.to_string())
}

fn trim_prompt_history(history: &mut Vec<String>) {
    if history.len() > MAX_PROMPT_HISTORY {
        let remove = history.len() - MAX_PROMPT_HISTORY;
        history.drain(..remove);
    }
}

fn compact_i64(value: i64) -> String {
    let value = value.max(0);
    if value >= 1_000_000 {
        format!("{:.1}m", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}k", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn append_capped(target: &mut String, text: &str, max_len: usize) {
    if target.len() >= max_len {
        return;
    }
    let remaining = max_len - target.len();
    if text.len() <= remaining {
        target.push_str(text);
    } else {
        let end = char_boundary_at_or_before(text, remaining);
        target.push_str(&text[..end]);
        target.push_str("\n[diff preview truncated]");
    }
}

fn char_boundary_at_or_before(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn normalize_error_message(raw: &str) -> String {
    let trimmed = raw.trim();
    let status = error_status_prefix(trimmed);
    if let Some(detail) = error_json_detail(trimmed) {
        if let Some(status) = status {
            format!("{status}\n{detail}")
        } else {
            detail
        }
    } else {
        trimmed.to_string()
    }
}

fn error_status_prefix(raw: &str) -> Option<String> {
    let first = raw.split(';').next()?.trim();
    let lower = first.to_ascii_lowercase();
    lower.starts_with("http ").then(|| {
        let status = first.get(5..).unwrap_or(first).trim();
        format!("HTTP {status}")
    })
}

fn error_json_detail(raw: &str) -> Option<String> {
    let candidates = error_json_candidates(raw);
    for candidate in candidates {
        if let Some(detail) = parse_error_detail(&candidate) {
            return Some(detail);
        }
    }
    None
}

fn error_json_candidates(raw: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    candidates.push(raw.trim().to_string());

    if let Some(inner) = raw
        .split_once("Some(")
        .and_then(|(_, rest)| rest.rsplit_once(')').map(|(inner, _)| inner.trim()))
    {
        candidates.push(inner.to_string());
    }

    if let (Some(start), Some(end)) = (raw.find('{'), raw.rfind('}'))
        && start <= end
    {
        candidates.push(raw[start..=end].to_string());
    }

    candidates
}

fn parse_error_detail(candidate: &str) -> Option<String> {
    let value = edgerun_json::from_str(candidate).ok()?;
    if let Some(text) = value.as_str() {
        return parse_error_detail(text).or_else(|| Some(text.to_string()));
    }
    ["detail", "error", "message"]
        .into_iter()
        .find_map(|key| value.get(key).and_then(error_value_text))
}

fn error_value_text(value: &edgerun_json::Value) -> Option<String> {
    value
        .as_str()
        .map(ToString::to_string)
        .or_else(|| Some(edgerun_json::to_string(value).ok()?))
}

fn render_node(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    rect: UiRect,
    runtime: &UiRuntimeState,
    node: edgerun_ui_core::gpu::UiNode,
) {
    let mut ui = UiPainter::with_font(scene, atlas);
    node.render_with_state(&mut ui, rect, Some(runtime));
}

fn sdl_key(sym: i32) -> UiKey {
    match sym {
        8 => UiKey::Backspace,
        9 => UiKey::Tab,
        13 => UiKey::Enter,
        27 => UiKey::Escape,
        127 | 1073741907 => UiKey::Delete,
        1073741898 => UiKey::Home,
        1073741901 => UiKey::End,
        1073741899 => UiKey::PageUp,
        1073741902 => UiKey::PageDown,
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

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_ui_core::gpu::HitKind;

    #[test]
    fn build_scene_renders_shadcn_chat_client_controls() {
        let (command_tx, _command_rx) = std::sync::mpsc::channel();
        let (_event_tx, event_rx) = std::sync::mpsc::channel();
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 1.0);
        let state = CodexUi {
            model: "gpt-5.5".to_string(),
            input: UiTextBuffer::new(),
            status: "Ready".to_string(),
            busy: false,
            animation_tick: 0,
            runtime,
            turns: 1,
            tools_run: 1,
            failures: 0,
            usage: UsageState::default(),
            active_tool: None,
            prompt_history: Vec::new(),
            prompt_history_index: None,
            prompt_history_path: None,
            messages: vec![
                Message {
                    role: Role::Assistant,
                    text: "Ready for local workspace work.".to_string(),
                },
                Message {
                    role: Role::User,
                    text: "Summarize the project.".to_string(),
                },
            ],
            assistant_streaming: false,
            reasoning_streaming: false,
            command_tx,
            event_rx,
        };
        let atlas = FontAtlas::load_inter(18.0).expect("load UI font");
        let mut scene = GpuScene::new(BG);

        build_scene(&mut scene, &atlas, &state, 1120.0, 720.0);

        assert!(scene.text_quads().len() > 80);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == SEND_ID),
            "scene hits: {:?}",
            scene.hits()
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == NEW_CHAT_ID)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == CLEAR_ID)
        );
    }

    #[test]
    fn streaming_deltas_append_to_current_transcript_rows() {
        let (command_tx, _command_rx) = std::sync::mpsc::channel();
        let (_event_tx, event_rx) = std::sync::mpsc::channel();
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.0);
        let mut state = CodexUi {
            model: "gpt-5.5".to_string(),
            input: UiTextBuffer::new(),
            status: "Thinking".to_string(),
            busy: true,
            animation_tick: 0,
            runtime,
            turns: 1,
            tools_run: 0,
            failures: 0,
            usage: UsageState::default(),
            active_tool: None,
            prompt_history: Vec::new(),
            prompt_history_index: None,
            prompt_history_path: None,
            messages: vec![Message {
                role: Role::User,
                text: "stream".to_string(),
            }],
            assistant_streaming: false,
            reasoning_streaming: false,
            command_tx,
            event_rx,
        };

        state.append_streaming_message(Role::Reasoning, "checking".to_string());
        state.append_streaming_message(Role::Reasoning, " files".to_string());
        state.append_streaming_message(Role::Assistant, "done".to_string());
        state.append_streaming_message(Role::Assistant, ".".to_string());

        assert_eq!(state.messages.len(), 3);
        assert_eq!(state.messages[1].role, Role::Reasoning);
        assert_eq!(state.messages[1].text, "checking files");
        assert_eq!(state.messages[2].role, Role::Assistant);
        assert_eq!(state.messages[2].text, "done.");
        assert_eq!(state.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID), 1.0);
    }

    #[test]
    fn tool_input_deltas_append_to_diff_row() {
        let (command_tx, _command_rx) = std::sync::mpsc::channel();
        let (_event_tx, event_rx) = std::sync::mpsc::channel();
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.0);
        let mut state = CodexUi {
            model: "gpt-5.5".to_string(),
            input: UiTextBuffer::new(),
            status: "Thinking".to_string(),
            busy: true,
            animation_tick: 0,
            runtime,
            turns: 1,
            tools_run: 0,
            failures: 0,
            usage: UsageState::default(),
            active_tool: None,
            prompt_history: Vec::new(),
            prompt_history_index: None,
            prompt_history_path: None,
            messages: Vec::new(),
            assistant_streaming: false,
            reasoning_streaming: false,
            command_tx,
            event_rx,
        };

        state.append_diff_message("*** Begin Patch\n".to_string());
        state.append_diff_message("+new line\n".to_string());

        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, Role::Diff);
        assert_eq!(state.messages[0].text, "*** Begin Patch\n+new line\n");
        assert_eq!(state.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID), 1.0);
    }

    #[test]
    fn normalize_error_message_extracts_escaped_detail_payloads() {
        let raw =
            r#"http 400 Bad Request; Some("{\"detail\":\"The model requires a newer Codex.\"}")"#;

        assert_eq!(
            normalize_error_message(raw),
            "HTTP 400 Bad Request\nThe model requires a newer Codex."
        );
    }

    #[test]
    fn normalize_error_message_handles_uppercase_http_prefix() {
        let raw = r#"HTTP 401 Unauthorized; {"detail":"session expired"}"#;

        assert_eq!(
            normalize_error_message(raw),
            "HTTP 401 Unauthorized\nsession expired"
        );
    }

    #[test]
    fn normalize_error_message_extracts_direct_message_payloads() {
        assert_eq!(
            normalize_error_message(r#"{"message":"auth token expired"}"#),
            "auth token expired"
        );
    }

    #[test]
    fn normalize_error_message_leaves_plain_errors_unchanged() {
        assert_eq!(
            normalize_error_message("auth: missing tokens.access_token"),
            "auth: missing tokens.access_token"
        );
    }

    #[test]
    fn prompt_history_uses_ctrl_p_and_ctrl_n() {
        let (command_tx, _command_rx) = std::sync::mpsc::channel();
        let (_event_tx, event_rx) = std::sync::mpsc::channel();
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 1.0);
        let mut state = CodexUi {
            model: "gpt-5.5".to_string(),
            input: UiTextBuffer::new(),
            status: "Ready".to_string(),
            busy: false,
            animation_tick: 0,
            runtime,
            turns: 0,
            tools_run: 0,
            failures: 0,
            usage: UsageState::default(),
            active_tool: None,
            prompt_history: vec!["first prompt".to_string(), "second prompt".to_string()],
            prompt_history_index: None,
            prompt_history_path: None,
            messages: Vec::new(),
            assistant_streaming: false,
            reasoning_streaming: false,
            command_tx,
            event_rx,
        };
        let ctrl = UiKeyModifiers {
            ctrl: true,
            ..UiKeyModifiers::default()
        };

        assert!(state.handle_history_key(UiKey::Other(b'p' as u32), ctrl));
        assert_eq!(state.input.as_str(), "second prompt");
        assert!(state.handle_history_key(UiKey::Other(b'p' as u32), ctrl));
        assert_eq!(state.input.as_str(), "first prompt");
        assert!(state.handle_history_key(UiKey::Other(b'n' as u32), ctrl));
        assert_eq!(state.input.as_str(), "second prompt");
        assert!(state.handle_history_key(UiKey::Other(b'n' as u32), ctrl));
        assert_eq!(state.input.as_str(), "");
        assert_eq!(state.prompt_history_index, None);
    }

    #[test]
    fn prompt_history_moves_repeated_prompts_to_most_recent() {
        let (command_tx, _command_rx) = std::sync::mpsc::channel();
        let (_event_tx, event_rx) = std::sync::mpsc::channel();
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 1.0);
        let mut state = CodexUi {
            model: "gpt-5.5".to_string(),
            input: UiTextBuffer::new(),
            status: "Ready".to_string(),
            busy: false,
            animation_tick: 0,
            runtime,
            turns: 0,
            tools_run: 0,
            failures: 0,
            usage: UsageState::default(),
            active_tool: None,
            prompt_history: vec!["first".to_string(), "second".to_string()],
            prompt_history_index: Some(0),
            prompt_history_path: None,
            messages: Vec::new(),
            assistant_streaming: false,
            reasoning_streaming: false,
            command_tx,
            event_rx,
        };

        state.remember_prompt("first".to_string());

        assert_eq!(state.prompt_history, vec!["second", "first"]);
        assert_eq!(state.prompt_history_index, None);
    }

    #[test]
    fn clear_transcript_resets_visible_metrics_without_dropping_history() {
        let (command_tx, _command_rx) = std::sync::mpsc::channel();
        let (_event_tx, event_rx) = std::sync::mpsc::channel();
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.0);
        let mut state = CodexUi {
            model: "gpt-5.5".to_string(),
            input: UiTextBuffer::new(),
            status: "Error".to_string(),
            busy: false,
            animation_tick: 0,
            runtime,
            turns: 4,
            tools_run: 3,
            failures: 2,
            usage: UsageState::default(),
            active_tool: Some("shell".to_string()),
            prompt_history: vec!["keep this".to_string()],
            prompt_history_index: Some(0),
            prompt_history_path: None,
            messages: vec![Message {
                role: Role::Error,
                text: "broken".to_string(),
            }],
            assistant_streaming: false,
            reasoning_streaming: false,
            command_tx,
            event_rx,
        };

        state.clear_transcript();

        assert_eq!(state.turns, 0);
        assert_eq!(state.tools_run, 0);
        assert_eq!(state.failures, 0);
        assert_eq!(state.active_tool, None);
        assert_eq!(state.prompt_history, vec!["keep this"]);
        assert_eq!(state.prompt_history_index, Some(0));
        assert_eq!(state.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID), 1.0);
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, Role::Assistant);
    }

    #[test]
    fn clear_and_new_chat_are_guarded_while_busy() {
        let (command_tx, _command_rx) = std::sync::mpsc::channel();
        let (_event_tx, event_rx) = std::sync::mpsc::channel();
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.25);
        let mut state = CodexUi {
            model: "gpt-5.5".to_string(),
            input: UiTextBuffer::new(),
            status: "Tool running".to_string(),
            busy: true,
            animation_tick: 0,
            runtime,
            turns: 2,
            tools_run: 1,
            failures: 0,
            usage: UsageState::default(),
            active_tool: Some("shell".to_string()),
            prompt_history: vec!["keep this".to_string()],
            prompt_history_index: Some(0),
            prompt_history_path: None,
            messages: vec![Message {
                role: Role::ToolRunning,
                text: "Running shell".to_string(),
            }],
            assistant_streaming: false,
            reasoning_streaming: false,
            command_tx,
            event_rx,
        };

        state.clear_transcript();

        assert_eq!(
            state.status,
            "Wait for Codex to finish before clearing the transcript"
        );
        assert_eq!(state.turns, 2);
        assert_eq!(state.tools_run, 1);
        assert_eq!(state.active_tool.as_deref(), Some("shell"));
        assert_eq!(state.messages.len(), 1);

        state.new_chat();

        assert_eq!(
            state.status,
            "Wait for Codex to finish before starting a new chat"
        );
        assert_eq!(state.turns, 2);
        assert_eq!(state.prompt_history, vec!["keep this"]);
        assert_eq!(state.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID), 0.25);
    }

    #[test]
    fn prompt_history_loads_saves_and_caps_recent_entries() {
        let dir =
            std::env::temp_dir().join(format!("edgerun-codex-history-test-{}", std::process::id()));
        let path = dir.join("history.json");
        let mut history = (0..(MAX_PROMPT_HISTORY + 3))
            .map(|index| format!("prompt {index}"))
            .collect::<Vec<_>>();
        history.push(String::new());

        save_prompt_history(&path, &history).expect("save prompt history");
        let loaded = load_prompt_history(&path);

        assert_eq!(loaded.len(), MAX_PROMPT_HISTORY);
        assert_eq!(loaded.first().map(String::as_str), Some("prompt 3"));
        let expected_last = format!("prompt {}", MAX_PROMPT_HISTORY + 2);
        assert_eq!(
            loaded.last().map(String::as_str),
            Some(expected_last.as_str())
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
