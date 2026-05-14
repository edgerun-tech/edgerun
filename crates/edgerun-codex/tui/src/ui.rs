use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;

use edgerun_ui_core::gpu::gl::GlRenderer;
use edgerun_ui_core::gpu::{
    Color4, FontAtlas, GpuScene, UiAction, UiEvent, UiIcon, UiKey, UiKeyModifiers, UiNode,
    UiPainter, UiRect, UiRuntimeState, UiShadcnActivity, UiShadcnBadgeVariant,
    UiShadcnButtonVariant, UiShadcnChatClientAction, UiShadcnChatClientIconAction,
    UiShadcnChatClientSpec, UiShadcnChatRole, UiShadcnComposerNotice, UiShadcnConversationMessage,
    UiShadcnSessionRow, UiShadcnSessionState, UiShadcnStatusTone, UiTextBuffer, UiTextBufferAction,
    column, icon_button, row, shadcn_alert, shadcn_badge, shadcn_card, shadcn_chat_client,
    shadcn_chat_message_height_for_role, shadcn_label, shadcn_progress, shadcn_separator,
    shadcn_switch, shadcn_toggle_group, spacer,
};

use super::{AgentEvent, codex_home, provider, read_chatgpt_auth, run_agent_loop, user_item};
use codex_core::TokenUsage;
use codex_core::protocol::protocol::RateLimitSnapshot;

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
const SDL_WINDOWEVENT_SIZE_CHANGED: u8 = 0x06;
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
const STOP_ID: u32 = 81_004;
const SETTINGS_ID: u32 = 81_005;
const INSPECTOR_ID: u32 = 81_006;
const PANEL_TAB_BASE_ID: u32 = 81_020;
const LOG_LEVEL_BASE_ID: u32 = 81_030;
const CONFIRM_CLEAR_ID: u32 = 81_040;
const DENSITY_BASE_ID: u32 = 81_050;
const SESSION_ROW_BASE_ID: u32 = 88_000;
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
    fn SDL_SetWindowMinimumSize(window: *mut SDL_Window, min_w: c_int, min_h: c_int);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InspectorPanel {
    Usage,
    Settings,
    Logs,
}

