// SPDX-License-Identifier: GPL-2.0-only
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use prost::Message;

use crate::context_engine::StorageBackedContextEngine;
use crate::docker_logger::{DockerLogAdapter, DockerLogDecoder};
use crate::durability::DurabilityLevel;
use crate::event::{ActorId, Event as StorageEvent, StreamId};
use crate::{StorageEngine, StorageError};

pub use crate::manifest::proto::{
    BlobStoredV1, BranchCreatedV1, BranchHeadMovedV1, FsDeltaAppliedV1, FsDeltaProposedV1,
    FsDeltaRejectedV1, LogEntryAppendedV1, MaterializedBranchStateV1, MaterializedRepoStateV1,
    PartitionDeclaredV1, SnapshotCheckpointedV1, SourceImportRequestV1, SourceImportedV1,
    VfsCursorV1, VfsEventEnvelopeV1, VfsEventTypeV1, VfsModeV1, VfsSourceKindV1,
};

static VFS_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct VirtualFsQueryRow {
    pub offset: u64,
    pub event_hash: String,
    pub envelope: VfsEventEnvelopeV1,
}

#[derive(Debug, Clone)]
pub struct VirtualFsQueryResult {
    pub events: Vec<VirtualFsQueryRow>,
    pub next_cursor_offset: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct VirtualFsQueryFilter {
    pub event_type: Option<VfsEventTypeV1>,
    pub branch_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct VirtualBranchState {
    pub branch_id: String,
    pub head_seq: u64,
    pub head_event_hash: Vec<u8>,
    pub proposed_delta_count: u64,
    pub applied_delta_count: u64,
    pub rejected_delta_count: u64,
    pub log_entry_count: u64,
}

#[derive(Debug, Clone)]
pub struct VirtualRepoState {
    pub repo_id: String,
    pub source_kind: VfsSourceKindV1,
    pub mode: VfsModeV1,
    pub imported: bool,
    pub max_seq: u64,
    pub branches: Vec<VirtualBranchState>,
}

#[derive(Debug, Clone)]
pub struct ImportReport {
    pub source_kind: VfsSourceKindV1,
    pub mode: VfsModeV1,
    pub imported_object_count: u64,
    pub imported_bytes: u64,
    pub root_projection_hash: Vec<u8>,
    pub first_event_offset: u64,
    pub last_event_offset: u64,
}

#[derive(Debug, Clone)]
pub struct FsSnapshotPolicy {
    pub include_hidden: bool,
    pub max_file_size_bytes: u64,
}

impl Default for FsSnapshotPolicy {
    fn default() -> Self {
        Self {
            include_hidden: false,
            max_file_size_bytes: 16 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VfsSnapshotPolicy {
    /// 0 disables automatic snapshots.
    pub auto_checkpoint_every_applied: u64,
}

impl Default for VfsSnapshotPolicy {
    fn default() -> Self {
        Self {
            auto_checkpoint_every_applied: 50,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogIngestEntry {
    pub entry_key: String,
    pub entry_payload: Vec<u8>,
    pub idempotency_key: String,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct LogIngestOutcome {
    pub appended: u64,
    pub skipped_idempotent: u64,
    pub first_event_offset: Option<u64>,
    pub last_event_offset: Option<u64>,
    pub declared_partition: bool,
}

impl VirtualRepoState {
    fn new(repo_id: String) -> Self {
        Self {
            repo_id,
            source_kind: VfsSourceKindV1::VfsSourceKindUnspecified,
            mode: VfsModeV1::VfsModeUnspecified,
            imported: false,
            max_seq: 0,
            branches: Vec::new(),
        }
    }

    fn ensure_branch(&mut self, branch_id: &str) -> &mut VirtualBranchState {
        if let Some(idx) = self.branches.iter().position(|b| b.branch_id == branch_id) {
            return &mut self.branches[idx];
        }
        self.branches.push(VirtualBranchState {
            branch_id: branch_id.to_string(),
            ..VirtualBranchState::default()
        });
        self.branches.last_mut().expect("branch exists after push")
    }

    fn into_proto(mut self) -> MaterializedRepoStateV1 {
        self.branches.sort_by(|a, b| a.branch_id.cmp(&b.branch_id));
        MaterializedRepoStateV1 {
            schema_version: 1,
            repo_id: self.repo_id,
            source_kind: self.source_kind as i32,
            mode: self.mode as i32,
            imported: self.imported,
            branches: self
                .branches
                .into_iter()
                .map(|b| MaterializedBranchStateV1 {
                    branch_id: b.branch_id,
                    head_seq: b.head_seq,
                    head_event_hash: b.head_event_hash,
                    proposed_delta_count: b.proposed_delta_count,
                    applied_delta_count: b.applied_delta_count,
                    rejected_delta_count: b.rejected_delta_count,
                    log_entry_count: b.log_entry_count,
                })
                .collect(),
        }
    }

    fn from_proto(proto: MaterializedRepoStateV1) -> Self {
        Self {
            repo_id: proto.repo_id,
            source_kind: source_kind_from_i32(proto.source_kind),
            mode: mode_from_i32(proto.mode),
            imported: proto.imported,
            max_seq: 0,
            branches: proto
                .branches
                .into_iter()
                .map(|b| VirtualBranchState {
                    branch_id: b.branch_id,
                    head_seq: b.head_seq,
                    head_event_hash: b.head_event_hash,
                    proposed_delta_count: b.proposed_delta_count,
                    applied_delta_count: b.applied_delta_count,
                    rejected_delta_count: b.rejected_delta_count,
                    log_entry_count: b.log_entry_count,
                })
                .collect(),
        }
    }
}

pub struct StorageBackedVirtualFs {
    engine: StorageEngine,
    repo_id: String,
    segment: String,
    stream_id: StreamId,
    actor_id: ActorId,
    envelopes_cache: RwLock<Option<Vec<VfsEventEnvelopeV1>>>,
    branch_envelopes_cache: RwLock<HashMap<String, Vec<VfsEventEnvelopeV1>>>,
    log_partition_cache: RwLock<HashMap<String, ExistingLogState>>,
    proposal_cache: RwLock<HashMap<String, FsDeltaProposedV1>>,
    proposed_payload_cache: RwLock<HashMap<String, FsDeltaProposedV1>>,
    partition_declared_payload_cache: RwLock<HashMap<String, PartitionDeclaredV1>>,
    log_entry_payload_cache: RwLock<HashMap<String, LogEntryAppendedV1>>,
    snapshot_payload_cache: RwLock<HashMap<String, SnapshotCheckpointedV1>>,
    append_cursor_state: RwLock<Option<AppendCursorState>>,
    snapshot_policy: VfsSnapshotPolicy,
    snapshot_progress: RwLock<Option<SnapshotProgress>>,
}

impl StorageBackedVirtualFs {
    pub fn open_writer(data_dir: PathBuf, repo_id: &str) -> Result<Self, StorageError> {
        Self::open_writer_with_snapshot_policy(data_dir, repo_id, VfsSnapshotPolicy::default())
    }

    pub fn open_writer_with_snapshot_policy(
        data_dir: PathBuf,
        repo_id: &str,
        snapshot_policy: VfsSnapshotPolicy,
    ) -> Result<Self, StorageError> {
        let engine = StorageEngine::new(data_dir)?;
        let normalized = normalize_repo_id(repo_id);
        Ok(Self {
            engine,
            repo_id: normalized.clone(),
            segment: format!("vfs.{}.journal", normalized),
            stream_id: stream_id_for_seed(&format!("edgerun-vfs-stream:{normalized}")),
            actor_id: actor_id_for_seed(&format!("edgerun-vfs-actor:{normalized}")),
            envelopes_cache: RwLock::new(None),
            branch_envelopes_cache: RwLock::new(HashMap::new()),
            log_partition_cache: RwLock::new(HashMap::new()),
            proposal_cache: RwLock::new(HashMap::new()),
            proposed_payload_cache: RwLock::new(HashMap::new()),
            partition_declared_payload_cache: RwLock::new(HashMap::new()),
            log_entry_payload_cache: RwLock::new(HashMap::new()),
            snapshot_payload_cache: RwLock::new(HashMap::new()),
            append_cursor_state: RwLock::new(None),
            snapshot_policy,
            snapshot_progress: RwLock::new(None),
        })
    }

    pub fn open_reader(data_dir: PathBuf, repo_id: &str) -> Result<Self, StorageError> {
        Self::open_writer(data_dir, repo_id)
    }

    pub fn import_source(
        &mut self,
        req: &SourceImportRequestV1,
        imported_object_count: u64,
        imported_bytes: u64,
        root_projection_hash: Vec<u8>,
        metadata_json: String,
    ) -> Result<u64, StorageError> {
        if req.repo_id != self.repo_id {
            return Err(StorageError::InvalidData(format!(
                "repo_id mismatch: request={}, writer={}",
                req.repo_id, self.repo_id
            )));
        }
        let payload = SourceImportedV1 {
            schema_version: 1,
            repo_id: req.repo_id.clone(),
            source_kind: req.source_kind,
            mode: req.mode,
            source_locator: req.source_locator.clone(),
            source_ref: req.source_ref.clone(),
            imported_object_count,
            imported_bytes,
            root_projection_hash,
            metadata_json,
        }
        .encode_to_vec();

        self.append_typed_event(
            "main",
            VfsEventTypeV1::VfsEventTypeSourceImported,
            "storage.vfs.source_imported.v1",
            payload,
        )
    }

    pub fn import_fs_snapshot(
        &mut self,
        root_path: &Path,
        policy: FsSnapshotPolicy,
        initiated_by: &str,
    ) -> Result<ImportReport, StorageError> {
        let files = collect_snapshot_files(root_path, &policy)?;
        let mut first_offset: Option<u64> = None;
        let mut last_file_offset: Option<u64> = None;
        let mut imported_bytes = 0u64;
        let mut path_hashes: Vec<(String, Vec<u8>)> = Vec::with_capacity(files.len());

        for file in &files {
            let hash = blake3::hash(&file.content).as_bytes().to_vec();
            imported_bytes = imported_bytes.saturating_add(file.content.len() as u64);
            path_hashes.push((file.path.clone(), hash.clone()));
            let off = self.append_blob(
                "main",
                &file.path,
                hash,
                file.content.clone(),
                format!("fs:{}:{}", self.repo_id, file.path),
            )?;
            if first_offset.is_none() {
                first_offset = Some(off);
            }
            last_file_offset = Some(off);
        }

        path_hashes.sort_by(|a, b| a.0.cmp(&b.0));
        let root_projection_hash = compute_root_projection_hash(&path_hashes);
        let req = SourceImportRequestV1 {
            schema_version: 1,
            repo_id: self.repo_id.clone(),
            source_kind: VfsSourceKindV1::VfsSourceKindFsSnapshot as i32,
            mode: VfsModeV1::VfsModeCode as i32,
            source_locator: root_path.display().to_string(),
            source_ref: "snapshot".to_string(),
            initiated_by: initiated_by.to_string(),
        };
        let source_offset = self.import_source(
            &req,
            files.len() as u64,
            imported_bytes,
            root_projection_hash.clone(),
            format!(
                "{{\"policy\":{{\"include_hidden\":{},\"max_file_size_bytes\":{}}}}}",
                policy.include_hidden, policy.max_file_size_bytes
            ),
        )?;
        if first_offset.is_none() {
            first_offset = Some(source_offset);
        }
        let last_event_offset = source_offset.max(last_file_offset.unwrap_or(source_offset));

        Ok(ImportReport {
            source_kind: VfsSourceKindV1::VfsSourceKindFsSnapshot,
            mode: VfsModeV1::VfsModeCode,
            imported_object_count: files.len() as u64,
            imported_bytes,
            root_projection_hash,
            first_event_offset: first_offset.unwrap_or(source_offset),
            last_event_offset,
        })
    }

    pub fn import_git_repo(
        &mut self,
        repo_path: &Path,
        git_ref: &str,
        initiated_by: &str,
    ) -> Result<ImportReport, StorageError> {
        let ls_tree = run_git(repo_path, &["ls-tree", "-r", "--full-tree", "-z", git_ref])?;
        let mut path_to_hash = parse_git_ls_tree_z(&ls_tree)?;
        path_to_hash.sort_by(|a, b| a.0.cmp(&b.0));

        let mut blob_cache: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for (_, hash_hex) in &path_to_hash {
            if blob_cache.contains_key(hash_hex) {
                continue;
            }
            let blob = run_git(repo_path, &["cat-file", "-p", hash_hex.as_str()])?;
            blob_cache.insert(hash_hex.clone(), blob);
        }

        let mut first_offset: Option<u64> = None;
        let mut last_offset = 0u64;
        let mut imported_bytes = 0u64;
        let mut root_pairs = Vec::with_capacity(path_to_hash.len());

        for (path, hash_hex) in &path_to_hash {
            let Some(content) = blob_cache.get(hash_hex) else {
                return Err(StorageError::InvalidData(format!(
                    "missing blob for hash {}",
                    hash_hex
                )));
            };
            let hash = hex::decode(hash_hex)
                .map_err(|e| StorageError::InvalidData(format!("invalid git blob hash: {e}")))?;
            imported_bytes = imported_bytes.saturating_add(content.len() as u64);
            root_pairs.push((path.clone(), hash.clone()));
            let off = self.append_blob(
                "main",
                path,
                hash,
                content.clone(),
                format!("git:{}:{}:{}", self.repo_id, git_ref, path),
            )?;
            if first_offset.is_none() {
                first_offset = Some(off);
            }
            last_offset = off;
        }

        let root_projection_hash = compute_root_projection_hash(&root_pairs);
        let req = SourceImportRequestV1 {
            schema_version: 1,
            repo_id: self.repo_id.clone(),
            source_kind: VfsSourceKindV1::VfsSourceKindGit as i32,
            mode: VfsModeV1::VfsModeCode as i32,
            source_locator: repo_path.display().to_string(),
            source_ref: git_ref.to_string(),
            initiated_by: initiated_by.to_string(),
        };
        let source_offset = self.import_source(
            &req,
            path_to_hash.len() as u64,
            imported_bytes,
            root_projection_hash.clone(),
            format!(
                "{{\"git_ref\":\"{}\",\"object_count\":{},\"initiated_by\":\"{}\"}}",
                json_escape(git_ref),
                path_to_hash.len(),
                json_escape(initiated_by)
            ),
        )?;
        if first_offset.is_none() {
            first_offset = Some(source_offset);
        }

        Ok(ImportReport {
            source_kind: VfsSourceKindV1::VfsSourceKindGit,
            mode: VfsModeV1::VfsModeCode,
            imported_object_count: path_to_hash.len() as u64,
            imported_bytes,
            root_projection_hash,
            first_event_offset: first_offset.unwrap_or(source_offset),
            last_event_offset: source_offset.max(last_offset),
        })
    }

    pub fn ingest_log_entries(
        &mut self,
        branch_id: &str,
        partition: &str,
        entries: &[LogIngestEntry],
        declared_by: &str,
    ) -> Result<LogIngestOutcome, StorageError> {
        let mut outcome = LogIngestOutcome::default();
        if branch_id.trim().is_empty() || partition.trim().is_empty() {
            return Err(StorageError::InvalidData(
                "branch_id and partition are required".to_string(),
            ));
        }

        let state_key = log_partition_state_key(branch_id, partition);
        let existing = {
            let cached = {
                let cache = self.log_partition_cache.read().unwrap();
                cache.get(&state_key).cloned()
            };
            if let Some(found) = cached {
                found
            } else {
                let envelopes = self.load_repo_envelopes()?;
                let rebuilt = self.collect_existing_log_state(&envelopes, branch_id, partition)?;
                {
                    let mut cache = self.log_partition_cache.write().unwrap();
                    cache.insert(state_key.clone(), rebuilt.clone());
                }
                rebuilt
            }
        };
        let mut known_idempotency = existing.idempotency_keys.clone();
        let mut max_partition_offset = existing.max_partition_offset;
        let mut partition_declared = existing.partition_declared;
        let mut pending = Vec::with_capacity(entries.len().saturating_add(1));

        if !partition_declared {
            outcome.declared_partition = true;
            partition_declared = true;
            pending.push(PendingTypedEvent {
                event_type: VfsEventTypeV1::VfsEventTypePartitionDeclared,
                payload_type: "storage.vfs.partition_declared.v1",
                payload: PartitionDeclaredV1 {
                    schema_version: 1,
                    repo_id: self.repo_id.clone(),
                    branch_id: branch_id.to_string(),
                    partition: partition.to_string(),
                    declared_by: declared_by.to_string(),
                }
                .encode_to_vec(),
            });
        }

        for item in entries {
            if !item.idempotency_key.is_empty() && known_idempotency.contains(&item.idempotency_key)
            {
                outcome.skipped_idempotent = outcome.skipped_idempotent.saturating_add(1);
                continue;
            }
            let partition_offset = item
                .offset
                .unwrap_or_else(|| max_partition_offset.saturating_add(1));
            max_partition_offset = max_partition_offset.max(partition_offset);

            let payload = LogEntryAppendedV1 {
                schema_version: 1,
                repo_id: self.repo_id.clone(),
                branch_id: branch_id.to_string(),
                partition: partition.to_string(),
                offset: partition_offset,
                entry_key: item.entry_key.clone(),
                entry_payload: item.entry_payload.clone(),
                idempotency_key: item.idempotency_key.clone(),
            }
            .encode_to_vec();

            pending.push(PendingTypedEvent {
                event_type: VfsEventTypeV1::VfsEventTypeLogEntryAppended,
                payload_type: "storage.vfs.log_entry_appended.v1",
                payload,
            });
            outcome.appended = outcome.appended.saturating_add(1);
            if !item.idempotency_key.is_empty() {
                known_idempotency.insert(item.idempotency_key.clone());
            }
        }

        if !pending.is_empty() {
            let offsets = self.append_typed_events(branch_id, &pending)?;
            outcome.first_event_offset = offsets.first().copied();
            outcome.last_event_offset = offsets.last().copied();
        }

        self.log_partition_cache.write().unwrap().insert(
            state_key,
            ExistingLogState {
                partition_declared,
                max_partition_offset,
                idempotency_keys: known_idempotency,
            },
        );

        Ok(outcome)
    }

    /// Ingest docker/runtime logs using pluggable decoder+adapter components.
    /// This path stays source-agnostic by mapping records into log-mode entries.
    pub fn ingest_docker_log_lines<D, A>(
        &mut self,
        branch_id: &str,
        lines: &[String],
        declared_by: &str,
        decoder: &mut D,
        adapter: &A,
    ) -> Result<LogIngestOutcome, StorageError>
    where
        D: DockerLogDecoder,
        A: DockerLogAdapter,
    {
        self.ingest_docker_log_lines_batched(
            branch_id,
            lines,
            declared_by,
            decoder,
            adapter,
            usize::MAX,
        )
    }

    /// Batched ingestion variant for long streams (bounds memory and smooths latency).
    pub fn ingest_docker_log_lines_batched<D, A>(
        &mut self,
        branch_id: &str,
        lines: &[String],
        declared_by: &str,
        decoder: &mut D,
        adapter: &A,
        max_partition_batch_entries: usize,
    ) -> Result<LogIngestOutcome, StorageError>
    where
        D: DockerLogDecoder,
        A: DockerLogAdapter,
    {
        let batch_limit = max_partition_batch_entries.max(1);
        let mut by_partition: BTreeMap<String, Vec<LogIngestEntry>> = BTreeMap::new();
        let mut total = LogIngestOutcome::default();
        for line in lines {
            let Some(record) = decoder.decode_line(line)? else {
                continue;
            };
            let partition = adapter.partition_for(&record);
            let entry = adapter.to_log_ingest_entry(&record);
            let buf = by_partition.entry(partition.clone()).or_default();
            buf.push(entry);
            if buf.len() >= batch_limit {
                let batch = by_partition.remove(&partition).unwrap_or_default();
                if !batch.is_empty() {
                    let out =
                        self.ingest_log_entries(branch_id, &partition, &batch, declared_by)?;
                    merge_log_outcome(&mut total, out);
                }
            }
        }

        for (partition, entries) in by_partition {
            let out = self.ingest_log_entries(branch_id, &partition, &entries, declared_by)?;
            merge_log_outcome(&mut total, out);
        }
        Ok(total)
    }

    pub fn create_branch(&mut self, branch: &BranchCreatedV1) -> Result<u64, StorageError> {
        if branch.repo_id != self.repo_id {
            return Err(StorageError::InvalidData(format!(
                "repo_id mismatch: branch={}, writer={}",
                branch.repo_id, self.repo_id
            )));
        }
        self.append_typed_event(
            &branch.branch_id,
            VfsEventTypeV1::VfsEventTypeBranchCreated,
            "storage.vfs.branch_created.v1",
            branch.encode_to_vec(),
        )
    }

    pub fn move_branch_head(&mut self, moved: &BranchHeadMovedV1) -> Result<u64, StorageError> {
        if moved.repo_id != self.repo_id {
            return Err(StorageError::InvalidData(format!(
                "repo_id mismatch: moved={}, writer={}",
                moved.repo_id, self.repo_id
            )));
        }
        self.append_typed_event(
            &moved.branch_id,
            VfsEventTypeV1::VfsEventTypeBranchHeadMoved,
            "storage.vfs.branch_head_moved.v1",
            moved.encode_to_vec(),
        )
    }

    pub fn propose_delta(&mut self, proposal: &FsDeltaProposedV1) -> Result<u64, StorageError> {
        if proposal.repo_id != self.repo_id {
            return Err(StorageError::InvalidData(format!(
                "repo_id mismatch: proposal={}, writer={}",
                proposal.repo_id, self.repo_id
            )));
        }
        let offset = self.append_typed_event(
            &proposal.branch_id,
            VfsEventTypeV1::VfsEventTypeFsDeltaProposed,
            "storage.vfs.fs_delta_proposed.v1",
            proposal.encode_to_vec(),
        )?;
        let mut cache = self.proposal_cache.write().unwrap();
        cache.insert(
            proposal_cache_key(&proposal.branch_id, &proposal.proposal_id),
            proposal.clone(),
        );
        Ok(offset)
    }

    pub fn apply_delta(&mut self, applied: &FsDeltaAppliedV1) -> Result<u64, StorageError> {
        if applied.repo_id != self.repo_id {
            return Err(StorageError::InvalidData(format!(
                "repo_id mismatch: applied={}, writer={}",
                applied.repo_id, self.repo_id
            )));
        }
        let offset = self.append_typed_event(
            &applied.branch_id,
            VfsEventTypeV1::VfsEventTypeFsDeltaApplied,
            "storage.vfs.fs_delta_applied.v1",
            applied.encode_to_vec(),
        )?;

        if !applied.proposal_id.trim().is_empty() {
            if let Some(diff_unified) =
                self.resolve_proposal_diff_unified(&applied.branch_id, &applied.proposal_id)?
            {
                let data_dir = self.engine.data_dir().clone();
                let mut context = StorageBackedContextEngine::open_writer(data_dir, &self.repo_id)?;
                let _ = context.record_touches_from_unified_diff(
                    &applied.branch_id,
                    &diff_unified,
                    &format!("fs_delta_applied:{}", applied.proposal_id),
                )?;
            }
        }

        let _ = self.maybe_auto_checkpoint_after_applied(&applied.branch_id)?;
        Ok(offset)
    }

    pub fn reject_delta(&mut self, rejected: &FsDeltaRejectedV1) -> Result<u64, StorageError> {
        if rejected.repo_id != self.repo_id {
            return Err(StorageError::InvalidData(format!(
                "repo_id mismatch: rejected={}, writer={}",
                rejected.repo_id, self.repo_id
            )));
        }
        self.append_typed_event(
            &rejected.branch_id,
            VfsEventTypeV1::VfsEventTypeFsDeltaRejected,
            "storage.vfs.fs_delta_rejected.v1",
            rejected.encode_to_vec(),
        )
    }

    pub fn append_log_entry(&mut self, entry: &LogEntryAppendedV1) -> Result<u64, StorageError> {
        if entry.repo_id != self.repo_id {
            return Err(StorageError::InvalidData(format!(
                "repo_id mismatch: entry={}, writer={}",
                entry.repo_id, self.repo_id
            )));
        }
        self.append_typed_event(
            &entry.branch_id,
            VfsEventTypeV1::VfsEventTypeLogEntryAppended,
            "storage.vfs.log_entry_appended.v1",
            entry.encode_to_vec(),
        )
    }

    pub fn checkpoint_snapshot(
        &mut self,
        branch_id: &str,
        reason: &str,
    ) -> Result<u64, StorageError> {
        let state = self.materialize()?;
        let snapshot_seq = state.max_seq;
        let state_payload = state.into_proto().encode_to_vec();
        let snapshot = SnapshotCheckpointedV1 {
            schema_version: 1,
            repo_id: self.repo_id.clone(),
            branch_id: branch_id.to_string(),
            snapshot_seq,
            snapshot_hash_blake3: blake3::hash(&state_payload).as_bytes().to_vec(),
            snapshot_payload: state_payload,
            reason: reason.to_string(),
        };
        self.append_typed_event(
            branch_id,
            VfsEventTypeV1::VfsEventTypeSnapshotCheckpointed,
            "storage.vfs.snapshot_checkpointed.v1",
            snapshot.encode_to_vec(),
        )
    }

    pub fn materialize(&self) -> Result<VirtualRepoState, StorageError> {
        let envelopes = self.load_repo_envelopes()?;

        let mut state = VirtualRepoState::new(self.repo_id.clone());
        let mut start_seq = 1u64;
        if let Some((snapshot_event_seq, snapshot)) = self.latest_snapshot(&envelopes)? {
            let mut snap_state =
                MaterializedRepoStateV1::decode(snapshot.snapshot_payload.as_slice())
                    .map_err(|e| {
                        StorageError::InvalidData(format!(
                            "invalid materialized snapshot payload protobuf: {e}"
                        ))
                    })
                    .map(VirtualRepoState::from_proto)?;
            snap_state.max_seq = snapshot.snapshot_seq.max(snapshot_event_seq);
            state = snap_state;
            start_seq = snapshot.snapshot_seq.saturating_add(1);
        }

        for envelope in envelopes {
            if envelope.seq < start_seq {
                continue;
            }
            apply_envelope_to_state(&mut state, &envelope)?;
        }

        state.branches.sort_by(|a, b| a.branch_id.cmp(&b.branch_id));
        Ok(state)
    }

    pub fn query(
        &self,
        limit: usize,
        cursor_offset: u64,
        filter: VirtualFsQueryFilter,
    ) -> Result<VirtualFsQueryResult, StorageError> {
        let rows_all = self.engine.query_segmented_journal_raw(&self.segment)?;
        let mut filtered = Vec::new();
        let mut next_cursor_offset = None;

        for (idx, row) in rows_all.iter().enumerate().skip(cursor_offset as usize) {
            let envelope = decode_envelope(&row.event.payload)?;
            if envelope.repo_id != self.repo_id {
                continue;
            }
            if let Some(expected) = filter.event_type {
                if envelope.event_type != expected as i32 {
                    continue;
                }
            }
            if let Some(branch_id) = filter.branch_id.as_ref() {
                if &envelope.branch_id != branch_id {
                    continue;
                }
            }

            filtered.push(VirtualFsQueryRow {
                offset: row.offset,
                event_hash: hex::encode(row.event_hash),
                envelope,
            });

            if filtered.len() >= limit {
                if idx + 1 < rows_all.len() {
                    next_cursor_offset = Some((idx + 1) as u64);
                }
                break;
            }
        }

        Ok(VirtualFsQueryResult {
            events: filtered,
            next_cursor_offset,
        })
    }

    pub fn find_proposal(
        &self,
        branch_id: &str,
        proposal_id: &str,
    ) -> Result<Option<FsDeltaProposedV1>, StorageError> {
        let key = proposal_cache_key(branch_id, proposal_id);
        if let Some(found) = self.proposal_cache.read().unwrap().get(&key).cloned() {
            return Ok(Some(found));
        }

        let envelopes = self.load_branch_envelopes(branch_id)?;
        let mut rebuilt = HashMap::new();
        for envelope in envelopes.iter().rev() {
            if envelope.event_type != VfsEventTypeV1::VfsEventTypeFsDeltaProposed as i32 {
                continue;
            }
            let proposal = self.decode_proposed_cached(envelope)?;
            let pkey = proposal_cache_key(&proposal.branch_id, &proposal.proposal_id);
            // Keep latest append-order winner for each key.
            rebuilt.entry(pkey).or_insert(proposal);
        }
        {
            let mut cache = self.proposal_cache.write().unwrap();
            *cache = rebuilt;
            if let Some(found) = cache.get(&key).cloned() {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    fn append_blob(
        &mut self,
        branch_id: &str,
        path: &str,
        content_hash_blake3: Vec<u8>,
        content_bytes: Vec<u8>,
        idempotency_key: String,
    ) -> Result<u64, StorageError> {
        let payload = BlobStoredV1 {
            schema_version: 1,
            repo_id: self.repo_id.clone(),
            branch_id: branch_id.to_string(),
            path: path.to_string(),
            content_hash_blake3,
            content_len: content_bytes.len() as u64,
            content_bytes,
            idempotency_key,
        }
        .encode_to_vec();
        self.append_typed_event(
            branch_id,
            VfsEventTypeV1::VfsEventTypeBlobStored,
            "storage.vfs.blob_stored.v1",
            payload,
        )
    }

    fn append_typed_event(
        &mut self,
        branch_id: &str,
        event_type: VfsEventTypeV1,
        payload_type: &str,
        payload: Vec<u8>,
    ) -> Result<u64, StorageError> {
        let mut offsets = self.append_typed_events(
            branch_id,
            &[PendingTypedEvent {
                event_type,
                payload_type,
                payload,
            }],
        )?;
        Ok(offsets.remove(0))
    }

    fn append_typed_events(
        &mut self,
        branch_id: &str,
        pending: &[PendingTypedEvent],
    ) -> Result<Vec<u64>, StorageError> {
        if pending.is_empty() {
            return Ok(Vec::new());
        }
        let cursor = self.ensure_append_cursor_state()?;
        let mut next_seq = cursor.next_seq;
        let mut prev_event_hash = cursor.prev_event_hash;
        let mut envelopes = Vec::with_capacity(pending.len());
        for item in pending {
            let mut envelope = self.build_envelope(
                branch_id,
                item.event_type,
                item.payload_type,
                item.payload.clone(),
            );
            envelope.seq = next_seq;
            if !prev_event_hash.is_empty() {
                envelope.prev_event_hash = prev_event_hash;
            }
            envelope.event_hash = compute_envelope_hash(&envelope);
            prev_event_hash = envelope.event_hash.clone();
            next_seq = next_seq.saturating_add(1);
            envelopes.push(envelope);
        }
        let offsets = self.append_raw_events(&envelopes)?;
        for envelope in envelopes {
            self.record_appended_envelope(envelope);
        }
        Ok(offsets)
    }

    fn build_envelope(
        &self,
        branch_id: &str,
        event_type: VfsEventTypeV1,
        payload_type: &str,
        payload: Vec<u8>,
    ) -> VfsEventEnvelopeV1 {
        let ts_unix_ms = now_unix_ms();
        let event_id = format!(
            "vfs-{}-{}-{}",
            ts_unix_ms,
            std::process::id(),
            VFS_COUNTER.fetch_add(1, Ordering::Relaxed)
        );

        let mut envelope = VfsEventEnvelopeV1 {
            schema_version: 1,
            event_id,
            seq: 0,
            ts_unix_ms,
            repo_id: self.repo_id.clone(),
            branch_id: branch_id.to_string(),
            event_type: event_type as i32,
            payload_type: payload_type.to_string(),
            payload: payload.clone(),
            payload_hash_blake3: blake3::hash(&payload).as_bytes().to_vec(),
            prev_event_hash: Vec::new(),
            event_hash: Vec::new(),
        };
        envelope.event_hash = compute_envelope_hash(&envelope);
        envelope
    }

    fn append_raw_events(
        &mut self,
        envelopes: &[VfsEventEnvelopeV1],
    ) -> Result<Vec<u64>, StorageError> {
        if envelopes.is_empty() {
            return Ok(Vec::new());
        }
        let mut events = Vec::with_capacity(envelopes.len());
        for envelope in envelopes {
            validate_envelope(envelope)?;
            let payload = envelope.encode_to_vec();
            events.push(StorageEvent::new(
                self.stream_id.clone(),
                self.actor_id.clone(),
                payload,
            ));
        }
        self.engine.append_batch_to_segmented_journal(
            &self.segment,
            &events,
            8 * 1024 * 1024,
            DurabilityLevel::AckDurable,
        )
    }

    fn load_repo_envelopes(&self) -> Result<Vec<VfsEventEnvelopeV1>, StorageError> {
        if let Some(cached) = self.envelopes_cache.read().unwrap().as_ref() {
            return Ok(cached.clone());
        }

        let rows_all = self.engine.query_segmented_journal_raw(&self.segment)?;
        let mut out = Vec::new();
        for row in rows_all {
            let envelope = decode_envelope(&row.event.payload)?;
            if envelope.repo_id == self.repo_id {
                out.push(envelope);
            }
        }
        *self.envelopes_cache.write().unwrap() = Some(out.clone());
        Ok(out)
    }

    fn load_branch_envelopes(
        &self,
        branch_id: &str,
    ) -> Result<Vec<VfsEventEnvelopeV1>, StorageError> {
        if let Some(cached) = self
            .branch_envelopes_cache
            .read()
            .unwrap()
            .get(branch_id)
            .cloned()
        {
            return Ok(cached);
        }

        let all = self.load_repo_envelopes()?;
        let filtered: Vec<VfsEventEnvelopeV1> = all
            .into_iter()
            .filter(|e| e.branch_id == branch_id)
            .collect();
        self.branch_envelopes_cache
            .write()
            .unwrap()
            .insert(branch_id.to_string(), filtered.clone());
        Ok(filtered)
    }

    fn record_appended_envelope(&self, envelope: VfsEventEnvelopeV1) {
        self.update_log_partition_cache_from_envelope(&envelope);
        {
            let mut cursor = self.append_cursor_state.write().unwrap();
            *cursor = Some(AppendCursorState {
                next_seq: envelope.seq.saturating_add(1),
                prev_event_hash: envelope.event_hash.clone(),
            });
        }
        if let Some(all) = self.envelopes_cache.write().unwrap().as_mut() {
            all.push(envelope.clone());
        }
        if let Some(branch_rows) = self
            .branch_envelopes_cache
            .write()
            .unwrap()
            .get_mut(&envelope.branch_id)
        {
            branch_rows.push(envelope);
        }
    }

    fn ensure_append_cursor_state(&self) -> Result<AppendCursorState, StorageError> {
        if let Some(found) = self.append_cursor_state.read().unwrap().as_ref().cloned() {
            return Ok(found);
        }
        let all = self.load_repo_envelopes()?;
        let state = if let Some(last) = all.last() {
            AppendCursorState {
                next_seq: last.seq.saturating_add(1),
                prev_event_hash: last.event_hash.clone(),
            }
        } else {
            AppendCursorState {
                next_seq: 1,
                prev_event_hash: Vec::new(),
            }
        };
        *self.append_cursor_state.write().unwrap() = Some(state.clone());
        Ok(state)
    }

    fn update_log_partition_cache_from_envelope(&self, envelope: &VfsEventEnvelopeV1) {
        if envelope.event_type == VfsEventTypeV1::VfsEventTypePartitionDeclared as i32 {
            if let Ok(declared) = self.decode_partition_declared_cached(envelope) {
                let key = log_partition_state_key(&declared.branch_id, &declared.partition);
                let mut cache = self.log_partition_cache.write().unwrap();
                let state = cache.entry(key).or_default();
                state.partition_declared = true;
            }
            return;
        }

        if envelope.event_type == VfsEventTypeV1::VfsEventTypeLogEntryAppended as i32 {
            if let Ok(entry) = self.decode_log_entry_cached(envelope) {
                let key = log_partition_state_key(&entry.branch_id, &entry.partition);
                let mut cache = self.log_partition_cache.write().unwrap();
                let state = cache.entry(key).or_default();
                state.partition_declared = true;
                state.max_partition_offset = state.max_partition_offset.max(entry.offset);
                if !entry.idempotency_key.is_empty() {
                    state.idempotency_keys.insert(entry.idempotency_key);
                }
            }
        }
    }

    fn maybe_auto_checkpoint_after_applied(
        &mut self,
        branch_id: &str,
    ) -> Result<Option<u64>, StorageError> {
        let every = self.snapshot_policy.auto_checkpoint_every_applied;
        if every == 0 {
            return Ok(None);
        }

        let mut tracker = self.snapshot_progress.write().unwrap();
        let (mut progress, bootstrapped_from_log) = if let Some(p) = *tracker {
            (p, false)
        } else {
            let boot = self.bootstrap_snapshot_progress()?;
            *tracker = Some(boot);
            (boot, true)
        };
        if !bootstrapped_from_log {
            progress.applied_since_snapshot = progress.applied_since_snapshot.saturating_add(1);
        }

        if progress.applied_since_snapshot < every {
            *tracker = Some(progress);
            return Ok(None);
        }

        drop(tracker);
        let snapshot_offset = self.checkpoint_snapshot(branch_id, "auto:applied_interval")?;
        *self.snapshot_progress.write().unwrap() = None;
        Ok(Some(snapshot_offset))
    }

    fn bootstrap_snapshot_progress(&self) -> Result<SnapshotProgress, StorageError> {
        let envelopes = self.load_repo_envelopes()?;
        let mut progress = SnapshotProgress::default();
        for envelope in &envelopes {
            if envelope.event_type == VfsEventTypeV1::VfsEventTypeSnapshotCheckpointed as i32 {
                progress.last_snapshot_seq = envelope.seq;
                progress.applied_since_snapshot = 0;
                continue;
            }
            if envelope.seq > progress.last_snapshot_seq
                && envelope.event_type == VfsEventTypeV1::VfsEventTypeFsDeltaApplied as i32
            {
                progress.applied_since_snapshot = progress.applied_since_snapshot.saturating_add(1);
            }
        }
        Ok(progress)
    }

    fn collect_existing_log_state(
        &self,
        envelopes: &[VfsEventEnvelopeV1],
        branch_id: &str,
        partition: &str,
    ) -> Result<ExistingLogState, StorageError> {
        let mut state = ExistingLogState::default();
        for envelope in envelopes {
            if envelope.branch_id != branch_id {
                continue;
            }
            if envelope.event_type == VfsEventTypeV1::VfsEventTypePartitionDeclared as i32 {
                let declared = self.decode_partition_declared_cached(envelope)?;
                if declared.partition == partition {
                    state.partition_declared = true;
                }
            }
            if envelope.event_type == VfsEventTypeV1::VfsEventTypeLogEntryAppended as i32 {
                let entry = self.decode_log_entry_cached(envelope)?;
                if entry.partition != partition {
                    continue;
                }
                state.max_partition_offset = state.max_partition_offset.max(entry.offset);
                if !entry.idempotency_key.is_empty() {
                    state.idempotency_keys.insert(entry.idempotency_key);
                }
            }
        }
        Ok(state)
    }

    fn latest_snapshot(
        &self,
        envelopes: &[VfsEventEnvelopeV1],
    ) -> Result<Option<(u64, SnapshotCheckpointedV1)>, StorageError> {
        let mut latest: Option<(u64, SnapshotCheckpointedV1)> = None;
        for envelope in envelopes {
            if envelope.event_type != VfsEventTypeV1::VfsEventTypeSnapshotCheckpointed as i32 {
                continue;
            }
            let snap = self.decode_snapshot_cached(envelope)?;
            latest = Some((envelope.seq, snap));
        }
        Ok(latest)
    }

    fn decode_proposed_cached(
        &self,
        envelope: &VfsEventEnvelopeV1,
    ) -> Result<FsDeltaProposedV1, StorageError> {
        if let Some(found) = self
            .proposed_payload_cache
            .read()
            .unwrap()
            .get(&envelope.event_id)
            .cloned()
        {
            return Ok(found);
        }
        let decoded = FsDeltaProposedV1::decode(envelope.payload.as_slice()).map_err(|e| {
            StorageError::InvalidData(format!("invalid FsDeltaProposedV1 payload: {e}"))
        })?;
        self.proposed_payload_cache
            .write()
            .unwrap()
            .insert(envelope.event_id.clone(), decoded.clone());
        Ok(decoded)
    }

    fn decode_partition_declared_cached(
        &self,
        envelope: &VfsEventEnvelopeV1,
    ) -> Result<PartitionDeclaredV1, StorageError> {
        if let Some(found) = self
            .partition_declared_payload_cache
            .read()
            .unwrap()
            .get(&envelope.event_id)
            .cloned()
        {
            return Ok(found);
        }
        let decoded = PartitionDeclaredV1::decode(envelope.payload.as_slice()).map_err(|e| {
            StorageError::InvalidData(format!("invalid PartitionDeclaredV1 payload: {e}"))
        })?;
        self.partition_declared_payload_cache
            .write()
            .unwrap()
            .insert(envelope.event_id.clone(), decoded.clone());
        Ok(decoded)
    }

    fn decode_log_entry_cached(
        &self,
        envelope: &VfsEventEnvelopeV1,
    ) -> Result<LogEntryAppendedV1, StorageError> {
        if let Some(found) = self
            .log_entry_payload_cache
            .read()
            .unwrap()
            .get(&envelope.event_id)
            .cloned()
        {
            return Ok(found);
        }
        let decoded = LogEntryAppendedV1::decode(envelope.payload.as_slice()).map_err(|e| {
            StorageError::InvalidData(format!("invalid LogEntryAppendedV1 payload: {e}"))
        })?;
        self.log_entry_payload_cache
            .write()
            .unwrap()
            .insert(envelope.event_id.clone(), decoded.clone());
        Ok(decoded)
    }

    fn decode_snapshot_cached(
        &self,
        envelope: &VfsEventEnvelopeV1,
    ) -> Result<SnapshotCheckpointedV1, StorageError> {
        if let Some(found) = self
            .snapshot_payload_cache
            .read()
            .unwrap()
            .get(&envelope.event_id)
            .cloned()
        {
            return Ok(found);
        }
        let decoded = SnapshotCheckpointedV1::decode(envelope.payload.as_slice()).map_err(|e| {
            StorageError::InvalidData(format!("invalid snapshot checkpoint protobuf payload: {e}"))
        })?;
        self.snapshot_payload_cache
            .write()
            .unwrap()
            .insert(envelope.event_id.clone(), decoded.clone());
        Ok(decoded)
    }

    fn resolve_proposal_diff_unified(
        &self,
        branch_id: &str,
        proposal_id: &str,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self
            .find_proposal(branch_id, proposal_id)?
            .map(|proposal| proposal.diff_unified))
    }
}

#[derive(Debug, Clone, Default)]
struct ExistingLogState {
    partition_declared: bool,
    max_partition_offset: u64,
    idempotency_keys: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct SnapshotProgress {
    last_snapshot_seq: u64,
    applied_since_snapshot: u64,
}

#[derive(Debug, Clone, Default)]
struct AppendCursorState {
    next_seq: u64,
    prev_event_hash: Vec<u8>,
}

#[derive(Debug, Clone)]
struct PendingTypedEvent<'a> {
    event_type: VfsEventTypeV1,
    payload_type: &'a str,
    payload: Vec<u8>,
}

fn merge_log_outcome(total: &mut LogIngestOutcome, out: LogIngestOutcome) {
    total.appended = total.appended.saturating_add(out.appended);
    total.skipped_idempotent = total
        .skipped_idempotent
        .saturating_add(out.skipped_idempotent);
    total.declared_partition = total.declared_partition || out.declared_partition;
    if total.first_event_offset.is_none() {
        total.first_event_offset = out.first_event_offset;
    }
    total.last_event_offset = out.last_event_offset.or(total.last_event_offset);
}

fn collect_snapshot_files(
    root_path: &Path,
    policy: &FsSnapshotPolicy,
) -> Result<Vec<SnapshotFile>, StorageError> {
    let mut out = Vec::new();
    collect_snapshot_files_inner(root_path, root_path, policy, &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn collect_snapshot_files_inner(
    base: &Path,
    current: &Path,
    policy: &FsSnapshotPolicy,
    out: &mut Vec<SnapshotFile>,
) -> Result<(), StorageError> {
    let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|a| a.file_name());

    for entry in entries {
        let path = entry.path();
        let rel = path.strip_prefix(base).map_err(|e| {
            StorageError::InvalidData(format!("failed to compute relative path: {e}"))
        })?;
        if rel.as_os_str().is_empty() {
            continue;
        }

        let rel_display = rel.to_string_lossy().replace('\\', "/");
        if !policy.include_hidden
            && rel
                .components()
                .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
        {
            continue;
        }

        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_snapshot_files_inner(base, &path, policy, out)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let meta = entry.metadata()?;
        if meta.len() > policy.max_file_size_bytes {
            return Err(StorageError::InvalidData(format!(
                "file exceeds max_file_size_bytes policy: {} ({} bytes > {})",
                rel_display,
                meta.len(),
                policy.max_file_size_bytes
            )));
        }

        let content = fs::read(&path)?;
        out.push(SnapshotFile {
            path: rel_display,
            content,
        });
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct SnapshotFile {
    path: String,
    content: Vec<u8>,
}

fn run_git(repo_path: &Path, args: &[&str]) -> Result<Vec<u8>, StorageError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()
        .map_err(|e| StorageError::InvalidData(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(StorageError::InvalidData(format!(
            "git command failed (args={args:?}): {}",
            stderr.trim()
        )));
    }

    Ok(output.stdout)
}

fn parse_git_ls_tree_z(data: &[u8]) -> Result<Vec<(String, String)>, StorageError> {
    let mut out = Vec::new();
    for record in data.split(|b| *b == 0) {
        if record.is_empty() {
            continue;
        }
        let Some(tab_idx) = record.iter().position(|b| *b == b'\t') else {
            return Err(StorageError::InvalidData(
                "invalid git ls-tree -z record: missing tab".to_string(),
            ));
        };
        let head = &record[..tab_idx];
        let path_bytes = &record[tab_idx + 1..];
        let path = String::from_utf8(path_bytes.to_vec())
            .map_err(|e| StorageError::InvalidData(format!("invalid utf8 path from git: {e}")))?;

        let head_text = String::from_utf8(head.to_vec()).map_err(|e| {
            StorageError::InvalidData(format!("invalid utf8 header from git ls-tree: {e}"))
        })?;
        let mut parts = head_text.split_whitespace();
        let _mode = parts.next();
        let kind = parts.next();
        let hash = parts.next();
        if kind != Some("blob") {
            continue;
        }
        let Some(hash) = hash else {
            return Err(StorageError::InvalidData(
                "invalid git ls-tree header: missing blob hash".to_string(),
            ));
        };
        out.push((path, hash.to_string()));
    }
    Ok(out)
}

fn compute_root_projection_hash(path_hashes: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut hasher = blake3::Hasher::new();
    for (path, hash) in path_hashes {
        hasher.update(path.as_bytes());
        hasher.update(&[0]);
        hasher.update(hash);
        hasher.update(&[0]);
    }
    hasher.finalize().as_bytes().to_vec()
}

fn apply_envelope_to_state(
    state: &mut VirtualRepoState,
    envelope: &VfsEventEnvelopeV1,
) -> Result<(), StorageError> {
    state.max_seq = state.max_seq.max(envelope.seq);

    let event_type = envelope.event_type;
    let branch_id = envelope.branch_id.clone();

    match event_type {
        x if x == VfsEventTypeV1::VfsEventTypeSourceImported as i32 => {
            let imported = SourceImportedV1::decode(envelope.payload.as_slice()).map_err(|e| {
                StorageError::InvalidData(format!("invalid SourceImportedV1 payload: {e}"))
            })?;
            state.imported = true;
            state.source_kind = source_kind_from_i32(imported.source_kind);
            state.mode = mode_from_i32(imported.mode);
            let branch = state.ensure_branch(&branch_id);
            branch.head_seq = envelope.seq;
            branch.head_event_hash = envelope.event_hash.clone();
        }
        x if x == VfsEventTypeV1::VfsEventTypeBlobStored as i32 => {
            let branch = state.ensure_branch(&branch_id);
            branch.head_seq = envelope.seq;
            branch.head_event_hash = envelope.event_hash.clone();
        }
        x if x == VfsEventTypeV1::VfsEventTypeBranchCreated as i32 => {
            let created = BranchCreatedV1::decode(envelope.payload.as_slice()).map_err(|e| {
                StorageError::InvalidData(format!("invalid BranchCreatedV1 payload: {e}"))
            })?;
            let branch = state.ensure_branch(&created.branch_id);
            branch.head_seq = envelope.seq;
            branch.head_event_hash = envelope.event_hash.clone();
        }
        x if x == VfsEventTypeV1::VfsEventTypeBranchHeadMoved as i32 => {
            let moved = BranchHeadMovedV1::decode(envelope.payload.as_slice()).map_err(|e| {
                StorageError::InvalidData(format!("invalid BranchHeadMovedV1 payload: {e}"))
            })?;
            let branch = state.ensure_branch(&moved.branch_id);
            if let Some(cursor) = moved.resulting_cursor {
                branch.head_seq = cursor.seq;
                branch.head_event_hash = cursor.head_event_hash;
            } else {
                branch.head_seq = envelope.seq;
                branch.head_event_hash = envelope.event_hash.clone();
            }
        }
        x if x == VfsEventTypeV1::VfsEventTypeFsDeltaProposed as i32 => {
            let branch = state.ensure_branch(&branch_id);
            branch.proposed_delta_count = branch.proposed_delta_count.saturating_add(1);
        }
        x if x == VfsEventTypeV1::VfsEventTypeFsDeltaApplied as i32 => {
            let branch = state.ensure_branch(&branch_id);
            branch.applied_delta_count = branch.applied_delta_count.saturating_add(1);
            branch.head_seq = envelope.seq;
            branch.head_event_hash = envelope.event_hash.clone();
        }
        x if x == VfsEventTypeV1::VfsEventTypeFsDeltaRejected as i32 => {
            let branch = state.ensure_branch(&branch_id);
            branch.rejected_delta_count = branch.rejected_delta_count.saturating_add(1);
        }
        x if x == VfsEventTypeV1::VfsEventTypePartitionDeclared as i32 => {
            let branch = state.ensure_branch(&branch_id);
            branch.head_seq = envelope.seq;
            branch.head_event_hash = envelope.event_hash.clone();
        }
        x if x == VfsEventTypeV1::VfsEventTypeLogEntryAppended as i32 => {
            let branch = state.ensure_branch(&branch_id);
            branch.log_entry_count = branch.log_entry_count.saturating_add(1);
            branch.head_seq = envelope.seq;
            branch.head_event_hash = envelope.event_hash.clone();
        }
        x if x == VfsEventTypeV1::VfsEventTypeSnapshotCheckpointed as i32 => {
            let _ = x;
        }
        _ => {}
    }

    Ok(())
}

fn decode_envelope(data: &[u8]) -> Result<VfsEventEnvelopeV1, StorageError> {
    VfsEventEnvelopeV1::decode(data).map_err(|e| {
        StorageError::InvalidData(format!("invalid vfs envelope protobuf payload: {e}"))
    })
}

fn validate_envelope(envelope: &VfsEventEnvelopeV1) -> Result<(), StorageError> {
    if envelope.schema_version != 1 {
        return Err(StorageError::InvalidData(format!(
            "invalid vfs envelope schema_version: expected 1, got {}",
            envelope.schema_version
        )));
    }
    if envelope.event_id.trim().is_empty() {
        return Err(StorageError::InvalidData(
            "invalid vfs envelope: empty event_id".to_string(),
        ));
    }
    if envelope.repo_id.trim().is_empty() {
        return Err(StorageError::InvalidData(
            "invalid vfs envelope: empty repo_id".to_string(),
        ));
    }
    if envelope.branch_id.trim().is_empty() {
        return Err(StorageError::InvalidData(
            "invalid vfs envelope: empty branch_id".to_string(),
        ));
    }
    if envelope.payload_type.trim().is_empty() {
        return Err(StorageError::InvalidData(
            "invalid vfs envelope: empty payload_type".to_string(),
        ));
    }
    if envelope.payload_hash_blake3.len() != 32 {
        return Err(StorageError::InvalidData(format!(
            "invalid vfs envelope: payload_hash_blake3 must be 32 bytes, got {}",
            envelope.payload_hash_blake3.len()
        )));
    }
    if envelope.payload_hash_blake3 != blake3::hash(&envelope.payload).as_bytes().to_vec() {
        return Err(StorageError::InvalidData(
            "invalid vfs envelope: payload hash mismatch".to_string(),
        ));
    }
    Ok(())
}

fn compute_envelope_hash(envelope: &VfsEventEnvelopeV1) -> Vec<u8> {
    let mut canonical = envelope.clone();
    canonical.event_hash.clear();
    let bytes = canonical.encode_to_vec();
    blake3::hash(&bytes).as_bytes().to_vec()
}

fn source_kind_from_i32(value: i32) -> VfsSourceKindV1 {
    match value {
        x if x == VfsSourceKindV1::VfsSourceKindGit as i32 => VfsSourceKindV1::VfsSourceKindGit,
        x if x == VfsSourceKindV1::VfsSourceKindFsSnapshot as i32 => {
            VfsSourceKindV1::VfsSourceKindFsSnapshot
        }
        x if x == VfsSourceKindV1::VfsSourceKindLogStream as i32 => {
            VfsSourceKindV1::VfsSourceKindLogStream
        }
        x if x == VfsSourceKindV1::VfsSourceKindCustom as i32 => {
            VfsSourceKindV1::VfsSourceKindCustom
        }
        _ => VfsSourceKindV1::VfsSourceKindUnspecified,
    }
}

fn mode_from_i32(value: i32) -> VfsModeV1 {
    match value {
        x if x == VfsModeV1::VfsModeCode as i32 => VfsModeV1::VfsModeCode,
        x if x == VfsModeV1::VfsModeLog as i32 => VfsModeV1::VfsModeLog,
        x if x == VfsModeV1::VfsModeHybrid as i32 => VfsModeV1::VfsModeHybrid,
        _ => VfsModeV1::VfsModeUnspecified,
    }
}

fn stream_id_for_seed(seed: &str) -> StreamId {
    let digest = blake3::hash(seed.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    StreamId::from_bytes(bytes)
}

fn actor_id_for_seed(seed: &str) -> ActorId {
    let digest = blake3::hash(seed.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[16..32]);
    ActorId::from_bytes(bytes)
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn normalize_repo_id(repo_id: &str) -> String {
    let trimmed = repo_id.trim();
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "default".to_string()
    } else {
        out
    }
}

fn json_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

fn proposal_cache_key(branch_id: &str, proposal_id: &str) -> String {
    format!("{branch_id}\u{1f}{proposal_id}")
}

fn log_partition_state_key(branch_id: &str, partition: &str) -> String {
    format!("{branch_id}\u{1f}{partition}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_engine::StorageBackedContextEngine;
    use crate::docker_logger::{DefaultDockerLogAdapter, PipeDockerLogDecoder};
    use tempfile::TempDir;

    #[test]
    fn import_source_supports_log_mode() {
        let tmp = TempDir::new().expect("tempdir");
        let mut vfs = StorageBackedVirtualFs::open_writer(tmp.path().to_path_buf(), "repo-a")
            .expect("open writer");

        let req = SourceImportRequestV1 {
            schema_version: 1,
            repo_id: "repo-a".to_string(),
            source_kind: VfsSourceKindV1::VfsSourceKindLogStream as i32,
            mode: VfsModeV1::VfsModeLog as i32,
            source_locator: "nats://local/events".to_string(),
            source_ref: "partition=jobs".to_string(),
            initiated_by: "tester".to_string(),
        };

        let _ = vfs
            .import_source(&req, 10, 2048, vec![7u8; 32], "{\"k\":\"v\"}".to_string())
            .expect("import");

        let queried = vfs
            .query(10, 0, VirtualFsQueryFilter::default())
            .expect("query");
        assert_eq!(queried.events.len(), 1);
        let env = &queried.events[0].envelope;
        assert_eq!(
            env.event_type,
            VfsEventTypeV1::VfsEventTypeSourceImported as i32
        );

        let imported = SourceImportedV1::decode(env.payload.as_slice()).expect("decode payload");
        assert_eq!(
            imported.source_kind,
            VfsSourceKindV1::VfsSourceKindLogStream as i32
        );
        assert_eq!(imported.mode, VfsModeV1::VfsModeLog as i32);
    }

    #[test]
    fn import_fs_snapshot_records_blob_events() {
        let tmp = TempDir::new().expect("tempdir");
        std::fs::create_dir_all(tmp.path().join("nested")).expect("mkdir");
        std::fs::write(tmp.path().join("a.txt"), b"alpha").expect("write a");
        std::fs::write(tmp.path().join("nested").join("b.txt"), b"beta").expect("write b");
        std::fs::write(tmp.path().join(".hidden"), b"skip-me").expect("write hidden");

        let mut vfs =
            StorageBackedVirtualFs::open_writer(tmp.path().to_path_buf(), "repo-fs").expect("open");
        let report = vfs
            .import_fs_snapshot(tmp.path(), FsSnapshotPolicy::default(), "tester")
            .expect("import fs snapshot");

        assert_eq!(report.source_kind, VfsSourceKindV1::VfsSourceKindFsSnapshot);
        assert_eq!(report.imported_object_count, 2);

        let blobs = vfs
            .query(
                100,
                0,
                VirtualFsQueryFilter {
                    event_type: Some(VfsEventTypeV1::VfsEventTypeBlobStored),
                    branch_id: Some("main".to_string()),
                },
            )
            .expect("query blobs");
        assert_eq!(blobs.events.len(), 2);

        let state = vfs.materialize().expect("materialize");
        assert!(state.imported);
        assert_eq!(state.source_kind, VfsSourceKindV1::VfsSourceKindFsSnapshot);
    }

    #[test]
    fn ingest_log_entries_enforces_idempotency() {
        let tmp = TempDir::new().expect("tempdir");
        let mut vfs = StorageBackedVirtualFs::open_writer(tmp.path().to_path_buf(), "repo-log")
            .expect("open");

        let req = SourceImportRequestV1 {
            schema_version: 1,
            repo_id: "repo-log".to_string(),
            source_kind: VfsSourceKindV1::VfsSourceKindLogStream as i32,
            mode: VfsModeV1::VfsModeLog as i32,
            source_locator: "nats://local/jobs".to_string(),
            source_ref: "jobs".to_string(),
            initiated_by: "tester".to_string(),
        };
        let _ = vfs
            .import_source(&req, 0, 0, vec![0u8; 32], "{}".to_string())
            .expect("import");

        let batch = vec![
            LogIngestEntry {
                entry_key: "k1".to_string(),
                entry_payload: b"1".to_vec(),
                idempotency_key: "id-1".to_string(),
                offset: None,
            },
            LogIngestEntry {
                entry_key: "k2".to_string(),
                entry_payload: b"2".to_vec(),
                idempotency_key: "id-2".to_string(),
                offset: None,
            },
            LogIngestEntry {
                entry_key: "k1-dup".to_string(),
                entry_payload: b"1-dup".to_vec(),
                idempotency_key: "id-1".to_string(),
                offset: None,
            },
        ];

        let out1 = vfs
            .ingest_log_entries("main", "jobs", &batch, "tester")
            .expect("ingest 1");
        assert_eq!(out1.appended, 2);
        assert_eq!(out1.skipped_idempotent, 1);
        assert!(out1.declared_partition);

        let out2 = vfs
            .ingest_log_entries("main", "jobs", &batch, "tester")
            .expect("ingest 2");
        assert_eq!(out2.appended, 0);
        assert_eq!(out2.skipped_idempotent, 3);
        assert!(!out2.declared_partition);

        let partitions = vfs
            .query(
                100,
                0,
                VirtualFsQueryFilter {
                    event_type: Some(VfsEventTypeV1::VfsEventTypePartitionDeclared),
                    branch_id: Some("main".to_string()),
                },
            )
            .expect("query partitions");
        assert_eq!(partitions.events.len(), 1);

        let state = vfs.materialize().expect("materialize");
        let main = state
            .branches
            .iter()
            .find(|b| b.branch_id == "main")
            .expect("main");
        assert_eq!(main.log_entry_count, 2);
    }

    #[test]
    fn ingest_docker_log_lines_is_pluggable_and_idempotent() {
        let tmp = TempDir::new().expect("tempdir");
        let mut vfs = StorageBackedVirtualFs::open_writer(tmp.path().to_path_buf(), "repo-docker")
            .expect("open");

        let req = SourceImportRequestV1 {
            schema_version: 1,
            repo_id: "repo-docker".to_string(),
            source_kind: VfsSourceKindV1::VfsSourceKindLogStream as i32,
            mode: VfsModeV1::VfsModeLog as i32,
            source_locator: "docker://local".to_string(),
            source_ref: "events".to_string(),
            initiated_by: "tester".to_string(),
        };
        let _ = vfs
            .import_source(&req, 0, 0, vec![0u8; 32], "{}".to_string())
            .expect("import");

        let lines = vec![
            "cid1|api|stdout|1709251200001|hello".to_string(),
            "cid1|api|stdout|1709251200001|hello".to_string(),
            "cid2|worker|stderr|1709251200002|boom".to_string(),
        ];
        let mut decoder = PipeDockerLogDecoder;
        let adapter = DefaultDockerLogAdapter::default();
        let out1 = vfs
            .ingest_docker_log_lines("main", &lines, "docker-plugin", &mut decoder, &adapter)
            .expect("ingest docker lines");
        assert_eq!(out1.appended, 2);
        assert_eq!(out1.skipped_idempotent, 1);
        assert!(out1.declared_partition);

        let mut decoder2 = PipeDockerLogDecoder;
        let out2 = vfs
            .ingest_docker_log_lines("main", &lines, "docker-plugin", &mut decoder2, &adapter)
            .expect("ingest docker lines again");
        assert_eq!(out2.appended, 0);
        assert_eq!(out2.skipped_idempotent, 3);
        assert!(!out2.declared_partition);

        let state = vfs.materialize().expect("materialize");
        let main = state
            .branches
            .iter()
            .find(|b| b.branch_id == "main")
            .expect("main");
        assert_eq!(main.log_entry_count, 2);
    }

    #[test]
    fn ingest_docker_log_lines_batched_handles_small_flush_windows() {
        let tmp = TempDir::new().expect("tempdir");
        let mut vfs =
            StorageBackedVirtualFs::open_writer(tmp.path().to_path_buf(), "repo-docker-b")
                .expect("open");

        let req = SourceImportRequestV1 {
            schema_version: 1,
            repo_id: "repo-docker-b".to_string(),
            source_kind: VfsSourceKindV1::VfsSourceKindLogStream as i32,
            mode: VfsModeV1::VfsModeLog as i32,
            source_locator: "docker://local".to_string(),
            source_ref: "events".to_string(),
            initiated_by: "tester".to_string(),
        };
        let _ = vfs
            .import_source(&req, 0, 0, vec![0u8; 32], "{}".to_string())
            .expect("import");

        let lines = vec![
            "cid1|api|stdout|1709251200001|hello".to_string(),
            "cid1|api|stdout|1709251200001|hello".to_string(),
            "cid2|worker|stderr|1709251200002|boom".to_string(),
        ];
        let mut decoder = PipeDockerLogDecoder;
        let adapter = DefaultDockerLogAdapter::default();
        let out = vfs
            .ingest_docker_log_lines_batched(
                "main",
                &lines,
                "docker-plugin",
                &mut decoder,
                &adapter,
                1,
            )
            .expect("batched ingest");
        assert_eq!(out.appended, 2);
        assert_eq!(out.skipped_idempotent, 1);
    }

    #[test]
    fn log_partition_cache_tracks_direct_log_appends() {
        let tmp = TempDir::new().expect("tempdir");
        let mut vfs =
            StorageBackedVirtualFs::open_writer(tmp.path().to_path_buf(), "repo-log-cache")
                .expect("open");

        let req = SourceImportRequestV1 {
            schema_version: 1,
            repo_id: "repo-log-cache".to_string(),
            source_kind: VfsSourceKindV1::VfsSourceKindLogStream as i32,
            mode: VfsModeV1::VfsModeLog as i32,
            source_locator: "log://cache".to_string(),
            source_ref: "p=jobs".to_string(),
            initiated_by: "tester".to_string(),
        };
        let _ = vfs
            .import_source(&req, 0, 0, vec![0u8; 32], "{}".to_string())
            .expect("import");

        let batch_a = vec![LogIngestEntry {
            entry_key: "k1".to_string(),
            entry_payload: b"one".to_vec(),
            idempotency_key: "id-1".to_string(),
            offset: None,
        }];
        let _ = vfs
            .ingest_log_entries("main", "jobs", &batch_a, "tester")
            .expect("ingest a");

        let direct = LogEntryAppendedV1 {
            schema_version: 1,
            repo_id: "repo-log-cache".to_string(),
            branch_id: "main".to_string(),
            partition: "jobs".to_string(),
            offset: 10,
            entry_key: "k-direct".to_string(),
            entry_payload: b"direct".to_vec(),
            idempotency_key: "id-direct".to_string(),
        };
        let _ = vfs.append_log_entry(&direct).expect("direct append");

        let batch_b = vec![LogIngestEntry {
            entry_key: "k2".to_string(),
            entry_payload: b"two".to_vec(),
            idempotency_key: "id-2".to_string(),
            offset: None,
        }];
        let _ = vfs
            .ingest_log_entries("main", "jobs", &batch_b, "tester")
            .expect("ingest b");

        let all = vfs
            .query(
                100,
                0,
                VirtualFsQueryFilter {
                    event_type: Some(VfsEventTypeV1::VfsEventTypeLogEntryAppended),
                    branch_id: Some("main".to_string()),
                },
            )
            .expect("query");
        let mut offsets = Vec::new();
        for row in all.events {
            let event =
                LogEntryAppendedV1::decode(row.envelope.payload.as_slice()).expect("decode");
            if event.partition == "jobs" {
                offsets.push(event.offset);
            }
        }
        offsets.sort_unstable();
        assert!(offsets.contains(&1));
        assert!(offsets.contains(&10));
        assert!(offsets.contains(&11));
    }

    #[test]
    fn can_filter_applied_delta_events() {
        let tmp = TempDir::new().expect("tempdir");
        let mut vfs = StorageBackedVirtualFs::open_writer(tmp.path().to_path_buf(), "repo-b")
            .expect("open writer");

        let proposal = FsDeltaProposedV1 {
            schema_version: 1,
            repo_id: "repo-b".to_string(),
            proposal_id: "p-1".to_string(),
            branch_id: "main".to_string(),
            base_cursor: Some(VfsCursorV1 {
                branch_id: "main".to_string(),
                seq: 0,
                head_event_hash: Vec::new(),
            }),
            agent_id: "agent-1".to_string(),
            intent: "update readme".to_string(),
            diff_unified: b"diff --git a/README.md b/README.md\n--- a/README.md\n+++ b/README.md\n@@ -1 +1 @@\n-old\n+new\n".to_vec(),
        };
        let _ = vfs.propose_delta(&proposal).expect("proposal");

        let applied = FsDeltaAppliedV1 {
            schema_version: 1,
            repo_id: "repo-b".to_string(),
            proposal_id: "p-1".to_string(),
            branch_id: "main".to_string(),
            base_cursor: Some(VfsCursorV1 {
                branch_id: "main".to_string(),
                seq: 1,
                head_event_hash: Vec::new(),
            }),
            resulting_cursor: Some(VfsCursorV1 {
                branch_id: "main".to_string(),
                seq: 2,
                head_event_hash: Vec::new(),
            }),
            applied_by: "arbiter".to_string(),
        };
        let _ = vfs.apply_delta(&applied).expect("applied");

        let queried = vfs
            .query(
                10,
                0,
                VirtualFsQueryFilter {
                    event_type: Some(VfsEventTypeV1::VfsEventTypeFsDeltaApplied),
                    branch_id: Some("main".to_string()),
                },
            )
            .expect("query");

        assert_eq!(queried.events.len(), 1);
        assert_eq!(
            queried.events[0].envelope.event_type,
            VfsEventTypeV1::VfsEventTypeFsDeltaApplied as i32
        );

        let ctx = StorageBackedContextEngine::open_reader(tmp.path().to_path_buf(), "repo-b")
            .expect("context reader");
        let bundle = ctx
            .build_bundle("main", &["README.md".to_string()], 10, 10)
            .expect("context bundle");
        assert_eq!(bundle.touched_files, vec!["README.md".to_string()]);
    }

    #[test]
    fn materialize_uses_snapshot_and_replays_tail() {
        let tmp = TempDir::new().expect("tempdir");
        let mut writer = StorageBackedVirtualFs::open_writer(tmp.path().to_path_buf(), "repo-c")
            .expect("open writer");

        let req = SourceImportRequestV1 {
            schema_version: 1,
            repo_id: "repo-c".to_string(),
            source_kind: VfsSourceKindV1::VfsSourceKindLogStream as i32,
            mode: VfsModeV1::VfsModeLog as i32,
            source_locator: "nats://local/jobs".to_string(),
            source_ref: "p=jobs".to_string(),
            initiated_by: "tester".to_string(),
        };
        let _ = writer
            .import_source(&req, 1, 128, vec![1u8; 32], "{}".to_string())
            .expect("import");

        let first_log = LogEntryAppendedV1 {
            schema_version: 1,
            repo_id: "repo-c".to_string(),
            branch_id: "main".to_string(),
            partition: "jobs".to_string(),
            offset: 1,
            entry_key: "k1".to_string(),
            entry_payload: b"one".to_vec(),
            idempotency_key: "id-1".to_string(),
        };
        let _ = writer.append_log_entry(&first_log).expect("append log 1");

        let _ = writer
            .checkpoint_snapshot("main", "test-checkpoint")
            .expect("snapshot");

        let second_log = LogEntryAppendedV1 {
            schema_version: 1,
            repo_id: "repo-c".to_string(),
            branch_id: "main".to_string(),
            partition: "jobs".to_string(),
            offset: 2,
            entry_key: "k2".to_string(),
            entry_payload: b"two".to_vec(),
            idempotency_key: "id-2".to_string(),
        };
        let _ = writer.append_log_entry(&second_log).expect("append log 2");

        let reader = StorageBackedVirtualFs::open_reader(tmp.path().to_path_buf(), "repo-c")
            .expect("open reader");
        let state = reader.materialize().expect("materialize");

        assert!(state.imported);
        assert_eq!(state.source_kind, VfsSourceKindV1::VfsSourceKindLogStream);
        assert_eq!(state.mode, VfsModeV1::VfsModeLog);
        let main = state
            .branches
            .iter()
            .find(|b| b.branch_id == "main")
            .expect("main branch");
        assert_eq!(main.log_entry_count, 2);

        let snapshots = reader
            .query(
                10,
                0,
                VirtualFsQueryFilter {
                    event_type: Some(VfsEventTypeV1::VfsEventTypeSnapshotCheckpointed),
                    branch_id: Some("main".to_string()),
                },
            )
            .expect("query snapshots");
        assert_eq!(snapshots.events.len(), 1);
    }

    #[test]
    fn branch_head_move_updates_materialized_head() {
        let tmp = TempDir::new().expect("tempdir");
        let mut vfs = StorageBackedVirtualFs::open_writer(tmp.path().to_path_buf(), "repo-d")
            .expect("open writer");

        let created = BranchCreatedV1 {
            schema_version: 1,
            repo_id: "repo-d".to_string(),
            branch_id: "feature/a".to_string(),
            from_cursor: None,
            created_by: "tester".to_string(),
        };
        let _ = vfs.create_branch(&created).expect("create branch");

        let moved = BranchHeadMovedV1 {
            schema_version: 1,
            repo_id: "repo-d".to_string(),
            branch_id: "feature/a".to_string(),
            previous_cursor: None,
            resulting_cursor: Some(VfsCursorV1 {
                branch_id: "feature/a".to_string(),
                seq: 99,
                head_event_hash: vec![9u8; 32],
            }),
            moved_by: "arbiter".to_string(),
            reason: "manual sync".to_string(),
        };
        let _ = vfs.move_branch_head(&moved).expect("move head");

        let state = vfs.materialize().expect("materialize");
        let branch = state
            .branches
            .iter()
            .find(|b| b.branch_id == "feature/a")
            .expect("branch");
        assert_eq!(branch.head_seq, 99);
        assert_eq!(branch.head_event_hash, vec![9u8; 32]);
    }

    #[test]
    fn auto_snapshot_triggers_on_applied_interval() {
        let tmp = TempDir::new().expect("tempdir");
        let mut writer = StorageBackedVirtualFs::open_writer_with_snapshot_policy(
            tmp.path().to_path_buf(),
            "repo-e",
            VfsSnapshotPolicy {
                auto_checkpoint_every_applied: 2,
            },
        )
        .expect("open writer");

        let req = SourceImportRequestV1 {
            schema_version: 1,
            repo_id: "repo-e".to_string(),
            source_kind: VfsSourceKindV1::VfsSourceKindGit as i32,
            mode: VfsModeV1::VfsModeCode as i32,
            source_locator: "git://repo-e".to_string(),
            source_ref: "main".to_string(),
            initiated_by: "tester".to_string(),
        };
        let _ = writer
            .import_source(&req, 0, 0, vec![0u8; 32], "{}".to_string())
            .expect("import");

        for idx in 0..2 {
            let applied = FsDeltaAppliedV1 {
                schema_version: 1,
                repo_id: "repo-e".to_string(),
                proposal_id: String::new(),
                branch_id: "main".to_string(),
                base_cursor: None,
                resulting_cursor: None,
                applied_by: format!("arbiter-{idx}"),
            };
            let _ = writer.apply_delta(&applied).expect("apply");
        }

        let snapshots = writer
            .query(
                10,
                0,
                VirtualFsQueryFilter {
                    event_type: Some(VfsEventTypeV1::VfsEventTypeSnapshotCheckpointed),
                    branch_id: Some("main".to_string()),
                },
            )
            .expect("query snapshots");
        assert_eq!(snapshots.events.len(), 1);

        let state = writer.materialize().expect("materialize");
        let main = state
            .branches
            .iter()
            .find(|b| b.branch_id == "main")
            .expect("main");
        assert_eq!(main.applied_delta_count, 2);
    }

    #[test]
    fn auto_snapshot_can_be_disabled() {
        let tmp = TempDir::new().expect("tempdir");
        let mut writer = StorageBackedVirtualFs::open_writer_with_snapshot_policy(
            tmp.path().to_path_buf(),
            "repo-f",
            VfsSnapshotPolicy {
                auto_checkpoint_every_applied: 0,
            },
        )
        .expect("open writer");

        let req = SourceImportRequestV1 {
            schema_version: 1,
            repo_id: "repo-f".to_string(),
            source_kind: VfsSourceKindV1::VfsSourceKindGit as i32,
            mode: VfsModeV1::VfsModeCode as i32,
            source_locator: "git://repo-f".to_string(),
            source_ref: "main".to_string(),
            initiated_by: "tester".to_string(),
        };
        let _ = writer
            .import_source(&req, 0, 0, vec![0u8; 32], "{}".to_string())
            .expect("import");

        let applied = FsDeltaAppliedV1 {
            schema_version: 1,
            repo_id: "repo-f".to_string(),
            proposal_id: String::new(),
            branch_id: "main".to_string(),
            base_cursor: None,
            resulting_cursor: None,
            applied_by: "arbiter".to_string(),
        };
        let _ = writer.apply_delta(&applied).expect("apply");

        let snapshots = writer
            .query(
                10,
                0,
                VirtualFsQueryFilter {
                    event_type: Some(VfsEventTypeV1::VfsEventTypeSnapshotCheckpointed),
                    branch_id: Some("main".to_string()),
                },
            )
            .expect("query snapshots");
        assert_eq!(snapshots.events.len(), 0);
    }
}
