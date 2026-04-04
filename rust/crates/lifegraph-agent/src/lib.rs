use std::path::{Path, PathBuf};

use lifegraph_proto::lifegraph::v0::stream::EventEnvelope;
use prost::Message;
use surrealdb::engine::any::{connect, Any};
use surrealdb::Surreal;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentEndpoint {
    Memory,
    SurrealKv { path: PathBuf },
    Remote { url: String },
}

impl AgentEndpoint {
    fn to_connection_string(&self) -> Result<String, AgentError> {
        match self {
            Self::Memory => Ok("mem://".to_owned()),
            Self::SurrealKv { path } => surreal_kv_endpoint(path),
            Self::Remote { url } => {
                if url.trim().is_empty() {
                    Err(AgentError::InvalidConfig(
                        "remote SurrealDB URL must not be empty",
                    ))
                } else {
                    Ok(url.clone())
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentConfig {
    pub agent_id: String,
    pub namespace: String,
    pub database: String,
    pub endpoint: AgentEndpoint,
    pub blob_root: PathBuf,
}

#[derive(Debug)]
pub enum AgentError {
    InvalidConfig(&'static str),
    Io(std::io::Error),
    Db(surrealdb::Error),
    Encode(prost::EncodeError),
}

impl core::fmt::Display for AgentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidConfig(msg) => f.write_str(msg),
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Db(err) => write!(f, "surrealdb error: {err}"),
            Self::Encode(err) => write!(f, "proto encoding error: {err}"),
        }
    }
}

impl std::error::Error for AgentError {}

impl From<std::io::Error> for AgentError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<surrealdb::Error> for AgentError {
    fn from(value: surrealdb::Error) -> Self {
        Self::Db(value)
    }
}

impl From<prost::EncodeError> for AgentError {
    fn from(value: prost::EncodeError) -> Self {
        Self::Encode(value)
    }
}

pub struct LifegraphAgent {
    config: AgentConfig,
    db: Surreal<Any>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobMetadata {
    pub blob_id: String,
    pub relative_path: PathBuf,
    pub encryption_suite: String,
    pub recipient_count: u32,
    pub size_bytes: u64,
    pub present: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PutBlobRequest {
    pub blob_id: String,
    pub ciphertext: Vec<u8>,
    pub encryption_suite: String,
    pub recipients: Vec<String>,
}

impl LifegraphAgent {
    pub async fn connect(config: AgentConfig) -> Result<Self, AgentError> {
        validate_config(&config)?;
        std::fs::create_dir_all(&config.blob_root)?;
        let db = connect(config.endpoint.to_connection_string()?).await?;
        db.use_ns(&config.namespace)
            .use_db(&config.database)
            .await?;

        let agent = Self { config, db };
        agent.initialize_schema().await?;
        agent.register_agent().await?;
        Ok(agent)
    }

    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    pub fn db(&self) -> &Surreal<Any> {
        &self.db
    }

    pub async fn initialize_schema(&self) -> Result<(), AgentError> {
        self.db
            .query(
                "
DEFINE TABLE IF NOT EXISTS agent SCHEMALESS;
DEFINE TABLE IF NOT EXISTS event SCHEMALESS;
DEFINE TABLE IF NOT EXISTS blob SCHEMALESS;
DEFINE TABLE IF NOT EXISTS chain_next TYPE RELATION IN event OUT event SCHEMALESS;
DEFINE INDEX IF NOT EXISTS agent_agent_id ON TABLE agent COLUMNS agent_id UNIQUE;
DEFINE INDEX IF NOT EXISTS event_record_id ON TABLE event COLUMNS record_id UNIQUE;
DEFINE INDEX IF NOT EXISTS event_stream_seq ON TABLE event COLUMNS stream_id_hex, seq UNIQUE;
DEFINE INDEX IF NOT EXISTS blob_blob_id ON TABLE blob COLUMNS blob_id UNIQUE;
",
            )
            .await?;
        Ok(())
    }

    pub async fn register_agent(&self) -> Result<(), AgentError> {
        let agent_id = self.config.agent_id.clone();
        let namespace = self.config.namespace.clone();
        let database = self.config.database.clone();
        let endpoint = self.config.endpoint.to_connection_string()?;

        self.db
            .query(
                "UPSERT type::thing('agent', $agent_id) MERGE {
                    agent_id: $agent_id,
                    namespace: $namespace,
                    database: $database,
                    endpoint: $endpoint,
                    blob_root: $blob_root,
                    kind: 'lifegraph-agent'
                };",
            )
            .bind(("agent_id", agent_id))
            .bind(("namespace", namespace))
            .bind(("database", database))
            .bind(("endpoint", endpoint))
            .bind(("blob_root", self.config.blob_root.display().to_string()))
            .await?;
        Ok(())
    }

    pub async fn heartbeat(&self, unix_ms: i64) -> Result<(), AgentError> {
        let agent_id = self.config.agent_id.clone();

        self.db
            .query(
                "UPSERT type::thing('agent', $agent_id) MERGE {
                    agent_id: $agent_id,
                    status: 'live',
                    last_seen_unix_ms: $last_seen_unix_ms
                };",
            )
            .bind(("agent_id", agent_id))
            .bind(("last_seen_unix_ms", unix_ms))
            .await?;
        Ok(())
    }

    pub async fn append_event(
        &self,
        record_id: &str,
        event_hash: &str,
        event: &EventEnvelope,
        accepted_at_ms: i64,
        previous_record_id: Option<&str>,
    ) -> Result<(), AgentError> {
        let record_id = record_id.to_owned();
        let event_hash = event_hash.to_owned();
        let mut event_bytes = Vec::new();
        event.encode(&mut event_bytes)?;
        let payload_hex = hex::encode(event_bytes);
        let prev_event_hash = event
            .prev_event_hash
            .as_ref()
            .map(|digest| hex::encode(&digest.value));
        let stream_id_hex = hex::encode(&event.stream_id);
        let agent_id = self.config.agent_id.clone();

        self.db
            .query(
                "UPSERT type::thing('event', $record_id) MERGE {
                    record_id: $record_id,
                    event_hash: $event_hash,
                    stream_id_hex: $stream_id_hex,
                    seq: $seq,
                    prev_event_hash: $prev_event_hash,
                    payload_proto: 'lifegraph.v0.stream.EventEnvelope',
                    payload_encoding: 'prost',
                    payload_hex: $payload_hex,
                    accepted_at_ms: $accepted_at_ms,
                    agent_id: $agent_id
                };",
            )
            .bind(("record_id", record_id.clone()))
            .bind(("event_hash", event_hash.clone()))
            .bind(("stream_id_hex", stream_id_hex.clone()))
            .bind(("seq", event.seq))
            .bind(("prev_event_hash", prev_event_hash))
            .bind(("payload_hex", payload_hex))
            .bind(("accepted_at_ms", accepted_at_ms))
            .bind(("agent_id", agent_id.clone()))
            .await?;

        if let Some(previous_record_id) = previous_record_id {
            let previous_record_id = previous_record_id.to_owned();
            self.db
                .query(
                    "RELATE type::thing('event', $previous_record_id)->chain_next->type::thing('event', $record_id)
                     CONTENT {
                        stream_id_hex: $stream_id_hex,
                        seq: $seq,
                        event_hash: $event_hash,
                        accepted_at_ms: $accepted_at_ms
                     };",
                )
                .bind(("previous_record_id", previous_record_id))
                .bind(("record_id", record_id))
                .bind(("stream_id_hex", stream_id_hex))
                .bind(("seq", event.seq))
                .bind(("event_hash", event_hash))
                .bind(("accepted_at_ms", accepted_at_ms))
                .await?;
        }

        Ok(())
    }

    pub async fn put_blob(&self, request: &PutBlobRequest) -> Result<BlobMetadata, AgentError> {
        if request.blob_id.trim().is_empty() {
            return Err(AgentError::InvalidConfig("blob_id must not be empty"));
        }
        if request.encryption_suite.trim().is_empty() {
            return Err(AgentError::InvalidConfig(
                "persisted blobs must declare a non-empty encryption suite",
            ));
        }
        if request.recipients.is_empty() {
            return Err(AgentError::InvalidConfig(
                "persisted blobs must have at least one recipient",
            ));
        }

        let relative_path = blob_relative_path(&request.blob_id);
        let absolute_path = self.config.blob_root.join(&relative_path);
        if let Some(parent) = absolute_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&absolute_path, &request.ciphertext)?;

        let blob_id = request.blob_id.clone();
        let encryption_suite = request.encryption_suite.clone();
        let recipients = request.recipients.clone();
        let relative_path_string = relative_path.display().to_string();
        let absolute_path_string = absolute_path.display().to_string();
        let size_bytes = request.ciphertext.len() as u64;

        self.db
            .query(
                "UPSERT type::thing('blob', $blob_id) MERGE {
                    blob_id: $blob_id,
                    relative_path: $relative_path,
                    absolute_path: $absolute_path,
                    encryption_suite: $encryption_suite,
                    recipients: $recipients,
                    recipient_count: $recipient_count,
                    size_bytes: $size_bytes,
                    present: true
                };",
            )
            .bind(("blob_id", blob_id.clone()))
            .bind(("relative_path", relative_path_string))
            .bind(("absolute_path", absolute_path_string))
            .bind(("encryption_suite", encryption_suite.clone()))
            .bind(("recipients", recipients.clone()))
            .bind(("recipient_count", recipients.len() as u64))
            .bind(("size_bytes", size_bytes))
            .await?;

        Ok(BlobMetadata {
            blob_id,
            relative_path,
            encryption_suite,
            recipient_count: recipients.len() as u32,
            size_bytes,
            present: true,
        })
    }
}