impl InspectorPanel {
    const LABELS: [&'static str; 3] = ["Usage", "Settings", "Logs"];

    fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Settings,
            2 => Self::Logs,
            _ => Self::Usage,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Usage => 0,
            Self::Settings => 1,
            Self::Logs => 2,
        }
    }

    fn label(self) -> &'static str {
        Self::LABELS[self.index()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum LogLevel {
    Quiet,
    Normal,
    Debug,
}

impl LogLevel {
    const LABELS: [&'static str; 3] = ["Quiet", "Normal", "Debug"];

    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Quiet,
            2 => Self::Debug,
            _ => Self::Normal,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Quiet => 0,
            Self::Normal => 1,
            Self::Debug => 2,
        }
    }

    fn label(self) -> &'static str {
        Self::LABELS[self.index()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DensityMode {
    Comfortable,
    Compact,
}

impl DensityMode {
    const LABELS: [&'static str; 2] = ["Comfort", "Compact"];

    fn from_index(index: usize) -> Self {
        if index == 1 {
            Self::Compact
        } else {
            Self::Comfortable
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Comfortable => 0,
            Self::Compact => 1,
        }
    }

    fn label(self) -> &'static str {
        Self::LABELS[self.index()]
    }
}

#[derive(Clone, Debug)]
struct RuntimeLog {
    level: LogLevel,
    text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct UiSessionId(u64);

#[derive(Clone, Debug)]
struct UiSession {
    id: UiSessionId,
    title: String,
    status: String,
    busy: bool,
    turns: usize,
    tools_run: usize,
    failures: usize,
    usage: UsageState,
    rate_limits: RateLimitState,
    active_tool: Option<String>,
    messages: Vec<Message>,
    assistant_streaming: bool,
    reasoning_streaming: bool,
}

impl UiSession {
    fn new(id: UiSessionId, title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            status: "Ready".to_string(),
            busy: false,
            turns: 0,
            tools_run: 0,
            failures: 0,
            usage: UsageState::default(),
            rate_limits: RateLimitState::default(),
            active_tool: None,
            messages: vec![Message {
                role: Role::Assistant,
                text: message.into(),
            }],
            assistant_streaming: false,
            reasoning_streaming: false,
        }
    }

    fn reset(&mut self, message: &str) {
        self.messages.clear();
        self.messages.push(Message {
            role: Role::Assistant,
            text: message.to_string(),
        });
        self.turns = 0;
        self.tools_run = 0;
        self.failures = 0;
        self.usage = UsageState::default();
        self.rate_limits = RateLimitState::default();
        self.active_tool = None;
        self.status = "Ready".to_string();
        self.busy = false;
        self.assistant_streaming = false;
        self.reasoning_streaming = false;
    }
}

#[derive(Debug)]
struct SessionStore {
    sessions: Vec<UiSession>,
    selected: UiSessionId,
    next_id: u64,
}

impl SessionStore {
    fn initial() -> Self {
        let selected = UiSessionId(1);
        Self {
            sessions: vec![UiSession::new(
                selected,
                "Current workspace",
                "EdgeRun Codex UI is ready for local workspace work.",
            )],
            selected,
            next_id: 2,
        }
    }

    fn selected(&self) -> &UiSession {
        self.sessions
            .iter()
            .find(|session| session.id == self.selected)
            .expect("selected session must exist")
    }

    fn selected_mut(&mut self) -> &mut UiSession {
        self.sessions
            .iter_mut()
            .find(|session| session.id == self.selected)
            .expect("selected session must exist")
    }

    fn session_mut(&mut self, id: UiSessionId) -> Option<&mut UiSession> {
        self.sessions.iter_mut().find(|session| session.id == id)
    }

    fn any_busy(&self) -> bool {
        self.sessions.iter().any(|session| session.busy)
    }

    fn active_count(&self) -> usize {
        self.sessions.iter().filter(|session| session.busy).count()
    }

    fn aggregate_usage(&self) -> TokenUsage {
        let mut total = TokenUsage::default();
        for session in &self.sessions {
            total.add_assign(&session.usage.total);
        }
        total
    }

    fn create_session(&mut self) -> UiSessionId {
        let id = UiSessionId(self.next_id);
        self.next_id += 1;
        let title = format!("Session {}", id.0);
        self.sessions
            .insert(0, UiSession::new(id, title, "New chat started."));
        self.selected = id;
        id
    }

    fn select_by_sidebar_index(&mut self, index: usize) -> bool {
        let Some(session) = self.sessions.get(index) else {
            return false;
        };
        self.selected = session.id;
        true
    }

    fn selected_index(&self) -> usize {
        self.sessions
            .iter()
            .position(|session| session.id == self.selected)
            .unwrap_or(0)
    }

    fn select_relative(&mut self, delta: isize) -> bool {
        if self.sessions.len() <= 1 {
            return false;
        }
        let len = self.sessions.len() as isize;
        let current = self.selected_index() as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.select_by_sidebar_index(next)
    }
}

#[derive(Debug)]
struct CodexUi {
    model: String,
    input: UiTextBuffer,
    status: String,
    busy: bool,
    animation_tick: u32,
    runtime: UiRuntimeState,
    sessions: SessionStore,
    clear_confirm_session: Option<UiSessionId>,
    inspector_open: bool,
    inspector_panel: InspectorPanel,
    log_level: LogLevel,
    confirm_clear_enabled: bool,
    density: DensityMode,
    logs: Vec<RuntimeLog>,
    prompt_history: Vec<String>,
    prompt_history_index: Option<usize>,
    prompt_history_path: Option<PathBuf>,
    command_tx: mpsc::Sender<WorkerCommand>,
    event_rx: mpsc::Receiver<WorkerEvent>,
}

#[derive(Debug)]
enum WorkerCommand {
    Prompt {
        session_id: UiSessionId,
        prompt: String,
    },
    Reset {
        session_id: UiSessionId,
    },
    Cancel {
        session_id: UiSessionId,
    },
}

#[derive(Debug)]
enum WorkerEvent {
    Ready,
    StartupError(String),
    Session {
        session_id: UiSessionId,
        event: SessionWorkerEvent,
    },
}

#[derive(Debug)]
enum SessionWorkerEvent {
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
    Cancelled,
    Done,
    Usage(TokenUsage),
    RateLimits(RateLimitSnapshot),
}

#[derive(Clone, Debug, Default)]
struct UsageState {
    total: TokenUsage,
    last: Option<TokenUsage>,
}

#[derive(Clone, Debug, Default)]
struct RateLimitState {
    latest: Option<RateLimitSnapshot>,
    updated_at: Option<Instant>,
}

impl RateLimitState {
    fn record(&mut self, snapshot: RateLimitSnapshot) {
        self.latest = Some(snapshot);
        self.updated_at = Some(Instant::now());
    }

    fn label(&self) -> String {
        let Some(snapshot) = &self.latest else {
            return "limits pending".to_string();
        };
        let name = snapshot.limit_name.as_deref().unwrap_or("limits");
        if let Some(primary) = snapshot.primary.as_ref() {
            format!("{name} {:.0}% used", primary.used_percent.clamp(0.0, 100.0))
        } else if let Some(credits) = snapshot.credits.as_ref() {
            if credits.unlimited {
                "credits unlimited".to_string()
            } else if let Some(balance) = credits.balance.as_deref() {
                format!("credits {balance}")
            } else if credits.has_credits {
                "credits available".to_string()
            } else {
                "credits empty".to_string()
            }
        } else {
            name.to_string()
        }
    }

    fn detail_lines(&self) -> Vec<String> {
        let Some(snapshot) = &self.latest else {
            return vec!["Rate limits have not been reported yet.".to_string()];
        };
        let mut lines = Vec::new();
        if let Some(name) = snapshot.limit_name.as_deref() {
            lines.push(format!("Window: {name}"));
        }
        if let Some(primary) = snapshot.primary.as_ref() {
            lines.push(format!(
                "Primary: {:.0}% used{}",
                primary.used_percent.clamp(0.0, 100.0),
                primary
                    .window_minutes
                    .map(|minutes| format!(" / {minutes}m"))
                    .unwrap_or_default()
            ));
        }
        if let Some(secondary) = snapshot.secondary.as_ref() {
            lines.push(format!(
                "Secondary: {:.0}% used",
                secondary.used_percent.clamp(0.0, 100.0)
            ));
        }
        if let Some(credits) = snapshot.credits.as_ref() {
            let credit_line = if credits.unlimited {
                "Credits: unlimited".to_string()
            } else if let Some(balance) = credits.balance.as_deref() {
                format!("Credits: {balance}")
            } else if credits.has_credits {
                "Credits: available".to_string()
            } else {
                "Credits: empty".to_string()
            };
            lines.push(credit_line);
        }
        if let Some(updated_at) = self.updated_at {
            lines.push(format!(
                "Refreshed: {}",
                elapsed_label(updated_at.elapsed())
            ));
        }
        if lines.is_empty() {
            lines.push("Rate limit response had no visible windows.".to_string());
        }
        lines
    }

    fn primary_percent(&self) -> Option<f32> {
        self.latest
            .as_ref()
            .and_then(|snapshot| snapshot.primary.as_ref())
            .map(|primary| primary.used_percent.clamp(0.0, 100.0) as f32 / 100.0)
    }
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

    fn detail_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "Session total: {}",
            compact_i64(self.total.blended_total())
        )];
        lines.push(format!(
            "Input: {} / output: {}",
            compact_i64(self.total.input_tokens),
            compact_i64(self.total.output_tokens)
        ));
        lines.push(format!(
            "Reasoning: {}",
            compact_i64(self.total.reasoning_output_tokens)
        ));
        if let Some(last) = self.last.as_ref() {
            lines.push(format!("Last turn: {}", compact_i64(last.blended_total())));
        } else {
            lines.push("Last turn: pending".to_string());
        }
        lines
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
        sessions: SessionStore::initial(),
        clear_confirm_session: None,
        inspector_open: false,
        inspector_panel: InspectorPanel::Usage,
        log_level: LogLevel::Normal,
        confirm_clear_enabled: true,
        density: DensityMode::Comfortable,
        logs: Vec::new(),
        prompt_history,
        prompt_history_index: None,
        prompt_history_path,
        command_tx,
        event_rx,
    };

    if options.dump_scene {
        let atlas = FontAtlas::load_inter(18.0)?;
        let mut scene = GpuScene::new(BG);
        state.drain_events();
        build_scene(&mut scene, &atlas, &state, 1120.0, 720.0);
        println!(
            "edgerun-codex ui scene rects={} icon_quads={} text_quads={}",
            scene.rects().len(),
            scene.icon_quads().len(),
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
        let auth = match read_chatgpt_auth() {
            Ok(auth) => auth,
            Err(error) => {
                let _ = event_tx.send(WorkerEvent::StartupError(format!("auth: {error}")));
                return;
            }
        };
        let histories: Arc<Mutex<HashMap<UiSessionId, Vec<codex_core::ResponseItem>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let cancellations: Arc<Mutex<HashMap<UiSessionId, Arc<AtomicBool>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let _ = event_tx.send(WorkerEvent::Ready);

        while let Ok(command) = command_rx.recv() {
            match command {
                WorkerCommand::Reset { session_id } => {
                    if let Ok(mut cancellations) = cancellations.lock()
                        && let Some(cancel) = cancellations.remove(&session_id)
                    {
                        cancel.store(true, Ordering::Relaxed);
                    }
                    if let Ok(mut histories) = histories.lock() {
                        histories.remove(&session_id);
                    }
                    send_session_event(
                        &event_tx,
                        session_id,
                        SessionWorkerEvent::Status("New chat".to_string()),
                    );
                }
                WorkerCommand::Cancel { session_id } => {
                    if let Ok(cancellations) = cancellations.lock()
                        && let Some(cancel) = cancellations.get(&session_id)
                    {
                        cancel.store(true, Ordering::Relaxed);
                        send_session_event(
                            &event_tx,
                            session_id,
                            SessionWorkerEvent::Status("Cancelling".to_string()),
                        );
                    }
                }
                WorkerCommand::Prompt { session_id, prompt } => {
                    let mut history = histories
                        .lock()
                        .ok()
                        .and_then(|mut histories| histories.remove(&session_id))
                        .unwrap_or_default();
                    history.push(user_item(prompt));
                    send_session_event(
                        &event_tx,
                        session_id,
                        SessionWorkerEvent::Status("Thinking".to_string()),
                    );
                    let cancel = Arc::new(AtomicBool::new(false));
                    if let Ok(mut cancellations) = cancellations.lock() {
                        if let Some(previous) = cancellations.insert(session_id, cancel.clone()) {
                            previous.store(true, Ordering::Relaxed);
                        }
                    }
                    let tx = event_tx.clone();
                    let histories = histories.clone();
                    let cancellations = cancellations.clone();
                    let auth = auth.clone();
                    let model = model.clone();
                    thread::spawn(move || {
                        let runtime = match edgerun_tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                        {
                            Ok(runtime) => runtime,
                            Err(error) => {
                                send_session_event(
                                    &tx,
                                    session_id,
                                    SessionWorkerEvent::Error(format!("runtime: {error}")),
                                );
                                return;
                            }
                        };
                        let client = codex_core::ModelClient::new_native(model, provider(), auth);
                        let tx_for_emit = tx.clone();
                        let mut emit = move |event: AgentEvent| match event {
                            AgentEvent::AssistantText(text) => send_session_event(
                                &tx_for_emit,
                                session_id,
                                SessionWorkerEvent::AssistantText(text),
                            ),
                            AgentEvent::AssistantTextDelta(text) => send_session_event(
                                &tx_for_emit,
                                session_id,
                                SessionWorkerEvent::AssistantTextDelta(text),
                            ),
                            AgentEvent::ReasoningDelta(text) => send_session_event(
                                &tx_for_emit,
                                session_id,
                                SessionWorkerEvent::ReasoningDelta(text),
                            ),
                            AgentEvent::ToolDiff(diff) => send_session_event(
                                &tx_for_emit,
                                session_id,
                                SessionWorkerEvent::ToolDiff(diff),
                            ),
                            AgentEvent::ToolInputDelta(delta) => send_session_event(
                                &tx_for_emit,
                                session_id,
                                SessionWorkerEvent::ToolInputDelta(delta),
                            ),
                            AgentEvent::ToolStarted(name) => send_session_event(
                                &tx_for_emit,
                                session_id,
                                SessionWorkerEvent::ToolStarted(name),
                            ),
                            AgentEvent::ToolCompleted {
                                name,
                                summary,
                                success,
                            } => send_session_event(
                                &tx_for_emit,
                                session_id,
                                SessionWorkerEvent::ToolCompleted {
                                    name,
                                    summary,
                                    success,
                                },
                            ),
                            AgentEvent::Usage(usage) => send_session_event(
                                &tx_for_emit,
                                session_id,
                                SessionWorkerEvent::Usage(usage),
                            ),
                            AgentEvent::RateLimits(snapshot) => send_session_event(
                                &tx_for_emit,
                                session_id,
                                SessionWorkerEvent::RateLimits(snapshot),
                            ),
                        };
                        let result = runtime.block_on(run_agent_loop(
                            &client,
                            history,
                            Some(&mut emit),
                            Some(cancel.clone()),
                        ));
                        if let Ok(mut cancellations) = cancellations.lock()
                            && cancellations
                                .get(&session_id)
                                .is_some_and(|existing| Arc::ptr_eq(existing, &cancel))
                        {
                            cancellations.remove(&session_id);
                        }
                        match result {
                            Ok((_, next_history)) => {
                                if let Ok(mut histories) = histories.lock() {
                                    histories.insert(session_id, next_history);
                                }
                                send_session_event(&tx, session_id, SessionWorkerEvent::Done);
                            }
                            Err(error) => {
                                if cancel.load(Ordering::Relaxed) {
                                    send_session_event(
                                        &tx,
                                        session_id,
                                        SessionWorkerEvent::Cancelled,
                                    );
                                    return;
                                }
                                if let Ok(mut histories) = histories.lock() {
                                    histories.remove(&session_id);
                                }
                                send_session_event(
                                    &tx,
                                    session_id,
                                    SessionWorkerEvent::Error(error.to_string()),
                                );
                            }
                        }
                    });
                }
            }
        }
    });
}

