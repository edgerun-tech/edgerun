// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use edgerun_runtime_proto::wire::snapshot_response_v1;
use edgerun_runtime_proto::wire::{
    CleanupResultV1, LabelPairV1, MountListV1, MountV1, RemoveResultV1, SnapshotItemV1,
    SnapshotKindV1, SnapshotListV1, SnapshotMaterializedEventV1, SnapshotOpV1, SnapshotRequestV1,
    SnapshotResponseV1, SnapshotUsageV1,
};
use edgerun_snapshotter::{
    Mount as LocalMount, PersistentSnapshotter, Snapshot as LocalSnapshot,
    SnapshotKind as LocalSnapshotKind, SnapshotUsage as LocalSnapshotUsage, Snapshotter,
};
use edgerun_storage::timeline::{StorageBackedTimeline, TimelineActorTypeV1, TimelineEventTypeV1};
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tracing::info;

const DEFAULT_TIMELINE_SEGMENT: &str = "runtime.timeline";

#[derive(Debug, Parser)]
#[command(
    name = "edgerun-snapshotterd",
    about = "EdgeRun snapshotter bootstrap daemon"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Prepare {
        #[arg(long)]
        workspace_id: String,
        #[arg(long)]
        state_file: PathBuf,
        #[arg(long)]
        key: String,
        #[arg(long)]
        parent: Option<String>,
    },
    View {
        #[arg(long)]
        workspace_id: String,
        #[arg(long)]
        state_file: PathBuf,
        #[arg(long)]
        key: String,
        #[arg(long)]
        parent: String,
    },
    Commit {
        #[arg(long)]
        workspace_id: String,
        #[arg(long)]
        state_file: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        key: String,
    },
    Remove {
        #[arg(long)]
        workspace_id: String,
        #[arg(long)]
        state_file: PathBuf,
        #[arg(long)]
        key: String,
    },
    Walk {
        #[arg(long)]
        workspace_id: String,
        #[arg(long)]
        state_file: PathBuf,
    },
    Cleanup {
        #[arg(long)]
        workspace_id: String,
        #[arg(long)]
        state_file: PathBuf,
    },
    Materialize {
        #[arg(long)]
        workspace_id: String,
        #[arg(long)]
        state_file: PathBuf,
        #[arg(long)]
        lane: String,
        #[arg(long)]
        snapshot_key: String,
        #[arg(long)]
        cursor_offset: u64,
        #[arg(long)]
        compacted_events: u64,
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
    },
    Serve {
        #[arg(long)]
        workspace_id: String,
        #[arg(long)]
        state_file: PathBuf,
        #[arg(long)]
        socket_path: PathBuf,
    },
    Rpc {
        #[arg(long)]
        workspace_id: String,
        #[arg(long)]
        state_file: PathBuf,
    },
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
        Command::Prepare {
            workspace_id,
            state_file,
            key,
            parent,
        } => {
            let snapshotter = PersistentSnapshotter::open(workspace_id, state_file)
                .context("open snapshotter")?;
            let snapshot = snapshotter
                .prepare(&key, parent.as_deref(), BTreeMap::new())
                .context("prepare snapshot")?;
            println!("prepared: {}", snapshot.key);
        }
        Command::View {
            workspace_id,
            state_file,
            key,
            parent,
        } => {
            let snapshotter = PersistentSnapshotter::open(workspace_id, state_file)
                .context("open snapshotter")?;
            let snapshot = snapshotter
                .view(&key, &parent, BTreeMap::new())
                .context("view snapshot")?;
            println!("view: {}", snapshot.key);
        }
        Command::Commit {
            workspace_id,
            state_file,
            name,
            key,
        } => {
            let snapshotter = PersistentSnapshotter::open(workspace_id, state_file)
                .context("open snapshotter")?;
            let committed = snapshotter.commit(&name, &key).context("commit snapshot")?;
            println!("committed: {}", committed.key);
        }
        Command::Remove {
            workspace_id,
            state_file,
            key,
        } => {
            let snapshotter = PersistentSnapshotter::open(workspace_id, state_file)
                .context("open snapshotter")?;
            snapshotter.remove(&key).context("remove snapshot")?;
            println!("removed: {key}");
        }
        Command::Walk {
            workspace_id,
            state_file,
        } => {
            let snapshotter = PersistentSnapshotter::open(workspace_id, state_file)
                .context("open snapshotter")?;
            let items = snapshotter.walk().context("walk snapshots")?;
            for item in items {
                let parent = item.parent.unwrap_or_else(|| "-".to_string());
                println!(
                    "key={} kind={:?} parent={} size_bytes={} inode_count={} mount_root={}",
                    item.key, item.kind, parent, item.size_bytes, item.inode_count, item.mount_root
                );
            }
        }
        Command::Cleanup {
            workspace_id,
            state_file,
        } => {
            let snapshotter = PersistentSnapshotter::open(workspace_id, state_file)
                .context("open snapshotter")?;
            let dropped = snapshotter.cleanup().context("cleanup snapshots")?;
            println!("cleanup removed {dropped} view snapshots");
        }
        Command::Materialize {
            workspace_id,
            state_file,
            lane,
            snapshot_key,
            cursor_offset,
            compacted_events,
            run_id,
            job_id,
            session_id,
            data_dir,
            segment,
        } => {
            let snapshotter = PersistentSnapshotter::open(workspace_id, state_file)
                .context("open snapshotter")?;
            let event = snapshotter.build_materialized_event(
                &lane,
                &snapshot_key,
                cursor_offset,
                compacted_events,
                now_unix_ms(),
            );

            let payload = SnapshotMaterializedEventV1 {
                schema_version: event.schema_version,
                workspace_id: event.workspace_id,
                lane: event.lane,
                snapshot_key: event.snapshot_key,
                cursor_offset: event.cursor_offset,
                root_hash_blake3_hex: event.root_hash_blake3_hex,
                events_compacted: event.events_compacted,
                ts_unix_ms: event.ts_unix_ms,
            }
            .encode_to_vec();
            let mut timeline =
                StorageBackedTimeline::open_writer(data_dir, &segment).context("open timeline")?;
            let envelope = StorageBackedTimeline::build_envelope(
                run_id,
                job_id,
                session_id,
                TimelineActorTypeV1::TimelineActorTypeSystem,
                "edgerun-snapshotterd".to_string(),
                TimelineEventTypeV1::TimelineEventTypeJobProgress,
                "snapshot_materialized_v1_pb".to_string(),
                payload,
            );
            let offset = timeline.publish(&envelope).context("publish timeline")?;
            info!(offset, snapshot_key = %snapshot_key, "published snapshot materialization event");
        }
        Command::Serve {
            workspace_id,
            state_file,
            socket_path,
        } => run_server(workspace_id, state_file, socket_path).await?,
        Command::Rpc {
            workspace_id,
            state_file,
        } => run_rpc_once(workspace_id, state_file)?,
    }
    Ok(())
}

