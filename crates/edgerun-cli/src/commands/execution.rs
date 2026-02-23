// SPDX-License-Identifier: Apache-2.0
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use edgerun_storage::os::proto::{
    ExecutionFinishedV1, ExecutionStartedV1, ExecutionStateV1, ExecutionStepFinishedV1,
    ExecutionStepStartedV1, IntentPriorityV1, IntentSubmittedV1,
};
use edgerun_storage::timeline::{
    StorageBackedTimeline, TimelineActorTypeV1, TimelineEventTypeV1, TimelineQueryFilter,
};
use prost::Message;

use crate::{ExecutionCommand, ExecutionStateArg};

pub(crate) fn run_execution_command(_root: &Path, command: ExecutionCommand) -> Result<()> {
    match command {
        ExecutionCommand::IntentSubmitted {
            data_dir,
            segment,
            run_id,
            job_id,
            session_id,
            actor_id,
            intent_id,
            intent_text,
        } => {
            let payload = IntentSubmittedV1 {
                schema_version: 1,
                intent_id,
                session_id: session_id.clone().unwrap_or_default(),
                actor_id: actor_id.clone(),
                priority: IntentPriorityV1::Normal as i32,
                intent_text,
                attachment: Vec::new(),
                attachment_type: String::new(),
                submitted_at_unix_ms: now_unix_ms(),
            }
            .encode_to_vec();
            append_event(
                data_dir,
                &segment,
                run_id,
                job_id,
                session_id,
                actor_id,
                TimelineEventTypeV1::TimelineEventTypeJobOpened,
                "os.intent_submitted.v1",
                payload,
            )
        }
        ExecutionCommand::ExecutionStarted {
            data_dir,
            segment,
            run_id,
            job_id,
            session_id,
            actor_id,
            intent_id,
            executor_id,
        } => {
            let payload = ExecutionStartedV1 {
                schema_version: 1,
                run_id: run_id.clone(),
                intent_id,
                executor_id,
                started_at_unix_ms: now_unix_ms(),
            }
            .encode_to_vec();
            append_event(
                data_dir,
                &segment,
                run_id,
                job_id,
                session_id,
                actor_id,
                TimelineEventTypeV1::TimelineEventTypeJobProgress,
                "os.execution_started.v1",
                payload,
            )
        }
        ExecutionCommand::StepStarted {
            data_dir,
            segment,
            run_id,
            job_id,
            session_id,
            actor_id,
            step_id,
        } => {
            let payload = ExecutionStepStartedV1 {
                schema_version: 1,
                run_id: run_id.clone(),
                step_id,
                started_at_unix_ms: now_unix_ms(),
            }
            .encode_to_vec();
            append_event(
                data_dir,
                &segment,
                run_id,
                job_id,
                session_id,
                actor_id,
                TimelineEventTypeV1::TimelineEventTypeJobProgress,
                "os.execution_step_started.v1",
                payload,
            )
        }
        ExecutionCommand::StepFinished {
            data_dir,
            segment,
            run_id,
            job_id,
            session_id,
            actor_id,
            step_id,
            state,
            reason,
        } => {
            let output = reason.unwrap_or_default().into_bytes();
            let payload = ExecutionStepFinishedV1 {
                schema_version: 1,
                run_id: run_id.clone(),
                step_id,
                state: map_state(state) as i32,
                output: output.clone(),
                output_type: "text/plain".to_string(),
                output_digest: blake3::hash(&output).to_hex().to_string(),
                finished_at_unix_ms: now_unix_ms(),
            }
            .encode_to_vec();
            append_event(
                data_dir,
                &segment,
                run_id,
                job_id,
                session_id,
                actor_id,
                TimelineEventTypeV1::TimelineEventTypeJobProgress,
                "os.execution_step_finished.v1",
                payload,
            )
        }
        ExecutionCommand::ExecutionFinished {
            data_dir,
            segment,
            run_id,
            job_id,
            session_id,
            actor_id,
            state,
            reason,
        } => {
            let mapped = map_state(state);
            let timeline_type = match mapped {
                ExecutionStateV1::Succeeded => TimelineEventTypeV1::TimelineEventTypeJobCompleted,
                ExecutionStateV1::Failed | ExecutionStateV1::Halted => {
                    TimelineEventTypeV1::TimelineEventTypeJobFailed
                }
                _ => TimelineEventTypeV1::TimelineEventTypeJobProgress,
            };
            let payload = ExecutionFinishedV1 {
                schema_version: 1,
                run_id: run_id.clone(),
                state: mapped as i32,
                reason: reason.unwrap_or_default(),
                finished_at_unix_ms: now_unix_ms(),
            }
            .encode_to_vec();
            append_event(
                data_dir,
                &segment,
                run_id,
                job_id,
                session_id,
                actor_id,
                timeline_type,
                "os.execution_finished.v1",
                payload,
            )
        }
        ExecutionCommand::QueryRun {
            data_dir,
            segment,
            run_id,
            limit,
            cursor_offset,
            protobuf,
        } => {
            let data_dir = data_dir.unwrap_or_else(|| PathBuf::from("out/timeline"));
            let mut timeline = StorageBackedTimeline::open_reader(data_dir, &segment)?;
            let result = timeline.query(
                limit,
                cursor_offset,
                TimelineQueryFilter {
                    run_id: Some(run_id),
                    ..TimelineQueryFilter::default()
                },
            )?;
            if protobuf {
                println!("schema=edgerun.timeline.v1.TimelineEventEnvelopeV1");
                println!("events_count={}", result.events.len());
                for row in result.events {
                    println!(
                        "event offset={} hash={} envelope_base64={}",
                        row.offset,
                        row.event_hash,
                        BASE64.encode(row.envelope.encode_to_vec())
                    );
                }
                if let Some(next) = result.next_cursor_offset {
                    println!("next_cursor_offset={next}");
                } else {
                    println!("next_cursor_offset=");
                }
                return Ok(());
            }
            println!("events_count={}", result.events.len());
            for row in result.events {
                println!(
                    "event offset={} hash={} seq={} id={} run_id={} job_id={} session_id={} actor_id={} event_type={} payload_type={} ts_unix_ms={}",
                    row.offset,
                    row.event_hash,
                    row.envelope.seq,
                    row.envelope.event_id,
                    row.envelope.run_id,
                    row.envelope.job_id,
                    row.envelope.session_id,
                    row.envelope.actor_id,
                    row.envelope.event_type,
                    row.envelope.payload_type,
                    row.envelope.ts_unix_ms
                );
            }
            if let Some(next) = result.next_cursor_offset {
                println!("next_cursor_offset={next}");
            } else {
                println!("next_cursor_offset=");
            }
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn append_event(
    data_dir: Option<PathBuf>,
    segment: &str,
    run_id: String,
    job_id: Option<String>,
    session_id: Option<String>,
    actor_id: String,
    event_type: TimelineEventTypeV1,
    payload_type: &str,
    payload: Vec<u8>,
) -> Result<()> {
    let data_dir = data_dir.unwrap_or_else(|| PathBuf::from("out/timeline"));
    let envelope = StorageBackedTimeline::build_envelope(
        run_id,
        job_id.unwrap_or_default(),
        session_id.unwrap_or_default(),
        TimelineActorTypeV1::TimelineActorTypeSystem,
        actor_id,
        event_type,
        payload_type.to_string(),
        payload,
    );
    let mut timeline = StorageBackedTimeline::open_writer(data_dir, segment)?;
    let offset = timeline.publish(&envelope)?;
    println!("ok=true");
    println!("offset={offset}");
    println!("event_id={}", envelope.event_id);
    Ok(())
}

fn map_state(state: ExecutionStateArg) -> ExecutionStateV1 {
    match state {
        ExecutionStateArg::Pending => ExecutionStateV1::Pending,
        ExecutionStateArg::Running => ExecutionStateV1::Running,
        ExecutionStateArg::Succeeded => ExecutionStateV1::Succeeded,
        ExecutionStateArg::Failed => ExecutionStateV1::Failed,
        ExecutionStateArg::Halted => ExecutionStateV1::Halted,
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