fn send_session_event(
    tx: &mpsc::Sender<WorkerEvent>,
    session_id: UiSessionId,
    event: SessionWorkerEvent,
) {
    let _ = tx.send(WorkerEvent::Session { session_id, event });
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
    unsafe {
        SDL_SetWindowMinimumSize(window.0, 420, 360);
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
                SDL_KEYDOWN if event.key_sym() == SDLK_ESCAPE => {
                    if !state.handle_escape() {
                        running = false;
                    }
                    scene_dirty = true;
                }
                SDL_KEYDOWN => {
                    let key = sdl_key(event.key_sym());
                    let modifiers = UiKeyModifiers {
                        shift: event.key_mod() & KMOD_SHIFT != 0,
                        ctrl: event.key_mod() & KMOD_CTRL != 0,
                        ..UiKeyModifiers::default()
                    };
                    if !state.handle_history_key(key, modifiers)
                        && !state.handle_session_key(key, modifiers)
                        && !state.handle_command_key(key, modifiers)
                    {
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
                SDL_WINDOWEVENT
                    if matches!(
                        event.window_event(),
                        SDL_WINDOWEVENT_RESIZED | SDL_WINDOWEVENT_SIZE_CHANGED
                    ) =>
                {
                    width = event.data1().max(640);
                    height = event.data2().max(480);
                    scene_dirty = true;
                }
                _ => {}
            }
        }

        if scene_dirty {
            if state.busy || state.sessions.any_busy() {
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
        } else if frames.is_some() || state.busy || state.sessions.any_busy() {
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
                    self.record_log(LogLevel::Normal, "worker ready");
                }
                WorkerEvent::StartupError(error) => {
                    self.busy = false;
                    self.status = error.clone();
                    self.record_log(LogLevel::Normal, format!("startup error: {error}"));
                    let session = self.sessions.selected_mut();
                    session.status = "Error".to_string();
                    session.failures += 1;
                    session.messages.push(Message {
                        role: Role::Error,
                        text: normalize_error_message(&error),
                    });
                }
                WorkerEvent::Session { session_id, event } => {
                    self.apply_session_event(session_id, event);
                }
            }
        }
        changed
    }

    fn apply_session_event(&mut self, session_id: UiSessionId, event: SessionWorkerEvent) {
        let selected = self.sessions.selected == session_id;
        match event {
            SessionWorkerEvent::Status(status) => {
                if let Some(session) = self.sessions.session_mut(session_id) {
                    session.status = status.clone();
                }
                if selected {
                    self.status = status;
                }
                self.record_log(
                    LogLevel::Debug,
                    format!("session {} status updated", session_id.0),
                );
            }
            SessionWorkerEvent::AssistantText(text) => {
                if let Some(session) = self.sessions.session_mut(session_id) {
                    session.messages.push(Message {
                        role: Role::Assistant,
                        text,
                    });
                    session.assistant_streaming = false;
                    session.reasoning_streaming = false;
                }
                if selected {
                    self.scroll_transcript_to_bottom();
                }
            }
            SessionWorkerEvent::AssistantTextDelta(text) => {
                self.append_streaming_message(session_id, Role::Assistant, text);
                if selected {
                    self.status = "Responding".to_string();
                    self.scroll_transcript_to_bottom();
                }
            }
            SessionWorkerEvent::ReasoningDelta(text) => {
                self.append_streaming_message(session_id, Role::Reasoning, text);
                if selected {
                    self.status = "Reasoning".to_string();
                    self.scroll_transcript_to_bottom();
                }
            }
            SessionWorkerEvent::ToolDiff(diff) => {
                if let Some(session) = self.sessions.session_mut(session_id) {
                    session.assistant_streaming = false;
                    session.reasoning_streaming = false;
                }
                self.append_diff_message(session_id, diff);
                if selected {
                    self.status = "Reviewing patch".to_string();
                }
            }
            SessionWorkerEvent::ToolInputDelta(delta) => {
                if let Some(session) = self.sessions.session_mut(session_id) {
                    session.assistant_streaming = false;
                    session.reasoning_streaming = false;
                }
                self.append_diff_message(session_id, delta);
                if selected {
                    self.status = "Drafting patch".to_string();
                }
            }
            SessionWorkerEvent::ToolStarted(name) => {
                if let Some(session) = self.sessions.session_mut(session_id) {
                    session.assistant_streaming = false;
                    session.reasoning_streaming = false;
                    session.active_tool = Some(name.clone());
                    session.status = "Tool running".to_string();
                    session.messages.push(Message {
                        role: Role::ToolRunning,
                        text: format!("Started\n{name}"),
                    });
                }
                self.record_log(LogLevel::Normal, format!("tool started: {name}"));
                if selected {
                    self.status = "Tool running".to_string();
                    self.scroll_transcript_to_bottom();
                }
            }
            SessionWorkerEvent::ToolCompleted {
                name,
                summary,
                success,
            } => {
                if let Some(session) = self.sessions.session_mut(session_id) {
                    session.assistant_streaming = false;
                    session.reasoning_streaming = false;
                    session.tools_run += 1;
                    if !success {
                        session.failures += 1;
                    }
                    session.active_tool = None;
                    session.messages.push(Message {
                        role: if success {
                            Role::ToolSuccess
                        } else {
                            Role::ToolError
                        },
                        text: format!(
                            "{}\n{name}\n{summary}",
                            if success { "Completed" } else { "Failed" }
                        ),
                    });
                    session.status = if success {
                        "Tool completed".to_string()
                    } else {
                        "Tool failed".to_string()
                    };
                    if selected {
                        self.status = session.status.clone();
                    }
                }
                self.record_log(
                    LogLevel::Normal,
                    format!(
                        "tool {}: {name}",
                        if success { "completed" } else { "failed" }
                    ),
                );
                if selected {
                    self.scroll_transcript_to_bottom();
                }
            }
            SessionWorkerEvent::Error(error) => {
                self.record_log(LogLevel::Normal, format!("session error: {error}"));
                if let Some(session) = self.sessions.session_mut(session_id) {
                    session.busy = false;
                    session.failures += 1;
                    session.active_tool = None;
                    session.assistant_streaming = false;
                    session.reasoning_streaming = false;
                    session.status = "Error".to_string();
                    session.messages.push(Message {
                        role: Role::Error,
                        text: normalize_error_message(&error),
                    });
                }
                if selected {
                    self.status = "Error".to_string();
                    self.scroll_transcript_to_bottom();
                }
            }
            SessionWorkerEvent::Cancelled => {
                self.record_log(
                    LogLevel::Normal,
                    format!("session {} stopped", session_id.0),
                );
                if let Some(session) = self.sessions.session_mut(session_id) {
                    session.busy = false;
                    session.active_tool = None;
                    session.assistant_streaming = false;
                    session.reasoning_streaming = false;
                    session.status = "Stopped".to_string();
                    session.messages.push(Message {
                        role: Role::ToolError,
                        text: "Turn stopped.".to_string(),
                    });
                }
                if selected {
                    self.status = "Stopped".to_string();
                    self.scroll_transcript_to_bottom();
                }
            }
            SessionWorkerEvent::Done => {
                if let Some(session) = self.sessions.session_mut(session_id) {
                    session.busy = false;
                    session.active_tool = None;
                    session.assistant_streaming = false;
                    session.reasoning_streaming = false;
                    session.status = "Ready".to_string();
                }
                if selected {
                    self.status = "Ready".to_string();
                }
            }
            SessionWorkerEvent::Usage(usage) => {
                if let Some(session) = self.sessions.session_mut(session_id) {
                    session.usage.record(usage);
                }
                self.record_log(
                    LogLevel::Debug,
                    format!("usage updated for session {}", session_id.0),
                );
            }
            SessionWorkerEvent::RateLimits(snapshot) => {
                if let Some(session) = self.sessions.session_mut(session_id) {
                    session.rate_limits.record(snapshot);
                }
                self.record_log(
                    LogLevel::Debug,
                    format!("rate limits updated for session {}", session_id.0),
                );
            }
        };
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

    fn handle_session_key(&mut self, key: UiKey, modifiers: UiKeyModifiers) -> bool {
        if !modifiers.ctrl {
            return false;
        }
        match key {
            UiKey::PageUp => self.switch_relative_session(-1),
            UiKey::PageDown => self.switch_relative_session(1),
            _ => false,
        }
    }

    fn handle_command_key(&mut self, key: UiKey, modifiers: UiKeyModifiers) -> bool {
        if !modifiers.ctrl {
            return false;
        }
        match key {
            UiKey::Other(code) if modifiers.shift && code_eq(code, b'n') => {
                self.new_chat();
                true
            }
            UiKey::Other(code) if code == b',' as u32 => {
                self.open_inspector_panel(InspectorPanel::Settings);
                true
            }
            UiKey::Other(code) if code == b'.' as u32 => {
                self.open_inspector_panel(InspectorPanel::Usage);
                true
            }
            UiKey::Other(code) if modifiers.shift && code_eq(code, b'l') => {
                self.open_inspector_panel(InspectorPanel::Logs);
                true
            }
            UiKey::Other(code) if code_eq(code, b'c') => {
                self.cancel_selected_session();
                true
            }
            UiKey::Backspace if modifiers.shift => {
                self.request_clear_transcript();
                true
            }
            _ => false,
        }
    }

    fn handle_escape(&mut self) -> bool {
        if self.clear_confirm_session == Some(self.sessions.selected) {
            self.dismiss_clear_confirmation();
            return true;
        }
        if self.inspector_open {
            self.inspector_open = false;
            self.record_log(LogLevel::Debug, "inspector closed");
            return true;
        }
        false
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
            UiAction::Activated(hit) if hit.id == STOP_ID => self.cancel_selected_session(),
            UiAction::Activated(hit) if hit.id == SETTINGS_ID => {
                self.open_inspector_panel(InspectorPanel::Settings);
            }
            UiAction::Activated(hit) if hit.id == INSPECTOR_ID => {
                self.inspector_open = !self.inspector_open;
                if self.inspector_open {
                    self.inspector_panel = InspectorPanel::Usage;
                }
            }
            UiAction::Activated(hit)
                if (PANEL_TAB_BASE_ID..PANEL_TAB_BASE_ID + 3).contains(&hit.id) =>
            {
                self.open_inspector_panel(InspectorPanel::from_index(
                    (hit.id - PANEL_TAB_BASE_ID) as usize,
                ));
            }
            UiAction::Activated(hit)
                if (LOG_LEVEL_BASE_ID..LOG_LEVEL_BASE_ID + 3).contains(&hit.id) =>
            {
                self.log_level = LogLevel::from_index((hit.id - LOG_LEVEL_BASE_ID) as usize);
                self.record_log(
                    LogLevel::Normal,
                    format!("log level set to {}", self.log_level.label()),
                );
            }
            UiAction::Activated(hit) if hit.id == CONFIRM_CLEAR_ID => {
                self.confirm_clear_enabled = !self.confirm_clear_enabled;
                self.record_log(
                    LogLevel::Normal,
                    format!(
                        "clear confirmation {}",
                        if self.confirm_clear_enabled {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    ),
                );
            }
            UiAction::Activated(hit)
                if (DENSITY_BASE_ID..DENSITY_BASE_ID + 2).contains(&hit.id) =>
            {
                self.density = DensityMode::from_index((hit.id - DENSITY_BASE_ID) as usize);
                self.record_log(
                    LogLevel::Normal,
                    format!("density set to {}", self.density.label()),
                );
            }
            UiAction::Activated(hit) if hit.id == CLEAR_ID => {
                self.request_clear_transcript();
            }
            UiAction::ScrollChanged { id, offset } if id == TRANSCRIPT_SCROLL_ID => {
                self.runtime.set_scroll_offset(id, offset);
            }
            UiAction::Activated(hit) if hit.id >= SESSION_ROW_BASE_ID => {
                self.switch_session((hit.id - SESSION_ROW_BASE_ID) as usize);
            }
            _ => {}
        }
    }

    fn open_inspector_panel(&mut self, panel: InspectorPanel) {
        self.inspector_open = true;
        self.inspector_panel = panel;
        self.record_log(
            LogLevel::Debug,
            format!("{} opened", panel.label().to_ascii_lowercase()),
        );
    }

    fn scroll_transcript_to_bottom(&mut self) {
        self.runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 1.0);
    }

    fn scroll_transcript_by(&mut self, delta: f32) {
        let next = self.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID) + delta;
        self.runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, next);
    }

    fn append_streaming_message(&mut self, session_id: UiSessionId, role: Role, text: String) {
        if text.is_empty() {
            return;
        }
        let Some(session) = self.sessions.session_mut(session_id) else {
            return;
        };
        let streaming = match role {
            Role::Assistant => &mut session.assistant_streaming,
            Role::Reasoning => &mut session.reasoning_streaming,
            _ => return,
        };
        if *streaming
            && let Some(message) = session.messages.last_mut()
            && message.role == role
        {
            message.text.push_str(&text);
        } else {
            let text = match role {
                Role::Reasoning => format!("Thinking\n{text}"),
                Role::Assistant => format!("Response\n{text}"),
                _ => text,
            };
            session.messages.push(Message { role, text });
            *streaming = true;
        }
        match role {
            Role::Assistant => session.reasoning_streaming = false,
            Role::Reasoning => session.assistant_streaming = false,
            _ => {}
        }
        if self.sessions.selected == session_id {
            self.scroll_transcript_to_bottom();
        }
    }

    fn append_diff_message(&mut self, session_id: UiSessionId, text: String) {
        if text.is_empty() {
            return;
        }
        let Some(session) = self.sessions.session_mut(session_id) else {
            return;
        };
        if let Some(message) = session.messages.last_mut()
            && message.role == Role::Diff
        {
            append_capped(&mut message.text, &text, MAX_DIFF_DISPLAY_BYTES);
        } else {
            let mut text = text;
            if text.len() > MAX_DIFF_DISPLAY_BYTES {
                text.truncate(char_boundary_at_or_before(&text, MAX_DIFF_DISPLAY_BYTES));
                text.push_str("\n[diff preview truncated]");
            }
            if !text.starts_with("Patch") && !text.starts_with("Tool input") {
                text = format!("Patch / tool input\n{text}");
            }
            session.messages.push(Message {
                role: Role::Diff,
                text,
            });
        }
        if self.sessions.selected == session_id {
            self.scroll_transcript_to_bottom();
        }
    }

    fn submit(&mut self) {
        if self.busy {
            self.status = "Codex worker is starting".to_string();
            return;
        }
        if self.sessions.selected().busy {
            self.status = "This session is still working".to_string();
            return;
        }
        let prompt = self.input.as_str().trim().to_string();
        if prompt.is_empty() {
            self.status = "Type a prompt before sending".to_string();
            return;
        }
        let session_id = self.sessions.selected;
        {
            let session = self.sessions.selected_mut();
            if session.turns == 0 {
                session.title = session_title_from_prompt(&prompt);
            }
            session.messages.push(Message {
                role: Role::User,
                text: prompt.clone(),
            });
            session.turns += 1;
            session.busy = true;
            session.status = "Queued".to_string();
            session.assistant_streaming = false;
            session.reasoning_streaming = false;
        }
        self.clear_pending_clear_confirmation();
        self.remember_prompt(prompt.clone());
        self.input.clear();
        self.status = "Queued".to_string();
        self.record_log(
            LogLevel::Normal,
            format!("queued session {} prompt", session_id.0),
        );
        self.scroll_transcript_to_bottom();
        if self
            .command_tx
            .send(WorkerCommand::Prompt { session_id, prompt })
            .is_err()
        {
            if let Some(session) = self.sessions.session_mut(session_id) {
                session.busy = false;
                session.status = "Worker disconnected".to_string();
            }
            self.status = "Worker disconnected".to_string();
        }
    }

    fn new_chat(&mut self) {
        self.clear_pending_clear_confirmation();
        let session_id = self.sessions.create_session();
        self.scroll_transcript_to_bottom();
        self.status = "New chat".to_string();
        self.prompt_history_index = None;
        let _ = self.command_tx.send(WorkerCommand::Reset { session_id });
    }

    fn clear_transcript(&mut self) {
        if !self.ensure_selected_idle("Wait for this session to finish before clearing it") {
            return;
        }
        self.clear_pending_clear_confirmation();
        self.reset_visible_session("Transcript cleared.");
        self.status = "Transcript cleared".to_string();
        let _ = self.command_tx.send(WorkerCommand::Reset {
            session_id: self.sessions.selected,
        });
    }

    fn request_clear_transcript(&mut self) {
        let session_id = self.sessions.selected;
        if !self.confirm_clear_enabled {
            self.clear_transcript();
            return;
        }
        if self.clear_confirm_session == Some(session_id) {
            self.clear_transcript();
            return;
        }
        if !self.ensure_selected_idle("Wait for this session to finish before clearing it") {
            return;
        }
        self.clear_confirm_session = Some(session_id);
        self.status = "Press clear again to confirm".to_string();
        if let Some(session) = self.sessions.session_mut(session_id) {
            session.status = "Confirm clear".to_string();
        }
    }

    fn dismiss_clear_confirmation(&mut self) {
        self.clear_pending_clear_confirmation();
        self.status = "Clear cancelled".to_string();
    }

    fn clear_pending_clear_confirmation(&mut self) {
        let Some(session_id) = self.clear_confirm_session.take() else {
            return;
        };
        if let Some(session) = self.sessions.session_mut(session_id)
            && session.status == "Confirm clear"
        {
            session.status = "Ready".to_string();
        }
    }

    fn cancel_selected_session(&mut self) {
        let session_id = self.sessions.selected;
        if !self.sessions.selected().busy {
            self.status = "Nothing running in this session".to_string();
            return;
        }
        self.clear_pending_clear_confirmation();
        self.status = "Cancelling".to_string();
        if let Some(session) = self.sessions.session_mut(session_id) {
            session.status = "Cancelling".to_string();
        }
        let _ = self.command_tx.send(WorkerCommand::Cancel { session_id });
    }

    fn ensure_selected_idle(&mut self, message: &str) -> bool {
        if self.busy || self.sessions.selected().busy {
            self.status = message.to_string();
            false
        } else {
            true
        }
    }

    fn reset_visible_session(&mut self, message: &str) {
        self.sessions.selected_mut().reset(message);
        self.scroll_transcript_to_bottom();
    }

    fn switch_session(&mut self, index: usize) {
        self.clear_pending_clear_confirmation();
        if self.sessions.select_by_sidebar_index(index) {
            self.status = self.sessions.selected().status.clone();
            self.scroll_transcript_to_bottom();
        }
    }

    fn switch_relative_session(&mut self, delta: isize) -> bool {
        self.clear_pending_clear_confirmation();
        if self.sessions.select_relative(delta) {
            self.status = self.sessions.selected().status.clone();
            self.scroll_transcript_to_bottom();
            true
        } else {
            false
        }
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

    fn record_log(&mut self, level: LogLevel, text: impl Into<String>) {
        if level > self.log_level {
            return;
        }
        self.logs.push(RuntimeLog {
            level,
            text: text.into(),
        });
        if self.logs.len() > 200 {
            let remove = self.logs.len() - 200;
            self.logs.drain(..remove);
        }
    }
}

