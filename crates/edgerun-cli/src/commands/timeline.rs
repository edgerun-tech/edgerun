// SPDX-License-Identifier: Apache-2.0
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use edgerun_storage::timeline::{
    InteractionPayloadV1, StorageBackedTimeline, TimelineActorTypeV1, TimelineEventTypeV1,
    TimelineQueryFilter,
};
use prost::Message;

use crate::{TimelineActor, TimelineCommand, TimelineEventKind};

pub(crate) fn run_timeline_command(_root: &Path, command: TimelineCommand) -> Result<()> {
    match command {
        TimelineCommand::Append {
            data_dir,
            segment,
            run_id,
            job_id,
            session_id,
            actor,
            actor_id,
            kind,
            text,
            text_file,
            stdin,
        } => {
            let data_dir = data_dir.unwrap_or_else(|| PathBuf::from("out/timeline"));
            let body = read_text_payload(text, text_file, stdin)?;
            let payload = InteractionPayloadV1 { text: body }.encode_to_vec();
            let envelope = StorageBackedTimeline::build_envelope(
                run_id,
                job_id.unwrap_or_default(),
                session_id.unwrap_or_default(),
                map_actor(actor),
                actor_id,
                map_kind(kind),
                "interaction.v1".to_string(),
                payload,
            );
            let mut timeline = StorageBackedTimeline::open_writer(data_dir, &segment)?;
            let offset = timeline.publish(&envelope)?;
            println!("ok=true");
            println!("offset={offset}");
            println!("event_id={}", envelope.event_id);
            println!("seq={}", envelope.seq);
            Ok(())
        }
        TimelineCommand::Query {
            data_dir,
            segment,
            limit,
            cursor_offset,
            kind,
            run_id,
            job_id,
            session_id,
            actor_id,
            payload_type,
        } => {
            let data_dir = data_dir.unwrap_or_else(|| PathBuf::from("out/timeline"));
            let mut timeline = StorageBackedTimeline::open_reader(data_dir, &segment)?;
            let result = timeline.query(
                limit,
                cursor_offset,
                TimelineQueryFilter {
                    event_type: kind.map(map_kind),
                    run_id,
                    job_id,
                    session_id,
                    actor_id,
                    payload_type,
                },
            )?;
            println!("events_count={}", result.events.len());
            for row in result.events {
                let mut text = String::new();
                if row.envelope.payload_type == "interaction.v1" {
                    if let Ok(payload) =
                        InteractionPayloadV1::decode(row.envelope.payload.as_slice())
                    {
                        text = payload.text;
                    }
                }
                println!(
                    "event offset={} hash={} seq={} id={} run_id={} job_id={} session_id={} actor_type={} actor_id={} event_type={} payload_type={} text_base64={} ts_unix_ms={}",
                    row.offset,
                    row.event_hash,
                    row.envelope.seq,
                    row.envelope.event_id,
                    row.envelope.run_id,
                    row.envelope.job_id,
                    row.envelope.session_id,
                    row.envelope.actor_type,
                    row.envelope.actor_id,
                    row.envelope.event_type,
                    row.envelope.payload_type,
                    BASE64.encode(text.as_bytes()),
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
        TimelineCommand::Status { data_dir, segment } => {
            let data_dir = data_dir.unwrap_or_else(|| PathBuf::from("out/timeline"));
            let timeline = StorageBackedTimeline::open_reader(data_dir, &segment)?;
            let status = timeline.status()?;
            println!("ok=true");
            println!("schema_version={}", status.schema_version);
            println!("events_total={}", status.events_total);
            println!("unique_run_ids={}", status.unique_run_ids);
            println!("unique_job_ids={}", status.unique_job_ids);
            println!("unique_session_ids={}", status.unique_session_ids);
            println!("last_event_id={}", status.last_event_id);
            println!("last_seq={}", status.last_seq);
            println!("last_ts_unix_ms={}", status.last_ts_unix_ms);
            Ok(())
        }
    }
}

fn map_actor(value: TimelineActor) -> TimelineActorTypeV1 {
    match value {
        TimelineActor::User => TimelineActorTypeV1::TimelineActorTypeUser,
        TimelineActor::Agent => TimelineActorTypeV1::TimelineActorTypeAgent,
        TimelineActor::System => TimelineActorTypeV1::TimelineActorTypeSystem,
    }
}

fn map_kind(value: TimelineEventKind) -> TimelineEventTypeV1 {
    match value {
        TimelineEventKind::UserInput => TimelineEventTypeV1::TimelineEventTypeInteractionUserInput,
        TimelineEventKind::AgentOutput => {
            TimelineEventTypeV1::TimelineEventTypeInteractionAgentOutput
        }
    }
}

fn read_text_payload(
    text: Option<String>,
    text_file: Option<PathBuf>,
    stdin: bool,
) -> Result<String> {
    if let Some(text) = text {
        return Ok(text);
    }
    if let Some(file) = text_file {
        return Ok(std::fs::read_to_string(file)?);
    }
    if stdin {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        return Ok(buf);
    }
    bail!("provide --text, --text-file, or --stdin")
}
