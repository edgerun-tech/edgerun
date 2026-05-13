use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use codex_core::Prompt;
use codex_core::Provider;
use codex_core::ResponseEvent;
use codex_core::TurnRequest;
use codex_core::api::AuthProvider;
use codex_core::api::RetryConfig;
use codex_core::protocol::models::ContentItem;
use codex_core::protocol::models::ResponseItem;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use crossterm::execute;
use crossterm::terminal;
use edgerun_http::HeaderMap;
use edgerun_http::HeaderValue;
use edgerun_http::header::AUTHORIZATION;
use edgerun_json::serde_json::Value;
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Position;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

#[derive(Debug)]
struct ChatGptAuth {
    access_token: String,
    account_id: Option<String>,
}

impl AuthProvider for ChatGptAuth {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        let bearer = format!("Bearer {}", self.access_token);
        if let Ok(value) = HeaderValue::from_str(&bearer) {
            headers.insert(AUTHORIZATION, value);
        }
        if let Some(account_id) = self.account_id.as_deref()
            && let Ok(value) = HeaderValue::from_str(account_id)
        {
            headers.insert("ChatGPT-Account-ID", value);
        }
    }
}

#[derive(Clone, Debug)]
enum Role {
    User,
    Assistant,
    System,
}

#[derive(Clone, Debug)]
struct Message {
    role: Role,
    text: String,
}

#[derive(Debug)]
struct App {
    model: String,
    input: String,
    messages: Vec<Message>,
    busy: bool,
    scroll: u16,
    tx: mpsc::Sender<WorkerRequest>,
    rx: mpsc::Receiver<WorkerEvent>,
}

#[derive(Debug)]
struct WorkerRequest {
    input: Vec<ResponseItem>,
}

#[derive(Debug)]
enum WorkerEvent {
    Started,
    Delta(String),
    Completed { response_id: Option<String> },
    Failed(String),
}

fn main() -> Result<(), Box<dyn Error>> {
    let auth = read_chatgpt_auth()?;
    let model = std::env::var("CODEX_TUI_MODEL").unwrap_or_else(|_| "gpt-5.5".to_string());

    if let Some(prompt) = prompt_arg()? {
        return run_prompt_mode(model, auth, prompt);
    }

    let (request_tx, request_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    spawn_worker(model.clone(), auth, request_rx, event_tx);

    let mut app = App {
        model,
        input: String::new(),
        messages: vec![Message {
            role: Role::System,
            text: "Native Codex TUI. Enter sends, Ctrl-C quits.".to_string(),
        }],
        busy: false,
        scroll: 0,
        tx: request_tx,
        rx: event_rx,
    };

    run_tui(&mut app)
}

fn run_tui(app: &mut App) -> Result<(), Box<dyn Error>> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_event_loop(app, &mut terminal);
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_event_loop(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    loop {
        drain_worker_events(app);
        terminal.draw(|frame| draw(app, frame))?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.input.clear();
            }
            KeyCode::Char(ch) if !app.busy => app.input.push(ch),
            KeyCode::Backspace if !app.busy => {
                app.input.pop();
            }
            KeyCode::Enter if !app.busy => submit(app),
            KeyCode::Up => app.scroll = app.scroll.saturating_add(1),
            KeyCode::Down => app.scroll = app.scroll.saturating_sub(1),
            KeyCode::PageUp => app.scroll = app.scroll.saturating_add(10),
            KeyCode::PageDown => app.scroll = app.scroll.saturating_sub(10),
            _ => {}
        }
    }
    Ok(())
}

fn prompt_arg() -> Result<Option<String>, Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut prompt = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--prompt" | "-p" => {
                let value = args
                    .next()
                    .ok_or("--prompt requires a prompt string argument")?;
                prompt = Some(value);
            }
            "--help" | "-h" => {
                println!("Usage: codex-tui [--prompt TEXT]");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    Ok(prompt)
}

fn run_prompt_mode(
    model: String,
    auth: Arc<ChatGptAuth>,
    prompt: String,
) -> Result<(), Box<dyn Error>> {
    let runtime = edgerun_tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let client = codex_core::ModelClient::new_native(model, provider(), auth);
    runtime.block_on(async {
        let input = vec![user_item(prompt)];
        let mut stream = client.stream_turn(turn_request(input)).await?;
        while let Some(event) = stream.rx_event.recv().await {
            if let ResponseEvent::OutputTextDelta(delta) = event? {
                print!("{delta}");
            }
        }
        println!();
        Ok::<(), Box<dyn Error>>(())
    })
}

fn draw(app: &App, frame: &mut Frame<'_>) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(area);

    let transcript = Paragraph::new(transcript_text(app))
        .block(Block::default().borders(Borders::ALL).title(" Codex "))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    frame.render_widget(transcript, rows[0]);

    let status = if app.busy {
        format!(" {} thinking...  ↑/↓ scroll", app.model)
    } else {
        format!(
            " {} ready  Enter send  Ctrl-U clear  Ctrl-C quit",
            app.model
        )
    };
    frame.render_widget(
        Paragraph::new(status).style(Style::default().fg(Color::DarkGray)),
        rows[1],
    );

    let input_title = if app.busy {
        " Prompt locked "
    } else {
        " Prompt "
    };
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(input_title))
        .wrap(Wrap { trim: false });
    frame.render_widget(input, rows[2]);
    if !app.busy {
        let max_x = rows[2].width.saturating_sub(2);
        let input_x = app.input.chars().count().min(max_x as usize) as u16;
        frame.set_cursor_position(Position::new(rows[2].x + 1 + input_x, rows[2].y + 1));
    }
}