fn build_scene(scene: &mut GpuScene, atlas: &FontAtlas, state: &CodexUi, width: f32, height: f32) {
    scene.clear = BG;
    scene.clear_rects();

    let w = width.max(1.0);
    let h = height.max(1.0);
    let selected = state.sessions.selected();
    let usage = selected.usage.total_label();
    let model = format!("model {}", state.model);
    let workspace = workspace_label();
    let tone = header_tone(state);
    let (activity_title, activity_detail, activity_icon) = sidebar_activity(state);
    let header_status = if state.busy {
        state.status.as_str()
    } else if selected.busy {
        selected.status.as_str()
    } else if state.sessions.any_busy() {
        "Background session running"
    } else {
        selected.status.as_str()
    };
    let inspector_width = if state.inspector_open && w >= 900.0 {
        320.0
    } else {
        0.0
    };
    let main_width = if w >= 980.0 {
        w - 260.0 - inspector_width
    } else {
        w - inspector_width
    };
    let messages = conversation_messages(atlas, state, main_width - 48.0);
    let composer_text = state.input.display_value(
        "Ask Codex to inspect, edit, run commands, or patch files...",
        '|',
    );

    let actions = [
        UiShadcnChatClientAction::icon(
            UiIcon::MessagePlus,
            NEW_CHAT_ID,
            UiShadcnButtonVariant::Default,
        ),
        UiShadcnChatClientAction::icon(
            UiIcon::Settings,
            SETTINGS_ID,
            UiShadcnButtonVariant::Secondary,
        ),
    ];
    let session_details = state
        .sessions
        .sessions
        .iter()
        .map(session_sidebar_detail)
        .collect::<Vec<_>>();
    let session_rows = state
        .sessions
        .sessions
        .iter()
        .zip(session_details.iter())
        .map(|(session, detail)| {
            UiShadcnSessionRow::new(
                session.title.as_str(),
                detail.as_str(),
                session.id == state.sessions.selected,
            )
            .with_state(session_sidebar_state(session))
        })
        .collect::<Vec<_>>();
    let limits = selected.rate_limits.label();
    let footer_lines = [
        model.as_str(),
        usage.as_str(),
        limits.as_str(),
        "shell, process, patch",
    ];
    let log_badge = format!("log {}", state.log_level.label().to_ascii_lowercase());
    let header_badges = if w < 520.0 {
        vec![usage.as_str()]
    } else if w < 700.0 {
        vec![state.model.as_str(), usage.as_str()]
    } else {
        vec![state.model.as_str(), usage.as_str(), log_badge.as_str()]
    };
    let header_actions = [
        UiShadcnChatClientIconAction::new(
            UiIcon::Activity,
            INSPECTOR_ID,
            state.inspector_open && state.inspector_panel == InspectorPanel::Usage,
        ),
        UiShadcnChatClientIconAction::new(
            UiIcon::Settings,
            SETTINGS_ID,
            state.inspector_open && state.inspector_panel == InspectorPanel::Settings,
        ),
    ];
    let hints: [&str; 0] = [];
    let composer_action = Some(if selected.busy {
        UiShadcnChatClientIconAction::new(UiIcon::X, STOP_ID, true)
    } else {
        UiShadcnChatClientIconAction::new(
            UiIcon::Trash,
            CLEAR_ID,
            state.clear_confirm_session == Some(state.sessions.selected),
        )
    });
    let composer_notice = (state.clear_confirm_session == Some(state.sessions.selected)).then_some(
        UiShadcnComposerNotice::new(
            "Clear transcript?",
            "Activate clear again to reset this session transcript.",
            UiIcon::Warning,
        ),
    );

    let client = shadcn_chat_client(UiShadcnChatClientSpec {
        show_sidebar: w >= 980.0,
        sidebar_title: "edgerun codex",
        sidebar_detail: workspace.as_str(),
        sidebar_actions: &actions,
        session_rows: &session_rows,
        activity: UiShadcnActivity::new(activity_title, activity_detail, activity_icon),
        footer_lines: &footer_lines,
        header_title: "Codex",
        header_status,
        header_badges: &header_badges,
        header_action: (w < 980.0).then_some(UiShadcnChatClientIconAction::new(
            UiIcon::MessagePlus,
            NEW_CHAT_ID,
            false,
        )),
        header_actions: &header_actions,
        header_tone: tone,
        activity_phase: (state.animation_tick / 8) as u8,
        messages: &messages,
        scroll_offset: state.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID),
        scroll_id: TRANSCRIPT_SCROLL_ID,
        input_label: "",
        input_value: &composer_text,
        input_id: 0,
        composer_action,
        composer_notice,
        composer_expanded: false,
        send_label: "",
        send_id: SEND_ID,
        busy: state.busy || selected.busy,
        hints: &hints,
    });

    let root = if state.inspector_open && w >= 900.0 {
        row("h-full bg-bg")
            .child(client.class("flex-1"))
            .child(inspector_panel_node(state).class("w-80 h-full"))
    } else if state.inspector_open {
        inspector_panel_node(state).class("h-full")
    } else {
        client
    };

    render_node(
        scene,
        atlas,
        UiRect::new(0.0, 0.0, w, h),
        &state.runtime,
        root,
    );
}

