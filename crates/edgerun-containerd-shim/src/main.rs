// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use edgerun_containerd_shim::{
    register_task_ttrpc_service, ContainerdTaskClient, ContainerdTaskTtrpcService, ShimTaskService,
    ShimTaskTtrpcService, TaskCommand, TaskLifecycle, TaskLifecycleState,
};
use edgerun_runtime_proto::wire::{
    shim_request_v1, EmitEventRequestV1, LifecycleCommandV1, RuntimeTaskEventV1, ShimRequestV1,
    ShimResponseV1, TaskServiceOpV1, TaskServiceRequestV1, TaskStateRequestV1,
};
use edgerun_runtime_proto::{runtime_task_subject, RuntimeTaskEvent, RuntimeTaskEventKind};
use edgerun_storage::timeline::{StorageBackedTimeline, TimelineActorTypeV1, TimelineEventTypeV1};
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tracing::info;

const DEFAULT_TIMELINE_SEGMENT: &str = "runtime.timeline";

#[derive(Parser, Debug)]
#[command(
    name = "containerd-shim-edgerun-v1",
    about = "EdgeRun containerd shim v1 bootstrap"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    EmitEvent {
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        event_id: String,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        job_id: String,
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        data_dir: PathBuf,
        #[arg(long, default_value = DEFAULT_TIMELINE_SEGMENT)]
        segment: String,
        #[arg(long)]
        command: LifecycleCommand,
        #[arg(long)]
        pid: Option<u32>,
        #[arg(long)]
        exit_code: Option<u32>,
        #[arg(long)]
        detail: Option<String>,
    },
    PrintSubject {
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        task_id: String,
        #[arg(long, default_value = "event")]
        lane: String,
    },
    Serve {
        #[arg(long)]
        socket_path: PathBuf,
        #[arg(long)]
        data_dir: PathBuf,
        #[arg(long, default_value = DEFAULT_TIMELINE_SEGMENT)]
        segment: String,
        #[arg(long, default_value = "runtime-run")]
        run_id: String,
        #[arg(long, default_value = "runtime-job")]
        job_id: String,
        #[arg(long, default_value = "runtime-session")]
        session_id: String,
    },
    Rpc {
        #[arg(long)]
        data_dir: PathBuf,
        #[arg(long, default_value = DEFAULT_TIMELINE_SEGMENT)]
        segment: String,
        #[arg(long, default_value = "runtime-run")]
        run_id: String,
        #[arg(long, default_value = "runtime-job")]
        job_id: String,
        #[arg(long, default_value = "runtime-session")]
        session_id: String,
    },
    ServeTtrpc {
        #[arg(long)]
        ttrpc_socket_path: PathBuf,
        #[arg(long)]
        shim_socket_path: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LifecycleCommand {
    Create,
    Start,
    Exit,
    Kill,
}

#[derive(Debug, Clone)]
struct ServerContext {
    data_dir: PathBuf,
    segment: String,
    run_id: String,
    job_id: String,
    session_id: String,
    tasks: Arc<Mutex<HashMap<String, TaskLifecycle>>>,
    task_service: Arc<Mutex<ShimTaskService>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(err) = run().await {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::EmitEvent {
            namespace,
            task_id,
            event_id,
            run_id,
            job_id,
            session_id,
            data_dir,
            segment,
            command,
            pid,
            exit_code,
            detail,
        } => {
            let mut lifecycle = TaskLifecycle::new(namespace.clone(), task_id.clone());
            let kind = lifecycle
                .transition(command.into())
                .or_else(|_| map_direct_kind(command.into()))
                .context("transition lifecycle")?;
            let event = lifecycle.build_event(kind.clone(), event_id, pid, exit_code, detail);
            event.validate().context("validate runtime event")?;
            let offset = publish_runtime_event(
                &data_dir,
                &segment,
                &run_id,
                &job_id,
                &session_id,
                event,
                kind,
            )?;
            info!(offset, namespace = %namespace, task_id = %task_id, "published runtime task event");
        }
        Command::PrintSubject {
            namespace,
            task_id,
            lane,
        } => {
            println!("{}", runtime_task_subject(&namespace, &task_id, &lane));
        }
        Command::Serve {
            socket_path,
            data_dir,
            segment,
            run_id,
            job_id,
            session_id,
        } => {
            run_server(
                ServerContext {
                    data_dir,
                    segment,
                    run_id,
                    job_id,
                    session_id,
                    tasks: Arc::new(Mutex::new(HashMap::new())),
                    task_service: Arc::new(Mutex::new(ShimTaskService::new())),
                },
                socket_path,
            )
            .await?;
        }
        Command::Rpc {
            data_dir,
            segment,
            run_id,
            job_id,
            session_id,
        } => {
            run_rpc_once(ServerContext {
                data_dir,
                segment,
                run_id,
                job_id,
                session_id,
                tasks: Arc::new(Mutex::new(HashMap::new())),
                task_service: Arc::new(Mutex::new(ShimTaskService::new())),
            })?;
        }
        Command::ServeTtrpc {
            ttrpc_socket_path,
            shim_socket_path,
        } => {
            run_ttrpc_server(ttrpc_socket_path, shim_socket_path).await?;
        }
    }
    Ok(())
}

async fn run_ttrpc_server(ttrpc_socket_path: PathBuf, shim_socket_path: PathBuf) -> Result<()> {
    use containerd_shim_protos::ttrpc::asynchronous::Server;

    if ttrpc_socket_path.exists() {
        std::fs::remove_file(&ttrpc_socket_path).with_context(|| {
            format!(
                "remove stale ttrpc unix socket {}",
                ttrpc_socket_path.to_string_lossy()
            )
        })?;
    }
    if let Some(parent) = ttrpc_socket_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "create ttrpc socket parent directory {}",
                parent.to_string_lossy()
            )
        })?;
    }

    let backend = ContainerdTaskClient::connect(&shim_socket_path)
        .await
        .with_context(|| format!("connect shim socket {}", shim_socket_path.to_string_lossy()))?;
    let service = ContainerdTaskTtrpcService::new(ShimTaskTtrpcService::new(backend))
        .with_runtime_version("edgerun.v1");
    let bind_addr = format!("unix://{}", ttrpc_socket_path.to_string_lossy());

    let mut server = register_task_ttrpc_service(
        Server::new()
            .bind(&bind_addr)
            .with_context(|| format!("bind ttrpc socket {}", bind_addr))?,
        service,
    );
    server.start().await.context("start ttrpc server")?;
    info!(
        ttrpc_socket = %ttrpc_socket_path.to_string_lossy(),
        shim_socket = %shim_socket_path.to_string_lossy(),
        "containerd task ttrpc server started"
    );
    tokio::signal::ctrl_c()
        .await
        .context("wait for shutdown signal")?;
    server.shutdown().await.unwrap_or_default();
    Ok(())
}