async fn run_server(workspace_id: String, state_file: PathBuf, socket_path: PathBuf) -> Result<()> {
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
    info!(socket = %socket_path.to_string_lossy(), "snapshotter serve started (protobuf)");

    loop {
        let (stream, _) = listener.accept().await.context("accept unix stream")?;
        let workspace_id = workspace_id.clone();
        let state_file = state_file.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_stream(stream, workspace_id, state_file).await {
                tracing::warn!(error = %err, "snapshotter connection handling failed");
            }
        });
    }
}

async fn handle_stream(
    mut stream: UnixStream,
    workspace_id: String,
    state_file: PathBuf,
) -> Result<()> {
    loop {
        let req = match read_pb_frame::<SnapshotRequestV1>(&mut stream).await {
            Ok(v) => v,
            Err(err) => {
                if err.to_string().contains("early eof") {
                    return Ok(());
                }
                return Err(err);
            }
        };
        let resp = handle_request(req, &workspace_id, &state_file);
        write_pb_frame(&mut stream, &resp).await?;
    }
}

fn run_rpc_once(workspace_id: String, state_file: PathBuf) -> Result<()> {
    let req = read_pb_frame_sync::<SnapshotRequestV1>(&mut std::io::stdin().lock())?;
    let resp = handle_request(req, &workspace_id, &state_file);
    write_pb_frame_sync(&mut std::io::stdout().lock(), &resp)?;
    Ok(())
}

