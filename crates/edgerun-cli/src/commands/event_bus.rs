// SPDX-License-Identifier: Apache-2.0
use std::path::{Path, PathBuf};

use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use edgerun_storage::event_bus::{BusQueryFilter, EventBus, StorageBackedEventBus};

use crate::EventBusCommand;

pub(crate) fn run_event_bus_command(_root: &Path, command: EventBusCommand) -> Result<()> {
    match command {
        EventBusCommand::Submit {
            data_dir,
            segment,
            nonce,
            publisher,
            signature,
            policy_id,
            recipients,
            payload_type,
            payload_base64,
        } => {
            let data_dir = data_dir.unwrap_or_else(|| PathBuf::from("out/event-bus"));
            let payload = BASE64.decode(payload_base64)?;
            let envelope = StorageBackedEventBus::build_envelope(
                nonce,
                publisher,
                signature,
                policy_id,
                recipients,
                payload_type,
                payload,
            );
            let mut bus = StorageBackedEventBus::open_writer(data_dir, &segment)?;
            let offset = bus.publish(&envelope)?;
            println!("ok=true");
            println!("offset={offset}");
            println!("event_id={}", envelope.event_id);
            println!("nonce={}", envelope.nonce);
            Ok(())
        }
        EventBusCommand::Query {
            data_dir,
            segment,
            limit,
            cursor_offset,
            publisher,
            payload_type,
        } => {
            let data_dir = data_dir.unwrap_or_else(|| PathBuf::from("out/event-bus"));
            let mut bus = StorageBackedEventBus::open_reader(data_dir, &segment)?;
            let result = bus.query(
                limit,
                cursor_offset,
                BusQueryFilter {
                    publisher,
                    payload_type,
                },
            )?;
            println!("events_count={}", result.events.len());
            for row in result.events {
                println!(
                    "event offset={} hash={} id={} nonce={} publisher={} policy_id={} payload_type={} payload_base64={} ts_unix_ms={}",
                    row.offset,
                    row.event_hash,
                    row.envelope.event_id,
                    row.envelope.nonce,
                    row.envelope.publisher,
                    row.envelope.policy_id,
                    row.envelope.payload_type,
                    BASE64.encode(row.envelope.payload),
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
        EventBusCommand::Status { data_dir, segment } => {
            let data_dir = data_dir.unwrap_or_else(|| PathBuf::from("out/event-bus"));
            let bus = StorageBackedEventBus::open_reader(data_dir, &segment)?;
            let status = bus.status()?;
            println!("ok=true");
            println!("schema_version={}", status.schema_version);
            println!("phase={}", status.phase);
            println!("policy_version={}", status.policy_version);
            println!("last_applied_event_id={}", status.last_applied_event_id);
            println!("last_offset={}", status.last_offset);
            println!(
                "latest_chain_progress_event_id={}",
                status.latest_chain_progress_event_id
            );
            println!("storage_ok={}", status.storage_ok);
            Ok(())
        }
    }
}