async fn run_server(ctx: ServerContext, socket_path: PathBuf) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).with_context(|| {
            format!("remove stale unix socket {}", socket_path.to_string_lossy())
        })?;
    }
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create socket parent {}", parent.to_string_lossy()))?;
    }
    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("bind unix socket {}", socket_path.to_string_lossy()))?;
    info!(socket = %socket_path.to_string_lossy(), "shim serve started (protobuf)");

    loop {
        let (stream, _) = listener.accept().await.context("accept unix stream")?;
        let child_ctx = ctx.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_stream(stream, child_ctx).await {
                tracing::warn!(error = %err, "connection handling failed");
            }
        });
    }
}

async fn handle_stream(mut stream: UnixStream, ctx: ServerContext) -> Result<()> {
    loop {
        let req = match read_pb_frame::<ShimRequestV1>(&mut stream).await {
            Ok(v) => v,
            Err(err) => {
                if err.to_string().contains("early eof") {
                    return Ok(());
                }
                return Err(err);
            }
        };
        let resp = handle_request(req, &ctx);
        write_pb_frame(&mut stream, &resp).await?;
    }
}

fn run_rpc_once(ctx: ServerContext) -> Result<()> {
    let req = read_pb_frame_sync::<ShimRequestV1>(&mut std::io::stdin().lock())?;
    let resp = handle_request(req, &ctx);
    write_pb_frame_sync(&mut std::io::stdout().lock(), &resp)?;
    Ok(())
}