fn handle_request(
    req: SnapshotRequestV1,
    workspace_id: &str,
    state_file: &Path,
) -> SnapshotResponseV1 {
    if req.schema_version != 1 {
        return snap_response(false, "unsupported schema_version", None);
    }

    let op = SnapshotOpV1::try_from(req.op).unwrap_or(SnapshotOpV1::Unspecified);
    if matches!(op, SnapshotOpV1::Health) {
        return snap_response(true, "ok", None);
    }

    let snapshotter =
        match PersistentSnapshotter::open(workspace_id.to_string(), state_file.to_path_buf()) {
            Ok(s) => s,
            Err(err) => return snap_response(false, &err.to_string(), None),
        };

    let result: Result<Option<snapshot_response_v1::Body>> = (|| match op {
        SnapshotOpV1::Prepare => {
            let item = snapshotter
                .prepare(
                    &req.key,
                    if req.parent.is_empty() {
                        None
                    } else {
                        Some(req.parent.as_str())
                    },
                    BTreeMap::new(),
                )
                .context("prepare")?;
            Ok(Some(snapshot_response_v1::Body::Snapshot(
                to_proto_snapshot(item),
            )))
        }
        SnapshotOpV1::View => {
            let item = snapshotter
                .view(&req.key, &req.parent, BTreeMap::new())
                .context("view")?;
            Ok(Some(snapshot_response_v1::Body::Snapshot(
                to_proto_snapshot(item),
            )))
        }
        SnapshotOpV1::Commit => {
            let item = snapshotter.commit(&req.name, &req.key).context("commit")?;
            Ok(Some(snapshot_response_v1::Body::Snapshot(
                to_proto_snapshot(item),
            )))
        }
        SnapshotOpV1::Remove => {
            snapshotter.remove(&req.key).context("remove")?;
            Ok(Some(snapshot_response_v1::Body::Removed(RemoveResultV1 {
                key: req.key,
            })))
        }
        SnapshotOpV1::Walk => {
            let items = snapshotter.walk().context("walk")?;
            Ok(Some(snapshot_response_v1::Body::Snapshots(
                SnapshotListV1 {
                    items: items.into_iter().map(to_proto_snapshot).collect(),
                },
            )))
        }
        SnapshotOpV1::Cleanup => {
            let dropped = snapshotter.cleanup().context("cleanup")?;
            Ok(Some(snapshot_response_v1::Body::Cleanup(CleanupResultV1 {
                dropped,
            })))
        }
        SnapshotOpV1::Stat => {
            let item = snapshotter.stat(&req.key).context("stat")?;
            Ok(Some(snapshot_response_v1::Body::Snapshot(
                to_proto_snapshot(item),
            )))
        }
        SnapshotOpV1::Mounts => {
            let item = snapshotter.mounts(&req.key).context("mounts")?;
            Ok(Some(snapshot_response_v1::Body::Mounts(MountListV1 {
                items: item.into_iter().map(to_proto_mount).collect(),
            })))
        }
        SnapshotOpV1::Usage => {
            let item = snapshotter.usage(&req.key).context("usage")?;
            Ok(Some(snapshot_response_v1::Body::Usage(to_proto_usage(
                item,
            ))))
        }
        SnapshotOpV1::Health => Ok(None),
        SnapshotOpV1::Unspecified => Err(anyhow::anyhow!("unspecified snapshot op")),
    })();

    match result {
        Ok(body) => snap_response(true, "ok", body),
        Err(err) => snap_response(false, &err.to_string(), None),
    }
}

fn to_proto_snapshot(item: LocalSnapshot) -> SnapshotItemV1 {
    SnapshotItemV1 {
        key: item.key,
        parent: item.parent.unwrap_or_default(),
        kind: match item.kind {
            LocalSnapshotKind::Active => SnapshotKindV1::Active as i32,
            LocalSnapshotKind::View => SnapshotKindV1::View as i32,
            LocalSnapshotKind::Committed => SnapshotKindV1::Committed as i32,
        },
        labels: item
            .labels
            .into_iter()
            .map(|(k, v)| LabelPairV1 { key: k, value: v })
            .collect(),
        size_bytes: item.size_bytes,
        inode_count: item.inode_count,
        mount_root: item.mount_root,
    }
}

fn to_proto_mount(item: LocalMount) -> MountV1 {
    MountV1 {
        r#type: item.r#type,
        source: item.source,
        options: item.options,
    }
}

fn to_proto_usage(item: LocalSnapshotUsage) -> SnapshotUsageV1 {
    SnapshotUsageV1 {
        size_bytes: item.size_bytes,
        inode_count: item.inode_count,
    }
}

fn snap_response(
    ok: bool,
    message: &str,
    body: Option<snapshot_response_v1::Body>,
) -> SnapshotResponseV1 {
    SnapshotResponseV1 {
        schema_version: 1,
        ok,
        message: message.to_string(),
        body,
    }
}

fn now_unix_ms() -> u64 {
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