fn transcript_text(app: &App) -> Text<'static> {
    let mut lines = Vec::new();
    for message in &app.messages {
        let (label, color) = match message.role {
            Role::User => ("› you", Color::Cyan),
            Role::Assistant => ("• codex", Color::Green),
            Role::System => ("status", Color::DarkGray),
        };
        lines.push(Line::from(vec![Span::styled(
            label,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )]));
        for line in message.text.lines() {
            lines.push(Line::from(line.to_string()));
        }
        lines.push(Line::from(""));
    }
    Text::from(lines)
}

fn submit(app: &mut App) {
    let input = app.input.trim().to_string();
    if input.is_empty() {
        return;
    }
    app.input.clear();
    app.scroll = 0;
    app.messages.push(Message {
        role: Role::User,
        text: input.clone(),
    });
    let prompt_input = prompt_input(&app.messages);
    app.messages.push(Message {
        role: Role::Assistant,
        text: String::new(),
    });
    app.busy = true;
    if let Err(error) = app.tx.send(WorkerRequest {
        input: prompt_input,
    }) {
        app.busy = false;
        app.messages.push(Message {
            role: Role::System,
            text: format!("worker unavailable: {error}"),
        });
    }
}

fn drain_worker_events(app: &mut App) {
    while let Ok(event) = app.rx.try_recv() {
        match event {
            WorkerEvent::Started => {
                app.busy = true;
            }
            WorkerEvent::Delta(delta) => {
                if let Some(message) = app
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|message| matches!(message.role, Role::Assistant))
                {
                    message.text.push_str(&delta);
                }
            }
            WorkerEvent::Completed { response_id } => {
                app.busy = false;
                if let Some(response_id) = response_id {
                    app.messages.push(Message {
                        role: Role::System,
                        text: format!("response {response_id}"),
                    });
                }
            }
            WorkerEvent::Failed(error) => {
                app.busy = false;
                app.messages.push(Message {
                    role: Role::System,
                    text: error,
                });
            }
        }
    }
}

fn spawn_worker(
    model: String,
    auth: Arc<ChatGptAuth>,
    rx: mpsc::Receiver<WorkerRequest>,
    tx: mpsc::Sender<WorkerEvent>,
) {
    thread::spawn(move || {
        let runtime = match edgerun_tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = tx.send(WorkerEvent::Failed(format!("runtime error: {error}")));
                return;
            }
        };
        let client = codex_core::ModelClient::new_native(model, provider(), auth);
        while let Ok(request) = rx.recv() {
            let _ = tx.send(WorkerEvent::Started);
            let result = runtime.block_on(run_turn(&client, request.input, tx.clone()));
            if let Err(error) = result {
                let _ = tx.send(WorkerEvent::Failed(error.to_string()));
            }
        }
    });
}

async fn run_turn(
    client: &codex_core::ModelClient,
    input: Vec<ResponseItem>,
    tx: mpsc::Sender<WorkerEvent>,
) -> Result<(), Box<dyn Error>> {
    let request = turn_request(input);
    let mut stream = client.stream_turn(request).await?;
    let mut response_id = None;
    while let Some(event) = stream.rx_event.recv().await {
        match event? {
            ResponseEvent::OutputTextDelta(delta) => {
                let _ = tx.send(WorkerEvent::Delta(delta));
            }
            ResponseEvent::Completed {
                response_id: id, ..
            } => response_id = Some(id),
            _ => {}
        }
    }
    let _ = tx.send(WorkerEvent::Completed { response_id });
    Ok(())
}

fn turn_request(input: Vec<ResponseItem>) -> TurnRequest {
    TurnRequest {
        prompt: Prompt {
            input,
            ..Prompt::default()
        },
        store: false,
        ..TurnRequest::default()
    }
}

fn prompt_input(messages: &[Message]) -> Vec<ResponseItem> {
    messages.iter().filter_map(message_item).collect()
}

fn message_item(message: &Message) -> Option<ResponseItem> {
    match message.role {
        Role::User => Some(user_item(message.text.clone())),
        Role::Assistant if !message.text.is_empty() => Some(assistant_item(message.text.clone())),
        Role::Assistant | Role::System => None,
    }
}

fn user_item(text: String) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText { text }],
        phase: None,
    }
}

fn assistant_item(text: String) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText { text }],
        phase: None,
    }
}

fn provider() -> Provider {
    let mut headers = HeaderMap::new();
    headers.insert(
        "version",
        HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
    );
    Provider {
        name: "OpenAI".to_string(),
        base_url: "https://chatgpt.com/backend-api/codex".to_string(),
        query_params: None::<HashMap<String, String>>,
        headers,
        retry: RetryConfig {
            max_attempts: 1,
            base_delay: Duration::from_millis(200),
            retry_429: false,
            retry_5xx: false,
            retry_transport: false,
        },
        stream_idle_timeout: Duration::from_secs(60),
    }
}

fn codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .expect("CODEX_HOME or HOME must be set")
}

fn read_chatgpt_auth() -> Result<Arc<ChatGptAuth>, Box<dyn Error>> {
    let auth_path = codex_home().join("auth.json");
    let auth: Value = edgerun_json::serde_json::from_slice(&std::fs::read(&auth_path)?)?;
    let access_token = auth
        .get("tokens")
        .and_then(|tokens| tokens.get("access_token"))
        .and_then(Value::as_str)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| format!("missing tokens.access_token in {}", auth_path.display()))?
        .to_string();
    let account_id = auth
        .get("tokens")
        .and_then(|tokens| tokens.get("account_id"))
        .and_then(Value::as_str)
        .filter(|account_id| !account_id.is_empty())
        .map(ToString::to_string);
    Ok(Arc::new(ChatGptAuth {
        access_token,
        account_id,
    }))
}