fn handle_request(req: ShimRequestV1, ctx: &ServerContext) -> ShimResponseV1 {
    if req.schema_version != 1 {
        return shim_response(false, "unsupported schema_version", None, "", "");
    }

    match req.op {
        Some(shim_request_v1::Op::Health(_)) => shim_response(true, "ok", None, "", ""),
        Some(shim_request_v1::Op::PrintSubject(p)) => {
            let lane = if p.lane.is_empty() {
                "event"
            } else {
                p.lane.as_str()
            };
            let subject = runtime_task_subject(&p.namespace, &p.task_id, lane);
            shim_response(true, "ok", None, &subject, "")
        }
        Some(shim_request_v1::Op::TaskState(TaskStateRequestV1 { namespace, task_id })) => {
            let service = ctx.task_service.lock();
            match service {
                Ok(mut service) => {
                    match service.apply(
                        &namespace,
                        &task_id,
                        TaskServiceOpV1::State,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ) {
                        Ok(resp) => shim_response_full(
                            true,
                            "state",
                            None,
                            "",
                            &resp.state.map(format_state).unwrap_or_default(),
                            resp.pending,
                            resp.pid,
                            resp.exit_code,
                        ),
                        Err(_) => shim_response(true, "state", None, "", ""),
                    }
                }
                Err(_) => shim_response(false, "task service lock poisoned", None, "", ""),
            }
        }
        Some(shim_request_v1::Op::EmitEvent(EmitEventRequestV1 {
            namespace,
            task_id,
            event_id,
            command,
            pid,
            exit_code,
            detail,
        })) => {
            let cmd =
                LifecycleCommandV1::try_from(command).unwrap_or(LifecycleCommandV1::Unspecified);
            let op = apply_task_command(
                ctx,
                namespace,
                task_id,
                cmd,
                if event_id.is_empty() {
                    None
                } else {
                    Some(event_id)
                },
                pid,
                exit_code,
                if detail.is_empty() {
                    None
                } else {
                    Some(detail)
                },
            );
            match op {
                Ok((offset, state)) => {
                    shim_response(true, "event emitted", Some(offset), "", &state)
                }
                Err(err) => shim_response(false, &err.to_string(), None, "", ""),
            }
        }
        Some(shim_request_v1::Op::TaskApi(TaskServiceRequestV1 {
            namespace,
            task_id,
            op,
            signal,
            runtime_name,
            runtime_selector_source,
            bundle_path,
            stdin_path,
            stdout_path,
            stderr_path,
            rootfs_source,
            rootfs_readonly,
            rootfs_type,
            rootfs_options_csv,
        })) => {
            let op = TaskServiceOpV1::try_from(op).unwrap_or(TaskServiceOpV1::Unspecified);
            let service = ctx.task_service.lock();
            match service {
                Ok(mut service) => match service.apply(
                    &namespace,
                    &task_id,
                    op,
                    signal,
                    if runtime_name.is_empty() {
                        None
                    } else {
                        Some(runtime_name.as_str())
                    },
                    if runtime_selector_source.is_empty() {
                        None
                    } else {
                        Some(runtime_selector_source.as_str())
                    },
                    if bundle_path.is_empty() {
                        None
                    } else {
                        Some(bundle_path.as_str())
                    },
                    if stdin_path.is_empty() {
                        None
                    } else {
                        Some(stdin_path.as_str())
                    },
                    if stdout_path.is_empty() {
                        None
                    } else {
                        Some(stdout_path.as_str())
                    },
                    if stderr_path.is_empty() {
                        None
                    } else {
                        Some(stderr_path.as_str())
                    },
                    if rootfs_source.is_empty() {
                        None
                    } else {
                        Some(rootfs_source.as_str())
                    },
                    if rootfs_readonly { Some(true) } else { None },
                    if rootfs_type.is_empty() {
                        None
                    } else {
                        Some(rootfs_type.as_str())
                    },
                    if rootfs_options_csv.is_empty() {
                        None
                    } else {
                        Some(rootfs_options_csv.as_str())
                    },
                ) {
                    Ok(resp) => {
                        let mut offset = None;
                        if let Some(kind) = task_service_event_kind(op) {
                            match publish_task_api_event(ctx, &namespace, &task_id, kind, &resp) {
                                Ok(v) => offset = Some(v),
                                Err(err) => {
                                    return shim_response(false, &err.to_string(), None, "", "")
                                }
                            }
                        }
                        shim_response_full(
                            true,
                            &resp.message,
                            offset,
                            "",
                            &resp.state.map(format_state).unwrap_or_default(),
                            resp.pending,
                            resp.pid,
                            resp.exit_code,
                        )
                    }
                    Err(err) => shim_response(false, &err.to_string(), None, "", ""),
                },
                Err(_) => shim_response(false, "task service lock poisoned", None, "", ""),
            }
        }
        None => shim_response(false, "missing op", None, "", ""),
    }
}

fn shim_response(
    ok: bool,
    message: &str,
    offset: Option<u64>,
    subject: &str,
    state: &str,
) -> ShimResponseV1 {
    shim_response_full(ok, message, offset, subject, state, false, None, None)
}

