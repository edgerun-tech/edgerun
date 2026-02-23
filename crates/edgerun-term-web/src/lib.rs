// SPDX-License-Identifier: Apache-2.0
#[cfg(not(target_arch = "wasm32"))]
pub fn start() {}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::{self, Write};
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use js_sys::Uint8Array;
    use term_core::render::{
        FONT_DATA, FONT_SIZE, GlyphCache, draw_background, draw_cursor_overlay, draw_grid,
        layout::compute_layout,
    };
    use term_core::terminal::{GridPerformer, Terminal};
    use vte::Parser as VteParser;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::{Clamped, JsCast};
    #[cfg(feature = "webgpu")]
    use wasm_bindgen_futures::spawn_local;
    use web_sys::{
        CanvasRenderingContext2d, ClipboardEvent, CompositionEvent, Document, Element,
        HtmlCanvasElement, HtmlTextAreaElement, ImageData, InputEvent, KeyboardEvent, MessageEvent,
        WebSocket,
    };
    #[cfg(feature = "webgpu")]
    use wgpu::SurfaceError;
    #[cfg(feature = "webgpu")]
    use wgpu::util::DeviceExt;

    const BLINK_INTERVAL: Duration = Duration::from_millis(700);
    const PTY_FRAME_STDIN: u8 = 1;
    const PTY_FRAME_STDOUT: u8 = 2;
    const PTY_FRAME_CONTROL_REQ: u8 = 0x7e;
    const PTY_FRAME_CONTROL_RESP: u8 = 0x7f;

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum WsTransport {
        Raw,
        Mux,
    }

    #[derive(Debug)]
    enum ShellRequest {
        Auth {
            token: String,
        },
        Spawn {
            id: Option<u32>,
            cmd: Option<String>,
            args: Option<Vec<String>>,
            cwd: Option<String>,
            env: Option<HashMap<String, String>>,
            cols: Option<u16>,
            rows: Option<u16>,
        },
        Resize {
            id: u32,
            cols: u16,
            rows: u16,
        },
        Close {
            id: u32,
        },
    }

    #[derive(Debug)]
    enum ShellResponse {
        AuthOk,
        AuthError { error: String },
        Spawned { id: u32, pid: Option<u32> },
        Exit { id: u32, code: u32, signal: Option<String> },
        Error { id: Option<u32>, error: String },
    }

    #[cfg(feature = "webgpu")]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    #[repr(C)]
    #[allow(dead_code)]
    struct Vertex {
        pos: [f32; 2],
        uv: [f32; 2],
    }

    #[cfg(feature = "webgpu")]
    #[allow(dead_code)]
    impl Vertex {
        fn desc() -> wgpu::VertexBufferLayout<'static> {
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                    wgpu::VertexAttribute {
                        offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                ],
            }
        }
    }

    #[derive(Clone)]
    #[allow(dead_code)]
    enum RenderBackend {
        #[cfg(feature = "webgpu")]
        WebGpu(Rc<RefCell<GpuState>>),
        Canvas2d(CanvasRenderingContext2d),
    }

    type SharedRenderBackend = Rc<RefCell<RenderBackend>>;

    #[cfg(feature = "webgpu")]
    const WEBGPU_STATE_KEY: &str = "edgerun.term.webgpu.state.v1";
    #[cfg(feature = "webgpu")]
    const WEBGPU_CRASH_KEY: &str = "edgerun.term.webgpu.inflight";
    #[cfg(feature = "webgpu")]
    const WEBGPU_BASE_COOLDOWN_MS: f64 = 30_000.0;
    #[cfg(feature = "webgpu")]
    const WEBGPU_MAX_COOLDOWN_MS: f64 = 900_000.0;

    #[cfg(feature = "webgpu")]
    #[derive(Clone, Debug, Default)]
    struct WebGpuRuntimeState {
        consecutive_failures: u32,
        total_failures: u32,
        total_successes: u32,
        disabled_until_ms: f64,
        last_failure_ms: f64,
    }

    #[cfg(feature = "webgpu")]
    struct GpuState {
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
        pipeline: wgpu::RenderPipeline,
        bind_group_layout: wgpu::BindGroupLayout,
        bind_group: wgpu::BindGroup,
        vertex_buffer: wgpu::Buffer,
        texture: wgpu::Texture,
        texture_view: wgpu::TextureView,
        sampler: wgpu::Sampler,
        padded_upload: Vec<u8>,
        _canvas: HtmlCanvasElement,
    }

    struct AppState {
        terminal: Terminal,
        parser: VteParser,
        app_cursor_keys: bool,
        outbox: Arc<Mutex<Vec<Vec<u8>>>>,
        glyphs: GlyphCache,
        cell_w: u32,
        cell_h: u32,
        layout: term_core::render::layout::LayoutMetrics,
        frame: Vec<u8>,
        image_data: Option<ImageData>,
        width: u32,
        height: u32,
        last_blink: Instant,
        blink_on: bool,
        ws: Option<WebSocket>,
        transport: WsTransport,
        allow_raw_fallback: bool,
        pane_id: Option<String>,
        mux_token: Option<String>,
        mux_session_id: Option<u32>,
        mux_failures: u8,
        pending_input: Vec<Vec<u8>>,
        reconnect_attempts: u32,
        status_el: Option<Element>,
    }

    struct OutboxWriter {
        queue: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl Write for OutboxWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.queue.lock().unwrap().push(buf.to_vec());
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn set_status(status_el: &Option<Element>, message: Option<&str>) {
        if let Some(el) = status_el {
            if let Some(msg) = message {
                el.remove_attribute("hidden").ok();
                if let Ok(Some(text_el)) = el.query_selector("#status-text") {
                    text_el.set_text_content(Some(msg));
                } else {
                    el.set_text_content(Some(msg));
                }
            } else {
                el.set_attribute("hidden", "true").ok();
                el.set_text_content(None);
            }
        }
    }

    #[cfg(feature = "webgpu")]
    fn now_ms() -> f64 {
        js_sys::Date::now()
    }

    #[cfg(feature = "webgpu")]
    fn local_storage(window: &web_sys::Window) -> Option<web_sys::Storage> {
        window.local_storage().ok().flatten()
    }

    #[cfg(feature = "webgpu")]
    fn parse_webgpu_state(raw: &str) -> Option<WebGpuRuntimeState> {
        let mut parts = raw.split(',');
        let version = parts.next()?;
        if version != "v1" {
            return None;
        }
        Some(WebGpuRuntimeState {
            consecutive_failures: parts.next()?.parse().ok()?,
            total_failures: parts.next()?.parse().ok()?,
            total_successes: parts.next()?.parse().ok()?,
            disabled_until_ms: parts.next()?.parse().ok()?,
            last_failure_ms: parts.next()?.parse().ok()?,
        })
    }

    #[cfg(feature = "webgpu")]
    fn format_webgpu_state(state: &WebGpuRuntimeState) -> String {
        format!(
            "v1,{},{},{},{},{}",
            state.consecutive_failures,
            state.total_failures,
            state.total_successes,
            state.disabled_until_ms,
            state.last_failure_ms
        )
    }

    #[cfg(feature = "webgpu")]
    fn load_webgpu_state(window: &web_sys::Window) -> WebGpuRuntimeState {
        let Some(storage) = local_storage(window) else {
            return WebGpuRuntimeState::default();
        };
        let Ok(Some(raw)) = storage.get_item(WEBGPU_STATE_KEY) else {
            return WebGpuRuntimeState::default();
        };
        parse_webgpu_state(&raw).unwrap_or_default()
    }

    #[cfg(feature = "webgpu")]
    fn save_webgpu_state(window: &web_sys::Window, state: &WebGpuRuntimeState) {
        if let Some(storage) = local_storage(window) {
            let _ = storage.set_item(WEBGPU_STATE_KEY, &format_webgpu_state(state));
        }
    }

    #[cfg(feature = "webgpu")]
    fn set_webgpu_inflight(window: &web_sys::Window, inflight: bool) {
        if let Some(storage) = local_storage(window) {
            if inflight {
                let _ = storage.set_item(WEBGPU_CRASH_KEY, "1");
            } else {
                let _ = storage.remove_item(WEBGPU_CRASH_KEY);
            }
        }
    }

    #[cfg(feature = "webgpu")]
    fn consume_previous_crash_flag(window: &web_sys::Window) -> bool {
        let Some(storage) = local_storage(window) else {
            return false;
        };
        let crashed = matches!(storage.get_item(WEBGPU_CRASH_KEY), Ok(Some(flag)) if flag == "1");
        if crashed {
            let _ = storage.remove_item(WEBGPU_CRASH_KEY);
        }
        crashed
    }

    #[cfg(feature = "webgpu")]
    fn webgpu_cooldown_ms(consecutive_failures: u32) -> f64 {
        if consecutive_failures == 0 {
            return 0.0;
        }
        let exp = consecutive_failures.saturating_sub(1).min(8);
        (WEBGPU_BASE_COOLDOWN_MS * (1u64 << exp) as f64).min(WEBGPU_MAX_COOLDOWN_MS)
    }

    #[cfg(feature = "webgpu")]
    fn record_webgpu_failure(window: &web_sys::Window, state: &mut WebGpuRuntimeState) -> f64 {
        state.total_failures = state.total_failures.saturating_add(1);
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        state.last_failure_ms = now_ms();
        let cooldown = webgpu_cooldown_ms(state.consecutive_failures);
        state.disabled_until_ms = state.last_failure_ms + cooldown;
        save_webgpu_state(window, state);
        cooldown
    }

    #[cfg(feature = "webgpu")]
    fn record_webgpu_success(window: &web_sys::Window, state: &mut WebGpuRuntimeState) {
        state.total_successes = state.total_successes.saturating_add(1);
        state.consecutive_failures = 0;
        state.disabled_until_ms = 0.0;
        save_webgpu_state(window, state);
    }

    #[cfg(feature = "webgpu")]
    fn should_attempt_webgpu(window: &web_sys::Window) -> (bool, String, WebGpuRuntimeState) {
        let navigator = window.navigator();
        let has_gpu = js_sys::Reflect::has(&navigator, &JsValue::from_str("gpu")).unwrap_or(false);
        if !has_gpu {
            return (
                false,
                "WebGPU unavailable in this browser; using 2D canvas".to_string(),
                WebGpuRuntimeState::default(),
            );
        }

        let mut state = load_webgpu_state(window);
        if consume_previous_crash_flag(window) {
            let cooldown = record_webgpu_failure(window, &mut state);
            let seconds = (cooldown / 1000.0).round() as u64;
            return (
                false,
                format!("WebGPU crashed last run; using 2D canvas for {seconds}s cooldown"),
                state,
            );
        }

        let now = now_ms();
        if state.disabled_until_ms > now {
            let remaining = ((state.disabled_until_ms - now) / 1000.0).ceil().max(1.0) as u64;
            return (
                false,
                format!("WebGPU cooldown active ({remaining}s); using 2D canvas"),
                state,
            );
        }

        (true, "Attempting WebGPU backend…".to_string(), state)
    }

    fn init_canvas_2d(canvas: &HtmlCanvasElement) -> Result<CanvasRenderingContext2d, JsValue> {
        let context = canvas
            .get_context("2d")?
            .ok_or("missing 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()?;
        context.set_image_smoothing_enabled(false);
        Ok(context)
    }

    fn query_param(search: &str, key: &str) -> Option<String> {
        let trimmed = search.strip_prefix('?').unwrap_or(search);
        for pair in trimmed.split('&') {
            if pair.is_empty() {
                continue;
            }
            let mut parts = pair.splitn(2, '=');
            let raw_key = parts.next().unwrap_or_default();
            if raw_key != key {
                continue;
            }
            let raw_value = parts.next().unwrap_or_default();
            return js_sys::decode_uri_component(raw_value)
                .ok()
                .and_then(|value| value.as_string())
                .or_else(|| Some(raw_value.to_string()));
        }
        None
    }

    fn send_mux_request(ws: &WebSocket, request: ShellRequest) {
        if let Ok(encoded) = encode_shell_request(&request) {
            let mut frame = Vec::with_capacity(1 + encoded.len());
            frame.push(PTY_FRAME_CONTROL_REQ);
            frame.extend_from_slice(&encoded);
            let _ = ws.send_with_u8_array(&frame);
        }
    }

    fn send_mux_auth(ws: &WebSocket, token: &str) {
        send_mux_request(
            ws,
            ShellRequest::Auth {
                token: token.to_string(),
            },
        );
    }

    fn send_mux_spawn(ws: &WebSocket, cols: usize, rows: usize) {
        send_mux_request(
            ws,
            ShellRequest::Spawn {
                id: None,
                cmd: None,
                args: None,
                cwd: None,
                env: None,
                cols: u16::try_from(cols).ok(),
                rows: u16::try_from(rows).ok(),
            },
        );
    }

    fn send_mux_resize(ws: &WebSocket, session_id: u32, cols: usize, rows: usize) {
        let (Some(cols), Some(rows)) = (u16::try_from(cols).ok(), u16::try_from(rows).ok()) else {
            return;
        };
        send_mux_request(
            ws,
            ShellRequest::Resize {
                id: session_id,
                cols,
                rows,
            },
        );
    }

    fn handle_mux_control_response(state: &mut AppState, value: ShellResponse) {
        match value {
            ShellResponse::Spawned { id, .. } => {
                state.mux_session_id = Some(id);
                state.mux_failures = 0;
                if let Some(ws) = state.ws.clone() {
                    send_mux_resize(&ws, id, state.layout.cols, state.layout.rows);
                    flush_pending_input(state, &ws);
                }
                state.reconnect_attempts = 0;
                set_status(&state.status_el, None);
            }
            ShellResponse::AuthError { error } | ShellResponse::Error { error, .. } => {
                if state.transport == WsTransport::Mux
                    && state.allow_raw_fallback
                    && state.mux_failures < u8::MAX
                {
                    state.mux_failures = state.mux_failures.saturating_add(1);
                }
                set_status(&state.status_el, Some(&error));
            }
            ShellResponse::Exit { .. } => {
                state.mux_session_id = None;
                set_status(&state.status_el, Some("Shell exited"));
            }
            ShellResponse::AuthOk => {}
        }
    }

    fn encode_stdin_frame(session_id: u32, bytes: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(5 + bytes.len());
        frame.push(PTY_FRAME_STDIN);
        frame.extend_from_slice(&session_id.to_be_bytes());
        frame.extend_from_slice(bytes);
        frame
    }

    fn feed_terminal_bytes(
        state: &mut AppState,
        writer: Arc<Mutex<Box<dyn Write + Send>>>,
        data: &[u8],
    ) {
        let mut performer = GridPerformer {
            grid: &mut state.terminal,
            writer,
            app_cursor_keys: &mut state.app_cursor_keys,
            dcs_state: None,
        };
        for byte in data {
            state.parser.advance(&mut performer, *byte);
        }
    }

    fn encode_shell_request(request: &ShellRequest) -> Result<Vec<u8>, JsValue> {
        let mut out = Vec::new();
        match request {
            ShellRequest::Auth { token } => {
                out.push(0);
                put_str(&mut out, token)?;
            }
            ShellRequest::Spawn {
                id,
                cmd,
                args,
                cwd,
                env,
                cols,
                rows,
            } => {
                out.push(1);
                let mut flags = 0u8;
                if id.is_some() {
                    flags |= 1 << 0;
                }
                if cmd.is_some() {
                    flags |= 1 << 1;
                }
                if args.is_some() {
                    flags |= 1 << 2;
                }
                if cwd.is_some() {
                    flags |= 1 << 3;
                }
                if env.is_some() {
                    flags |= 1 << 4;
                }
                if cols.is_some() {
                    flags |= 1 << 5;
                }
                if rows.is_some() {
                    flags |= 1 << 6;
                }
                out.push(flags);
                if let Some(id) = id {
                    out.extend_from_slice(&id.to_be_bytes());
                }
                if let Some(cmd) = cmd {
                    put_str(&mut out, cmd)?;
                }
                if let Some(args) = args {
                    let count =
                        u16::try_from(args.len()).map_err(|_| JsValue::from_str("too many args"))?;
                    out.extend_from_slice(&count.to_be_bytes());
                    for value in args {
                        put_str(&mut out, value)?;
                    }
                }
                if let Some(cwd) = cwd {
                    put_str(&mut out, cwd)?;
                }
                if let Some(env) = env {
                    let count = u16::try_from(env.len())
                        .map_err(|_| JsValue::from_str("too many env vars"))?;
                    out.extend_from_slice(&count.to_be_bytes());
                    for (key, value) in env {
                        put_str(&mut out, key)?;
                        put_str(&mut out, value)?;
                    }
                }
                if let Some(cols) = cols {
                    out.extend_from_slice(&cols.to_be_bytes());
                }
                if let Some(rows) = rows {
                    out.extend_from_slice(&rows.to_be_bytes());
                }
            }
            ShellRequest::Resize { id, cols, rows } => {
                out.push(2);
                out.extend_from_slice(&id.to_be_bytes());
                out.extend_from_slice(&cols.to_be_bytes());
                out.extend_from_slice(&rows.to_be_bytes());
            }
            ShellRequest::Close { id } => {
                out.push(3);
                out.extend_from_slice(&id.to_be_bytes());
            }
        }
        Ok(out)
    }

    fn decode_shell_response(bytes: &[u8]) -> Result<ShellResponse, JsValue> {
        let mut cur = 0usize;
        let tag = take_u8(bytes, &mut cur)?;
        match tag {
            0 => Ok(ShellResponse::AuthOk),
            1 => Ok(ShellResponse::AuthError {
                error: take_str(bytes, &mut cur)?,
            }),
            2 => {
                let id = take_u32(bytes, &mut cur)?;
                let has_pid = take_u8(bytes, &mut cur)? != 0;
                let pid = if has_pid {
                    Some(take_u32(bytes, &mut cur)?)
                } else {
                    None
                };
                Ok(ShellResponse::Spawned { id, pid })
            }
            3 => {
                let id = take_u32(bytes, &mut cur)?;
                let code = take_u32(bytes, &mut cur)?;
                let has_signal = take_u8(bytes, &mut cur)? != 0;
                let signal = if has_signal {
                    Some(take_str(bytes, &mut cur)?)
                } else {
                    None
                };
                Ok(ShellResponse::Exit { id, code, signal })
            }
            4 => {
                let has_id = take_u8(bytes, &mut cur)? != 0;
                let id = if has_id {
                    Some(take_u32(bytes, &mut cur)?)
                } else {
                    None
                };
                let error = take_str(bytes, &mut cur)?;
                Ok(ShellResponse::Error { id, error })
            }
            _ => Err(JsValue::from_str("unknown response tag")),
        }
    }

    fn put_str(out: &mut Vec<u8>, value: &str) -> Result<(), JsValue> {
        let len = u16::try_from(value.len()).map_err(|_| JsValue::from_str("string too long"))?;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(value.as_bytes());
        Ok(())
    }

    fn take_u8(bytes: &[u8], cur: &mut usize) -> Result<u8, JsValue> {
        let Some(value) = bytes.get(*cur).copied() else {
            return Err(JsValue::from_str("unexpected eof"));
        };
        *cur += 1;
        Ok(value)
    }

    fn take_u16(bytes: &[u8], cur: &mut usize) -> Result<u16, JsValue> {
        if bytes.len().saturating_sub(*cur) < 2 {
            return Err(JsValue::from_str("unexpected eof"));
        }
        let value = u16::from_be_bytes([bytes[*cur], bytes[*cur + 1]]);
        *cur += 2;
        Ok(value)
    }

    fn take_u32(bytes: &[u8], cur: &mut usize) -> Result<u32, JsValue> {
        if bytes.len().saturating_sub(*cur) < 4 {
            return Err(JsValue::from_str("unexpected eof"));
        }
        let value = u32::from_be_bytes([bytes[*cur], bytes[*cur + 1], bytes[*cur + 2], bytes[*cur + 3]]);
        *cur += 4;
        Ok(value)
    }

    fn take_str(bytes: &[u8], cur: &mut usize) -> Result<String, JsValue> {
        let len = take_u16(bytes, cur)? as usize;
        if bytes.len().saturating_sub(*cur) < len {
            return Err(JsValue::from_str("unexpected eof"));
        }
        let slice = &bytes[*cur..*cur + len];
        *cur += len;
        String::from_utf8(slice.to_vec()).map_err(|_| JsValue::from_str("invalid utf8"))
    }

    fn flush_pending_input(state: &mut AppState, ws: &WebSocket) {
        if state.pending_input.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut state.pending_input);
        match state.transport {
            WsTransport::Raw => {
                for chunk in pending {
                    let _ = ws.send_with_u8_array(&chunk);
                }
            }
            WsTransport::Mux => {
                let Some(session_id) = state.mux_session_id else {
                    state.pending_input = pending;
                    return;
                };
                for chunk in pending {
                    let frame = encode_stdin_frame(session_id, &chunk);
                    let _ = ws.send_with_u8_array(&frame);
                }
            }
        }
    }

    fn post_transport_status(
        window: &web_sys::Window,
        pane_id: Option<&str>,
        transport: WsTransport,
    ) {
        let Ok(Some(parent)) = window.parent() else {
            return;
        };
        let payload = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
            &payload,
            &JsValue::from_str("source"),
            &JsValue::from_str("edgerun-term-web"),
        );
        let _ = js_sys::Reflect::set(
            &payload,
            &JsValue::from_str("type"),
            &JsValue::from_str("transport"),
        );
        let transport_value = match transport {
            WsTransport::Mux => "mux",
            WsTransport::Raw => "raw",
        };
        let _ = js_sys::Reflect::set(
            &payload,
            &JsValue::from_str("transport"),
            &JsValue::from_str(transport_value),
        );
        if let Some(pane_id) = pane_id {
            let _ = js_sys::Reflect::set(
                &payload,
                &JsValue::from_str("sid"),
                &JsValue::from_str(pane_id),
            );
        }
        let _ = parent.post_message(&payload, "*");
    }

    fn queue_or_send(state: &Rc<RefCell<AppState>>, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let (ws, transport, session_id) = {
            let state = state.borrow();
            (state.ws.clone(), state.transport, state.mux_session_id)
        };
        if let Some(ws) = ws {
            match transport {
                WsTransport::Raw => {
                    let _ = ws.send_with_u8_array(bytes);
                }
                WsTransport::Mux => {
                    if let Some(id) = session_id {
                        let frame = encode_stdin_frame(id, bytes);
                        let _ = ws.send_with_u8_array(&frame);
                    } else {
                        state.borrow_mut().pending_input.push(bytes.to_vec());
                    }
                }
            }
            return;
        }
        state.borrow_mut().pending_input.push(bytes.to_vec());
    }

    fn schedule_reconnect(
        window: &web_sys::Window,
        state: Rc<RefCell<AppState>>,
        writer: Arc<Mutex<Box<dyn Write + Send>>>,
        delay_ms: u64,
    ) {
        let window_clone = window.clone();
        let state_clone = state.clone();
        let writer_clone = writer.clone();
        let cb = Closure::<dyn FnMut()>::new(move || {
            let _ = setup_websocket(&window_clone, state_clone.clone(), writer_clone.clone());
        });
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            delay_ms as i32,
        );
        cb.forget();
    }

    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        console_error_panic_hook::set_once();

        let window = web_sys::window().ok_or("missing window")?;
        let document: Document = window.document().ok_or("missing document")?;
        let status_el = document.get_element_by_id("status");
        set_status(&status_el, Some("Starting web terminal…"));
        let canvas = document
            .get_element_by_id("term-canvas")
            .ok_or("missing #term-canvas")?
            .dyn_into::<HtmlCanvasElement>()?;

        let (width, height) = canvas_size(&canvas, &window);
        canvas.set_width(width);
        canvas.set_height(height);

        #[cfg(feature = "webgpu")]
        let render_backend: SharedRenderBackend = Rc::new(RefCell::new(RenderBackend::Canvas2d(
            init_canvas_2d(&canvas)?,
        )));
        #[cfg(not(feature = "webgpu"))]
        let render_backend: SharedRenderBackend = Rc::new(RefCell::new({
            set_status(&status_el, Some("WebGPU feature disabled, using 2D canvas"));
            RenderBackend::Canvas2d(init_canvas_2d(&canvas)?)
        }));

        #[cfg(feature = "webgpu")]
        {
            let (should_try, reason, mut runtime_state) = should_attempt_webgpu(&window);
            set_status(&status_el, Some(&reason));
            if should_try {
                let canvas_for_gpu = canvas.clone();
                let status_for_gpu = status_el.clone();
                let backend_for_gpu = render_backend.clone();
                let window_for_gpu = window.clone();
                set_webgpu_inflight(&window, true);
                spawn_local(async move {
                    match init_gpu(&canvas_for_gpu, width, height).await {
                        Ok((gpu, _format)) => {
                            set_webgpu_inflight(&window_for_gpu, false);
                            record_webgpu_success(&window_for_gpu, &mut runtime_state);
                            *backend_for_gpu.borrow_mut() =
                                RenderBackend::WebGpu(Rc::new(RefCell::new(gpu)));
                            set_status(&status_for_gpu, Some("WebGPU active"));
                        }
                        Err(err) => {
                            set_webgpu_inflight(&window_for_gpu, false);
                            let cooldown_ms =
                                record_webgpu_failure(&window_for_gpu, &mut runtime_state);
                            let cooldown_s = (cooldown_ms / 1000.0).round() as u64;
                            web_sys::console::warn_1(&JsValue::from_str(&format!(
                                "WebGPU init failed: {err:?}"
                            )));
                            set_status(
                                &status_for_gpu,
                                Some(&format!(
                                    "WebGPU init failed; using 2D canvas ({cooldown_s}s cooldown)"
                                )),
                            );
                        }
                    }
                });
            }
        }

        let primary = std::sync::Arc::new(FONT_DATA.to_vec());
        let glyphs = GlyphCache::new(primary, FONT_SIZE);
        let (cell_w, cell_h) = glyphs.cell_size();

        let layout = compute_layout(width, height, cell_w, cell_h, 0, 0, 0, 0, 0);
        let terminal = Terminal::new(layout.cols, layout.rows);

        let outbox = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(OutboxWriter {
                queue: outbox.clone(),
            })));
        let location_search = window.location().search().unwrap_or_default();
        let transport = match query_param(&location_search, "transport").as_deref() {
            Some("mux") => WsTransport::Mux,
            _ => WsTransport::Raw,
        };
        let allow_raw_fallback = !matches!(
            query_param(&location_search, "fallback_raw").as_deref(),
            Some("0") | Some("false") | Some("off")
        );
        let pane_id = query_param(&location_search, "sid");
        let mux_token = query_param(&location_search, "mux_token")
            .or_else(|| query_param(&location_search, "token"));

        let app_state = Rc::new(RefCell::new(AppState {
            terminal,
            parser: VteParser::new(),
            app_cursor_keys: false,
            outbox,
            glyphs,
            cell_w,
            cell_h,
            layout,
            frame: vec![0u8; (width * height * 4) as usize],
            image_data: None,
            width,
            height,
            last_blink: Instant::now(),
            blink_on: true,
            ws: None,
            transport,
            allow_raw_fallback,
            pane_id: pane_id.clone(),
            mux_token,
            mux_session_id: None,
            mux_failures: 0,
            pending_input: Vec::new(),
            reconnect_attempts: 0,
            status_el: status_el.clone(),
        }));
        post_transport_status(&window, pane_id.as_deref(), transport);

        setup_websocket(&window, app_state.clone(), writer.clone())?;
        setup_keyboard(&window, app_state.clone())?;
        setup_paste(&window, app_state.clone())?;
        setup_text_input(&window, &document, app_state.clone())?;
        setup_resize(
            &window,
            canvas.clone(),
            app_state.clone(),
            render_backend.clone(),
        )?;

        start_render_loop(window, app_state, render_backend);

        Ok(())
    }

    fn canvas_size(canvas: &HtmlCanvasElement, window: &web_sys::Window) -> (u32, u32) {
        let dpr = window.device_pixel_ratio().max(1.0);
        let width = (canvas.client_width() as f64 * dpr).round() as u32;
        let height = (canvas.client_height() as f64 * dpr).round() as u32;
        let width = width.max(320);
        let height = height.max(240);
        (width, height)
    }

    fn setup_keyboard(
        window: &web_sys::Window,
        state: Rc<RefCell<AppState>>,
    ) -> Result<(), JsValue> {
        let handler = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            if let Some(bytes) = map_key(&event) {
                event.prevent_default();
                queue_or_send(&state, &bytes);
            }
        });
        window.add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref())?;
        handler.forget();
        Ok(())
    }

    fn setup_paste(window: &web_sys::Window, state: Rc<RefCell<AppState>>) -> Result<(), JsValue> {
        let handler = Closure::<dyn FnMut(ClipboardEvent)>::new(move |event: ClipboardEvent| {
            if let Some(clipboard) = event.clipboard_data() {
                if let Ok(text) = clipboard.get_data("text") {
                    if !text.is_empty() {
                        queue_or_send(&state, text.as_bytes());
                    }
                }
            }
        });
        window.add_event_listener_with_callback("paste", handler.as_ref().unchecked_ref())?;
        handler.forget();
        Ok(())
    }

    fn setup_text_input(
        window: &web_sys::Window,
        document: &Document,
        state: Rc<RefCell<AppState>>,
    ) -> Result<(), JsValue> {
        let Some(el) = document.get_element_by_id("term-input") else {
            return Ok(());
        };
        let input = el.dyn_into::<HtmlTextAreaElement>()?;
        input.set_value("");

        let input_for_text = input.clone();
        let state_for_text = state.clone();
        let on_input = Closure::<dyn FnMut(InputEvent)>::new(move |_event: InputEvent| {
            let value = input_for_text.value();
            if !value.is_empty() {
                queue_or_send(&state_for_text, value.as_bytes());
                input_for_text.set_value("");
            }
        });
        input.add_event_listener_with_callback("input", on_input.as_ref().unchecked_ref())?;
        on_input.forget();

        let input_for_comp = input.clone();
        let state_for_comp = state.clone();
        let on_comp_end =
            Closure::<dyn FnMut(CompositionEvent)>::new(move |event: CompositionEvent| {
                if let Some(text) = event.data() {
                    if !text.is_empty() {
                        queue_or_send(&state_for_comp, text.as_bytes());
                    }
                }
                input_for_comp.set_value("");
            });
        input.add_event_listener_with_callback(
            "compositionend",
            on_comp_end.as_ref().unchecked_ref(),
        )?;
        on_comp_end.forget();

        let focus_input = input.clone();
        let focus_cb = Closure::<dyn FnMut()>::new(move || {
            let _ = focus_input.focus();
        });
        window.add_event_listener_with_callback("click", focus_cb.as_ref().unchecked_ref())?;
        window.add_event_listener_with_callback("focus", focus_cb.as_ref().unchecked_ref())?;
        focus_cb.forget();

        let canvas_focus = input.clone();
        if let Some(canvas) = document.get_element_by_id("term-canvas") {
            let focus_canvas = Closure::<dyn FnMut()>::new(move || {
                let _ = canvas_focus.focus();
            });
            canvas
                .add_event_listener_with_callback("click", focus_canvas.as_ref().unchecked_ref())?;
            focus_canvas.forget();
        }

        Ok(())
    }

    fn setup_websocket(
        window: &web_sys::Window,
        state: Rc<RefCell<AppState>>,
        writer: Arc<Mutex<Box<dyn Write + Send>>>,
    ) -> Result<(), JsValue> {
        let location = window.location();
        let protocol = location.protocol()?;
        let host = location.host()?;
        let ws_scheme = if protocol == "https:" { "wss" } else { "ws" };
        let transport = state.borrow().transport;
        let endpoint = if transport == WsTransport::Mux {
            "/ws-mux"
        } else {
            "/ws"
        };
        let url = format!("{ws_scheme}://{host}{endpoint}");
        if state.borrow().ws.is_some() {
            return Ok(());
        }
        set_status(&state.borrow().status_el, Some("Connecting to server…"));
        let ws = WebSocket::new(&url)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let onmessage_state = state.clone();
        let onmessage_writer = writer.clone();
        let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            if let Ok(buffer) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
                let data = Uint8Array::new(&buffer).to_vec();
                let mut state = onmessage_state.borrow_mut();
                match state.transport {
                    WsTransport::Raw => {
                        feed_terminal_bytes(&mut state, onmessage_writer.clone(), &data)
                    }
                    WsTransport::Mux => {
                        if data.first().copied() == Some(PTY_FRAME_CONTROL_RESP) {
                            if let Ok(value) = decode_shell_response(&data[1..]) {
                                handle_mux_control_response(&mut state, value);
                            }
                            return;
                        }
                        if data.len() < 5 || data[0] != PTY_FRAME_STDOUT {
                            return;
                        }
                        let frame_id = u32::from_be_bytes([data[1], data[2], data[3], data[4]]);
                        if Some(frame_id) != state.mux_session_id {
                            return;
                        }
                        feed_terminal_bytes(&mut state, onmessage_writer.clone(), &data[5..]);
                    }
                }
            }
        });
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        state.borrow_mut().ws = Some(ws);
        let onopen_state = state.clone();
        let onopen = Closure::<dyn FnMut()>::new(move || {
            if let Some(ws) = onopen_state.borrow().ws.as_ref() {
                let mut state = onopen_state.borrow_mut();
                match state.transport {
                    WsTransport::Raw => {
                        send_resize(ws, state.layout.cols, state.layout.rows);
                        flush_pending_input(&mut state, ws);
                        state.reconnect_attempts = 0;
                        state.mux_failures = 0;
                        set_status(&state.status_el, None);
                    }
                    WsTransport::Mux => {
                        state.mux_session_id = None;
                        if let Some(token) = state.mux_token.clone() {
                            send_mux_auth(ws, &token);
                        }
                        send_mux_spawn(ws, state.layout.cols, state.layout.rows);
                        set_status(&state.status_el, Some("Starting shell…"));
                    }
                }
            }
        });
        if let Some(ws) = state.borrow().ws.as_ref() {
            ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        }
        onopen.forget();

        let onclose_state = state.clone();
        let onclose_window = window.clone();
        let onclose_writer = writer.clone();
        let onclose = Closure::<dyn FnMut()>::new(move || {
            if onclose_state.borrow().ws.is_none() {
                return;
            }
            let delay = {
                let mut state = onclose_state.borrow_mut();
                state.ws = None;
                state.mux_session_id = None;
                if state.transport == WsTransport::Mux {
                    state.mux_failures = state.mux_failures.saturating_add(1);
                    if state.allow_raw_fallback && state.mux_failures >= 2 {
                        state.transport = WsTransport::Raw;
                        state.mux_failures = 0;
                        post_transport_status(
                            &onclose_window,
                            state.pane_id.as_deref(),
                            WsTransport::Raw,
                        );
                        set_status(
                            &state.status_el,
                            Some("Mux unavailable; falling back to raw terminal transport"),
                        );
                    } else {
                        set_status(&state.status_el, Some("Connection lost. Reconnecting…"));
                    }
                } else {
                    set_status(&state.status_el, Some("Connection lost. Reconnecting…"));
                }
                let backoff = 500u64.saturating_mul(1 << state.reconnect_attempts.min(5));
                state.reconnect_attempts = state.reconnect_attempts.saturating_add(1);
                backoff
            };
            schedule_reconnect(
                &onclose_window,
                onclose_state.clone(),
                onclose_writer.clone(),
                delay,
            );
        });
        if let Some(ws) = state.borrow().ws.as_ref() {
            ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
            ws.set_onerror(Some(onclose.as_ref().unchecked_ref()));
        }
        onclose.forget();
        Ok(())
    }

    fn apply_resize(
        window: &web_sys::Window,
        canvas: &HtmlCanvasElement,
        state: &Rc<RefCell<AppState>>,
        backend: &SharedRenderBackend,
    ) {
        let (width, height) = canvas_size(canvas, window);
        if {
            let state = state.borrow();
            state.width == width && state.height == height
        } {
            return;
        }

        canvas.set_width(width);
        canvas.set_height(height);

        let mut state = state.borrow_mut();
        state.width = width;
        state.height = height;
        state.layout = compute_layout(width, height, state.cell_w, state.cell_h, 0, 0, 0, 0, 0);
        let cols = state.layout.cols;
        let rows = state.layout.rows;
        state.terminal.resize(cols, rows);
        if let Some(ws) = state.ws.as_ref() {
            match state.transport {
                WsTransport::Raw => send_resize(ws, cols, rows),
                WsTransport::Mux => {
                    if let Some(session_id) = state.mux_session_id {
                        send_mux_resize(ws, session_id, cols, rows);
                    }
                }
            }
        }
        state.frame.resize((width * height * 4) as usize, 0);
        state.image_data = None;

        match backend.borrow().clone() {
            #[cfg(feature = "webgpu")]
            RenderBackend::WebGpu(gpu) => {
                let mut gpu = gpu.borrow_mut();
                gpu.config.width = width;
                gpu.config.height = height;
                gpu.surface.configure(&gpu.device, &gpu.config);
                let (texture, view) = create_frame_texture(&gpu.device, width, height);
                gpu.texture = texture;
                gpu.texture_view = view;
                gpu.bind_group = create_bind_group_with_layout(
                    &gpu.device,
                    &gpu.bind_group_layout,
                    &gpu.texture_view,
                    &gpu.sampler,
                );
            }
            RenderBackend::Canvas2d(ctx) => {
                ctx.clear_rect(0.0, 0.0, width as f64, height as f64);
            }
        }
    }

    fn setup_resize(
        window: &web_sys::Window,
        canvas: HtmlCanvasElement,
        state: Rc<RefCell<AppState>>,
        backend: SharedRenderBackend,
    ) -> Result<(), JsValue> {
        let window_for_size = window.clone();
        let canvas_for_size = canvas.clone();
        let state_for_size = state.clone();
        let backend_for_size = backend.clone();
        let handler = Closure::<dyn FnMut()>::new(move || {
            apply_resize(
                &window_for_size,
                &canvas_for_size,
                &state_for_size,
                &backend_for_size,
            );
        });
        window.add_event_listener_with_callback("resize", handler.as_ref().unchecked_ref())?;
        handler.forget();

        let window_for_observer = window.clone();
        let canvas_for_observer = canvas.clone();
        let state_for_observer = state.clone();
        let backend_for_observer = backend.clone();
        let observer_cb = Closure::<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>::new(
            move |_entries, _observer| {
                apply_resize(
                    &window_for_observer,
                    &canvas_for_observer,
                    &state_for_observer,
                    &backend_for_observer,
                );
            },
        );
        let observer = web_sys::ResizeObserver::new(observer_cb.as_ref().unchecked_ref())?;
        observer.observe(&canvas);
        observer_cb.forget();
        std::mem::forget(observer);
        Ok(())
    }

    fn start_render_loop(
        window: web_sys::Window,
        state: Rc<RefCell<AppState>>,
        backend: SharedRenderBackend,
    ) {
        let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
        let g = f.clone();

        let window_for_frame = window.clone();
        *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            {
                let mut state = state.borrow_mut();
                if state.last_blink.elapsed() >= BLINK_INTERVAL {
                    state.blink_on = !state.blink_on;
                    state.last_blink = Instant::now();
                }
                let width = state.width;
                let height = state.height;
                let cell_w = state.cell_w;
                let cell_h = state.cell_h;
                let layout = state.layout;
                let blink_on = state.blink_on;
                let bg = state.terminal.default_bg();
                draw_background(&mut state.frame, width, height, Instant::now(), bg);
                let state = &mut *state;
                let terminal_ptr = &state.terminal as *const _;
                let glyphs = &mut state.glyphs;
                let frame_buf = &mut state.frame;
                let terminal = unsafe { &*terminal_ptr };
                draw_grid(
                    terminal,
                    glyphs,
                    frame_buf,
                    width,
                    height,
                    cell_w,
                    cell_h,
                    layout.content_x,
                    layout.content_y,
                    None,
                    None,
                    blink_on,
                    None,
                    None,
                );
                draw_cursor_overlay(
                    terminal,
                    glyphs,
                    frame_buf,
                    width,
                    height,
                    cell_w,
                    cell_h,
                    layout.content_x,
                    layout.content_y,
                    None,
                    blink_on,
                    blink_on,
                );

                drain_outbox(state);
            }

            let render_result = match backend.borrow().clone() {
                #[cfg(feature = "webgpu")]
                RenderBackend::WebGpu(gpu) => render_gpu(&state, &gpu),
                RenderBackend::Canvas2d(ctx) => render_canvas_2d(&state, &ctx),
            };

            if let Err(err) = render_result {
                web_sys::console::warn_1(&err);
            }

            let _ = window_for_frame
                .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
        }) as Box<dyn FnMut()>));

        let _ =
            window.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }

    fn send_resize(ws: &WebSocket, cols: usize, rows: usize) {
        let _ = ws.send_with_str(&format!("resize:{cols}x{rows}"));
    }

    fn drain_outbox(state: &mut AppState) {
        let Some(ws) = state.ws.as_ref() else {
            return;
        };
        let mut pending = state.outbox.lock().unwrap();
        for bytes in pending.drain(..) {
            match state.transport {
                WsTransport::Raw => {
                    let _ = ws.send_with_u8_array(&bytes);
                }
                WsTransport::Mux => {
                    if let Some(session_id) = state.mux_session_id {
                        let frame = encode_stdin_frame(session_id, &bytes);
                        let _ = ws.send_with_u8_array(&frame);
                    } else {
                        state.pending_input.push(bytes);
                    }
                }
            }
        }
    }

    #[cfg(feature = "webgpu")]
    fn render_gpu(
        state: &Rc<RefCell<AppState>>,
        gpu: &Rc<RefCell<GpuState>>,
    ) -> Result<(), JsValue> {
        let mut gpu = gpu.borrow_mut();
        let frame = state.borrow();

        let mut padded = std::mem::take(&mut gpu.padded_upload);
        upload_frame(
            &gpu.queue,
            &gpu.texture,
            frame.width,
            frame.height,
            &frame.frame,
            &mut padded,
        );
        gpu.padded_upload = padded;

        let output = match gpu.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(SurfaceError::Lost) | Err(SurfaceError::Outdated) => {
                gpu.surface.configure(&gpu.device, &gpu.config);
                return Ok(());
            }
            Err(SurfaceError::Timeout) => return Ok(()),
            Err(SurfaceError::OutOfMemory) => {
                return Err(JsValue::from_str("surface out of memory"));
            }
            Err(SurfaceError::Other) => return Ok(()),
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("term-web-encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("term-web-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&gpu.pipeline);
            pass.set_bind_group(0, &gpu.bind_group, &[]);
            pass.set_vertex_buffer(0, gpu.vertex_buffer.slice(..));
            pass.draw(0..4, 0..1);
        }

        gpu.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }

    fn render_canvas_2d(
        state: &Rc<RefCell<AppState>>,
        ctx: &CanvasRenderingContext2d,
    ) -> Result<(), JsValue> {
        let mut frame = state.borrow_mut();
        let width = frame.width;
        let height = frame.height;
        frame.image_data = Some(ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&frame.frame),
            width,
            height,
        )?);
        if let Some(data) = frame.image_data.as_ref() {
            ctx.put_image_data(data, 0.0, 0.0)?;
        }
        Ok(())
    }

    fn map_key(event: &KeyboardEvent) -> Option<Vec<u8>> {
        if event.is_composing() {
            return None;
        }
        let key = event.key();
        if event.ctrl_key() || event.meta_key() {
            if key.len() == 1 {
                let b = key.as_bytes()[0];
                if b.is_ascii_alphabetic() {
                    return Some(vec![b.to_ascii_uppercase() - b'@']);
                }
            }
        }

        match key.as_str() {
            "Enter" => Some(vec![b'\r']),
            "Tab" => Some(vec![b'\t']),
            "Backspace" => Some(vec![0x7f]),
            "Escape" => Some(vec![0x1b]),
            "ArrowUp" => Some(b"\x1b[A".to_vec()),
            "ArrowDown" => Some(b"\x1b[B".to_vec()),
            "ArrowRight" => Some(b"\x1b[C".to_vec()),
            "ArrowLeft" => Some(b"\x1b[D".to_vec()),
            "Home" => Some(b"\x1b[H".to_vec()),
            "End" => Some(b"\x1b[F".to_vec()),
            "PageUp" => Some(b"\x1b[5~".to_vec()),
            "PageDown" => Some(b"\x1b[6~".to_vec()),
            _ => {
                if key.len() == 1 {
                    Some(key.as_bytes().to_vec())
                } else {
                    None
                }
            }
        }
    }

    #[cfg(feature = "webgpu")]
    #[allow(dead_code)]
    async fn init_gpu(
        canvas: &HtmlCanvasElement,
        width: u32,
        height: u32,
    ) -> Result<(GpuState, wgpu::TextureFormat), JsValue> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });
        let canvas_handle = canvas.clone();
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas_handle.clone()))
            .map_err(|err| JsValue::from_str(&format!("surface error: {err:?}")))?;
        let surface: wgpu::Surface<'static> = unsafe { std::mem::transmute(surface) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|err| JsValue::from_str(&format!("adapter error: {err:?}")))?;

        let limits = wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("term-web-device"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|err| JsValue::from_str(&format!("device error: {err:?}")))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![format],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let (texture, texture_view) = create_frame_texture(&device, width, height);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("term-web-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("term-web-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group =
            create_bind_group_with_layout(&device, &bind_group_layout, &texture_view, &sampler);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("term-web-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("term-web-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("term-web-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let vertices = [
            Vertex {
                pos: [-1.0, -1.0],
                uv: [0.0, 1.0],
            },
            Vertex {
                pos: [1.0, -1.0],
                uv: [1.0, 1.0],
            },
            Vertex {
                pos: [-1.0, 1.0],
                uv: [0.0, 0.0],
            },
            Vertex {
                pos: [1.0, 1.0],
                uv: [1.0, 0.0],
            },
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("term-web-quad"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Ok((
            GpuState {
                surface,
                device,
                queue,
                config,
                pipeline,
                bind_group_layout,
                bind_group,
                vertex_buffer,
                texture,
                texture_view,
                sampler,
                padded_upload: Vec::new(),
                _canvas: canvas_handle,
            },
            format,
        ))
    }

    #[cfg(feature = "webgpu")]
    fn upload_frame(
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
        frame: &[u8],
        padded: &mut Vec<u8>,
    ) {
        let unpadded_bytes_per_row = (width * 4) as usize;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) / align * align;

        if padded_bytes_per_row == unpadded_bytes_per_row {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                frame,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(unpadded_bytes_per_row as u32),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            return;
        }

        let needed = padded_bytes_per_row * height as usize;
        if padded.len() != needed {
            padded.resize(needed, 0);
        }
        for row in 0..height as usize {
            let src = &frame[row * unpadded_bytes_per_row..(row + 1) * unpadded_bytes_per_row];
            let dst_start = row * padded_bytes_per_row;
            padded[dst_start..dst_start + unpadded_bytes_per_row].copy_from_slice(src);
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            padded,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row as u32),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
    #[cfg(feature = "webgpu")]
    fn create_frame_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("term-web-frame"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    #[cfg(feature = "webgpu")]
    fn create_bind_group_with_layout(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("term-web-bind-group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::start;
