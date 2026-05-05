//! Codex-facing bridge records archived through Edgerun's rkyv wire boundary.
//!
//! This crate is deliberately small. It gives local Codex integrations a
//! concrete Edgerun-backed context record without adding a second wire format
//! or a compatibility path.

use edgerun_wire::Archive;
use edgerun_wire::Deserialize;
use edgerun_wire::Serialize;
use edgerun_wire::WireError;

pub const WIRE_PROTOCOL: &str = edgerun_wire::WIRE_PROTOCOL;

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct CodexBoostRecord {
    pub schema_version: u16,
    pub kind: CodexBoostRecordKind,
    pub source: String,
    pub body: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub enum CodexBoostRecordKind {
    MemorySummary,
    RepoContext,
    ToolObservation,
}

impl CodexBoostRecord {
    pub fn repo_context(source: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            schema_version: 1,
            kind: CodexBoostRecordKind::RepoContext,
            source: source.into(),
            body: body.into(),
        }
    }
}

pub fn archive_record(record: &CodexBoostRecord) -> Result<Vec<u8>, WireError> {
    Ok(edgerun_wire::to_bytes::<WireError>(record)?.into_vec())
}

pub fn decode_record(bytes: &[u8]) -> Result<CodexBoostRecord, WireError> {
    edgerun_wire::from_bytes::<CodexBoostRecord, WireError>(bytes)
}

pub fn access_record(bytes: &[u8]) -> Result<&edgerun_wire::Archived<CodexBoostRecord>, WireError> {
    edgerun_wire::access::<edgerun_wire::Archived<CodexBoostRecord>, WireError>(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archives_codex_context_through_edgerun_wire() {
        let record = CodexBoostRecord::repo_context(
            "AGENTS.md",
            "internal Edgerun wire protocol is rkyv only",
        );

        let bytes = archive_record(&record).expect("archive record");
        let archived = access_record(&bytes).expect("access archived record");
        let decoded = decode_record(&bytes).expect("decode record");

        assert_eq!(archived.schema_version, record.schema_version);
        assert_eq!(decoded, record);
        assert_eq!(WIRE_PROTOCOL, "rkyv");
    }
}