#[allow(clippy::too_many_arguments)]
fn shim_response_full(
    ok: bool,
    message: &str,
    offset: Option<u64>,
    subject: &str,
    state: &str,
    pending: bool,
    pid: Option<u32>,
    exit_code: Option<u32>,
) -> ShimResponseV1 {
    ShimResponseV1 {
        schema_version: 1,
        ok,
        message: message.to_string(),
        offset,
        subject: subject.to_string(),
        state: state.to_string(),
        pending,
        pid,
        exit_code,
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_task_command(
    ctx: &ServerContext,
    namespace: String,
    task_id: String,
    command: LifecycleCommandV1,
    event_id: Option<String>,
    pid: Option<u32>,
    exit_code: Option<u32>,
    detail: Option<String>,
) -> Result<(u64, String)> {
    let key = task_key(&namespace, &task_id);
    let mut tasks = ctx
        .tasks
        .lock()
        .map_err(|_| anyhow::anyhow!("task map lock poisoned"))?;
    let lifecycle = tasks
        .entry(key)
        .or_insert_with(|| TaskLifecycle::new(namespace.clone(), task_id.clone()));

    let command = map_lifecycle_command(command)?;
    let kind = lifecycle
        .transition(command)
        .or_else(|_| map_direct_kind(command))
        .context("transition lifecycle")?;

    let generated = format!(
        "evt-{}-{}-{}",
        now_unix_ms(),
        namespace.replace('.', "_"),
        task_id.replace('.', "_")
    );
    let event = lifecycle.build_event(
        kind.clone(),
        event_id.unwrap_or(generated),
        pid,
        exit_code,
        detail,
    );
    event.validate().context("validate runtime event")?;
    let state = format_state(lifecycle.state());
    drop(tasks);

    let offset = publish_runtime_event(
        &ctx.data_dir,
        &ctx.segment,
        &ctx.run_id,
        &ctx.job_id,
        &ctx.session_id,
        event,
        kind,
    )?;
    Ok((offset, state))
}

fn publish_runtime_event(
    data_dir: &Path,
    segment: &str,
    run_id: &str,
    job_id: &str,
    session_id: &str,
    event: edgerun_runtime_proto::RuntimeTaskEvent,
    kind: RuntimeTaskEventKind,
) -> Result<u64> {
    let mut timeline = StorageBackedTimeline::open_writer(data_dir.to_path_buf(), segment)
        .context("open timeline writer")?;
    let payload = RuntimeTaskEventV1 {
        schema_version: event.schema_version,
        namespace: event.namespace,
        task_id: event.task_id,
        event_id: event.event_id,
        kind: format!("{:?}", event.kind).to_lowercase(),
        ts_unix_ms: event.ts_unix_ms,
        pid: event.pid,
        exit_code: event.exit_code,
        detail: event.detail.unwrap_or_default(),
    }
    .encode_to_vec();
    let envelope = StorageBackedTimeline::build_envelope(
        run_id.to_string(),
        job_id.to_string(),
        session_id.to_string(),
        TimelineActorTypeV1::TimelineActorTypeAgent,
        "containerd-shim-edgerun-v1".to_string(),
        map_timeline_event_type(kind),
        "runtime_task_event_v1_pb".to_string(),
        payload,
    );
    timeline
        .publish(&envelope)
        .context("publish timeline event")
}

fn publish_task_api_event(
    ctx: &ServerContext,
    namespace: &str,
    task_id: &str,
    kind: RuntimeTaskEventKind,
    resp: &edgerun_containerd_shim::TaskApiResponse,
) -> Result<u64> {
    let mut detail = format!("task_api:{}", resp.message);
    if let Some(runtime) = resp.runtime_name.as_deref() {
        detail.push_str(" runtime=");
        detail.push_str(runtime);
    }
    let event = RuntimeTaskEvent {
        schema_version: 1,
        namespace: namespace.to_string(),
        task_id: task_id.to_string(),
        event_id: format!("evt-{}-{}-{}", now_unix_ms(), namespace, task_id),
        kind: kind.clone(),
        ts_unix_ms: now_unix_ms(),
        pid: resp.pid,
        exit_code: resp.exit_code,
        detail: Some(detail),
    };
    event.validate().context("validate task api event")?;
    publish_runtime_event(
        &ctx.data_dir,
        &ctx.segment,
        &ctx.run_id,
        &ctx.job_id,
        &ctx.session_id,
        event,
        kind,
    )
}

fn task_service_event_kind(op: TaskServiceOpV1) -> Option<RuntimeTaskEventKind> {
    match op {
        TaskServiceOpV1::Create => Some(RuntimeTaskEventKind::Created),
        TaskServiceOpV1::Start => Some(RuntimeTaskEventKind::Started),
        TaskServiceOpV1::Kill => Some(RuntimeTaskEventKind::Killed),
        TaskServiceOpV1::State
        | TaskServiceOpV1::Delete
        | TaskServiceOpV1::Wait
        | TaskServiceOpV1::Unspecified => None,
    }
}

fn map_timeline_event_type(kind: RuntimeTaskEventKind) -> TimelineEventTypeV1 {
    match kind {
        RuntimeTaskEventKind::Exited | RuntimeTaskEventKind::Killed | RuntimeTaskEventKind::Oom => {
            TimelineEventTypeV1::TimelineEventTypeJobFailed
        }
        RuntimeTaskEventKind::Started => TimelineEventTypeV1::TimelineEventTypeJobProgress,
        RuntimeTaskEventKind::Created
        | RuntimeTaskEventKind::ExecStarted
        | RuntimeTaskEventKind::ExecExited => TimelineEventTypeV1::TimelineEventTypeJobOpened,
    }
}

fn map_lifecycle_command(command: LifecycleCommandV1) -> Result<TaskCommand> {
    match command {
        LifecycleCommandV1::Create => Ok(TaskCommand::Create),
        LifecycleCommandV1::Start => Ok(TaskCommand::Start),
        LifecycleCommandV1::Exit => Ok(TaskCommand::Exit),
        LifecycleCommandV1::Kill => Ok(TaskCommand::Kill),
        LifecycleCommandV1::Unspecified => Err(anyhow::anyhow!("unspecified lifecycle command")),
    }
}

fn map_direct_kind(command: TaskCommand) -> Result<RuntimeTaskEventKind> {
    let kind = match command {
        TaskCommand::Create => RuntimeTaskEventKind::Created,
        TaskCommand::Start => RuntimeTaskEventKind::Started,
        TaskCommand::Exit => RuntimeTaskEventKind::Exited,
        TaskCommand::Kill => RuntimeTaskEventKind::Killed,
    };
    Ok(kind)
}

fn task_key(namespace: &str, task_id: &str) -> String {
    format!("{namespace}/{task_id}")
}

fn format_state(state: TaskLifecycleState) -> String {
    match state {
        TaskLifecycleState::Created => "created".to_string(),
        TaskLifecycleState::Running => "running".to_string(),
        TaskLifecycleState::Exited => "exited".to_string(),
    }
}

fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

async fn read_pb_frame<T: Message + Default>(stream: &mut UnixStream) -> Result<T> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .context("read frame length")?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut payload = vec![0u8; len];
    stream
        .read_exact(&mut payload)
        .await
        .context("read frame payload")?;
    T::decode(payload.as_slice()).context("decode protobuf frame")
}

async fn write_pb_frame<T: Message>(stream: &mut UnixStream, value: &T) -> Result<()> {
    let mut payload = Vec::new();
    value
        .encode(&mut payload)
        .context("encode protobuf frame")?;
    let len = (payload.len() as u32).to_be_bytes();
    stream.write_all(&len).await.context("write frame length")?;
    stream
        .write_all(&payload)
        .await
        .context("write frame payload")?;
    Ok(())
}

fn read_pb_frame_sync<T: Message + Default>(reader: &mut impl std::io::Read) -> Result<T> {
    let mut len_buf = [0u8; 4];
    std::io::Read::read_exact(reader, &mut len_buf).context("read frame length (sync)")?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut payload = vec![0u8; len];
    std::io::Read::read_exact(reader, &mut payload).context("read frame payload (sync)")?;
    T::decode(payload.as_slice()).context("decode protobuf frame (sync)")
}

fn write_pb_frame_sync<T: Message>(writer: &mut impl std::io::Write, value: &T) -> Result<()> {
    let mut payload = Vec::new();
    value
        .encode(&mut payload)
        .context("encode protobuf frame (sync)")?;
    let len = (payload.len() as u32).to_be_bytes();
    std::io::Write::write_all(writer, &len).context("write frame length (sync)")?;
    std::io::Write::write_all(writer, &payload).context("write frame payload (sync)")?;
    std::io::Write::flush(writer).context("flush frame (sync)")?;
    Ok(())
}

impl From<LifecycleCommand> for TaskCommand {
    fn from(value: LifecycleCommand) -> Self {
        match value {
            LifecycleCommand::Create => TaskCommand::Create,
            LifecycleCommand::Start => TaskCommand::Start,
            LifecycleCommand::Exit => TaskCommand::Exit,
            LifecycleCommand::Kill => TaskCommand::Kill,
        }
    }
}