fn validate_config(config: &AgentConfig) -> Result<(), AgentError> {
    if config.agent_id.trim().is_empty() {
        return Err(AgentError::InvalidConfig("agent_id must not be empty"));
    }
    if config.namespace.trim().is_empty() {
        return Err(AgentError::InvalidConfig("namespace must not be empty"));
    }
    if config.database.trim().is_empty() {
        return Err(AgentError::InvalidConfig("database must not be empty"));
    }
    if config.blob_root.as_os_str().is_empty() {
        return Err(AgentError::InvalidConfig("blob_root must not be empty"));
    }
    Ok(())
}

fn surreal_kv_endpoint(path: &Path) -> Result<String, AgentError> {
    if path.as_os_str().is_empty() {
        return Err(AgentError::InvalidConfig(
            "SurrealKV path must not be empty",
        ));
    }
    Ok(format!("surrealkv://{}", path.display()))
}

fn blob_relative_path(blob_id: &str) -> PathBuf {
    let a = &blob_id[0..blob_id.len().min(2)];
    let b = &blob_id[blob_id.len().min(2)..blob_id.len().min(4)];
    PathBuf::from(a).join(b).join(blob_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifegraph_proto::lifegraph::v0::common::Digest;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("lifegraph-agent-{name}-{nanos}"))
    }

    fn test_config(name: &str) -> AgentConfig {
        let root = test_root(name);
        AgentConfig {
            agent_id: format!("agent-{name}"),
            namespace: "lifegraph_test".to_owned(),
            database: "core".to_owned(),
            endpoint: AgentEndpoint::SurrealKv {
                path: root.join("surreal"),
            },
            blob_root: root.join("blobs"),
        }
    }

    fn sample_event(stream_id: &[u8], seq: u64, prev: Option<&[u8]>) -> EventEnvelope {
        EventEnvelope {
            envelope_version: 1,
            stream_id: stream_id.to_vec(),
            seq,
            prev_event_hash: prev.map(|bytes| Digest {
                algorithm: 1,
                value: bytes.to_vec(),
            }),
            event_type: 1,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: Vec::new(),
            related_commands: Vec::new(),
            related_objects: Vec::new(),
            related_delegations: Vec::new(),
            related_revocations: Vec::new(),
            event_metadata: None,
            signature: None,
        }
    }

    #[tokio::test]
    async fn put_blob_writes_ciphertext_to_disk() {
        let config = test_config("blob");
        let blob_root = config.blob_root.clone();
        let agent = LifegraphAgent::connect(config).await.unwrap();

        let meta = agent
            .put_blob(&PutBlobRequest {
                blob_id: "a1b2c3d4".to_owned(),
                ciphertext: vec![1, 2, 3, 4, 5],
                encryption_suite: "test.v1".to_owned(),
                recipients: vec!["node:test".to_owned()],
            })
            .await
            .unwrap();

        let blob_path = blob_root.join(&meta.relative_path);
        assert_eq!(std::fs::read(blob_path).unwrap(), vec![1, 2, 3, 4, 5]);
        assert_eq!(meta.recipient_count, 1);
        assert_eq!(meta.size_bytes, 5);
    }

    #[tokio::test]
    async fn append_event_records_sequential_chain() {
        let agent = LifegraphAgent::connect(test_config("events"))
            .await
            .unwrap();
        let stream_id = b"stream-1";
        let event1 = sample_event(stream_id, 1, None);
        let event2 = sample_event(stream_id, 2, Some(b"prev-hash-1"));

        agent
            .append_event("record-1", "hash-1", &event1, 1000, None)
            .await
            .unwrap();
        agent
            .append_event("record-2", "hash-2", &event2, 1001, Some("record-1"))
            .await
            .unwrap();

        let stream_id_hex = hex::encode(stream_id);
        let mut response = agent
            .db()
            .query("RETURN count((SELECT VALUE id FROM event WHERE stream_id_hex = $stream_id_hex)); RETURN count((SELECT VALUE id FROM chain_next));")
            .bind(("stream_id_hex", stream_id_hex))
            .await
            .unwrap();

        let event_count: u64 = response.take(0).unwrap();
        let edge_count: u64 = response.take(1).unwrap();
        assert_eq!(event_count, 2);
        assert_eq!(edge_count, 1);
    }
}