fn inspector_panel_node(state: &CodexUi) -> UiNode {
    let selected = state.sessions.selected();
    let title = match state.inspector_panel {
        InspectorPanel::Usage => "Usage",
        InspectorPanel::Settings => "Settings",
        InspectorPanel::Logs => "Runtime log",
    };
    let mut panel = column("h-full bg-panel border p-3 gap-3")
        .child(
            row("h-8 items-center justify-between")
                .child(shadcn_label(title).class("h-5 text-text"))
                .child(
                    row("gap-2 items-center")
                        .child(shadcn_badge(
                            selected.status.as_str(),
                            if selected.busy {
                                UiShadcnBadgeVariant::Default
                            } else {
                                UiShadcnBadgeVariant::Secondary
                            },
                        ))
                        .child(icon_button(UiIcon::X, INSPECTOR_ID).class("h-8 w-8")),
                ),
        )
        .child(shadcn_toggle_group(
            &InspectorPanel::LABELS,
            state.inspector_panel.index(),
            PANEL_TAB_BASE_ID,
        ))
        .child(shadcn_separator());

    panel = match state.inspector_panel {
        InspectorPanel::Usage => panel.child(usage_panel_node(selected, &state.sessions)),
        InspectorPanel::Settings => panel.child(settings_panel_node(state)),
        InspectorPanel::Logs => panel.child(log_panel_node(state)),
    };
    panel.child(spacer("flex-1"))
}

fn usage_panel_node(session: &UiSession, sessions: &SessionStore) -> UiNode {
    let usage_lines = session.usage.detail_lines();
    let rate_lines = session.rate_limits.detail_lines();
    let total_label = session.usage.total_label();
    let app_total = sessions.aggregate_usage();
    let app_lines = [
        format!("Sessions: {}", sessions.sessions.len()),
        format!("Running: {}", sessions.active_count()),
        format!("App total: {}", compact_i64(app_total.blended_total())),
        format!(
            "Reasoning total: {}",
            compact_i64(app_total.reasoning_output_tokens)
        ),
    ];
    let usage_rows = usage_lines.iter().map(String::as_str).collect::<Vec<_>>();
    let rate_rows = rate_lines.iter().map(String::as_str).collect::<Vec<_>>();
    let app_rows = app_lines.iter().map(String::as_str).collect::<Vec<_>>();

    let mut node = column("gap-3")
        .child(shadcn_alert(
            "Session usage",
            total_label.as_str(),
            UiIcon::Activity,
        ))
        .child(label_stack("App usage", &app_rows))
        .child(label_stack("Tokens", &usage_rows))
        .child(label_stack("Rate limits", &rate_rows));
    if let Some(percent) = session.rate_limits.primary_percent() {
        node = node.child(shadcn_progress(percent).class("h-3"));
    }
    node
}

fn settings_panel_node(state: &CodexUi) -> UiNode {
    column("gap-3")
        .child(shadcn_card("Model", state.model.as_str()))
        .child(shadcn_card(
            "Logging",
            "Controls which runtime events are shown in the local log.",
        ))
        .child(shadcn_toggle_group(
            &LogLevel::LABELS,
            state.log_level.index(),
            LOG_LEVEL_BASE_ID,
        ))
        .child(shadcn_card(
            "Session safety",
            "Clear can require a second activation before transcript reset.",
        ))
        .child(
            row("h-10 items-center justify-between gap-2")
                .child(shadcn_label("Confirm clear").class("h-5 text-text"))
                .child(shadcn_switch(state.confirm_clear_enabled, CONFIRM_CLEAR_ID)),
        )
        .child(shadcn_card("Density", "Adjusts timeline spacing."))
        .child(shadcn_toggle_group(
            &DensityMode::LABELS,
            state.density.index(),
            DENSITY_BASE_ID,
        ))
}

