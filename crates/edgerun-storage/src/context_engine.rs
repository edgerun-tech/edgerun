// SPDX-License-Identifier: GPL-2.0-only
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use prost::Message;

use crate::durability::DurabilityLevel;
use crate::event::{ActorId, Event as StorageEvent, StreamId};
use crate::{StorageEngine, StorageError};

pub use crate::manifest::proto::{
    ContextDiagnosticRecordedV1, ContextEventEnvelopeV1, ContextEventTypeV1,
    ContextReferenceRecordedV1, ContextSnapshotCheckpointedV1, ContextSymbolUpsertedV1,
    ContextTouchRecordedV1, MaterializedContextBranchStateV1, MaterializedContextStateV1,
    RustAnalyzerSnapshotV1,
};

static CONTEXT_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct ContextQueryRow {
    pub offset: u64,
    pub event_hash: String,
    pub envelope: ContextEventEnvelopeV1,
}

#[derive(Debug, Clone)]
pub struct ContextQueryResult {
    pub events: Vec<ContextQueryRow>,
    pub next_cursor_offset: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct ContextQueryFilter {
    pub event_type: Option<ContextEventTypeV1>,
    pub branch_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MaterializedContextBranch {
    pub branch_id: String,
    pub symbols: BTreeMap<String, ContextSymbolUpsertedV1>,
    pub references: Vec<ContextReferenceRecordedV1>,
    pub diagnostics: Vec<ContextDiagnosticRecordedV1>,
    pub touched_files: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct MaterializedContextState {
    pub repo_id: String,
    pub max_seq: u64,
    pub branches: BTreeMap<String, MaterializedContextBranch>,
}

#[derive(Debug, Clone, Default)]
pub struct ContextBundle {
    pub branch_id: String,
    pub file_paths: Vec<String>,
    pub symbols: Vec<ContextSymbolUpsertedV1>,
    pub references: Vec<ContextReferenceRecordedV1>,
    pub diagnostics: Vec<ContextDiagnosticRecordedV1>,
    pub touched_files: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RustAnalyzerIngestOutcome {
    pub symbols_upserted: u64,
    pub references_recorded: u64,
    pub diagnostics_recorded: u64,
}

pub struct StorageBackedContextEngine {
    engine: StorageEngine,
    repo_id: String,
    segment: String,
    stream_id: StreamId,
    actor_id: ActorId,
    bundle_cache: RwLock<HashMap<String, ContextBundle>>,
    bundle_cache_order: RwLock<Vec<String>>,
    bundle_cache_capacity: usize,
}

impl StorageBackedContextEngine {
    pub fn open_writer(data_dir: PathBuf, repo_id: &str) -> Result<Self, StorageError> {
        let engine = StorageEngine::new(data_dir)?;
        let normalized = normalize_repo_id(repo_id);
        Ok(Self {
            engine,
            repo_id: normalized.clone(),
            segment: format!("ctx.{}.journal", normalized),
            stream_id: stream_id_for_seed(&format!("edgerun-context-stream:{normalized}")),
            actor_id: actor_id_for_seed(&format!("edgerun-context-actor:{normalized}")),
            bundle_cache: RwLock::new(HashMap::new()),
            bundle_cache_order: RwLock::new(Vec::new()),
            bundle_cache_capacity: 128,
        })
    }

    pub fn open_reader(data_dir: PathBuf, repo_id: &str) -> Result<Self, StorageError> {
        Self::open_writer(data_dir, repo_id)
    }

    pub fn upsert_symbol(&mut self, symbol: &ContextSymbolUpsertedV1) -> Result<u64, StorageError> {
        self.assert_repo(&symbol.repo_id)?;
        self.append_typed_event_idempotent(
            &symbol.branch_id,
            ContextEventTypeV1::ContextEventTypeSymbolUpserted,
            "storage.context.symbol_upserted.v1",
            symbol.encode_to_vec(),
            &symbol.idempotency_key,
        )
    }

    pub fn record_reference(
        &mut self,
        reference: &ContextReferenceRecordedV1,
    ) -> Result<u64, StorageError> {
        self.assert_repo(&reference.repo_id)?;
        self.append_typed_event_idempotent(
            &reference.branch_id,
            ContextEventTypeV1::ContextEventTypeReferenceRecorded,
            "storage.context.reference_recorded.v1",
            reference.encode_to_vec(),
            &reference.idempotency_key,
        )
    }

    pub fn record_diagnostic(
        &mut self,
        diagnostic: &ContextDiagnosticRecordedV1,
    ) -> Result<u64, StorageError> {
        self.assert_repo(&diagnostic.repo_id)?;
        self.append_typed_event_idempotent(
            &diagnostic.branch_id,
            ContextEventTypeV1::ContextEventTypeDiagnosticRecorded,
            "storage.context.diagnostic_recorded.v1",
            diagnostic.encode_to_vec(),
            &diagnostic.idempotency_key,
        )
    }

    pub fn record_touch(&mut self, touch: &ContextTouchRecordedV1) -> Result<u64, StorageError> {
        self.assert_repo(&touch.repo_id)?;
        self.append_typed_event_idempotent(
            &touch.branch_id,
            ContextEventTypeV1::ContextEventTypeTouchRecorded,
            "storage.context.touch_recorded.v1",
            touch.encode_to_vec(),
            &touch.idempotency_key,
        )
    }

    pub fn record_touches_from_unified_diff(
        &mut self,
        branch_id: &str,
        diff_unified: &[u8],
        reason: &str,
    ) -> Result<Vec<u64>, StorageError> {
        let paths = extract_paths_from_unified_diff(diff_unified);
        let mut known = self
            .load_idempotency_keys(branch_id, ContextEventTypeV1::ContextEventTypeTouchRecorded)?;
        let mut offsets = Vec::with_capacity(paths.len());
        for path in paths {
            let touch = ContextTouchRecordedV1 {
                schema_version: 1,
                repo_id: self.repo_id.clone(),
                branch_id: branch_id.to_string(),
                file_path: path.clone(),
                reason: reason.to_string(),
                idempotency_key: format!("touch:{branch_id}:{reason}:{path}"),
            };
            let offset = self.append_typed_event_idempotent_with_index(
                branch_id,
                ContextEventTypeV1::ContextEventTypeTouchRecorded,
                "storage.context.touch_recorded.v1",
                touch.encode_to_vec(),
                &touch.idempotency_key,
                &mut known,
            )?;
            offsets.push(offset);
        }
        Ok(offsets)
    }

    pub fn ingest_rust_analyzer_snapshot_proto(
        &mut self,
        branch_id: &str,
        snapshot_proto: &[u8],
    ) -> Result<RustAnalyzerIngestOutcome, StorageError> {
        let snapshot = RustAnalyzerSnapshotV1::decode(snapshot_proto).map_err(|e| {
            StorageError::InvalidData(format!("invalid RustAnalyzerSnapshotV1 payload: {e}"))
        })?;
        if !snapshot.repo_id.is_empty() && snapshot.repo_id != self.repo_id {
            return Err(StorageError::InvalidData(format!(
                "rust-analyzer snapshot repo mismatch: snapshot={}, writer={}",
                snapshot.repo_id, self.repo_id
            )));
        }
        if !snapshot.branch_id.is_empty() && snapshot.branch_id != branch_id {
            return Err(StorageError::InvalidData(format!(
                "rust-analyzer snapshot branch mismatch: snapshot={}, requested={}",
                snapshot.branch_id, branch_id
            )));
        }
        let mut out = RustAnalyzerIngestOutcome::default();
        let mut symbol_known = self.load_idempotency_keys(
            branch_id,
            ContextEventTypeV1::ContextEventTypeSymbolUpserted,
        )?;
        let mut reference_known = self.load_idempotency_keys(
            branch_id,
            ContextEventTypeV1::ContextEventTypeReferenceRecorded,
        )?;
        let mut diagnostic_known = self.load_idempotency_keys(
            branch_id,
            ContextEventTypeV1::ContextEventTypeDiagnosticRecorded,
        )?;

        for mut symbol in snapshot.symbols {
            symbol.repo_id = self.repo_id.clone();
            symbol.branch_id = branch_id.to_string();
            if symbol.idempotency_key.is_empty() {
                symbol.idempotency_key = format!(
                    "sym:{branch_id}:{}:{}:{}:{}:{}:{}",
                    symbol.symbol_id,
                    symbol.file_path,
                    symbol.line_start,
                    symbol.col_start,
                    symbol.line_end,
                    symbol.col_end
                );
            }
            let _ = self.append_typed_event_idempotent_with_index(
                branch_id,
                ContextEventTypeV1::ContextEventTypeSymbolUpserted,
                "storage.context.symbol_upserted.v1",
                symbol.encode_to_vec(),
                &symbol.idempotency_key,
                &mut symbol_known,
            )?;
            out.symbols_upserted = out.symbols_upserted.saturating_add(1);
        }

        for mut reference in snapshot.references {
            reference.repo_id = self.repo_id.clone();
            reference.branch_id = branch_id.to_string();
            if reference.idempotency_key.is_empty() {
                reference.idempotency_key = format!(
                    "ref:{branch_id}:{}:{}:{}:{}",
                    reference.symbol_id, reference.file_path, reference.line, reference.col
                );
            }
            let _ = self.append_typed_event_idempotent_with_index(
                branch_id,
                ContextEventTypeV1::ContextEventTypeReferenceRecorded,
                "storage.context.reference_recorded.v1",
                reference.encode_to_vec(),
                &reference.idempotency_key,
                &mut reference_known,
            )?;
            out.references_recorded = out.references_recorded.saturating_add(1);
        }

        for mut diagnostic in snapshot.diagnostics {
            diagnostic.repo_id = self.repo_id.clone();
            diagnostic.branch_id = branch_id.to_string();
            if diagnostic.idempotency_key.is_empty() {
                diagnostic.idempotency_key = format!(
                    "diag:{branch_id}:{}:{}:{}:{}",
                    diagnostic.diagnostic_id, diagnostic.file_path, diagnostic.line, diagnostic.col
                );
            }
            let _ = self.append_typed_event_idempotent_with_index(
                branch_id,
                ContextEventTypeV1::ContextEventTypeDiagnosticRecorded,
                "storage.context.diagnostic_recorded.v1",
                diagnostic.encode_to_vec(),
                &diagnostic.idempotency_key,
                &mut diagnostic_known,
            )?;
            out.diagnostics_recorded = out.diagnostics_recorded.saturating_add(1);
        }

        Ok(out)
    }

    pub fn checkpoint_snapshot(
        &mut self,
        branch_id: &str,
        reason: &str,
    ) -> Result<u64, StorageError> {
        let state = self.materialize()?;
        let snapshot_seq = state.max_seq;
        let snapshot_payload = state.snapshot_proto().encode_to_vec();
        let payload = ContextSnapshotCheckpointedV1 {
            schema_version: 1,
            repo_id: self.repo_id.clone(),
            branch_id: branch_id.to_string(),
            snapshot_seq,
            snapshot_hash_blake3: blake3::hash(&snapshot_payload).as_bytes().to_vec(),
            snapshot_payload,
            reason: reason.to_string(),
        };
        self.append_typed_event(
            branch_id,
            ContextEventTypeV1::ContextEventTypeSnapshotCheckpointed,
            "storage.context.snapshot_checkpointed.v1",
            payload.encode_to_vec(),
        )
    }

    pub fn materialize(&self) -> Result<MaterializedContextState, StorageError> {
        let envelopes = self.load_repo_envelopes()?;
        let mut state = MaterializedContextState::new(self.repo_id.clone());

        let mut start_seq = 1u64;
        if let Some((snapshot_event_seq, snapshot)) = latest_snapshot(&envelopes)? {
            let mut snap_state =
                MaterializedContextStateV1::decode(snapshot.snapshot_payload.as_slice())
                    .map_err(|e| {
                        StorageError::InvalidData(format!(
                            "invalid materialized context snapshot payload: {e}"
                        ))
                    })
                    .map(|proto| {
                        MaterializedContextState::from_snapshot_proto(&self.repo_id, proto)
                    });
            if let Ok(ref mut s) = snap_state {
                s.max_seq = snapshot.snapshot_seq.max(snapshot_event_seq);
            }
            state = snap_state?;
            start_seq = snapshot.snapshot_seq.saturating_add(1);
        }

        for envelope in envelopes {
            if envelope.seq < start_seq {
                continue;
            }
            state.apply_envelope(&envelope)?;
        }

        Ok(state)
    }

    pub fn build_bundle(
        &self,
        branch_id: &str,
        file_paths: &[String],
        symbol_limit: usize,
        diagnostic_limit: usize,
    ) -> Result<ContextBundle, StorageError> {
        let cache_key =
            build_bundle_cache_key(branch_id, file_paths, symbol_limit, diagnostic_limit);
        if let Some(cached) = self.bundle_cache.read().unwrap().get(&cache_key).cloned() {
            return Ok(cached);
        }

        let state = self.materialize()?;
        let Some(branch) = state.branches.get(branch_id) else {
            let empty = ContextBundle {
                branch_id: branch_id.to_string(),
                file_paths: file_paths.to_vec(),
                ..ContextBundle::default()
            };
            self.bundle_cache_insert(cache_key, empty.clone());
            return Ok(empty);
        };

        let wanted: BTreeSet<String> = file_paths.iter().cloned().collect();
        let mut symbols = Vec::new();
        for symbol in branch.symbols.values() {
            if wanted.contains(&symbol.file_path) {
                symbols.push(symbol.clone());
            }
            if symbols.len() >= symbol_limit {
                break;
            }
        }

        let symbol_ids: BTreeSet<String> = symbols.iter().map(|s| s.symbol_id.clone()).collect();
        let references: Vec<ContextReferenceRecordedV1> = branch
            .references
            .iter()
            .filter(|r| wanted.contains(&r.file_path) || symbol_ids.contains(&r.symbol_id))
            .take(symbol_limit.saturating_mul(2).max(1))
            .cloned()
            .collect();

        let diagnostics: Vec<ContextDiagnosticRecordedV1> = branch
            .diagnostics
            .iter()
            .filter(|d| wanted.contains(&d.file_path))
            .take(diagnostic_limit)
            .cloned()
            .collect();

        let touched_files: Vec<String> = branch
            .touched_files
            .iter()
            .filter(|p| wanted.contains(*p))
            .cloned()
            .collect();

        let bundle = ContextBundle {
            branch_id: branch_id.to_string(),
            file_paths: file_paths.to_vec(),
            symbols,
            references,
            diagnostics,
            touched_files,
        };
        self.bundle_cache_insert(cache_key, bundle.clone());
        Ok(bundle)
    }

    pub fn query(
        &self,
        limit: usize,
        cursor_offset: u64,
        filter: ContextQueryFilter,
    ) -> Result<ContextQueryResult, StorageError> {
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

            filtered.push(ContextQueryRow {
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

        Ok(ContextQueryResult {
            events: filtered,
            next_cursor_offset,
        })
    }

    fn assert_repo(&self, repo_id: &str) -> Result<(), StorageError> {
        if repo_id != self.repo_id {
            return Err(StorageError::InvalidData(format!(
                "repo_id mismatch: payload={}, writer={}",
                repo_id, self.repo_id
            )));
        }
        Ok(())
    }

    fn append_typed_event(
        &mut self,
        branch_id: &str,
        event_type: ContextEventTypeV1,
        payload_type: &str,
        payload: Vec<u8>,
    ) -> Result<u64, StorageError> {
        let mut envelope = self.build_envelope(branch_id, event_type, payload_type, payload);
        let all_rows = self.engine.query_segmented_journal_raw(&self.segment)?;
        envelope.seq = all_rows.len() as u64 + 1;
        if let Some(last) = all_rows.last() {
            let last_envelope = decode_envelope(&last.event.payload)?;
            envelope.prev_event_hash = last_envelope.event_hash;
        }
        envelope.event_hash = compute_envelope_hash(&envelope);
        let offset = self.append_raw_event(&envelope)?;
        self.invalidate_bundle_cache();
        Ok(offset)
    }

    fn append_typed_event_idempotent(
        &mut self,
        branch_id: &str,
        event_type: ContextEventTypeV1,
        payload_type: &str,
        payload: Vec<u8>,
        idempotency_key: &str,
    ) -> Result<u64, StorageError> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            return self.append_typed_event(branch_id, event_type, payload_type, payload);
        }
        if let Some(existing_offset) =
            self.find_existing_offset_by_idempotency(branch_id, event_type, key)?
        {
            return Ok(existing_offset);
        }
        self.append_typed_event(branch_id, event_type, payload_type, payload)
    }

    fn append_typed_event_idempotent_with_index(
        &mut self,
        branch_id: &str,
        event_type: ContextEventTypeV1,
        payload_type: &str,
        payload: Vec<u8>,
        idempotency_key: &str,
        known: &mut HashSet<String>,
    ) -> Result<u64, StorageError> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            return self.append_typed_event(branch_id, event_type, payload_type, payload);
        }
        if known.contains(key) {
            if let Some(existing_offset) =
                self.find_existing_offset_by_idempotency(branch_id, event_type, key)?
            {
                return Ok(existing_offset);
            }
        }
        let offset = self.append_typed_event(branch_id, event_type, payload_type, payload)?;
        known.insert(key.to_string());
        Ok(offset)
    }

    fn build_envelope(
        &self,
        branch_id: &str,
        event_type: ContextEventTypeV1,
        payload_type: &str,
        payload: Vec<u8>,
    ) -> ContextEventEnvelopeV1 {
        let ts_unix_ms = now_unix_ms();
        let event_id = format!(
            "ctx-{}-{}-{}",
            ts_unix_ms,
            std::process::id(),
            CONTEXT_COUNTER.fetch_add(1, Ordering::Relaxed)
        );

        let mut envelope = ContextEventEnvelopeV1 {
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

    fn append_raw_event(&mut self, envelope: &ContextEventEnvelopeV1) -> Result<u64, StorageError> {
        validate_envelope(envelope)?;
        let payload = envelope.encode_to_vec();
        let event = StorageEvent::new(self.stream_id.clone(), self.actor_id.clone(), payload);
        self.engine.append_event_to_segmented_journal(
            &self.segment,
            &event,
            8 * 1024 * 1024,
            DurabilityLevel::AckDurable,
        )
    }

    fn load_repo_envelopes(&self) -> Result<Vec<ContextEventEnvelopeV1>, StorageError> {
        let rows_all = self.engine.query_segmented_journal_raw(&self.segment)?;
        let mut out = Vec::new();
        for row in rows_all {
            let envelope = decode_envelope(&row.event.payload)?;
            if envelope.repo_id == self.repo_id {
                out.push(envelope);
            }
        }
        Ok(out)
    }

    fn find_existing_offset_by_idempotency(
        &self,
        branch_id: &str,
        event_type: ContextEventTypeV1,
        idempotency_key: &str,
    ) -> Result<Option<u64>, StorageError> {
        let rows = self.engine.query_segmented_journal_raw(&self.segment)?;
        for row in rows {
            let envelope = decode_envelope(&row.event.payload)?;
            if envelope.repo_id != self.repo_id
                || envelope.branch_id != branch_id
                || envelope.event_type != event_type as i32
            {
                continue;
            }
            if extract_idempotency_key(&envelope)?.as_deref() == Some(idempotency_key) {
                return Ok(Some(row.offset));
            }
        }
        Ok(None)
    }

    fn load_idempotency_keys(
        &self,
        branch_id: &str,
        event_type: ContextEventTypeV1,
    ) -> Result<HashSet<String>, StorageError> {
        let rows = self.engine.query_segmented_journal_raw(&self.segment)?;
        let mut keys = HashSet::new();
        for row in rows {
            let envelope = decode_envelope(&row.event.payload)?;
            if envelope.repo_id != self.repo_id
                || envelope.branch_id != branch_id
                || envelope.event_type != event_type as i32
            {
                continue;
            }
            if let Some(key) = extract_idempotency_key(&envelope)? {
                keys.insert(key);
            }
        }
        Ok(keys)
    }

    fn invalidate_bundle_cache(&self) {
        self.bundle_cache.write().unwrap().clear();
        self.bundle_cache_order.write().unwrap().clear();
    }

    fn bundle_cache_insert(&self, key: String, bundle: ContextBundle) {
        {
            let mut cache = self.bundle_cache.write().unwrap();
            cache.insert(key.clone(), bundle);
        }
        let mut order = self.bundle_cache_order.write().unwrap();
        order.retain(|k| k != &key);
        order.push(key.clone());

        let capacity = self.bundle_cache_capacity.max(1);
        while order.len() > capacity {
            if let Some(oldest) = order.first().cloned() {
                order.remove(0);
                self.bundle_cache.write().unwrap().remove(&oldest);
            } else {
                break;
            }
        }
    }
}

impl MaterializedContextState {
    fn new(repo_id: String) -> Self {
        Self {
            repo_id,
            max_seq: 0,
            branches: BTreeMap::new(),
        }
    }

    fn ensure_branch(&mut self, branch_id: &str) -> &mut MaterializedContextBranch {
        self.branches
            .entry(branch_id.to_string())
            .or_insert_with(|| MaterializedContextBranch {
                branch_id: branch_id.to_string(),
                symbols: BTreeMap::new(),
                references: Vec::new(),
                diagnostics: Vec::new(),
                touched_files: BTreeSet::new(),
            })
    }

    fn apply_envelope(&mut self, envelope: &ContextEventEnvelopeV1) -> Result<(), StorageError> {
        self.max_seq = self.max_seq.max(envelope.seq);
        let branch = self.ensure_branch(&envelope.branch_id);

        match envelope.event_type {
            x if x == ContextEventTypeV1::ContextEventTypeSymbolUpserted as i32 => {
                let symbol =
                    ContextSymbolUpsertedV1::decode(envelope.payload.as_slice()).map_err(|e| {
                        StorageError::InvalidData(format!("invalid ContextSymbolUpsertedV1: {e}"))
                    })?;
                branch.symbols.insert(symbol.symbol_id.clone(), symbol);
            }
            x if x == ContextEventTypeV1::ContextEventTypeReferenceRecorded as i32 => {
                let reference = ContextReferenceRecordedV1::decode(envelope.payload.as_slice())
                    .map_err(|e| {
                        StorageError::InvalidData(format!(
                            "invalid ContextReferenceRecordedV1: {e}"
                        ))
                    })?;
                branch.references.push(reference);
            }
            x if x == ContextEventTypeV1::ContextEventTypeDiagnosticRecorded as i32 => {
                let diagnostic = ContextDiagnosticRecordedV1::decode(envelope.payload.as_slice())
                    .map_err(|e| {
                    StorageError::InvalidData(format!("invalid ContextDiagnosticRecordedV1: {e}"))
                })?;
                branch.diagnostics.push(diagnostic);
            }
            x if x == ContextEventTypeV1::ContextEventTypeTouchRecorded as i32 => {
                let touch =
                    ContextTouchRecordedV1::decode(envelope.payload.as_slice()).map_err(|e| {
                        StorageError::InvalidData(format!("invalid ContextTouchRecordedV1: {e}"))
                    })?;
                branch.touched_files.insert(touch.file_path);
            }
            x if x == ContextEventTypeV1::ContextEventTypeSnapshotCheckpointed as i32 => {
                let _ = x;
            }
            _ => {}
        }

        Ok(())
    }

    fn snapshot_proto(&self) -> MaterializedContextStateV1 {
        let branches = self
            .branches
            .values()
            .map(|branch| MaterializedContextBranchStateV1 {
                branch_id: branch.branch_id.clone(),
                symbols: branch.symbols.values().cloned().collect(),
                references: branch.references.clone(),
                diagnostics: branch.diagnostics.clone(),
                touched_files: branch.touched_files.iter().cloned().collect(),
            })
            .collect();

        MaterializedContextStateV1 {
            schema_version: 1,
            repo_id: self.repo_id.clone(),
            branches,
        }
    }

    fn from_snapshot_proto(repo_id: &str, proto: MaterializedContextStateV1) -> Self {
        let mut state = Self::new(repo_id.to_string());
        for b in proto.branches {
            let branch = state.ensure_branch(&b.branch_id);
            for symbol in b.symbols {
                branch.symbols.insert(symbol.symbol_id.clone(), symbol);
            }
            branch.references = b.references;
            branch.diagnostics = b.diagnostics;
            for path in b.touched_files {
                branch.touched_files.insert(path);
            }
        }
        state
    }
}

fn latest_snapshot(
    envelopes: &[ContextEventEnvelopeV1],
) -> Result<Option<(u64, ContextSnapshotCheckpointedV1)>, StorageError> {
    let mut latest: Option<(u64, ContextSnapshotCheckpointedV1)> = None;
    for envelope in envelopes {
        if envelope.event_type != ContextEventTypeV1::ContextEventTypeSnapshotCheckpointed as i32 {
            continue;
        }
        let snapshot =
            ContextSnapshotCheckpointedV1::decode(envelope.payload.as_slice()).map_err(|e| {
                StorageError::InvalidData(format!("invalid ContextSnapshotCheckpointedV1: {e}"))
            })?;
        latest = Some((envelope.seq, snapshot));
    }
    Ok(latest)
}

fn extract_idempotency_key(
    envelope: &ContextEventEnvelopeV1,
) -> Result<Option<String>, StorageError> {
    match envelope.event_type {
        x if x == ContextEventTypeV1::ContextEventTypeSymbolUpserted as i32 => {
            let payload =
                ContextSymbolUpsertedV1::decode(envelope.payload.as_slice()).map_err(|e| {
                    StorageError::InvalidData(format!("invalid ContextSymbolUpsertedV1: {e}"))
                })?;
            Ok(non_empty(payload.idempotency_key))
        }
        x if x == ContextEventTypeV1::ContextEventTypeReferenceRecorded as i32 => {
            let payload =
                ContextReferenceRecordedV1::decode(envelope.payload.as_slice()).map_err(|e| {
                    StorageError::InvalidData(format!("invalid ContextReferenceRecordedV1: {e}"))
                })?;
            Ok(non_empty(payload.idempotency_key))
        }
        x if x == ContextEventTypeV1::ContextEventTypeDiagnosticRecorded as i32 => {
            let payload = ContextDiagnosticRecordedV1::decode(envelope.payload.as_slice())
                .map_err(|e| {
                    StorageError::InvalidData(format!("invalid ContextDiagnosticRecordedV1: {e}"))
                })?;
            Ok(non_empty(payload.idempotency_key))
        }
        x if x == ContextEventTypeV1::ContextEventTypeTouchRecorded as i32 => {
            let payload =
                ContextTouchRecordedV1::decode(envelope.payload.as_slice()).map_err(|e| {
                    StorageError::InvalidData(format!("invalid ContextTouchRecordedV1: {e}"))
                })?;
            Ok(non_empty(payload.idempotency_key))
        }
        _ => Ok(None),
    }
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn extract_paths_from_unified_diff(diff: &[u8]) -> Vec<String> {
    let mut out = BTreeSet::new();
    let text = String::from_utf8_lossy(diff);
    for line in text.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            if !path.trim().is_empty() {
                out.insert(path.trim().to_string());
            }
        }
        if let Some(path) = line.strip_prefix("--- a/") {
            if !path.trim().is_empty() && path.trim() != "/dev/null" {
                out.insert(path.trim().to_string());
            }
        }
    }
    out.into_iter().collect()
}

fn decode_envelope(data: &[u8]) -> Result<ContextEventEnvelopeV1, StorageError> {
    ContextEventEnvelopeV1::decode(data)
        .map_err(|e| StorageError::InvalidData(format!("invalid context envelope: {e}")))
}

fn validate_envelope(envelope: &ContextEventEnvelopeV1) -> Result<(), StorageError> {
    if envelope.schema_version != 1 {
        return Err(StorageError::InvalidData(format!(
            "invalid context envelope schema_version: expected 1, got {}",
            envelope.schema_version
        )));
    }
    if envelope.repo_id.trim().is_empty() || envelope.branch_id.trim().is_empty() {
        return Err(StorageError::InvalidData(
            "invalid context envelope: repo_id/branch_id required".to_string(),
        ));
    }
    if envelope.payload_hash_blake3 != blake3::hash(&envelope.payload).as_bytes().to_vec() {
        return Err(StorageError::InvalidData(
            "invalid context envelope: payload hash mismatch".to_string(),
        ));
    }
    Ok(())
}

fn compute_envelope_hash(envelope: &ContextEventEnvelopeV1) -> Vec<u8> {
    let mut canonical = envelope.clone();
    canonical.event_hash.clear();
    let bytes = canonical.encode_to_vec();
    blake3::hash(&bytes).as_bytes().to_vec()
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

fn build_bundle_cache_key(
    branch_id: &str,
    file_paths: &[String],
    symbol_limit: usize,
    diagnostic_limit: usize,
) -> String {
    let mut normalized = file_paths.to_vec();
    normalized.sort();
    let mut hasher = blake3::Hasher::new();
    hasher.update(branch_id.as_bytes());
    hasher.update(&[0]);
    for p in &normalized {
        hasher.update(p.as_bytes());
        hasher.update(&[0]);
    }
    hasher.update(symbol_limit.to_string().as_bytes());
    hasher.update(&[0]);
    hasher.update(diagnostic_limit.to_string().as_bytes());
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn bundle_filters_by_file_path() {
        let tmp = TempDir::new().expect("tempdir");
        let mut ctx = StorageBackedContextEngine::open_writer(tmp.path().to_path_buf(), "repo-ctx")
            .expect("open");

        let sym1 = ContextSymbolUpsertedV1 {
            schema_version: 1,
            repo_id: "repo-ctx".to_string(),
            branch_id: "main".to_string(),
            symbol_id: "sym-a".to_string(),
            symbol_name: "alpha".to_string(),
            symbol_kind: "fn".to_string(),
            file_path: "src/a.rs".to_string(),
            line_start: 1,
            col_start: 1,
            line_end: 1,
            col_end: 10,
            signature: "fn alpha()".to_string(),
            idempotency_key: "sym:main:sym-a:src/a.rs:1:1:1:10".to_string(),
        };
        let sym2 = ContextSymbolUpsertedV1 {
            file_path: "src/b.rs".to_string(),
            symbol_id: "sym-b".to_string(),
            symbol_name: "beta".to_string(),
            ..sym1.clone()
        };
        let _ = ctx.upsert_symbol(&sym1).expect("upsert 1");
        let _ = ctx.upsert_symbol(&sym2).expect("upsert 2");

        let diag = ContextDiagnosticRecordedV1 {
            schema_version: 1,
            repo_id: "repo-ctx".to_string(),
            branch_id: "main".to_string(),
            diagnostic_id: "d1".to_string(),
            severity: "error".to_string(),
            message: "boom".to_string(),
            file_path: "src/a.rs".to_string(),
            line: 5,
            col: 3,
            source: "cargo-check".to_string(),
            idempotency_key: "diag:main:d1:src/a.rs:5:3".to_string(),
        };
        let _ = ctx.record_diagnostic(&diag).expect("diag");

        let bundle = ctx
            .build_bundle("main", &["src/a.rs".to_string()], 10, 10)
            .expect("bundle");
        assert_eq!(bundle.symbols.len(), 1);
        assert_eq!(bundle.symbols[0].symbol_id, "sym-a");
        assert_eq!(bundle.diagnostics.len(), 1);
    }

    #[test]
    fn unified_diff_touch_extraction_records_files() {
        let tmp = TempDir::new().expect("tempdir");
        let mut ctx =
            StorageBackedContextEngine::open_writer(tmp.path().to_path_buf(), "repo-ctx2")
                .expect("open");

        let diff = b"diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1 +1 @@\n-1\n+2\n";
        let offsets = ctx
            .record_touches_from_unified_diff("main", diff, "fs_delta_applied")
            .expect("touches");
        assert_eq!(offsets.len(), 1);

        let bundle = ctx
            .build_bundle("main", &["src/a.rs".to_string()], 10, 10)
            .expect("bundle");
        assert_eq!(bundle.touched_files, vec!["src/a.rs".to_string()]);
    }

    #[test]
    fn snapshot_plus_tail_replay_matches_counts() {
        let tmp = TempDir::new().expect("tempdir");
        let mut writer =
            StorageBackedContextEngine::open_writer(tmp.path().to_path_buf(), "repo-ctx3")
                .expect("open");

        let sym = ContextSymbolUpsertedV1 {
            schema_version: 1,
            repo_id: "repo-ctx3".to_string(),
            branch_id: "main".to_string(),
            symbol_id: "sym-a".to_string(),
            symbol_name: "alpha".to_string(),
            symbol_kind: "fn".to_string(),
            file_path: "src/a.rs".to_string(),
            line_start: 1,
            col_start: 1,
            line_end: 1,
            col_end: 10,
            signature: "fn alpha()".to_string(),
            idempotency_key: "sym:main:sym-a:src/a.rs:1:1:1:10".to_string(),
        };
        let _ = writer.upsert_symbol(&sym).expect("upsert");
        let _ = writer
            .checkpoint_snapshot("main", "checkpoint")
            .expect("snapshot");

        let diag = ContextDiagnosticRecordedV1 {
            schema_version: 1,
            repo_id: "repo-ctx3".to_string(),
            branch_id: "main".to_string(),
            diagnostic_id: "d1".to_string(),
            severity: "warn".to_string(),
            message: "m".to_string(),
            file_path: "src/a.rs".to_string(),
            line: 10,
            col: 1,
            source: "ra".to_string(),
            idempotency_key: "diag:main:d1:src/a.rs:10:1".to_string(),
        };
        let _ = writer.record_diagnostic(&diag).expect("diag");

        let reader = StorageBackedContextEngine::open_reader(tmp.path().to_path_buf(), "repo-ctx3")
            .expect("open reader");
        let state = reader.materialize().expect("materialize");
        let branch = state.branches.get("main").expect("branch main");
        assert_eq!(branch.symbols.len(), 1);
        assert_eq!(branch.diagnostics.len(), 1);
    }

    #[test]
    fn ingest_rust_analyzer_snapshot_proto_records_all_parts() {
        let tmp = TempDir::new().expect("tempdir");
        let mut ctx =
            StorageBackedContextEngine::open_writer(tmp.path().to_path_buf(), "repo-ctx4")
                .expect("open");

        let snapshot = RustAnalyzerSnapshotV1 {
            schema_version: 1,
            repo_id: "repo-ctx4".to_string(),
            branch_id: "main".to_string(),
            symbols: vec![ContextSymbolUpsertedV1 {
                schema_version: 1,
                repo_id: String::new(),
                branch_id: String::new(),
                symbol_id: "sym-main".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                file_path: "src/main.rs".to_string(),
                line_start: 1,
                col_start: 1,
                line_end: 3,
                col_end: 1,
                signature: "fn main()".to_string(),
                idempotency_key: String::new(),
            }],
            references: vec![ContextReferenceRecordedV1 {
                schema_version: 1,
                repo_id: String::new(),
                branch_id: String::new(),
                symbol_id: "sym-main".to_string(),
                file_path: "src/lib.rs".to_string(),
                line: 10,
                col: 5,
                context_snippet: "main();".to_string(),
                idempotency_key: String::new(),
            }],
            diagnostics: vec![ContextDiagnosticRecordedV1 {
                schema_version: 1,
                repo_id: String::new(),
                branch_id: String::new(),
                diagnostic_id: "diag-1".to_string(),
                severity: "error".to_string(),
                message: "cannot find function".to_string(),
                file_path: "src/lib.rs".to_string(),
                line: 10,
                col: 5,
                source: "rust-analyzer".to_string(),
                idempotency_key: String::new(),
            }],
        };

        let out = ctx
            .ingest_rust_analyzer_snapshot_proto("main", &snapshot.encode_to_vec())
            .expect("ingest");
        assert_eq!(out.symbols_upserted, 1);
        assert_eq!(out.references_recorded, 1);
        assert_eq!(out.diagnostics_recorded, 1);

        let out_repeat = ctx
            .ingest_rust_analyzer_snapshot_proto("main", &snapshot.encode_to_vec())
            .expect("repeat ingest");
        assert_eq!(out_repeat.symbols_upserted, 1);
        assert_eq!(out_repeat.references_recorded, 1);
        assert_eq!(out_repeat.diagnostics_recorded, 1);

        let bundle = ctx
            .build_bundle("main", &["src/lib.rs".to_string()], 20, 20)
            .expect("bundle");
        assert_eq!(bundle.references.len(), 1);
        assert_eq!(bundle.diagnostics.len(), 1);

        let symbol_events = ctx
            .query(
                100,
                0,
                ContextQueryFilter {
                    event_type: Some(ContextEventTypeV1::ContextEventTypeSymbolUpserted),
                    branch_id: Some("main".to_string()),
                },
            )
            .expect("query symbol events");
        let ref_events = ctx
            .query(
                100,
                0,
                ContextQueryFilter {
                    event_type: Some(ContextEventTypeV1::ContextEventTypeReferenceRecorded),
                    branch_id: Some("main".to_string()),
                },
            )
            .expect("query ref events");
        let diag_events = ctx
            .query(
                100,
                0,
                ContextQueryFilter {
                    event_type: Some(ContextEventTypeV1::ContextEventTypeDiagnosticRecorded),
                    branch_id: Some("main".to_string()),
                },
            )
            .expect("query diag events");
        assert_eq!(symbol_events.events.len(), 1);
        assert_eq!(ref_events.events.len(), 1);
        assert_eq!(diag_events.events.len(), 1);
    }
}