fn log_panel_node(state: &CodexUi) -> UiNode {
    let visible_logs = state
        .logs
        .iter()
        .rev()
        .filter(|entry| entry.level <= state.log_level)
        .take(9)
        .map(|entry| {
            format!(
                "{}: {}",
                entry.level.label().to_ascii_lowercase(),
                entry.text
            )
        })
        .collect::<Vec<_>>();
    if visible_logs.is_empty() {
        return shadcn_alert(
            "Runtime log",
            "No runtime events at this level.",
            UiIcon::Activity,
        );
    }
    let log_rows = visible_logs.iter().map(String::as_str).collect::<Vec<_>>();
    label_stack("Recent events", &log_rows)
}

fn label_stack(title: &str, rows: &[&str]) -> UiNode {
    let mut node = column("gap-2").child(shadcn_label(title).class("h-5 text-muted-foreground"));
    for row_text in rows {
        node = node.child(shadcn_label(row_text).class("h-5 text-text"));
    }
    node
}

fn header_tone(state: &CodexUi) -> UiShadcnStatusTone {
    if state.sessions.selected().failures > 0 {
        UiShadcnStatusTone::Error
    } else if state.busy || state.sessions.any_busy() {
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
        .sessions
        .selected()
        .messages
        .iter()
        .map(|message| {
            let role = chat_role(message.role);
            UiShadcnConversationMessage::new(
                role,
                &message.text,
                message_height_for_density(
                    shadcn_chat_message_height_for_role(atlas, role, &message.text, width),
                    state.density,
                ),
            )
        })
        .collect()
}

fn message_height_for_density(height: f32, density: DensityMode) -> f32 {
    match density {
        DensityMode::Comfortable => height,
        DensityMode::Compact => (height * 0.86).max(72.0),
    }
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
    if let Some(tool) = state.sessions.selected().active_tool.as_deref() {
        ("Running", tool, UiIcon::Terminal)
    } else if state.busy || state.sessions.selected().busy {
        (
            "Thinking",
            state.sessions.selected().status.as_str(),
            UiIcon::Code,
        )
    } else {
        ("Idle", "No active tool", UiIcon::Check)
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

fn session_title_from_prompt(prompt: &str) -> String {
    let trimmed = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut title = trimmed.chars().take(32).collect::<String>();
    if trimmed.chars().count() > 32 {
        title.push_str("...");
    }
    if title.is_empty() {
        "Untitled session".to_string()
    } else {
        title
    }
}

fn session_sidebar_detail(session: &UiSession) -> String {
    let turns = count_label(session.turns, "turn");
    let tools = count_label(session.tools_run, "tool");
    if session.busy {
        let running = session
            .active_tool
            .as_deref()
            .unwrap_or(session.status.as_str());
        return format!("running {running} | {turns} | {tools}");
    }
    if session.failures > 0 {
        return format!(
            "{} | {} | {}",
            turns,
            count_label(session.failures, "failure"),
            session.usage.total_label()
        );
    }
    if session.turns == 0 {
        return "new | usage pending".to_string();
    }
    format!("{turns} | {tools} | {}", session.usage.detail_label())
}

fn session_sidebar_state(session: &UiSession) -> UiShadcnSessionState {
    if session.failures > 0 {
        UiShadcnSessionState::Error
    } else if session.busy {
        UiShadcnSessionState::Running
    } else {
        UiShadcnSessionState::Idle
    }
}

fn count_label(count: usize, unit: &str) -> String {
    if count == 1 {
        format!("1 {unit}")
    } else {
        format!("{count} {unit}s")
    }
}

fn code_eq(code: u32, ascii_lowercase: u8) -> bool {
    code == ascii_lowercase as u32 || code == ascii_lowercase.to_ascii_uppercase() as u32
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

fn elapsed_label(duration: Duration) -> String {
    if duration.as_secs() < 60 {
        "just now".to_string()
    } else if duration.as_secs() < 3600 {
        format!("{}m ago", duration.as_secs() / 60)
    } else {
        format!("{}h ago", duration.as_secs() / 3600)
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

    fn test_state(
        status: &str,
        busy: bool,
        runtime: UiRuntimeState,
        session: UiSession,
    ) -> CodexUi {
        let (command_tx, _command_rx) = std::sync::mpsc::channel();
        let (_event_tx, event_rx) = std::sync::mpsc::channel();
        CodexUi {
            model: "gpt-5.5".to_string(),
            input: UiTextBuffer::new(),
            status: status.to_string(),
            busy,
            animation_tick: 0,
            runtime,
            sessions: SessionStore {
                selected: session.id,
                next_id: session.id.0 + 1,
                sessions: vec![session],
            },
            clear_confirm_session: None,
            inspector_open: false,
            inspector_panel: InspectorPanel::Usage,
            log_level: LogLevel::Normal,
            confirm_clear_enabled: true,
            density: DensityMode::Comfortable,
            logs: Vec::new(),
            prompt_history: Vec::new(),
            prompt_history_index: None,
            prompt_history_path: None,
            command_tx,
            event_rx,
        }
    }

    fn test_session(messages: Vec<Message>) -> UiSession {
        let mut session = UiSession::new(UiSessionId(1), "Current workspace", "ready");
        session.messages = messages;
        session
    }

    #[test]
    fn build_scene_renders_shadcn_chat_client_controls() {
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 1.0);
        let mut session = test_session(vec![
            Message {
                role: Role::Assistant,
                text: "Ready for local workspace work.".to_string(),
            },
            Message {
                role: Role::User,
                text: "Summarize the project.".to_string(),
            },
        ]);
        session.turns = 1;
        session.tools_run = 1;
        let state = test_state("Ready", false, runtime, session);
        let atlas = FontAtlas::load_inter(18.0).expect("load UI font");
        let mut scene = GpuScene::new(BG);

        build_scene(&mut scene, &atlas, &state, 1120.0, 720.0);

        assert!(scene.text_quads().len() > 80);
        assert!(scene.icon_quads().len() >= 2);
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
                .any(|hit| hit.kind == HitKind::Button && hit.id == CLEAR_ID),
            "scene hits: {:?}",
            scene.hits()
        );
    }

    #[test]
    fn compact_scene_keeps_primary_actions_inside_viewport() {
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 1.0);
        let mut session = test_session(vec![Message {
            role: Role::Assistant,
            text: "Ready for local workspace work.".to_string(),
        }]);
        session.turns = 1;
        let state = test_state("Ready", false, runtime, session);
        let atlas = FontAtlas::load_inter(18.0).expect("load UI font");
        let mut scene = GpuScene::new(BG);

        build_scene(&mut scene, &atlas, &state, 640.0, 520.0);

        for id in [NEW_CHAT_ID, CLEAR_ID, SEND_ID] {
            let hit = scene
                .hits()
                .iter()
                .find(|hit| hit.kind == HitKind::Button && hit.id == id)
                .unwrap_or_else(|| panic!("missing button hit {id}; hits: {:?}", scene.hits()));
            assert!(
                hit.x >= 0.0 && hit.y >= 0.0,
                "hit outside top-left: {hit:?}"
            );
            assert!(
                hit.x + hit.w <= 640.0 && hit.y + hit.h <= 520.0,
                "hit outside compact viewport: {hit:?}"
            );
        }
        assert!(scene.icon_quads().len() >= 3);
    }

    #[test]
    fn streaming_deltas_append_to_current_transcript_rows() {
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.0);
        let mut session = test_session(vec![Message {
            role: Role::User,
            text: "stream".to_string(),
        }]);
        session.turns = 1;
        let mut state = test_state("Thinking", true, runtime, session);

        let session_id = state.sessions.selected;
        state.append_streaming_message(session_id, Role::Reasoning, "checking".to_string());
        state.append_streaming_message(session_id, Role::Reasoning, " files".to_string());
        state.append_streaming_message(session_id, Role::Assistant, "done".to_string());
        state.append_streaming_message(session_id, Role::Assistant, ".".to_string());

        let messages = &state.sessions.selected().messages;
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[1].role, Role::Reasoning);
        assert_eq!(messages[1].text, "Thinking\nchecking files");
        assert_eq!(messages[2].role, Role::Assistant);
        assert_eq!(messages[2].text, "Response\ndone.");
        assert_eq!(state.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID), 1.0);
    }

    #[test]
    fn tool_input_deltas_append_to_diff_row() {
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.0);
        let mut session = test_session(Vec::new());
        session.turns = 1;
        let mut state = test_state("Thinking", true, runtime, session);

        let session_id = state.sessions.selected;
        state.append_diff_message(session_id, "*** Begin Patch\n".to_string());
        state.append_diff_message(session_id, "+new line\n".to_string());

        let messages = &state.sessions.selected().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, Role::Diff);
        assert_eq!(
            messages[0].text,
            "Patch / tool input\n*** Begin Patch\n+new line\n"
        );
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
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 1.0);
        let mut state = test_state("Ready", false, runtime, test_session(Vec::new()));
        state.prompt_history = vec!["first prompt".to_string(), "second prompt".to_string()];
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
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 1.0);
        let mut state = test_state("Ready", false, runtime, test_session(Vec::new()));
        state.prompt_history = vec!["first".to_string(), "second".to_string()];
        state.prompt_history_index = Some(0);

        state.remember_prompt("first".to_string());

        assert_eq!(state.prompt_history, vec!["second", "first"]);
        assert_eq!(state.prompt_history_index, None);
    }

    #[test]
    fn clear_transcript_resets_visible_metrics_without_dropping_history() {
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.0);
        let mut session = test_session(vec![Message {
            role: Role::Error,
            text: "broken".to_string(),
        }]);
        session.turns = 4;
        session.tools_run = 3;
        session.failures = 2;
        session.active_tool = Some("shell".to_string());
        let mut state = test_state("Error", false, runtime, session);
        state.prompt_history = vec!["keep this".to_string()];
        state.prompt_history_index = Some(0);

        state.clear_transcript();

        let session = state.sessions.selected();
        assert_eq!(session.turns, 0);
        assert_eq!(session.tools_run, 0);
        assert_eq!(session.failures, 0);
        assert_eq!(session.active_tool, None);
        assert_eq!(state.prompt_history, vec!["keep this"]);
        assert_eq!(state.prompt_history_index, Some(0));
        assert_eq!(state.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID), 1.0);
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].role, Role::Assistant);
    }

    #[test]
    fn request_clear_requires_second_activation() {
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.0);
        let mut session = test_session(vec![Message {
            role: Role::User,
            text: "keep until confirmed".to_string(),
        }]);
        session.turns = 1;
        let mut state = test_state("Ready", false, runtime, session);

        state.request_clear_transcript();

        assert_eq!(state.status, "Press clear again to confirm");
        assert_eq!(state.clear_confirm_session, Some(state.sessions.selected));
        assert_eq!(state.sessions.selected().status, "Confirm clear");
        assert_eq!(state.sessions.selected().turns, 1);
        assert_eq!(state.sessions.selected().messages.len(), 1);

        state.request_clear_transcript();

        assert_eq!(state.clear_confirm_session, None);
        assert_eq!(state.status, "Transcript cleared");
        assert_eq!(state.sessions.selected().turns, 0);
        assert_eq!(state.sessions.selected().messages.len(), 1);
        assert_eq!(state.sessions.selected().messages[0].role, Role::Assistant);
    }

    #[test]
    fn escape_dismisses_clear_confirmation_before_quitting() {
        let runtime = UiRuntimeState::default();
        let session = test_session(vec![Message {
            role: Role::User,
            text: "keep".to_string(),
        }]);
        let mut state = test_state("Ready", false, runtime, session);

        state.request_clear_transcript();
        assert_eq!(state.clear_confirm_session, Some(state.sessions.selected));

        assert!(state.handle_escape());
        assert_eq!(state.clear_confirm_session, None);
        assert_eq!(state.status, "Clear cancelled");
        assert_eq!(state.sessions.selected().status, "Ready");
        assert_eq!(state.sessions.selected().messages.len(), 1);
    }

    #[test]
    fn clear_is_guarded_but_new_chat_keeps_busy_session_running() {
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.25);
        let mut session = test_session(vec![Message {
            role: Role::ToolRunning,
            text: "Running shell".to_string(),
        }]);
        session.turns = 2;
        session.tools_run = 1;
        session.active_tool = Some("shell".to_string());
        session.busy = true;
        let mut state = test_state("Tool running", false, runtime, session);
        state.prompt_history = vec!["keep this".to_string()];
        state.prompt_history_index = Some(0);

        state.clear_transcript();

        assert_eq!(
            state.status,
            "Wait for this session to finish before clearing it"
        );
        let busy_session_id = state.sessions.selected;
        let busy_session = state.sessions.selected();
        assert_eq!(busy_session.turns, 2);
        assert_eq!(busy_session.tools_run, 1);
        assert_eq!(busy_session.active_tool.as_deref(), Some("shell"));
        assert_eq!(busy_session.messages.len(), 1);

        state.new_chat();

        assert_eq!(state.status, "New chat");
        assert_eq!(state.sessions.sessions.len(), 2);
        assert_ne!(state.sessions.selected, busy_session_id);
        assert_eq!(state.sessions.selected().turns, 0);
        assert_eq!(
            state.sessions.selected().messages[0].text,
            "New chat started."
        );
        let background = state
            .sessions
            .sessions
            .iter()
            .find(|session| session.id == busy_session_id)
            .expect("busy session should remain available");
        assert!(background.busy);
        assert_eq!(background.turns, 2);
        assert_eq!(background.tools_run, 1);
        assert_eq!(background.active_tool.as_deref(), Some("shell"));
        assert_eq!(background.messages.len(), 1);
        assert_eq!(state.prompt_history, vec!["keep this"]);
        assert_eq!(state.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID), 1.0);
    }

    #[test]
    fn new_chat_creates_selected_session_without_dropping_existing_one() {
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.0);
        let mut session = test_session(vec![Message {
            role: Role::User,
            text: "existing".to_string(),
        }]);
        session.turns = 1;
        let mut state = test_state("Ready", false, runtime, session);

        state.new_chat();

        assert_eq!(state.sessions.sessions.len(), 2);
        assert_eq!(state.sessions.selected().turns, 0);
        assert_eq!(
            state.sessions.selected().messages[0].text,
            "New chat started."
        );
        assert!(state.sessions.sessions.iter().any(|session| {
            session
                .messages
                .iter()
                .any(|message| message.text == "existing")
        }));
    }

    #[test]
    fn sidebar_session_selection_switches_when_idle() {
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.0);
        let mut state = test_state(
            "Ready",
            false,
            runtime,
            test_session(vec![Message {
                role: Role::Assistant,
                text: "first".to_string(),
            }]),
        );
        let second = UiSession::new(UiSessionId(2), "Second", "second");
        state.sessions.sessions.push(second);

        state.handle_ui_action(UiAction::Activated(edgerun_ui_core::gpu::GpuHit::new(
            HitKind::Button,
            SESSION_ROW_BASE_ID + 1,
            0.0,
            0.0,
            1.0,
            1.0,
        )));

        assert_eq!(state.sessions.selected, UiSessionId(2));
        assert_eq!(state.sessions.selected().title, "Second");
        assert_eq!(state.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID), 1.0);
    }

    #[test]
    fn switching_session_dismisses_pending_clear_confirmation() {
        let mut state = test_state(
            "Ready",
            false,
            UiRuntimeState::default(),
            test_session(vec![]),
        );
        let first_id = state.sessions.selected;
        let second = UiSession::new(UiSessionId(2), "Second", "second");
        state.sessions.sessions.push(second);

        state.request_clear_transcript();
        assert_eq!(state.clear_confirm_session, Some(first_id));
        assert_eq!(state.sessions.selected().status, "Confirm clear");

        state.switch_session(1);

        let first = state
            .sessions
            .sessions
            .iter()
            .find(|session| session.id == first_id)
            .expect("first session exists");
        assert_eq!(state.clear_confirm_session, None);
        assert_eq!(first.status, "Ready");
        assert_eq!(state.sessions.selected, UiSessionId(2));
    }

    #[test]
    fn ctrl_page_keys_switch_sessions_without_pointer() {
        let mut state = test_state(
            "Ready",
            false,
            UiRuntimeState::default(),
            test_session(vec![]),
        );
        state
            .sessions
            .sessions
            .push(UiSession::new(UiSessionId(2), "Second", "second"));
        state
            .sessions
            .sessions
            .push(UiSession::new(UiSessionId(3), "Third", "third"));
        let ctrl = UiKeyModifiers {
            ctrl: true,
            ..UiKeyModifiers::default()
        };

        assert!(state.handle_session_key(UiKey::PageDown, ctrl));
        assert_eq!(state.sessions.selected, UiSessionId(2));
        assert_eq!(state.status, "Ready");

        assert!(state.handle_session_key(UiKey::PageDown, ctrl));
        assert_eq!(state.sessions.selected, UiSessionId(3));

        assert!(state.handle_session_key(UiKey::PageDown, ctrl));
        assert_eq!(state.sessions.selected, UiSessionId(1));

        assert!(state.handle_session_key(UiKey::PageUp, ctrl));
        assert_eq!(state.sessions.selected, UiSessionId(3));
    }

    #[test]
    fn command_keys_cover_main_chrome_actions() {
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        let (_event_tx, event_rx) = std::sync::mpsc::channel();
        let session_id = UiSessionId(1);
        let mut state = CodexUi {
            model: "gpt-5.5".to_string(),
            input: UiTextBuffer::new(),
            status: "Ready".to_string(),
            busy: false,
            animation_tick: 0,
            runtime: UiRuntimeState::default(),
            sessions: SessionStore {
                sessions: vec![UiSession::new(session_id, "Work", "ready")],
                selected: session_id,
                next_id: 2,
            },
            clear_confirm_session: None,
            inspector_open: false,
            inspector_panel: InspectorPanel::Usage,
            log_level: LogLevel::Normal,
            confirm_clear_enabled: true,
            density: DensityMode::Comfortable,
            logs: Vec::new(),
            prompt_history: Vec::new(),
            prompt_history_index: None,
            prompt_history_path: None,
            command_tx,
            event_rx,
        };
        let ctrl = UiKeyModifiers {
            ctrl: true,
            ..UiKeyModifiers::default()
        };
        let ctrl_shift = UiKeyModifiers {
            ctrl: true,
            shift: true,
            ..UiKeyModifiers::default()
        };

        assert!(state.handle_command_key(UiKey::Other(b',' as u32), ctrl));
        assert!(state.inspector_open);
        assert_eq!(state.inspector_panel, InspectorPanel::Settings);

        assert!(state.handle_command_key(UiKey::Other(b'.' as u32), ctrl));
        assert_eq!(state.inspector_panel, InspectorPanel::Usage);

        assert!(state.handle_command_key(UiKey::Other(b'L' as u32), ctrl_shift));
        assert_eq!(state.inspector_panel, InspectorPanel::Logs);

        assert!(state.handle_command_key(UiKey::Backspace, ctrl_shift));
        assert_eq!(state.clear_confirm_session, Some(session_id));

        assert!(state.handle_command_key(UiKey::Other(b'N' as u32), ctrl_shift));
        match command_rx.recv().expect("new chat sends reset") {
            WorkerCommand::Reset { session_id } => assert_eq!(session_id, state.sessions.selected),
            command => panic!("unexpected command: {command:?}"),
        }
        assert_eq!(state.sessions.sessions.len(), 2);
    }

    #[test]
    fn background_session_events_update_their_own_transcript() {
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.25);
        let mut state = test_state(
            "Ready",
            false,
            runtime,
            test_session(vec![Message {
                role: Role::Assistant,
                text: "foreground".to_string(),
            }]),
        );
        let background_id = UiSessionId(2);
        state
            .sessions
            .sessions
            .push(UiSession::new(background_id, "Background", "background"));

        state.apply_session_event(
            background_id,
            SessionWorkerEvent::AssistantTextDelta("working".to_string()),
        );

        assert_eq!(state.sessions.selected, UiSessionId(1));
        assert_eq!(state.status, "Ready");
        assert_eq!(state.sessions.selected().messages[0].text, "foreground");
        let background = state
            .sessions
            .sessions
            .iter()
            .find(|session| session.id == background_id)
            .expect("background session should exist");
        assert_eq!(background.messages.len(), 2);
        assert_eq!(background.messages[1].role, Role::Assistant);
        assert_eq!(background.messages[1].text, "Response\nworking");
        assert_eq!(state.runtime.scroll_offset(TRANSCRIPT_SCROLL_ID), 0.25);
    }

    #[test]
    fn submit_targets_selected_idle_session_while_background_session_runs() {
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        let (_event_tx, event_rx) = std::sync::mpsc::channel();
        let foreground_id = UiSessionId(1);
        let background_id = UiSessionId(2);
        let mut foreground = UiSession::new(foreground_id, "Foreground", "ready");
        let mut background = UiSession::new(background_id, "Background", "busy");
        background.busy = true;
        background.status = "Thinking".to_string();
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(TRANSCRIPT_SCROLL_ID, 0.0);
        let mut state = CodexUi {
            model: "gpt-5.5".to_string(),
            input: UiTextBuffer::new(),
            status: "Ready".to_string(),
            busy: false,
            animation_tick: 0,
            runtime,
            sessions: SessionStore {
                sessions: vec![foreground.clone(), background],
                selected: foreground_id,
                next_id: 3,
            },
            clear_confirm_session: None,
            inspector_open: false,
            inspector_panel: InspectorPanel::Usage,
            log_level: LogLevel::Normal,
            confirm_clear_enabled: true,
            density: DensityMode::Comfortable,
            logs: Vec::new(),
            prompt_history: Vec::new(),
            prompt_history_index: None,
            prompt_history_path: None,
            command_tx,
            event_rx,
        };
        state.input.set_text("run this in foreground");

        state.submit();

        foreground = state.sessions.selected().clone();
        assert!(foreground.busy);
        assert_eq!(foreground.turns, 1);
        assert_eq!(foreground.title, "run this in foreground");
        let background = state
            .sessions
            .sessions
            .iter()
            .find(|session| session.id == background_id)
            .expect("background session should remain");
        assert!(background.busy);
        let command = command_rx
            .recv()
            .expect("submit should send worker command");
        match command {
            WorkerCommand::Prompt { session_id, prompt } => {
                assert_eq!(session_id, foreground_id);
                assert_eq!(prompt, "run this in foreground");
            }
            WorkerCommand::Reset { .. } => panic!("submit should not reset"),
            WorkerCommand::Cancel { .. } => panic!("submit should not cancel"),
        }
    }

    #[test]
    fn cancel_selected_session_sends_worker_cancel_and_marks_status() {
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        let (_event_tx, event_rx) = std::sync::mpsc::channel();
        let session_id = UiSessionId(1);
        let mut session = UiSession::new(session_id, "Work", "Thinking");
        session.busy = true;
        let mut state = CodexUi {
            model: "gpt-5.5".to_string(),
            input: UiTextBuffer::new(),
            status: "Thinking".to_string(),
            busy: false,
            animation_tick: 0,
            runtime: UiRuntimeState::default(),
            sessions: SessionStore {
                sessions: vec![session],
                selected: session_id,
                next_id: 2,
            },
            clear_confirm_session: Some(session_id),
            inspector_open: false,
            inspector_panel: InspectorPanel::Usage,
            log_level: LogLevel::Normal,
            confirm_clear_enabled: true,
            density: DensityMode::Comfortable,
            logs: Vec::new(),
            prompt_history: Vec::new(),
            prompt_history_index: None,
            prompt_history_path: None,
            command_tx,
            event_rx,
        };

        state.cancel_selected_session();

        assert_eq!(state.status, "Cancelling");
        assert_eq!(state.clear_confirm_session, None);
        assert_eq!(state.sessions.selected().status, "Cancelling");
        match command_rx
            .recv()
            .expect("cancel should send worker command")
        {
            WorkerCommand::Cancel { session_id: sent } => assert_eq!(sent, session_id),
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn cancelled_event_marks_session_idle_without_counting_failure() {
        let mut session = test_session(vec![Message {
            role: Role::User,
            text: "stop me".to_string(),
        }]);
        session.busy = true;
        session.failures = 0;
        session.active_tool = Some("shell".to_string());
        let mut state = test_state("Thinking", false, UiRuntimeState::default(), session);

        state.apply_session_event(UiSessionId(1), SessionWorkerEvent::Cancelled);

        let session = state.sessions.selected();
        assert!(!session.busy);
        assert_eq!(session.failures, 0);
        assert_eq!(session.active_tool, None);
        assert_eq!(session.status, "Stopped");
        assert_eq!(
            session.messages.last().map(|message| message.role),
            Some(Role::ToolError)
        );
        assert_eq!(state.status, "Stopped");
    }

    #[test]
    fn session_sidebar_detail_surfaces_running_and_usage_state() {
        let mut session = UiSession::new(UiSessionId(1), "Work", "ready");
        assert_eq!(session_sidebar_detail(&session), "new | usage pending");

        session.turns = 1;
        session.tools_run = 2;
        assert_eq!(
            session_sidebar_detail(&session),
            "1 turn | 2 tools | awaiting usage"
        );

        session.busy = true;
        session.status = "Reasoning".to_string();
        session.active_tool = Some("shell".to_string());
        assert_eq!(
            session_sidebar_detail(&session),
            "running shell | 1 turn | 2 tools"
        );

        session.busy = false;
        session.active_tool = None;
        session.failures = 1;
        assert_eq!(
            session_sidebar_detail(&session),
            "1 turn | 1 failure | usage pending"
        );
        assert_eq!(session_sidebar_state(&session), UiShadcnSessionState::Error);
    }

    #[test]
    fn session_store_aggregates_usage_and_running_sessions() {
        let mut sessions = SessionStore::initial();
        sessions.selected_mut().usage.record(TokenUsage {
            input_tokens: 100,
            cached_input_tokens: 20,
            output_tokens: 30,
            reasoning_output_tokens: 12,
            total_tokens: 130,
        });
        let second_id = sessions.create_session();
        {
            let second = sessions.session_mut(second_id).expect("second session");
            second.busy = true;
            second.usage.record(TokenUsage {
                input_tokens: 80,
                cached_input_tokens: 0,
                output_tokens: 20,
                reasoning_output_tokens: 8,
                total_tokens: 100,
            });
        }

        let total = sessions.aggregate_usage();
        assert_eq!(sessions.active_count(), 1);
        assert_eq!(total.blended_total(), 210);
        assert_eq!(total.reasoning_output_tokens, 20);
    }

    #[test]
    fn rate_limit_state_surfaces_primary_window_and_credits() {
        let mut state = RateLimitState::default();
        assert_eq!(state.label(), "limits pending");

        state.record(RateLimitSnapshot {
            limit_id: None,
            limit_name: Some("daily".to_string()),
            primary: Some(codex_core::protocol::protocol::RateLimitWindow {
                used_percent: 42.4,
                window_minutes: Some(1440),
                resets_at: None,
            }),
            secondary: None,
            credits: None,
            plan_type: None,
            rate_limit_reached_type: None,
        });
        assert_eq!(state.label(), "daily 42% used");

        state.record(RateLimitSnapshot {
            limit_id: None,
            limit_name: None,
            primary: None,
            secondary: None,
            credits: Some(codex_core::protocol::protocol::CreditsSnapshot {
                has_credits: true,
                unlimited: false,
                balance: Some("12.50".to_string()),
            }),
            plan_type: None,
            rate_limit_reached_type: None,
        });
        assert_eq!(state.label(), "credits 12.50");
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
